use egui::Ui;

use crate::config::AppConfig;

use super::UiActions;

pub fn show(ui: &mut Ui, config: &mut AppConfig, actions: &mut UiActions) {
    ui.heading("Settings");
    ui.add_space(12.0);

    let prev_interval = config.sampling_interval_ms;
    let prev_theme = config.theme;

    egui::Grid::new("settings_grid").show(ui, |ui| {
        ui.label("Sampling interval (ms)");
        ui.add(egui::DragValue::new(&mut config.sampling_interval_ms).range(100..=10_000));
        ui.end_row();

        ui.label("History duration (seconds)");
        ui.add(egui::DragValue::new(&mut config.history_duration_secs).range(10..=3600));
        ui.end_row();

        ui.label("Active core threshold (%)");
        ui.add(
            egui::DragValue::new(&mut config.active_core_threshold_percent)
                .range(0.0..=100.0)
                .speed(0.5),
        );
        ui.end_row();

        ui.label("Theme");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut config.theme, crate::config::ThemeChoice::Dark, "Dark");
            ui.selectable_value(
                &mut config.theme,
                crate::config::ThemeChoice::Light,
                "Light",
            );
        });
        ui.end_row();

        ui.label("Temperature unit");
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut config.temperature_unit,
                crate::config::TemperatureUnit::Celsius,
                "°C",
            );
            ui.selectable_value(
                &mut config.temperature_unit,
                crate::config::TemperatureUnit::Fahrenheit,
                "°F",
            );
        });
        ui.end_row();
    });

    if config.sampling_interval_ms != prev_interval {
        actions.new_interval_ms = Some(config.sampling_interval_ms);
    }
    if config.theme != prev_theme {
        actions.theme_changed = true;
    }

    ui.add_space(16.0);
    ui.label("Sampling interval applies immediately. Settings persist on exit.");
}
