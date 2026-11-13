mod cpu_page;
mod dashboard;
mod gpu_page;
mod memory_page;
mod settings_page;
mod storage_page;
mod widgets;

use egui::{Context, SidePanel, TopBottomPanel};

use crate::config::AppConfig;
use crate::domain::SystemSnapshot;
use crate::monitoring::HistoryBuffer;

/// Signals the app wants to propagate back to the monitoring service.
pub struct UiActions {
    pub new_interval_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Cpu,
    Memory,
    Gpu,
    Storage,
    Settings,
}

impl Page {
    fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
            Self::Gpu => "GPU",
            Self::Storage => "Storage",
            Self::Settings => "Settings",
        }
    }

    const ALL: [Page; 6] = [
        Page::Dashboard,
        Page::Cpu,
        Page::Memory,
        Page::Gpu,
        Page::Storage,
        Page::Settings,
    ];
}

pub fn render(
    ctx: &Context,
    active_page: &mut Page,
    snapshot: Option<&SystemSnapshot>,
    cpu_history: &HistoryBuffer,
    ram_history: &HistoryBuffer,
    gpu_history: &HistoryBuffer,
    config: &mut AppConfig,
) -> UiActions {
    let mut actions = UiActions {
        new_interval_ms: None,
    };

    TopBottomPanel::top("header").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("PC Monitor");
            ui.separator();
            if let Some(snap) = snapshot {
                ui.label(format!("seq #{}", snap.sequence));
                ui.label(match snap.status {
                    crate::domain::DataStatus::Fresh => "● Live",
                    crate::domain::DataStatus::Stale => "● Stale",
                    crate::domain::DataStatus::Unavailable => "● Unavailable",
                });
            } else {
                ui.label("Waiting for first sample…");
            }
        });
    });

    SidePanel::left("nav").exact_width(140.0).show(ctx, |ui| {
        ui.add_space(8.0);
        for page in Page::ALL {
            if ui
                .selectable_label(*active_page == page, page.label())
                .clicked()
            {
                *active_page = page;
            }
        }
    });

    egui::CentralPanel::default().show(ctx, |ui| match active_page {
        Page::Dashboard => dashboard::show(ui, snapshot, cpu_history, ram_history, gpu_history),
        Page::Cpu => cpu_page::show(ui, snapshot),
        Page::Memory => memory_page::show(ui, snapshot),
        Page::Gpu => gpu_page::show(ui, snapshot),
        Page::Storage => storage_page::show(ui, snapshot),
        Page::Settings => settings_page::show(ui, config, &mut actions),
    });

    actions
}
