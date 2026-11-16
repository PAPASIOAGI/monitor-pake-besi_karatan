use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::Win32::Storage::FileSystem::{
    GetLogicalDrives, GetDiskFreeSpaceExW, GetVolumeInformationW,
    GetDriveTypeW,
};

use crate::domain::storage::{Volume, VolumeKind};
use crate::error::CollectorError;
use crate::platform::windows::disk::DiskCollector;

pub struct VolumeCollector {
    device_map: Vec<(String, u32)>,
}

impl VolumeCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let device_map = Self::build_device_map()?;
        Ok(Self { device_map })
    }

    fn build_device_map() -> Result<Vec<(String, u32)>, CollectorError> {
        let mut device_map = Vec::new();
        let drives = unsafe { GetLogicalDrives() };
        
        for i in 0..26 {
            if drives & (1 << i) != 0 {
                let drive_letter = (b'A' + i) as char;
                let volume_path = format!("{}:\\", drive_letter);
                
                if let Ok(device_number) = DiskCollector::get_device_number(&volume_path) {
                    device_map.push((volume_path, device_number));
                }
            }
        }
        
        Ok(device_map)
    }

    pub fn collect(&self) -> Result<Vec<Volume>, CollectorError> {
        let mut volumes = Vec::new();
        let drives = unsafe { GetLogicalDrives() };
        
        for i in 0..26 {
            if drives & (1 << i) != 0 {
                let drive_letter = (b'A' + i) as char;
                let volume_path = format!("{}:\\", drive_letter);
                
                let volume_path_wide: Vec<u16> = OsStr::new(&volume_path).encode_wide().chain(Some(0)).collect();
                
                let mut volume_name = [0u16; 260];
                let mut fs_name = [0u16; 260];
                let mut serial = 0u32;
                let mut max_comp = 0u32;
                let mut fs_flags = 0u32;
                
                let _result = unsafe {
                    GetVolumeInformationW(
                        windows::core::PCWSTR(volume_path_wide.as_ptr()),
                        Some(&mut volume_name),
                        Some(&mut serial),
                        Some(&mut max_comp),
                        Some(&mut fs_flags),
                        Some(&mut fs_name),
                    )
                };
                
                let label = String::from_utf16_lossy(&volume_name).trim_end_matches('\0').to_string();
                let filesystem = String::from_utf16_lossy(&fs_name).trim_end_matches('\0').to_string();
                
                let mut free_bytes = 0u64;
                let mut total_bytes = 0u64;
                let mut total_free = 0u64;
                
                let _ = unsafe {
                    GetDiskFreeSpaceExW(
                        windows::core::PCWSTR(volume_path_wide.as_ptr()),
                        Some(&mut free_bytes),
                        Some(&mut total_bytes),
                        Some(&mut total_free),
                    )
                };
                
                let used_bytes = total_bytes.saturating_sub(free_bytes);
                
                let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(volume_path_wide.as_ptr())) };
                let kind = match drive_type {
                    3 => VolumeKind::Local,        // DRIVE_FIXED
                    2 => VolumeKind::Removable,    // DRIVE_REMOVABLE
                    4 => VolumeKind::Network,      // DRIVE_REMOTE
                    5 => VolumeKind::Removable,    // DRIVE_CDROM
                    _ => VolumeKind::Unknown,
                };
                
                volumes.push(Volume {
                    mount_point: volume_path,
                    label,
                    filesystem,
                    total_bytes,
                    used_bytes,
                    available_bytes: free_bytes,
                    kind,
                });
            }
        }
        
        Ok(volumes)
    }

    pub fn get_device_number(&self, mount_point: &str) -> Option<u32> {
        self.device_map.iter()
            .find(|(mp, _)| mp == mount_point)
            .map(|(_, dn)| *dn)
    }
}