use crate::git::{self, Git};
use crate::markdown;
use crate::vault::{self, Vault};
use eframe::egui::{self, Align, Align2, Key, KeyboardShortcut, Layout, Modifiers, RichText, Ui};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const AUTOSAVE_DELAY: Duration = Duration::from_millis(800);
const READABLE_WIDTH: f32 = 760.0;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Edit,
    Preview,
    Split,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub vault: Option<PathBuf>,
    pub recent: Vec<PathBuf>,
    pub last_note: Option<String>,
    pub dark: bool,
    pub font_size: f32,
    pub view: View,
    pub show_sidebar: bool,
    pub show_links: bool,
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
            dark: true,
            font_size: 15.0,
            view: View::Edit,
            show_sidebar: true,
            show_links: true,
            auto_sync_minutes: 10,
            sync_on_open: true,
            sync_on_close: true,
        }
    }
}

struct OpenNote {
    rel: String,
    text: String,
    saved: String,
    /// The file used CRLF line endings; they are restored on save.
    crlf: bool,
    edited: Option<Instant>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SideTab {
    Files,
    Search,
}

enum Dialog {
    NewNote { folder: String, name: String },
    NewFolder { parent: String, name: String },
    Rename { rel: String, name: String },
    Delete { rel: String },
    Remote { url: String },
    Clone { url: String, parent: Option<PathBuf> },
}

enum Action {
    Open(String),
    Link(String),
    NewNote(String),
    NewFolder(String),
    Rename(String),
    Delete(String),
    Save,
    Sync,
    Rescan,
    OpenVault,
    Back,
    Forward,
    Switcher,
    Search,
    ToggleView,
}

#[derive(Default)]
struct Switcher {
    query: String,
    selected: usize,
}

struct Flash {
    text: String,
    err: bool,
    at: Instant,
}

pub struct App {
    ctx: egui::Context,
    s: Settings,
    vault: Option<Vault>,
    note: Option<OpenNote>,
    history: Vec<String>,
    hist_pos: usize,
    side: SideTab,
    search: String,
    hits: Vec<vault::Hit>,
    hits_key: (String, u64),
    backlinks: Vec<usize>,
    backlinks_key: (String, u64),
    switcher: Option<Switcher>,
    dialog: Option<Dialog>,
    show_settings: bool,
    git: Git,
    last_sync: Instant,
    unsynced: bool,
    sync_after_status: bool,
    flash: Option<Flash>,
    focused: bool,
    focus_editor: bool,
    focus_search: bool,
    title: String,
    actions: Vec<Action>,
    /// Phone-sized window: the file list replaces the note instead of sitting beside it.
    narrow: bool,
}

impl App {
    /// `default_vault` is opened (and created) when no vault has been chosen yet;
    /// on Android, where there is no folder picker, that is the only vault.
    pub fn new(cc: &eframe::CreationContext<'_>, default_vault: Option<PathBuf>) -> Self {
        let s: Settings = cc
            .storage
            .and_then(|st| eframe::get_value(st, eframe::APP_KEY))
            .unwrap_or_default();
        cc.egui_ctx.set_theme(if s.dark { egui::Theme::Dark } else { egui::Theme::Light });
        let mut app = App {
            ctx: cc.egui_ctx.clone(),
            s,
            vault: None,
            note: None,
            history: Vec::new(),
            hist_pos: 0,
            side: SideTab::Files,
            search: String::new(),
            hits: Vec::new(),
            hits_key: (String::new(), 0),
            backlinks: Vec::new(),
            backlinks_key: (String::new(), 0),
            switcher: None,
            dialog: None,
            show_settings: false,
            git: Git::new(),
            last_sync: Instant::now(),
            unsynced: false,
            sync_after_status: false,
            flash: None,
            focused: true,
            focus_editor: false,
            focus_search: false,
            title: String::new(),
            actions: Vec::new(),
            narrow: false,
        };
        let vault = app.s.vault.clone().or_else(|| default_vault.and_then(|p| seed_vault(p).ok()));
        if let Some(path) = vault {
            app.open_vault(path);
        }
        app
    }

    // ---------------------------------------------------------------------
    // State changes
    // ---------------------------------------------------------------------

    fn info(&mut self, text: impl Into<String>) {
        self.flash = Some(Flash { text: text.into(), err: false, at: Instant::now() });
    }

    fn error(&mut self, text: impl Into<String>) {
        self.flash = Some(Flash { text: text.into(), err: true, at: Instant::now() });
    }

    fn open_vault(&mut self, path: PathBuf) {
        self.save_now();
        match Vault::open(path.clone()) {
            Ok(v) => {
                self.vault = Some(v);
                self.note = None;
                self.history.clear();
                self.hist_pos = 0;
                self.s.recent.retain(|p| p != &path);
                self.s.recent.insert(0, path.clone());
                self.s.recent.truncate(8);
                self.s.vault = Some(path.clone());
                self.git.set_root(Some(path), &self.ctx);
                self.sync_after_status = self.s.sync_on_open;
                self.last_sync = Instant::now();
                self.unsynced = false;
                let last = self.s.last_note.clone();
                if let Some(rel) = last {
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

    fn open(&mut self, rel: &str, push: bool) {
        self.save_now();
        let Some(v) = &self.vault else { return };
        match v.read(rel) {
            Ok(raw) => {
                let (text, crlf) = normalize(raw);
                self.note = Some(OpenNote { rel: rel.to_string(), saved: text.clone(), text, crlf, edited: None });
                if push {
                    if !self.history.is_empty() {
                        self.history.truncate(self.hist_pos + 1);
                    }
                    if self.history.last().map(String::as_str) != Some(rel) {
                        self.history.push(rel.to_string());
                    }
                    self.hist_pos = self.history.len() - 1;
                }
                self.s.last_note = Some(rel.to_string());
                self.focus_editor = true;
            }
            Err(e) => self.error(format!("Could not open {rel}: {e}")),
        }
    }

    fn save_now(&mut self) {
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

    fn rescan(&mut self) {
        self.save_now();
        if let Some(v) = &mut self.vault {
            v.rescan();
        }
        self.reload_current();
    }

    fn sync(&mut self) {
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
                self.show_settings = true;
                self.info("This vault isn't a git repository yet. Set it up under Git sync.");
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
                Ok(msg) => self.info(msg),
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
        let folder = self.note.as_ref().map(|n| vault::parent(&n.rel).to_string()).unwrap_or_default();
        let joined = vault::join(&folder, path);
        let found = self.vault.as_ref().and_then(|v| {
            v.index_of(&joined).or_else(|| v.index_of(path)).map(|i| v.notes[i].rel.clone())
        });
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
            Ok(rel) => self.open(&rel, true),
            Err(e) => self.error(format!("Could not create note: {e}")),
        }
    }

    fn current_folder(&self) -> String {
        self.note.as_ref().map(|n| vault::parent(&n.rel).to_string()).unwrap_or_default()
    }

    fn process_actions(&mut self) {
        for action in std::mem::take(&mut self.actions) {
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
                Action::Save => {
                    self.save_now();
                    self.info("Saved");
                }
                Action::Sync => self.sync(),
                Action::Rescan => self.rescan(),
                Action::OpenVault => {
                    #[cfg(not(target_os = "android"))]
                    if let Some(path) = rfd::FileDialog::new().set_title("Open folder as vault").pick_folder() {
                        self.open_vault(path);
                    }
                }
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
                Action::Switcher => {
                    if self.vault.is_some() {
                        self.switcher = Some(Switcher::default());
                    }
                }
                Action::Search => {
                    self.s.show_sidebar = true;
                    self.side = SideTab::Search;
                    self.focus_search = true;
                }
                Action::ToggleView => {
                    self.s.view = if self.s.view == View::Edit { View::Preview } else { View::Edit };
                    self.focus_editor = self.s.view == View::Edit;
                }
            }
        }
    }

    fn submit(&mut self, dialog: Dialog) {
        match dialog {
            Dialog::NewNote { folder, name } => {
                let Some(v) = &mut self.vault else { return };
                match v.create_note(&folder, &name) {
                    Ok(rel) => self.open(&rel, true),
                    Err(e) => self.error(format!("Could not create note: {e}")),
                }
            }
            Dialog::NewFolder { parent, name } => {
                let Some(v) = &mut self.vault else { return };
                if let Err(e) = v.create_folder(&parent, &name) {
                    self.error(format!("Could not create folder: {e}"));
                }
            }
            Dialog::Rename { rel, name } => {
                self.save_now();
                let Some(v) = &mut self.vault else { return };
                match v.rename(&rel, &name) {
                    Ok(new_rel) => {
                        let prefix = format!("{rel}/");
                        let fix = |p: &mut String| {
                            if *p == rel {
                                *p = new_rel.clone();
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
                        // Other notes may have had their links rewritten.
                        self.reload_current();
                    }
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
                        self.history.retain(|p| *p != rel && !p.starts_with(&prefix));
                        self.hist_pos = self.history.len().saturating_sub(1);
                        self.info(format!("Moved {rel} to {}", vault::TRASH));
                    }
                    Err(e) => self.error(format!("Could not delete: {e}")),
                }
            }
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

        // More specific combinations first; consume_shortcut ignores extra Shift.
        if hit(cmd_shift, Key::F) {
            self.actions.push(Action::Search);
        }
        if hit(cmd_shift, Key::S) {
            self.actions.push(Action::Sync);
        }
        if hit(cmd_alt, Key::ArrowLeft) {
            self.actions.push(Action::Back);
        }
        if hit(cmd_alt, Key::ArrowRight) {
            self.actions.push(Action::Forward);
        }
        if hit(cmd, Key::O) || hit(cmd, Key::P) {
            self.actions.push(Action::Switcher);
        }
        if hit(cmd, Key::N) && self.vault.is_some() {
            let folder = self.current_folder();
            self.actions.push(Action::NewNote(folder));
        }
        if hit(cmd, Key::S) {
            self.actions.push(Action::Save);
        }
        if hit(cmd, Key::E) && self.note.is_some() {
            self.actions.push(Action::ToggleView);
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

        if let Some(f) = &self.flash {
            if !f.err {
                let shown = f.at.elapsed();
                if shown > Duration::from_secs(6) {
                    self.flash = None;
                } else {
                    ctx.request_repaint_after(Duration::from_secs(6) - shown);
                }
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match (&self.note, &self.vault) {
            (Some(n), _) => format!("{} - Scrap", vault::stem(&n.rel)),
            (None, Some(v)) => format!(
                "{} - Scrap",
                v.root.file_name().map_or_else(|| v.root.display().to_string(), |n| n.to_string_lossy().into_owned())
            ),
            _ => "Scrap".to_string(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    // ---------------------------------------------------------------------
    // Panels
    // ---------------------------------------------------------------------

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("☰").on_hover_text("Toggle sidebar").clicked() {
                    self.s.show_sidebar = !self.s.show_sidebar;
                }
                let back = ui.add_enabled(self.hist_pos > 0, egui::Button::new("◀"));
                if back.on_hover_text("Back (Ctrl+Alt+Left)").clicked() {
                    self.actions.push(Action::Back);
                }
                let fwd = ui.add_enabled(self.hist_pos + 1 < self.history.len(), egui::Button::new("▶"));
                if fwd.on_hover_text("Forward (Ctrl+Alt+Right)").clicked() {
                    self.actions.push(Action::Forward);
                }
                if self.vault.is_some() && ui.button("🔍").on_hover_text("Quick open (Ctrl+O)").clicked() {
                    self.actions.push(Action::Switcher);
                }
                if let Some(n) = &self.note {
                    ui.separator();
                    ui.label(RichText::new(vault::stem(&n.rel)).strong());
                    let folder = vault::parent(&n.rel);
                    if !folder.is_empty() {
                        ui.label(RichText::new(folder).weak());
                    }
                    if n.text != n.saved {
                        ui.label(RichText::new("●").weak()).on_hover_text("Unsaved changes");
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    if self.vault.is_some() && self.git.available {
                        let label = if self.git.busy.is_some() { "⏳ Syncing…" } else { "🔄 Sync" };
                        let btn = ui.add_enabled(self.git.busy.is_none(), egui::Button::new(label));
                        if btn.on_hover_text("Commit, pull and push (Ctrl+Shift+S)").clicked() {
                            self.actions.push(Action::Sync);
                        }
                    }
                    if self.note.is_some() {
                        ui.separator();
                        if !self.narrow {
                            ui.selectable_value(&mut self.s.view, View::Split, "Split");
                        }
                        ui.selectable_value(&mut self.s.view, View::Preview, "Read");
                        ui.selectable_value(&mut self.s.view, View::Edit, "Edit");
                    }
                });
            });
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let git_text = if !self.git.available {
                    "git not found".to_string()
                } else {
                    match &self.git.status {
                        _ if self.vault.is_none() => String::new(),
                        None => "…".to_string(),
                        Some(st) if !st.is_repo => "Not a git repo".to_string(),
                        Some(st) => {
                            let mut t = st.branch.clone();
                            if st.remote.is_none() {
                                t += " · no remote";
                            }
                            if st.changes > 0 {
                                t += &format!(" · {} changed", st.changes);
                            }
                            if st.ahead > 0 {
                                t += &format!(" · ↑{}", st.ahead);
                            }
                            if st.behind > 0 {
                                t += &format!(" · ↓{}", st.behind);
                            }
                            t
                        }
                    }
                };
                if !git_text.is_empty() {
                    ui.label(RichText::new(git_text).weak());
                }

                let mut dismiss = false;
                if let Some(f) = &self.flash {
                    ui.separator();
                    let first = f.text.lines().next().unwrap_or("");
                    let color = if f.err { ui.visuals().error_fg_color } else { ui.visuals().weak_text_color() };
                    let label = ui.add(egui::Label::new(RichText::new(first).color(color)).sense(egui::Sense::click()));
                    if label.on_hover_text(format!("{}\n\n(click to dismiss)", f.text)).clicked() {
                        dismiss = true;
                    }
                }
                if dismiss {
                    self.flash = None;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(n) = &self.note {
                        let words = n.text.split_whitespace().count();
                        ui.label(RichText::new(format!("{words} words")).weak());
                    }
                });
            });
        });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(250.0)
            .width_range(160.0..=500.0)
            .show(ctx, |ui| self.side_contents(ui));
    }

    fn side_contents(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.side, SideTab::Files, "📁 Files");
            ui.selectable_value(&mut self.side, SideTab::Search, "🔍 Search");
        });
        ui.separator();
        match self.side {
            SideTab::Files => self.files_ui(ui),
            SideTab::Search => self.search_ui(ui),
        }
    }

    fn files_ui(&mut self, ui: &mut Ui) {
        let Some(v) = &self.vault else { return };
        let mut actions = Vec::new();
        let folder = self.note.as_ref().map(|n| vault::parent(&n.rel).to_string()).unwrap_or_default();
        ui.horizontal(|ui| {
            if ui.small_button("➕ Note").on_hover_text("New note (Ctrl+N)").clicked() {
                actions.push(Action::NewNote(folder));
            }
            if ui.small_button("➕ Folder").clicked() {
                actions.push(Action::NewFolder(String::new()));
            }
            if ui.small_button("⟲").on_hover_text("Reload from disk").clicked() {
                actions.push(Action::Rescan);
            }
        });
        ui.add_space(4.0);
        let current = self.note.as_ref().map(|n| n.rel.as_str());
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            dir_ui(ui, &v.tree, v, current, &mut actions);
            if v.notes.is_empty() {
                ui.label(RichText::new("No notes yet").weak());
            }
        });
        self.actions.extend(actions);
    }

    fn search_ui(&mut self, ui: &mut Ui) {
        let edit = ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search notes…")
                .desired_width(f32::INFINITY),
        );
        if self.focus_search {
            edit.request_focus();
            self.focus_search = false;
        }
        let Some(v) = &self.vault else { return };
        let key = (self.search.clone(), v.generation);
        if self.hits_key != key {
            self.hits = v.search(&self.search, 300);
            self.hits_key = key;
        }
        if self.search.trim().is_empty() {
            return;
        }
        ui.label(RichText::new(format!("{} results", self.hits.len())).weak().small());
        ui.add_space(4.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for hit in &self.hits {
                let note = &v.notes[hit.note];
                ui.push_id((hit.note, &hit.text), |ui| {
                    if ui.selectable_label(false, RichText::new(&note.name).strong()).clicked() {
                        self.actions.push(Action::Open(note.rel.clone()));
                    }
                    if !hit.text.is_empty() {
                        ui.label(RichText::new(&hit.text).weak().small());
                    }
                });
                ui.add_space(4.0);
            }
        });
    }

    fn links_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("links")
            .resizable(true)
            .default_width(210.0)
            .width_range(150.0..=400.0)
            .show(ctx, |ui| {
                let (Some(v), Some(n)) = (&self.vault, &self.note) else { return };
                let key = (n.rel.clone(), v.generation);
                if self.backlinks_key != key {
                    self.backlinks = v.index_of(&n.rel).map(|i| v.backlinks(i)).unwrap_or_default();
                    self.backlinks_key = key;
                }
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.label(RichText::new("Backlinks").strong());
                    if self.backlinks.is_empty() {
                        ui.label(RichText::new("None").weak());
                    }
                    for &i in &self.backlinks {
                        if ui.link(&v.notes[i].name).on_hover_text(&v.notes[i].rel).clicked() {
                            self.actions.push(Action::Open(v.notes[i].rel.clone()));
                        }
                    }

                    ui.add_space(12.0);
                    ui.label(RichText::new("Outgoing links").strong());
                    let mut links = vault::extract_links(&n.text);
                    links.dedup();
                    if links.is_empty() {
                        ui.label(RichText::new("None").weak());
                    }
                    for link in links {
                        let exists = v.resolve(&link).is_some();
                        let text = if exists { RichText::new(&link) } else { RichText::new(&link).weak().italics() };
                        let hint = if exists { "Open" } else { "Not created yet; click to create" };
                        if ui.link(text).on_hover_text(hint).clicked() {
                            self.actions.push(Action::Link(format!("wiki:{link}")));
                        }
                    }
                });
            });
    }

    fn central(&mut self, ui: &mut Ui) {
        if self.vault.is_none() {
            self.welcome_ui(ui);
            return;
        }
        if self.note.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.3);
                ui.label(RichText::new("No note open").size(20.0).weak());
                ui.add_space(12.0);
                if ui.button("Quick open (Ctrl+O)").clicked() {
                    self.actions.push(Action::Switcher);
                }
                if ui.button("New note (Ctrl+N)").clicked() {
                    self.actions.push(Action::NewNote(String::new()));
                }
            });
            return;
        }
        match self.s.view {
            View::Edit => self.editor_ui(ui),
            View::Preview => self.preview_ui(ui),
            View::Split => {
                ui.columns(2, |cols| {
                    self.editor_ui(&mut cols[0]);
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
        egui::ScrollArea::vertical()
            .id_salt("editor")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                centered(ui, |ui| {
                    let mut layouter = |ui: &Ui, text: &str, wrap_width: f32| {
                        let mut job = markdown::highlight(text, ui.visuals(), size);
                        job.wrap.max_width = wrap_width;
                        ui.fonts(|f| f.layout_job(job))
                    };
                    let out = egui::TextEdit::multiline(&mut n.text)
                        .id(egui::Id::new(("editor", &n.rel)))
                        .frame(false)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .min_size(egui::vec2(0.0, ui.available_height()))
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
                            follow = markdown::link_at(&n.text, range.primary.ccursor.index);
                        }
                    }
                });
            });
        if let Some(url) = follow {
            self.actions.push(Action::Link(url));
        }
    }

    fn preview_ui(&mut self, ui: &mut Ui) {
        let Some(n) = &self.note else { return };
        let size = self.s.font_size;
        let mut clicked = None;
        egui::ScrollArea::vertical()
            .id_salt("preview")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                centered(ui, |ui| {
                    ui.add_space(4.0);
                    clicked = markdown::preview(ui, &n.text, size);
                    ui.add_space(40.0);
                });
            });
        if let Some(url) = clicked {
            self.actions.push(Action::Link(url));
        }
    }

    fn welcome_ui(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.2);
            ui.label(RichText::new("Scrap").size(34.0).strong());
            ui.label(RichText::new("Plain markdown notes, synced with git.").weak());
            ui.add_space(24.0);
            if ui.button("📂  Open folder as vault…").clicked() {
                self.actions.push(Action::OpenVault);
            }
            ui.add_space(4.0);
            let clone = ui.add_enabled(self.git.available, egui::Button::new("⬇  Clone vault from git…"));
            if clone.clicked() {
                self.dialog = Some(Dialog::Clone { url: String::new(), parent: None });
            }
            if self.git.busy == Some("clone") {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Cloning…");
                });
            }
            if !self.s.recent.is_empty() {
                ui.add_space(24.0);
                ui.label(RichText::new("Recent").weak());
                for path in self.s.recent.clone() {
                    if ui.link(path.display().to_string()).clicked() {
                        self.open_vault(path);
                    }
                }
            }
        });
    }

    // ---------------------------------------------------------------------
    // Floating windows
    // ---------------------------------------------------------------------

    fn switcher_ui(&mut self, ctx: &egui::Context) {
        let Some(v) = &self.vault else { return };
        let Some(sw) = &mut self.switcher else { return };

        let mut results: Vec<(i32, usize)> = v
            .notes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| vault::fuzzy(n.rel.trim_end_matches(".md"), &sw.query).map(|s| (s, i)))
            .collect();
        results.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| v.notes[a.1].rel.cmp(&v.notes[b.1].rel)));
        results.truncate(100);

        let key = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
        let down = key(Key::ArrowDown);
        let up = key(Key::ArrowUp);
        if down {
            sw.selected = (sw.selected + 1).min(results.len().saturating_sub(1));
        }
        if up {
            sw.selected = sw.selected.saturating_sub(1);
        }
        let enter = key(Key::Enter);
        let escape = key(Key::Escape);
        sw.selected = sw.selected.min(results.len().saturating_sub(1));

        let mut picked: Option<usize> = None;
        egui::Window::new("Quick open")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_TOP, [0.0, 60.0])
            .show(ctx, |ui| {
                ui.set_width(480.0);
                ui.add(
                    egui::TextEdit::singleline(&mut sw.query)
                        .hint_text("Find or create a note…")
                        .desired_width(f32::INFINITY),
                )
                .request_focus();
                ui.separator();
                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for (k, &(_, i)) in results.iter().enumerate() {
                        let note = &v.notes[i];
                        let folder = vault::parent(&note.rel);
                        let mut job = egui::text::LayoutJob::default();
                        RichText::new(&note.name).append_to(&mut job, ui.style(), egui::FontSelection::Default, Align::Center);
                        if !folder.is_empty() {
                            RichText::new(format!("   {folder}")).weak().small().append_to(
                                &mut job,
                                ui.style(),
                                egui::FontSelection::Default,
                                Align::Center,
                            );
                        }
                        let row = ui.selectable_label(k == sw.selected, job);
                        if k == sw.selected && (up || down) {
                            row.scroll_to_me(None);
                        }
                        if row.clicked() {
                            picked = Some(i);
                        }
                    }
                    if !sw.query.trim().is_empty() {
                        ui.label(RichText::new(format!("Shift+Enter: create \"{}\"", sw.query.trim())).weak().small());
                    }
                });
            });

        let shift = ctx.input(|i| i.modifiers.shift);
        let mut create = None;
        if enter {
            if shift || results.is_empty() {
                create = Some(sw.query.trim().to_string()).filter(|q| !q.is_empty());
            } else {
                picked = results.get(sw.selected).map(|r| r.1);
            }
        }
        let open = picked.map(|i| v.notes[i].rel.clone());
        if escape || open.is_some() || create.is_some() {
            self.switcher = None;
            self.focus_editor = true;
        }
        if let Some(rel) = open {
            self.actions.push(Action::Open(rel));
        }
        if let Some(name) = create {
            self.actions.push(Action::Link(format!("wiki:{name}")));
        }
    }

    fn dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.dialog else { return };
        let title = match dialog {
            Dialog::NewNote { .. } => "New note",
            Dialog::NewFolder { .. } => "New folder",
            Dialog::Rename { .. } => "Rename",
            Dialog::Delete { .. } => "Delete",
            Dialog::Remote { .. } => "Git remote",
            Dialog::Clone { .. } => "Clone vault",
        };
        let ok_label = match dialog {
            Dialog::Delete { .. } => "Move to trash",
            Dialog::Clone { .. } => "Clone",
            Dialog::Rename { .. } => "Rename",
            Dialog::Remote { .. } => "Save",
            _ => "Create",
        };
        let mut submit = false;
        let mut close = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_TOP, [0.0, 120.0])
            .show(ctx, |ui| {
                ui.set_width(380.0);
                let field = |ui: &mut Ui, value: &mut String, hint: &str| {
                    let r = ui.add(egui::TextEdit::singleline(value).hint_text(hint).desired_width(f32::INFINITY));
                    let entered = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    r.request_focus();
                    entered
                };
                match dialog {
                    Dialog::NewNote { folder, name } => {
                        if !folder.is_empty() {
                            ui.label(RichText::new(format!("In {folder}/")).weak());
                        }
                        submit |= field(ui, name, "Untitled");
                    }
                    Dialog::NewFolder { name, .. } => submit |= field(ui, name, "Folder name"),
                    Dialog::Rename { name, .. } => submit |= field(ui, name, "New name"),
                    Dialog::Delete { rel } => {
                        ui.label(format!("Move \"{rel}\" to the vault's {} folder?", vault::TRASH));
                    }
                    Dialog::Remote { url } => {
                        ui.label("Remote repository URL (SSH or HTTPS):");
                        submit |= field(ui, url, "git@github.com:you/notes.git");
                        ui.label(
                            RichText::new("Uses your existing git credentials (SSH keys / credential manager).")
                                .weak()
                                .small(),
                        );
                    }
                    Dialog::Clone { url, parent } => {
                        ui.label("Repository URL:");
                        field(ui, url, "git@github.com:you/notes.git");
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.button("Choose folder…").clicked() {
                                #[cfg(not(target_os = "android"))]
                                if let Some(p) = rfd::FileDialog::new().set_title("Clone into…").pick_folder() {
                                    *parent = Some(p);
                                }
                            }
                            match parent {
                                Some(p) => ui.label(p.join(git::repo_name(url)).display().to_string()),
                                None => ui.label(RichText::new("No folder chosen").weak()),
                            };
                        });
                    }
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(ok_label).clicked() {
                        submit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });

        if submit {
            if let Some(d) = self.dialog.take() {
                self.submit(d);
            }
            self.focus_editor = true;
        } else if close {
            self.dialog = None;
            self.focus_editor = true;
        }
    }

    fn settings_ui(&mut self, ctx: &egui::Context) {
        let mut open = self.show_settings;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::RIGHT_TOP, [-12.0, 48.0])
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.label(RichText::new("Appearance").strong());
                if ui.checkbox(&mut self.s.dark, "Dark theme").changed() {
                    ctx.set_theme(if self.s.dark { egui::Theme::Dark } else { egui::Theme::Light });
                }
                ui.add(egui::Slider::new(&mut self.s.font_size, 11.0..=24.0).text("Editor font size"));
                ui.checkbox(&mut self.s.show_links, "Show links panel");

                ui.separator();
                ui.label(RichText::new("Vault").strong());
                if let Some(v) = &self.vault {
                    ui.label(RichText::new(v.root.display().to_string()).weak());
                }
                if ui.button("Open another folder…").clicked() {
                    self.actions.push(Action::OpenVault);
                }

                ui.separator();
                ui.label(RichText::new("Git sync").strong());
                if !self.git.available {
                    let msg = if cfg!(target_os = "android") {
                        "Sync isn't available on Android yet."
                    } else {
                        "git wasn't found on PATH. Install git to enable sync."
                    };
                    ui.colored_label(ui.visuals().warn_fg_color, msg);
                    return;
                }
                if self.vault.is_none() {
                    return;
                }
                match &self.git.status {
                    None => {
                        ui.spinner();
                    }
                    Some(st) if !st.is_repo => {
                        ui.label("This vault isn't a git repository.");
                        if ui.add_enabled(self.git.busy.is_none(), egui::Button::new("Initialize repository")).clicked() {
                            self.git.init(ctx);
                        }
                    }
                    Some(st) => {
                        let remote = st.remote.clone();
                        ui.label(format!("Branch: {}", st.branch));
                        ui.horizontal(|ui| {
                            ui.label("Remote:");
                            ui.label(RichText::new(remote.as_deref().unwrap_or("none")).weak());
                        });
                        let label = if remote.is_some() { "Change remote…" } else { "Add remote…" };
                        if ui.button(label).clicked() {
                            self.dialog = Some(Dialog::Remote { url: remote.unwrap_or_default() });
                        }
                        ui.add_space(6.0);
                        ui.add(egui::Slider::new(&mut self.s.auto_sync_minutes, 0..=120).text("Auto-sync (minutes, 0 = off)"));
                        ui.checkbox(&mut self.s.sync_on_open, "Sync when the vault opens");
                        ui.checkbox(&mut self.s.sync_on_close, "Sync when closing the app");
                    }
                }
            });
        self.show_settings = open;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_git();
        self.shortcuts(ctx);
        self.timers(ctx);

        self.narrow = ctx.screen_rect().width() < 700.0;
        if self.narrow && self.s.view == View::Split {
            self.s.view = View::Edit;
        }
        self.top_bar(ctx);
        self.status_bar(ctx);
        let files_only = self.narrow && self.vault.is_some() && self.s.show_sidebar;
        if !files_only {
            if self.vault.is_some() && self.s.show_sidebar {
                self.sidebar(ctx);
            }
            if self.note.is_some() && self.s.show_links && !self.narrow {
                self.links_panel(ctx);
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if files_only {
                self.side_contents(ui);
            } else {
                self.central(ui);
            }
        });

        self.switcher_ui(ctx);
        self.dialog_ui(ctx);
        self.settings_ui(ctx);
        self.process_actions();
        self.update_title(ctx);

        if ctx.input(|i| i.viewport().close_requested()) {
            self.on_close();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.s);
    }
}

fn dir_ui(ui: &mut Ui, dir: &vault::Dir, v: &Vault, current: Option<&str>, actions: &mut Vec<Action>) {
    for d in &dir.dirs {
        let contains_current = current.is_some_and(|c| c.starts_with(&format!("{}/", d.rel)));
        let resp = egui::CollapsingHeader::new(format!("📁 {}", d.name))
            .id_salt(&d.rel)
            .default_open(contains_current)
            .show(ui, |ui| dir_ui(ui, d, v, current, actions));
        resp.header_response.context_menu(|ui| {
            if ui.button("New note here").clicked() {
                actions.push(Action::NewNote(d.rel.clone()));
                ui.close_menu();
            }
            if ui.button("New folder here").clicked() {
                actions.push(Action::NewFolder(d.rel.clone()));
                ui.close_menu();
            }
            if ui.button("Rename…").clicked() {
                actions.push(Action::Rename(d.rel.clone()));
                ui.close_menu();
            }
            if ui.button("Delete…").clicked() {
                actions.push(Action::Delete(d.rel.clone()));
                ui.close_menu();
            }
        });
    }
    for &i in &dir.notes {
        let note = &v.notes[i];
        let row = ui.selectable_label(current == Some(note.rel.as_str()), &note.name);
        if row.clicked() {
            actions.push(Action::Open(note.rel.clone()));
        }
        row.context_menu(|ui| {
            if ui.button("Rename…").clicked() {
                actions.push(Action::Rename(note.rel.clone()));
                ui.close_menu();
            }
            if ui.button("Delete…").clicked() {
                actions.push(Action::Delete(note.rel.clone()));
                ui.close_menu();
            }
        });
    }
}

/// Lays content out in a centered column of readable width.
fn centered<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
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

/// Creates the vault folder with a welcome note the first time it is used.
fn seed_vault(path: PathBuf) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(&path)?;
    if std::fs::read_dir(&path)?.next().is_none() {
        std::fs::write(
            path.join("Welcome.md"),
            "# Welcome to Scrap

Notes are plain **markdown** files. Link notes with [[Ideas]],              then tap *Read* to follow links.

- [ ] Write something
",
        )?;
    }
    Ok(path)
}

fn normalize(raw: String) -> (String, bool) {
    if raw.contains("\r\n") {
        (raw.replace("\r\n", "\n"), true)
    } else {
        (raw, false)
    }
}
