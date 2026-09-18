//! Экранная клавиатура: физическая раскладка ANSI со смещением рядов.
//! Смещение важно — мышечная память строится на реальных позициях, а не на
//! ровной сетке. Пальцевые зоны подкрашены, чтобы глаз привыкал к делению.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Vec2};

use super::theme::Palette;
use crate::core::keys::Key;

struct KeyCap {
    label: &'static str,
    /// Символ, который эта клавиша порождает без Shift.
    ch: Option<char>,
    /// Ширина в клавишных единицах.
    width: f32,
}

const fn cap(label: &'static str, ch: char) -> KeyCap {
    KeyCap { label, ch: Some(ch), width: 1.0 }
}

const fn wide(label: &'static str, width: f32) -> KeyCap {
    KeyCap { label, ch: None, width }
}

fn rows() -> Vec<(f32, Vec<KeyCap>)> {
    vec![
        (
            0.0,
            vec![
                cap("`", '`'),
                cap("1", '1'),
                cap("2", '2'),
                cap("3", '3'),
                cap("4", '4'),
                cap("5", '5'),
                cap("6", '6'),
                cap("7", '7'),
                cap("8", '8'),
                cap("9", '9'),
                cap("0", '0'),
                cap("-", '-'),
                cap("=", '='),
                wide("⌫", 2.0),
            ],
        ),
        (
            0.0,
            vec![
                wide("⇥", 1.5),
                cap("q", 'q'),
                cap("w", 'w'),
                cap("e", 'e'),
                cap("r", 'r'),
                cap("t", 't'),
                cap("y", 'y'),
                cap("u", 'u'),
                cap("i", 'i'),
                cap("o", 'o'),
                cap("p", 'p'),
                cap("[", '['),
                cap("]", ']'),
                wide("\\", 1.5),
            ],
        ),
        (
            0.0,
            vec![
                wide("esc", 1.75),
                cap("a", 'a'),
                cap("s", 's'),
                cap("d", 'd'),
                cap("f", 'f'),
                cap("g", 'g'),
                cap("h", 'h'),
                cap("j", 'j'),
                cap("k", 'k'),
                cap("l", 'l'),
                cap(";", ';'),
                cap("'", '\''),
                wide("⏎", 2.25),
            ],
        ),
        (
            0.0,
            vec![
                wide("⇧", 2.25),
                cap("z", 'z'),
                cap("x", 'x'),
                cap("c", 'c'),
                cap("v", 'v'),
                cap("b", 'b'),
                cap("n", 'n'),
                cap("m", 'm'),
                cap(",", ','),
                cap(".", '.'),
                cap("/", '/'),
                wide("⇧", 2.75),
            ],
        ),
        (0.0, vec![wide("ctrl", 1.25), wide("alt", 1.25), wide("␣", 8.0), wide("alt", 1.25), wide("ctrl", 1.25)]),
    ]
}

/// Пальцевые зоны слепой печати: 0–3 левая рука от мизинца, 4–7 правая.
fn finger_zone(ch: char) -> usize {
    match ch {
        '`' | '1' | 'q' | 'a' | 'z' => 0,
        '2' | 'w' | 's' | 'x' => 1,
        '3' | 'e' | 'd' | 'c' => 2,
        '4' | '5' | 'r' | 't' | 'f' | 'g' | 'v' | 'b' => 3,
        '6' | '7' | 'y' | 'u' | 'h' | 'j' | 'n' | 'm' => 4,
        '8' | 'i' | 'k' | ',' => 5,
        '9' | 'o' | 'l' | '.' => 6,
        _ => 7,
    }
}

fn zone_color(p: Palette, zone: usize) -> Color32 {
    let base = match zone {
        0 | 7 => p.purple,
        1 | 6 => p.blue,
        2 | 5 => p.aqua,
        _ => p.green,
    };
    base.gamma_multiply(0.20)
}

/// Рисует клавиатуру и подсвечивает последнее нажатие и ожидаемую клавишу.
pub fn show(
    ui: &mut egui::Ui,
    p: Palette,
    last: Option<Key>,
    expected: Option<Key>,
) -> egui::Response {
    let rows = rows();
    let units_wide = 15.0_f32;
    let available = ui.available_width().min(760.0);
    let unit = (available / units_wide).clamp(22.0, 44.0);
    let gap = unit * 0.08;
    let height = rows.len() as f32 * (unit + gap) + gap;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(unit * units_wide, height), Sense::hover());
    let painter = ui.painter_at(rect);

    let last_char = match last {
        Some(Key::Char(c)) => Some(c.to_ascii_lowercase()),
        _ => None,
    };
    let expected_char = match expected {
        Some(Key::Char(c)) => Some(c.to_ascii_lowercase()),
        _ => None,
    };
    let last_special = special_label(last);
    let expected_special = special_label(expected);

    let mut y = rect.top() + gap;
    for (offset, keys) in rows {
        let mut x = rect.left() + offset * unit + gap;
        for k in keys {
            let w = k.width * unit - gap;
            let key_rect = Rect::from_min_size(egui::pos2(x, y), Vec2::new(w, unit - gap));
            let is_last = match (k.ch, last_char) {
                (Some(c), Some(l)) => c == l,
                _ => last_special == Some(k.label),
            };
            let is_expected = match (k.ch, expected_char) {
                (Some(c), Some(e)) => c == e,
                _ => expected_special == Some(k.label),
            };
            let (fill, stroke, text_color) = if is_last {
                (p.green.gamma_multiply(0.85), Stroke::new(1.5, p.green), p.bg0)
            } else if is_expected {
                (p.yellow.gamma_multiply(0.30), Stroke::new(1.5, p.yellow), p.fg)
            } else {
                let z = k.ch.map(finger_zone).unwrap_or(7);
                (zone_color(p, z), Stroke::new(1.0, p.bg3), p.dim)
            };
            painter.rect_filled(key_rect, CornerRadius::same(6), fill);
            painter.rect_stroke(key_rect, CornerRadius::same(6), stroke, egui::StrokeKind::Inside);
            painter.text(
                key_rect.center(),
                Align2::CENTER_CENTER,
                k.label,
                FontId::monospace(unit * 0.40),
                text_color,
            );
            x += k.width * unit;
        }
        y += unit + gap;
    }
    response
}

fn special_label(key: Option<Key>) -> Option<&'static str> {
    match key {
        Some(Key::Esc) => Some("esc"),
        Some(Key::Enter) => Some("⏎"),
        Some(Key::Backspace) => Some("⌫"),
        Some(Key::Tab) => Some("⇥"),
        Some(Key::Char(' ')) => Some("␣"),
        _ => None,
    }
}
