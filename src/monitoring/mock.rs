use std::time::SystemTime;

use crate::domain::*;
use crate::error::CollectorError;

use super::collector::Collector;

pub struct MockCollector {
    logical_count: u32,
    physical_count: u32,
}

impl MockCollector {
    pub fn new() -> Self {
        let logical = 16;
        Self {
            logical_count: logical,
            physical_count: logical / 2,
        }
    }
}

impl Collector for MockCollector {
    fn collect(&mut self, sequence: u64) -> Result<SystemSnapshot, CollectorError> {
        let t = sequence as f32 * 0.5;
        let cpu_total = 25.0 + 20.0 * (t * 0.13).sin() + 5.0 * (t * 0.71).cos();

        let per_logical: Vec<LogicalProcessorUsage> = (0..self.logical_count)
            .map(|i| {
                let phase = i as f32 * 0.4;
                let util = (cpu_total + 15.0 * (t * 0.5 + phase).sin()).clamp(0.0, 100.0);
                LogicalProcessorUsage {
                    index: i,
                    physical_core_index: i / 2,
                    utilization: util,
                }
            })
            .collect();

        let active = (0..self.physical_count)
            .filter(|core| {
                per_logical
                    .iter()
                    .filter(|lp| lp.physical_core_index == *core)
                    .any(|lp| lp.utilization > 5.0)
            })
            .count() as u32;

        let ram_total = 16 * 1024 * 1024 * 1024u64;
        let ram_used = (ram_total as f64 * (0.45 + 0.15 * (t * 0.07).sin() as f64)) as u64;

        let gpu_util = (45.0 + 30.0 * (t * 0.21).sin()).clamp(0.0, 100.0);

        Ok(SystemSnapshot {
            sequence,
            captured_at: SystemTime::now(),
            status: DataStatus::Fresh,
            cpu: CpuSnapshot {
                name: "Mock Processor (16 logical / 8 physical)".into(),
                total_utilization: cpu_total.clamp(0.0, 100.0),
                physical_cores: self.physical_count,
                logical_processors: self.logical_count,
                sockets: 1,
                active_cores: active,
                per_logical,
                base_clock_mhz: Some(3600),
                current_clock_mhz: Some(4200 + (300.0 * (t * 0.3).sin()) as u32),
            },
            memory: MemorySnapshot {
                total_bytes: ram_total,
                used_bytes: ram_used,
                available_bytes: ram_total - ram_used,
                commit_used_bytes: ram_used + 2 * 1024 * 1024 * 1024,
                commit_limit_bytes: ram_total + 8 * 1024 * 1024 * 1024,
                page_file_used_bytes: Some(2 * 1024 * 1024 * 1024),
                page_file_total_bytes: Some(8 * 1024 * 1024 * 1024),
                cached_bytes: Some(3 * 1024 * 1024 * 1024),
            },
            gpus: vec![GpuSnapshot {
                name: "Mock GPU".into(),
                vendor: "MockVendor".into(),
                total_utilization: gpu_util,
                engines: vec![
                    GpuEngine {
                        kind: GpuEngineKind::ThreeD,
                        utilization: gpu_util,
                    },
                    GpuEngine {
                        kind: GpuEngineKind::VideoDecode,
                        utilization: (10.0 + 8.0 * (t * 0.4).cos()).clamp(0.0, 100.0),
                    },
                    GpuEngine {
                        kind: GpuEngineKind::Copy,
                        utilization: (5.0 + 4.0 * (t * 0.6).sin()).clamp(0.0, 100.0),
                    },
                ],
                dedicated_vram_total_bytes: Some(8 * 1024 * 1024 * 1024),
                dedicated_vram_used_bytes: Some(
                    (8.0 * 1024.0 * 1024.0 * 1024.0 * (0.4 + 0.2 * (t * 0.1).sin() as f64)) as u64,
                ),
                shared_memory_total_bytes: Some(8 * 1024 * 1024 * 1024),
                shared_memory_used_bytes: Some(512 * 1024 * 1024),
                temperature_celsius: Some(55.0 + 10.0 * (t * 0.15).sin()),
                core_clock_mhz: Some(1800 + (200.0 * (t * 0.25).cos()) as u32),
                memory_clock_mhz: Some(7000),
                power_watts: Some(120.0 + 40.0 * (t * 0.2).sin()),
                fan_rpm: Some(1400 + (300.0 * (t * 0.18).cos()) as u32),
            }],
            storage: StorageSnapshot {
                physical_disks: vec![PhysicalDisk {
                    device_number: 0,
                    model: "Mock NVMe SSD 1TB".into(),
                    total_bytes: 1_000_204_886_016,
                    media_type: MediaType::Ssd,
                    bus_type: BusType::Nvme,
                    read_bytes_per_sec: Some(
                        (50.0 * 1024.0 * 1024.0 * (1.0 + (t * 0.33).sin() as f64)) as u64,
                    ),
                    write_bytes_per_sec: Some(
                        (20.0 * 1024.0 * 1024.0 * (1.0 + (t * 0.41).cos() as f64)) as u64,
                    ),
                    active_time_percent: Some((15.0 + 12.0 * (t * 0.27).sin()).clamp(0.0, 100.0)),
                }],
                volumes: vec![
                    Volume {
                        mount_point: "C:\\".into(),
                        label: "System".into(),
                        filesystem: "NTFS".into(),
                        total_bytes: 500_107_862_016,
                        used_bytes: 280_000_000_000 + (sequence % 1000) * 1_000_000,
                        available_bytes: 220_107_862_016,
                        kind: VolumeKind::Local,
                    },
                    Volume {
                        mount_point: "D:\\".into(),
                        label: "Data".into(),
                        filesystem: "NTFS".into(),
                        total_bytes: 500_097_024_000,
                        used_bytes: 150_000_000_000,
                        available_bytes: 350_097_024_000,
                        kind: VolumeKind::Local,
                    },
                ],
            },
            warnings: vec![],
        })
    }
}
