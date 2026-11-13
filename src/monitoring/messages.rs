use std::time::Duration;

use crate::domain::SystemSnapshot;

#[derive(Debug)]
pub enum MonitorCommand {
    SetInterval(Duration),
    SetPaused(bool),
    Shutdown,
}

#[derive(Debug)]
pub enum MonitorEvent {
    Snapshot(Box<SystemSnapshot>),
}
