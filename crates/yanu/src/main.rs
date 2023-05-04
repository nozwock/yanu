// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "windows", target_os = "linux")
))]
use common::defines::APP_NAME;
use common::log;
use eyre::Result;
use std::env;
use tracing::{error, info};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use yanu::gui::app;

fn main() -> Result<()> {
    // Colorful errors
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;

    // Tracing
    let file_appender = tracing_appender::rolling::hourly("", "yanu.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // TODO: Take a look at `filter_fn` in tracing-subscriber
    let filter = tracing_subscriber::filter::EnvFilter::new(
        "cache=debug,common=debug,config=debug,hac=debug,yanu=debug,off",
    );
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .event_format(log::CustomFmt)
                .with_writer(non_blocking),
        )
        .with(filter)
        .init();

    // Exit signals handling
    ctrlc::set_handler(|| {})?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        arch = std::env::consts::ARCH,
        os = std::env::consts::OS,
        "Launching {}",
        env!("CARGO_PKG_NAME"),
    );

    let native_options = eframe::NativeOptions {
        min_window_size: Some(egui::vec2(600., 400.)),
        initial_window_size: Some(egui::vec2(600., 400.)),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| Box::new(app::YanuApp::new(cc))),
    )
    .unwrap();

    Ok(())
}
