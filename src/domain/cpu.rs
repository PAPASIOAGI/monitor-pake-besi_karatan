#[derive(Debug, Clone, Default)]
pub struct CpuSnapshot {
    pub name: String,
    pub total_utilization: f32,
    pub physical_cores: u32,
    pub logical_processors: u32,
    pub sockets: u32,
    pub active_cores: u32,
    pub per_logical: Vec<LogicalProcessorUsage>,
    pub base_clock_mhz: Option<u32>,
    pub current_clock_mhz: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LogicalProcessorUsage {
    pub index: u32,
    pub physical_core_index: u32,
    pub utilization: f32,
}
