use std::time::SystemTime;

use super::{
    CpuSnapshot, DataStatus, GpuSnapshot, MemorySnapshot, ProcessSnapshot, StorageSnapshot,
};

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub sequence: u64,
    #[allow(dead_code)] // used by upcoming stale-detection & timestamp UI
    pub captured_at: SystemTime,
    pub status: DataStatus,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub gpus: Vec<GpuSnapshot>,
    pub storage: StorageSnapshot,
    pub processes: ProcessSnapshot,
    #[allow(dead_code)] // populated once real collectors emit warnings
    pub warnings: Vec<CollectorWarning>,
}

#[allow(dead_code)] // constructed by real collectors in later stages
#[derive(Debug, Clone)]
pub struct CollectorWarning {
    pub source: String,
    pub message: String,
}
