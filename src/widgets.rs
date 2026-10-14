//! Small custom-drawn widgets that give the app its look. They take colours from
//! [`theme::palette`] so they follow light and dark mode.

use crate::theme::{self, icon, Palette};
use eframe::egui::{
    self, text::LayoutJob, Align2, Color32, CornerRadius, CursorIcon, FontId, Galley, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, TextFormat, Ui, Vec2,
};
use std::sync::Arc;

pub fn pal(ui: &Ui) -> Palette {
    theme::palette(ui.ctx())
}

/// A single line of text cut off with "…" to fit `width`.
pub fn line(ui: &Ui, text: &str, font: FontId, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_string(), TextFormat::simple(font, color));
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(1.0));
    ui.painter().layout_job(job)
}

/// Square icon button that only shows a background on hover.
pub fn icon_button(ui: &mut Ui, glyph: &str, tooltip: &str) -> Response {
    icon_button_ex(ui, glyph, tooltip, false, true)
}

pub fn icon_button_ex(ui: &mut Ui, glyph: &str, tooltip: &str, active: bool, enabled: bool) -> Response {
    let p = pal(ui);
    let size = Vec2::splat(28.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let fill = if active {
            p.accent_soft
        } else if enabled && resp.hovered() {
            p.hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
        let color = if !enabled {
            p.faint
        } else if active {
            p.accent
        } else if resp.hovered() {
            p.text
        } else {
            p.muted
        };
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(17.0), color);
    }
    let resp = if enabled { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp };
    if tooltip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tooltip)
    }
}

pub struct Row<'a> {
    pub glyph: &'a str,
    pub label: &'a str,
    /// Muted text on the right (counts, times, shortcuts).
    pub detail: Option<&'a str>,
    pub indent: f32,
    pub selected: bool,
    /// Some(open) draws a disclosure caret for folders.
    pub caret: Option<bool>,
    pub height: f32,
}

impl<'a> Row<'a> {
    pub fn new(glyph: &'a str, label: &'a str) -> Self {
        Row { glyph, label, detail: None, indent: 0.0, selected: false, caret: None, height: 30.0 }
    }
}

/// A full-width list row: [caret] icon label ……… detail
pub fn row(ui: &mut Ui, r: Row) -> Response {
    let p = pal(ui);
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, r.height), Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let painter = ui.painter();
    let fill = if r.selected {
        p.accent_soft
    } else if resp.hovered() {
        p.hover
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_filled(rect, CornerRadius::same(6), fill);
    let mut x = rect.left() + 6.0 + r.indent;
    let cy = rect.center().y;
    if let Some(open) = r.caret {
        let caret = if open { icon::CARET_DOWN } else { icon::CARET_RIGHT };
        painter.text(Pos2::new(x + 6.0, cy), Align2::CENTER_CENTER, caret, FontId::proportional(11.0), p.faint);
    }
    x += 14.0;
    let icon_color = if r.selected { p.accent } else { p.muted };
    painter.text(Pos2::new(x + 8.0, cy), Align2::CENTER_CENTER, r.glyph, FontId::proportional(16.0), icon_color);
    x += 24.0;

    let mut right = rect.right() - 8.0;
    if let Some(detail) = r.detail {
        let g = line(ui, detail, FontId::proportional(11.5), p.faint, 120.0);
        right -= g.size().x;
        painter.galley(Pos2::new(right, cy - g.size().y / 2.0), g, p.faint);
        right -= 8.0;
    }
    let color = if r.selected { p.text } else { p.text.gamma_multiply(0.92) };
    let font = if r.selected { FontId::new(13.5, theme::semibold()) } else { FontId::proportional(13.5) };
    let g = line(ui, r.label, font, color, right - x);
    painter.galley(Pos2::new(x, cy - g.size().y / 2.0), g, color);
    resp
}

/// Pill-shaped segmented control. Returns true when the value changed.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, &str, &str)]) -> bool {
    let p = pal(ui);
    let seg_w: Vec<f32> = options
        .iter()
        .map(|(_, glyph, label)| {
            let text = seg_text(glyph, label);
            let g = ui.painter().layout_no_wrap(text, FontId::proportional(13.0), p.text);
            g.size().x + 22.0
        })
        .collect();
    let total = seg_w.iter().sum::<f32>() + 6.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total, 30.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(8), p.hover);
    let mut x = rect.left() + 3.0;
    let mut changed = false;
    for ((v, glyph, label), w) in options.iter().zip(seg_w) {
        let r = Rect::from_min_size(Pos2::new(x, rect.top() + 3.0), egui::vec2(w, rect.height() - 6.0));
        let id = ui.id().with(("seg", x as i32));
        let resp = ui.interact(r, id, Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
        let on = *value == *v;
        if on {
            ui.painter().rect_filled(r, CornerRadius::same(6), p.surface);
            ui.painter().rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0, p.border), StrokeKind::Inside);
        }
        let color = if on || resp.hovered() { p.text } else { p.muted };
        ui.painter().text(r.center(), Align2::CENTER_CENTER, seg_text(glyph, label), FontId::proportional(13.0), color);
        if resp.clicked() && !on {
            *value = *v;
            changed = true;
        }
        x += w;
    }
    changed
}

fn seg_text(glyph: &str, label: &str) -> String {
    match (glyph.is_empty(), label.is_empty()) {
        (true, _) => label.to_string(),
        (_, true) => glyph.to_string(),
        _ => format!("{glyph}  {label}"),
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

/// A text button with an optional leading icon.
pub fn button(ui: &mut Ui, kind: Kind, glyph: &str, label: &str) -> Response {
    button_ex(ui, kind, glyph, label, true)
}

pub fn button_ex(ui: &mut Ui, kind: Kind, glyph: &str, label: &str, enabled: bool) -> Response {
    let p = pal(ui);
    let text = seg_text(glyph, label);
    let font = FontId::new(13.5, theme::semibold());
    let g = ui.painter().layout_no_wrap(text.clone(), font.clone(), p.text);
    let size = egui::vec2(g.size().x + 28.0, 32.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let hovered = enabled && resp.hovered();
        let (fill, stroke, fg) = match kind {
            Kind::Primary => (if hovered { p.accent.gamma_multiply(0.9) } else { p.accent }, None, p.on_accent),
            Kind::Danger => (if hovered { p.danger.gamma_multiply(0.9) } else { p.danger }, None, Color32::WHITE),
            Kind::Secondary => (if hovered { p.hover } else { p.surface }, Some(p.border), p.text),
            Kind::Ghost => (if hovered { p.hover } else { Color32::TRANSPARENT }, None, p.text),
        };
        let (fill, fg) = if enabled { (fill, fg) } else { (fill.gamma_multiply(0.5), fg.gamma_multiply(0.6)) };
        ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
        if let Some(s) = stroke {
            ui.painter().rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0, s), StrokeKind::Inside);
        }
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, font, fg);
    }
    if enabled {
        resp.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// An on/off switch with a label to its left, filling the row.
pub fn switch(ui: &mut Ui, on: &mut bool, label: &str, help: &str) -> bool {
    let p = pal(ui);
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(label);
            if !help.is_empty() {
                ui.label(egui::RichText::new(help).small().color(p.muted));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(36.0, 20.0), Sense::click());
            let t = ui.ctx().animate_bool(resp.id, *on);
            let track = lerp_color(p.border, p.accent, t);
            ui.painter().rect_filled(rect, CornerRadius::same(10), track);
            let x = egui::lerp(rect.left() + 10.0..=rect.right() - 10.0, t);
            ui.painter().circle_filled(Pos2::new(x, rect.center().y), 7.5, Color32::WHITE);
            if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                *on = !*on;
                changed = true;
            }
        });
    });
    changed
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Framed single-line text input.
pub fn input(ui: &mut Ui, value: &mut String, hint: &str) -> Response {
    let p = pal(ui);
    let frame = egui::Frame::new()
        .fill(p.bg)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 7));
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(egui::RichText::new(hint).color(p.faint))
            .frame(frame)
            .desired_width(f32::INFINITY),
    )
}

/// Small uppercase label above a group of rows.
pub fn section_label(ui: &mut Ui, text: &str) {
    let p = pal(ui);
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text.to_uppercase()).size(11.0).color(p.faint).family(theme::semibold()));
    ui.add_space(2.0);
}

/// A bordered card.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui);
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A full-width menu entry with an icon.
pub fn menu_item(ui: &mut Ui, glyph: &str, label: &str) -> Response {
    menu_item_ex(ui, glyph, label, None, false)
}

pub fn menu_item_ex(ui: &mut Ui, glyph: &str, label: &str, shortcut: Option<&str>, danger: bool) -> Response {
    let p = pal(ui);
    let width = ui.available_width().max(200.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 30.0), Sense::click());
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(6), p.hover);
        }
        let fg = if danger { p.danger } else { p.text };
        let icon_fg = if danger { p.danger } else { p.muted };
        let cy = rect.center().y;
        ui.painter().text(Pos2::new(rect.left() + 18.0, cy), Align2::CENTER_CENTER, glyph, FontId::proportional(16.0), icon_fg);
        ui.painter().text(Pos2::new(rect.left() + 36.0, cy), Align2::LEFT_CENTER, label, FontId::proportional(13.5), fg);
        if let Some(s) = shortcut {
            ui.painter().text(Pos2::new(rect.right() - 10.0, cy), Align2::RIGHT_CENTER, s, FontId::proportional(12.0), p.faint);
        }
    }
    resp.on_hover_cursor(CursorIcon::PointingHand)
}
