//! Look and feel: fonts, the colour palette for light and dark mode, and egui's
//! widget styling derived from it.

use eframe::egui::{
    self, epaint::Shadow, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Theme, Visuals,
};
use std::sync::Arc;

pub use egui_phosphor::regular as icon;

/// Semibold text (headings, titles, emphasis in the UI).
pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

/// Colours for one theme. Everything in the UI is drawn from these: Gruvbox
/// (https://github.com/morhetz/gruvbox) dark and light.
#[derive(Clone, Copy)]
pub struct Palette {
    /// Note area.
    pub bg: Color32,
    /// Sidebar and other chrome.
    pub panel: Color32,
    /// Popups, dialogs, cards.
    pub surface: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub faint: Color32,
    pub accent: Color32,
    /// Text on top of `accent`.
    pub on_accent: Color32,
    /// Selected rows and the active segment.
    pub accent_soft: Color32,
    pub hover: Color32,
    pub code_bg: Color32,
    pub danger: Color32,
    pub success: Color32,
    // Gruvbox accents, for colour-coded icons and markdown.
    pub red: Color32,
    pub green: Color32,
    pub yellow: Color32,
    pub blue: Color32,
    pub purple: Color32,
    pub aqua: Color32,
    pub orange: Color32,
    pub gray: Color32,
}

/// Gruvbox light: bg0 fbf1c7, bg0_h f9f5d7, bg0_s f2e5bc, bg1 ebdbb2, bg2 d5c4a1,
/// fg1 3c3836, fg4 7c6f64, bg4 a89984; faded accents.
pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xFB, 0xF1, 0xC7),
    panel: Color32::from_rgb(0xF2, 0xE5, 0xBC),
    surface: Color32::from_rgb(0xF9, 0xF5, 0xD7),
    border: Color32::from_rgb(0xD5, 0xC4, 0xA1),
    text: Color32::from_rgb(0x3C, 0x38, 0x36),
    muted: Color32::from_rgb(0x7C, 0x6F, 0x64),
    faint: Color32::from_rgb(0xA8, 0x99, 0x84),
    accent: Color32::from_rgb(0xB5, 0x76, 0x14),
    on_accent: Color32::from_rgb(0xFB, 0xF1, 0xC7),
    accent_soft: Color32::from_rgb(0xD5, 0xC4, 0xA1),
    hover: Color32::from_rgb(0xEB, 0xDB, 0xB2),
    code_bg: Color32::from_rgb(0xF2, 0xE5, 0xBC),
    danger: Color32::from_rgb(0x9D, 0x00, 0x06),
    success: Color32::from_rgb(0x79, 0x74, 0x0E),
    red: Color32::from_rgb(0x9D, 0x00, 0x06),
    green: Color32::from_rgb(0x79, 0x74, 0x0E),
    yellow: Color32::from_rgb(0xB5, 0x76, 0x14),
    blue: Color32::from_rgb(0x07, 0x66, 0x78),
    purple: Color32::from_rgb(0x8F, 0x3F, 0x71),
    aqua: Color32::from_rgb(0x42, 0x7B, 0x58),
    orange: Color32::from_rgb(0xAF, 0x3A, 0x03),
    gray: Color32::from_rgb(0x7C, 0x6F, 0x64),
};

/// Gruvbox dark: bg0 282828, bg0_h 1d2021, bg0_s 32302f, bg1 3c3836, bg2 504945,
/// fg1 ebdbb2, fg4 a89984, bg4 7c6f64; bright accents.
pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x28, 0x28, 0x28),
    panel: Color32::from_rgb(0x1D, 0x20, 0x21),
    surface: Color32::from_rgb(0x32, 0x30, 0x2F),
    border: Color32::from_rgb(0x3C, 0x38, 0x36),
    text: Color32::from_rgb(0xEB, 0xDB, 0xB2),
    muted: Color32::from_rgb(0xA8, 0x99, 0x84),
    faint: Color32::from_rgb(0x7C, 0x6F, 0x64),
    accent: Color32::from_rgb(0xFA, 0xBD, 0x2F),
    on_accent: Color32::from_rgb(0x28, 0x28, 0x28),
    accent_soft: Color32::from_rgb(0x50, 0x49, 0x45),
    hover: Color32::from_rgb(0x3C, 0x38, 0x36),
    code_bg: Color32::from_rgb(0x32, 0x30, 0x2F),
    danger: Color32::from_rgb(0xFB, 0x49, 0x34),
    success: Color32::from_rgb(0xB8, 0xBB, 0x26),
    red: Color32::from_rgb(0xFB, 0x49, 0x34),
    green: Color32::from_rgb(0xB8, 0xBB, 0x26),
    yellow: Color32::from_rgb(0xFA, 0xBD, 0x2F),
    blue: Color32::from_rgb(0x83, 0xA5, 0x98),
    purple: Color32::from_rgb(0xD3, 0x86, 0x9B),
    aqua: Color32::from_rgb(0x8E, 0xC0, 0x7C),
    orange: Color32::from_rgb(0xFE, 0x80, 0x19),
    gray: Color32::from_rgb(0x92, 0x83, 0x74),
};

pub fn palette(ctx: &egui::Context) -> Palette {
    if ctx.theme() == Theme::Dark {
        DARK
    } else {
        LIGHT
    }
}

pub const RADIUS: u8 = 8;

pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    for (theme, p) in [(Theme::Light, LIGHT), (Theme::Dark, DARK)] {
        ctx.style_mut_of(theme, |style| {
            style.visuals = visuals(theme == Theme::Dark, &p);
            style.text_styles = [
                (TextStyle::Small, FontId::proportional(11.5)),
                (TextStyle::Body, FontId::proportional(14.0)),
                (TextStyle::Button, FontId::proportional(14.0)),
                (TextStyle::Heading, FontId::new(20.0, semibold())),
                (TextStyle::Monospace, FontId::monospace(13.0)),
            ]
            .into();
            let s = &mut style.spacing;
            s.item_spacing = egui::vec2(8.0, 6.0);
            s.button_padding = egui::vec2(10.0, 5.0);
            s.interact_size.y = 28.0;
            s.menu_margin = Margin::same(6);
            s.window_margin = Margin::same(16);
            s.indent = 14.0;
            s.icon_width = 16.0;
            s.icon_width_inner = 9.0;
            s.slider_width = 180.0;
            s.scroll = egui::style::ScrollStyle::floating();
            s.scroll.bar_width = 8.0;
        });
    }
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add(&mut fonts, "inter", include_bytes!("../assets/fonts/Inter-Regular.ttf"));
    add(&mut fonts, "inter-semibold", include_bytes!("../assets/fonts/Inter-SemiBold.ttf"));
    add(&mut fonts, "jetbrains-mono", include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"));

    // egui's bundled fonts stay as fallbacks (emoji, symbols).
    let fallbacks = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "inter".into());
    fonts.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "jetbrains-mono".into());
    let mut bold = vec!["inter-semibold".to_string()];
    bold.extend(fallbacks);
    fonts.families.insert(semibold(), bold);

    // Icons resolve in both text families.
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    if let Some(keys) = fonts.families.get_mut(&semibold()) {
        keys.insert(1, "phosphor".into());
    }
    fonts
}

fn visuals(dark: bool, p: &Palette) -> Visuals {
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    let r = CornerRadius::same(RADIUS);
    v.override_text_color = Some(p.text);
    v.weak_text_color = Some(p.muted);
    v.hyperlink_color = p.blue;
    v.faint_bg_color = p.panel;
    v.extreme_bg_color = p.bg;
    v.text_edit_bg_color = Some(p.bg);
    v.code_bg_color = p.code_bg;
    v.panel_fill = p.panel;
    v.window_fill = p.surface;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = r;
    let shadow_alpha = if dark { 110 } else { 28 };
    v.window_shadow = Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(shadow_alpha) };
    v.popup_shadow = Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(shadow_alpha) };
    v.selection.bg_fill = p.accent.gamma_multiply(if dark { 0.45 } else { 0.25 });
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.warn_fg_color = p.yellow;
    v.error_fg_color = p.danger;
    v.striped = false;
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;
    v.indent_has_left_vline = false;
    v.collapsing_header_frame = false;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = p.hover;
    w.inactive.weak_bg_fill = Color32::TRANSPARENT;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = r;

    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::NONE;
    w.hovered.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.accent_soft;
    w.active.weak_bg_fill = p.accent_soft;
    w.active.bg_stroke = Stroke::NONE;
    w.active.fg_stroke = Stroke::new(1.0, p.text);
    w.active.corner_radius = r;
    w.active.expansion = 0.0;

    w.open.bg_fill = p.hover;
    w.open.weak_bg_fill = p.hover;
    w.open.bg_stroke = Stroke::NONE;
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.corner_radius = r;
    v
}
