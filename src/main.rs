mod app;
mod config;
mod domain;
mod error;
mod monitoring;
mod platform;
mod ui;
mod util;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let config = config::AppConfig::load_or_default();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PC Monitor")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PC Monitor",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::MonitorApp::new(cc, config)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}
