use thiserror::Error;

/// Reserved for upcoming Windows collectors (PDH, Win32, vendor APIs).
#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum CollectorError {
    #[error("PDH error: {0}")]
    Pdh(String),
    #[error("Windows API error: {0}")]
    Windows(String),
    #[error("collector not available: {0}")]
    Unavailable(String),
}
