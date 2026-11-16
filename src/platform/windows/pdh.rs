use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::ptr;

use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhCollectQueryDataWithTime,
    PdhGetFormattedCounterArrayW, PdhOpenQueryW, PdhValidatePathW,
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    PDH_HQUERY, PDH_HCOUNTER,
};

use crate::error::CollectorError;

pub struct PdhQuery {
    handle: PDH_HQUERY,
}

impl PdhQuery {
    pub fn new() -> Result<Self, CollectorError> {
        let mut handle = PDH_HQUERY::default();
        let status = unsafe { PdhOpenQueryW(None, 0, &mut handle) };
        if status != 0 {
            return Err(CollectorError::Pdh(format!("PdhOpenQueryW failed: 0x{:X}", status)));
        }
        Ok(Self { handle })
    }

    pub fn add_counter(&self, path: &str) -> Result<PdhCounter, CollectorError> {
        let path_wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
        let mut counter_handle = PDH_HCOUNTER::default();
        let status = unsafe { PdhAddEnglishCounterW(self.handle, windows::core::PCWSTR(path_wide.as_ptr()), 0, &mut counter_handle) };
        if status != 0 {
            return Err(CollectorError::Pdh(format!("PdhAddEnglishCounterW failed for {}: 0x{:X}", path, status)));
        }
        Ok(PdhCounter { handle: counter_handle })
    }

    pub fn collect(&self) -> Result<(), CollectorError> {
        let status = unsafe { PdhCollectQueryData(self.handle) };
        if status != 0 {
            return Err(CollectorError::Pdh(format!("PdhCollectQueryData failed: 0x{:X}", status)));
        }
        Ok(())
    }

    pub fn collect_with_time(&self) -> Result<u64, CollectorError> {
        let mut timestamp = 0i64;
        let status = unsafe { PdhCollectQueryDataWithTime(self.handle, &mut timestamp) };
        if status != 0 {
            return Err(CollectorError::Pdh(format!("PdhCollectQueryDataWithTime failed: 0x{:X}", status)));
        }
        Ok(timestamp as u64)
    }
}

impl Drop for PdhQuery {
    fn drop(&mut self) {
        unsafe { PdhCloseQuery(self.handle) };
    }
}

pub struct PdhCounter {
    handle: PDH_HCOUNTER,
}

impl PdhCounter {
    pub fn get_double(&self) -> Result<f64, CollectorError> {
        let mut buffer_size = 0u32;
        let mut item_count = 0u32;
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                ptr::null_mut(),
            )
        };
        if status != 0 && status != 0x800007D2 {
            return Err(CollectorError::Pdh(format!("PdhGetFormattedCounterArrayW size query failed: 0x{:X}", status)));
        }

        let mut buffer = vec![0u8; buffer_size as usize];
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                buffer.as_mut_ptr() as *mut _,
            )
        };
        if status != 0 {
            return Err(CollectorError::Pdh(format!("PdhGetFormattedCounterArrayW failed: 0x{:X}", status)));
        }

        if item_count == 0 {
            return Err(CollectorError::Pdh("No counter data available".into()));
        }

        let item = unsafe { &*(buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W) };
        Ok(unsafe { item.FmtValue.Anonymous.doubleValue })
    }

    pub fn get_instance_values(&self) -> Result<Vec<(String, f64)>, CollectorError> {
        let mut buffer_size = 0u32;
        let mut item_count = 0u32;
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                ptr::null_mut(),
            )
        };
        if status != 0 && status != 0x800007D2 {
            return Err(CollectorError::Pdh(format!("PdhGetFormattedCounterArrayW size query failed: 0x{:X}", status)));
        }

        let mut buffer = vec![0u8; buffer_size as usize];
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                buffer.as_mut_ptr() as *mut _,
            )
        };
        if status != 0 {
            return Err(CollectorError::Pdh(format!("PdhGetFormattedCounterArrayW failed: 0x{:X}", status)));
        }

        let mut results = Vec::new();
        let items = unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W, item_count as usize) };
        for item in items {
            let name_ptr = item.szName;
            if name_ptr.is_null() {
                continue;
            }
            let name = unsafe { std::ffi::OsString::from_wide(
                std::slice::from_raw_parts(name_ptr.0, (0..).take_while(|&i| unsafe { *name_ptr.0.add(i) } != 0).count())
            ).to_string_lossy().into_owned() };
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            results.push((name, value));
        }
        Ok(results)
    }
}

pub fn validate_counter_path(path: &str) -> Result<(), CollectorError> {
    let path_wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
    let status = unsafe { PdhValidatePathW(windows::core::PCWSTR(path_wide.as_ptr())) };
    if status != 0 {
        return Err(CollectorError::Pdh(format!("PdhValidatePathW failed for {}: 0x{:X}", path, status)));
    }
    Ok(())
}