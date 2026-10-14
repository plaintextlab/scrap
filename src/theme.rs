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

/// Colours for one theme. Everything in the UI is drawn from these.
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
}

pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    panel: Color32::from_rgb(0xF6, 0xF6, 0xF8),
    surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    border: Color32::from_rgb(0xE4, 0xE4, 0xE9),
    text: Color32::from_rgb(0x1E, 0x1E, 0x24),
    muted: Color32::from_rgb(0x6A, 0x6A, 0x76),
    faint: Color32::from_rgb(0xA3, 0xA3, 0xAE),
    accent: Color32::from_rgb(0x5B, 0x4B, 0xDB),
    on_accent: Color32::WHITE,
    accent_soft: Color32::from_rgb(0xEC, 0xE9, 0xFC),
    hover: Color32::from_rgb(0xEC, 0xEC, 0xF0),
    code_bg: Color32::from_rgb(0xF3, 0xF3, 0xF6),
    danger: Color32::from_rgb(0xD1, 0x3C, 0x3C),
    success: Color32::from_rgb(0x1E, 0x9E, 0x63),
};

pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x18, 0x18, 0x1C),
    panel: Color32::from_rgb(0x1F, 0x1F, 0x24),
    surface: Color32::from_rgb(0x26, 0x26, 0x2C),
    border: Color32::from_rgb(0x32, 0x32, 0x3A),
    text: Color32::from_rgb(0xE6, 0xE6, 0xEB),
    muted: Color32::from_rgb(0x9C, 0x9C, 0xA8),
    faint: Color32::from_rgb(0x66, 0x66, 0x72),
    accent: Color32::from_rgb(0xA3, 0x99, 0xFF),
    on_accent: Color32::from_rgb(0x16, 0x12, 0x33),
    accent_soft: Color32::from_rgb(0x33, 0x2E, 0x52),
    hover: Color32::from_rgb(0x2C, 0x2C, 0x33),
    code_bg: Color32::from_rgb(0x24, 0x24, 0x2A),
    danger: Color32::from_rgb(0xF2, 0x6D, 0x6D),
    success: Color32::from_rgb(0x4C, 0xC9, 0x8E),
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
    v.hyperlink_color = p.accent;
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
    v.warn_fg_color = Color32::from_rgb(0xD9, 0x8E, 0x1F);
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
