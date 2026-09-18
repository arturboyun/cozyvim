//! Настройки. Всё применяется сразу и пишется на диск — кнопки «сохранить» нет.

use eframe::egui;
use rust_i18n::t;

use crate::app::App;
use crate::core::settings::{CheckMode, KeyboardVisibility, Lang, Theme};
use crate::ui::{map::top_bar, widgets::{card, column}};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    top_bar(app, ui);
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            column(ui, |ui| {
            ui.add_space(8.0);
            card(ui, p, |ui| {
                ui.label(egui::RichText::new(t!("settings.lang")).size(18.0).strong());
                ui.horizontal(|ui| {
                    for lang in [Lang::Ru, Lang::En] {
                        if ui.selectable_label(app.settings.lang == lang, lang.label()).clicked() {
                            app.settings.lang = lang;
                            app.save_settings();
                        }
                    }
                });
            });
            ui.add_space(8.0);

            card(ui, p, |ui| {
                ui.label(egui::RichText::new(t!("settings.mode")).size(18.0).strong());
                for (mode, label, sub) in [
                    (CheckMode::Mix, t!("mode.mix"), t!("mode.mix_sub")),
                    (CheckMode::Strict, t!("mode.strict"), t!("mode.strict_sub")),
                    (CheckMode::Free, t!("mode.free"), t!("mode.free_sub")),
                ] {
                    if ui.selectable_label(app.settings.check_mode == mode, label).clicked() {
                        app.settings.check_mode = mode;
                        app.save_settings();
                    }
                    ui.label(egui::RichText::new(sub).small().color(p.dim));
                    ui.add_space(4.0);
                }
            });
            ui.add_space(8.0);

            card(ui, p, |ui| {
                ui.label(egui::RichText::new(t!("settings.theme")).size(18.0).strong());
                ui.horizontal(|ui| {
                    for (theme, label) in
                        [(Theme::Dark, t!("settings.theme_dark")), (Theme::Light, t!("settings.theme_light"))]
                    {
                        if ui.selectable_label(app.settings.theme == theme, label).clicked() {
                            app.settings.theme = theme;
                            app.save_settings();
                        }
                    }
                });
            });
            ui.add_space(8.0);

            card(ui, p, |ui| {
                ui.label(egui::RichText::new(t!("settings.keyboard")).size(18.0).strong());
                ui.horizontal(|ui| {
                    for (vis, label) in [
                        (KeyboardVisibility::Auto, t!("settings.keyboard_auto")),
                        (KeyboardVisibility::Always, t!("settings.keyboard_always")),
                        (KeyboardVisibility::Never, t!("settings.keyboard_never")),
                    ] {
                        if ui.selectable_label(app.settings.keyboard == vis, label).clicked() {
                            app.settings.keyboard = vis;
                            app.keyboard_open = vis != KeyboardVisibility::Never;
                            app.save_settings();
                        }
                    }
                });
            });
            ui.add_space(8.0);

            card(ui, p, |ui| {
                if !app.confirm_reset {
                    if ui
                        .button(egui::RichText::new(t!("settings.reset_progress")).color(p.red))
                        .clicked()
                    {
                        app.confirm_reset = true;
                    }
                } else {
                    ui.colored_label(p.red, t!("settings.reset_confirm"));
                    ui.horizontal(|ui| {
                        if ui.button(t!("settings.reset_yes")).clicked() {
                            app.reset_progress();
                            app.confirm_reset = false;
                        }
                        if ui.button(t!("settings.reset_no")).clicked() {
                            app.confirm_reset = false;
                        }
                    });
                }
            });
            ui.add_space(24.0);
        });
        });
    });
}
