use std::time::Duration;

use eframe::egui;

use crate::config::AppConfig;
use crate::domain::SystemSnapshot;
use crate::monitoring::{HistoryBuffer, MonitoringService};
use crate::ui;

pub struct MonitorApp {
    config: AppConfig,
    service: MonitoringService,
    latest: Option<SystemSnapshot>,
    history: AppHistory,
    active_page: ui::Page,
    paused: bool,
}

struct AppHistory {
    cpu: HistoryBuffer,
    ram: HistoryBuffer,
    gpu: HistoryBuffer,
}

impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: AppConfig) -> Self {
        apply_theme(&cc.egui_ctx, &config);
        let interval = Duration::from_millis(config.sampling_interval_ms);
        let capacity =
            (config.history_duration_secs * 1000 / config.sampling_interval_ms.max(1)) as usize;
        Self {
            service: MonitoringService::start(interval),
            config,
            latest: None,
            history: AppHistory {
                cpu: HistoryBuffer::new(capacity),
                ram: HistoryBuffer::new(capacity),
                gpu: HistoryBuffer::new(capacity),
            },
            active_page: ui::Page::Overview,
            paused: false,
        }
    }
}

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(snapshot) = self.service.latest_snapshot() {
            self.history.cpu.push(snapshot.cpu.total_utilization);
            self.history.ram.push(snapshot.memory.usage_percent());
            self.history.gpu.push(
                snapshot
                    .gpus
                    .first()
                    .map(|g| g.total_utilization)
                    .unwrap_or(0.0),
            );
            self.latest = Some(snapshot);
        }

        let actions = ui::render(
            ctx,
            &mut self.active_page,
            self.latest.as_ref(),
            &self.history.cpu,
            &self.history.ram,
            &self.history.gpu,
            &mut self.config,
            self.paused,
        );
        if let Some(ms) = actions.new_interval_ms {
            self.service.set_interval(Duration::from_millis(ms));
        }
        if let Some(paused) = actions.paused {
            self.paused = paused;
            self.service.set_paused(paused);
        }
        if actions.theme_changed {
            apply_theme(ctx, &self.config);
        }

        ctx.request_repaint_after(Duration::from_millis(100));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.service.shutdown();
        let _ = self.config.save();
    }
}

fn apply_theme(ctx: &egui::Context, config: &AppConfig) {
    ui::apply_theme(ctx, config.theme == crate::config::ThemeChoice::Light);
}
