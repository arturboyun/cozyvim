//! Статистика. Самый полезный блок здесь — слабые места: он единственный
//! меняет поведение, а не просто хвалит.

use eframe::egui;
use rust_i18n::t;

use crate::app::App;
use crate::ui::{map::top_bar, widgets::{card, column}};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette();
    top_bar(app, ui);
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            column(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                tile(ui, app, &t!("stats.stars"), &format!("{}", app.progress.total_stars()), p.yellow);
                tile(
                    ui,
                    app,
                    &t!("stats.exercises"),
                    &format!("{}", app.progress.exercises_done()),
                    p.green,
                );
                tile(ui, app, &t!("stats.streak"), &format!("{}", app.progress.streak()), p.orange);
            });
            ui.add_space(10.0);

            card(ui, p, |ui| {
                ui.label(egui::RichText::new(t!("stats.problems")).size(18.0).strong());
                let problems = app.progress.problem_commands(6);
                if problems.is_empty() {
                    ui.colored_label(p.dim, t!("stats.problems_empty"));
                } else {
                    ui.colored_label(p.dim, t!("stats.problems_hint"));
                    ui.add_space(6.0);
                    let worst = problems.first().map(|x| x.1).unwrap_or(1).max(1);
                    for (command, count) in problems {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&command).monospace().color(p.red));
                            let frac = count as f32 / worst as f32;
                            bar(ui, p.red, frac, 180.0);
                            ui.colored_label(p.faint, t!("stats.attempts", n = count));
                        });
                    }
                }
            });
            ui.add_space(10.0);

            card(ui, p, |ui| {
                ui.label(egui::RichText::new(t!("stats.efficiency")).size(18.0).strong());
                let attempts: Vec<_> =
                    app.journal.attempts.iter().rev().take(20).rev().collect();
                if attempts.is_empty() {
                    ui.colored_label(p.dim, t!("stats.efficiency_empty"));
                } else {
                    for a in attempts {
                        let ratio = a.keystroke_count as f32 / a.optimal.max(1) as f32;
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&a.lesson_id).monospace().small().color(p.dim),
                            );
                            let color = if ratio <= 1.0 {
                                p.green
                            } else if ratio <= 1.5 {
                                p.yellow
                            } else {
                                p.orange
                            };
                            bar(ui, color, (ratio / 3.0).min(1.0), 160.0);
                            ui.colored_label(
                                p.faint,
                                format!("{} / {}", a.keystroke_count, a.optimal),
                            );
                        });
                    }
                }
            });
            ui.add_space(10.0);

            card(ui, p, |ui| {
                ui.label(egui::RichText::new(t!("stats.activity")).size(18.0).strong());
                let days = app.progress.days_active();
                if days.is_empty() {
                    ui.colored_label(p.dim, "—");
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for day in days {
                            ui.label(
                                egui::RichText::new(day)
                                    .monospace()
                                    .small()
                                    .color(p.green),
                            );
                        }
                    });
                }
            });
            ui.add_space(24.0);
        });
        });
    });
}

fn tile(ui: &mut egui::Ui, app: &App, label: &str, value: &str, color: egui::Color32) {
    let p = app.palette();
    let width = (ui.available_width() - 20.0) / 3.0;
    egui::Frame::new()
        .fill(p.bg1)
        .corner_radius(egui::CornerRadius::same(14))
        .stroke(egui::Stroke::new(1.0, p.bg2))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(width - 32.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(value).size(30.0).color(color));
                ui.colored_label(p.dim, label);
            });
        });
}

fn bar(ui: &mut egui::Ui, color: egui::Color32, frac: f32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 10.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, egui::CornerRadius::same(5), color.gamma_multiply(0.18));
    let filled = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width() * frac.clamp(0.02, 1.0), rect.height()),
    );
    painter.rect_filled(filled, egui::CornerRadius::same(5), color);
}
