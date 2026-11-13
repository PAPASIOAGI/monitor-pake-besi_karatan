use egui::Ui;

use crate::domain::SystemSnapshot;
use crate::monitoring::HistoryBuffer;
use crate::util::format;

use super::widgets::{metric_card, time_series_chart};

pub fn show(
    ui: &mut Ui,
    snapshot: Option<&SystemSnapshot>,
    cpu_history: &HistoryBuffer,
    ram_history: &HistoryBuffer,
    gpu_history: &HistoryBuffer,
) {
    ui.heading("Dashboard");
    ui.add_space(8.0);

    let Some(snap) = snapshot else {
        ui.label("Waiting for first snapshot…");
        return;
    };

    ui.horizontal_wrapped(|ui| {
        metric_card(
            ui,
            "CPU",
            &format!("{:.1}%", snap.cpu.total_utilization),
            &snap.cpu.name,
        );
        metric_card(
            ui,
            "Memory",
            &format!("{:.1}%", snap.memory.usage_percent()),
            &format!(
                "{} / {}",
                format::format_bytes(snap.memory.used_bytes),
                format::format_bytes(snap.memory.total_bytes)
            ),
        );
        if let Some(gpu) = snap.gpus.first() {
            metric_card(
                ui,
                "GPU",
                &format!("{:.1}%", gpu.total_utilization),
                &gpu.name,
            );
        }
        if let Some(vol) = snap.storage.volumes.first() {
            let pct = if vol.total_bytes > 0 {
                vol.used_bytes as f64 / vol.total_bytes as f64 * 100.0
            } else {
                0.0
            };
            metric_card(
                ui,
                &format!("Storage {}", vol.mount_point),
                &format!("{pct:.1}%"),
                &format!("{} free", format::format_bytes(vol.available_bytes)),
            );
        }
    });

    ui.add_space(12.0);
    time_series_chart(ui, "CPU Usage (%)", cpu_history, 140.0);
    ui.add_space(8.0);
    time_series_chart(ui, "Memory Usage (%)", ram_history, 140.0);
    ui.add_space(8.0);
    time_series_chart(ui, "GPU Usage (%)", gpu_history, 140.0);
}
