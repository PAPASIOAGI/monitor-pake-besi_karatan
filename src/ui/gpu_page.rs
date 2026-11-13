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

    if snap.gpus.is_empty() {
        panel(ui, |ui| {
            ui.label("No compatible GPU adapter detected.");
        });
        return;
    }

    ui.horizontal_wrapped(|ui| {
        for gpu in &snap.gpus {
            ui.add(egui::Button::selectable(true, &gpu.name));
            ui.label(RichText::new(&gpu.vendor).color(theme::MUTED));
        }
    });
    ui.add_space(12.0);

    for gpu in snap.gpus.iter().take(1) {
        ui.horizontal_wrapped(|ui| {
            if let Some(temp) = gpu.temperature_celsius {
                ui.label(format!("{temp:.0} °C"));
            }
            if let Some(clock) = gpu.core_clock_mhz {
                ui.label(format!("{clock} MHz"));
            }
            if let Some(memory) = gpu.memory_clock_mhz {
                ui.label(format!("Memory {memory} MHz"));
            }
            if let Some(power) = gpu.power_watts {
                ui.label(format!("{power:.0} W"));
            }
            if let Some(fan) = gpu.fan_rpm {
                ui.label(format!("{fan} RPM"));
            }
        });
        ui.add_space(12.0);
        ui.columns(2, |columns| {
            panel(&mut columns[0], |ui| {
                section_heading(ui, "Engine activity");
                ui.label(
                    RichText::new(format!("{:.1}%", gpu.total_utilization))
                        .size(32.0)
                        .strong(),
                );
                ui.add_space(12.0);
                for engine in &gpu.engines {
                    ui.horizontal(|ui| {
                        ui.allocate_ui(egui::vec2(110.0, 20.0), |ui| {
                            ui.label(RichText::new(engine.kind.label()).color(theme::SECONDARY));
                        });
                        usage_bar(ui, engine.utilization / 100.0, theme::INFO);
                        ui.label(RichText::new(format!("{:5.1}%", engine.utilization)).monospace());
                    });
                }
            });
            panel(&mut columns[1], |ui| {
                section_heading(ui, "Memory");
                ui.add_space(8.0);
                memory_row(
                    ui,
                    "Dedicated",
                    gpu.dedicated_vram_used_bytes,
                    gpu.dedicated_vram_total_bytes,
                );
                ui.add_space(16.0);
                memory_row(
                    ui,
                    "Shared",
                    gpu.shared_memory_used_bytes,
                    gpu.shared_memory_total_bytes,
                );
            });
        });
    }
}

fn memory_row(ui: &mut Ui, label: &str, used: Option<u64>, total: Option<u64>) {
    ui.label(RichText::new(label).color(theme::SECONDARY));
    match (used, total) {
        (Some(used), Some(total)) => {
            ui.label(
                RichText::new(format!(
                    "{} / {}",
                    format::format_bytes(used),
                    format::format_bytes(total)
                ))
                .size(18.0)
                .strong(),
            );
            usage_bar(ui, used as f32 / total.max(1) as f32, theme::ACCENT);
        }
        _ => {
            ui.label(RichText::new("N/A").color(theme::MUTED));
        }
    }
}
