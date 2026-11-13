use egui::Ui;

use crate::domain::SystemSnapshot;
use crate::util::format;

pub fn show(ui: &mut Ui, snapshot: Option<&SystemSnapshot>) {
    ui.heading("GPU");
    ui.add_space(8.0);

    let Some(snap) = snapshot else {
        ui.label("No data yet.");
        return;
    };

    for gpu in &snap.gpus {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading(&gpu.name);
            ui.label(format!("Vendor: {}", gpu.vendor));
            ui.label(format!("Total utilization: {:.1}%", gpu.total_utilization));

            ui.add_space(6.0);
            ui.label("Engines:");
            for engine in &gpu.engines {
                ui.horizontal(|ui| {
                    ui.label(format!("  {}", engine.kind.label()));
                    ui.label(format!("{:.1}%", engine.utilization));
                });
            }

            ui.add_space(6.0);
            if let (Some(used), Some(total)) = (
                gpu.dedicated_vram_used_bytes,
                gpu.dedicated_vram_total_bytes,
            ) {
                ui.label(format!(
                    "VRAM: {} / {}",
                    format::format_bytes(used),
                    format::format_bytes(total)
                ));
            }
            if let (Some(used), Some(total)) =
                (gpu.shared_memory_used_bytes, gpu.shared_memory_total_bytes)
            {
                ui.label(format!(
                    "Shared: {} / {}",
                    format::format_bytes(used),
                    format::format_bytes(total)
                ));
            }

            ui.add_space(6.0);
            if let Some(t) = gpu.temperature_celsius {
                ui.label(format!("Temperature: {t:.1} °C"));
            }
            if let Some(c) = gpu.core_clock_mhz {
                ui.label(format!("Core clock: {c} MHz"));
            }
            if let Some(c) = gpu.memory_clock_mhz {
                ui.label(format!("Memory clock: {c} MHz"));
            }
            if let Some(p) = gpu.power_watts {
                ui.label(format!("Power: {p:.1} W"));
            }
            if let Some(f) = gpu.fan_rpm {
                ui.label(format!("Fan: {f} RPM"));
            }
        });
        ui.add_space(8.0);
    }
}
