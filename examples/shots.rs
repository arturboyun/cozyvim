//! Служебный пример: рендерит экраны в PNG, чтобы посмотреть на них без окна.
//! Запуск: cargo run --example shots -- <папка>

use cozyvim::app::{App, Session};
use cozyvim::ui::Screen;
use egui_kittest::Harness;

fn shot(name: &str, dir: &str, size: (f32, f32), screen: Screen, exercise: Option<usize>) {
    let mut harness = Harness::builder()
        .wgpu()
        .with_size(eframe::egui::vec2(size.0, size.1))
        .build_ui(|ui| {
            let mut app = App::with_context(ui.ctx());
            app.settings.onboarded = true;
            let palette = app.palette();
            cozyvim::ui::theme::apply(ui.ctx(), palette, true);
            app.screen = screen.clone();
            if let (Screen::Lesson { lesson_id }, Some(index)) = (&app.screen, exercise) {
                app.session = Session::new(&app.curriculum, lesson_id, index);
            }
            match app.screen.clone() {
                Screen::Onboarding { step } => cozyvim::ui::onboarding::show(&mut app, ui, step),
                Screen::Map => cozyvim::ui::map::show(&mut app, ui),
                Screen::Stats => cozyvim::ui::stats::show(&mut app, ui),
                Screen::Settings => cozyvim::ui::settings_screen::show(&mut app, ui),
                Screen::Lesson { .. } => cozyvim::ui::lesson::show(&mut app, ui),
            }
        });
    harness.run();
    match harness.render() {
        Ok(image) => {
            let path = format!("{dir}/{name}.png");
            image.save(&path).expect("не удалось сохранить снимок");
            println!("saved {path}");
        }
        Err(e) => println!("render failed for {name}: {e}"),
    }
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    std::fs::create_dir_all(&dir).ok();
    shot("onboarding", &dir, (980.0, 760.0), Screen::Onboarding { step: 1 }, None);
    shot("map", &dir, (980.0, 760.0), Screen::Map, None);
    shot(
        "lesson",
        &dir,
        (980.0, 900.0),
        Screen::Lesson { lesson_id: "grammar/d-motion".into() },
        Some(0),
    );
    shot("stats", &dir, (980.0, 760.0), Screen::Stats, None);
    shot("settings", &dir, (980.0, 760.0), Screen::Settings, None);
}
