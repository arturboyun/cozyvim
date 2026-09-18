//! Мелкие кирпичики интерфейса: карточка, звёзды, окно буфера.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Vec2};

use super::theme::Palette;
use crate::vim::engine::Mode;

/// Колонка содержимого: ограничиваем ширину и центрируем — длинные строки
/// текста читать тяжело, а воздух по краям и есть половина уюта.
pub fn column<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let width = ui.available_width().min(900.0);
    let pad = ((ui.available_width() - width) / 2.0).max(0.0);
    let height = ui.available_height();
    ui.horizontal_top(|ui| {
        ui.add_space(pad);
        ui.allocate_ui_with_layout(
            egui::vec2(width, height),
            egui::Layout::top_down(egui::Align::Min),
            add,
        )
        .inner
    })
    .inner
}

/// Мягкая карточка — основной приём «уюта»: нет резких границ, есть воздух.
pub fn card<R>(ui: &mut egui::Ui, p: Palette, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(p.bg1)
        .stroke(Stroke::new(1.0, p.bg2))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn stars_label(earned: u8) -> String {
    let full = "★".repeat(earned as usize);
    let empty = "☆".repeat(3usize.saturating_sub(earned as usize));
    format!("{full}{empty}")
}

pub fn stars(ui: &mut egui::Ui, p: Palette, earned: u8) {
    let color = if earned > 0 { p.yellow } else { p.faint };
    ui.colored_label(color, stars_label(earned));
}

pub struct BufferView<'a> {
    pub lines: &'a [String],
    pub cursor: (usize, usize),
    pub mode: Mode,
    pub selection: Option<((usize, usize), (usize, usize))>,
    /// Строки, которых коснулась последняя команда, и яркость подсветки 0..1.
    pub touched: Option<(usize, usize)>,
    pub flash: f32,
    pub show_line_numbers: bool,
}

/// Окно буфера. Подсветка изменённых строк — самая полезная анимация:
/// она показывает, что именно сделала команда.
pub fn buffer_view(ui: &mut egui::Ui, p: Palette, view: BufferView<'_>) {
    let font = FontId::monospace(19.0);
    let (char_w, line_h) = ui.ctx().fonts_mut(|f| {
        let w = f.glyph_width(&font, 'M');
        let h = f.row_height(&font);
        (w, h + 4.0)
    });
    let gutter = if view.show_line_numbers { char_w * 4.0 } else { 0.0 };
    let rows = view.lines.len().max(1);
    let height = rows as f32 * line_h + 24.0;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(14), p.bg1);
    painter.rect_stroke(
        rect,
        CornerRadius::same(14),
        Stroke::new(1.0, p.bg2),
        egui::StrokeKind::Inside,
    );

    let text_left = rect.left() + 16.0 + gutter;
    let top = rect.top() + 12.0;

    for (i, line) in view.lines.iter().enumerate() {
        let y = top + i as f32 * line_h;
        if let Some((from, to)) = view.touched {
            if i >= from && i <= to && view.flash > 0.0 {
                let band = Rect::from_min_size(
                    egui::pos2(rect.left() + 8.0, y - 2.0),
                    Vec2::new(rect.width() - 16.0, line_h),
                );
                painter.rect_filled(
                    band,
                    CornerRadius::same(6),
                    p.green.gamma_multiply(0.22 * view.flash),
                );
            }
        }
        if let Some((start, end)) = view.selection {
            if i >= start.0 && i <= end.0 {
                let chars: Vec<char> = line.chars().collect();
                let from = if i == start.0 { start.1 } else { 0 };
                let to = if i == end.0 { end.1 + 1 } else { chars.len().max(1) };
                let to = to.min(chars.len().max(1));
                if to > from {
                    let sel = Rect::from_min_size(
                        egui::pos2(text_left + from as f32 * char_w, y),
                        Vec2::new((to - from) as f32 * char_w, line_h - 4.0),
                    );
                    painter.rect_filled(sel, CornerRadius::same(3), p.blue.gamma_multiply(0.30));
                }
            }
        }
        if view.show_line_numbers {
            painter.text(
                egui::pos2(rect.left() + 12.0 + gutter - char_w * 1.5, y),
                Align2::RIGHT_TOP,
                format!("{}", i + 1),
                font.clone(),
                p.faint,
            );
        }
        painter.text(
            egui::pos2(text_left, y),
            Align2::LEFT_TOP,
            line,
            font.clone(),
            p.fg,
        );
    }

    // курсор: блок в нормальном режиме, вертикальная черта в режиме вставки
    let (row, col) = view.cursor;
    let cx = text_left + col as f32 * char_w;
    let cy = top + row as f32 * line_h;
    let cursor_color = match view.mode {
        Mode::Insert => p.orange,
        Mode::Visual | Mode::VisualLine => p.blue,
        Mode::Normal => p.green,
    };
    if view.mode == Mode::Insert {
        painter.rect_filled(
            Rect::from_min_size(egui::pos2(cx - 1.0, cy), Vec2::new(2.0, line_h - 4.0)),
            CornerRadius::ZERO,
            cursor_color,
        );
    } else {
        let cell = Rect::from_min_size(egui::pos2(cx, cy), Vec2::new(char_w, line_h - 4.0));
        painter.rect_filled(cell, CornerRadius::same(3), cursor_color.gamma_multiply(0.85));
        let ch = view
            .lines
            .get(row)
            .and_then(|l| l.chars().nth(col))
            .unwrap_or(' ');
        painter.text(cell.left_top(), Align2::LEFT_TOP, ch, font, p.bg0);
    }
}

pub fn mode_badge(ui: &mut egui::Ui, p: Palette, mode: Mode, label: &str) {
    let color = match mode {
        Mode::Normal => p.green,
        Mode::Insert => p.orange,
        Mode::Visual | Mode::VisualLine => p.blue,
    };
    let text = egui::RichText::new(format!(" {label} "))
        .monospace()
        .size(13.0)
        .color(p.bg0)
        .background_color(color);
    ui.label(text);
}

pub fn pill(ui: &mut egui::Ui, p: Palette, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .corner_radius(CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.5)))
        .show(ui, |ui| {
            ui.colored_label(if color == p.fg { p.fg } else { color }, text);
        });
}
