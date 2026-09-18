//! Карта глав. Главы открываются по порядку, уроки внутри главы — все сразу.

use eframe::egui;
use rust_i18n::t;

use crate::app::App;
use crate::ui::{
    Screen,
    widgets::{column, card, pill, stars_label},
};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    top_bar(app, ui);
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            column(ui, |ui| {
            ui.add_space(8.0);
            let chapters = app.curriculum.chapters.clone();
            let lang = app.settings.lang.code().to_string();
            for chapter in chapters {
                let text = chapter.text(&lang);
                let unlocked = app.chapter_unlocked(&chapter.id);
                card(ui, p, |ui| {
                    ui.horizontal(|ui| {
                        let title_color = if unlocked && !chapter.coming_soon { p.fg } else { p.faint };
                        ui.label(
                            egui::RichText::new(&text.title).size(20.0).color(title_color).strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if chapter.coming_soon {
                                pill(ui, p, &t!("map.coming_soon"), p.blue);
                            } else if !unlocked {
                                pill(ui, p, &t!("map.locked"), p.faint);
                            }
                        });
                    });
                    ui.label(egui::RichText::new(&text.subtitle).color(p.dim));
                    if chapter.coming_soon {
                        return;
                    }
                    ui.add_space(8.0);
                    let lessons: Vec<_> = app
                        .curriculum
                        .lessons_of(&chapter.id)
                        .into_iter()
                        .cloned()
                        .collect();
                    for lesson in lessons {
                        let lesson_text = lesson.text(&lang);
                        let (earned, max) = app.progress.lesson_stars(&app.curriculum, &lesson.id);
                        let done = app.progress.lesson_done(&app.curriculum, &lesson.id);
                        ui.horizontal(|ui| {
                            let label = egui::RichText::new(&lesson_text.title).size(16.0);
                            let button = egui::Button::new(if done {
                                label.color(p.green)
                            } else {
                                label
                            })
                            .min_size(egui::vec2(300.0, 34.0))
                            .wrap_mode(egui::TextWrapMode::Extend);
                            if ui.add_enabled(unlocked, button).clicked() {
                                app.open_lesson(&lesson.id);
                            }
                            ui.colored_label(
                                if earned > 0 { p.yellow } else { p.faint },
                                format!("{earned}/{max} ★"),
                            );
                            ui.colored_label(
                                p.faint,
                                t!("map.exercises", count = lesson.exercises.len()),
                            );
                        });
                    }
                });
                ui.add_space(6.0);
            }
            ui.add_space(24.0);
        });
        });
    });
    let _ = stars_label(0);
}

pub fn top_bar(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    egui::Panel::top("top").frame(
        egui::Frame::new().fill(p.bg0).inner_margin(egui::Margin::symmetric(16, 10)),
    ).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(t!("app.name"))
                    .size(20.0)
                    .color(p.green)
                    .family(egui::FontFamily::Monospace),
            );
            ui.add_space(12.0);
            if ui.selectable_label(app.screen == Screen::Map, t!("nav.map")).clicked() {
                app.screen = Screen::Map;
            }
            if ui.selectable_label(app.screen == Screen::Stats, t!("nav.stats")).clicked() {
                app.screen = Screen::Stats;
            }
            if ui.selectable_label(app.screen == Screen::Settings, t!("nav.settings")).clicked() {
                app.screen = Screen::Settings;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(p.yellow, format!("{} ★", app.progress.total_stars()));
                let streak = app.progress.streak();
                if streak > 0 {
                    ui.colored_label(p.orange, format!("{streak} 🔥"));
                }
            });
        });
    });
}
