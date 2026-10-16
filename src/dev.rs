//! Screenshots for development and docs. With `SCRAP_SCREENSHOT=out.png` the app sets
//! itself up as `SCRAP_DEMO` describes, saves a screenshot of its window and quits.
//!
//! `SCRAP_DEMO` is a `;`-separated list of: `open:<note path>`, `view:edit|read|split`,
//! `search:<query>`, `page:trash|settings`, `theme:light|dark`, `move:<path>`,
//! `delete:<path>`, `links`, `nosidebar`, `expand:<folder>`, `select:<from>-<to>`,
//! `fmt:bold|italic|heading|list|check|undo|redo`, `addimage:<file>`, `save`.

use crate::app::{Action, App, Dialog, Page, SearchState, ThemeChoice, View};
use crate::editor::Op;
use eframe::egui::{self, text::CCursor, text::CCursorRange, text_edit::TextEditState, Event, UserData};
use std::path::PathBuf;

pub struct Dev {
    out: PathBuf,
    script: Vec<String>,
    frame: u32,
    requested: bool,
}

/// Frames to wait for layout to settle and images to load before the screenshot.
const SETTLE_FRAMES: u32 = 45;

impl Dev {
    pub fn from_env() -> Option<Self> {
        let out = std::env::var_os("SCRAP_SCREENSHOT")?;
        let script = std::env::var("SCRAP_DEMO")
            .unwrap_or_default()
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        Some(Dev { out: PathBuf::from(out), script, frame: 0, requested: false })
    }
}

impl App {
    pub(crate) fn dev_before(&mut self, ctx: &egui::Context) {
        let Some(dev) = &mut self.dev else { return };
        dev.frame += 1;
        ctx.request_repaint();
        let frame = dev.frame;
        let out = dev.out.clone();
        if frame == 3 {
            let script = dev.script.clone();
            for cmd in script {
                self.dev_command(ctx, &cmd);
            }
        }
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            let [w, h] = image.size;
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b(), 255]).collect();
            if let Err(e) = image::save_buffer(&out, &rgba, w as u32, h as u32, image::ExtendedColorType::Rgba8) {
                eprintln!("screenshot failed: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub(crate) fn dev_after(&mut self, ctx: &egui::Context) {
        // Wait for an update check started by the script to finish.
        let waiting = self.updater.busy();
        let Some(dev) = &mut self.dev else { return };
        if dev.frame >= SETTLE_FRAMES && !dev.requested && !waiting {
            dev.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(UserData::default()));
        }
    }

    fn dev_command(&mut self, ctx: &egui::Context, cmd: &str) {
        let (name, arg) = cmd.split_once(':').unwrap_or((cmd, ""));
        match name {
            "open" => self.open(arg, true),
            "view" => {
                self.s.view = match arg {
                    "read" => View::Preview,
                    "split" => View::Split,
                    _ => View::Edit,
                }
            }
            "search" => self.search = Some(SearchState { query: arg.to_string(), selected: 0 }),
            "page" => self.run(Action::Page(if arg == "trash" { Page::Trash } else { Page::Settings })),
            "checkupdate" => self.updater.check(ctx),
            "palette" => {
                if let Some(t) = crate::theme::ThemeName::ALL.into_iter().find(|t| t.id() == arg) {
                    self.s.color_theme = t;
                    crate::theme::apply(&self.ctx, t);
                }
            }
            "theme" => {
                self.s.theme = if arg == "dark" { ThemeChoice::Dark } else { ThemeChoice::Light };
                self.apply_theme();
            }
            "move" => self.dialog = Some(Dialog::Move { rel: arg.to_string() }),
            "delete" => self.submit(Dialog::Delete { rel: arg.to_string() }),
            "links" => self.s.show_links = true,
            "nosidebar" => self.s.show_sidebar = false,
            "expand" => {
                self.s.expanded.insert(arg.to_string());
            }
            "select" => {
                // Selects characters in the editor, e.g. to show the toolbar acting on them.
                let Some((a, b)) = arg.split_once('-') else { return };
                let (Ok(a), Ok(b)) = (a.parse::<usize>(), b.parse::<usize>()) else { return };
                if let Some(n) = &self.note {
                    let id = egui::Id::new(("editor", n.rel.as_str()));
                    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
                    state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
                    state.store(ctx, id);
                }
            }
            "fmt" => {
                let op = match arg {
                    "bold" => Op::Bold,
                    "italic" => Op::Italic,
                    "heading" => Op::Heading,
                    "list" => Op::List,
                    "check" => Op::Checklist,
                    "undo" => Op::Undo,
                    "redo" => Op::Redo,
                    _ => return,
                };
                self.format(op);
            }
            "addimage" => self.run(Action::AddImages(Some(vec![PathBuf::from(arg)]))),
            "save" => self.save_now(),
            _ => eprintln!("unknown SCRAP_DEMO command: {cmd}"),
        }
    }
}
