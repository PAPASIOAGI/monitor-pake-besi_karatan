use egui::Ui;

use crate::domain::SystemSnapshot;
use crate::util::format;

pub fn show(ui: &mut Ui, snapshot: Option<&SystemSnapshot>) {
    ui.heading("Storage");
    ui.add_space(8.0);

    let Some(snap) = snapshot else {
        ui.label("No data yet.");
        return;
    };

    ui.label("Volumes:");
    for vol in &snap.storage.volumes {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            let pct = if vol.total_bytes > 0 {
                vol.used_bytes as f64 / vol.total_bytes as f64 * 100.0
            } else {
                0.0
            };
            ui.horizontal(|ui| {
                ui.heading(format!("{} ({})", vol.mount_point, vol.label));
                ui.label(format!("[{}]", vol.filesystem));
            });
            ui.label(format!(
                "{} / {} ({pct:.1}%) — {} free",
                format::format_bytes(vol.used_bytes),
                format::format_bytes(vol.total_bytes),
                format::format_bytes(vol.available_bytes)
            ));
            ui.add(egui::ProgressBar::new(pct as f32 / 100.0).desired_width(ui.available_width()));
        });
        ui.add_space(4.0);
    }

    ui.add_space(12.0);
    ui.label("Physical disks:");
    for disk in &snap.storage.physical_disks {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading(format!("Disk {} — {}", disk.device_number, disk.model));
            ui.label(format!(
                "Capacity: {}",
                format::format_bytes(disk.total_bytes)
            ));
            ui.label(format!(
                "Type: {:?} | Bus: {:?}",
                disk.media_type, disk.bus_type
            ));
            if let (Some(r), Some(w)) = (disk.read_bytes_per_sec, disk.write_bytes_per_sec) {
                ui.label(format!(
                    "Read: {}/s | Write: {}/s",
                    format::format_bytes(r),
                    format::format_bytes(w)
                ));
            }
            if let Some(active) = disk.active_time_percent {
                ui.label(format!("Active time: {active:.1}%"));
            }
        });
        ui.add_space(4.0);
    }
}
