use std::time::SystemTime;

use crate::domain::{CollectorWarning, DataStatus, StorageSnapshot, SystemSnapshot};
use crate::error::CollectorError;
use crate::monitoring::collector::Collector;

use super::volume::VolumeCollector;
use super::{cpu::CpuCollector, disk::DiskCollector, gpu::GpuCollector, memory::MemoryCollector};

pub struct WindowsCollector {
    cpu: Option<CpuCollector>,
    memory: Option<MemoryCollector>,
    gpu: Option<GpuCollector>,
    volume: Option<VolumeCollector>,
    disk: Option<DiskCollector>,
    warnings: Vec<CollectorWarning>,
}

impl WindowsCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let mut warnings = Vec::new();
        let cpu = initialize("CPU", CpuCollector::new(), &mut warnings);
        let memory = initialize("Memory", MemoryCollector::new(), &mut warnings);
        let gpu = initialize("GPU", GpuCollector::new(), &mut warnings);
        let volume = initialize("Volumes", VolumeCollector::new(), &mut warnings);
        let disk = initialize("Physical disks", DiskCollector::new(), &mut warnings);
        if cpu.is_none() && memory.is_none() {
            return Err(CollectorError::Unavailable(
                "CPU and memory monitoring could not initialize".into(),
            ));
        }
        Ok(Self {
            cpu,
            memory,
            gpu,
            volume,
            disk,
            warnings,
        })
    }
}

impl Collector for WindowsCollector {
    fn collect(&mut self, sequence: u64) -> Result<SystemSnapshot, CollectorError> {
        let cpu = collect_part(
            "CPU",
            &mut self.cpu,
            CpuCollector::collect,
            &mut self.warnings,
        )
        .unwrap_or_default();
        let memory = collect_part(
            "Memory",
            &mut self.memory,
            MemoryCollector::collect,
            &mut self.warnings,
        )
        .unwrap_or_default();
        let gpus = collect_part(
            "GPU",
            &mut self.gpu,
            GpuCollector::collect,
            &mut self.warnings,
        )
        .unwrap_or_default();
        let volumes = collect_part(
            "Volumes",
            &mut self.volume,
            VolumeCollector::collect,
            &mut self.warnings,
        )
        .unwrap_or_default();
        let physical_disks = collect_part(
            "Physical disks",
            &mut self.disk,
            DiskCollector::collect,
            &mut self.warnings,
        )
        .unwrap_or_default();
        let status = if self.cpu.is_none() || self.memory.is_none() {
            DataStatus::Unavailable
        } else {
            DataStatus::Fresh
        };
        Ok(SystemSnapshot {
            sequence,
            captured_at: SystemTime::now(),
            status,
            cpu,
            memory,
            gpus,
            storage: StorageSnapshot {
                physical_disks,
                volumes,
            },
            warnings: self.warnings.clone(),
        })
    }
}

fn initialize<T>(
    source: &str,
    result: Result<T, CollectorError>,
    warnings: &mut Vec<CollectorWarning>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            warnings.push(CollectorWarning {
                source: source.into(),
                message: error.to_string(),
            });
            None
        }
    }
}

fn collect_part<T, R>(
    source: &str,
    collector: &mut Option<T>,
    collect: fn(&mut T) -> Result<R, CollectorError>,
    warnings: &mut Vec<CollectorWarning>,
) -> Option<R> {
    match collect(collector.as_mut()?) {
        Ok(value) => {
            warnings.retain(|warning| warning.source != source);
            Some(value)
        }
        Err(error) => {
            if let Some(warning) = warnings.iter_mut().find(|warning| warning.source == source) {
                warning.message = error.to_string();
            } else {
                warnings.push(CollectorWarning {
                    source: source.into(),
                    message: error.to_string(),
                });
            }
            None
        }
    }
}
