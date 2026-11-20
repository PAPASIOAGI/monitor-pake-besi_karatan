#[derive(Debug, Clone, Default)]
pub struct ProcessSnapshot {
    pub processes: Vec<ProcessInfo>,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: Option<f32>,
    pub memory_working_set: Option<u64>,
    pub memory_private: Option<u64>,
    pub memory_page_file: Option<u64>,
    // Internal tracking fields for CPU delta calculation
    #[allow(dead_code)]
    last_cpu_time: Option<u64>,
    #[allow(dead_code)]
    last_sample_time: Option<std::time::Instant>,
}

impl ProcessInfo {
    pub fn new(pid: u32, name: String) -> Self {
        Self {
            pid,
            name,
            cpu_usage: None,
            memory_working_set: None,
            memory_private: None,
            memory_page_file: None,
            last_cpu_time: None,
            last_sample_time: None,
        }
    }

    pub fn update_cpu(&mut self, total_cpu_time: u64, now: std::time::Instant) {
        if let (Some(last_cpu), Some(last_time)) = (self.last_cpu_time, self.last_sample_time) {
            let elapsed = now.duration_since(last_time).as_secs_f64();
            if elapsed > 0.0 {
                let cpu_delta = total_cpu_time.saturating_sub(last_cpu) as f64;
                // Convert FILETIME (100ns units) to seconds
                let cpu_seconds = cpu_delta * 1e-7;
                self.cpu_usage = Some((cpu_seconds / elapsed * 100.0).clamp(0.0, 100.0) as f32);
            }
        }
        self.last_cpu_time = Some(total_cpu_time);
        self.last_sample_time = Some(now);
    }
}
