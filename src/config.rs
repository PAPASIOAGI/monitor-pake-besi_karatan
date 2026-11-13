use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_INTERVAL_MS: u64 = 500;
const DEFAULT_HISTORY_SECS: u64 = 60;
const DEFAULT_ACTIVE_CORE_THRESHOLD: f32 = 5.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub sampling_interval_ms: u64,
    pub history_duration_secs: u64,
    pub active_core_threshold_percent: f32,
    pub theme: ThemeChoice,
    pub temperature_unit: TemperatureUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sampling_interval_ms: DEFAULT_INTERVAL_MS,
            history_duration_secs: DEFAULT_HISTORY_SECS,
            active_core_threshold_percent: DEFAULT_ACTIVE_CORE_THRESHOLD,
            theme: ThemeChoice::Dark,
            temperature_unit: TemperatureUnit::Celsius,
        }
    }
}

impl AppConfig {
    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str::<AppConfig>(&text) {
                Ok(cfg) => {
                    let mut cfg = cfg;
                    cfg.validate();
                    cfg
                }
                Err(e) => {
                    tracing::warn!("config parse failed, using defaults: {e}");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, text)
    }

    pub fn validate(&mut self) {
        if !(100..=60_000).contains(&self.sampling_interval_ms) {
            self.sampling_interval_ms = DEFAULT_INTERVAL_MS;
        }
        if !(10..=3600).contains(&self.history_duration_secs) {
            self.history_duration_secs = DEFAULT_HISTORY_SECS;
        }
        if !(0.0..=100.0).contains(&self.active_core_threshold_percent)
            || self.active_core_threshold_percent.is_nan()
        {
            self.active_core_threshold_percent = DEFAULT_ACTIVE_CORE_THRESHOLD;
        }
    }

    fn config_path() -> PathBuf {
        directories::ProjectDirs::from("com", "pcmonitor", "PCMonitor")
            .map(|d| d.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }
}
