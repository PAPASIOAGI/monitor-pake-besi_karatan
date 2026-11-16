use std::time::SystemTime;

use crate::domain::{
    cpu::CpuSnapshot,
    gpu::GpuSnapshot,
    memory::MemorySnapshot,
    storage::{StorageSnapshot, PhysicalDisk, Volume},
    status::DataStatus,
    SystemSnapshot,
};
use crate::error::CollectorError;
use crate::monitoring::collector::Collector;

pub struct WindowsCollector {
    cpu_collector: crate::platform::windows::cpu::CpuCollector,
    memory_collector: crate::platform::windows::memory::MemoryCollector,
    gpu_collector: crate::platform::windows::gpu::GpuCollector,
    volume_collector: crate::platform::windows::volume::VolumeCollector,
    disk_collector: crate::platform::windows::disk::DiskCollector,
    sequence: u64,
}

impl WindowsCollector {
    pub fn new() -> Result<Self, CollectorError> {
        let cpu_collector = crate::platform::windows::cpu::CpuCollector::new()?;
        let memory_collector = crate::platform::windows::memory::MemoryCollector::new()?;
        let gpu_collector = crate::platform::windows::gpu::GpuCollector::new()?;
        let volume_collector = crate::platform::windows::volume::VolumeCollector::new()?;
        let disk_collector = crate::platform::windows::disk::DiskCollector::new()?;
        
        Ok(Self {
            cpu_collector,
            memory_collector,
            gpu_collector,
            volume_collector,
            disk_collector,
            sequence: 0,
        })
    }
}

impl Collector for WindowsCollector {
    fn collect(&mut self, sequence: u64) -> Result<SystemSnapshot, CollectorError> {
        self.sequence = sequence;
        
        let cpu = self.cpu_collector.collect()?;
        let memory = self.memory_collector.collect()?;
        let gpus = self.gpu_collector.collect()?;
        let volumes = self.volume_collector.collect()?;
        let physical_disks = self.disk_collector.collect()?;
        
        let storage = StorageSnapshot {
            physical_disks,
            volumes,
        };
        
        Ok(SystemSnapshot {
            sequence,
            captured_at: SystemTime::now(),
            status: DataStatus::Fresh,
            cpu,
            memory,
            gpus,
            storage,
            warnings: vec![],
        })
    }
}