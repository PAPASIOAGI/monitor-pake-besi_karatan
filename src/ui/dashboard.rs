use egui::{RichText, Ui};

use crate::domain::SystemSnapshot;
use crate::monitoring::HistoryBuffer;
use crate::util::format;

use super::theme;
use super::widgets::{metric_summary, panel, section_heading, time_series_chart, usage_bar};

pub fn show(
    ui: &mut Ui,
    snapshot: Option<&SystemSnapshot>,
    cpu_history: &HistoryBuffer,
    ram_history: &HistoryBuffer,
    gpu_history: &HistoryBuffer,
    history_secs: u64,
) {
    let Some(snap) = snapshot else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("Collecting first sample…").color(theme::SECONDARY));
        });
        return;
    };

    let columns = 4;
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 32.0 * columns as f32) / columns as f32;
        ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
            metric_summary(
                ui,
                "CPU",
                &format!("{:.1}%", snap.cpu.total_utilization),
                &format!(
                    "{} cores · {} active",
                    snap.cpu.physical_cores, snap.cpu.active_cores
                ),
                snap.cpu.total_utilization / 100.0,
            );
        });
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
        if let Some(gpu) = snap.gpus.first() {
            ui.allocate_ui(egui::vec2(width, 100.0), |ui| {
                metric_summary(
                    ui,
                    "GPU",
                    &format!("{:.1}%", gpu.total_utilization),
                    &gpu.name,
                    gpu.total_utilization / 100.0,
                );
            });
        }
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
    });

    ui.add_space(12.0);
    panel(ui, |ui| {
        section_heading(ui, "Activity");
        time_series_chart(
            ui,
            &[
                ("CPU", cpu_history, theme::ACCENT),
                ("GPU", gpu_history, theme::INFO),
                ("Memory", ram_history, theme::WARNING),
            ],
            230.0,
            history_secs,
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
                    usage_bar(ui, logical.utilization / 100.0, theme::ACCENT);
                    ui.label(RichText::new(format!("{:5.1}%", logical.utilization)).monospace());
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
