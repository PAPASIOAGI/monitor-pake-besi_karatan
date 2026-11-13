use egui::{Color32, RichText, Ui};

use crate::ui::theme;

pub fn metric_summary(ui: &mut Ui, title: &str, value: &str, detail: &str, ratio: f32) {
    let width = (ui.available_width() - 24.0).max(220.0);
    ui.allocate_ui_with_layout(
        egui::vec2(width, 90.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.label(RichText::new(title).size(13.0).color(theme::SECONDARY));
            ui.label(RichText::new(value).size(28.0).strong().color(theme::TEXT));
            ui.label(RichText::new(detail).size(12.0).color(theme::MUTED));
            usage_bar(ui, ratio, theme::ACCENT);
        },
    );
}

pub fn section_heading(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).size(14.0).strong().color(theme::TEXT));
}

pub fn usage_bar(ui: &mut Ui, ratio: f32, color: Color32) {
    let desired = egui::vec2(ui.available_width(), 4.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, theme::BORDER);
    let mut fill = rect;
    fill.set_width(rect.width() * ratio.clamp(0.0, 1.0));
    painter.rect_filled(fill, 2.0, color);
}

pub fn panel<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
        .corner_radius(5.0)
        .inner_margin(16.0)
        .show(ui, add)
        .inner
}
