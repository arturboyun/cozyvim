//! Сквозной сценарий: упражнение проходится нажатиями, результат попадает
//! в журнал, прогресс и замки глав обновляются.

use cozyvim::app::{App, Session};
use cozyvim::core::grading::AttemptState;
use cozyvim::core::keys::parse_keys;
use cozyvim::core::settings::CheckMode;
use cozyvim::ui::Screen;

/// Каталог данных задаётся переменной окружения, то есть на весь процесс —
/// поэтому тесты, которые его подменяют, идут по одному.
static DATA_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn app_in_temp_dir(name: &str) -> (App, std::sync::MutexGuard<'static, ()>) {
    let guard = DATA_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("cozyvim-test-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    unsafe { std::env::set_var("COZYVIM_DATA_DIR", &dir) };
    let ctx = eframe::egui::Context::default();
    (App::with_context(&ctx), guard)
}

fn play(app: &mut App, keys: &str) {
    for k in parse_keys(keys) {
        app.press(k);
    }
}

#[test]
fn optimal_solution_earns_three_stars_and_is_logged() {
    let (mut app, _guard) = app_in_temp_dir("optimal");
    app.settings.check_mode = CheckMode::Mix;
    app.open_lesson("grammar/d-motion");
    play(&mut app, "dw");

    let session = app.session.as_ref().expect("сессия должна быть открыта");
    assert_eq!(session.state, AttemptState::Success { stars: 3 });
    assert_eq!(app.journal.attempts.len(), 1);
    assert!(app.journal.attempts[0].success);
    assert_eq!(app.progress.stars("grammar/d-motion", 0), 3);
}

#[test]
fn clumsy_path_still_passes_but_earns_fewer_stars() {
    let (mut app, _guard) = app_in_temp_dir("clumsy");
    app.settings.check_mode = CheckMode::Mix;
    app.open_lesson("grammar/d-motion");
    // то же самое посимвольно: цель достигнута, но нажатий втрое больше
    play(&mut app, "xxxx");

    let session = app.session.as_ref().unwrap();
    assert_eq!(session.state, AttemptState::Success { stars: 1 });
}

#[test]
fn strict_mode_reports_the_taught_solution() {
    let (mut app, _guard) = app_in_temp_dir("strict");
    app.settings.check_mode = CheckMode::Strict;
    app.open_lesson("grammar/d-motion");
    play(&mut app, "dj");

    let session = app.session.as_ref().unwrap();
    match &session.state {
        AttemptState::Failed { expected } => assert_eq!(expected, "dw"),
        other => panic!("ожидался провал, получено {other:?}"),
    }
    assert_eq!(app.journal.attempts.len(), 1);
    assert!(!app.journal.attempts[0].success);
    assert_eq!(app.progress.problem_commands(1)[0].0, "dw");
}

#[test]
fn unsupported_command_raises_a_notice_and_is_not_counted() {
    let (mut app, _guard) = app_in_temp_dir("unsupported");
    app.open_lesson("grammar/d-motion");
    play(&mut app, ":");

    assert!(app.toast.is_some(), "должно появиться сообщение о неподдержанной команде");
    let session = app.session.as_ref().unwrap();
    assert!(session.pressed.is_empty(), "такое нажатие не должно идти в счёт");
}

#[test]
fn chapters_unlock_in_order() {
    let (mut app, _guard) = app_in_temp_dir("unlock");
    assert!(app.chapter_unlocked("motion"));
    assert!(!app.chapter_unlocked("insert"));

    // проходим всю первую главу эталонными решениями
    let lessons: Vec<String> =
        app.curriculum.lessons_of("motion").iter().map(|l| l.id.clone()).collect();
    for id in lessons {
        let count = app.curriculum.lesson(&id).unwrap().exercises.len();
        for index in 0..count {
            let solution = {
                let lesson = app.curriculum.lesson(&id).unwrap();
                lesson.exercises[index].canonical().to_string()
            };
            app.session = Session::new(&app.curriculum, &id, index);
            app.screen = Screen::Lesson { lesson_id: id.clone() };
            play(&mut app, &solution);
            assert!(
                matches!(app.session.as_ref().unwrap().state, AttemptState::Success { .. }),
                "{id} #{index}: эталонное решение не засчиталось"
            );
        }
    }
    assert!(app.chapter_unlocked("insert"), "вторая глава должна открыться");
    assert!(!app.chapter_unlocked("grammar"));
}
