#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub commit_used_bytes: u64,
    pub commit_limit_bytes: u64,
    pub page_file_used_bytes: Option<u64>,
    pub page_file_total_bytes: Option<u64>,
    pub cached_bytes: Option<u64>,
}

impl MemorySnapshot {
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
    }
}
