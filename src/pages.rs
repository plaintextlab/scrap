//! Full pages in the main area: welcome, nothing open, trash and settings, plus the
//! links panel.

use crate::app::{Action, App, OpenIn, Page, ThemeChoice};
use crate::layout::centered;
use crate::theme::{self, icon};
use crate::time;
use crate::vault;
use crate::widgets::{self, Kind, Row};
use eframe::egui::{self, Align, Align2, CornerRadius, FontId, Layout, Margin, RichText, ScrollArea, Ui};

impl App {
    pub(crate) fn welcome_ui(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space((ui.available_height() * 0.16).max(32.0));
            ui.vertical_centered(|ui| {
                ui.set_max_width(440.0);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, CornerRadius::same(18), p.accent);
                ui.painter().text(rect.center(), Align2::CENTER_CENTER, icon::NOTE_PENCIL, FontId::proportional(32.0), p.on_accent);
                ui.add_space(18.0);
                ui.label(RichText::new("Scrap").family(theme::semibold()).size(30.0));
                ui.add_space(4.0);
                ui.label(RichText::new("Plain markdown notes in a folder, synced with git.").size(15.0).color(p.muted));
                ui.add_space(28.0);

                centered_row(ui, |ui| {
                    if widgets::button(ui, Kind::Primary, icon::FOLDER_OPEN, "Open a folder").clicked() {
                        self.actions.push(Action::OpenVault);
                    }
                    ui.add_space(6.0);
                    let clone = widgets::button_ex(ui, Kind::Secondary, icon::DOWNLOAD_SIMPLE, "Clone from git", self.git.available);
                    if clone.clicked() {
                        self.actions.push(Action::CloneVault);
                    }
                });
                if self.git.busy == Some("clone") {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 50.0);
                        ui.spinner();
                        ui.label("Cloning…");
                    });
                }
                if !self.s.recent.is_empty() {
                    ui.add_space(36.0);
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        widgets::section_label(ui, "Recent vaults");
                        for path in self.s.recent.clone() {
                            let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
                            let full = path.display().to_string();
                            let mut row = Row::new(icon::FOLDER_SIMPLE, &name);
                            row.tint = Some(p.yellow);
                            row.detail = Some(&full);
                            row.height = 34.0;
                            if widgets::row(ui, row).on_hover_text(&full).clicked() {
                                self.open_vault(path);
                            }
                        }
                    });
                }
            });
        });
    }

    pub(crate) fn empty_ui(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.empty_contents(ui));
    }

    fn empty_contents(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.add_space(8.0);
                self.sidebar_toggle(ui);
            });
        });
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.22).max(24.0));
            ui.label(RichText::new(icon::NOTE_BLANK).size(52.0).color(p.faint));
            ui.add_space(12.0);
            ui.label(RichText::new("No note open").family(theme::semibold()).size(20.0));
            ui.add_space(4.0);
            ui.label(RichText::new("Pick a note on the left, or start a new one.").color(p.muted));
            ui.add_space(20.0);
            centered_row(ui, |ui| {
                if widgets::button(ui, Kind::Primary, icon::NOTE_PENCIL, "New note").clicked() {
                    self.actions.push(Action::NewNote(String::new()));
                }
                ui.add_space(6.0);
                if widgets::button(ui, Kind::Secondary, icon::MAGNIFYING_GLASS, "Search").clicked() {
                    self.actions.push(Action::Search);
                }
            });
            ui.add_space(36.0);
            let width = ui.available_width().min(440.0);
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - width) / 2.0);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    widgets::card(ui, shortcuts);
                });
            });
        });
    }

    /// Title row for a page, with the sidebar toggle.
    fn page_header(&mut self, ui: &mut Ui, title: &str, subtitle: &str, right: impl FnOnce(&mut Self, &mut Ui)) {
        let p = theme::palette(ui.ctx());
        ui.horizontal(|ui| {
            self.sidebar_toggle(ui);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).family(theme::semibold()).size(24.0));
                if !subtitle.is_empty() {
                    ui.label(RichText::new(subtitle).color(p.muted));
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| right(self, ui));
        });
        ui.add_space(18.0);
    }

    pub(crate) fn trash_ui(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().inner_margin(Margin::symmetric(28, 26)).show(ui, |ui| {
                centered(ui, |ui| {
                    let has_items = !self.trash.is_empty();
                    self.page_header(ui, "Trash", "Deleted notes and folders. The trash stays on this device and isn't synced.", |app, ui| {
                        if has_items && widgets::button(ui, Kind::Danger, icon::TRASH, "Empty trash").clicked() {
                            app.actions.push(Action::EmptyTrash);
                        }
                    });
                    if self.trash.is_empty() {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new(icon::TRASH).size(48.0).color(p.faint));
                            ui.add_space(8.0);
                            ui.label(RichText::new("Trash is empty").family(theme::semibold()).size(17.0));
                            ui.label(RichText::new("Deleted notes and folders show up here, ready to restore.").color(p.muted));
                        });
                        return;
                    }
                    for item in self.trash.clone() {
                        widgets::card(ui, |ui| {
                            ui.horizontal(|ui| {
                                let glyph = if item.is_dir { icon::FOLDER_SIMPLE } else { icon::FILE_TEXT };
                                ui.label(RichText::new(glyph).size(20.0).color(p.muted));
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    let name = item.original.rsplit('/').next().unwrap_or(&item.original);
                                    let name = if item.is_dir { name } else { name.strip_suffix(".md").unwrap_or(name) };
                                    ui.label(RichText::new(name).family(theme::semibold()));
                                    let from = vault::parent(&item.original);
                                    let mut detail = format!("From {}", if from.is_empty() { "Notes" } else { from });
                                    let ago = time::ago(item.deleted);
                                    if !ago.is_empty() {
                                        detail += &format!("  ·  deleted {ago}");
                                    }
                                    ui.label(RichText::new(detail).size(12.5).color(p.muted));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if widgets::icon_button(ui, icon::TRASH, "Delete forever").clicked() {
                                        self.actions.push(Action::DeleteForever(item.name.clone()));
                                    }
                                    if widgets::button(ui, Kind::Secondary, icon::ARROW_COUNTER_CLOCKWISE, "Restore").clicked() {
                                        self.actions.push(Action::Restore(item.name.clone()));
                                    }
                                });
                            });
                        });
                        ui.add_space(8.0);
                    }
                });
            });
        });
    }

    pub(crate) fn settings_ui(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Frame::new().inner_margin(Margin::symmetric(28, 26)).show(ui, |ui| {
                centered(ui, |ui| {
                    self.page_header(ui, "Settings", "", |_, _| {});

                    section(ui, "Appearance");
                    widgets::card(ui, |ui| {
                        setting_row(ui, "Theme", "", |ui| {
                            let mut theme = self.s.theme;
                            let options = [
                                (ThemeChoice::System, icon::DESKTOP, "System"),
                                (ThemeChoice::Light, icon::SUN, "Light"),
                                (ThemeChoice::Dark, icon::MOON, "Dark"),
                            ];
                            if widgets::segmented(ui, &mut theme, &options) {
                                self.s.theme = theme;
                                self.apply_theme();
                            }
                        });
                        ui.add_space(10.0);
                        setting_row(ui, "Text size", "For notes in the editor and reader.", |ui| {
                            ui.add(egui::Slider::new(&mut self.s.font_size, 12.0..=24.0).step_by(1.0).fixed_decimals(0).suffix(" px"));
                        });
                    });

                    section(ui, "Editor");
                    widgets::card(ui, |ui| {
                        setting_row(ui, "Open notes in", "", |ui| {
                            let mut open_in = self.s.open_in;
                            let options = [
                                (OpenIn::Last, "", "Last used"),
                                (OpenIn::Edit, icon::PENCIL_SIMPLE, "Edit"),
                                (OpenIn::Read, icon::BOOK_OPEN, "Read"),
                            ];
                            if widgets::segmented(ui, &mut open_in, &options) {
                                self.s.open_in = open_in;
                            }
                        });
                        ui.add_space(10.0);
                        widgets::switch(ui, &mut self.s.show_links, "Links panel", "Backlinks and links of the open note, on the right.");
                    });

                    section(ui, "Repositories");
                    widgets::card(ui, |ui| self.repos_settings(ui));

                    section(ui, "Sync");
                    widgets::card(ui, |ui| self.sync_settings(ui));

                    section(ui, "Keyboard shortcuts");
                    widgets::card(ui, shortcuts);
                    ui.add_space(12.0);
                    ui.label(RichText::new(format!("Scrap {}", env!("CARGO_PKG_VERSION"))).size(12.0).color(p.faint));
                });
            });
        });
    }

    /// Every vault in the switcher, with the current one marked; switch, remove or add.
    fn repos_settings(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        ui.label(
            RichText::new("Each repository is its own folder of notes with its own git remote. Switch between them here or from the vault menu at the top of the sidebar.")
                .size(12.5)
                .color(p.muted),
        );
        ui.add_space(10.0);
        let current = self.vault.as_ref().map(|v| v.root.clone());
        for repo in self.s.repos.clone() {
            let is_current = current.as_ref() == Some(&repo.path);
            let remote = if repo.url.is_empty() { "No git remote".to_string() } else { repo.url.clone() };
            let help = format!("{remote}\n{}", repo.path.display());
            ui.horizontal(|ui| {
                let glyph = if repo.url.is_empty() { icon::FOLDER_SIMPLE } else { icon::GIT_BRANCH };
                let color = if is_current { p.aqua } else { p.yellow };
                ui.label(RichText::new(glyph).size(18.0).color(color));
                setting_row(ui, &repo.name(), &help, |ui| {
                    if is_current {
                        ui.label(RichText::new(format!("{}  Current", icon::CHECK)).color(p.green));
                    } else {
                        if widgets::icon_button(ui, icon::X, "Remove from the list (the folder stays on disk)").clicked() {
                            self.actions.push(Action::RemoveRepo(repo.path.clone()));
                        }
                        if widgets::button(ui, Kind::Secondary, icon::ARROWS_LEFT_RIGHT, "Switch").clicked() {
                            self.actions.push(Action::SwitchRepo(repo.path.clone()));
                        }
                    }
                });
            });
            ui.add_space(8.0);
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if widgets::button(ui, Kind::Secondary, icon::DOWNLOAD_SIMPLE, "Add from git…").clicked() {
                self.actions.push(Action::CloneVault);
            }
            if widgets::button(ui, Kind::Secondary, icon::FOLDER_OPEN, "Add a folder…").clicked() {
                self.actions.push(Action::OpenVault);
            }
            if widgets::button(ui, Kind::Ghost, icon::ARROW_SQUARE_OUT, "Show in file manager").clicked() {
                self.actions.push(Action::Reveal(String::new()));
            }
        });
    }

    fn sync_settings(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        if !self.git.available {
            ui.label(RichText::new(format!("{}  git wasn't found", icon::WARNING_CIRCLE)).color(ui.visuals().warn_fg_color));
            ui.label(RichText::new("Install git to sync this vault with GitHub or any other git remote.").color(p.muted));
            return;
        }
        let Some(st) = self.git.status.clone() else {
            ui.spinner();
            return;
        };
        let busy = self.git.busy.is_some();
        if !st.is_repo {
            ui.label("This vault isn't a git repository yet.");
            ui.label(
                RichText::new("Turn it into one to keep a history of your notes and sync them to GitHub or any git server.")
                    .color(p.muted),
            );
            ui.add_space(10.0);
            if widgets::button_ex(ui, Kind::Primary, icon::GIT_BRANCH, "Set up git", !busy).clicked() {
                self.git.init(&self.ctx);
            }
            return;
        }

        setting_row(ui, "Remote", st.remote.as_deref().unwrap_or("None yet: add one to sync with GitHub or another server."), |ui| {
            let label = if st.remote.is_some() { "Change…" } else { "Add remote…" };
            if widgets::button(ui, Kind::Secondary, "", label).clicked() {
                self.dialog = Some(crate::app::Dialog::Remote { url: st.remote.clone().unwrap_or_default() });
            }
        });
        ui.add_space(8.0);
        let mut status = vec![format!("Branch {}", st.branch)];
        if st.changes > 0 {
            status.push(format!("{} unsynced change{}", st.changes, if st.changes == 1 { "" } else { "s" }));
        }
        if st.ahead > 0 {
            status.push(format!("{} to push", st.ahead));
        }
        if st.behind > 0 {
            status.push(format!("{} to pull", st.behind));
        }
        if let Some(t) = self.synced_at {
            status.push(format!("synced {}", time::ago(t)));
        }
        setting_row(ui, "Status", &status.join("  ·  "), |ui| {
            let label = if busy { "Syncing…" } else { "Sync now" };
            if widgets::button_ex(ui, Kind::Primary, icon::ARROWS_CLOCKWISE, label, !busy && st.remote.is_some()).clicked() {
                self.actions.push(Action::Sync);
            }
        });
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);
        setting_row(ui, "Sync automatically", "Minutes between syncs; 0 turns it off.", |ui| {
            ui.add(egui::Slider::new(&mut self.s.auto_sync_minutes, 0..=120).suffix(" min"));
        });
        ui.add_space(6.0);
        widgets::switch(ui, &mut self.s.sync_on_open, "Sync when the vault opens", "");
        ui.add_space(6.0);
        widgets::switch(ui, &mut self.s.sync_on_close, "Sync when closing Scrap", "");
        ui.add_space(10.0);
        ui.label(
            RichText::new(
                "Uses your installed git and its credentials (SSH keys, credential manager). If the same note \
                 changed on two devices, both versions are kept: yours stays put and the other is saved as \
                 \"Note (conflict).md\".",
            )
            .size(12.5)
            .color(p.muted),
        );
    }

    pub(crate) fn links_ui(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui.ctx());
        let (Some(v), Some(n)) = (&self.vault, &self.note) else { return };
        let key = (n.rel.clone(), v.generation);
        if self.backlinks_key != key {
            self.backlinks = v.index_of(&n.rel).map(|i| v.backlinks(i)).unwrap_or_default();
            self.backlinks_key = key;
        }
        let mut actions = Vec::new();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Links").family(theme::semibold()).size(15.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, icon::X, "Close").clicked() {
                    self.s.show_links = false;
                }
            });
        });
        ui.add_space(8.0);
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            widgets::section_label(ui, "Linked from");
            if self.backlinks.is_empty() {
                ui.label(RichText::new("No notes link here yet.").color(p.muted));
            }
            for &i in &self.backlinks {
                let note = &v.notes[i];
                let folder = vault::parent(&note.rel);
                let mut row = Row::new(icon::FILE_TEXT, &note.name);
                row.tint = Some(p.blue);
                row.detail = (!folder.is_empty()).then_some(folder);
                if widgets::row(ui, row).clicked() {
                    actions.push(Action::Open(note.rel.clone()));
                }
            }
            ui.add_space(14.0);
            widgets::section_label(ui, "Links in this note");
            let mut links = vault::extract_links(&n.text);
            links.dedup();
            if links.is_empty() {
                ui.label(RichText::new("Link to another note with [[Note name]].").color(p.muted));
            }
            for link in links {
                let exists = v.resolve(&link).is_some();
                let mut row = Row::new(if exists { icon::FILE_TEXT } else { icon::FILE_DASHED }, &link);
                row.tint = Some(if exists { p.aqua } else { p.faint });
                row.detail = (!exists).then_some("create");
                let resp = widgets::row(ui, row);
                let hint = if exists { "Open" } else { "Not created yet; click to create it" };
                if resp.on_hover_text(hint).clicked() {
                    actions.push(Action::Link(format!("wiki:{link}")));
                }
            }
        });
        self.actions.extend(actions);
        let _ = Page::Notes;
    }
}

/// A row of widgets centred horizontally. The row is measured on the first frame and
/// centred from then on, which egui's immediate mode needs for content of unknown width.
fn centered_row(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let id = ui.id().with(("centered-row", ui.next_auto_id()));
    let width: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(0.0);
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
        let start = ui.cursor().left();
        add(ui);
        let used = ui.min_rect().right() - start;
        if (used - width).abs() > 0.5 {
            ui.ctx().data_mut(|d| d.insert_temp(id, used));
            ui.ctx().request_discard("centering");
        }
    });
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(14.0);
    widgets::section_label(ui, title);
    ui.add_space(2.0);
}

/// Label (and optional help) on the left, a control on the right.
fn setting_row(ui: &mut Ui, label: &str, help: &str, control: impl FnOnce(&mut Ui)) {
    let p = theme::palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width((ui.available_width() - 300.0).max(160.0));
            ui.label(label);
            if !help.is_empty() {
                ui.add(egui::Label::new(RichText::new(help).size(12.5).color(p.muted)).wrap());
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
}

fn shortcuts(ui: &mut Ui) {
    let p = theme::palette(ui.ctx());
    let keys = [
        ("Ctrl K", "Search notes"),
        ("Ctrl N", "New note"),
        ("Ctrl Shift N", "New folder"),
        ("Ctrl E", "Switch between Edit and Read"),
        ("Ctrl B / Ctrl I", "Bold / italic"),
        ("Ctrl \\", "Show or hide the sidebar"),
        ("Ctrl Shift S", "Sync"),
        ("Ctrl Alt ← / →", "Back / forward"),
        ("Ctrl click", "Follow a link in the editor"),
        ("Ctrl + / Ctrl −", "Zoom"),
    ];
    egui::Grid::new("shortcuts").num_columns(2).spacing([28.0, 8.0]).show(ui, |ui| {
        for (k, what) in keys {
            ui.label(RichText::new(k).family(egui::FontFamily::Monospace).size(12.5).color(p.text));
            ui.label(RichText::new(what).color(p.muted));
            ui.end_row();
        }
    });
}
