use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::Win32::Foundation::{INVALID_HANDLE_VALUE, WIN32_ERROR};
use windows::Win32::Storage::FileSystem::{
    GetLogicalDrives, GetDiskFreeSpaceExW,
};
use windows::Win32::Storage::FileSystem::GetVolumeNameForVolumeMountPointW;

use crate::domain::storage::{PhysicalDisk, MediaType, BusType};
use crate::error::CollectorError;
use crate::platform::windows::pdh::{PdhQuery, PdhCounter};

pub struct DiskCollector {
    query: PdhQuery,
    read_counters: Vec<PdhCounter>,
    write_counters: Vec<PdhCounter>,
    active_time_counters: Vec<PdhCounter>,
    device_numbers: Vec<u32>,
}

impl DiskCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let query = PdhQuery::new()?;
        
        let mut device_numbers = Vec::new();
        let mut read_counters = Vec::new();
        let mut write_counters = Vec::new();
        let mut active_time_counters = Vec::new();

        let drives = unsafe { GetLogicalDrives() };
        for i in 0..26 {
            if drives & (1 << i) != 0 {
                let drive_letter = (b'A' + i) as char;
                let volume_path = format!("{}:\\", drive_letter);
                
                if let Ok(device_number) = Self::get_device_number(&volume_path) {
                    device_numbers.push(device_number);
                    
                    let read_path = format!(r"\PhysicalDisk({})\Disk Read Bytes/sec", device_number);
                    let write_path = format!(r"\PhysicalDisk({})\Disk Write Bytes/sec", device_number);
                    let active_path = format!(r"\PhysicalDisk({})\% Disk Time", device_number);
                    
                    if let Ok(c) = query.add_counter(&read_path) {
                        read_counters.push(c);
                    }
                    if let Ok(c) = query.add_counter(&write_path) {
                        write_counters.push(c);
                    }
                    if let Ok(c) = query.add_counter(&active_path) {
                        active_time_counters.push(c);
                    }
                }
            }
        }

        Ok(Self {
            query,
            read_counters,
            write_counters,
            active_time_counters,
            device_numbers,
        })
    }

    pub fn get_device_number(volume_path: &str) -> Result<u32, CollectorError> {
        let volume_path_wide: Vec<u16> = OsStr::new(volume_path).encode_wide().chain(Some(0)).collect();
        
        let mut volume_name = [0u16; 260];
        let result = unsafe {
            GetVolumeNameForVolumeMountPointW(
                windows::core::PCWSTR(volume_path_wide.as_ptr()),
                &mut volume_name,
            )
        };
        
        if result == 0 {
            return Err(CollectorError::Windows("GetVolumeNameForVolumeMountPointW failed".into()));
        }
        
        let volume_handle = unsafe {
            windows::Win32::System::IO::CreateFileW(
                windows::core::PCWSTR(volume_name.as_ptr()),
                windows::Win32::System::IO::GENERIC_READ,
                windows::Win32::System::IO::FILE_SHARE_READ | windows::Win32::System::IO::FILE_SHARE_WRITE,
                None,
                windows::Win32::System::IO::OPEN_EXISTING,
                windows::Win32::System::IO::FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };
        
        if volume_handle == INVALID_HANDLE_VALUE {
            return Err(CollectorError::Windows("CreateFileW for volume failed".into()));
        }
        
        let mut extents = windows::Win32::Storage::DeviceIOControl::VOLUME_DISK_EXTENTS {
            NumberOfDiskExtents: 0,
            Extents: [Default::default(); 1],
        };
        let mut bytes_returned = 0u32;
        
        let result = unsafe {
            windows::Win32::System::IO::DeviceIoControl(
                volume_handle,
                windows::Win32::Storage::DeviceIOControl::IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
                None,
                0,
                Some(&mut extents as *mut _ as *mut _),
                std::mem::size_of::<windows::Win32::Storage::DeviceIOControl::VOLUME_DISK_EXTENTS>() as u32,
                Some(&mut bytes_returned),
                None,
            )
        };
        
        unsafe { windows::Win32::Foundation::CloseHandle(volume_handle) };
        
        if result == 0 || extents.NumberOfDiskExtents == 0 {
            return Err(CollectorError::Windows("IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS failed".into()));
        }
        
        Ok(extents.Extents[0].DiskNumber)
    }

    pub fn collect(&mut self) -> Result<Vec<PhysicalDisk>, CollectorError> {
        self.query.collect()?;

        let mut disks = Vec::new();
        
        for (idx, &device_number) in self.device_numbers.iter().enumerate() {
            let (model, media_type, bus_type, total_bytes) = Self::get_disk_info(device_number)?;
            
            let read_bytes = if idx < self.read_counters.len() {
                self.read_counters[idx].get_double().ok().map(|v| v as u64)
            } else { None };
            
            let write_bytes = if idx < self.write_counters.len() {
                self.write_counters[idx].get_double().ok().map(|v| v as u64)
            } else { None };
            
            let active_time = if idx < self.active_time_counters.len() {
                self.active_time_counters[idx].get_double().ok().map(|v| v as f32)
            } else { None };

            disks.push(PhysicalDisk {
                device_number,
                model,
                total_bytes,
                media_type,
                bus_type,
                read_bytes_per_sec: read_bytes,
                write_bytes_per_sec: write_bytes,
                active_time_percent: active_time,
            });
        }

        Ok(disks)
    }

    fn get_disk_info(device_number: u32) -> Result<(String, MediaType, BusType, u64), CollectorError> {
        let device_path = format!(r"\\.\PhysicalDrive{}", device_number);
        let device_path_wide: Vec<u16> = OsStr::new(&device_path).encode_wide().chain(Some(0)).collect();
        
        let handle = unsafe {
            windows::Win32::System::IO::CreateFileW(
                windows::core::PCWSTR(device_path_wide.as_ptr()),
                windows::Win32::System::IO::GENERIC_READ,
                windows::Win32::System::IO::FILE_SHARE_READ | windows::Win32::System::IO::FILE_SHARE_WRITE,
                None,
                windows::Win32::System::IO::OPEN_EXISTING,
                windows::Win32::System::IO::FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };
        
        if handle == INVALID_HANDLE_VALUE {
            return Ok(("Unknown".into(), MediaType::Unknown, BusType::Unknown, 0));
        }
        
        let mut model = String::new();
        let mut media_type = MediaType::Unknown;
        let mut bus_type = BusType::Unknown;
        let mut total_bytes = 0u64;

        let mut query = windows::Win32::Storage::DeviceIOControl::STORAGE_PROPERTY_QUERY {
            PropertyId: windows::Win32::Storage::DeviceIOControl::STORAGE_PROPERTY_ID::StorageDeviceProperty,
            QueryType: windows::Win32::Storage::DeviceIOControl::STORAGE_QUERY_TYPE::PropertyStandardQuery,
            AdditionalParameters: [0; 1],
        };
        
        let mut buffer = [0u8; 1024];
        let mut bytes_returned = 0u32;
        
        let result = unsafe {
            windows::Win32::System::IO::DeviceIoControl(
                handle,
                windows::Win32::Storage::DeviceIOControl::IOCTL_STORAGE_QUERY_PROPERTY,
                Some(&query as *const _ as *const _),
                std::mem::size_of::<windows::Win32::Storage::DeviceIOControl::STORAGE_PROPERTY_QUERY>() as u32,
                Some(buffer.as_mut_ptr() as *mut _),
                buffer.len() as u32,
                Some(&mut bytes_returned),
                None,
            )
        };
        
        if result != 0 && bytes_returned > 0 {
            let descriptor = unsafe { &*(buffer.as_ptr() as *const windows::Win32::Storage::DeviceIOControl::STORAGE_DEVICE_DESCRIPTOR) };
            
            if descriptor.ProductIdOffset > 0 {
                let offset = descriptor.ProductIdOffset as usize;
                let end = buffer[offset..].iter().position(|&b| b == 0).unwrap_or(buffer.len() - offset);
                model = String::from_utf8_lossy(&buffer[offset..offset + end]).trim().to_string();
            }
            
            media_type = match descriptor.DeviceType {
                0x07 => MediaType::Ssd,
                0x00 => MediaType::Hdd,
                _ => MediaType::Unknown,
            };
            
            bus_type = match descriptor.BusType {
                17 => BusType::Nvme,
                11 => BusType::Sata,
                7 => BusType::Usb,
                _ => BusType::Unknown,
            };
        }

        let mut total_bytes64 = 0u64;
        let mut free_bytes = 0u64;
        let mut total_free = 0u64;
        let device_path_wide: Vec<u16> = OsStr::new(&device_path).encode_wide().chain(Some(0)).collect();
        
        let _ = unsafe {
            GetDiskFreeSpaceExW(
                windows::core::PCWSTR(device_path_wide.as_ptr()),
                Some(&mut free_bytes),
                Some(&mut total_bytes64),
                Some(&mut total_free),
            )
        };
        
        total_bytes = total_bytes64;
        
        unsafe { windows::Win32::Foundation::CloseHandle(handle) };

        Ok((model, media_type, bus_type, total_bytes))
    }
}