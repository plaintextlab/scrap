// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

fn main() -> eframe::Result {
    // `scrap <folder>` opens that folder as the vault.
    let vault = std::env::args_os().nth(1).map(PathBuf::from);
    // SCRAP_CONFIG_DIR keeps settings somewhere other than the per-user default
    // (a portable install, or a separate profile for testing).
    let persistence_path = std::env::var_os("SCRAP_CONFIG_DIR").map(|d| PathBuf::from(d).join("app.ron"));
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Scrap")
        .with_app_id("scrap")
        .with_inner_size([1180.0, 760.0])
        .with_min_inner_size([480.0, 320.0]);
    // Screenshot runs (see dev.rs) stay out of the way: off screen and unfocused.
    if std::env::var_os("SCRAP_SCREENSHOT").is_some() {
        let size = std::env::var("SCRAP_SIZE").ok().and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some([w.parse().ok()?, h.parse().ok()?])
        });
        viewport = viewport.with_position([-6000.0, 0.0]).with_active(false).with_inner_size(size.unwrap_or([1180.0, 760.0]));
    }
    let options = eframe::NativeOptions {
        viewport,
        persistence_path,
        ..Default::default()
    };
    eframe::run_native(
        "Scrap",
        options,
        Box::new(move |cc| Ok(Box::new(scrap::App::new(cc, vault)))),
    )
}
