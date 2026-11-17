#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Fields populated by Windows memory collector; UI renders subset for MVP
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub commit_used_bytes: Option<u64>,
    pub commit_limit_bytes: Option<u64>,
    pub page_file_used_bytes: Option<u64>,
    pub page_file_total_bytes: Option<u64>,
    pub cached_bytes: Option<u64>,
    pub modules: Vec<MemoryModule>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields populated by Windows memory collector; UI renders subset for MVP
pub struct MemoryModule {
    pub slot: String,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub serial_number: Option<String>,
    pub capacity_bytes: u64,
    pub configured_speed_mt_s: Option<u32>,
    pub rated_speed_mt_s: Option<u32>,
    pub memory_type: String,
    pub form_factor: String,
    pub cas_latency: Option<u32>,
}

impl MemorySnapshot {
    pub fn usage_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
    }
}
