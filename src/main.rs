#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cozyvim::app::App;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([980.0, 760.0])
            .with_min_inner_size([760.0, 600.0])
            .with_title("cozyvim"),
        ..Default::default()
    };
    eframe::run_native("cozyvim", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
