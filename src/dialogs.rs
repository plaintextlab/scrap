//! Modal dialogs: naming things, confirming deletes, picking a folder to move to,
//! and git setup.

use crate::app::{Action, App, Dialog};
use crate::git;
use crate::theme::{self, icon};
use crate::vault::{self, Dir};
use crate::widgets::{self, Kind, Row};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Id, Key, Layout, Margin, Modal, RichText, ScrollArea,
    Stroke, Ui,
};

fn all_folders(dir: &Dir, out: &mut Vec<String>) {
    for d in &dir.dirs {
        out.push(d.rel.clone());
        all_folders(d, out);
    }
}

impl App {
    pub(crate) fn dialog_ui(&mut self, ctx: &egui::Context) {
        if self.dialog.is_none() {
            return;
        }
        let p = theme::palette(ctx);
        let mut folders = Vec::new();
        if let Some(v) = &self.vault {
            all_folders(&v.tree, &mut folders);
        }
        let Some(dialog) = &mut self.dialog else { return };

        let (title, glyph, ok, danger) = match dialog {
            Dialog::NewNote { .. } => ("New note", icon::NOTE_PENCIL, "Create", false),
            Dialog::NewFolder { .. } => ("New folder", icon::FOLDER_SIMPLE_PLUS, "Create", false),
            Dialog::Rename { .. } => ("Rename", icon::PENCIL_SIMPLE, "Rename", false),
            Dialog::Delete { .. } => ("Move to trash?", icon::TRASH, "Move to trash", true),
            Dialog::Move { .. } => ("Move to…", icon::FOLDER_SIMPLE, "", false),
            Dialog::Remote { .. } => ("Git remote", icon::GIT_BRANCH, "Save", false),
            Dialog::Clone { .. } => ("Clone a vault", icon::DOWNLOAD_SIMPLE, "Clone", false),
            Dialog::EmptyTrash => ("Empty the trash?", icon::TRASH, "Empty trash", true),
            Dialog::DeleteForever { .. } => ("Delete forever?", icon::TRASH, "Delete forever", true),
        };
        let mut submit = false;
        let mut move_to: Option<String> = None;

        let modal = Modal::new(Id::new("dialog"))
            .backdrop_color(Color32::from_black_alpha(if ctx.theme() == egui::Theme::Dark { 130 } else { 70 }))
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0, p.border))
                    .corner_radius(CornerRadius::same(14))
                    .shadow(ctx.style_of(ctx.theme()).visuals.window_shadow)
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.set_width(420.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
                    let (bg, fg) = if danger { (p.danger.gamma_multiply(0.15), p.danger) } else { (p.accent_soft, p.accent) };
                    ui.painter().rect_filled(rect, CornerRadius::same(10), bg);
                    ui.painter().text(rect.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(19.0), fg);
                    ui.add_space(4.0);
                    ui.label(RichText::new(title).family(theme::semibold()).size(17.0));
                });
                ui.add_space(14.0);

                let field = |ui: &mut Ui, value: &mut String, hint: &str| {
                    let r = widgets::input(ui, value, hint);
                    let entered = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    r.request_focus();
                    entered
                };
                let muted = |ui: &mut Ui, text: &str| {
                    ui.add(egui::Label::new(RichText::new(text).color(p.muted)).wrap());
                };
                match dialog {
                    Dialog::NewNote { folder, name } => {
                        if !folder.is_empty() {
                            muted(ui, &format!("In {folder}"));
                            ui.add_space(6.0);
                        }
                        submit |= field(ui, name, "Untitled");
                    }
                    Dialog::NewFolder { parent, name } => {
                        if !parent.is_empty() {
                            muted(ui, &format!("In {parent}"));
                            ui.add_space(6.0);
                        }
                        submit |= field(ui, name, "Folder name");
                    }
                    Dialog::Rename { name, .. } => {
                        submit |= field(ui, name, "New name");
                        ui.add_space(6.0);
                        muted(ui, "Links to it from other notes are updated too.");
                    }
                    Dialog::Delete { rel } => {
                        muted(ui, &format!("“{}” goes to the trash. You can restore it from Trash in the sidebar.", vault::stem(rel)));
                    }
                    Dialog::EmptyTrash => muted(ui, "Everything in the trash will be deleted for good. This can't be undone."),
                    Dialog::DeleteForever { name } => {
                        muted(ui, &format!("“{name}” will be deleted for good. This can't be undone."));
                    }
                    Dialog::Move { rel } => {
                        let is_dir = folders.contains(rel);
                        let current = vault::parent(rel).to_string();
                        muted(ui, &format!("Choose where to move “{}”.", vault::stem(rel)));
                        ui.add_space(8.0);
                        ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            let mut targets = vec![String::new()];
                            targets.extend(folders.iter().cloned());
                            for folder in targets {
                                let inside = is_dir && (folder == *rel || folder.starts_with(&format!("{rel}/")));
                                let disabled = folder == current || inside;
                                let depth = if folder.is_empty() { 0 } else { folder.matches('/').count() + 1 };
                                let name = if folder.is_empty() { "Notes (top level)".to_string() } else { vault::stem(&folder) };
                                let glyph = if folder.is_empty() { icon::TREE_STRUCTURE } else { icon::FOLDER_SIMPLE };
                                let mut row = Row::new(glyph, &name);
                                row.indent = depth.saturating_sub(1) as f32 * 16.0;
                                row.detail = (folder == current).then_some("current");
                                let resp = ui.add_enabled_ui(!disabled, |ui| widgets::row(ui, row)).inner;
                                if resp.clicked() && !disabled {
                                    move_to = Some(folder.clone());
                                }
                            }
                        });
                    }
                    Dialog::Remote { url } => {
                        muted(ui, "The repository to sync with, over SSH or HTTPS. Your existing git credentials are used.");
                        ui.add_space(8.0);
                        submit |= field(ui, url, "git@github.com:you/notes.git");
                    }
                    Dialog::Clone { url, parent } => {
                        muted(ui, "Copies a vault from GitHub or another git server to this computer.");
                        ui.add_space(8.0);
                        field(ui, url, "git@github.com:you/notes.git");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if widgets::button(ui, Kind::Secondary, icon::FOLDER_OPEN, "Choose folder…").clicked() {
                                if let Some(dir) = rfd::FileDialog::new().set_title("Clone into…").pick_folder() {
                                    *parent = Some(dir);
                                }
                            }
                            match parent {
                                Some(dir) => ui.label(RichText::new(dir.join(git::repo_name(url)).display().to_string()).size(12.5)),
                                None => ui.label(RichText::new("No folder chosen").color(p.faint)),
                            };
                        });
                    }
                }

                ui.add_space(18.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !ok.is_empty() {
                        let kind = if danger { Kind::Danger } else { Kind::Primary };
                        if widgets::button(ui, kind, "", ok).clicked() {
                            submit = true;
                        }
                    }
                    if widgets::button(ui, Kind::Secondary, "", "Cancel").clicked() {
                        ui.close();
                    }
                });
            });

        // Enter confirms the yes/no dialogs too.
        let confirm_only = matches!(dialog, Dialog::Delete { .. } | Dialog::EmptyTrash | Dialog::DeleteForever { .. });
        if confirm_only && ctx.input(|i| i.key_pressed(Key::Enter)) {
            submit = true;
        }
        let close = modal.should_close();
        if let Some(folder) = move_to {
            if let Some(Dialog::Move { rel }) = self.dialog.take() {
                self.actions.push(Action::MoveTo(rel, folder));
            }
            self.focus_editor = true;
        } else if submit {
            if let Some(d) = self.dialog.take() {
                self.submit(d);
            }
            self.focus_editor = true;
        } else if close {
            self.dialog = None;
            self.focus_editor = true;
        }
    }
}
