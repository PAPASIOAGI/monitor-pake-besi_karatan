use std::collections::BTreeMap;

use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ADAPTER_DESC1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1,
};

use crate::domain::{GpuEngine, GpuEngineKind, GpuSnapshot};
use crate::error::CollectorError;
use crate::platform::windows::pdh::{PdhCounter, PdhQuery};
use crate::platform::windows::sensors::TemperatureSensor;
use crate::platform::windows::sensors::gpu_temp::GpuTemperatureSensor;

struct Adapter {
    name: String,
    vendor: String,
    vendor_id: u32,
    device_id: u32,
    luid: i64,
    dedicated_total: u64,
    shared_total: u64,
    temp_sensor: Option<GpuTemperatureSensor>,
}

pub struct GpuCollector {
    query: PdhQuery,
    adapters: Vec<Adapter>,
    engines: Option<PdhCounter>,
    dedicated: Option<PdhCounter>,
    shared: Option<PdhCounter>,
}

impl GpuCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let adapters = enumerate_adapters()?;
        let query = PdhQuery::new()?;
        Ok(Self {
            engines: query
                .add_counter(r"\GPU Engine(*)\Utilization Percentage")
                .ok(),
            dedicated: query
                .add_counter(r"\GPU Adapter Memory(*)\Dedicated Usage")
                .ok(),
            shared: query
                .add_counter(r"\GPU Adapter Memory(*)\Shared Usage")
                .ok(),
            query,
            adapters,
        })
    }

    pub fn collect(&mut self) -> Result<Vec<GpuSnapshot>, CollectorError> {
        self.query.collect()?;
        let engine_values = self
            .engines
            .as_ref()
            .and_then(|counter| counter.get_instance_values().ok())
            .unwrap_or_default();
        let dedicated_values = self
            .dedicated
            .as_ref()
            .and_then(|counter| counter.get_instance_values().ok())
            .unwrap_or_default();
        let shared_values = self
            .shared
            .as_ref()
            .and_then(|counter| counter.get_instance_values().ok())
            .unwrap_or_default();

        Ok(self
            .adapters
            .iter_mut()
            .map(|adapter| {
                let (engines, total) = aggregate_engines(&engine_values, adapter.luid);
                let temperature = adapter
                    .temp_sensor
                    .as_mut()
                    .and_then(|sensor| sensor.read_temperature().ok().flatten())
                    .map(|t| t.0);
                GpuSnapshot {
                    name: adapter.name.clone(),
                    vendor: adapter.vendor.clone(),
                    vendor_id: adapter.vendor_id,
                    device_id: adapter.device_id,
                    board_vendor: None,
                    memory_vendor: None,
                    total_utilization: total,
                    engines,
                    dedicated_vram_total_bytes: Some(adapter.dedicated_total),
                    dedicated_vram_used_bytes: memory_usage(&dedicated_values, adapter.luid),
                    shared_memory_total_bytes: Some(adapter.shared_total),
                    shared_memory_used_bytes: memory_usage(&shared_values, adapter.luid),
                    temperature_celsius: temperature,
                    core_clock_mhz: None,
                    memory_clock_mhz: None,
                    power_watts: None,
                    fan_rpm: None,
                }
            })
            .collect())
    }
}

fn enumerate_adapters() -> Result<Vec<Adapter>, CollectorError> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }
        .map_err(|error| CollectorError::Windows(format!("CreateDXGIFactory1: {error}")))?;
    let mut adapters = Vec::new();
    for index in 0.. {
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
            break;
        };
        let desc: DXGI_ADAPTER_DESC1 = unsafe { adapter.GetDesc1() }
            .map_err(|error| CollectorError::Windows(format!("DXGI GetDesc1: {error}")))?;
        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        let description = desc
            .Description
            .split(|character| *character == 0)
            .next()
            .unwrap_or_default();
        let luid = ((desc.AdapterLuid.HighPart as i64) << 32) | desc.AdapterLuid.LowPart as i64;
        let temp_sensor = GpuTemperatureSensor::new(luid, desc.VendorId);
        adapters.push(Adapter {
            name: String::from_utf16_lossy(description),
            vendor: vendor_name(desc.VendorId).into(),
            vendor_id: desc.VendorId,
            device_id: desc.DeviceId,
            luid,
            dedicated_total: desc.DedicatedVideoMemory as u64,
            shared_total: desc.SharedSystemMemory as u64,
            temp_sensor: Some(temp_sensor),
        });
    }
    Ok(adapters)
}

fn vendor_name(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x10DE => "NVIDIA",
        0x1002 => "AMD",
        0x8086 => "Intel",
        0x13B5 => "ARM",
        _ => "Unknown",
    }
}

fn instance_luid(instance: &str) -> Option<i64> {
    let start = instance.find("luid_")? + 5;
    let rest = &instance[start..];
    // Format: high_low_...
    let parts: Vec<&str> = rest.split('_').collect();
    if parts.len() < 2 {
        return None;
    }
    let high = i64::from_str_radix(parts[0].strip_prefix("0x")?, 16).ok()?;
    let low = i64::from_str_radix(parts[1].strip_prefix("0x")?, 16).ok()?;
    Some((high << 32) | low)
}

fn aggregate_engines(values: &[(String, f64)], luid: i64) -> (Vec<GpuEngine>, Option<f32>) {
    let mut totals = BTreeMap::<GpuEngineKind, f64>::new();
    for (instance, value) in values {
        if instance_luid(instance) != Some(luid) || !value.is_finite() {
            continue;
        }
        let kind = engine_kind(instance);
        *totals.entry(kind).or_default() += value.clamp(0.0, 100.0);
    }
    let total = if totals.is_empty() {
        None
    } else {
        Some(
            totals
                .values()
                .fold(0.0_f64, |maximum, value| maximum.max(*value))
                .clamp(0.0, 100.0) as f32,
        )
    };
    let engines = totals
        .into_iter()
        .map(|(kind, value)| GpuEngine {
            kind,
            utilization: value.clamp(0.0, 100.0) as f32,
        })
        .collect();
    (engines, total)
}

fn engine_kind(instance: &str) -> GpuEngineKind {
    let marker = "engtype_";
    let kind = instance
        .split(marker)
        .nth(1)
        .unwrap_or_default()
        .split('_')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match kind.as_str() {
        "3d" | "graphics" => GpuEngineKind::ThreeD,
        "compute" => GpuEngineKind::Compute,
        "copy" => GpuEngineKind::Copy,
        "videodecode" => GpuEngineKind::VideoDecode,
        "videoencode" => GpuEngineKind::VideoEncode,
        "videoprocessing" => GpuEngineKind::VideoProcessing,
        _ => GpuEngineKind::Other,
    }
}

fn memory_usage(values: &[(String, f64)], luid: i64) -> Option<u64> {
    let matching = values
        .iter()
        .filter(|(instance, value)| instance_luid(instance) == Some(luid) && value.is_finite())
        .map(|(_, value)| value.max(0.0) as u64)
        .collect::<Vec<_>>();
    (!matching.is_empty()).then(|| matching.into_iter().sum())
}

#[cfg(test)]
mod tests {
    use super::{aggregate_engines, instance_luid};
    use crate::domain::GpuEngineKind;

    #[test]
    fn parses_signed_luid() {
        let instance = "pid_1_luid_0x00000000_0x0001c803_phys_0_eng_0_engtype_3D";
        assert_eq!(instance_luid(instance), Some(0x1c803));
    }

    #[test]
    fn filters_gpu_and_uses_busiest_engine() {
        let values = vec![
            (
                "pid_1_luid_0x00000000_0x0001c803_eng_0_engtype_3D".into(),
                42.0,
            ),
            (
                "pid_2_luid_0x00000000_0x0001c803_eng_1_engtype_Copy".into(),
                12.0,
            ),
            (
                "pid_1_luid_0x00000001_0x0001c803_eng_0_engtype_3D".into(),
                90.0,
            ),
        ];
        let (engines, total) = aggregate_engines(&values, 0x1c803);
        assert_eq!(total, Some(42.0));
        assert_eq!(engines.len(), 2);
        assert_eq!(engines[0].kind, GpuEngineKind::ThreeD);
    }
}
