use egui::{RichText, Ui};

use crate::ui::UiActions;

use super::theme;
use super::widgets::{panel, section_heading};

pub fn show(ui: &mut Ui, actions: &mut UiActions) {
    let _ = actions; // unused for now
    panel(ui, |ui| {
        section_heading(ui, "Storage Analyzer");
        ui.add_space(8.0);
        ui.label(
            RichText::new("Choose a drive or folder to scan. Scanning reads file metadata only.")
                .color(theme::SECONDARY),
        );
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut String::new())
                    .hint_text("Select a folder…")
                    .desired_width(420.0),
            );
            ui.add_enabled(false, egui::Button::new("Browse"));
            ui.add_enabled(false, egui::Button::new("Start scan"));
        });
        ui.add_space(12.0);
        ui.label(
            RichText::new("Storage Analyzer implementation in progress.")
                .size(12.0)
                .color(theme::MUTED),
        );
    });
}
