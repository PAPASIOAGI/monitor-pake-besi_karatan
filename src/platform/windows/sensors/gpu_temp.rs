use super::{TemperatureCelsius, TemperatureSensor};
use crate::error::CollectorError;

/// GPU temperature sensor using vendor-specific APIs
pub struct GpuTemperatureSensor {
    #[allow(dead_code)] // Reserved for future vendor-specific initialization
    adapter_luid: i64,
    #[allow(dead_code)] // Reserved for future vendor-specific initialization
    vendor_id: u32,
    sensor_type: GpuSensorType,
    initialized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GpuSensorType {
    Nvml,
    AmdAdl,
    IntelIgcl,
    #[allow(dead_code)] // Reserved for future DXGI-based temperature reading
    DxgiFallback,
    None,
}

impl GpuTemperatureSensor {
    pub fn new(adapter_luid: i64, vendor_id: u32) -> Self {
        let sensor_type = match vendor_id {
            0x10DE => GpuSensorType::Nvml,      // NVIDIA
            0x1002 => GpuSensorType::AmdAdl,    // AMD
            0x8086 => GpuSensorType::IntelIgcl, // Intel
            _ => GpuSensorType::None,
        };
        Self {
            adapter_luid,
            vendor_id,
            sensor_type,
            initialized: false,
        }
    }

    fn try_init(&mut self) -> Result<(), CollectorError> {
        if self.initialized {
            return Ok(());
        }
        self.initialized = true;

        match self.sensor_type {
            GpuSensorType::Nvml => {
                // TODO: Initialize NVML
                // For now, mark as unavailable since we don't want to add NVML dependency yet
                self.sensor_type = GpuSensorType::None;
                Err(CollectorError::Unavailable("NVML not implemented".into()))
            }
            GpuSensorType::AmdAdl => {
                // TODO: Initialize AMD ADL
                self.sensor_type = GpuSensorType::None;
                Err(CollectorError::Unavailable(
                    "AMD ADL not implemented".into(),
                ))
            }
            GpuSensorType::IntelIgcl => {
                // TODO: Initialize Intel IGCL
                self.sensor_type = GpuSensorType::None;
                Err(CollectorError::Unavailable(
                    "Intel IGCL not implemented".into(),
                ))
            }
            GpuSensorType::DxgiFallback => {
                // DXGI doesn't expose temperature directly
                Err(CollectorError::Unavailable(
                    "DXGI doesn't expose temperature".into(),
                ))
            }
            GpuSensorType::None => Err(CollectorError::Unavailable(
                "No GPU temperature sensor available".into(),
            )),
        }
    }
}

impl TemperatureSensor for GpuTemperatureSensor {
    fn name(&self) -> &str {
        match self.sensor_type {
            GpuSensorType::Nvml => "GPU (NVML)",
            GpuSensorType::AmdAdl => "GPU (AMD ADL)",
            GpuSensorType::IntelIgcl => "GPU (Intel IGCL)",
            GpuSensorType::DxgiFallback => "GPU (DXGI)",
            GpuSensorType::None => "GPU (Unavailable)",
        }
    }

    fn read_temperature(&mut self) -> Result<Option<TemperatureCelsius>, CollectorError> {
        self.try_init()?;

        match self.sensor_type {
            GpuSensorType::None => Ok(None),
            _ => Ok(None), // Placeholder for actual implementation
        }
    }
}
