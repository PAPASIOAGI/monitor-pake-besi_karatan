use serde::Deserialize;
use wmi::WMIConnection;

use super::{TemperatureCelsius, TemperatureSensor};
use crate::error::CollectorError;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WmiThermalZone {
    current_temperature: Option<u32>,
}

/// CPU temperature sensor using WMI thermal zones
pub struct CpuThermalZoneSensor {
    available: bool,
}

impl CpuThermalZoneSensor {
    pub fn new() -> Self {
        // Test if WMI thermal zones are accessible
        let available = WMIConnection::new()
            .and_then(|conn| -> Result<Vec<WmiThermalZone>, _> {
                conn.raw_query("SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature")
            })
            .is_ok();
        Self { available }
    }
}

impl TemperatureSensor for CpuThermalZoneSensor {
    fn name(&self) -> &str {
        "CPU Thermal Zone (WMI)"
    }

    fn read_temperature(&mut self) -> Result<Option<TemperatureCelsius>, CollectorError> {
        if !self.available {
            return Ok(None);
        }

        let conn = WMIConnection::new()
            .map_err(|e| CollectorError::Windows(format!("WMI connection: {e}")))?;
        let results: Vec<WmiThermalZone> = conn
            .query()
            .map_err(|e| CollectorError::Windows(format!("WMI query: {e}")))?;

        // Get the first valid temperature reading
        for zone in results {
            if let Some(temp) = zone.current_temperature {
                // WMI returns temperature in tenths of Kelvin
                let kelvin = temp as f32 / 10.0;
                let celsius = kelvin - 273.15;
                if celsius > -50.0 && celsius < 150.0 {
                    return Ok(Some(TemperatureCelsius(celsius)));
                }
            }
        }

        Ok(None)
    }
}

/// Alternative CPU temperature sensor using OpenHardwareMonitor/LibreHardwareMonitor WMI
pub struct CpuHardwareMonitorSensor {
    available: bool,
}

#[allow(dead_code)] // Used only for WMI query type inference
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WmiSensorTest {
    value: Option<f32>,
}

impl CpuHardwareMonitorSensor {
    pub fn new() -> Self {
        let available = WMIConnection::new()
            .and_then(|conn| -> Result<Vec<WmiSensorTest>, _> {
                conn.raw_query(
                    "SELECT Value FROM Sensor WHERE SensorType='Temperature' AND Name LIKE '%CPU%'",
                )
            })
            .is_ok();
        Self { available }
    }
}

impl TemperatureSensor for CpuHardwareMonitorSensor {
    fn name(&self) -> &str {
        "CPU (Hardware Monitor)"
    }

    fn read_temperature(&mut self) -> Result<Option<TemperatureCelsius>, CollectorError> {
        if !self.available {
            return Ok(None);
        }

        let conn = WMIConnection::new()
            .map_err(|e| CollectorError::Windows(format!("WMI connection: {e}")))?;
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct SensorValue {
            value: Option<f32>,
        }
        let results: Vec<SensorValue> = conn
            .raw_query("SELECT Value FROM Sensor WHERE SensorType='Temperature' AND (Name LIKE '%CPU Package%' OR Name LIKE '%CPU Core%' OR Name LIKE '%CPU%')")
            .map_err(|e| CollectorError::Windows(format!("WMI query: {e}")))?;

        // Return the first valid reading (typically CPU Package)
        for sensor in results {
            if let Some(temp) = sensor.value
                && temp > -50.0
                && temp < 150.0
            {
                return Ok(Some(TemperatureCelsius(temp)));
            }
        }

        Ok(None)
    }
}

pub fn create_cpu_temperature_sensor() -> Box<dyn TemperatureSensor> {
    // Try Hardware Monitor first (more accurate), fall back to thermal zones
    let hm = CpuHardwareMonitorSensor::new();
    if hm.available {
        Box::new(hm)
    } else {
        Box::new(CpuThermalZoneSensor::new())
    }
}
