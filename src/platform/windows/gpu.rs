use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, IDXGIAdapter1, DXGI_ADAPTER_DESC1,
    DXGI_ADAPTER_FLAG_SOFTWARE,
};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    PdhCloseQuery, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
    PDH_HQUERY, PDH_HCOUNTER,
};

use crate::domain::gpu::{GpuSnapshot, GpuEngine, GpuEngineKind};
use crate::error::CollectorError;
use crate::platform::windows::pdh::{PdhQuery, PdhCounter};

pub struct GpuCollector {
    query: PdhQuery,
    adapters: Vec<GpuAdapterInfo>,
    engine_counters: Vec<PdhCounter>,
    dedicated_counters: Vec<PdhCounter>,
    shared_counters: Vec<PdhCounter>,
    initialized: bool,
}

struct GpuAdapterInfo {
    name: String,
    vendor: String,
    luid: i64,
    dedicated_memory: u64,
    shared_memory: u64,
}

impl GpuCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let adapters = Self::enumerate_adapters()?;
        
        let query = PdhQuery::new()?;
        
        let mut engine_counters = Vec::new();
        let mut dedicated_counters = Vec::new();
        let mut shared_counters = Vec::new();
        
        for idx in 0..adapters.len() {
            let engine_path = format!(r"\GPU Engine({})\Utilization Percentage", idx);
            if let Ok(c) = query.add_counter(&engine_path) {
                engine_counters.push(c);
            }
            
            let dedicated_path = format!(r"\GPU Adapter Memory({})\Dedicated Usage", idx);
            if let Ok(c) = query.add_counter(&dedicated_path) {
                dedicated_counters.push(c);
            }
            
            let shared_path = format!(r"\GPU Adapter Memory({})\Shared Usage", idx);
            if let Ok(c) = query.add_counter(&shared_path) {
                shared_counters.push(c);
            }
        }
        
        Ok(Self {
            query,
            adapters,
            engine_counters,
            dedicated_counters,
            shared_counters,
            initialized: false,
        })
    }

    fn enumerate_adapters() -> Result<Vec<GpuAdapterInfo>, CollectorError> {
        let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1().map_err(|e| CollectorError::Windows(format!("CreateDXGIFactory1 failed: {}", e)))? };
        
        let mut adapters = Vec::new();
        let mut index = 0u32;
        
        loop {
            let adapter_result: Result<IDXGIAdapter1, _> = unsafe { factory.EnumAdapters1(index) };
            if adapter_result.is_err() {
                break;
            }
            
            let adapter = adapter_result.unwrap();
            let desc: DXGI_ADAPTER_DESC1 = unsafe { adapter.GetDesc1().map_err(|e| CollectorError::Windows(format!("GetDesc1 failed: {}", e)))? };
            
            if (desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
                index += 1;
                continue;
            }
            
            let name = String::from_utf16_lossy(&desc.Description).trim_end_matches('\0').to_string();
            let vendor = Self::get_vendor_name(desc.VendorId);
            let luid = ((desc.AdapterLuid.HighPart as i64) << 32) | (desc.AdapterLuid.LowPart as u32 as i64);
            let dedicated_memory = desc.DedicatedVideoMemory;
            let shared_memory = desc.SharedSystemMemory;
            
            adapters.push(GpuAdapterInfo {
                name,
                vendor,
                luid,
                dedicated_memory,
                shared_memory,
            });
            
            index += 1;
        }
        
        if adapters.is_empty() {
            return Err(CollectorError::Unavailable("No GPU adapters found".into()));
        }
        
        Ok(adapters)
    }

    fn get_vendor_name(vendor_id: u32) -> String {
        match vendor_id {
            0x10DE => "NVIDIA".into(),
            0x1002 => "AMD".into(),
            0x8086 => "Intel".into(),
            0x13B5 => "ARM".into(),
            0x1010 => "ImgTec".into(),
            _ => format!("Unknown (0x{:04X})", vendor_id),
        }
    }

    pub fn collect(&mut self) -> Result<Vec<GpuSnapshot>, CollectorError> {
        self.query.collect()?;
        
        let mut gpus = Vec::new();
        
        for (idx, adapter) in self.adapters.iter().enumerate() {
            let total_util = if idx < self.engine_counters.len() {
                self.engine_counters[idx].get_double().unwrap_or(0.0).clamp(0.0, 100.0) as f32
            } else { 0.0 };
            
            let engines = Self::parse_engine_instances(idx)?;
            
            let dedicated_used = if idx < self.dedicated_counters.len() {
                self.dedicated_counters[idx].get_double().ok().map(|v| v as u64)
            } else { None };
            
            let shared_used = if idx < self.shared_counters.len() {
                self.shared_counters[idx].get_double().ok().map(|v| v as u64)
            } else { None };
            
            let dedicated_total = if adapter.dedicated_memory > 0 {
                Some(adapter.dedicated_memory)
            } else { None };
            
            let shared_total = if adapter.shared_memory > 0 {
                Some(adapter.shared_memory)
            } else { None };
            
            gpus.push(GpuSnapshot {
                name: adapter.name.clone(),
                vendor: adapter.vendor.clone(),
                total_utilization: total_util,
                engines,
                dedicated_vram_total_bytes: dedicated_total,
                dedicated_vram_used_bytes: dedicated_used,
                shared_memory_total_bytes: shared_total,
                shared_memory_used_bytes: shared_used,
                temperature_celsius: None,
                core_clock_mhz: None,
                memory_clock_mhz: None,
                power_watts: None,
                fan_rpm: None,
            });
        }
        
        self.initialized = true;
        Ok(gpus)
    }

    fn parse_engine_instances(adapter_idx: usize) -> Result<Vec<GpuEngine>, CollectorError> {
        let mut engines = Vec::new();
        
        let counter_path = format!(r"\GPU Engine({})\Utilization Percentage", adapter_idx);
        let counter_path_wide: Vec<u16> = OsStr::new(&counter_path).encode_wide().chain(Some(0)).collect();
        
        let mut buffer_size = 0u32;
        let mut item_count = 0u32;
        let status = unsafe {
            windows::Win32::System::Performance::PdhGetFormattedCounterArrayW(
                ptr::null_mut(),
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                ptr::null_mut(),
            )
        };
        
        if status != 0 && status != 0x800007D2 {
            return Err(CollectorError::Pdh(format!("PdhGetFormattedCounterArrayW size query failed: 0x{:X}", status)));
        }
        
        if buffer_size == 0 {
            return Ok(engines);
        }
        
        let mut buffer = vec![0u8; buffer_size as usize];
        let status = unsafe {
            windows::Win32::System::Performance::PdhGetFormattedCounterArrayW(
                ptr::null_mut(),
                PDH_FMT_DOUBLE,
                &mut buffer_size,
                &mut item_count,
                buffer.as_mut_ptr() as *mut _,
            )
        };
        
        if status != 0 {
            return Err(CollectorError::Pdh(format!("PdhGetFormattedCounterArrayW failed: 0x{:X}", status)));
        }
        
        let items = unsafe { std::slice::from_raw_parts(buffer.as_ptr() as *const windows::Win32::System::Performance::PDH_FMT_COUNTERVALUE_ITEM_W, item_count as usize) };
        
        for item in items {
            let name_ptr = item.szName;
            if name_ptr.is_null() {
                continue;
            }
            
            let name_len = (0..).take_while(|&i| unsafe { *name_ptr.0.add(i) != 0 }).count();
            let name_slice = unsafe { std::slice::from_raw_parts(name_ptr.0, name_len) };
            let instance_name = String::from_utf16_lossy(name_slice);
            
            if let Some(engine_kind) = Self::parse_engine_type(&instance_name) {
                let util = unsafe { item.FmtValue.Anonymous.doubleValue }.clamp(0.0, 100.0) as f32;
                engines.push(GpuEngine {
                    kind: engine_kind,
                    utilization: util,
                });
            }
        }
        
        if engines.is_empty() {
            engines.push(GpuEngine {
                kind: GpuEngineKind::ThreeD,
                utilization: 0.0,
            });
        }
        
        Ok(engines)
    }

    fn parse_engine_type(instance_name: &str) -> Option<GpuEngineKind> {
        let name = instance_name.to_lowercase();
        
        if name.contains("3d") || name.contains("graphics") {
            Some(GpuEngineKind::ThreeD)
        } else if name.contains("compute") {
            Some(GpuEngineKind::Compute)
        } else if name.contains("copy") {
            Some(GpuEngineKind::Copy)
        } else if name.contains("video") && name.contains("decode") {
            Some(GpuEngineKind::VideoDecode)
        } else if name.contains("video") && name.contains("encode") {
            Some(GpuEngineKind::VideoEncode)
        } else if name.contains("video") && name.contains("process") {
            Some(GpuEngineKind::VideoProcessing)
        } else {
            Some(GpuEngineKind::Other)
        }
    }
}