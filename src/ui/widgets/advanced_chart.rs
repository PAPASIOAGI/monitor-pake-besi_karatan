use crate::monitoring::HistoryBuffer;
use crate::ui::theme;
use egui::{Align2, Color32, FontId, Layout, Pos2, RichText, Sense, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRange {
    Secs30,
    Secs60,
    Mins5,
    All,
}

impl TimeRange {
    pub fn label(self) -> &'static str {
        match self {
            Self::Secs30 => "30s",
            Self::Secs60 => "1m",
            Self::Mins5 => "5m",
            Self::All => "All",
        }
    }

    pub fn seconds(self) -> Option<u64> {
        match self {
            Self::Secs30 => Some(30),
            Self::Secs60 => Some(60),
            Self::Mins5 => Some(300),
            Self::All => None,
        }
    }

    pub fn all() -> [Self; 4] {
        [Self::Secs30, Self::Secs60, Self::Mins5, Self::All]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartStats {
    pub min: f32,
    pub max: f32,
    pub avg: f32,
    pub current: f32,
}

impl ChartStats {
    pub fn from_values(values: &[f32]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let min = values.iter().copied().fold(f32::INFINITY, f32::min);
        let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let avg = values.iter().copied().sum::<f32>() / values.len() as f32;
        let current = *values.last().unwrap_or(&0.0);
        Some(Self {
            min,
            max,
            avg,
            current,
        })
    }
}

pub struct AdvancedChartState {
    pub selected_metrics: Vec<bool>,
    pub time_range: TimeRange,
    pub paused: bool,
    pub show_stats: bool,
}

impl Default for AdvancedChartState {
    fn default() -> Self {
        Self {
            selected_metrics: vec![true, true, true], // CPU, GPU, Memory
            time_range: TimeRange::Secs60,
            paused: false,
            show_stats: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChartConfig {
    pub height: f32,
    pub max_duration_secs: u64,
}

pub fn advanced_chart(
    ui: &mut Ui,
    series: &mut [(&str, &HistoryBuffer, Color32)],
    selected: &mut [bool],
    time_range: &mut TimeRange,
    paused: &mut bool,
    show_stats: &mut bool,
    config: ChartConfig,
) {
    // Control bar
    ui.horizontal(|ui| {
        // Metric selection
        for (i, (name, _, color)) in series.iter().enumerate() {
            let (dot, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 3.5, *color);
            ui.checkbox(&mut selected[i], *name);
            ui.add_space(8.0);
        }

        ui.separator();
        ui.add_space(8.0);

        // Time range selection
        ui.label(RichText::new("Range:").size(11.0).color(theme::SECONDARY));
        egui::ComboBox::from_id_salt("chart_time_range")
            .selected_text(time_range.label())
            .width(60.0)
            .show_ui(ui, |ui| {
                for range in TimeRange::all() {
                    ui.selectable_value(time_range, range, range.label());
                }
            });
        ui.add_space(8.0);

        // Pause button
        if ui
            .selectable_label(
                *paused,
                RichText::new(if *paused { "Paused" } else { "Live" }).size(11.0),
            )
            .clicked()
        {
            *paused = !*paused;
        }
        ui.add_space(8.0);

        // Stats toggle
        ui.checkbox(show_stats, "Stats");
    });

    ui.add_space(4.0);

    // Chart area
    let available = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(available, config.height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::WINDOW);

    // Grid lines and Y-axis labels
    for step in 0..=4 {
        let frac = step as f32 / 4.0;
        let y = rect.top() + rect.height() * frac;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5_f32, theme::BORDER),
        );
        painter.text(
            Pos2::new(rect.left() + 6.0, y + 3.0),
            Align2::LEFT_TOP,
            format!("{:.0}%", (1.0 - frac) * 100.0),
            FontId::proportional(10.0),
            theme::MUTED,
        );
    }

    // Determine visible range based on time_range
    let visible_duration = time_range.seconds().unwrap_or(config.max_duration_secs);
    let samples_per_sec = 2.0; // Assuming 500ms default interval
    let visible_samples = (visible_duration as f32 * samples_per_sec).ceil() as usize;

    // Crosshair
    let hover_pos = if *paused {
        None // Keep last hover position when paused
    } else {
        response.hover_pos().map(|p| p.x)
    };

    if let Some(x) = hover_pos {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(1.0_f32, theme::MUTED),
        );
    }

    // Track statistics for visible range
    let mut stats_by_series: Vec<Option<ChartStats>> = Vec::new();

    // Draw series
    for (i, (name, history, color)) in series.iter().enumerate() {
        if !selected[i] {
            stats_by_series.push(None);
            continue;
        }

        let len = history.len();
        if len < 2 {
            stats_by_series.push(None);
            continue;
        }

        // Calculate visible range
        let start_idx = len.saturating_sub(visible_samples);
        let visible_values: Vec<f32> = history.iter().skip(start_idx).copied().collect();

        // Compute stats for visible range
        stats_by_series.push(ChartStats::from_values(&visible_values));

        // Draw line
        let points: Vec<Pos2> = visible_values
            .iter()
            .enumerate()
            .map(|(i, value)| {
                let x = rect.left()
                    + i as f32 / (visible_values.len().max(2) - 1) as f32 * rect.width();
                let y = rect.bottom() - (value.clamp(0.0, 100.0) / 100.0) * rect.height();
                Pos2::new(x, y)
            })
            .collect();

        for window in points.windows(2) {
            painter.line_segment([window[0], window[1]], Stroke::new(1.5_f32, *color));
        }

        // Draw hover marker and tooltip
        if let Some(hover_x) = hover_pos {
            let rel_x = (hover_x - rect.left()) / rect.width();
            let idx = (rel_x * (visible_values.len() - 1) as f32).round() as usize;
            if idx < visible_values.len() {
                let value = visible_values[idx];
                let y = rect.bottom() - value.clamp(0.0, 100.0) / 100.0 * rect.height();
                painter.circle_filled(Pos2::new(hover_x, y), 4.0, *color);

                // Tooltip
                let tooltip_text = format!("{}: {:.1}%", name, value);
                let tooltip_pos = Pos2::new(hover_x + 10.0, y - 20.0);
                painter.text(
                    tooltip_pos,
                    Align2::LEFT_BOTTOM,
                    tooltip_text,
                    FontId::proportional(11.0),
                    *color,
                );
            }
        }
    }

    // Statistics panel
    if *show_stats {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for (i, (name, _, color)) in series.iter().enumerate() {
                let stats = stats_by_series[i];
                if !selected[i] || stats.is_none() {
                    continue;
                }
                let stats = stats.unwrap();
                ui.group(|ui| {
                    ui.with_layout(Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.horizontal(|ui| {
                            let (dot, _) = ui.allocate_exact_size(Vec2::splat(6.0), Sense::hover());
                            ui.painter().circle_filled(dot.center(), 3.0, *color);
                            ui.label(RichText::new(*name).size(11.0).strong().color(*color));
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("Cur: {:.1}%", stats.current))
                                    .size(10.0)
                                    .color(theme::TEXT),
                            );
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(format!("Avg: {:.1}%", stats.avg))
                                    .size(10.0)
                                    .color(theme::SECONDARY),
                            );
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(format!("Min: {:.1}%", stats.min))
                                    .size(10.0)
                                    .color(theme::INFO),
                            );
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(format!("Max: {:.1}%", stats.max))
                                    .size(10.0)
                                    .color(theme::WARNING),
                            );
                        });
                    });
                });
                ui.add_space(8.0);
            }
        });
    }
}
