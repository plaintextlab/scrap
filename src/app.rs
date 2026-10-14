//! Application state and the actions that change it. The views live in their own
//! modules (`sidebar`, `editor`, `pages`, `search`, `dialogs`) as further `impl App` blocks.

use crate::git::{self, Git};
use crate::theme;
use crate::vault::{self, TrashItem, Vault};
use crate::{attach, editor, time};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);
const TOAST_TIME: Duration = Duration::from_secs(5);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Edit,
    Preview,
    Split,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeChoice {
    System,
    Light,
    Dark,
}

/// How a note opens.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpenIn {
    /// Whatever view was used last.
    Last,
    Edit,
    Read,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sort {
    Name,
    Recent,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub vault: Option<PathBuf>,
    pub recent: Vec<PathBuf>,
    pub last_note: Option<String>,
    pub theme: ThemeChoice,
    pub font_size: f32,
    pub view: View,
    pub open_in: OpenIn,
    pub show_sidebar: bool,
    pub show_links: bool,
    pub sort: Sort,
    /// Folders open in the sidebar tree.
    pub expanded: BTreeSet<String>,
    pub auto_sync_minutes: u32,
    pub sync_on_open: bool,
    pub sync_on_close: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            vault: None,
            recent: Vec::new(),
            last_note: None,
            theme: ThemeChoice::System,
            font_size: 16.0,
            view: View::Edit,
            open_in: OpenIn::Last,
            show_sidebar: true,
            show_links: false,
            sort: Sort::Name,
            expanded: BTreeSet::new(),
            auto_sync_minutes: 10,
            sync_on_open: true,
            sync_on_close: true,
        }
    }
}

pub(crate) struct OpenNote {
    pub rel: String,
    pub text: String,
    pub saved: String,
    /// The file used CRLF line endings; they are restored on save.
    pub crlf: bool,
    pub edited: Option<Instant>,
}

/// What the main area shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Page {
    Notes,
    Trash,
    Settings,
}

pub(crate) enum Dialog {
    NewNote { folder: String, name: String },
    NewFolder { parent: String, name: String },
    Rename { rel: String, name: String },
    Delete { rel: String },
    Move { rel: String },
    Remote { url: String },
    Clone { url: String, parent: Option<PathBuf> },
    EmptyTrash,
    DeleteForever { name: String },
}

#[derive(Clone, Debug)]
pub(crate) enum Action {
    Open(String),
    Link(String),
    NewNote(String),
    NewFolder(String),
    Rename(String),
    Delete(String),
    Move(String),
    MoveTo(String, String),
    Restore(String),
    DeleteForever(String),
    EmptyTrash,
    Save,
    Sync,
    Rescan,
    OpenVault,
    CloneVault,
    Back,
    Forward,
    Search,
    Page(Page),
    SetView(View),
    ToggleView,
    Format(editor::Op),
    /// Add images to the open note; None asks with a file dialog.
    AddImages(Option<Vec<PathBuf>>),
    CopyNote,
    Reveal(String),
    ToggleFolder(String),
}

pub(crate) struct Toast {
    pub text: String,
    pub err: bool,
    pub at: Instant,
    /// A button on the toast, such as "Undo".
    pub action: Option<(String, Action)>,
}

#[derive(Default)]
pub(crate) struct SearchState {
    pub query: String,
    pub selected: usize,
}

pub struct App {
    pub(crate) ctx: egui::Context,
    pub(crate) s: Settings,
    pub(crate) vault: Option<Vault>,
    pub(crate) note: Option<OpenNote>,
    pub(crate) page: Page,
    pub(crate) history: Vec<String>,
    pub(crate) hist_pos: usize,
    pub(crate) search: Option<SearchState>,
    pub(crate) dialog: Option<Dialog>,
    pub(crate) git: Git,
    pub(crate) last_sync: Instant,
    /// When the last sync finished successfully (Unix millis).
    pub(crate) synced_at: Option<u64>,
    pub(crate) unsynced: bool,
    sync_after_status: bool,
    pub(crate) toasts: Vec<Toast>,
    focused: bool,
    pub(crate) focus_editor: bool,
    title: String,
    pub(crate) actions: Vec<Action>,
    /// Phone-sized window: the file list replaces the note instead of sitting beside it.
    pub(crate) narrow: bool,
    pub(crate) trash: Vec<TrashItem>,
    pub(crate) backlinks: Vec<usize>,
    pub(crate) backlinks_key: (String, u64),
    pub(crate) dev: Option<crate::dev::Dev>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, vault: Option<PathBuf>) -> Self {
        let s: Settings = cc.storage.and_then(|st| eframe::get_value(st, eframe::APP_KEY)).unwrap_or_default();
        theme::install(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let mut app = App {
            ctx: cc.egui_ctx.clone(),
            s,
            vault: None,
            note: None,
            page: Page::Notes,
            history: Vec::new(),
            hist_pos: 0,
            search: None,
            dialog: None,
            git: Git::new(),
            last_sync: Instant::now(),
            synced_at: None,
            unsynced: false,
            sync_after_status: false,
            toasts: Vec::new(),
            focused: true,
            focus_editor: false,
            title: String::new(),
            actions: Vec::new(),
            narrow: false,
            trash: Vec::new(),
            backlinks: Vec::new(),
            backlinks_key: (String::new(), 0),
            dev: crate::dev::Dev::from_env(),
        };
        app.apply_theme();
        if let Some(path) = vault.or_else(|| app.s.vault.clone()) {
            app.open_vault(path);
        }
        app
    }

    pub(crate) fn apply_theme(&self) {
        self.ctx.set_theme(match self.s.theme {
            ThemeChoice::System => egui::ThemePreference::System,
            ThemeChoice::Light => egui::ThemePreference::Light,
            ThemeChoice::Dark => egui::ThemePreference::Dark,
        });
    }

    // ---------------------------------------------------------------------
    // Messages
    // ---------------------------------------------------------------------

    pub(crate) fn info(&mut self, text: impl Into<String>) {
        self.toast(text, false, None);
    }

    pub(crate) fn error(&mut self, text: impl Into<String>) {
        self.toast(text, true, None);
    }

    pub(crate) fn toast(&mut self, text: impl Into<String>, err: bool, action: Option<(String, Action)>) {
        self.toasts.retain(|t| t.err);
        self.toasts.push(Toast { text: text.into(), err, at: Instant::now(), action });
        self.toasts.truncate(3);
    }

    // ---------------------------------------------------------------------
    // Vault and notes
    // ---------------------------------------------------------------------

    pub(crate) fn open_vault(&mut self, path: PathBuf) {
        self.save_now();
        match Vault::open(path.clone()) {
            Ok(v) => {
                self.vault = Some(v);
                self.note = None;
                self.page = Page::Notes;
                self.history.clear();
                self.hist_pos = 0;
                self.s.recent.retain(|p| p != &path);
                self.s.recent.insert(0, path.clone());
                self.s.recent.truncate(8);
                if self.s.vault.as_ref() != Some(&path) {
                    self.s.expanded.clear();
                }
                self.s.vault = Some(path.clone());
                self.git.set_root(Some(path), &self.ctx);
                self.sync_after_status = self.s.sync_on_open;
                self.last_sync = Instant::now();
                self.synced_at = None;
                self.unsynced = false;
                if let Some(rel) = self.s.last_note.clone() {
                    if self.vault.as_ref().is_some_and(|v| v.index_of(&rel).is_some()) {
                        self.open(&rel, true);
                    }
                }
            }
            Err(e) => {
                self.s.vault = None;
                self.error(format!("Could not open {}: {e}", path.display()));
            }
        }
    }

    pub(crate) fn open(&mut self, rel: &str, push: bool) {
        self.save_now();
        let Some(v) = &self.vault else { return };
        match v.read(rel) {
            Ok(raw) => {
                let (text, crlf) = normalize(raw);
                let empty = text.trim().is_empty();
                self.note = Some(OpenNote { rel: rel.to_string(), saved: text.clone(), text, crlf, edited: None });
                self.page = Page::Notes;
                if push {
                    if !self.history.is_empty() {
                        self.history.truncate(self.hist_pos + 1);
                    }
                    if self.history.last().map(String::as_str) != Some(rel) {
                        self.history.push(rel.to_string());
                    }
                    self.hist_pos = self.history.len() - 1;
                }
                match self.s.open_in {
                    OpenIn::Last => {}
                    OpenIn::Edit => self.s.view = View::Edit,
                    OpenIn::Read => self.s.view = View::Preview,
                }
                // An empty note is for writing.
                if empty && self.s.view == View::Preview {
                    self.s.view = View::Edit;
                }
                self.reveal_in_tree(rel);
                self.s.last_note = Some(rel.to_string());
                self.focus_editor = true;
            }
            Err(e) => self.error(format!("Could not open {rel}: {e}")),
        }
    }

    /// Opens the folders above `rel` in the sidebar.
    fn reveal_in_tree(&mut self, rel: &str) {
        let mut folder = vault::parent(rel);
        while !folder.is_empty() {
            self.s.expanded.insert(folder.to_string());
            folder = vault::parent(folder);
        }
    }

    pub(crate) fn save_now(&mut self) {
        let (Some(v), Some(n)) = (self.vault.as_mut(), self.note.as_mut()) else { return };
        n.edited = None;
        if n.text == n.saved {
            return;
        }
        let out = if n.crlf { n.text.replace('\n', "\r\n") } else { n.text.clone() };
        match v.save(&n.rel, &out) {
            Ok(()) => {
                n.saved = n.text.clone();
                self.unsynced = true;
            }
            Err(e) => {
                let msg = format!("Could not save {}: {e}", n.rel);
                self.error(msg);
            }
        }
    }

    /// Picks up on-disk changes to the open note, unless it has unsaved edits.
    fn reload_current(&mut self) {
        let Some(v) = &self.vault else { return };
        let Some(n) = &mut self.note else { return };
        if n.text != n.saved {
            return;
        }
        match v.read(&n.rel) {
            Ok(raw) => {
                let (text, crlf) = normalize(raw);
                if text != n.saved {
                    n.text = text.clone();
                    n.saved = text;
                    n.crlf = crlf;
                }
            }
            Err(_) => self.note = None,
        }
    }

    pub(crate) fn rescan(&mut self) {
        self.save_now();
        if let Some(v) = &mut self.vault {
            v.rescan();
        }
        self.reload_current();
        self.refresh_trash();
    }

    pub(crate) fn refresh_trash(&mut self) {
        self.trash = self.vault.as_ref().map(Vault::trash).unwrap_or_default();
    }

    /// Points everything that refers to `rel` (or something inside it) at `new_rel`.
    fn relocated(&mut self, rel: &str, new_rel: &str) {
        let prefix = format!("{rel}/");
        let fix = |p: &mut String| {
            if p == rel {
                *p = new_rel.to_string();
            } else if let Some(rest) = p.strip_prefix(&prefix).map(str::to_string) {
                *p = format!("{new_rel}/{rest}");
            }
        };
        if let Some(n) = &mut self.note {
            fix(&mut n.rel);
        }
        self.history.iter_mut().for_each(fix);
        if let Some(last) = &mut self.s.last_note {
            fix(last);
        }
        let expanded: Vec<String> = std::mem::take(&mut self.s.expanded).into_iter().collect();
        for mut e in expanded {
            fix(&mut e);
            self.s.expanded.insert(e);
        }
        if let Some(n) = &self.note {
            let rel = n.rel.clone();
            self.reveal_in_tree(&rel);
        }
        // Links in the open note may have been rewritten.
        self.reload_current();
        self.unsynced = true;
    }

    // ---------------------------------------------------------------------
    // Sync
    // ---------------------------------------------------------------------

    pub(crate) fn sync(&mut self) {
        if !self.git.available {
            self.error("git is not installed or not on PATH");
            return;
        }
        if self.vault.is_none() || self.git.busy.is_some() {
            return;
        }
        match self.git.status.as_ref().map(|s| s.is_repo) {
            Some(true) => {
                self.save_now();
                self.git.sync(&self.ctx);
                self.last_sync = Instant::now();
                self.unsynced = false;
            }
            Some(false) => {
                self.page = Page::Settings;
                self.info("This vault isn't a git repository yet. Set it up under Sync.");
            }
            None => {}
        }
    }

    fn poll_git(&mut self) {
        for finished in self.git.poll() {
            let op = finished.op;
            match finished.result {
                Ok(path) if op == "clone" => {
                    self.open_vault(PathBuf::from(path));
                    self.info("Vault cloned");
                }
                Ok(msg) => {
                    if op == "sync" {
                        self.synced_at = Some(time::now_millis());
                    }
                    // Routine "nothing to do" syncs don't need a message.
                    if msg != "Already up to date" {
                        self.info(msg);
                    }
                }
                Err(e) => self.error(e),
            }
            if op != "clone" {
                self.rescan();
            }
        }
        if self.sync_after_status {
            if let Some(st) = &self.git.status {
                self.sync_after_status = false;
                if st.is_repo && st.remote.is_some() {
                    self.sync();
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // Links
    // ---------------------------------------------------------------------

    fn follow_link(&mut self, url: &str) {
        if let Some(target) = url.strip_prefix("wiki:") {
            self.open_wiki(target);
            return;
        }
        if url.contains("://") || url.starts_with("mailto:") {
            self.ctx.open_url(egui::OpenUrl::new_tab(url));
            return;
        }
        let decoded = url.replace("%20", " ");
        let path = decoded.split('#').next().unwrap_or("").trim_start_matches("./");
        if path.is_empty() {
            return;
        }
        let folder = self.current_folder();
        let joined = vault::join(&folder, path);
        let found = self
            .vault
            .as_ref()
            .and_then(|v| v.index_of(&joined).or_else(|| v.index_of(path)).map(|i| v.notes[i].rel.clone()));
        match found {
            Some(rel) => self.open(&rel, true),
            None => self.open_wiki(path),
        }
    }

    /// Opens the note a wiki link points to, creating it if it doesn't exist yet.
    fn open_wiki(&mut self, target: &str) {
        let Some(v) = &mut self.vault else { return };
        if let Some(i) = v.resolve(target) {
            let rel = v.notes[i].rel.clone();
            self.open(&rel, true);
            return;
        }
        let name = target.split('#').next().unwrap_or("").trim();
        let name = name.strip_suffix(".md").unwrap_or(name);
        if name.is_empty() {
            return;
        }
        let (folder, name) = name.rsplit_once('/').unwrap_or(("", name));
        match v.create_note(folder, name) {
            Ok(rel) => {
                self.unsynced = true;
                self.open(&rel, true);
            }
            Err(e) => self.error(format!("Could not create note: {e}")),
        }
    }

    pub(crate) fn current_folder(&self) -> String {
        self.note.as_ref().map(|n| vault::parent(&n.rel).to_string()).unwrap_or_default()
    }

    // ---------------------------------------------------------------------
    // Actions
    // ---------------------------------------------------------------------

    fn process_actions(&mut self) {
        for action in std::mem::take(&mut self.actions) {
            self.run(action);
        }
    }

    pub(crate) fn run(&mut self, action: Action) {
        match action {
            Action::Open(rel) => {
                self.open(&rel, true);
                if self.narrow {
                    self.s.show_sidebar = false;
                }
            }
            Action::Link(url) => self.follow_link(&url),
            Action::NewNote(folder) => self.dialog = Some(Dialog::NewNote { folder, name: String::new() }),
            Action::NewFolder(parent) => self.dialog = Some(Dialog::NewFolder { parent, name: String::new() }),
            Action::Rename(rel) => {
                let name = vault::stem(&rel);
                self.dialog = Some(Dialog::Rename { rel, name });
            }
            Action::Delete(rel) => self.dialog = Some(Dialog::Delete { rel }),
            Action::Move(rel) => self.dialog = Some(Dialog::Move { rel }),
            Action::MoveTo(rel, folder) => self.move_to(&rel, &folder),
            Action::Restore(name) => {
                let Some(v) = &mut self.vault else { return };
                match v.restore(&name) {
                    Ok(rel) => {
                        self.unsynced = true;
                        let to = vault::parent(&rel);
                        self.info(format!("Restored to {}", if to.is_empty() { "Notes" } else { to }));
                        self.refresh_trash();
                        if v_is_note(&rel) {
                            self.reveal_in_tree(&rel);
                        }
                    }
                    Err(e) => self.error(format!("Could not restore: {e}")),
                }
            }
            Action::DeleteForever(name) => self.dialog = Some(Dialog::DeleteForever { name }),
            Action::EmptyTrash => self.dialog = Some(Dialog::EmptyTrash),
            Action::Save => {
                self.save_now();
                self.info("Saved");
            }
            Action::Sync => self.sync(),
            Action::Rescan => self.rescan(),
            Action::OpenVault => {
                if let Some(path) = rfd::FileDialog::new().set_title("Open folder as vault").pick_folder() {
                    self.open_vault(path);
                }
            }
            Action::CloneVault => self.dialog = Some(Dialog::Clone { url: String::new(), parent: None }),
            Action::Back => {
                if self.hist_pos > 0 {
                    self.hist_pos -= 1;
                    let rel = self.history[self.hist_pos].clone();
                    self.open(&rel, false);
                }
            }
            Action::Forward => {
                if self.hist_pos + 1 < self.history.len() {
                    self.hist_pos += 1;
                    let rel = self.history[self.hist_pos].clone();
                    self.open(&rel, false);
                }
            }
            Action::Search => {
                if self.vault.is_some() {
                    self.search = Some(SearchState::default());
                }
            }
            Action::Page(page) => {
                self.save_now();
                self.page = page;
                if page == Page::Trash {
                    self.refresh_trash();
                }
                if self.narrow {
                    self.s.show_sidebar = false;
                }
            }
            Action::SetView(view) => {
                self.s.view = view;
                self.focus_editor = view != View::Preview;
            }
            Action::ToggleView => {
                let view = if self.s.view == View::Preview { View::Edit } else { View::Preview };
                self.run(Action::SetView(view));
            }
            Action::Format(op) => self.format(op),
            Action::AddImages(paths) => {
                let paths = paths.or_else(|| {
                    rfd::FileDialog::new()
                        .set_title("Add images")
                        .add_filter("Images", &attach::EXTENSIONS)
                        .pick_files()
                });
                if let Some(paths) = paths {
                    self.add_images(&paths);
                }
            }
            Action::CopyNote => {
                if let Some(n) = &self.note {
                    self.ctx.copy_text(n.text.clone());
                    self.info("Copied the note as markdown");
                }
            }
            Action::Reveal(rel) => {
                if let Some(v) = &self.vault {
                    let path = v.path(&rel);
                    let dir = if path.is_dir() { path } else { path.parent().map(PathBuf::from).unwrap_or(path) };
                    self.ctx.open_url(egui::OpenUrl::new_tab(attach::file_uri(&dir)));
                }
            }
            Action::ToggleFolder(rel) => {
                if !self.s.expanded.remove(&rel) {
                    self.s.expanded.insert(rel);
                }
            }
        }
    }

    fn move_to(&mut self, rel: &str, folder: &str) {
        self.save_now();
        let Some(v) = &mut self.vault else { return };
        match v.move_to(rel, folder) {
            Ok(new_rel) if new_rel != rel => {
                self.relocated(rel, &new_rel);
                if !folder.is_empty() {
                    self.s.expanded.insert(folder.to_string());
                }
                let name = vault::stem(rel);
                let to = if folder.is_empty() { "Notes".to_string() } else { vault::stem(folder) };
                self.info(format!("Moved {name} to {to}"));
            }
            Ok(_) => {}
            Err(e) => self.error(format!("Could not move: {e}")),
        }
    }

    fn add_images(&mut self, paths: &[PathBuf]) {
        let (Some(v), Some(n)) = (&self.vault, &self.note) else {
            self.error("Open a note to add images to it");
            return;
        };
        let root = v.root.clone();
        let rel = n.rel.clone();
        let mut markdown = Vec::new();
        for path in paths {
            if !attach::is_image(path) {
                self.error(format!("{} isn't an image", path.display()));
                continue;
            }
            match attach::import(&root, path) {
                Ok(saved) => markdown.push(format!("![]({})", scrap_core::attachments::link_from(&rel, &saved))),
                Err(e) => self.error(e),
            }
        }
        if markdown.is_empty() {
            return;
        }
        if self.s.view == View::Preview {
            self.s.view = View::Edit;
        }
        self.insert_block(&markdown.join("\n\n"));
        self.save_now();
        let n = markdown.len();
        self.info(if n == 1 { "Added image".to_string() } else { format!("Added {n} images") });
    }

    pub(crate) fn submit(&mut self, dialog: Dialog) {
        match dialog {
            Dialog::NewNote { folder, name } => {
                let Some(v) = &mut self.vault else { return };
                match v.create_note(&folder, &name) {
                    Ok(rel) => {
                        self.unsynced = true;
                        self.s.view = View::Edit;
                        self.open(&rel, true);
                    }
                    Err(e) => self.error(format!("Could not create note: {e}")),
                }
            }
            Dialog::NewFolder { parent, name } => {
                let Some(v) = &mut self.vault else { return };
                match v.create_folder(&parent, &name) {
                    Ok(rel) => {
                        self.unsynced = true;
                        if !parent.is_empty() {
                            self.s.expanded.insert(parent);
                        }
                        self.s.expanded.insert(rel);
                    }
                    Err(e) => self.error(format!("Could not create folder: {e}")),
                }
            }
            Dialog::Rename { rel, name } => {
                self.save_now();
                let Some(v) = &mut self.vault else { return };
                match v.rename(&rel, &name) {
                    Ok(new_rel) => self.relocated(&rel, &new_rel),
                    Err(e) => self.error(format!("Could not rename: {e}")),
                }
            }
            Dialog::Delete { rel } => {
                let prefix = format!("{rel}/");
                if self.note.as_ref().is_some_and(|n| n.rel == rel || n.rel.starts_with(&prefix)) {
                    self.save_now();
                    self.note = None;
                }
                let Some(v) = &mut self.vault else { return };
                match v.delete(&rel) {
                    Ok(()) => {
                        self.unsynced = true;
                        self.history.retain(|p| *p != rel && !p.starts_with(&prefix));
                        self.hist_pos = self.history.len().saturating_sub(1);
                        self.refresh_trash();
                        // The newest trash entry from this path is the one just deleted.
                        let undo = self
                            .trash
                            .iter()
                            .filter(|t| t.original == rel)
                            .max_by_key(|t| t.deleted)
                            .map(|t| ("Undo".to_string(), Action::Restore(t.name.clone())));
                        self.toast(format!("Moved {} to the trash", vault::stem(&rel)), false, undo);
                    }
                    Err(e) => self.error(format!("Could not delete: {e}")),
                }
            }
            Dialog::Move { .. } => {}
            Dialog::Remote { url } => {
                let url = url.trim().to_string();
                if !url.is_empty() {
                    self.git.set_remote(url, &self.ctx);
                }
            }
            Dialog::Clone { url, parent } => {
                let url = url.trim().to_string();
                let Some(parent) = parent else {
                    self.error("Choose a folder to clone into");
                    return;
                };
                if url.is_empty() {
                    return;
                }
                let dest = parent.join(git::repo_name(&url));
                let occupied = dest.exists() && dest.read_dir().map_or(true, |mut d| d.next().is_some());
                if occupied {
                    self.error(format!("{} already exists and isn't empty", dest.display()));
                    return;
                }
                self.git.clone_repo(url, dest, &self.ctx);
            }
            Dialog::EmptyTrash => {
                let Some(v) = &mut self.vault else { return };
                match v.empty_trash() {
                    Ok(()) => self.info("Trash emptied"),
                    Err(e) => self.error(format!("Could not empty the trash: {e}")),
                }
                self.refresh_trash();
            }
            Dialog::DeleteForever { name } => {
                let Some(v) = &mut self.vault else { return };
                if let Err(e) = v.delete_forever(&name) {
                    self.error(format!("Could not delete: {e}"));
                }
                self.refresh_trash();
            }
        }
    }

    fn on_close(&mut self) {
        self.save_now();
        let is_repo = self.git.status.as_ref().is_some_and(|s| s.is_repo);
        if !(self.s.sync_on_close && self.unsynced && is_repo && self.git.busy.is_none()) {
            return;
        }
        if let Some(v) = &self.vault {
            // Give the sync a bounded amount of time so closing never hangs for long.
            let root = v.root.clone();
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(git::sync(&root));
            });
            let _ = rx.recv_timeout(Duration::from_secs(20));
            self.unsynced = false;
        }
    }

    // ---------------------------------------------------------------------
    // Per-frame plumbing
    // ---------------------------------------------------------------------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let cmd_alt = Modifiers::COMMAND | Modifiers::ALT;
        let hit = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        let has_vault = self.vault.is_some();
        let editing = self.note.is_some() && self.page == Page::Notes && self.s.view != View::Preview;

        // More specific combinations first; consume_shortcut ignores extra Shift.
        if hit(cmd_shift, Key::F) && has_vault {
            self.actions.push(Action::Search);
        }
        if hit(cmd_shift, Key::S) {
            self.actions.push(Action::Sync);
        }
        if hit(cmd_shift, Key::N) && has_vault {
            let folder = self.current_folder();
            self.actions.push(Action::NewFolder(folder));
        }
        if hit(cmd_alt, Key::ArrowLeft) {
            self.actions.push(Action::Back);
        }
        if hit(cmd_alt, Key::ArrowRight) {
            self.actions.push(Action::Forward);
        }
        if (hit(cmd, Key::K) || hit(cmd, Key::P) || hit(cmd, Key::O)) && has_vault {
            self.actions.push(Action::Search);
        }
        if hit(cmd, Key::N) && has_vault {
            let folder = self.current_folder();
            self.actions.push(Action::NewNote(folder));
        }
        if hit(cmd, Key::S) {
            self.actions.push(Action::Save);
        }
        if hit(cmd, Key::E) && self.note.is_some() {
            self.actions.push(Action::ToggleView);
        }
        if hit(cmd, Key::Backslash) {
            self.s.show_sidebar = !self.s.show_sidebar;
        }
        if editing && self.search.is_none() && self.dialog.is_none() {
            if hit(cmd, Key::B) {
                self.actions.push(Action::Format(editor::Op::Bold));
            }
            if hit(cmd, Key::I) {
                self.actions.push(Action::Format(editor::Op::Italic));
            }
        }
    }

    fn timers(&mut self, ctx: &egui::Context) {
        if let Some(elapsed) = self.note.as_ref().and_then(|n| n.edited).map(|t| t.elapsed()) {
            if elapsed >= AUTOSAVE_DELAY {
                self.save_now();
            } else {
                ctx.request_repaint_after(AUTOSAVE_DELAY - elapsed);
            }
        }

        // Pick up edits made outside the app when the window regains focus.
        let focused = ctx.input(|i| i.focused);
        if focused && !self.focused && self.vault.is_some() {
            self.rescan();
            self.git.refresh(ctx);
        }
        self.focused = focused;

        let is_repo = self.git.status.as_ref().is_some_and(|s| s.is_repo);
        if is_repo && self.s.auto_sync_minutes > 0 {
            let due = self.last_sync + Duration::from_secs(u64::from(self.s.auto_sync_minutes) * 60);
            let now = Instant::now();
            if now >= due {
                if self.git.busy.is_none() {
                    self.sync();
                }
            } else {
                ctx.request_repaint_after(due - now);
            }
        }

        self.toasts.retain(|t| t.err || t.at.elapsed() < TOAST_TIME);
        if let Some(next) = self.toasts.iter().filter(|t| !t.err).map(|t| TOAST_TIME.saturating_sub(t.at.elapsed())).min() {
            ctx.request_repaint_after(next);
        }
        // Keeps "edited 2 min ago" and the sync time current.
        ctx.request_repaint_after(Duration::from_secs(30));
    }

    /// Images dropped onto the window go into the open note.
    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if dropped.is_empty() {
            return;
        }
        let images: Vec<PathBuf> = dropped.iter().filter(|p| attach::is_image(p)).cloned().collect();
        if images.is_empty() {
            self.error("Only images can be dropped into a note");
        } else {
            self.actions.push(Action::AddImages(Some(images)));
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match (&self.note, &self.vault) {
            (Some(n), _) if self.page == Page::Notes => format!("{} - Scrap", vault::stem(&n.rel)),
            (_, Some(v)) => format!("{} - Scrap", vault_name(v)),
            _ => "Scrap".to_string(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.dev_before(&ctx);
        self.poll_git();
        self.shortcuts(&ctx);
        self.timers(&ctx);
        self.handle_drops(&ctx);

        self.narrow = ctx.content_rect().width() < 720.0;
        if self.narrow && self.s.view == View::Split {
            self.s.view = View::Edit;
        }

        self.layout(ui);

        self.search_ui(&ctx);
        self.dialog_ui(&ctx);
        self.toasts_ui(&ctx);
        self.drop_overlay(&ctx);
        self.process_actions();
        self.update_title(&ctx);

        if ctx.input(|i| i.viewport().close_requested()) {
            self.on_close();
        }
        self.dev_after(&ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.s);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        let p = if visuals.dark_mode { theme::DARK } else { theme::LIGHT };
        p.bg.to_normalized_gamma_f32()
    }
}

pub(crate) fn vault_name(v: &Vault) -> String {
    v.root.file_name().map_or_else(|| v.root.display().to_string(), |n| n.to_string_lossy().into_owned())
}

fn v_is_note(rel: &str) -> bool {
    rel.to_ascii_lowercase().ends_with(".md")
}

fn normalize(raw: String) -> (String, bool) {
    if raw.contains("\r\n") {
        (raw.replace("\r\n", "\n"), true)
    } else {
        (raw, false)
    }
}
