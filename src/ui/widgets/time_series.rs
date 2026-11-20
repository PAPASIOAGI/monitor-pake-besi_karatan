use egui::{Color32, Pos2, Sense, Stroke, Ui, Vec2};

use crate::monitoring::HistoryBuffer;
use crate::ui::theme;

#[allow(dead_code)]
pub fn time_series_chart(
    ui: &mut Ui,
    series: &[(&str, &HistoryBuffer, Color32)],
    height: f32,
    duration_secs: u64,
) {
    ui.horizontal(|ui| {
        for (name, _, color) in series {
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 3.5, *color);
            ui.label(
                egui::RichText::new(*name)
                    .color(theme::SECONDARY)
                    .size(12.0),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!("{duration_secs} s"))
                    .color(theme::MUTED)
                    .size(11.0),
            );
        });
    });

    let available = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(available, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::WINDOW);

    for step in 0..=4 {
        let frac = step as f32 / 4.0;
        let y = rect.top() + rect.height() * frac;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5_f32, theme::BORDER),
        );
        painter.text(
            Pos2::new(rect.left() + 6.0, y + 3.0),
            egui::Align2::LEFT_TOP,
            format!("{:.0}%", (1.0 - frac) * 100.0),
            egui::FontId::proportional(10.0),
            theme::MUTED,
        );
    }

    let hover_x = response.hover_pos().map(|p| p.x);
    if let Some(x) = hover_x {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0_f32, theme::MUTED),
        );
    }

    for (_, history, color) in series {
        let len = history.len();
        if len < 2 {
            continue;
        }
        let points: Vec<Pos2> = history
            .iter()
            .enumerate()
            .map(|(i, value)| {
                let x = rect.left() + i as f32 / (len - 1) as f32 * rect.width();
                let y = rect.bottom() - (value.clamp(0.0, 100.0) / 100.0) * rect.height();
                Pos2::new(x, y)
            })
            .collect();
        for window in points.windows(2) {
            painter.line_segment([window[0], window[1]], Stroke::new(1.5_f32, *color));
        }
        if let Some(hover_x) = hover_x
            && let Some(value) = history.value_at_x(rect, hover_x)
        {
            let y = rect.bottom() - value.clamp(0.0, 100.0) / 100.0 * rect.height();
            painter.circle_filled(Pos2::new(hover_x, y), 3.0, *color);
        }
    }
}
