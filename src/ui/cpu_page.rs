use egui::{RichText, Ui};

use crate::domain::SystemSnapshot;
use crate::util::format;

use super::theme;
use super::widgets::{panel, section_heading, usage_bar};

pub fn show(ui: &mut Ui, snapshot: Option<&SystemSnapshot>, threshold: f32) {
    let Some(snap) = snapshot else {
        ui.label("Collecting first hardware sample…");
        return;
    };
    let cpu = &snap.cpu;

    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&cpu.name).size(16.0).strong());
        ui.label(RichText::new(format!("Vendor: {}", cpu.vendor)).color(theme::SECONDARY));
    });
    ui.label(
        RichText::new(format!(
            "{} physical cores · {} logical processors · {} sockets",
            cpu.physical_cores, cpu.logical_processors, cpu.sockets
        ))
        .color(theme::SECONDARY),
    );
    ui.add_space(16.0);
    ui.columns(2, |columns| {
        panel(&mut columns[0], |ui| {
            section_heading(ui, "Utilization");
            ui.label(
                RichText::new(format::percent(cpu.total_utilization))
                    .size(36.0)
                    .strong(),
            );
            usage_bar(ui, cpu.total_utilization.unwrap_or(0.0) / 100.0, theme::ACCENT);
            ui.add_space(8.0);
            ui.label(RichText::new(format!("{} active cores", cpu.active_cores)).color(theme::SECONDARY))
                .on_hover_text(format!("A physical core is active when at least one logical processor exceeds {threshold:.0}% utilization."));
        });
        panel(&mut columns[1], |ui| {
            section_heading(ui, "Clock");
            ui.label(
                RichText::new(
                    cpu.current_clock_mhz
                        .map(|clock| format!("{clock} MHz"))
                        .unwrap_or_else(|| "N/A".into()),
                )
                .size(28.0)
                .strong(),
            );
            ui.label(
                RichText::new(
                    cpu.base_clock_mhz
                        .map(|clock| format!("Reported maximum clock {clock} MHz"))
                        .unwrap_or_else(|| "Maximum clock N/A".into()),
                )
                .color(theme::SECONDARY),
            );
        });
    });
    ui.add_space(16.0);
    panel(ui, |ui| {
        section_heading(ui, "Logical processors");
        ui.label(
            RichText::new("Windows topology · grouped by physical core")
                .size(12.0)
                .color(theme::MUTED),
        );
        ui.add_space(12.0);
        egui::Grid::new("logical_processor_grid")
            .num_columns(3)
            .spacing([24.0, 12.0])
            .show(ui, |ui| {
                for logical in &cpu.per_logical {
                    ui.label(
                        RichText::new(format!("Core {}", logical.physical_core_index))
                            .color(theme::MUTED),
                    );
                    ui.label(format!("LP {}", logical.index));
                    ui.horizontal(|ui| {
                        usage_bar(
                            ui,
                            logical.utilization.unwrap_or(0.0) / 100.0,
                            theme::ACCENT,
                        );
                        ui.label(RichText::new(format::percent(logical.utilization)).monospace());
                    });
                    ui.end_row();
                }
            });
    });
}
