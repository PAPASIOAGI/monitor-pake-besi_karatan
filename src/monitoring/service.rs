use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, bounded};

use crate::domain::SystemSnapshot;

use super::collector::Collector;
use super::messages::{MonitorCommand, MonitorEvent};
use super::mock::MockCollector;

pub struct MonitoringService {
    command_tx: Sender<MonitorCommand>,
    event_rx: Receiver<MonitorEvent>,
    worker: Option<JoinHandle<()>>,
}

impl MonitoringService {
    pub fn start(initial_interval: Duration) -> Self {
        Self::with_collector(Box::new(MockCollector::new()), initial_interval)
    }

    pub fn with_collector(collector: Box<dyn Collector>, initial_interval: Duration) -> Self {
        let (command_tx, command_rx) = bounded::<MonitorCommand>(8);
        let (event_tx, event_rx) = bounded::<MonitorEvent>(2);
        let drain_rx = event_rx.clone();

        let worker = thread::Builder::new()
            .name("monitor-worker".into())
            .spawn(move || run_worker(collector, command_rx, event_tx, drain_rx, initial_interval))
            .expect("failed to spawn monitor worker");

        Self {
            command_tx,
            event_rx,
            worker: Some(worker),
        }
    }

    pub fn latest_snapshot(&self) -> Option<SystemSnapshot> {
        let mut latest = None;
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                MonitorEvent::Snapshot(snap) => latest = Some(*snap),
            }
        }
        latest
    }

    pub fn set_interval(&self, interval: Duration) {
        let _ = self
            .command_tx
            .try_send(MonitorCommand::SetInterval(interval));
    }

    pub fn set_paused(&self, paused: bool) {
        let _ = self.command_tx.try_send(MonitorCommand::SetPaused(paused));
    }

    pub fn shutdown(&mut self) {
        let _ = self.command_tx.send(MonitorCommand::Shutdown);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for MonitoringService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_worker(
    mut collector: Box<dyn Collector>,
    command_rx: Receiver<MonitorCommand>,
    event_tx: Sender<MonitorEvent>,
    drain_rx: Receiver<MonitorEvent>,
    initial_interval: Duration,
) {
    let mut interval = initial_interval;
    let mut paused = false;
    let mut sequence: u64 = 0;
    let mut next_tick = Instant::now();

    loop {
        let now = Instant::now();
        if now >= next_tick && !paused {
            sequence = sequence.wrapping_add(1);
            match collector.collect(sequence) {
                Ok(snapshot) => {
                    let mut event = MonitorEvent::Snapshot(Box::new(snapshot));
                    // Bounded channel: if full, drop the oldest snapshot and retry once.
                    if let Err(crossbeam_channel::TrySendError::Full(returned)) =
                        event_tx.try_send(event)
                    {
                        let _ = drain_rx.try_recv();
                        event = returned;
                        let _ = event_tx.try_send(event);
                    }
                }
                Err(e) => {
                    tracing::warn!("collector error: {e}");
                }
            }
            next_tick = now + interval;
        }

        match command_rx.recv_timeout(Duration::from_millis(10)) {
            Ok(MonitorCommand::SetInterval(d)) => {
                interval = d;
                next_tick = Instant::now();
            }
            Ok(MonitorCommand::SetPaused(value)) => {
                paused = value;
                next_tick = Instant::now();
            }
            Ok(MonitorCommand::Shutdown) => break,
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
        }
    }
}
