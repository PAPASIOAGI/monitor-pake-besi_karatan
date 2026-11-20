use std::time::SystemTime;

use crate::domain::{CollectorWarning, DataStatus, StorageSnapshot, SystemSnapshot};
use crate::error::CollectorError;
use crate::monitoring::collector::Collector;
use crate::platform::windows::sensors::cpu_temp::create_cpu_temperature_sensor;

use super::volume::VolumeCollector;
use super::{
    cpu::CpuCollector, disk::DiskCollector, gpu::GpuCollector, memory::MemoryCollector,
    process::ProcessCollector,
};

pub struct WindowsCollector {
    cpu: Option<CpuCollector>,
    memory: Option<MemoryCollector>,
    gpu: Option<GpuCollector>,
    volume: Option<VolumeCollector>,
    disk: Option<DiskCollector>,
    process: Option<ProcessCollector>,
    cpu_temp_sensor: Option<Box<dyn crate::platform::windows::sensors::TemperatureSensor>>,
    warnings: Vec<CollectorWarning>,
}

impl WindowsCollector {
    pub fn new(active_core_threshold: f32) -> Result<Self, CollectorError> {
        let mut warnings = Vec::new();
        let cpu = initialize(
            "CPU",
            CpuCollector::new(active_core_threshold),
            &mut warnings,
        );
        let memory = initialize("Memory", MemoryCollector::new(), &mut warnings);
        let gpu = initialize("GPU", GpuCollector::new(), &mut warnings);
        let volume = initialize("Volumes", VolumeCollector::new(), &mut warnings);
        let disk = initialize("Physical disks", DiskCollector::new(), &mut warnings);
        let process = initialize("Processes", ProcessCollector::new(), &mut warnings);

        // Initialize CPU temperature sensor
        let cpu_temp_sensor = create_cpu_temperature_sensor();
        if !cpu_temp_sensor.name().contains("Unavailable") {
            // Sensor is available
        } else {
            warnings.push(CollectorWarning {
                source: "CPU Temperature".into(),
                message: "No CPU temperature sensor available".into(),
            });
        }

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
            process,
            cpu_temp_sensor: Some(cpu_temp_sensor),
            warnings,
        })
    }
}

impl Collector for WindowsCollector {
    fn collect(&mut self, sequence: u64) -> Result<SystemSnapshot, CollectorError> {
        let mut cpu = collect_part(
            "CPU",
            &mut self.cpu,
            CpuCollector::collect,
            &mut self.warnings,
        )
        .unwrap_or_default();

        // Read CPU temperature
        if let Some(sensor) = &mut self.cpu_temp_sensor
            && let Ok(Some(temp)) = sensor.read_temperature()
        {
            cpu.temperature_celsius = Some(temp.0);
        }

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
        let processes = collect_part(
            "Processes",
            &mut self.process,
            ProcessCollector::collect,
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
            processes,
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
