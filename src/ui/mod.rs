mod cpu_page;
mod dashboard;
mod gpu_page;
mod memory_page;
mod processes_page;
mod settings_page;
mod storage_analyzer_page;
mod storage_page;
pub mod theme;
pub mod widgets;

pub use theme::apply as apply_theme;

use egui::{Context, RichText, SidePanel, TopBottomPanel};

use crate::config::AppConfig;
use crate::domain::SystemSnapshot;
use crate::monitoring::HistoryBuffer;
use crate::ui::widgets::AdvancedChartState;

#[derive(Default)]
pub struct UiActions {
    pub new_interval_ms: Option<u64>,
    pub new_history_secs: Option<u64>,
    pub paused: Option<bool>,
    pub theme_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Cpu,
    Memory,
    Gpu,
    Storage,
    Processes,
    StorageAnalyzer,
    Settings,
}

impl Page {
    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
            Self::Gpu => "GPU",
            Self::Storage => "Storage",
            Self::Processes => "Processes",
            Self::StorageAnalyzer => "Storage Analyzer",
            Self::Settings => "Settings",
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render(
    ctx: &Context,
    active_page: &mut Page,
    snapshot: Option<&SystemSnapshot>,
    cpu_history: &HistoryBuffer,
    ram_history: &HistoryBuffer,
    gpu_history: &HistoryBuffer,
    config: &mut AppConfig,
    paused: bool,
    chart_state: &mut AdvancedChartState,
) -> UiActions {
    let mut actions = UiActions::default();

    SidePanel::left("navigation")
        .exact_width(184.0)
        .frame(egui::Frame::new().fill(theme::SIDEBAR).inner_margin(12.0))
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new("PC MONITOR")
                    .size(15.0)
                    .strong()
                    .color(theme::TEXT),
            );
            ui.add_space(28.0);
            nav_group(ui, "MONITOR");
            for page in [
                Page::Overview,
                Page::Cpu,
                Page::Memory,
                Page::Gpu,
                Page::Storage,
            ] {
                nav_item(ui, active_page, page);
            }
            ui.add_space(20.0);
            nav_group(ui, "ACTIVITY");
            nav_item(ui, active_page, Page::Processes);
            ui.add_space(20.0);
            nav_group(ui, "ANALYSIS");
            nav_item(ui, active_page, Page::StorageAnalyzer);
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                nav_group(ui, "PREFERENCES");
                nav_item(ui, active_page, Page::Settings);
            });
        });

    TopBottomPanel::bottom("footer")
        .frame(
            egui::Frame::new()
                .fill(theme::SIDEBAR)
                .inner_margin(egui::Margin::symmetric(16, 6)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Normal user").size(11.0).color(theme::MUTED));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Local only · No data leaves this PC")
                            .size(11.0)
                            .color(theme::MUTED),
                    );
                });
            });
        });

    TopBottomPanel::top("toolbar")
        .frame(
            egui::Frame::new()
                .fill(theme::SIDEBAR)
                .inner_margin(egui::Margin::symmetric(20, 12)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new(active_page.label())
                        .size(22.0)
                        .color(theme::TEXT),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if paused { "Resume" } else { "Pause" }).clicked() {
                        actions.paused = Some(!paused);
                    }
                    egui::ComboBox::from_id_salt("sample_interval")
                        .selected_text(format!("{} ms", config.sampling_interval_ms))
                        .width(88.0)
                        .show_ui(ui, |ui| {
                            for ms in [250, 500, 1000, 2000, 5000] {
                                if ui
                                    .selectable_value(
                                        &mut config.sampling_interval_ms,
                                        ms,
                                        format!("{ms} ms"),
                                    )
                                    .changed()
                                {
                                    actions.new_interval_ms = Some(ms);
                                }
                            }
                        });
                    if let Some(snap) = snapshot {
                        ui.label(
                            RichText::new(format!(
                                "#{} · {}",
                                snap.sequence,
                                match snap.status {
                                    crate::domain::DataStatus::Fresh => "Fresh",
                                    crate::domain::DataStatus::Stale => "Stale",
                                    crate::domain::DataStatus::Unavailable => "Unavailable",
                                }
                            ))
                            .size(11.0)
                            .color(theme::MUTED),
                        );
                    }
                    status_badge(ui, paused, snapshot.is_some());
                });
            });
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(theme::WINDOW).inner_margin(20.0))
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match active_page {
                    Page::Overview => dashboard::show(
                        ui,
                        snapshot,
                        cpu_history,
                        ram_history,
                        gpu_history,
                        config.history_duration_secs,
                        chart_state,
                        config,
                    ),
                    Page::Cpu => cpu_page::show(ui, snapshot, config.active_core_threshold_percent),
                    Page::Memory => memory_page::show(ui, snapshot),
                    Page::Gpu => gpu_page::show(ui, snapshot),
                    Page::Storage => storage_page::show(ui, snapshot),
                    Page::Processes => processes_page::show(ui, snapshot),
                    Page::StorageAnalyzer => storage_analyzer_page::show(ui, &mut actions),
                    Page::Settings => settings_page::show(ui, config, &mut actions),
                });
        });

    actions
}

fn nav_group(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(10.0).strong().color(theme::MUTED));
    ui.add_space(4.0);
}

fn nav_item(ui: &mut egui::Ui, active: &mut Page, page: Page) {
    let selected = *active == page;
    let text = RichText::new(page.label()).size(13.0).color(if selected {
        theme::TEXT
    } else {
        theme::SECONDARY
    });
    if ui
        .add_sized([160.0, 34.0], egui::Button::selectable(selected, text))
        .clicked()
    {
        *active = page;
    }
}

fn status_badge(ui: &mut egui::Ui, paused: bool, has_data: bool) {
    let (text, color) = if paused {
        ("Paused", theme::WARNING)
    } else if has_data {
        ("Live", theme::ACCENT)
    } else {
        ("Starting", theme::MUTED)
    };
    ui.label(RichText::new(text).size(12.0).strong().color(color));
}
