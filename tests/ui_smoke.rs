//! Дымовые тесты интерфейса: каждый экран рисуется в безголовом контексте.
//! Ловят паники в разметке, конфликты идентификаторов и арифметику в рисовании.

use cozyvim::app::App;
use cozyvim::ui::Screen;

fn screens() -> Vec<Screen> {
    vec![
        Screen::Onboarding { step: 0 },
        Screen::Onboarding { step: 1 },
        Screen::Onboarding { step: 2 },
        Screen::Onboarding { step: 3 },
        Screen::Map,
        Screen::Stats,
        Screen::Settings,
    ]
}

#[test]
fn every_screen_renders() {
    for screen in screens() {
        eframe::egui::__run_test_ui(|ui| {
            let mut app = App::with_context(ui.ctx());
            app.screen = screen.clone();
            match app.screen.clone() {
                Screen::Onboarding { step } => cozyvim::ui::onboarding::show(&mut app, ui, step),
                Screen::Map => cozyvim::ui::map::show(&mut app, ui),
                Screen::Stats => cozyvim::ui::stats::show(&mut app, ui),
                Screen::Settings => cozyvim::ui::settings_screen::show(&mut app, ui),
                Screen::Lesson { .. } => {}
            }
        });
    }
}

#[test]
fn lesson_screen_renders_for_every_exercise() {
    eframe::egui::__run_test_ui(|ui| {
        let mut app = App::with_context(ui.ctx());
        let lessons: Vec<(String, usize)> = app
            .curriculum
            .lessons
            .iter()
            .map(|l| (l.id.clone(), l.exercises.len()))
            .collect();
        for (id, count) in lessons {
            for index in 0..count {
                app.session = cozyvim::app::Session::new(&app.curriculum, &id, index);
                app.screen = Screen::Lesson { lesson_id: id.clone() };
                cozyvim::ui::lesson::show(&mut app, ui);
            }
        }
    });
}
