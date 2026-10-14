//! The search palette (Ctrl+K): recent notes when empty, then matching note names and
//! lines of text as you type, with the matches highlighted.

use crate::app::{Action, App};
use crate::theme::{self, icon, Palette};
use crate::vault::{self, Vault};
use crate::widgets;
use eframe::egui::{
    self, text::LayoutJob, Align2, Color32, CornerRadius, FontId, Frame, Id, Key, Margin, Modal, Modifiers, Pos2,
    RichText, ScrollArea, Sense, Stroke, TextFormat,
};

enum Item {
    /// A note, optionally with the matching line.
    Note { index: usize, line: Option<String> },
    Create(String),
}

const MAX_NAMES: usize = 8;
const MAX_LINES: usize = 40;

fn results(v: &Vault, query: &str) -> (Vec<Item>, usize) {
    let q = query.trim();
    let mut items = Vec::new();
    if q.is_empty() {
        let mut recent: Vec<usize> = (0..v.notes.len()).collect();
        recent.sort_by(|&a, &b| v.notes[b].modified.cmp(&v.notes[a].modified));
        items.extend(recent.into_iter().take(12).map(|index| Item::Note { index, line: None }));
        let n = items.len();
        return (items, n);
    }
    let mut names: Vec<(i32, usize)> = v
        .notes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| vault::fuzzy(n.rel.trim_end_matches(".md"), q).map(|s| (s, i)))
        .collect();
    names.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| v.notes[a.1].rel.cmp(&v.notes[b.1].rel)));
    items.extend(names.iter().take(MAX_NAMES).map(|&(_, index)| Item::Note { index, line: None }));
    let name_count = items.len();
    for hit in v.search(q, MAX_LINES * 2) {
        if !hit.text.is_empty() && items.len() - name_count < MAX_LINES {
            items.push(Item::Note { index: hit.note, line: Some(hit.text) });
        }
    }
    let exact = v.notes.iter().any(|n| n.name.eq_ignore_ascii_case(q));
    if !exact {
        items.push(Item::Create(q.to_string()));
    }
    (items, name_count)
}

impl App {
    pub(crate) fn search_ui(&mut self, ctx: &egui::Context) {
        let Some(v) = &self.vault else { return };
        let Some(st) = &mut self.search else { return };
        let p = theme::palette(ctx);
        let (items, name_count) = results(v, &st.query);

        let key = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        let (down, up) = (key(Key::ArrowDown), key(Key::ArrowUp));
        if down {
            st.selected = (st.selected + 1).min(items.len().saturating_sub(1));
        }
        if up {
            st.selected = st.selected.saturating_sub(1);
        }
        let enter = key(Key::Enter);
        st.selected = st.selected.min(items.len().saturating_sub(1));
        let shift = ctx.input(|i| i.modifiers.shift);

        let mut picked: Option<usize> = None;
        let id = Id::new("search-palette");
        let area = Modal::default_area(id).anchor(Align2::CENTER_TOP, [0.0, 72.0]);
        let width = (ctx.content_rect().width() - 32.0).min(620.0);
        let modal = Modal::new(id)
            .area(area)
            .backdrop_color(Color32::from_black_alpha(if ctx.theme() == egui::Theme::Dark { 120 } else { 60 }))
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(CornerRadius::same(14))
                    .shadow(ctx.style_of(ctx.theme()).visuals.window_shadow)
                    .inner_margin(Margin::same(8)),
            )
            .show(ctx, |ui| {
                ui.set_width(width);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(icon::MAGNIFYING_GLASS).size(18.0).color(p.muted));
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut st.query)
                            .hint_text(RichText::new("Search notes, or type a name to create one").color(p.faint))
                            .font(FontId::proportional(16.0))
                            .frame(Frame::NONE)
                            .margin(Margin::symmetric(4, 10))
                            .desired_width(f32::INFINITY),
                    );
                    edit.request_focus();
                    if edit.changed() {
                        st.selected = 0;
                    }
                });
                ui.add_space(4.0);
                let r = ui.available_rect_before_wrap();
                ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, p.border));
                ui.add_space(6.0);

                ScrollArea::vertical().max_height(420.0).auto_shrink([false, true]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    if items.is_empty() {
                        ui.add_space(12.0);
                        ui.label(RichText::new("   No notes yet.").color(p.muted));
                        ui.add_space(12.0);
                    }
                    let q = st.query.trim().to_string();
                    for (k, item) in items.iter().enumerate() {
                        if k == 0 {
                            let title = if q.is_empty() { "Recent" } else if name_count > 0 { "Notes" } else { "" };
                            if !title.is_empty() {
                                group(ui, title);
                            }
                        }
                        if k == name_count && k < items.len() && matches!(item, Item::Note { line: Some(_), .. }) {
                            group(ui, "In notes");
                        }
                        let resp = item_ui(ui, v, item, k == st.selected, &q, &p);
                        if k == st.selected && (up || down) {
                            resp.scroll_to_me(None);
                        }
                        if resp.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                            st.selected = k;
                        }
                        if resp.clicked() {
                            picked = Some(k);
                        }
                    }
                });
                ui.add_space(4.0);
                let r = ui.available_rect_before_wrap();
                ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, p.border));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let hint = "↑↓ to move   ·   Enter to open   ·   Shift+Enter to create   ·   Esc to close";
                    ui.label(RichText::new(hint).size(11.5).color(p.faint));
                });
            });

        if enter {
            if shift && !st.query.trim().is_empty() {
                picked = items.iter().position(|i| matches!(i, Item::Create(_))).or(Some(usize::MAX));
            } else {
                picked = Some(st.selected);
            }
        }
        let query = st.query.trim().to_string();
        let close = modal.should_close() || picked.is_some();
        let action = picked.map(|k| match items.get(k) {
            Some(Item::Note { index, .. }) => Action::Open(v.notes[*index].rel.clone()),
            Some(Item::Create(name)) => Action::Link(format!("wiki:{name}")),
            None => Action::Link(format!("wiki:{query}")),
        });
        if close {
            self.search = None;
            self.focus_editor = true;
        }
        if let Some(a) = action {
            if !matches!(&a, Action::Link(l) if l == "wiki:") {
                self.actions.push(a);
            }
        }
    }
}

fn group(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        widgets::section_label(ui, title);
    });
}

fn item_ui(ui: &mut egui::Ui, v: &Vault, item: &Item, selected: bool, q: &str, p: &Palette) -> egui::Response {
    let line = matches!(item, Item::Note { line: Some(_), .. });
    let height = if line { 50.0 } else { 36.0 };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::click());
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, CornerRadius::same(8), p.accent_soft);
    }
    let x = rect.left() + 12.0;
    let text_x = x + 26.0;
    let width = rect.right() - text_x - 12.0;
    match item {
        Item::Note { index, line } => {
            let note = &v.notes[*index];
            let folder = vault::parent(&note.rel);
            let top = if line.is_some() { rect.top() + 16.0 } else { rect.center().y };
            let glyph_color = if selected { p.accent } else { p.muted };
            painter.text(Pos2::new(x + 8.0, top), Align2::CENTER_CENTER, icon::FILE_TEXT, FontId::proportional(16.0), glyph_color);
            let mut job = highlighted(&note.name, q, FontId::new(14.0, theme::semibold()), p.text, p.accent);
            if !folder.is_empty() {
                job.append(&format!("   {folder}"), 0.0, TextFormat::simple(FontId::proportional(12.0), p.faint));
            }
            job.wrap = egui::text::TextWrapping::truncate_at_width(width);
            let g = painter.layout_job(job);
            painter.galley(Pos2::new(text_x, top - g.size().y / 2.0), g, p.text);
            if let Some(text) = line {
                let mut job = highlighted(text, q, FontId::proportional(12.5), p.muted, p.accent);
                job.wrap = egui::text::TextWrapping::truncate_at_width(width);
                let g = painter.layout_job(job);
                painter.galley(Pos2::new(text_x, rect.top() + 28.0), g, p.muted);
            }
        }
        Item::Create(name) => {
            painter.text(Pos2::new(x + 8.0, rect.center().y), Align2::CENTER_CENTER, icon::PLUS, FontId::proportional(16.0), p.accent);
            let mut job = LayoutJob::default();
            job.append("Create note ", 0.0, TextFormat::simple(FontId::proportional(14.0), p.text));
            job.append(&format!("“{name}”"), 0.0, TextFormat::simple(FontId::new(14.0, theme::semibold()), p.accent));
            job.wrap = egui::text::TextWrapping::truncate_at_width(width);
            let g = painter.layout_job(job);
            painter.galley(Pos2::new(text_x, rect.center().y - g.size().y / 2.0), g, p.text);
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// `text` with case-insensitive matches of `q` in the accent colour.
fn highlighted(text: &str, q: &str, font: FontId, color: Color32, accent: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat::simple(font.clone(), color);
    let hit = TextFormat { background: accent.gamma_multiply(0.18), ..TextFormat::simple(font, accent) };
    let mut last = 0;
    for (a, z) in match_ranges(text, q) {
        job.append(&text[last..a], 0.0, plain.clone());
        job.append(&text[a..z], 0.0, hit.clone());
        last = z;
    }
    job.append(&text[last..], 0.0, plain);
    job
}

/// Byte ranges of case-insensitive matches of `q` in `text`.
fn match_ranges(text: &str, q: &str) -> Vec<(usize, usize)> {
    let needle: Vec<char> = q.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= chars.len() {
        let matched = needle
            .iter()
            .enumerate()
            .all(|(k, n)| chars[i + k].1.to_lowercase().next() == Some(*n));
        if matched {
            let end = chars.get(i + needle.len()).map_or(text.len(), |c| c.0);
            out.push((chars[i].0, end));
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_ignore_case() {
        assert_eq!(match_ranges("Debian and debian", "DEBIAN"), vec![(0, 6), (11, 17)]);
        assert_eq!(match_ranges("Äpfel", "äp"), vec![(0, 3)]);
        assert!(match_ranges("abc", "").is_empty());
    }
}
