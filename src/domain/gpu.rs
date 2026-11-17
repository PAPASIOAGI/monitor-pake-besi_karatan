#[derive(Debug, Clone, Default)]
#[allow(dead_code)] // Fields populated by Windows GPU collector; UI reads subset for MVP
pub struct GpuSnapshot {
    pub name: String,
    pub vendor: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub board_vendor: Option<String>,
    pub memory_vendor: Option<String>,
    pub total_utilization: Option<f32>,
    pub engines: Vec<GpuEngine>,
    pub dedicated_vram_total_bytes: Option<u64>,
    pub dedicated_vram_used_bytes: Option<u64>,
    pub shared_memory_total_bytes: Option<u64>,
    pub shared_memory_used_bytes: Option<u64>,
    pub temperature_celsius: Option<f32>,
    pub core_clock_mhz: Option<u32>,
    pub memory_clock_mhz: Option<u32>,
    pub power_watts: Option<f32>,
    pub fan_rpm: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct GpuEngine {
    pub kind: GpuEngineKind,
    pub utilization: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GpuEngineKind {
    ThreeD,
    Compute,
    Copy,
    VideoDecode,
    VideoEncode,
    VideoProcessing,
    Other,
}

impl GpuEngineKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ThreeD => "3D",
            Self::Compute => "Compute",
            Self::Copy => "Copy",
            Self::VideoDecode => "Video Decode",
            Self::VideoEncode => "Video Encode",
            Self::VideoProcessing => "Video Processing",
            Self::Other => "Other",
        }
    }
}
