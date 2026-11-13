#[derive(Debug, Clone, Default)]
pub struct StorageSnapshot {
    pub physical_disks: Vec<PhysicalDisk>,
    pub volumes: Vec<Volume>,
}

#[derive(Debug, Clone)]
pub struct PhysicalDisk {
    pub device_number: u32,
    pub model: String,
    pub total_bytes: u64,
    pub media_type: MediaType,
    pub bus_type: BusType,
    pub read_bytes_per_sec: Option<u64>,
    pub write_bytes_per_sec: Option<u64>,
    pub active_time_percent: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct Volume {
    pub mount_point: String,
    pub label: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    #[allow(dead_code)] // consumed by storage detail UI in a later stage
    pub kind: VolumeKind,
}

/// Variants beyond `Local` will be produced by the Win32 volume enumerator.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeKind {
    Local,
    Removable,
    Network,
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Ssd,
    Hdd,
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    Nvme,
    Sata,
    Usb,
    Unknown,
}
