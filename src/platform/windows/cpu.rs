use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, GetSystemInfo,
    LOGICAL_PROCESSOR_RELATIONSHIP, RelationProcessorCore,
    SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX, SYSTEM_INFO,
};
use windows::Win32::System::Registry::{
    RegOpenKeyExW, RegQueryValueExW, RegCloseKey, HKEY, KEY_READ,
    HKEY_LOCAL_MACHINE, REG_SZ, REG_VALUE_TYPE,
};

use crate::domain::cpu::{CpuSnapshot, LogicalProcessorUsage};
use crate::error::CollectorError;
use crate::platform::windows::pdh::{PdhQuery, PdhCounter};

pub struct CpuCollector {
    query: PdhQuery,
    total_counter: PdhCounter,
    per_core_counters: Vec<PdhCounter>,
    logical_count: u32,
    physical_count: u32,
    sockets: u32,
    core_to_logical: Vec<Vec<u32>>,
    name: String,
    initialized: bool,
}

impl CpuCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let (logical_count, physical_count, sockets, core_to_logical, name) = Self::get_topology()?;
        
        let query = PdhQuery::new()?;
        
        let total_counter = query.add_counter(r"\Processor Information(_Total)\% Processor Utility")?;
        
        let mut per_core_counters = Vec::new();
        for i in 0..logical_count {
            let path = format!(r"\Processor Information({},{})\% Processor Utility", i, i);
            match query.add_counter(&path) {
                Ok(c) => per_core_counters.push(c),
                Err(_) => break,
            }
        }

        Ok(Self {
            query,
            total_counter,
            per_core_counters,
            logical_count,
            physical_count,
            sockets,
            core_to_logical,
            name,
            initialized: false,
        })
    }

    fn get_topology() -> Result<(u32, u32, u32, Vec<Vec<u32>>, String), CollectorError> {
        let mut return_length = 0u32;
        
        let result = unsafe {
            GetLogicalProcessorInformationEx(RelationProcessorCore, None, &mut return_length)
        };
        
        if result != 0 && return_length == 0 {
            return Err(CollectorError::Windows("GetLogicalProcessorInformationEx returned 0 length".into()));
        }

        let mut buffer = vec![0u8; return_length as usize];
        
        let result = unsafe {
            GetLogicalProcessorInformationEx(
                RelationProcessorCore,
                Some(buffer.as_mut_ptr() as *mut _),
                &mut return_length,
            )
        };
        
        if result == 0 {
            return Err(CollectorError::Windows("GetLogicalProcessorInformationEx failed".into()));
        }

        let mut physical_cores = 0u32;
        let mut core_to_logical = Vec::new();
        let mut offset = 0usize;
        
        while offset < return_length as usize {
            let info = unsafe { &*(buffer.as_ptr().add(offset) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX) };
            
            if info.Relationship == RelationProcessorCore {
                let processor_core = unsafe { &info.Anonymous.Processor };
                let mask = processor_core.GroupMask[0].Mask;
                let mut logicals = Vec::new();
                
                for i in 0..64 {
                    if (mask as u64) & (1u64 << i) != 0 {
                        logicals.push(i as u32);
                    }
                }
                
                if !logicals.is_empty() {
                    physical_cores += 1;
                    core_to_logical.push(logicals);
                }
            }
            
            offset += info.Size as usize;
        }

        let sockets = if physical_cores > 0 { 1 } else { 0 };
        
        let mut logical_count = 0u32;
        for core in &core_to_logical {
            logical_count += core.len() as u32;
        }

        let mut sys_info = SYSTEM_INFO::default();
        unsafe { GetSystemInfo(&mut sys_info) };
        
        if logical_count == 0 {
            logical_count = sys_info.dwNumberOfProcessors;
        }

        let name = Self::get_processor_name()?;
        
        Ok((logical_count, physical_cores, sockets, core_to_logical, name))
    }

    fn get_processor_name() -> Result<String, CollectorError> {
        let subkey = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0";
        let subkey_wide: Vec<u16> = OsStr::new(subkey).encode_wide().chain(Some(0)).collect();
        
        let mut key = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                windows::core::PCWSTR(subkey_wide.as_ptr()),
                Some(0),
                KEY_READ,
                &mut key,
            )
        };
        
        if status != WIN32_ERROR(0) {
            return Ok("Unknown Processor".into());
        }
        
        let value_name = "ProcessorNameString";
        let value_name_wide: Vec<u16> = OsStr::new(value_name).encode_wide().chain(Some(0)).collect();
        
        let mut value_buffer = vec![0u8; 256];
        let mut value_size = value_buffer.len() as u32;
        let mut value_type = 0u32;
        
        let status = unsafe {
            RegQueryValueExW(
                key,
                windows::core::PCWSTR(value_name_wide.as_ptr()),
                None,
                Some(&mut value_type as *mut u32),
                Some(value_buffer.as_mut_ptr()),
                Some(&mut value_size),
            )
        };
        
        let mut name = String::new();
        if status == WIN32_ERROR(0) && value_type == REG_SZ.0 as u32 {
            let wide_slice = unsafe { std::slice::from_raw_parts(value_buffer.as_ptr() as *const u16, (value_size / 2) as usize) };
            name = String::from_utf16_lossy(wide_slice).trim_end_matches('\0').to_string();
        }
        
        unsafe { RegCloseKey(key) };
        
        if name.is_empty() {
            name = "Unknown Processor".into();
        }
        
        Ok(name)
    }

    pub fn collect(&mut self) -> Result<CpuSnapshot, CollectorError> {
        self.query.collect()?;
        
        let total_util = self.total_counter.get_double()?.clamp(0.0, 100.0) as f32;
        
        let mut per_logical = vec![0.0f32; self.logical_count as usize];
        
        if self.per_core_counters.len() == self.logical_count as usize {
            for (i, counter) in self.per_core_counters.iter().enumerate() {
                if let Ok(val) = counter.get_double() {
                    let util = val.clamp(0.0, 100.0) as f32;
                    if i < self.core_to_logical.len() {
                        for &lp in &self.core_to_logical[i] {
                            if (lp as usize) < per_logical.len() {
                                per_logical[lp as usize] = util;
                            }
                        }
                    }
                }
            }
        } else {
            for i in 0..self.logical_count as usize {
                per_logical[i] = total_util;
            }
        }
        
        let active_cores = if self.initialized {
            self.core_to_logical.iter()
                .filter(|core| {
                    core.iter().any(|&lp| {
                        (lp as usize) < per_logical.len() && per_logical[lp as usize] > 5.0
                    })
                })
                .count() as u32
        } else {
            self.physical_count
        };

        self.initialized = true;

        let logical_usage: Vec<LogicalProcessorUsage> = per_logical.iter().enumerate()
            .map(|(i, &util)| {
                let physical_core = self.core_to_logical.iter().position(|core| core.contains(&(i as u32)))
                    .unwrap_or(i / 2) as u32;
                LogicalProcessorUsage {
                    index: i as u32,
                    physical_core_index: physical_core,
                    utilization: util,
                }
            })
            .collect();

        Ok(CpuSnapshot {
            name: self.name.clone(),
            total_utilization: total_util,
            physical_cores: self.physical_count,
            logical_processors: self.logical_count,
            sockets: self.sockets,
            active_cores,
            per_logical: logical_usage,
            base_clock_mhz: None,
            current_clock_mhz: None,
        })
    }
}