use std::collections::BTreeMap;
use std::mem::size_of;

use serde::Deserialize;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, WIN32_ERROR};
use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows::Win32::System::SystemInformation::{
    GetLogicalProcessorInformationEx, LOGICAL_PROCESSOR_RELATIONSHIP, RelationAll,
    RelationProcessorCore, RelationProcessorPackage, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
};
use windows::core::PCWSTR;

use crate::domain::{CpuSnapshot, LogicalProcessorUsage};
use crate::error::CollectorError;
use crate::platform::windows::pdh::{PdhCounter, PdhQuery};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WmiProcessor {
    name: Option<String>,
    manufacturer: Option<String>,
    max_clock_speed: Option<u32>,
}

struct Topology {
    cores: Vec<Vec<u32>>,
    logical_count: u32,
    sockets: u32,
}

pub struct CpuCollector {
    query: PdhQuery,
    total: PdhCounter,
    logical: Option<PdhCounter>,
    performance: Option<PdhCounter>,
    topology: Topology,
    name: String,
    vendor: String,
    base_clock_mhz: Option<u32>,
}

impl CpuCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let query = PdhQuery::new()?;
        let total = query.add_counter(r"\Processor Information(_Total)\% Processor Utility")?;
        let logical = query
            .add_counter(r"\Processor Information(*)\% Processor Utility")
            .ok();
        let performance = query
            .add_counter(r"\Processor Information(_Total)\% Processor Performance")
            .ok();
        let topology = read_topology()?;
        let identity = read_identity();
        Ok(Self {
            query,
            total,
            logical,
            performance,
            topology,
            name: identity.0,
            vendor: identity.1,
            base_clock_mhz: identity.2,
        })
    }

    pub fn collect(&mut self) -> Result<CpuSnapshot, CollectorError> {
        self.query.collect()?;
        let total = finite_percent(self.total.get_double().ok());
        let performance = self
            .performance
            .as_ref()
            .and_then(|counter| counter.get_double().ok())
            .filter(|value| value.is_finite() && *value >= 0.0);
        let current_clock_mhz = self
            .base_clock_mhz
            .and_then(|base| performance.map(|value| (base as f64 * value / 100.0).round() as u32));

        let per_instance = self
            .logical
            .as_ref()
            .and_then(|counter| counter.get_instance_values().ok())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(name, value)| {
                let (group, number) = parse_processor_instance(&name)?;
                Some(((group, number), finite_percent(Some(value))?))
            })
            .collect::<BTreeMap<_, _>>();

        let per_logical = self
            .topology
            .cores
            .iter()
            .enumerate()
            .flat_map(|(core_index, logicals)| {
                let per_core = per_instance.clone();
                logicals.iter().map(move |&index| {
                    let group = index / usize::BITS;
                    let number = index % usize::BITS;
                    LogicalProcessorUsage {
                        index,
                        physical_core_index: core_index as u32,
                        utilization: per_core.get(&(group, number)).copied(),
                    }
                })
            })
            .collect::<Vec<_>>();
        let active_cores = self
            .topology
            .cores
            .iter()
            .enumerate()
            .filter(|(core, _)| {
                per_logical
                    .iter()
                    .filter(|logical| logical.physical_core_index == *core as u32)
                    .filter_map(|logical| logical.utilization)
                    .any(|usage| usage > 5.0)
            })
            .count() as u32;

        Ok(CpuSnapshot {
            name: self.name.clone(),
            vendor: self.vendor.clone(),
            total_utilization: total,
            physical_cores: self.topology.cores.len() as u32,
            logical_processors: self.topology.logical_count,
            sockets: self.topology.sockets,
            active_cores,
            per_logical,
            base_clock_mhz: self.base_clock_mhz,
            current_clock_mhz,
        })
    }
}

fn finite_percent(value: Option<f64>) -> Option<f32> {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 100.0) as f32)
}

fn parse_processor_instance(value: &str) -> Option<(u32, u32)> {
    let (group, number) = value.split_once(',')?;
    Some((group.parse().ok()?, number.parse().ok()?))
}

fn read_topology() -> Result<Topology, CollectorError> {
    let buffer = processor_information(RelationAll)?;
    let mut cores = Vec::new();
    let mut logical_count = 0;
    let mut sockets = 0;
    let mut offset = 0;
    while offset < buffer.len() {
        // SAFETY: Windows returns aligned variable-length records with a valid Size.
        let info = unsafe {
            &*buffer
                .as_ptr()
                .add(offset)
                .cast::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>()
        };
        if info.Size == 0 {
            break;
        }
        if info.Relationship == RelationProcessorCore
            || info.Relationship == RelationProcessorPackage
        {
            // SAFETY: the Relationship discriminator selects the Processor union member.
            let processor = unsafe { &info.Anonymous.Processor };
            let indices = processor_indices(info, processor.GroupCount, &processor.GroupMask[0]);
            if info.Relationship == RelationProcessorPackage {
                sockets += 1;
            } else {
                logical_count += indices.len() as u32;
                cores.push(indices);
            }
        }
        offset += info.Size as usize;
    }
    if cores.is_empty() {
        return Err(CollectorError::Unavailable(
            "Windows returned no physical processor cores".into(),
        ));
    }
    Ok(Topology {
        cores,
        logical_count,
        sockets,
    })
}

fn processor_indices(
    info: &SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    group_count: u16,
    first: &windows::Win32::System::SystemInformation::GROUP_AFFINITY,
) -> Vec<u32> {
    // SAFETY: GroupMask is a flexible array with GroupCount entries in the record.
    let masks = unsafe { std::slice::from_raw_parts(first, group_count as usize) };
    let record_end = info as *const _ as usize + info.Size as usize;
    let mut indices = Vec::new();
    for affinity in masks {
        if affinity as *const windows::Win32::System::SystemInformation::GROUP_AFFINITY as usize
            + size_of::<windows::Win32::System::SystemInformation::GROUP_AFFINITY>()
            > record_end
        {
            break;
        }
        for bit in 0..usize::BITS {
            if affinity.Mask & (1_usize << bit) != 0 {
                indices.push(affinity.Group as u32 * usize::BITS + bit);
            }
        }
    }
    indices
}

fn processor_information(
    relationship: LOGICAL_PROCESSOR_RELATIONSHIP,
) -> Result<Vec<u8>, CollectorError> {
    let mut length = 0;
    let error = unsafe { GetLogicalProcessorInformationEx(relationship, None, &mut length) }
        .expect_err("the sizing call must request a buffer");
    if error.code() != ERROR_INSUFFICIENT_BUFFER.to_hresult() || length == 0 {
        return Err(CollectorError::Windows(format!(
            "processor topology sizing failed: {error}"
        )));
    }
    let mut buffer = vec![0_u8; length as usize];
    unsafe {
        GetLogicalProcessorInformationEx(
            relationship,
            Some(buffer.as_mut_ptr().cast()),
            &mut length,
        )
    }
    .map_err(|error| CollectorError::Windows(format!("processor topology failed: {error}")))?;
    Ok(buffer)
}

fn read_identity() -> (String, String, Option<u32>) {
    let wmi = wmi::WMIConnection::new()
        .and_then(|connection| connection.query::<WmiProcessor>())
        .ok()
        .and_then(|rows| rows.into_iter().next());
    let registry_name = registry_processor_name();
    let name = wmi
        .as_ref()
        .and_then(|cpu| clean(&cpu.name))
        .or(registry_name)
        .unwrap_or_else(|| "N/A".into());
    let vendor = wmi
        .as_ref()
        .and_then(|cpu| clean(&cpu.manufacturer))
        .map(|vendor| match vendor.as_str() {
            "AuthenticAMD" => "AMD".into(),
            "GenuineIntel" => "Intel".into(),
            other => other.into(),
        })
        .unwrap_or_else(|| "N/A".into());
    let base_clock = wmi.and_then(|cpu| cpu.max_clock_speed);
    (name, vendor, base_clock)
}

fn registry_processor_name() -> Option<String> {
    let subkey = wide(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0");
    let value = wide("ProcessorNameString");
    let mut bytes = 0;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut bytes),
        )
    };
    if status != WIN32_ERROR(0) || bytes < 2 {
        return None;
    }
    let mut buffer = vec![0_u16; bytes as usize / 2];
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status != WIN32_ERROR(0) {
        return None;
    }
    String::from_utf16(&buffer)
        .ok()
        .map(|name| name.trim_end_matches('\0').trim().to_owned())
        .filter(|name| !name.is_empty())
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::parse_processor_instance;

    #[test]
    fn parses_processor_groups() {
        assert_eq!(parse_processor_instance("0,11"), Some((0, 11)));
        assert_eq!(parse_processor_instance("1,3"), Some((1, 3)));
        assert_eq!(parse_processor_instance("_Total"), None);
    }
}
