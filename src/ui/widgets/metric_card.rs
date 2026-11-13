use egui::{Color32, RichText, Ui};

pub fn metric_card(ui: &mut Ui, title: &str, value: &str, subtitle: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(200.0);
        ui.label(RichText::new(title).weak().size(13.0));
        ui.label(
            RichText::new(value)
                .size(28.0)
                .strong()
                .color(Color32::WHITE),
        );
        ui.label(RichText::new(subtitle).weak().size(12.0));
    });
}
