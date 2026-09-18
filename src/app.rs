//! Состояние приложения и маршрутизация экранов.

use std::time::Instant;

use eframe::egui;

use crate::content::Curriculum;
use crate::core::buffer::Buffer;
use crate::core::grading::{AttemptState, evaluate};
use crate::core::journal::{Attempt, Journal};
use crate::core::keys::{Key, keys_notation};
use crate::core::progress::Progress;
use crate::core::settings::{KeyboardVisibility, Settings, Theme};
use crate::ui::{Screen, theme};
use crate::vim::engine::{Engine, Mode, StepResult};

/// Одно упражнение в работе.
pub struct Session {
    pub lesson_id: String,
    pub index: usize,
    pub engine: Engine,
    pub pressed: Vec<Key>,
    pub started: Instant,
    pub state: AttemptState,
    pub hint_shown: bool,
    /// Яркость подсветки изменённых строк, гаснет за несколько кадров.
    pub flash: f32,
    pub last_key: Option<Key>,
    pub logged: bool,
}

impl Session {
    pub fn new(curriculum: &Curriculum, lesson_id: &str, index: usize) -> Option<Self> {
        let lesson = curriculum.lesson(lesson_id)?;
        let ex = lesson.exercises.get(index)?;
        let mut buf = Buffer::new(&ex.buffer);
        buf.cursor = (ex.cursor[0], ex.cursor[1]);
        Some(Self {
            lesson_id: lesson_id.to_string(),
            index,
            engine: Engine::new(buf),
            pressed: Vec::new(),
            started: Instant::now(),
            state: AttemptState::InProgress,
            hint_shown: false,
            flash: 0.0,
            last_key: None,
            logged: false,
        })
    }

    pub fn reset(&mut self, curriculum: &Curriculum) {
        if let Some(fresh) = Session::new(curriculum, &self.lesson_id, self.index) {
            let hint = self.hint_shown;
            *self = fresh;
            self.hint_shown = hint;
        }
    }
}

pub struct App {
    pub settings: Settings,
    pub curriculum: Curriculum,
    pub journal: Journal,
    pub progress: Progress,
    pub screen: Screen,
    pub session: Option<Session>,
    /// Сообщение о неподдерживаемой команде и момент его появления.
    pub toast: Option<(String, Instant)>,
    pub last_escape: Option<Instant>,
    pub keyboard_open: bool,
    pub keyboard_auto_hidden: bool,
    pub confirm_reset: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::with_context(&cc.egui_ctx)
    }

    /// Отдельный конструктор от голого контекста: так приложение можно
    /// отрисовать в тесте, без окна и бэкенда.
    pub fn with_context(ctx: &egui::Context) -> Self {
        let settings = Settings::load();
        rust_i18n::set_locale(settings.lang.code());
        theme::install_fonts(ctx);
        theme::text_styles(ctx);

        let journal = Journal::load();
        let progress = Progress::from_journal(&journal);
        let curriculum = Curriculum::load();
        let screen =
            if settings.onboarded { Screen::Map } else { Screen::Onboarding { step: 0 } };
        let keyboard_open = settings.keyboard != KeyboardVisibility::Never;

        Self {
            settings,
            curriculum,
            journal,
            progress,
            screen,
            session: None,
            toast: None,
            last_escape: None,
            keyboard_open,
            keyboard_auto_hidden: false,
            confirm_reset: false,
        }
    }

    pub fn palette(&self) -> theme::Palette {
        theme::palette(self.settings.theme)
    }

    pub fn save_settings(&mut self) {
        rust_i18n::set_locale(self.settings.lang.code());
        self.settings.save();
    }

    pub fn open_lesson(&mut self, lesson_id: &str) {
        self.session = Session::new(&self.curriculum, lesson_id, 0);
        self.screen = Screen::Lesson { lesson_id: lesson_id.to_string() };
    }

    /// Главы открываются по очереди; уверенному пользователю, который так и
    /// сказал на онбординге, замки не нужны.
    pub fn chapter_unlocked(&self, chapter: &str) -> bool {
        if self.settings.experience == crate::core::settings::Experience::Confident {
            return true;
        }
        self.progress.chapter_unlocked(&self.curriculum, chapter)
    }

    /// Клавиатура прячется сама, когда навык вырос — но только в режиме «Авто».
    pub fn keyboard_visible(&self) -> bool {
        match self.settings.keyboard {
            KeyboardVisibility::Always => true,
            KeyboardVisibility::Never => false,
            KeyboardVisibility::Auto => self.keyboard_open && !self.keyboard_auto_hidden,
        }
    }

    fn update_auto_keyboard(&mut self) {
        if self.settings.keyboard == KeyboardVisibility::Auto
            && self.progress.exercises_done() >= 20
        {
            self.keyboard_auto_hidden = true;
        }
    }

    /// Следующая ожидаемая клавиша — если известна однозначно.
    pub fn expected_key(&self) -> Option<Key> {
        let session = self.session.as_ref()?;
        let lesson = self.curriculum.lesson(&session.lesson_id)?;
        let ex = lesson.exercises.get(session.index)?;
        if !session.hint_shown && self.settings.check_mode != crate::core::settings::CheckMode::Strict
        {
            return None;
        }
        let candidates: Vec<Key> = ex
            .solution_keys()
            .into_iter()
            .filter(|s| s.len() > session.pressed.len() && s[..session.pressed.len()] == session.pressed[..])
            .map(|s| s[session.pressed.len()])
            .collect();
        let first = candidates.first().copied()?;
        if candidates.iter().all(|k| *k == first) { Some(first) } else { None }
    }

    /// Собирает нажатия из событий egui и скармливает их движку.
    fn handle_input(&mut self, ctx: &egui::Context) {
        if !matches!(self.screen, Screen::Lesson { .. }) {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let mut keys: Vec<Key> = Vec::new();
        for event in events {
            match event {
                egui::Event::Text(text) => {
                    for c in text.chars() {
                        keys.push(Key::Char(c));
                    }
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    if modifiers.ctrl {
                        if let Some(name) = key.name().chars().next() {
                            keys.push(Key::Ctrl(name.to_ascii_lowercase()));
                        }
                        continue;
                    }
                    match key {
                        egui::Key::Escape => keys.push(Key::Esc),
                        egui::Key::Enter => keys.push(Key::Enter),
                        egui::Key::Backspace => keys.push(Key::Backspace),
                        egui::Key::Tab => keys.push(Key::Tab),
                        egui::Key::F1 => self.toggle_hint(),
                        egui::Key::F5 => self.reset_exercise(),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        for key in keys {
            self.feed_key(key);
        }
    }

    pub fn toggle_hint(&mut self) {
        if let Some(s) = self.session.as_mut() {
            s.hint_shown = !s.hint_shown;
        }
    }

    pub fn reset_exercise(&mut self) {
        let curriculum = &self.curriculum;
        if let Some(s) = self.session.as_mut() {
            s.reset(curriculum);
        }
    }

    /// Одно нажатие «снаружи»: используется тестами и повтором сценариев.
    pub fn press(&mut self, key: Key) {
        self.feed_key(key);
    }

    fn feed_key(&mut self, key: Key) {
        // двойной Esc в нормальном режиме — выход в меню
        if key == Key::Esc {
            let now = Instant::now();
            let double = self
                .last_escape
                .map(|t| now.duration_since(t).as_millis() < 600)
                .unwrap_or(false);
            let in_normal = self
                .session
                .as_ref()
                .map(|s| s.engine.mode == Mode::Normal && s.engine.cmdline.is_none())
                .unwrap_or(true);
            self.last_escape = Some(now);
            if double && in_normal {
                self.leave_lesson();
                return;
            }
        }

        let Some(session) = self.session.as_mut() else { return };
        if !matches!(session.state, AttemptState::InProgress) {
            return;
        }
        let result = session.engine.feed(key);
        session.last_key = Some(key);
        if session.engine.last_touched.is_some() {
            session.flash = 1.0;
        }
        match result {
            StepResult::Unsupported(what) => {
                self.toast = Some((what, Instant::now()));
                return;
            }
            StepResult::Ignored => {}
            _ => {}
        }
        session.pressed.push(key);
        self.check_state();
    }

    fn check_state(&mut self) {
        let mode = self.settings.check_mode;
        let Some(session) = self.session.as_mut() else { return };
        let Some(lesson) = self.curriculum.lesson(&session.lesson_id) else { return };
        let Some(ex) = lesson.exercises.get(session.index) else { return };
        let lines = session.engine.buf.to_lines();
        let state = evaluate(
            mode,
            &session.pressed,
            ex,
            &lines,
            session.engine.buf.cursor,
            session.engine.mode == Mode::Normal,
        );
        if state != AttemptState::InProgress {
            session.state = state.clone();
            let elapsed = session.started.elapsed().as_millis() as u64;
            let (success, stars) = match &state {
                AttemptState::Success { stars } => (true, *stars),
                _ => (false, 0),
            };
            if !session.logged {
                session.logged = true;
                let attempt = Attempt {
                    ts: chrono::Local::now().to_rfc3339(),
                    lesson_id: session.lesson_id.clone(),
                    exercise_index: session.index,
                    mode,
                    keystrokes: keys_notation(&session.pressed),
                    keystroke_count: session.pressed.len(),
                    optimal: ex.optimal_keystrokes(),
                    success,
                    stars,
                    elapsed_ms: elapsed,
                    canonical: ex.taught().to_string(),
                };
                self.journal.append(attempt);
                self.progress = Progress::from_journal(&self.journal);
                self.update_auto_keyboard();
            }
        }
    }

    pub fn next_exercise(&mut self) {
        let Some(session) = self.session.as_ref() else { return };
        let lesson_id = session.lesson_id.clone();
        let next = session.index + 1;
        let count = self
            .curriculum
            .lesson(&lesson_id)
            .map(|l| l.exercises.len())
            .unwrap_or(0);
        if next < count {
            self.session = Session::new(&self.curriculum, &lesson_id, next);
        } else {
            self.leave_lesson();
        }
    }

    pub fn retry_exercise(&mut self) {
        let Some(session) = self.session.as_ref() else { return };
        let (lesson_id, index) = (session.lesson_id.clone(), session.index);
        self.session = Session::new(&self.curriculum, &lesson_id, index);
    }

    pub fn leave_lesson(&mut self) {
        self.session = None;
        self.screen = Screen::Map;
    }

    pub fn reset_progress(&mut self) {
        self.journal.clear();
        self.progress = Progress::from_journal(&self.journal);
        self.keyboard_auto_hidden = false;
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let p = self.palette();
        theme::apply(&ctx, p, self.settings.theme == Theme::Dark);

        self.handle_input(&ctx);

        if let Some(session) = self.session.as_mut() {
            if session.flash > 0.0 {
                session.flash = (session.flash - ctx.input(|i| i.stable_dt) * 2.5).max(0.0);
                ctx.request_repaint();
            }
        }
        if let Some((_, at)) = &self.toast {
            if at.elapsed().as_secs_f32() > 3.0 {
                self.toast = None;
            } else {
                ctx.request_repaint();
            }
        }

        match self.screen.clone() {
            Screen::Onboarding { step } => crate::ui::onboarding::show(self, ui, step),
            Screen::Map => crate::ui::map::show(self, ui),
            Screen::Lesson { .. } => crate::ui::lesson::show(self, ui),
            Screen::Stats => crate::ui::stats::show(self, ui),
            Screen::Settings => crate::ui::settings_screen::show(self, ui),
        }
    }
}
