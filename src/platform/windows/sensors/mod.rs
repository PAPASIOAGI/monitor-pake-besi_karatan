use crate::error::CollectorError;

pub mod cpu_temp;
pub mod gpu_temp;

/// Temperature reading in Celsius
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemperatureCelsius(pub f32);

#[allow(dead_code)] // Reserved for future use in conversion utilities
impl TemperatureCelsius {
    pub fn to_fahrenheit(self) -> f32 {
        self.0 * 9.0 / 5.0 + 32.0
    }
}

/// Trait for temperature sensors
pub trait TemperatureSensor: Send + Sync {
    fn name(&self) -> &str;
    fn read_temperature(&mut self) -> Result<Option<TemperatureCelsius>, CollectorError>;
}

/// Sensor availability status
#[allow(dead_code)] // Reserved for future use in sensor state reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorStatus {
    Available,
    Unavailable,
    Unsupported,
    PermissionDenied,
    Error,
}
