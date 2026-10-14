//! The note body: markdown editor with formatting commands, and the rendered reader.

use crate::app::{Action, App, View};
use crate::format;
use crate::layout::centered;
use crate::markdown;
use crate::theme;
use eframe::egui::{
    self,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
    Frame, Id, Margin, ScrollArea, TextBuffer, Ui,
};
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
        let Some(n) = &mut self.note else { return };
        let mut follow = None;
        ScrollArea::vertical().id_salt("editor").auto_shrink([false, false]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::symmetric(28, 20)).show(ui, |ui| {
                centered(ui, |ui| {
                    let mut layouter = |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
                        let mut job = markdown::highlight(text.as_str(), ui.visuals(), size);
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
