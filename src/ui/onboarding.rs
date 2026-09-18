//! Первый запуск: язык, опыт, режим проверки, готово. Четыре коротких экрана —
//! это не трение, а первое впечатление.

use eframe::egui;
use rust_i18n::t;

use crate::app::App;
use crate::core::settings::{CheckMode, Experience, Lang};
use crate::ui::{Screen, widgets::card};

pub fn show(app: &mut App, ui: &mut egui::Ui, step: usize) {
    let p = app.palette();
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
            ui.label(
                egui::RichText::new(t!("app.name"))
                    .size(40.0)
                    .color(p.green)
                    .family(egui::FontFamily::Monospace),
            );
            ui.label(egui::RichText::new(t!("app.tagline")).color(p.dim));
            ui.add_space(28.0);

            ui.allocate_ui_with_layout(
                egui::vec2(520.0, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| match step {
                    0 => lang_step(app, ui),
                    1 => experience_step(app, ui),
                    2 => mode_step(app, ui),
                    _ => ready_step(app, ui),
                },
            );
        });
    });
}

fn heading(ui: &mut egui::Ui, app: &App, title: &str, sub: &str) {
    let p = app.palette();
    ui.label(egui::RichText::new(title).heading());
    ui.label(egui::RichText::new(sub).color(p.dim));
    ui.add_space(12.0);
}

fn lang_step(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    card(ui, p, |ui| {
        heading(ui, app, &t!("onboarding.lang_title"), &t!("onboarding.lang_sub"));
        for lang in [Lang::Ru, Lang::En] {
            let selected = app.settings.lang == lang;
            if ui.selectable_label(selected, lang.label()).clicked() {
                app.settings.lang = lang;
                app.save_settings();
            }
        }
        ui.add_space(12.0);
        if ui.button(t!("onboarding.next")).clicked() {
            app.screen = Screen::Onboarding { step: 1 };
        }
    });
}

fn experience_step(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    card(ui, p, |ui| {
        heading(ui, app, &t!("onboarding.exp_title"), &t!("onboarding.exp_sub"));
        let options = [
            (Experience::Newcomer, t!("onboarding.exp_newcomer"), t!("onboarding.exp_newcomer_sub")),
            (Experience::Dabbler, t!("onboarding.exp_dabbler"), t!("onboarding.exp_dabbler_sub")),
            (
                Experience::Confident,
                t!("onboarding.exp_confident"),
                t!("onboarding.exp_confident_sub"),
            ),
        ];
        for (exp, label, sub) in options {
            let selected = app.settings.experience == exp;
            let response = ui.selectable_label(selected, egui::RichText::new(label).size(17.0));
            ui.label(egui::RichText::new(sub).small().color(p.dim));
            ui.add_space(6.0);
            if response.clicked() {
                app.settings.experience = exp;
                // Уверенным сразу ставим строгую проверку — они за ней и пришли.
                app.settings.check_mode = match exp {
                    Experience::Confident => CheckMode::Strict,
                    _ => CheckMode::Mix,
                };
                app.save_settings();
            }
        }
        ui.add_space(12.0);
        if ui.button(t!("onboarding.next")).clicked() {
            app.screen = Screen::Onboarding { step: 2 };
        }
    });
}

fn mode_step(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    card(ui, p, |ui| {
        heading(ui, app, &t!("onboarding.mode_title"), &t!("onboarding.mode_sub"));
        for (mode, label, sub) in [
            (CheckMode::Mix, t!("mode.mix"), t!("mode.mix_sub")),
            (CheckMode::Strict, t!("mode.strict"), t!("mode.strict_sub")),
            (CheckMode::Free, t!("mode.free"), t!("mode.free_sub")),
        ] {
            let selected = app.settings.check_mode == mode;
            if ui.selectable_label(selected, egui::RichText::new(label).size(17.0)).clicked() {
                app.settings.check_mode = mode;
                app.save_settings();
            }
            ui.label(egui::RichText::new(sub).small().color(p.dim));
            ui.add_space(6.0);
        }
        ui.add_space(12.0);
        if ui.button(t!("onboarding.next")).clicked() {
            app.screen = Screen::Onboarding { step: 3 };
        }
    });
}

fn ready_step(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    card(ui, p, |ui| {
        heading(ui, app, &t!("onboarding.ready_title"), &t!("onboarding.ready_sub"));
        if ui.button(egui::RichText::new(t!("onboarding.start")).size(18.0)).clicked() {
            app.settings.onboarded = true;
            app.save_settings();
            let first = app
                .curriculum
                .lessons
                .iter()
                .find(|l| l.chapter == "motion")
                .map(|l| l.id.clone());
            match first {
                Some(id) => app.open_lesson(&id),
                None => app.screen = Screen::Map,
            }
        }
    });
}
