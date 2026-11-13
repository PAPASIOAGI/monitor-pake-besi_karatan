use egui::Ui;

use crate::domain::SystemSnapshot;
use crate::util::format;

pub fn show(ui: &mut Ui, snapshot: Option<&SystemSnapshot>) {
    ui.heading("Memory");
    ui.add_space(8.0);

    let Some(snap) = snapshot else {
        ui.label("No data yet.");
        return;
    };
    let m = &snap.memory;

    egui::Grid::new("mem_grid").striped(true).show(ui, |ui| {
        ui.label("Total");
        ui.label(format::format_bytes(m.total_bytes));
        ui.end_row();

        ui.label("Used");
        ui.label(format!(
            "{} ({:.1}%)",
            format::format_bytes(m.used_bytes),
            m.usage_percent()
        ));
        ui.end_row();

        ui.label("Available");
        ui.label(format::format_bytes(m.available_bytes));
        ui.end_row();

        ui.label("Commit");
        ui.label(format!(
            "{} / {}",
            format::format_bytes(m.commit_used_bytes),
            format::format_bytes(m.commit_limit_bytes)
        ));
        ui.end_row();

        if let (Some(pf_used), Some(pf_total)) = (m.page_file_used_bytes, m.page_file_total_bytes) {
            ui.label("Page file");
            ui.label(format!(
                "{} / {}",
                format::format_bytes(pf_used),
                format::format_bytes(pf_total)
            ));
            ui.end_row();
        }

        if let Some(cached) = m.cached_bytes {
            ui.label("Cached");
            ui.label(format::format_bytes(cached));
            ui.end_row();
        }
    });
}
