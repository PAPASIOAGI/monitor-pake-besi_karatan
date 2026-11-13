use std::time::Duration;

use crate::domain::SystemSnapshot;

#[derive(Debug)]
pub enum MonitorCommand {
    SetInterval(Duration),
    Shutdown,
}

#[derive(Debug)]
pub enum MonitorEvent {
    Snapshot(Box<SystemSnapshot>),
}
