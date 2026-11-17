use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;

use serde::Deserialize;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, GetLogicalDrives,
    GetVolumeNameForVolumeMountPointW, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, OPEN_EXISTING,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    GET_LENGTH_INFORMATION, IOCTL_DISK_GET_LENGTH_INFO, IOCTL_STORAGE_QUERY_PROPERTY,
    PropertyStandardQuery, STORAGE_DEVICE_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
    StorageDeviceProperty, VOLUME_DISK_EXTENTS,
};
use windows::core::PCWSTR;

use crate::domain::{BusType, MediaType, PhysicalDisk};
use crate::error::CollectorError;
use crate::platform::windows::pdh::{PdhCounter, PdhQuery};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WmiDisk {
    index: u32,
    model: Option<String>,
    size: Option<u64>,
    interface_type: Option<String>,
    #[serde(rename = "PNPDeviceID")]
    pnp_device_id: Option<String>,
}

struct DiskIdentity {
    model: String,
    total_bytes: u64,
    media_type: MediaType,
    bus_type: BusType,
}

struct DeviceHandle(HANDLE);

impl Drop for DeviceHandle {
    fn drop(&mut self) {
        // SAFETY: the handle is owned by this object and closed exactly once.
        unsafe { CloseHandle(self.0).ok() };
    }
}

pub struct DiskCollector {
    query: PdhQuery,
    activity: Option<PdhCounter>,
    read: Option<PdhCounter>,
    write: Option<PdhCounter>,
    identities: BTreeMap<u32, DiskIdentity>,
}

impl DiskCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let query = PdhQuery::new()?;
        let identities = wmi::WMIConnection::new()
            .and_then(|connection| connection.query::<WmiDisk>())
            .map(|rows| {
                rows.into_iter()
                    .map(|row| {
                        let bus_type =
                            bus_type(row.interface_type.as_deref(), row.pnp_device_id.as_deref());
                        (
                            row.index,
                            DiskIdentity {
                                model: row.model.unwrap_or_else(|| "N/A".into()),
                                total_bytes: row.size.unwrap_or(0),
                                media_type: MediaType::Unknown,
                                bus_type,
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            activity: query.add_counter(r"\PhysicalDisk(*)\% Idle Time").ok(),
            read: query
                .add_counter(r"\PhysicalDisk(*)\Disk Read Bytes/sec")
                .ok(),
            write: query
                .add_counter(r"\PhysicalDisk(*)\Disk Write Bytes/sec")
                .ok(),
            query,
            identities,
        })
    }

    pub fn collect(&mut self) -> Result<Vec<PhysicalDisk>, CollectorError> {
        self.query.collect()?;
        let activity = self
            .activity
            .as_ref()
            .and_then(|counter| counter.get_instance_values().ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|(name, _)| name != "_Total")
            .filter_map(|(name, value)| {
                let number = name.split_whitespace().next()?.parse::<u32>().ok()?;
                value
                    .is_finite()
                    .then_some((number, (100.0 - value).clamp(0.0, 100.0) as f32))
            })
            .collect::<BTreeMap<_, _>>();
        let read = numeric_instances(&self.read);
        let write = numeric_instances(&self.write);
        let active_devices = active_device_numbers();
        let mut identities = std::mem::take(&mut self.identities);
        let disks = identities
            .iter_mut()
            .map(|(&number, identity)| {
                if active_devices.contains(&number)
                    && let Ok(native) = native_disk_identity(number)
                {
                    identity.model = native.model;
                    identity.total_bytes = native.total_bytes;
                    identity.media_type = native.media_type;
                    if native.bus_type != BusType::Unknown {
                        identity.bus_type = native.bus_type;
                    }
                }
                PhysicalDisk {
                    device_number: number,
                    model: identity.model.clone(),
                    total_bytes: identity.total_bytes,
                    media_type: identity.media_type,
                    bus_type: identity.bus_type,
                    read_bytes_per_sec: read.get(&number).copied(),
                    write_bytes_per_sec: write.get(&number).copied(),
                    active_time_percent: activity.get(&number).copied(),
                }
            })
            .collect();
        self.identities = identities;
        Ok(disks)
    }
}

pub fn get_device_number(volume_path: &str) -> Result<u32, CollectorError> {
    let path = wide(volume_path);
    let mut volume_name = [0_u16; 260];
    unsafe { GetVolumeNameForVolumeMountPointW(PCWSTR(path.as_ptr()), &mut volume_name) }
        .map_err(|error| CollectorError::Windows(format!("volume name: {error}")))?;
    let mut device_name = volume_name.to_vec();
    if let Some(position) = device_name.iter().position(|character| *character == 0) {
        device_name.truncate(position);
    }
    if device_name.ends_with(&['\\' as u16]) {
        device_name.pop();
    }
    device_name.push(0);
    // A volume handle with zero access only queries metadata and does not require raw disk access.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(device_name.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|error| CollectorError::Windows(format!("volume open: {error}")))?;
    let handle = DeviceHandle(handle);
    let mut bytes = 0;
    let mut buffer = [0_u8; 512];
    unsafe {
        DeviceIoControl(
            handle.0,
            IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
            None,
            0,
            Some(buffer.as_mut_ptr().cast()),
            buffer.len() as u32,
            Some(&mut bytes),
            None,
        )
    }
    .map_err(|error| CollectorError::Windows(format!("volume disk extents: {error}")))?;
    // SAFETY: the IOCTL returned a VOLUME_DISK_EXTENTS structure in this buffer.
    let extents = unsafe { &*buffer.as_ptr().cast::<VOLUME_DISK_EXTENTS>() };
    if extents.NumberOfDiskExtents == 0 {
        return Err(CollectorError::Unavailable(
            "volume has no disk extent".into(),
        ));
    }
    // SAFETY: Extents is a flexible array, and at least one extent was returned.
    let extent = unsafe { &*extents.Extents.as_ptr() };
    Ok(extent.DiskNumber)
}

fn numeric_instances(counter: &Option<PdhCounter>) -> BTreeMap<u32, u64> {
    counter
        .as_ref()
        .and_then(|counter| counter.get_instance_values().ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|(name, _)| name != "_Total")
        .filter_map(|(name, value)| {
            let number = name.split_whitespace().next()?.parse::<u32>().ok()?;
            (value.is_finite() && value >= 0.0).then_some((number, value as u64))
        })
        .collect()
}

fn active_device_numbers() -> BTreeSet<u32> {
    let drives = unsafe { GetLogicalDrives() };
    (0..26)
        .filter(|index| drives & (1 << index) != 0)
        .filter_map(|index| get_device_number(&format!("{}:\\", (b'A' + index) as char)).ok())
        .collect()
}

fn native_disk_identity(number: u32) -> Result<DiskIdentity, CollectorError> {
    let path = wide(&format!(r"\\.\PhysicalDrive{number}"));
    let handle = unsafe {
        CreateFileW(
            PCWSTR(path.as_ptr()),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    }
    .map_err(|error| CollectorError::Windows(format!("disk {number} open: {error}")))?;
    let handle = DeviceHandle(handle);
    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        AdditionalParameters: [0; 1],
    };
    let mut bytes = 0;
    let mut buffer = [0_u8; 1024];
    unsafe {
        DeviceIoControl(
            handle.0,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some((&query as *const STORAGE_PROPERTY_QUERY).cast()),
            size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            Some(buffer.as_mut_ptr().cast()),
            buffer.len() as u32,
            Some(&mut bytes),
            None,
        )
    }
    .map_err(|error| CollectorError::Windows(format!("disk {number} properties: {error}")))?;
    // SAFETY: the IOCTL returned a STORAGE_DEVICE_DESCRIPTOR with variable trailing strings.
    let descriptor = unsafe { &*buffer.as_ptr().cast::<STORAGE_DEVICE_DESCRIPTOR>() };
    let model =
        descriptor_string(&buffer, descriptor.ProductIdOffset).unwrap_or_else(|| "N/A".into());
    let mut length = GET_LENGTH_INFORMATION::default();
    let total_bytes = unsafe {
        DeviceIoControl(
            handle.0,
            IOCTL_DISK_GET_LENGTH_INFO,
            None,
            0,
            Some((&mut length as *mut GET_LENGTH_INFORMATION).cast()),
            size_of::<GET_LENGTH_INFORMATION>() as u32,
            Some(&mut bytes),
            None,
        )
    }
    .map(|_| length.Length as u64)
    .unwrap_or(0);
    Ok(DiskIdentity {
        model,
        total_bytes,
        media_type: MediaType::Unknown,
        bus_type: match descriptor.BusType.0 {
            17 => BusType::Nvme,
            11 => BusType::Sata,
            7 => BusType::Usb,
            _ => BusType::Unknown,
        },
    })
}

fn descriptor_string(buffer: &[u8], offset: u32) -> Option<String> {
    let tail = buffer.get(offset as usize..)?;
    let length = tail
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(tail.len());
    let text = String::from_utf8_lossy(&tail[..length]).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

fn bus_type(interface: Option<&str>, pnp: Option<&str>) -> BusType {
    if pnp.is_some_and(|id| id.contains("VEN_NVME")) {
        BusType::Nvme
    } else if pnp.is_some_and(|id| id.starts_with("USBSTOR")) {
        BusType::Usb
    } else if interface.is_some_and(|kind| kind == "SCSI") {
        BusType::Unknown
    } else {
        BusType::Sata
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
