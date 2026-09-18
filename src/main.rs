#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cozyvim::app::App;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1650.0, 920.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("cozyvim"),
        ..Default::default()
    };
    eframe::run_native("cozyvim", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
