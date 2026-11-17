use serde::Deserialize;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

use crate::domain::{MemoryModule, MemorySnapshot};
use crate::error::CollectorError;
use crate::platform::windows::pdh::{PdhCounter, PdhQuery};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WmiMemory {
    device_locator: Option<String>,
    manufacturer: Option<String>,
    part_number: Option<String>,
    serial_number: Option<String>,
    capacity: Option<u64>,
    speed: Option<u32>,
    configured_clock_speed: Option<u32>,
    #[serde(rename = "SMBIOSMemoryType")]
    smbios_memory_type: Option<u16>,
    form_factor: Option<u16>,
}

pub struct MemoryCollector {
    query: PdhQuery,
    commit_used: Option<PdhCounter>,
    commit_limit: Option<PdhCounter>,
    cached: Option<PdhCounter>,
    page_file_used: Option<PdhCounter>,
    #[allow(dead_code)] // Reserved for future page file total tracking
    page_file_total: Option<PdhCounter>,
    modules: Vec<MemoryModule>,
}

impl MemoryCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let query = PdhQuery::new()?;
        let modules = wmi::WMIConnection::new()
            .and_then(|connection| connection.query::<WmiMemory>())
            .map(|rows| rows.into_iter().map(module_from_wmi).collect())
            .unwrap_or_default();
        Ok(Self {
            commit_used: query.add_counter(r"\Memory\Committed Bytes").ok(),
            commit_limit: query.add_counter(r"\Memory\Commit Limit").ok(),
            cached: query.add_counter(r"\Memory\Cache Bytes").ok(),
            page_file_used: query.add_counter(r"\Paging File(_Total)\% Usage").ok(),
            page_file_total: query.add_counter(r"\Paging File(_Total)\% Usage Peak").ok(),
            query,
            modules,
        })
    }

    pub fn collect(&mut self) -> Result<MemorySnapshot, CollectorError> {
        let mut status = MEMORYSTATUSEX {
            dwLength: size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        unsafe { GlobalMemoryStatusEx(&mut status) }
            .map_err(|error| CollectorError::Windows(format!("GlobalMemoryStatusEx: {error}")))?;
        self.query.collect()?;

        let total_bytes = status.ullTotalPhys;
        let available_bytes = status.ullAvailPhys.min(total_bytes);
        let used_bytes = total_bytes - available_bytes;
        let commit_used_bytes = value(&self.commit_used);
        let commit_limit_bytes = value(&self.commit_limit);
        let page_file_used_bytes = self
            .page_file_used
            .as_ref()
            .and_then(|counter| counter.get_double().ok())
            .filter(|value| value.is_finite())
            .map(|value| (status.ullTotalPageFile as f64 * value.clamp(0.0, 100.0) / 100.0) as u64);

        Ok(MemorySnapshot {
            total_bytes,
            used_bytes,
            available_bytes,
            commit_used_bytes,
            commit_limit_bytes,
            page_file_used_bytes,
            page_file_total_bytes: Some(status.ullTotalPageFile),
            cached_bytes: value(&self.cached),
            modules: self.modules.clone(),
        })
    }
}

fn value(counter: &Option<PdhCounter>) -> Option<u64> {
    counter
        .as_ref()?
        .get_double()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value as u64)
}

fn module_from_wmi(row: WmiMemory) -> MemoryModule {
    MemoryModule {
        slot: text(row.device_locator).unwrap_or_else(|| "N/A".into()),
        manufacturer: text(row.manufacturer).filter(|value| value != "Unknown"),
        part_number: text(row.part_number),
        serial_number: text(row.serial_number).filter(|value| value != "00000000"),
        capacity_bytes: row.capacity.unwrap_or(0),
        configured_speed_mt_s: nonzero(row.configured_clock_speed),
        rated_speed_mt_s: nonzero(row.speed),
        memory_type: memory_type(row.smbios_memory_type),
        form_factor: form_factor(row.form_factor),
        // WMI does not expose active memory timings; never infer CL from speed/model.
        cas_latency: None,
    }
}

fn nonzero(value: Option<u32>) -> Option<u32> {
    value.filter(|value| *value > 0)
}

fn text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn memory_type(value: Option<u16>) -> String {
    match value {
        Some(20) => "DDR",
        Some(21) => "DDR2",
        Some(24) => "DDR3",
        Some(26) => "DDR4",
        Some(34) => "DDR5",
        Some(0) => "N/A",
        Some(value) => return format!("SMBIOS type {value}"),
        None => "N/A",
    }
    .into()
}

fn form_factor(value: Option<u16>) -> String {
    match value {
        Some(8) => "DIMM",
        Some(12) => "SODIMM",
        Some(13) => "LPDDR",
        _ => "N/A",
    }
    .into()
}
