use egui::{Color32, Pos2, Sense, Stroke, Ui, Vec2};

use crate::monitoring::HistoryBuffer;

pub fn time_series_chart(ui: &mut Ui, title: &str, history: &HistoryBuffer, height: f32) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(title);
        let available = ui.available_width();
        let (rect, _response) =
            ui.allocate_exact_size(Vec2::new(available, height), Sense::hover());

        let painter = ui.painter_at(rect);
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0_f32, Color32::from_gray(60)),
            egui::StrokeKind::Inside,
        );

        if history.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Collecting…",
                egui::FontId::proportional(14.0),
                Color32::GRAY,
            );
            return;
        }

        for frac in [0.25_f32, 0.5, 0.75] {
            let y = rect.top() + rect.height() * frac;
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(0.5_f32, Color32::from_gray(45)),
            );
            painter.text(
                Pos2::new(rect.left() + 4.0, y - 2.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{:.0}%", (1.0 - frac) * 100.0),
                egui::FontId::proportional(10.0),
                Color32::from_gray(120),
            );
        }

        let points: Vec<Pos2> = history
            .iter()
            .enumerate()
            .map(|(i, value)| {
                let x = rect.left()
                    + (i as f32 / (history.iter().count().max(2) - 1) as f32) * rect.width();
                let y = rect.bottom() - (value.clamp(0.0, 100.0) / 100.0) * rect.height();
                Pos2::new(x, y)
            })
            .collect();

        for window in points.windows(2) {
            painter.line_segment(
                [window[0], window[1]],
                Stroke::new(1.5_f32, Color32::from_rgb(80, 180, 255)),
            );
        }
    });
}
