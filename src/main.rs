// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod git;
mod markdown;
mod vault;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Scrap")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Scrap",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
