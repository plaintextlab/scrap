//! The note body: markdown editor with formatting commands, and the rendered reader.

use crate::app::{Action, App, View};
use crate::format;
use crate::layout::centered;
use crate::markdown;
use crate::theme;
use crate::youtube;
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
    Frame, Id, Margin, ScrollArea, TextBuffer, Ui,
};
use std::collections::HashMap;
use std::time::Instant;

/// Formatting toolbar commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Undo,
    Redo,
    Heading,
    Bold,
    Italic,
    Strike,
    List,
    Checklist,
    Quote,
    WikiLink,
    Code,
}

fn editor_id(rel: &str) -> Id {
    Id::new(("editor", rel))
}

fn sorted(range: &CCursorRange) -> (usize, usize) {
    let (a, b) = (range.primary.index.0, range.secondary.index.0);
    (a.min(b), a.max(b))
}

impl App {
    /// Applies a toolbar command to the open note at its cursor or selection.
    pub(crate) fn format(&mut self, op: Op) {
        let Some(n) = &mut self.note else { return };
        let id = editor_id(&n.rel);
        let mut state = TextEditState::load(&self.ctx, id).unwrap_or_default();
        let end = n.text.chars().count();
        let range = state.cursor.char_range().unwrap_or_else(|| CCursorRange::one(CCursor::new(end)));
        let sel = sorted(&range);
        let mut undoer = state.undoer();
        let current = (range, n.text.clone());
        match op {
            Op::Undo | Op::Redo => {
                let step = if op == Op::Undo { undoer.undo(&current) } else { undoer.redo(&current) };
                let Some((r, text)) = step.cloned() else { return };
                n.text = text;
                state.cursor.set_char_range(Some(r));
            }
            _ => {
                let text = &n.text;
                let edit = match op {
                    Op::Heading => format::toggle_line_prefix(text, sel, "# "),
                    Op::Bold => format::wrap(text, sel, "**", "**"),
                    Op::Italic => format::wrap(text, sel, "*", "*"),
                    Op::Strike => format::wrap(text, sel, "~~", "~~"),
                    Op::List => format::toggle_line_prefix(text, sel, "- "),
                    Op::Checklist => format::toggle_line_prefix(text, sel, "- [ ] "),
                    Op::Quote => format::toggle_line_prefix(text, sel, "> "),
                    Op::WikiLink => format::wrap(text, sel, "[[", "]]"),
                    Op::Code => {
                        let multiline = text.chars().skip(sel.0).take(sel.1 - sel.0).any(|c| c == '\n');
                        if multiline {
                            format::wrap(text, sel, "```\n", "\n```")
                        } else {
                            format::wrap(text, sel, "`", "`")
                        }
                    }
                    Op::Undo | Op::Redo => unreachable!(),
                };
                undoer.add_undo(&current);
                n.text = edit.text;
                let (a, b) = edit.selection;
                state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
            }
        }
        n.edited = Some(Instant::now());
        state.set_undoer(undoer);
        state.store(&self.ctx, id);
        self.focus_editor = true;
    }

    /// Whether undo and redo are available in the open note's editor.
    pub(crate) fn undo_state(&self) -> (bool, bool) {
        let Some(n) = &self.note else { return (false, false) };
        let Some(state) = TextEditState::load(&self.ctx, editor_id(&n.rel)) else { return (false, false) };
        let end = n.text.chars().count();
        let range = state.cursor.char_range().unwrap_or_else(|| CCursorRange::one(CCursor::new(end)));
        let current = (range, n.text.clone());
        let undoer = state.undoer();
        (undoer.has_undo(&current), undoer.has_redo(&current))
    }

    /// Inserts a paragraph (such as an image) at the cursor, or at the end.
    pub(crate) fn insert_block(&mut self, block: &str) {
        let Some(n) = &mut self.note else { return };
        let id = editor_id(&n.rel);
        let mut state = TextEditState::load(&self.ctx, id).unwrap_or_default();
        let end = n.text.chars().count();
        let at = state.cursor.char_range().map_or(end, |r| sorted(&r).1);
        let mut undoer = state.undoer();
        undoer.add_undo(&(CCursorRange::one(CCursor::new(at)), n.text.clone()));
        let edit = format::insert_block(&n.text, at, block);
        n.text = edit.text;
        state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(edit.selection.0))));
        state.set_undoer(undoer);
        state.store(&self.ctx, id);
        n.edited = Some(Instant::now());
        self.focus_editor = true;
    }

    pub(crate) fn note_body(&mut self, ui: &mut Ui) {
        match self.s.view {
            View::Edit => self.editor_ui(ui),
            View::Preview => self.preview_ui(ui),
            View::Split => {
                let p = theme::palette(ui.ctx());
                ui.columns(2, |cols| {
                    self.editor_ui(&mut cols[0]);
                    let r = cols[1].max_rect();
                    cols[1].painter().vline(r.left() - 4.0, r.y_range(), egui::Stroke::new(1.0, p.border));
                    self.preview_ui(&mut cols[1]);
                });
            }
        }
    }

    fn editor_ui(&mut self, ui: &mut Ui) {
        let size = self.s.font_size;
        let focus = std::mem::take(&mut self.focus_editor);
        let root = self.vault.as_ref().map(|v| v.root.clone());
        let Some(n) = &mut self.note else { return };
        let pictures = root.map(|root| pictures(ui.ctx(), &root, &n.rel, &n.text)).unwrap_or_default();
        let mut follow = None;
        ScrollArea::vertical().id_salt("editor").auto_shrink([false, false]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::symmetric(28, 20)).show(ui, |ui| {
                centered(ui, |ui| {
                    let mut layouter = |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
                        let room = |url: &str| pictures.get(url).map_or(0.0, |p| p.fit(wrap_width).y + PICTURE_GAP);
                        let mut job = markdown::highlight(text.as_str(), ui.visuals(), size, &room);
                        job.wrap.max_width = wrap_width;
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    let out = egui::TextEdit::multiline(&mut n.text)
                        .id(editor_id(&n.rel))
                        .frame(Frame::NONE)
                        .margin(Margin::ZERO)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .min_size(egui::vec2(0.0, (ui.available_height() - 40.0).max(200.0)))
                        .layouter(&mut layouter)
                        .show(ui);
                    paint_pictures(ui, &out, &n.text, &pictures, size);
                    if out.response.changed() {
                        n.edited = Some(Instant::now());
                    }
                    if focus {
                        out.response.request_focus();
                    }
                    // Ctrl/Cmd+click follows the link under the cursor.
                    if out.response.clicked() && ui.input(|i| i.modifiers.command) {
                        if let Some(range) = out.cursor_range {
                            follow = markdown::link_at(&n.text, range.primary.index.0);
                        }
                    }
                });
            });
        });
        if let Some(url) = follow {
            self.actions.push(Action::Link(url));
        }
    }

    fn preview_ui(&mut self, ui: &mut Ui) {
        let Some(n) = &self.note else { return };
        let Some(v) = &self.vault else { return };
        let images = markdown::Images { root: &v.root, note_rel: &n.rel };
        let size = self.s.font_size;
        let mut clicked = None;
        ScrollArea::vertical().id_salt("preview").auto_shrink([false, false]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::symmetric(28, 22)).show(ui, |ui| {
                centered(ui, |ui| {
                    if n.text.trim().is_empty() {
                        let p = theme::palette(ui.ctx());
                        ui.label(egui::RichText::new("This note is empty. Switch to Edit to start writing.").color(p.muted));
                    }
                    clicked = markdown::preview(ui, &n.text, size, &images);
                    ui.add_space(60.0);
                });
            });
        });
        if let Some(url) = clicked {
            self.actions.push(Action::Link(url));
        }
    }
}

/// Tallest an image is shown while editing; Read view shows it full size.
const PICTURE_MAX_HEIGHT: f32 = 420.0;
/// Space between an image line's text and its picture, and below the picture.
const PICTURE_GAP: f32 = 10.0;

/// An image shown under its `![](...)` line in the editor, or a YouTube link's video card.
struct Picture {
    /// The image; for a video, its thumbnail once downloaded.
    uri: Option<String>,
    size: egui::Vec2,
    video: bool,
}

impl Picture {
    /// Display size within `width`: never enlarged, at most `PICTURE_MAX_HEIGHT` tall.
    fn fit(&self, width: f32) -> egui::Vec2 {
        if self.video {
            return youtube::card_size(width);
        }
        if self.size.x <= 0.0 || self.size.y <= 0.0 {
            return egui::Vec2::ZERO;
        }
        let scale = (width / self.size.x).min(PICTURE_MAX_HEIGHT / self.size.y).min(1.0);
        self.size * scale
    }
}

/// Images on their own line in `text` whose size is known, and YouTube links, keyed by their
/// link. Other images (still loading, missing, on the web) get no preview; the loader
/// repaints when one is ready.
fn pictures(ctx: &egui::Context, root: &std::path::Path, note_rel: &str, text: &str) -> HashMap<String, Picture> {
    let mut out = HashMap::new();
    for url in image_lines(text).map(|(_, url)| url) {
        if out.contains_key(url) {
            continue;
        }
        if let Some(id) = youtube::video_id(url) {
            let uri = youtube::thumbnail(ctx, id);
            out.insert(url.to_string(), Picture { uri, size: egui::Vec2::ZERO, video: true });
            continue;
        }
        let Some(rel) = scrap_core::attachments::resolve(note_rel, url) else { continue };
        let path = root.join(rel);
        if !path.is_file() {
            continue;
        }
        let uri = crate::attach::file_uri(&path);
        let size = match ctx.try_load_texture(&uri, egui::TextureOptions::LINEAR, egui::SizeHint::default()) {
            Ok(egui::load::TexturePoll::Ready { texture }) => Some(texture.size),
            Ok(egui::load::TexturePoll::Pending { size }) => size,
            Err(_) => None,
        };
        if let Some(size) = size {
            out.insert(url.to_string(), Picture { uri: Some(uri), size, video: false });
        }
    }
    out
}

/// Image-only lines outside code blocks: (char index of the line's last character, link).
fn image_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut in_fence = false;
    let mut chars = 0;
    text.split_inclusive('\n').filter_map(move |line| {
        let start = chars;
        chars += line.chars().count();
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            return None;
        }
        if in_fence {
            return None;
        }
        let url = markdown::preview_line(line)?;
        let body = line.trim_end_matches(['\n', '\r']);
        Some((start + body.chars().count().saturating_sub(1), url))
    })
}

/// Paints each picture in the room `markdown::highlight` left below its line.
fn paint_pictures(ui: &Ui, out: &egui::text_edit::TextEditOutput, text: &str, pictures: &HashMap<String, Picture>, size: f32) {
    if pictures.is_empty() {
        return;
    }
    let width = out.response.rect.width();
    for (last_char, url) in image_lines(text) {
        let Some(picture) = pictures.get(url) else { continue };
        let row = out.galley.pos_from_cursor(egui::text::CCursor::new(last_char));
        let top = out.galley_pos.y + row.top() + markdown::editor_row_height(size) + PICTURE_GAP / 2.0;
        let rect = egui::Rect::from_min_size(egui::pos2(out.galley_pos.x, top), picture.fit(width));
        if !ui.is_rect_visible(rect) {
            continue;
        }
        if picture.video {
            let hovered = ui.rect_contains_pointer(rect);
            youtube::paint(ui, rect, picture.uri.as_deref(), hovered);
        } else if let Some(uri) = &picture.uri {
            egui::Image::new(uri.as_str()).corner_radius(egui::CornerRadius::same(10)).paint_at(ui, rect);
        }
    }
}
