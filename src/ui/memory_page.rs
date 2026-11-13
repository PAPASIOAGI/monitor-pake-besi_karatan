use egui::{Pos2, RichText, Sense, Ui, Vec2};

use crate::domain::SystemSnapshot;
use crate::util::format;

use super::theme;
use super::widgets::{panel, section_heading};

pub fn show(ui: &mut Ui, snapshot: Option<&SystemSnapshot>) {
    let Some(snap) = snapshot else {
        ui.label("Collecting first sample…");
        return;
    };
    let m = &snap.memory;
    let usage = m.usage_percent();
    let cached = m.cached_bytes.unwrap_or(0) as f32 / m.total_bytes.max(1) as f32 * 100.0;

    panel(ui, |ui| {
        section_heading(ui, "Physical memory");
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!(
                "{} used of {}",
                format::format_bytes(m.used_bytes),
                format::format_bytes(m.total_bytes)
            ))
            .size(24.0)
            .strong(),
        );
        ui.label(
            RichText::new(format!("{usage:.1}%"))
                .size(16.0)
                .color(theme::ACCENT),
        );
        ui.add_space(12.0);
        composition_bar(
            ui,
            m.used_bytes as f32 / m.total_bytes.max(1) as f32,
            cached / 100.0,
        );
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            legend(ui, "In use", theme::ACCENT);
            legend(ui, "Cached", theme::INFO);
            legend(ui, "Available", theme::BORDER);
        });
    });
    ui.add_space(16.0);
    panel(ui, |ui| {
        egui::Grid::new("memory_details")
            .num_columns(2)
            .spacing([48.0, 12.0])
            .show(ui, |ui| {
                detail_row(ui, "In use", &format::format_bytes(m.used_bytes));
                detail_row(ui, "Available", &format::format_bytes(m.available_bytes));
                detail_row(
                    ui,
                    "Cached",
                    &m.cached_bytes
                        .map(format::format_bytes)
                        .unwrap_or_else(|| "N/A".into()),
                );
                detail_row(
                    ui,
                    "Committed",
                    &format!(
                        "{} / {}",
                        format::format_bytes(m.commit_used_bytes),
                        format::format_bytes(m.commit_limit_bytes)
                    ),
                );
                let pagefile = match (m.page_file_used_bytes, m.page_file_total_bytes) {
                    (Some(used), Some(total)) => format!(
                        "{} / {}",
                        format::format_bytes(used),
                        format::format_bytes(total)
                    ),
                    _ => "N/A".into(),
                };
                detail_row(ui, "Page file", &pagefile);
            });
    });
}

fn composition_bar(ui: &mut Ui, used: f32, cached: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, theme::BORDER);
    let used_width = rect.width() * used.clamp(0.0, 1.0);
    painter.rect_filled(
        egui::Rect::from_min_size(rect.min, Vec2::new(used_width, rect.height())),
        3.0,
        theme::ACCENT,
    );
    let cached_width = rect.width() * cached.clamp(0.0, 1.0 - used);
    painter.rect_filled(
        egui::Rect::from_min_size(
            Pos2::new(rect.left() + used_width, rect.top()),
            Vec2::new(cached_width, rect.height()),
        ),
        0.0,
        theme::INFO,
    );
}

fn legend(ui: &mut Ui, text: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
        ui.painter().rect_filled(rect, 1.0, color);
        ui.label(RichText::new(text).size(12.0).color(theme::SECONDARY));
    });
}

fn detail_row(ui: &mut Ui, name: &str, value: &str) {
    ui.label(RichText::new(name).color(theme::SECONDARY));
    ui.label(RichText::new(value).monospace());
    ui.end_row();
}
