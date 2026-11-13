use egui::{RichText, Ui};

use crate::domain::SystemSnapshot;
use crate::util::format;

use super::theme;
use super::widgets::{panel, section_heading, usage_bar};

pub fn show(ui: &mut Ui, snapshot: Option<&SystemSnapshot>) {
    let Some(snap) = snapshot else {
        ui.label("Collecting first sample…");
        return;
    };

    panel(ui, |ui| {
        section_heading(ui, "Volumes");
        ui.add_space(12.0);
        egui::Grid::new("volumes_table")
            .num_columns(7)
            .striped(true)
            .spacing([20.0, 14.0])
            .show(ui, |ui| {
                for heading in [
                    "Volume",
                    "Label",
                    "File system",
                    "Used",
                    "Free",
                    "Total",
                    "Usage",
                ] {
                    ui.label(
                        RichText::new(heading)
                            .size(12.0)
                            .strong()
                            .color(theme::MUTED),
                    );
                }
                ui.end_row();
                for volume in &snap.storage.volumes {
                    let ratio = volume.used_bytes as f32 / volume.total_bytes.max(1) as f32;
                    ui.label(RichText::new(&volume.mount_point).strong());
                    ui.label(&volume.label);
                    ui.label(&volume.filesystem);
                    ui.label(format::format_bytes(volume.used_bytes));
                    ui.label(format::format_bytes(volume.available_bytes));
                    ui.label(format::format_bytes(volume.total_bytes));
                    ui.horizontal(|ui| {
                        usage_bar(ui, ratio, theme::ACCENT);
                        ui.label(format!("{:.0}%", ratio * 100.0));
                    });
                    ui.end_row();
                }
            });
    });

    ui.add_space(16.0);
    panel(ui, |ui| {
        section_heading(ui, "Physical disks");
        ui.add_space(12.0);
        egui::Grid::new("disks_table")
            .num_columns(8)
            .striped(true)
            .spacing([16.0, 14.0])
            .show(ui, |ui| {
                for heading in [
                    "Disk", "Model", "Type", "Bus", "Total", "Read", "Write", "Active",
                ] {
                    ui.label(
                        RichText::new(heading)
                            .size(12.0)
                            .strong()
                            .color(theme::MUTED),
                    );
                }
                ui.end_row();
                for disk in &snap.storage.physical_disks {
                    ui.label(disk.device_number.to_string());
                    ui.label(&disk.model);
                    ui.label(format!("{:?}", disk.media_type));
                    ui.label(format!("{:?}", disk.bus_type));
                    ui.label(format::format_bytes(disk.total_bytes));
                    ui.label(rate(disk.read_bytes_per_sec));
                    ui.label(rate(disk.write_bytes_per_sec));
                    ui.label(
                        disk.active_time_percent
                            .map(|v| format!("{v:.0}%"))
                            .unwrap_or_else(|| "N/A".into()),
                    );
                    ui.end_row();
                }
            });
    });
}

fn rate(value: Option<u64>) -> String {
    value
        .map(|v| format!("{}/s", format::format_bytes(v)))
        .unwrap_or_else(|| "N/A".into())
}
