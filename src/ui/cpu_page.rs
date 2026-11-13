use egui::Ui;

use crate::domain::SystemSnapshot;

pub fn show(ui: &mut Ui, snapshot: Option<&SystemSnapshot>) {
    ui.heading("CPU");
    ui.add_space(8.0);

    let Some(snap) = snapshot else {
        ui.label("No data yet.");
        return;
    };
    let cpu = &snap.cpu;

    ui.horizontal_wrapped(|ui| {
        ui.label(format!("Name: {}", cpu.name));
        ui.separator();
        ui.label(format!(
            "Physical cores: {} | Logical processors: {} | Sockets: {}",
            cpu.physical_cores, cpu.logical_processors, cpu.sockets
        ));
        ui.separator();
        ui.label(format!("Active cores: {}", cpu.active_cores));
    });

    ui.add_space(8.0);
    ui.label(format!("Total utilization: {:.1}%", cpu.total_utilization));
    if let (Some(base), Some(cur)) = (cpu.base_clock_mhz, cpu.current_clock_mhz) {
        ui.label(format!("Clock: {cur} MHz (base {base} MHz)"));
    }

    ui.add_space(12.0);
    ui.separator();
    ui.label("Per logical processor:");

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("cpu_grid").striped(true).show(ui, |ui| {
            ui.label("LP");
            ui.label("Core");
            ui.label("Usage");
            ui.end_row();
            for lp in &cpu.per_logical {
                ui.label(format!("{}", lp.index));
                ui.label(format!("{}", lp.physical_core_index));
                ui.label(format!("{:.1}%", lp.utilization));
                ui.end_row();
            }
        });
    });
}
