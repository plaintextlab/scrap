//! Look and feel: fonts, the colour themes (each with a light and a dark palette), and
//! egui's widget styling derived from them.

use eframe::egui::{
    self, epaint::Shadow, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle, Theme, Visuals,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

pub use egui_phosphor::regular as icon;

/// Semibold text (headings, titles, emphasis in the UI).
pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

/// Colours for one theme in one mode. Everything in the UI is drawn from these.
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
    // Named accents, for colour-coded icons and markdown.
    pub red: Color32,
    pub green: Color32,
    pub yellow: Color32,
    pub blue: Color32,
    pub purple: Color32,
    pub aqua: Color32,
    pub orange: Color32,
    pub gray: Color32,
}

/// A colour scheme with a light and a dark variant.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeName {
    #[default]
    Gruvbox,
    Everforest,
    Nord,
    Catppuccin,
    Solarized,
    TokyoNight,
}

impl ThemeName {
    pub const ALL: [ThemeName; 6] = [
        ThemeName::Gruvbox,
        ThemeName::Everforest,
        ThemeName::Nord,
        ThemeName::Catppuccin,
        ThemeName::Solarized,
        ThemeName::TokyoNight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeName::Gruvbox => "Gruvbox",
            ThemeName::Everforest => "Everforest",
            ThemeName::Nord => "Nord",
            ThemeName::Catppuccin => "Catppuccin",
            ThemeName::Solarized => "Solarized",
            ThemeName::TokyoNight => "Tokyo Night",
        }
    }

    /// File-name friendly id, as used for the app icons.
    pub fn id(self) -> &'static str {
        match self {
            ThemeName::Gruvbox => "gruvbox",
            ThemeName::Everforest => "everforest",
            ThemeName::Nord => "nord",
            ThemeName::Catppuccin => "catppuccin",
            ThemeName::Solarized => "solarized",
            ThemeName::TokyoNight => "tokyonight",
        }
    }

    pub fn palette(self, dark: bool) -> Palette {
        let (light, dark_p) = palettes(self);
        if dark {
            dark_p
        } else {
            light
        }
    }
}

const fn c(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Builds a palette from hex colours, in field order.
#[allow(clippy::too_many_arguments)]
#[rustfmt::skip]
const fn p(
    bg: u32, panel: u32, surface: u32, border: u32, text: u32, muted: u32, faint: u32,
    accent: u32, on_accent: u32, accent_soft: u32, hover: u32, code_bg: u32,
    red: u32, green: u32, yellow: u32, blue: u32, purple: u32, aqua: u32, orange: u32, gray: u32,
) -> Palette {
    Palette {
        bg: c(bg), panel: c(panel), surface: c(surface), border: c(border), text: c(text),
        muted: c(muted), faint: c(faint), accent: c(accent), on_accent: c(on_accent),
        accent_soft: c(accent_soft), hover: c(hover), code_bg: c(code_bg),
        danger: c(red), success: c(green),
        red: c(red), green: c(green), yellow: c(yellow), blue: c(blue), purple: c(purple),
        aqua: c(aqua), orange: c(orange), gray: c(gray),
    }
}

/// (light, dark) palettes of each theme. Keep in sync with android/.../ui/Theme.kt.
#[rustfmt::skip]
fn palettes(t: ThemeName) -> (Palette, Palette) {
    match t {
        // https://github.com/morhetz/gruvbox: faded accents on light, bright on dark.
        ThemeName::Gruvbox => (
            p(0xFBF1C7, 0xF2E5BC, 0xF9F5D7, 0xD5C4A1, 0x3C3836, 0x7C6F64, 0xA89984,
              0xB57614, 0xFBF1C7, 0xD5C4A1, 0xEBDBB2, 0xF2E5BC,
              0x9D0006, 0x79740E, 0xB57614, 0x076678, 0x8F3F71, 0x427B58, 0xAF3A03, 0x7C6F64),
            p(0x282828, 0x1D2021, 0x32302F, 0x3C3836, 0xEBDBB2, 0xA89984, 0x7C6F64,
              0xFABD2F, 0x282828, 0x504945, 0x3C3836, 0x32302F,
              0xFB4934, 0xB8BB26, 0xFABD2F, 0x83A598, 0xD3869B, 0x8EC07C, 0xFE8019, 0x928374),
        ),
        // https://github.com/sainnhe/everforest (medium contrast).
        ThemeName::Everforest => (
            p(0xFDF6E3, 0xF4F0D9, 0xFFFBEF, 0xE0DCC7, 0x5C6A72, 0x829181, 0xA6B0A0,
              0x8DA101, 0x2D353B, 0xE6E2CC, 0xEFEBD4, 0xF4F0D9,
              0xF85552, 0x8DA101, 0xDFA000, 0x3A94C5, 0xDF69BA, 0x35A77C, 0xF57D26, 0x939F91),
            p(0x2D353B, 0x232A2E, 0x343F44, 0x3D484D, 0xD3C6AA, 0x9DA9A0, 0x7A8478,
              0xA7C080, 0x2D353B, 0x475258, 0x3D484D, 0x343F44,
              0xE67E80, 0xA7C080, 0xDBBC7F, 0x7FBBB3, 0xD699B6, 0x83C092, 0xE69875, 0x859289),
        ),
        // https://www.nordtheme.com: Snow Storm for light, Polar Night for dark.
        ThemeName::Nord => (
            p(0xECEFF4, 0xE5E9F0, 0xF5F7FA, 0xD8DEE9, 0x2E3440, 0x4C566A, 0x7B88A1,
              0x5E81AC, 0xECEFF4, 0xD8DEE9, 0xDEE3EB, 0xE5E9F0,
              0xBF616A, 0x6F8F57, 0xB08A3B, 0x5E81AC, 0x9A6F94, 0x4C8C8B, 0xC0724F, 0x7B88A1),
            p(0x2E3440, 0x292E39, 0x3B4252, 0x434C5E, 0xE5E9F0, 0xA3ACBD, 0x616E88,
              0x88C0D0, 0x2E3440, 0x434C5E, 0x3B4252, 0x3B4252,
              0xBF616A, 0xA3BE8C, 0xEBCB8B, 0x81A1C1, 0xB48EAD, 0x8FBCBB, 0xD08770, 0x616E88),
        ),
        // https://catppuccin.com: Latte for light, Mocha for dark.
        ThemeName::Catppuccin => (
            p(0xEFF1F5, 0xE6E9EF, 0xF5F6F9, 0xCCD0DA, 0x4C4F69, 0x6C6F85, 0x9CA0B0,
              0x8839EF, 0xEFF1F5, 0xCCD0DA, 0xDCE0E8, 0xE6E9EF,
              0xD20F39, 0x40A02B, 0xDF8E1D, 0x1E66F5, 0x7287FD, 0x179299, 0xFE640B, 0x8C8FA1),
            p(0x1E1E2E, 0x181825, 0x313244, 0x45475A, 0xCDD6F4, 0xA6ADC8, 0x6C7086,
              0xCBA6F7, 0x1E1E2E, 0x45475A, 0x313244, 0x313244,
              0xF38BA8, 0xA6E3A1, 0xF9E2AF, 0x89B4FA, 0xB4BEFE, 0x94E2D5, 0xFAB387, 0x7F849C),
        ),
        // https://ethanschoonover.com/solarized
        ThemeName::Solarized => (
            p(0xFDF6E3, 0xEEE8D5, 0xFFFBEE, 0xDDD6C1, 0x586E75, 0x657B83, 0x93A1A1,
              0x268BD2, 0xFDF6E3, 0xE1DAC3, 0xE8E2CE, 0xEEE8D5,
              0xDC322F, 0x859900, 0xB58900, 0x268BD2, 0x6C71C4, 0x2AA198, 0xCB4B16, 0x93A1A1),
            p(0x002B36, 0x00212B, 0x073642, 0x0D4250, 0x93A1A1, 0x839496, 0x586E75,
              0x268BD2, 0xFDF6E3, 0x104858, 0x073642, 0x073642,
              0xDC322F, 0x859900, 0xB58900, 0x268BD2, 0x6C71C4, 0x2AA198, 0xCB4B16, 0x657B83),
        ),
        // https://github.com/folke/tokyonight.nvim: Day for light, Night for dark.
        ThemeName::TokyoNight => (
            p(0xE1E2E7, 0xD5D6DB, 0xE9E9ED, 0xC4C8DA, 0x3760BF, 0x6172B0, 0x848CB5,
              0x2E7DE9, 0xE9E9ED, 0xB7C1E3, 0xCBCFDD, 0xD0D3DD,
              0xF52A65, 0x587539, 0x8C6C3E, 0x2E7DE9, 0x9854F1, 0x118C74, 0xB15C00, 0x848CB5),
            p(0x1A1B26, 0x16161E, 0x24283B, 0x292E42, 0xC0CAF5, 0xA9B1D6, 0x565F89,
              0x7AA2F7, 0x1A1B26, 0x283457, 0x292E42, 0x24283B,
              0xF7768E, 0x9ECE6A, 0xE0AF68, 0x7AA2F7, 0xBB9AF7, 0x73DACA, 0xFF9E64, 0x565F89),
        ),
    }
}

/// The theme in use, readable from anywhere that draws (set by `install`).
static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn current() -> ThemeName {
    ThemeName::ALL[CURRENT.load(Ordering::Relaxed) as usize]
}

/// The current theme's palette for light or dark mode.
pub fn palette_for(dark: bool) -> Palette {
    current().palette(dark)
}

pub fn palette(ctx: &egui::Context) -> Palette {
    palette_for(ctx.theme() == Theme::Dark)
}

pub const RADIUS: u8 = 8;

/// The window icon of each theme (the installed app icon is always Gruvbox).
pub fn icon_png(t: ThemeName) -> &'static [u8] {
    match t {
        ThemeName::Gruvbox => include_bytes!("../packaging/icons/themes/gruvbox.png"),
        ThemeName::Everforest => include_bytes!("../packaging/icons/themes/everforest.png"),
        ThemeName::Nord => include_bytes!("../packaging/icons/themes/nord.png"),
        ThemeName::Catppuccin => include_bytes!("../packaging/icons/themes/catppuccin.png"),
        ThemeName::Solarized => include_bytes!("../packaging/icons/themes/solarized.png"),
        ThemeName::TokyoNight => include_bytes!("../packaging/icons/themes/tokyonight.png"),
    }
}

/// Installs fonts once; see `apply` for colours.
pub fn install(ctx: &egui::Context, t: ThemeName) {
    ctx.set_fonts(fonts());
    apply(ctx, t);
}

/// Restyles every widget for `t` (both light and dark), and swaps the window icon.
pub fn apply(ctx: &egui::Context, t: ThemeName) {
    let index = ThemeName::ALL.iter().position(|&x| x == t).unwrap_or(0);
    CURRENT.store(index as u8, Ordering::Relaxed);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(icon_png(t)) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Icon(Some(Arc::new(icon))));
    }
    for theme in [Theme::Light, Theme::Dark] {
        let p = t.palette(theme == Theme::Dark);
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
