//! Палитра Everforest и стиль egui. Цвета живут одной структурой, поэтому
//! добавить ещё одну тему — значит добавить ещё одну константу.

use eframe::egui::{self, Color32, CornerRadius, Stroke};

use crate::core::settings::Theme;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg0: Color32,
    pub bg1: Color32,
    pub bg2: Color32,
    pub bg3: Color32,
    pub fg: Color32,
    pub dim: Color32,
    pub faint: Color32,
    pub red: Color32,
    pub orange: Color32,
    pub yellow: Color32,
    pub green: Color32,
    pub aqua: Color32,
    pub blue: Color32,
    pub purple: Color32,
}

const fn c(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, ((hex >> 8) & 0xff) as u8, (hex & 0xff) as u8)
}

pub const EVERFOREST_DARK: Palette = Palette {
    bg0: c(0x2d353b),
    bg1: c(0x343f44),
    bg2: c(0x3d484d),
    bg3: c(0x475258),
    fg: c(0xd3c6aa),
    dim: c(0x9da9a0),
    faint: c(0x7a8478),
    red: c(0xe67e80),
    orange: c(0xe69875),
    yellow: c(0xdbbc7f),
    green: c(0xa7c080),
    aqua: c(0x83c092),
    blue: c(0x7fbbb3),
    purple: c(0xd699b6),
};

pub const EVERFOREST_LIGHT: Palette = Palette {
    bg0: c(0xfdf6e3),
    bg1: c(0xf4f0d9),
    bg2: c(0xefebd4),
    bg3: c(0xe6e2cc),
    fg: c(0x5c6a72),
    dim: c(0x829181),
    faint: c(0xa6b0a0),
    red: c(0xf85552),
    orange: c(0xf57d26),
    yellow: c(0xdfa000),
    green: c(0x8da101),
    aqua: c(0x35a77c),
    blue: c(0x3a94c5),
    purple: c(0xdf69ba),
};

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Dark => EVERFOREST_DARK,
        Theme::Light => EVERFOREST_LIGHT,
    }
}

/// Шрифты: моно для буфера и клавиш, мягкий гротеск для всего остального.
pub fn install_fonts(ctx: &egui::Context) {
    use eframe::egui::{FontData, FontDefinitions, FontFamily};
    use std::sync::Arc;

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "ui".to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/Inter-Variable.ttf"))),
    );
    fonts.font_data.insert(
        "mono".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "mono-bold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"))),
    );
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "ui".to_owned());
    fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "mono".to_owned());
    fonts
        .families
        .entry(FontFamily::Name("mono-bold".into()))
        .or_default()
        .insert(0, "mono-bold".to_owned());
    ctx.set_fonts(fonts);
}

pub fn text_styles(ctx: &egui::Context) {
    use eframe::egui::{FontFamily, FontId, TextStyle};
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::new(26.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(16.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(13.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(18.0, FontFamily::Monospace)),
        ]
        .into();
    });
}

/// Уют делается щедрыми отступами, скруглениями и отсутствием резких границ.
pub fn apply(ctx: &egui::Context, p: Palette, dark: bool) {
    let mut visuals = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let radius = CornerRadius::same(10);

    visuals.panel_fill = p.bg0;
    visuals.window_fill = p.bg1;
    visuals.extreme_bg_color = p.bg1;
    visuals.faint_bg_color = p.bg1;
    visuals.override_text_color = Some(p.fg);
    visuals.window_stroke = Stroke::new(1.0, p.bg3);
    visuals.window_corner_radius = CornerRadius::same(14);
    visuals.selection.bg_fill = p.green.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, p.fg);
    visuals.hyperlink_color = p.blue;

    for (w, fill) in [
        (&mut visuals.widgets.noninteractive, p.bg1),
        (&mut visuals.widgets.inactive, p.bg2),
        (&mut visuals.widgets.hovered, p.bg3),
        (&mut visuals.widgets.active, p.bg3),
        (&mut visuals.widgets.open, p.bg2),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.corner_radius = radius;
        w.bg_stroke = Stroke::new(1.0, p.bg3);
        w.fg_stroke = Stroke::new(1.0, p.fg);
        w.expansion = 0.0;
    }
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.green.gamma_multiply(0.6));
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, p.green);

    ctx.set_visuals(visuals);
    ctx.all_styles_mut(|style| {
        style.visuals.selection.bg_fill = p.green.gamma_multiply(0.30);
        style.visuals.selection.stroke = Stroke::new(1.0, p.green);
        style.visuals.override_text_color = Some(p.fg);
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(16);
        style.spacing.interact_size.y = 32.0;
    });
}
