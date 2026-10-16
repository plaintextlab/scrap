//! The left sidebar: vault header, search, the folder tree (with drag and drop to
//! move things), and links to the trash, settings and sync.

use crate::app::{vault_name, Action, App, Page, Settings, Sort, ThemeChoice};
use crate::theme::{self, icon};
use crate::vault::{self, Dir, Vault};
use crate::widgets::{self, Row};
use eframe::egui::{
    self, Align, Align2, CornerRadius, CursorIcon, DragAndDrop, FontId, Layout, Popup, Pos2, RichText, ScrollArea,
    Sense, Stroke, StrokeKind, Ui,
};

/// What's being dragged in the tree: the path of a note or folder.
struct Dragged(String);

const FOOTER_HEIGHT: f32 = 112.0;

impl App {
    pub(crate) fn sidebar_ui(&mut self, ui: &mut Ui) {
        let Some(v) = &self.vault else { return };
        let p = theme::palette(ui.ctx());
        let name = vault_name(v);
        let folder = self.current_folder();
        let mut actions = Vec::new();
        let mut toggle_theme = false;

        // Header: vault name with its menu, then quick actions.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let label = format!("{name}  {}", icon::CARET_DOWN);
            let header = ui
                .add(egui::Label::new(RichText::new(label).family(theme::semibold()).size(15.0)).sense(Sense::click()))
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text(v.root.display().to_string());
            Popup::menu(&header).show(|ui| {
                ui.set_min_width(230.0);
                if self.s.repos.len() > 1 {
                    for repo in &self.s.repos {
                        let current = repo.path == v.root;
                        let glyph = if current { icon::CHECK } else { icon::ARROWS_LEFT_RIGHT };
                        let resp = widgets::menu_item(ui, glyph, &repo.name());
                        let resp = if repo.url.is_empty() { resp } else { resp.on_hover_text(&repo.url) };
                        if resp.clicked() {
                            if !current {
                                actions.push(Action::SwitchRepo(repo.path.clone()));
                            }
                            ui.close();
                        }
                    }
                    ui.separator();
                }
                if widgets::menu_item(ui, icon::FOLDER_OPEN, "Open another folder…").clicked() {
                    actions.push(Action::OpenVault);
                    ui.close();
                }
                if widgets::menu_item(ui, icon::DOWNLOAD_SIMPLE, "Clone a vault from git…").clicked() {
                    actions.push(Action::CloneVault);
                    ui.close();
                }
                if widgets::menu_item(ui, icon::ARROW_SQUARE_OUT, "Show in file manager").clicked() {
                    actions.push(Action::Reveal(String::new()));
                    ui.close();
                }
                if widgets::menu_item(ui, icon::ARROWS_CLOCKWISE, "Reload from disk").clicked() {
                    actions.push(Action::Rescan);
                    ui.close();
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if !self.narrow && widgets::icon_button(ui, icon::SIDEBAR_SIMPLE, "Hide sidebar (Ctrl+\\)").clicked() {
                    self.s.show_sidebar = false;
                }
                if self.narrow && self.note.is_some() && widgets::icon_button(ui, icon::X, "Back to the note").clicked() {
                    self.s.show_sidebar = false;
                }
                let (glyph, tip) = if ui.ctx().theme() == egui::Theme::Dark {
                    (icon::SUN, "Light theme")
                } else {
                    (icon::MOON, "Dark theme")
                };
                if widgets::icon_button(ui, glyph, tip).clicked() {
                    toggle_theme = true;
                }
                if widgets::icon_button(ui, icon::FOLDER_SIMPLE_PLUS, "New folder (Ctrl+Shift+N)").clicked() {
                    actions.push(Action::NewFolder(folder.clone()));
                }
                if widgets::icon_button(ui, icon::NOTE_PENCIL, "New note (Ctrl+N)").clicked() {
                    actions.push(Action::NewNote(folder.clone()));
                }
            });
        });
        if toggle_theme {
            let dark = ui.ctx().theme() == egui::Theme::Dark;
            self.s.theme = if dark { ThemeChoice::Light } else { ThemeChoice::Dark };
            self.apply_theme();
        }
        ui.add_space(8.0);

        // Search field look-alike that opens the search palette.
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), Sense::click());
        let fill = if resp.hovered() { p.hover } else { p.bg };
        ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
        ui.painter().rect_stroke(rect, CornerRadius::same(8), Stroke::new(1.0, p.border), StrokeKind::Inside);
        let cy = rect.center().y;
        ui.painter().text(Pos2::new(rect.left() + 18.0, cy), Align2::CENTER_CENTER, icon::MAGNIFYING_GLASS, FontId::proportional(15.0), p.muted);
        ui.painter().text(Pos2::new(rect.left() + 34.0, cy), Align2::LEFT_CENTER, "Search", FontId::proportional(13.5), p.faint);
        ui.painter().text(Pos2::new(rect.right() - 10.0, cy), Align2::RIGHT_CENTER, "Ctrl K", FontId::proportional(11.5), p.faint);
        if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            actions.push(Action::Search);
        }
        ui.add_space(14.0);

        // "Notes" header: sort menu, and a drop target for moving things to the top level.
        let header = ui.horizontal(|ui| {
            widgets::section_label(ui, "Notes");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let sort = widgets::icon_button(ui, icon::SORT_ASCENDING, "Sort");
                Popup::menu(&sort).show(|ui| {
                    ui.set_min_width(200.0);
                    for (value, label) in [(Sort::Name, "Name"), (Sort::Recent, "Last edited")] {
                        let glyph = if self.s.sort == value { icon::CHECK } else { "" };
                        if widgets::menu_item(ui, glyph, label).clicked() {
                            self.s.sort = value;
                            ui.close();
                        }
                    }
                });
            });
        });
        let root_drop = ui.interact(header.response.rect, ui.id().with("root-drop"), Sense::hover());
        drop_target(ui, &root_drop, "", &mut actions);

        let current = if self.page == Page::Notes { self.note.as_ref().map(|n| n.rel.clone()) } else { None };
        let tree_height = (ui.available_height() - FOOTER_HEIGHT).max(80.0);
        ScrollArea::vertical().max_height(tree_height).auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            tree(ui, v, &v.tree, 0, &self.s, current.as_deref(), &mut actions);
            if v.notes.is_empty() && v.tree.dirs.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new("No notes yet. Press Ctrl+N to write one.").color(p.muted));
            }
        });
        drag_preview(ui);

        // Footer: trash, settings, sync.
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            self.sync_row(ui, &mut actions);
            let mut settings = Row::new(icon::GEAR_SIX, "Settings");
            settings.tint = Some(theme::palette(ui.ctx()).purple);
            settings.selected = self.page == Page::Settings;
            if widgets::row(ui, settings).clicked() {
                actions.push(Action::Page(Page::Settings));
            }
            let count = self.trash.len().to_string();
            let mut trash = Row::new(icon::TRASH, "Trash");
            trash.tint = Some(theme::palette(ui.ctx()).red);
            trash.selected = self.page == Page::Trash;
            trash.detail = (!self.trash.is_empty()).then_some(count.as_str());
            let resp = widgets::row(ui, trash);
            if resp.clicked() {
                actions.push(Action::Page(Page::Trash));
            }
            ui.add_space(4.0);
            let r = ui.available_rect_before_wrap();
            ui.painter().hline(r.x_range(), r.bottom(), Stroke::new(1.0, p.border));
        });
        self.actions.extend(actions);
    }

    fn sync_row(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        if !self.git.available {
            let mut row = Row::new(icon::CLOUD_SLASH, "Sync needs git");
            row.detail = Some("not installed");
            widgets::row(ui, row).on_hover_text("Install git to sync this vault with GitHub or any git remote.");
            return;
        }
        let Some(st) = &self.git.status else {
            widgets::row(ui, Row::new(icon::CLOUD, "Checking sync…"));
            return;
        };
        if !st.is_repo {
            if widgets::row(ui, Row::new(icon::CLOUD_SLASH, "Set up sync")).clicked() {
                actions.push(Action::Page(Page::Settings));
            }
            return;
        }
        let busy = self.git.busy.is_some();
        let label = if busy {
            "Syncing…".to_string()
        } else if st.remote.is_none() {
            "No remote yet".to_string()
        } else {
            match self.synced_at {
                Some(t) => format!("Synced {}", crate::time::ago(t)),
                None => "Sync now".to_string(),
            }
        };
        let mut detail = Vec::new();
        if st.changes > 0 {
            detail.push(format!("{} changed", st.changes));
        }
        if st.ahead > 0 {
            detail.push(format!("↑{}", st.ahead));
        }
        if st.behind > 0 {
            detail.push(format!("↓{}", st.behind));
        }
        let detail = detail.join(" ");
        let glyph = if busy {
            icon::CIRCLE_NOTCH
        } else if st.changes > 0 || st.ahead > 0 {
            icon::CLOUD_ARROW_UP
        } else {
            icon::CLOUD_CHECK
        };
        let mut row = Row::new(glyph, &label);
        let p = theme::palette(ui.ctx());
        row.tint = Some(if busy {
            p.aqua
        } else if st.changes > 0 || st.ahead > 0 {
            p.orange
        } else {
            p.green
        });
        row.detail = (!detail.is_empty()).then_some(detail.as_str());
        let resp = widgets::row(ui, row);
        if busy {
            ui.ctx().request_repaint();
        }
        let hint = format!("Branch {} · Click to sync (Ctrl+Shift+S)", st.branch);
        if resp.on_hover_text(hint).clicked() && !busy {
            if st.remote.is_none() {
                actions.push(Action::Page(Page::Settings));
            } else {
                actions.push(Action::Sync);
            }
        }
    }
}

fn tree(ui: &mut Ui, v: &Vault, dir: &Dir, depth: usize, s: &Settings, current: Option<&str>, actions: &mut Vec<Action>) {
    let indent = depth as f32 * 14.0;
    let p = theme::palette(ui.ctx());
    for d in &dir.dirs {
        let open = s.expanded.contains(&d.rel);
        let mut row = Row::new(if open { icon::FOLDER_OPEN } else { icon::FOLDER_SIMPLE }, &d.name);
        row.indent = indent;
        row.tint = Some(p.yellow);
        row.caret = Some(open);
        let resp = widgets::row(ui, row);
        resp.dnd_set_drag_payload(Dragged(d.rel.clone()));
        if resp.clicked() {
            actions.push(Action::ToggleFolder(d.rel.clone()));
        }
        drop_target(ui, &resp, &d.rel, actions);
        resp.context_menu(|ui| folder_menu(ui, &d.rel, actions));
        if open {
            tree(ui, v, d, depth + 1, s, current, actions);
        }
    }
    let mut notes: Vec<usize> = dir.notes.clone();
    if s.sort == Sort::Recent {
        notes.sort_by(|&a, &b| v.notes[b].modified.cmp(&v.notes[a].modified));
    }
    for i in notes {
        let note = &v.notes[i];
        let mut row = Row::new(icon::FILE_TEXT, &note.name);
        row.tint = Some(p.blue);
        row.indent = indent;
        row.selected = current == Some(note.rel.as_str());
        let resp = widgets::row(ui, row);
        resp.dnd_set_drag_payload(Dragged(note.rel.clone()));
        if resp.clicked() {
            actions.push(Action::Open(note.rel.clone()));
        }
        let edited = crate::time::ago(note.modified);
        let resp = if edited.is_empty() { resp } else { resp.on_hover_text(format!("Edited {edited}")) };
        resp.context_menu(|ui| note_menu(ui, &note.rel, actions));
    }
}

/// Highlights `resp` while something can be dropped on it, and moves it into `folder`
/// when dropped.
fn drop_target(ui: &Ui, resp: &egui::Response, folder: &str, actions: &mut Vec<Action>) {
    let Some(hover) = resp.dnd_hover_payload::<Dragged>() else { return };
    let allowed = hover.0 != folder && !folder.starts_with(&format!("{}/", hover.0)) && vault::parent(&hover.0) != folder;
    if !allowed {
        return;
    }
    let p = theme::palette(ui.ctx());
    ui.painter().rect_stroke(resp.rect, CornerRadius::same(6), Stroke::new(1.5, p.accent), StrokeKind::Inside);
    if let Some(dropped) = resp.dnd_release_payload::<Dragged>() {
        actions.push(Action::MoveTo(dropped.0.clone(), folder.to_string()));
    }
}

/// A label following the pointer while something is being dragged.
fn drag_preview(ui: &Ui) {
    let Some(dragged) = DragAndDrop::payload::<Dragged>(ui.ctx()) else { return };
    let Some(pos) = ui.ctx().pointer_interact_pos() else { return };
    let p = theme::palette(ui.ctx());
    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, "drag".into()));
    let text = format!("{}  {}", icon::ARROW_ELBOW_DOWN_RIGHT, vault::stem(&dragged.0));
    let g = painter.layout_no_wrap(text, FontId::proportional(13.0), p.text);
    let rect = egui::Rect::from_min_size(pos + egui::vec2(14.0, 10.0), g.size() + egui::vec2(20.0, 12.0));
    painter.rect_filled(rect, CornerRadius::same(6), p.surface);
    painter.rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, p.border), StrokeKind::Inside);
    painter.galley(rect.min + egui::vec2(10.0, 6.0), g, p.text);
    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
}

fn folder_menu(ui: &mut Ui, rel: &str, actions: &mut Vec<Action>) {
    ui.set_min_width(220.0);
    let items: [(&str, &str, Action); 5] = [
        (icon::NOTE_PENCIL, "New note here", Action::NewNote(rel.to_string())),
        (icon::FOLDER_SIMPLE_PLUS, "New folder here", Action::NewFolder(rel.to_string())),
        (icon::PENCIL_SIMPLE, "Rename…", Action::Rename(rel.to_string())),
        (icon::FOLDER_SIMPLE, "Move to…", Action::Move(rel.to_string())),
        (icon::ARROW_SQUARE_OUT, "Show in file manager", Action::Reveal(rel.to_string())),
    ];
    for (glyph, label, action) in items {
        if widgets::menu_item(ui, glyph, label).clicked() {
            actions.push(action);
            ui.close();
        }
    }
    ui.separator();
    if widgets::menu_item_ex(ui, icon::TRASH, "Move to trash", None, true).clicked() {
        actions.push(Action::Delete(rel.to_string()));
        ui.close();
    }
}

fn note_menu(ui: &mut Ui, rel: &str, actions: &mut Vec<Action>) {
    ui.set_min_width(220.0);
    let items: [(&str, &str, Action); 3] = [
        (icon::PENCIL_SIMPLE, "Rename…", Action::Rename(rel.to_string())),
        (icon::FOLDER_SIMPLE, "Move to…", Action::Move(rel.to_string())),
        (icon::ARROW_SQUARE_OUT, "Show in file manager", Action::Reveal(rel.to_string())),
    ];
    for (glyph, label, action) in items {
        if widgets::menu_item(ui, glyph, label).clicked() {
            actions.push(action);
            ui.close();
        }
    }
    ui.separator();
    if widgets::menu_item_ex(ui, icon::TRASH, "Move to trash", None, true).clicked() {
        actions.push(Action::Delete(rel.to_string()));
        ui.close();
    }
}
