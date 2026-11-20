use egui::{RichText, Ui};

use crate::config::AppConfig;
use crate::domain::SystemSnapshot;
use crate::monitoring::HistoryBuffer;
use crate::util::format::{self, percent};

use super::theme;
use super::widgets::{
    AdvancedChartState, ChartConfig, advanced_chart, metric_summary, panel, section_heading,
    usage_bar,
};

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut Ui,
    snapshot: Option<&SystemSnapshot>,
    cpu_history: &HistoryBuffer,
    ram_history: &HistoryBuffer,
    gpu_history: &HistoryBuffer,
    history_secs: u64,
    chart_state: &mut AdvancedChartState,
    config: &AppConfig,
) {
    let Some(snap) = snapshot else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("Collecting first sample…").color(theme::SECONDARY));
        });
        return;
    };

    // Metric cards row
    let mut card_count = 4; // CPU, Memory, GPU, Storage
    if snap.cpu.temperature_celsius.is_some() {
        card_count += 1; // CPU temp
    }
    if snap.gpus.iter().any(|g| g.temperature_celsius.is_some()) {
        card_count += 1; // GPU temp
    }

    // Temperature unit conversion helper
    let temp_unit = config.temperature_unit;
    let format_temp = |celsius: f32| -> String {
        match temp_unit {
            crate::config::TemperatureUnit::Celsius => format!("{:.0} °C", celsius),
            crate::config::TemperatureUnit::Fahrenheit => {
                format!("{:.0} °F", celsius * 9.0 / 5.0 + 32.0)
            }
        }
    };

    ui.horizontal(|ui| {
        let width = (ui.available_width() - 16.0 * card_count as f32) / card_count as f32;

        // CPU Card
        ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
            metric_summary(
                ui,
                "CPU",
                &percent(snap.cpu.total_utilization),
                &format!(
                    "{} cores · {} active",
                    snap.cpu.physical_cores, snap.cpu.active_cores
                ),
                snap.cpu.total_utilization.unwrap_or(0.0) / 100.0,
            );
        });

        // Memory Card
        ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
            metric_summary(
                ui,
                "Memory",
                &format!(
                    "{} / {}",
                    format::format_bytes(snap.memory.used_bytes),
                    format::format_bytes(snap.memory.total_bytes)
                ),
                &format!("{:.1}% used", snap.memory.usage_percent()),
                snap.memory.usage_percent() / 100.0,
            );
        });

        // GPU Card
        if let Some(gpu) = snap.gpus.first() {
            ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
                metric_summary(
                    ui,
                    "GPU",
                    &percent(gpu.total_utilization),
                    &gpu.name,
                    gpu.total_utilization.unwrap_or(0.0) / 100.0,
                );
            });
        }

        // Primary Storage Card
        if let Some(volume) = snap.storage.volumes.first() {
            let ratio = volume.used_bytes as f32 / volume.total_bytes.max(1) as f32;
            ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
                metric_summary(
                    ui,
                    &format!("Storage {}", volume.mount_point),
                    &format!("{:.1}%", ratio * 100.0),
                    &format!("{} free", format::format_bytes(volume.available_bytes)),
                    ratio,
                );
            });
        }

        // CPU Temperature Card (if available)
        if let Some(temp_c) = snap.cpu.temperature_celsius {
            ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
                metric_summary(
                    ui,
                    "CPU Temp",
                    &format_temp(temp_c),
                    "CPU Temperature",
                    (temp_c / 100.0).clamp(0.0, 1.0),
                );
            });
        }

        // GPU Temperature Card (if available)
        if let Some(gpu) = snap.gpus.first()
            && let Some(temp_c) = gpu.temperature_celsius
        {
            ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
                metric_summary(
                    ui,
                    "GPU Temp",
                    &format_temp(temp_c),
                    "GPU Temperature",
                    (temp_c / 100.0).clamp(0.0, 1.0),
                );
            });
        }
    });

    ui.add_space(12.0);

    // Advanced Chart
    panel(ui, |ui| {
        section_heading(ui, "Activity");
        let mut series = [
            ("CPU", cpu_history, theme::ACCENT),
            ("GPU", gpu_history, theme::INFO),
            ("Memory", ram_history, theme::WARNING),
        ];
        advanced_chart(
            ui,
            &mut series,
            &mut chart_state.selected_metrics,
            &mut chart_state.time_range,
            &mut chart_state.paused,
            &mut chart_state.show_stats,
            ChartConfig {
                height: 230.0,
                max_duration_secs: history_secs,
            },
        );
    });

    ui.add_space(12.0);
    ui.columns(2, |columns| {
        panel(&mut columns[0], |ui| {
            section_heading(ui, "Logical processors");
            ui.add_space(8.0);
            for logical in snap.cpu.per_logical.iter().take(6) {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("LP {:02}", logical.index))
                            .monospace()
                            .color(theme::SECONDARY),
                    );
                    ui.add_space(8.0);
                    usage_bar(
                        ui,
                        logical.utilization.unwrap_or(0.0) / 100.0,
                        theme::ACCENT,
                    );
                    ui.label(RichText::new(percent(logical.utilization)).monospace());
                });
            }
        });
        panel(&mut columns[1], |ui| {
            section_heading(ui, "GPU engines");
            ui.add_space(8.0);
            if let Some(gpu) = snap.gpus.first() {
                for engine in gpu.engines.iter().take(6) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(engine.kind.label()).color(theme::SECONDARY));
                        usage_bar(ui, engine.utilization / 100.0, theme::INFO);
                        ui.label(RichText::new(format!("{:5.1}%", engine.utilization)).monospace());
                    });
                }
            }
        });
    });
}
