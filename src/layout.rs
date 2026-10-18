//! How the window is put together: sidebar, note header and toolbar, the main area,
//! and floating toasts.

use crate::app::{Action, App, Page, View};
use crate::editor::Op;
use crate::theme::{self, icon};
use crate::vault;
use crate::widgets::{self, Kind};
use eframe::egui::{
    self, Align, Align2, CentralPanel, Color32, CornerRadius, Frame, Layout, Margin, Panel, Popup, RichText, Stroke,
    Ui,
};

/// Width of the text column in the editor and reader.
pub const READABLE_WIDTH: f32 = 760.0;

impl App {
    pub(crate) fn layout(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        if self.vault.is_none() {
            CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| self.welcome_ui(ui));
            return;
        }

        let files_only = self.narrow && self.s.show_sidebar;
        if files_only {
            CentralPanel::default()
                .frame(Frame::new().fill(p.panel).inner_margin(Margin::same(12)))
                .show(ui, |ui| self.sidebar_ui(ui));
            return;
        }
        if self.s.show_sidebar {
            Panel::left("sidebar")
                .resizable(true)
                .default_size(268.0)
                .size_range(210.0..=440.0)
                .frame(Frame::new().fill(p.panel).inner_margin(Margin { left: 10, right: 10, top: 12, bottom: 10 }))
                .show(ui, |ui| self.sidebar_ui(ui));
        }

        match self.page {
            Page::Notes if self.note.is_some() => {
                if self.s.show_links && !self.narrow {
                    Panel::right("links")
                        .resizable(true)
                        .default_size(250.0)
                        .size_range(200.0..=420.0)
                        .frame(Frame::new().fill(p.panel).inner_margin(Margin::same(14)))
                        .show(ui, |ui| self.links_ui(ui));
                }
                Panel::top("note-bar")
                    .frame(Frame::new().fill(p.bg).inner_margin(Margin { left: 12, right: 12, top: 8, bottom: 4 }))
                    .show_separator_line(false)
                    .show(ui, |ui| self.note_bar(ui));
                if self.s.view != View::Preview {
                    Panel::top("toolbar")
                        .frame(Frame::new().fill(p.bg).inner_margin(Margin { left: 12, right: 12, top: 2, bottom: 6 }))
                        .show_separator_line(true)
                        .show(ui, |ui| self.toolbar_ui(ui));
                }
                Panel::bottom("note-footer")
                    .frame(Frame::new().fill(p.bg).inner_margin(Margin::symmetric(16, 6)))
                    .show_separator_line(false)
                    .show(ui, |ui| self.note_footer(ui));
                CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| self.note_body(ui));
            }
            Page::Notes => {
                CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| self.empty_ui(ui));
            }
            Page::Trash => {
                CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| self.trash_ui(ui));
            }
            Page::Settings => {
                CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| self.settings_ui(ui));
            }
        }
    }

    /// Sidebar toggle shown in page headers when the sidebar is hidden.
    pub(crate) fn sidebar_toggle(&mut self, ui: &mut Ui) {
        if !self.s.show_sidebar && widgets::icon_button(ui, icon::SIDEBAR_SIMPLE, "Show sidebar (Ctrl+\\)").clicked() {
            self.s.show_sidebar = true;
        }
    }

    fn note_bar(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        let Some(n) = &self.note else { return };
        let rel = n.rel.clone();
        let dirty = n.text != n.saved;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            self.sidebar_toggle(ui);
            if widgets::icon_button_ex(ui, icon::ARROW_LEFT, "Back (Ctrl+Alt+←)", false, self.hist_pos > 0).clicked() {
                self.actions.push(Action::Back);
            }
            let fwd = self.hist_pos + 1 < self.history.len();
            if widgets::icon_button_ex(ui, icon::ARROW_RIGHT, "Forward (Ctrl+Alt+→)", false, fwd).clicked() {
                self.actions.push(Action::Forward);
            }
            ui.add_space(8.0);

            // Breadcrumb: folders, then the note's name.
            let folder = vault::parent(&rel);
            if !folder.is_empty() {
                for part in folder.split('/') {
                    ui.label(RichText::new(part).color(p.muted));
                    ui.label(RichText::new(" / ").color(p.faint));
                }
            }
            ui.label(RichText::new(vault::stem(&rel)).family(theme::semibold()).size(14.5));
            if dirty {
                ui.add_space(6.0);
                ui.label(RichText::new("●").size(9.0).color(p.faint)).on_hover_text("Unsaved changes");
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let more = widgets::icon_button(ui, icon::DOTS_THREE, "More");
                Popup::menu(&more).show(|ui| {
                    ui.set_min_width(220.0);
                    if widgets::menu_item(ui, icon::PENCIL_SIMPLE, "Rename…").clicked() {
                        self.actions.push(Action::Rename(rel.clone()));
                        ui.close();
                    }
                    if widgets::menu_item(ui, icon::FOLDER_SIMPLE, "Move to…").clicked() {
                        self.actions.push(Action::Move(rel.clone()));
                        ui.close();
                    }
                    if widgets::menu_item(ui, icon::IMAGE, "Add images…").clicked() {
                        self.actions.push(Action::AddImages(None));
                        ui.close();
                    }
                    if widgets::menu_item(ui, icon::COPY, "Copy as markdown").clicked() {
                        self.actions.push(Action::CopyNote);
                        ui.close();
                    }
                    if widgets::menu_item(ui, icon::ARROW_SQUARE_OUT, "Show in folder").clicked() {
                        self.actions.push(Action::Reveal(rel.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if widgets::menu_item_ex(ui, icon::TRASH, "Move to trash", None, true).clicked() {
                        self.actions.push(Action::Delete(rel.clone()));
                        ui.close();
                    }
                });
                let links = widgets::icon_button_ex(ui, icon::LINK_SIMPLE, "Links and backlinks", self.s.show_links, true);
                if links.clicked() {
                    self.s.show_links = !self.s.show_links;
                }
                ui.add_space(6.0);
                let mut view = self.s.view;
                let mut options = vec![
                    (View::Edit, icon::PENCIL_SIMPLE, "Edit"),
                    (View::Preview, icon::BOOK_OPEN, "Read"),
                ];
                if !self.narrow {
                    options.push((View::Split, icon::COLUMNS, ""));
                }
                if widgets::segmented(ui, &mut view, &options) {
                    self.actions.push(Action::SetView(view));
                }
            });
        });
    }

    fn toolbar_ui(&mut self, ui: &mut Ui) {
        let (can_undo, can_redo) = self.undo_state();
        centered(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                let mut tool = |ui: &mut Ui, glyph: &str, tip: &str, op: Op, enabled: bool| {
                    if widgets::icon_button_ex(ui, glyph, tip, false, enabled).clicked() {
                        self.actions.push(Action::Format(op));
                    }
                };
                tool(ui, icon::ARROW_U_UP_LEFT, "Undo (Ctrl+Z)", Op::Undo, can_undo);
                tool(ui, icon::ARROW_U_UP_RIGHT, "Redo (Ctrl+Y)", Op::Redo, can_redo);
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);
                tool(ui, icon::TEXT_H, "Heading", Op::Heading, true);
                tool(ui, icon::TEXT_B, "Bold (Ctrl+B)", Op::Bold, true);
                tool(ui, icon::TEXT_ITALIC, "Italic (Ctrl+I)", Op::Italic, true);
                tool(ui, icon::TEXT_STRIKETHROUGH, "Strikethrough", Op::Strike, true);
                tool(ui, icon::LIST_BULLETS, "Bulleted list", Op::List, true);
                tool(ui, icon::CHECK_SQUARE, "Checklist", Op::Checklist, true);
                tool(ui, icon::QUOTES, "Quote", Op::Quote, true);
                tool(ui, icon::BRACKETS_SQUARE, "Link to a note ([[ ]])", Op::WikiLink, true);
                tool(ui, icon::CODE, "Code", Op::Code, true);
                let colors = widgets::icon_button(ui, icon::PALETTE, "Text colour");
                Popup::menu(&colors).show(|ui| {
                    if let Some(op) = color_menu(ui) {
                        self.actions.push(Action::Format(op));
                        ui.close();
                    }
                });
                if widgets::icon_button(ui, icon::IMAGE, "Add images… (or drop them on the window)").clicked() {
                    self.actions.push(Action::AddImages(None));
                }
            });
        });
    }

    fn note_footer(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        let Some(n) = &self.note else { return };
        // Colour tags aren't words.
        let words = scrap_core::colors::strip(&n.text).split_whitespace().count();
        let chars = n.text.chars().count();
        let edited = self
            .vault
            .as_ref()
            .and_then(|v| v.index_of(&n.rel).map(|i| v.notes[i].modified))
            .map(crate::time::ago)
            .unwrap_or_default();
        let mut parts = Vec::new();
        if n.text != n.saved {
            parts.push("Saving…".to_string());
        } else if !edited.is_empty() {
            parts.push(format!("Edited {edited}"));
        }
        parts.push(format!("{} word{}", thousands(words), if words == 1 { "" } else { "s" }));
        parts.push(format!("{} character{}", thousands(chars), if chars == 1 { "" } else { "s" }));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(parts.join("  ·  ")).size(11.5).color(p.faint));
        });
    }

    pub(crate) fn toasts_ui(&mut self, ctx: &egui::Context) {
        if self.toasts.is_empty() {
            return;
        }
        let p = theme::palette(ctx);
        let mut dismiss = None;
        let mut run = None;
        egui::Area::new("toasts".into())
            .anchor(Align2::RIGHT_BOTTOM, [-18.0, -18.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                for (i, t) in self.toasts.iter().enumerate() {
                    Frame::new()
                        .fill(p.surface)
                        .stroke(Stroke::new(1.0, p.border))
                        .corner_radius(CornerRadius::same(10))
                        .shadow(ui.visuals().popup_shadow)
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_max_width(420.0);
                            ui.horizontal(|ui| {
                                let (glyph, color) =
                                    if t.err { (icon::WARNING_CIRCLE, p.danger) } else { (icon::CHECK_CIRCLE, p.success) };
                                ui.label(RichText::new(glyph).size(17.0).color(color));
                                let first = t.text.lines().next().unwrap_or("");
                                let label = ui.add(egui::Label::new(first).wrap());
                                if t.text.lines().count() > 1 {
                                    label.on_hover_text(&t.text);
                                }
                                if let Some((name, action)) = &t.action {
                                    if widgets::button(ui, Kind::Ghost, "", name).clicked() {
                                        run = Some(action.clone());
                                        dismiss = Some(i);
                                    }
                                }
                                if t.err && widgets::icon_button(ui, icon::X, "Dismiss").clicked() {
                                    dismiss = Some(i);
                                }
                            });
                        });
                    ui.add_space(8.0);
                }
            });
        if let Some(i) = dismiss {
            self.toasts.remove(i);
        }
        if let Some(a) = run {
            self.actions.push(a);
        }
    }

    pub(crate) fn drop_overlay(&self, ctx: &egui::Context) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let p = theme::palette(ctx);
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, "drop".into()));
        painter.rect_filled(rect, 0.0, Color32::from_black_alpha(90));
        let card = egui::Rect::from_center_size(rect.center(), egui::vec2(360.0, 140.0));
        painter.rect_filled(card, CornerRadius::same(16), p.surface);
        painter.rect_stroke(card, CornerRadius::same(16), Stroke::new(2.0, p.accent), egui::StrokeKind::Inside);
        painter.text(card.center() - egui::vec2(0.0, 18.0), Align2::CENTER_CENTER, icon::IMAGE, egui::FontId::proportional(30.0), p.accent);
        let text = if self.note.is_some() { "Drop images to add them to this note" } else { "Open a note to add images" };
        painter.text(card.center() + egui::vec2(0.0, 24.0), Align2::CENTER_CENTER, text, egui::FontId::proportional(14.0), p.text);
    }
}

/// Lays content out in a centered column of readable width.
pub fn centered<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let width = ui.available_width();
    let column = width.min(READABLE_WIDTH);
    let pad = ((width - column) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.set_width(column);
            add(ui)
        })
        .inner
    })
    .inner
}

pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Swatches for each text colour, and "No colour". Returns the chosen command.
fn color_menu(ui: &mut Ui) -> Option<Op> {
    let p = theme::palette(ui.ctx());
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for name in scrap_core::colors::NAMES {
            let color = p.text_color(name).unwrap_or(p.text);
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, CornerRadius::same(6), p.hover);
            }
            ui.painter().circle_filled(rect.center(), 9.0, color);
            let label = format!("{}{}", name[..1].to_uppercase(), &name[1..]);
            if resp.on_hover_text(label).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                chosen = Some(Op::Color(Some(name)));
            }
        }
    });
    ui.add_space(2.0);
    if widgets::menu_item(ui, icon::PROHIBIT, "No colour").clicked() {
        chosen = Some(Op::Color(None));
    }
    chosen
}
