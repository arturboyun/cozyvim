//! Экран упражнения: задание, буфер, экранная клавиатура. Вертикальный стек —
//! взгляд не должен метаться, он должен быть на буфере.

use eframe::egui;
use rust_i18n::t;

use crate::app::App;
use crate::core::grading::AttemptState;
use crate::core::keys::keys_notation;
use crate::core::settings::CheckMode;
use crate::ui::{
    keyboard,
    widgets::{column, self, BufferView, card, mode_badge, pill},
};
use crate::vim::engine::Mode;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    let lang = app.settings.lang.code().to_string();

    let Some(session) = app.session.as_ref() else {
        app.leave_lesson();
        return;
    };
    let Some(lesson) = app.curriculum.lesson(&session.lesson_id).cloned() else {
        app.leave_lesson();
        return;
    };
    let Some(exercise) = lesson.exercises.get(session.index).cloned() else {
        app.leave_lesson();
        return;
    };
    let lesson_text = lesson.text(&lang);
    let ex_text = exercise.text(&lang);
    let index = session.index;
    let total = lesson.exercises.len();
    let pressed = session.pressed.clone();
    let state = session.state.clone();
    let hint_shown = session.hint_shown;
    let lines = session.engine.buf.to_lines();
    let cursor = session.engine.buf.cursor;
    let mode = session.engine.mode;
    let selection = session.engine.visual_range();
    let touched = session.engine.last_touched;
    let flash = session.flash;
    let last_key = session.last_key;
    let cmdline = session.engine.cmdline.clone();
    let expected = app.expected_key();

    // ---------- верх: где мы и сколько сделано ----------
    egui::Panel::top("lesson_top")
        .frame(egui::Frame::new().fill(p.bg0).inner_margin(egui::Margin::symmetric(20, 12)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button(format!("← {}", t!("lesson.menu"))).clicked() {
                    app.leave_lesson();
                }
                ui.add_space(8.0);
                ui.label(egui::RichText::new(&lesson_text.title).size(19.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mode_label = match mode {
                        Mode::Normal => t!("lesson.mode_normal"),
                        Mode::Insert => t!("lesson.mode_insert"),
                        Mode::Visual => t!("lesson.mode_visual"),
                        Mode::VisualLine => t!("lesson.mode_vline"),
                    };
                    mode_badge(ui, p, mode, &mode_label);
                    ui.colored_label(
                        p.dim,
                        t!("lesson.exercise", i = index + 1, n = total),
                    );
                    let stars = app.progress.stars(&lesson.id, index);
                    widgets::stars(ui, p, stars);
                });
            });
        });

    // ---------- низ: клавиатура и панель управления ----------
    egui::Panel::bottom("lesson_bottom")
        .frame(egui::Frame::new().fill(p.bg0).inner_margin(egui::Margin::symmetric(20, 12)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(p.faint, t!("lesson.pressed"));
                let text = if pressed.is_empty() { "—".to_string() } else { keys_notation(&pressed) };
                ui.label(egui::RichText::new(text).monospace().color(p.aqua));
                if let Some((query, forward)) = &cmdline {
                    let prefix = if *forward { "/" } else { "?" };
                    ui.label(
                        egui::RichText::new(format!("{prefix}{query}")).monospace().color(p.orange),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("{} · F5", t!("lesson.reset"))).clicked() {
                        app.reset_exercise();
                    }
                    if ui
                        .selectable_label(hint_shown, format!("{} · F1", t!("lesson.show_hint")))
                        .clicked()
                    {
                        app.toggle_hint();
                    }
                });
            });
            if app.keyboard_visible() {
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    keyboard::show(ui, p, last_key, expected);
                });
            } else if app.keyboard_auto_hidden {
                ui.colored_label(p.faint, t!("lesson.keyboard_hidden"));
            }
        });

    // ---------- центр: задание, буфер, результат ----------
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            column(ui, |ui| {
            ui.add_space(4.0);
            if index == 0 && !lesson_text.intro.is_empty() {
                card(ui, p, |ui| {
                    ui.label(egui::RichText::new(lesson_text.intro.trim()).color(p.dim));
                });
                ui.add_space(6.0);
            }
            card(ui, p, |ui| {
                ui.label(egui::RichText::new(&ex_text.task).size(18.0));
                if hint_shown && !ex_text.hint.is_empty() {
                    ui.add_space(6.0);
                    ui.colored_label(p.yellow, format!("💡 {}", ex_text.hint));
                }
            });
            ui.add_space(10.0);

            widgets::buffer_view(
                ui,
                p,
                BufferView {
                    lines: &lines,
                    cursor,
                    mode,
                    selection,
                    touched,
                    flash,
                    show_line_numbers: lines.len() > 1,
                },
            );

            if exercise.goal_cursor.is_none() && lines != exercise.goal {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.colored_label(p.faint, format!("{}:", t!("lesson.goal")));
                    ui.label(
                        egui::RichText::new(exercise.goal.join(" ⏎ ")).monospace().color(p.faint),
                    );
                });
            }

            ui.add_space(12.0);
            match &state {
                AttemptState::InProgress => {}
                AttemptState::Success { stars } => {
                    card(ui, p, |ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                p.green,
                                egui::RichText::new(format!("✓ {}", t!("lesson.success")))
                                    .size(20.0),
                            );
                            widgets::stars(ui, p, *stars);
                        });
                        ui.add_space(4.0);
                        ui.colored_label(
                            p.dim,
                            t!(
                                "lesson.optimal",
                                keys = exercise.canonical(),
                                n = exercise.optimal_keystrokes()
                            ),
                        );
                        ui.colored_label(p.dim, t!("lesson.your_keys", n = pressed.len()));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let label = if index + 1 < total {
                                t!("lesson.next")
                            } else {
                                t!("lesson.finish")
                            };
                            if ui.button(egui::RichText::new(label).size(17.0)).clicked() {
                                app.next_exercise();
                            }
                            if ui.button(t!("lesson.retry")).clicked() {
                                app.retry_exercise();
                            }
                        });
                    });
                }
                AttemptState::Failed { expected } => {
                    card(ui, p, |ui| {
                        ui.colored_label(
                            p.red,
                            egui::RichText::new(format!("✗ {}", t!("lesson.failed"))).size(20.0),
                        );
                        ui.colored_label(p.dim, t!("lesson.expected", keys = expected));
                        ui.colored_label(
                            p.faint,
                            format!("{}: {}", t!("lesson.pressed"), keys_notation(&pressed)),
                        );
                        ui.add_space(8.0);
                        if ui.button(egui::RichText::new(t!("lesson.retry")).size(17.0)).clicked() {
                            app.retry_exercise();
                        }
                    });
                }
            }

            if app.settings.check_mode == CheckMode::Strict {
                ui.add_space(6.0);
                ui.horizontal(|ui| pill(ui, p, &t!("mode.strict"), p.purple));
            }

            if let Some((what, _)) = &app.toast {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    pill(ui, p, &format!("{} ({what})", t!("lesson.unsupported")), p.orange);
                });
            }
            ui.add_space(24.0);
        });
        });
    });
}
