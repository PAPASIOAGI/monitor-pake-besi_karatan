use crate::domain::SystemSnapshot;
use crate::error::CollectorError;

pub trait Collector: Send {
    fn collect(&mut self, sequence: u64) -> Result<SystemSnapshot, CollectorError>;
}
