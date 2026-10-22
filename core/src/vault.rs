//! A vault is a plain folder of markdown files. Everything lives on disk;
//! this module keeps a small in-memory index for the tree, links and search.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Deleted notes are moved here instead of being destroyed.
pub const TRASH: &str = ".trash";
/// Inside the trash: where each trashed item came from, one tab-separated line each.
const TRASH_INDEX: &str = ".origins";

#[derive(Clone, Debug)]
pub struct TrashItem {
    /// File or folder name inside the trash.
    pub name: String,
    /// Path it was deleted from, relative to the vault root.
    pub original: String,
    pub is_dir: bool,
    /// When it was deleted, in milliseconds since the Unix epoch (0 if unknown).
    pub deleted: u64,
}

pub struct Note {
    /// Path relative to the vault root, always with `/` separators.
    pub rel: String,
    /// File name without the `.md` extension.
    pub name: String,
    pub content: String,
    /// Targets of `[[wiki links]]` found in the content.
    pub links: Vec<String>,
    /// Last modification time in milliseconds since the Unix epoch (0 if unknown).
    pub modified: u64,
}

impl Note {
    pub fn pinned(&self) -> bool {
        crate::meta::pinned(&self.content)
    }

    /// Background colour name from the note's front matter.
    pub fn color(&self) -> Option<String> {
        crate::meta::color(&self.content)
    }

    pub fn tags(&self) -> Vec<String> {
        crate::meta::tags(&self.content)
    }
}

#[derive(Default)]
pub struct Dir {
    pub name: String,
    pub rel: String,
    pub dirs: Vec<Dir>,
    /// Indexes into `Vault::notes`.
    pub notes: Vec<usize>,
}

pub struct Hit {
    pub note: usize,
    /// Matching line, empty when only the note name matched.
    pub text: String,
}

pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
    pub tree: Dir,
    /// Bumped whenever the index changes, so callers can cache derived data.
    pub generation: u64,
    by_name: HashMap<String, Vec<usize>>,
    /// Lowercased name and content of each note, parallel to `notes`, so a search
    /// doesn't lowercase the whole vault on every keystroke.
    lower: Vec<(String, String)>,
}

impl Vault {
    pub fn open(root: PathBuf) -> io::Result<Self> {
        if !root.is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "folder does not exist"));
        }
        let mut vault = Vault {
            root,
            notes: Vec::new(),
            tree: Dir::default(),
            generation: 0,
            by_name: HashMap::new(),
            lower: Vec::new(),
        };
        vault.rescan();
        Ok(vault)
    }

    pub fn rescan(&mut self) {
        let mut notes = Vec::new();
        let mut tree = Dir::default();
        scan_dir(&self.root, "", &mut tree, &mut notes);
        self.notes = notes;
        self.tree = tree;
        self.by_name.clear();
        for (i, n) in self.notes.iter().enumerate() {
            self.by_name.entry(n.name.to_lowercase()).or_default().push(i);
        }
        self.lower = self.notes.iter().map(|n| (n.name.to_lowercase(), n.content.to_lowercase())).collect();
        self.generation += 1;
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    pub fn index_of(&self, rel: &str) -> Option<usize> {
        self.notes.iter().position(|n| n.rel == rel)
    }

    pub fn read(&self, rel: &str) -> io::Result<String> {
        fs::read(self.path(rel)).map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    pub fn save(&mut self, rel: &str, text: &str) -> io::Result<()> {
        fs::write(self.path(rel), text)?;
        match self.index_of(rel) {
            Some(i) => {
                let note = &mut self.notes[i];
                note.content = text.to_string();
                note.links = extract_links(text);
                note.modified = now_millis();
                self.lower[i].1 = text.to_lowercase();
                self.generation += 1;
            }
            None => self.rescan(),
        }
        Ok(())
    }

    /// Resolves a wiki-link target (`Note`, `folder/Note`, `Note#Heading`) to a note.
    pub fn resolve(&self, target: &str) -> Option<usize> {
        let target = target.split('#').next().unwrap_or("").trim();
        let target = target.strip_suffix(".md").unwrap_or(target);
        if target.is_empty() {
            return None;
        }
        let lower = target.to_lowercase().replace('\\', "/");
        if lower.contains('/') {
            let want = format!("{lower}.md");
            let suffix = format!("/{want}");
            return self
                .notes
                .iter()
                .enumerate()
                .filter(|(_, n)| {
                    let rel = n.rel.to_lowercase();
                    rel == want || rel.ends_with(&suffix)
                })
                .min_by_key(|(_, n)| n.rel.len())
                .map(|(i, _)| i);
        }
        // Several notes may share a name; prefer the one closest to the root.
        self.by_name
            .get(&lower)?
            .iter()
            .copied()
            .min_by_key(|&i| self.notes[i].rel.len())
    }

    pub fn backlinks(&self, idx: usize) -> Vec<usize> {
        (0..self.notes.len())
            .filter(|&j| j != idx && self.notes[j].links.iter().any(|l| self.resolve(l) == Some(idx)))
            .collect()
    }

    /// Notes whose name or text contain `query` (ignoring case), a line per match. A query
    /// of `#tag` or `tag:tag` instead finds the notes with that tag, pinned ones first.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        if let Some(tag) = crate::meta::tag_query(query) {
            return self.tagged(tag, limit);
        }
        let q = query.trim().to_lowercase();
        let mut hits = Vec::new();
        if q.is_empty() {
            return hits;
        }
        for (i, (n, (name, content))) in self.notes.iter().zip(&self.lower).enumerate() {
            if name.contains(&q) {
                hits.push(Hit { note: i, text: String::new() });
            }
            // Most notes don't match at all; only walk the lines of those that do.
            // Lowercasing never adds or removes line breaks, so the lines stay aligned.
            if !content.contains(&q) {
                continue;
            }
            for (line, lower) in n.content.lines().zip(content.lines()) {
                if let Some(at) = lower.find(&q) {
                    // Character position of the match (lowercasing rarely changes it).
                    let at = lower[..at].chars().count();
                    hits.push(Hit { note: i, text: snippet_around(line, at, 140) });
                    if hits.len() >= limit {
                        return hits;
                    }
                }
            }
        }
        hits
    }

    /// Notes with `tag` (or a tag nested under it), pinned first, then most recent.
    fn tagged(&self, tag: &str, limit: usize) -> Vec<Hit> {
        let mut found: Vec<usize> = (0..self.notes.len()).filter(|&i| crate::meta::has_tag(&self.notes[i].content, tag)).collect();
        found.sort_by_key(|&i| (!self.notes[i].pinned(), std::cmp::Reverse(self.notes[i].modified)));
        found.truncate(limit);
        found
            .into_iter()
            .map(|i| {
                let tags = self.notes[i].tags().iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join("  ");
                Hit { note: i, text: tags }
            })
            .collect()
    }

    /// Every tag in the vault with how many notes have it, most used first.
    pub fn all_tags(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for n in &self.notes {
            for t in n.tags() {
                match counts.iter_mut().find(|(c, _)| c.eq_ignore_ascii_case(&t)) {
                    Some(entry) => entry.1 += 1,
                    None => counts.push((t, 1)),
                }
            }
        }
        counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
        counts
    }

    pub fn create_note(&mut self, folder: &str, name: &str) -> io::Result<String> {
        let base = sanitize(name.strip_suffix(".md").unwrap_or(name));
        let base = if base.is_empty() { "Untitled".to_string() } else { base };
        if !folder.is_empty() {
            fs::create_dir_all(self.path(folder))?;
        }
        let rel = self.unique(folder, &base, ".md");
        fs::write(self.path(&rel), "")?;
        self.rescan();
        Ok(rel)
    }

    pub fn create_folder(&mut self, parent: &str, name: &str) -> io::Result<String> {
        let base = sanitize(name);
        let base = if base.is_empty() { "New folder".to_string() } else { base };
        let rel = self.unique(parent, &base, "");
        fs::create_dir_all(self.path(&rel))?;
        // Git doesn't track empty folders; this keeps a new folder in sync before it has notes.
        fs::write(self.path(&rel).join(".gitkeep"), "")?;
        self.rescan();
        Ok(rel)
    }

    /// Renames a note or folder. Renaming a note also rewrites `[[links]]` to it.
    pub fn rename(&mut self, rel: &str, new_name: &str) -> io::Result<String> {
        let src = self.path(rel);
        let is_dir = src.is_dir();
        let base = sanitize(new_name.strip_suffix(".md").unwrap_or(new_name));
        if base.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "name is empty"));
        }
        let file = if is_dir { base.clone() } else { format!("{base}.md") };
        let new_rel = join(parent(rel), &file);
        if new_rel == rel {
            return Ok(new_rel);
        }
        // A case-only rename on a case-insensitive file system sees itself as existing.
        if self.path(&new_rel).exists() && new_rel.to_lowercase() != rel.to_lowercase() {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("\"{file}\" already exists")));
        }
        fs::rename(&src, self.path(&new_rel))?;
        if !is_dir {
            let old = stem(rel);
            if old != base {
                for n in self.notes.iter().filter(|n| n.rel != rel) {
                    let updated = replace_link_target(&n.content, &old, &base);
                    if updated != n.content {
                        fs::write(self.path(&n.rel), updated)?;
                    }
                }
            }
        }
        self.rescan();
        Ok(new_rel)
    }

    /// Moves a note or folder into `folder` ("" for the top level), keeping its name (or
    /// numbering it if taken). Relative links in the moved notes, such as embedded images,
    /// are rewritten so they still point at the same files. Returns the new path.
    pub fn move_to(&mut self, rel: &str, folder: &str) -> io::Result<String> {
        let src = self.path(rel);
        if !src.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("\"{rel}\" doesn't exist")));
        }
        if !folder.is_empty() && !self.path(folder).is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("folder \"{folder}\" doesn't exist")));
        }
        let is_dir = src.is_dir();
        if is_dir && (folder == rel || folder.starts_with(&format!("{rel}/"))) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "a folder can't be moved into itself"));
        }
        if parent(rel) == folder {
            return Ok(rel.to_string());
        }
        let file = rel.rsplit('/').next().unwrap_or(rel);
        let new_rel = match file.strip_suffix(".md") {
            Some(base) if !is_dir => self.unique(folder, base, ".md"),
            _ => self.unique(folder, file, ""),
        };

        let prefix = format!("{rel}/");
        let moved: Vec<(String, String)> = self
            .notes
            .iter()
            .filter(|n| n.rel == rel || n.rel.starts_with(&prefix))
            .map(|n| (n.rel.clone(), n.content.clone()))
            .collect();
        fs::rename(&src, self.path(&new_rel))?;

        // Where something at `old` (vault-relative) lives after the move.
        let relocate = |old: &str| -> String {
            if old == rel {
                new_rel.clone()
            } else if let Some(rest) = old.strip_prefix(&prefix) {
                format!("{new_rel}/{rest}")
            } else {
                old.to_string()
            }
        };
        for (old_note, content) in moved {
            let new_note = relocate(&old_note);
            let updated = rewrite_relative_links(&content, parent(&old_note), parent(&new_note), &relocate);
            if updated != content {
                fs::write(self.path(&new_note), updated)?;
            }
        }
        self.rescan();
        Ok(new_rel)
    }

    /// Makes a copy of a note next to it, named "<name> copy" (numbered if taken).
    pub fn duplicate(&mut self, rel: &str) -> io::Result<String> {
        let base = format!("{} copy", stem(rel));
        let new_rel = self.unique(parent(rel), &base, ".md");
        fs::write(self.path(&new_rel), self.read(rel)?)?;
        self.rescan();
        Ok(new_rel)
    }

    /// Copies a note into `folder` ("" for the top level), keeping its name (numbered if
    /// taken); relative links, such as images, are rewritten to still point at their files.
    pub fn copy_to(&mut self, rel: &str, folder: &str) -> io::Result<String> {
        if !folder.is_empty() && !self.path(folder).is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("folder \"{folder}\" doesn't exist")));
        }
        let content = self.read(rel)?;
        let new_rel = self.unique(folder, &stem(rel), ".md");
        let same = |p: &str| p.to_string();
        fs::write(self.path(&new_rel), rewrite_relative_links(&content, parent(rel), folder, &same))?;
        self.rescan();
        Ok(new_rel)
    }

    /// Moves a note or folder into the vault's `.trash` folder, remembering where it came
    /// from so it can be restored.
    pub fn delete(&mut self, rel: &str) -> io::Result<()> {
        let trash = self.root.join(TRASH);
        fs::create_dir_all(&trash)?;
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let mut trashed = name.to_string();
        let mut n = 1;
        while trash.join(&trashed).exists() {
            trashed = format!("{n} {name}");
            n += 1;
        }
        fs::rename(self.path(rel), trash.join(&trashed))?;
        let mut origins = self.trash_origins();
        origins.retain(|o| o.0 != trashed);
        origins.push((trashed, rel.to_string(), now_millis()));
        self.write_trash_origins(&origins)?;
        self.rescan();
        Ok(())
    }

    /// Everything in the trash, most recently deleted first.
    pub fn trash(&self) -> Vec<TrashItem> {
        let origins = self.trash_origins();
        let Ok(read) = fs::read_dir(self.root.join(TRASH)) else { return Vec::new() };
        let mut items: Vec<TrashItem> = read
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    return None;
                }
                let is_dir = e.file_type().ok()?.is_dir();
                let known = origins.iter().find(|o| o.0 == name);
                let deleted = known.map(|o| o.2).unwrap_or_else(|| {
                    e.metadata().ok().and_then(|m| m.modified().ok()).map_or(0, millis)
                });
                // Items trashed before origins were recorded go back to the vault root.
                let original = known.map(|o| o.1.clone()).unwrap_or_else(|| name.clone());
                Some(TrashItem { name, original, is_dir, deleted })
            })
            .collect();
        items.sort_by(|a, b| b.deleted.cmp(&a.deleted).then_with(|| a.name.cmp(&b.name)));
        items
    }

    /// Moves a trashed item back to where it was deleted from (recreating missing folders).
    /// If that name is taken it gets a numbered name. Returns its new path.
    pub fn restore(&mut self, name: &str) -> io::Result<String> {
        let src = self.trash_path(name)?;
        let original = self.trash().into_iter().find(|t| t.name == name).map(|t| t.original);
        let original = original.unwrap_or_else(|| name.to_string());
        let folder = parent(&original).to_string();
        let file = original.rsplit('/').next().unwrap_or(&original);
        let rel = if self.path(&original).exists() {
            match file.strip_suffix(".md") {
                Some(stem) if !src.is_dir() => self.unique(&folder, stem, ".md"),
                _ => self.unique(&folder, file, ""),
            }
        } else {
            original.clone()
        };
        if !folder.is_empty() {
            fs::create_dir_all(self.path(&folder))?;
        }
        fs::rename(&src, self.path(&rel))?;
        self.forget_trashed(name)?;
        self.rescan();
        Ok(rel)
    }

    /// Permanently deletes one trashed item.
    pub fn delete_forever(&mut self, name: &str) -> io::Result<()> {
        let path = self.trash_path(name)?;
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
        self.forget_trashed(name)
    }

    /// Permanently deletes everything in the trash.
    pub fn empty_trash(&mut self) -> io::Result<()> {
        let trash = self.root.join(TRASH);
        if trash.exists() {
            fs::remove_dir_all(&trash)?;
        }
        Ok(())
    }

    /// Path of a top-level trash entry; refuses names that would escape the trash.
    fn trash_path(&self, name: &str) -> io::Result<PathBuf> {
        if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." || name.starts_with('.') {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not in the trash"));
        }
        let path = self.root.join(TRASH).join(name);
        if !path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("\"{name}\" is no longer in the trash")));
        }
        Ok(path)
    }

    fn forget_trashed(&self, name: &str) -> io::Result<()> {
        let mut origins = self.trash_origins();
        origins.retain(|o| o.0 != name);
        self.write_trash_origins(&origins)
    }

    /// (name in trash, original path, deleted at) for each trashed item.
    fn trash_origins(&self) -> Vec<(String, String, u64)> {
        let text = fs::read_to_string(self.root.join(TRASH).join(TRASH_INDEX)).unwrap_or_default();
        text.lines()
            .filter_map(|l| {
                let mut parts = l.splitn(3, '\t');
                let name = parts.next()?.to_string();
                let rel = parts.next()?.to_string();
                let at = parts.next()?.parse().ok()?;
                Some((name, rel, at))
            })
            .collect()
    }

    fn write_trash_origins(&self, origins: &[(String, String, u64)]) -> io::Result<()> {
        let text: String = origins.iter().map(|(n, r, t)| format!("{n}\t{r}\t{t}\n")).collect();
        fs::write(self.root.join(TRASH).join(TRASH_INDEX), text)
    }

    fn unique(&self, folder: &str, base: &str, ext: &str) -> String {
        let mut n = 0;
        loop {
            let name = if n == 0 { format!("{base}{ext}") } else { format!("{base} {n}{ext}") };
            let rel = join(folder, &name);
            if !self.path(&rel).exists() {
                return rel;
            }
            n += 1;
        }
    }
}

/// Contents of the note created in a brand-new vault.
pub const WELCOME: &str = "# Welcome to Scrap\n\nNotes are plain **markdown** files. Link notes with [[Ideas]], \
then open the link to create that note.\n\n- [ ] Write something\n";

/// Creates the vault folder with a welcome note the first time it is used.
pub fn seed(root: &Path) -> io::Result<()> {
    fs::create_dir_all(root)?;
    if fs::read_dir(root)?.next().is_none() {
        fs::write(root.join("Welcome.md"), WELCOME)?;
    }
    Ok(())
}

fn scan_dir(abs: &Path, rel: &str, dir: &mut Dir, notes: &mut Vec<Note>) {
    let Ok(read) = fs::read_dir(abs) else { return };
    let mut entries: Vec<_> = read.flatten().collect();
    entries.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        // Skips .git, .trash, .obsidian and other hidden things.
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else { continue };
        let child_rel = join(rel, &name);
        if kind.is_dir() {
            let mut child = Dir { name, rel: child_rel.clone(), ..Default::default() };
            scan_dir(&entry.path(), &child_rel, &mut child, notes);
            dir.dirs.push(child);
        } else if kind.is_file() && is_markdown(&name) {
            let content = fs::read(entry.path())
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .unwrap_or_default();
            dir.notes.push(notes.len());
            let modified = entry.metadata().ok().and_then(|m| m.modified().ok()).map_or(0, millis);
            notes.push(Note { links: extract_links(&content), name: stem(&name), rel: child_rel, content, modified });
        }
    }
}

/// Wiki-link targets in `src`, skipping fenced code blocks.
pub fn extract_links(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in src.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("[[") {
            rest = &rest[start + 2..];
            let Some(end) = rest.find("]]") else { break };
            let target = rest[..end].split(['|', '#']).next().unwrap_or("").trim();
            if !target.is_empty() {
                out.push(target.to_string());
            }
            rest = &rest[end + 2..];
        }
    }
    out
}

/// Rewrites `[[old]]`, `[[old|alias]]` and `[[old#heading]]` to point at `new`.
fn replace_link_target(src: &str, old: &str, new: &str) -> String {
    let old = old.to_lowercase();
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(start) = rest.find("[[") {
        out.push_str(&rest[..start + 2]);
        rest = &rest[start + 2..];
        let Some(end) = rest.find("]]") else { break };
        let inner = &rest[..end];
        let cut = inner.find(['|', '#']).unwrap_or(inner.len());
        if !inner.contains('\n') && inner[..cut].trim().to_lowercase() == old {
            out.push_str(new);
            out.push_str(&inner[cut..]);
            rest = &rest[end..];
        }
    }
    out.push_str(rest);
    out
}

/// Subsequence match used by the quick switcher. Higher is better.
pub fn fuzzy(hay: &str, needle: &str) -> Option<i32> {
    let needle = needle.trim().to_lowercase();
    if needle.is_empty() {
        return Some(0);
    }
    let hay = hay.to_lowercase();
    let chars: Vec<char> = hay.chars().collect();
    let mut score = 0;
    let mut i = 0;
    let mut prev: Option<usize> = None;
    for nc in needle.chars() {
        while i < chars.len() && chars[i] != nc {
            i += 1;
        }
        if i == chars.len() {
            return None;
        }
        score += 10;
        if i > 0 && prev == Some(i - 1) {
            score += 15;
        }
        if i == 0 || matches!(chars[i - 1], '/' | ' ' | '-' | '_') {
            score += 10;
        }
        prev = Some(i);
        i += 1;
    }
    if hay.contains(&needle) {
        score += 50;
    }
    Some(score - chars.len() as i32 / 4)
}

fn millis(t: std::time::SystemTime) -> u64 {
    t.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

fn now_millis() -> u64 {
    millis(std::time::SystemTime::now())
}

pub fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() { name.to_string() } else { format!("{folder}/{name}") }
}

pub fn parent(rel: &str) -> &str {
    rel.rsplit_once('/').map_or("", |(p, _)| p)
}

pub fn stem(rel: &str) -> String {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    match file.len().checked_sub(3) {
        Some(cut) if file.is_char_boundary(cut) && file[cut..].eq_ignore_ascii_case(".md") => file[..cut].to_string(),
        _ => file.to_string(),
    }
}

fn is_markdown(name: &str) -> bool {
    name.len() > 3 && name.is_char_boundary(name.len() - 3) && name[name.len() - 3..].eq_ignore_ascii_case(".md")
}

/// Rewrites relative markdown link and image targets (`](path)`) in a note that moved
/// from folder `old_dir` to `new_dir`, so they keep pointing at the same files. `relocate`
/// maps a vault path to where it is now (for links to things that moved along with it).
/// URLs, anchors, absolute paths and links inside fenced code are left alone.
fn rewrite_relative_links(src: &str, old_dir: &str, new_dir: &str, relocate: &dyn Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_fence = false;
    for line in src.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
        }
        if in_fence || !line.contains("](") {
            out.push_str(line);
            continue;
        }
        let mut rest = line;
        while let Some(i) = rest.find("](") {
            out.push_str(&rest[..i + 2]);
            rest = &rest[i + 2..];
            let end = rest.find(|c: char| c == ')' || c.is_whitespace()).unwrap_or(rest.len());
            let target = &rest[..end];
            let skip = target.is_empty() || target.contains(':') || target.starts_with(['#', '/', '<']);
            match (skip, resolve_relative(old_dir, target)) {
                (false, Some(abs)) => out.push_str(&relative_path(new_dir, &relocate(&abs))),
                _ => out.push_str(target),
            }
            rest = &rest[end..];
        }
        out.push_str(rest);
    }
    out
}

/// `dir` joined with relative `link`, with `.` and `..` resolved. None if it leaves the vault.
fn resolve_relative(dir: &str, link: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in link.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

/// Relative link from folder `from` to vault path `to`.
fn relative_path(from: &str, to: &str) -> String {
    let from: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let common = from.iter().zip(&to_parts).take_while(|(a, b)| a == b).count();
    let mut out = "../".repeat(from.len() - common);
    out.push_str(&to_parts[common..].join("/"));
    out
}

/// Strips characters that are invalid in file names on any of the target OSes.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '[' | ']' | '#' | '^'))
        .collect();
    cleaned.trim().trim_end_matches('.').trim().to_string()
}

fn snippet(line: &str, max: usize) -> String {
    if line.chars().count() <= max {
        line.to_string()
    } else {
        line.chars().take(max).collect::<String>() + "…"
    }
}

/// Up to `max` characters of `line`, starting a little before the character at `at` so
/// that a match far into a long line is still visible.
fn snippet_around(line: &str, at: usize, max: usize) -> String {
    let lead = line.chars().take_while(|c| c.is_whitespace()).count();
    let start = at.saturating_sub(40).max(lead);
    if start == lead {
        return snippet(line.trim(), max);
    }
    let rest: String = line.chars().skip(start).collect();
    "…".to_string() + &snippet(rest.trim_end(), max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scrap-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn links_are_extracted_outside_code() {
        let src = "See [[Alpha]] and [[folder/Beta|b]] and [[Gamma#Intro]].\n```\n[[NotALink]]\n```\n";
        assert_eq!(extract_links(src), ["Alpha", "folder/Beta", "Gamma"]);
    }

    #[test]
    fn rename_rewrites_links_and_resolves() {
        let root = temp_dir("rename");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("Old.md"), "# Old").unwrap();
        fs::write(root.join("sub/Other.md"), "[[Old]] [[old|alias]] [[Old#h]] [[Older]]").unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/ignored.md"), "").unwrap();

        let mut v = Vault::open(root.clone()).unwrap();
        assert_eq!(v.notes.len(), 2, "hidden folders are skipped");
        let other = v.resolve("Other").unwrap();
        assert_eq!(v.notes[other].rel, "sub/Other.md");
        assert_eq!(v.resolve("sub/other"), Some(other));
        assert_eq!(v.backlinks(v.resolve("Old").unwrap()), vec![other]);

        let new_rel = v.rename("Old.md", "New").unwrap();
        assert_eq!(new_rel, "New.md");
        let content = fs::read_to_string(root.join("sub/Other.md")).unwrap();
        assert_eq!(content, "[[New]] [[New|alias]] [[New#h]] [[Older]]");
        assert!(v.resolve("Old").is_none());
        assert!(v.resolve("New").is_some());
    }

    #[test]
    fn create_and_delete() {
        let root = temp_dir("create");
        let mut v = Vault::open(root.clone()).unwrap();
        let a = v.create_note("", "Idea: one?").unwrap();
        assert_eq!(a, "Idea one.md");
        let b = v.create_note("", "Idea one").unwrap();
        assert_eq!(b, "Idea one 1.md");
        let c = v.create_note("deep/er", "").unwrap();
        assert_eq!(c, "deep/er/Untitled.md");
        v.delete(&a).unwrap();
        assert!(root.join(TRASH).join("Idea one.md").exists());
        assert_eq!(v.notes.len(), 2);
        assert_eq!(v.search("untitled", 10).len(), 1);

        let outer = v.create_folder("", "Projects").unwrap();
        let inner = v.create_folder(&outer, "2026").unwrap();
        assert_eq!(inner, "Projects/2026");
        assert!(root.join("Projects/2026/.gitkeep").exists());
        assert_eq!(v.tree.dirs.iter().find(|d| d.rel == outer).unwrap().dirs[0].rel, inner);
    }

    #[test]
    fn duplicates_copies_and_finds_tags() {
        let root = temp_dir("copy");
        fs::create_dir_all(root.join("A")).unwrap();
        fs::write(root.join("List.md"), "---\npinned: true\n---\n![](.attachments/p.jpg) #home\n").unwrap();
        fs::write(root.join("Other.md"), "#home/kitchen and #work").unwrap();
        let mut v = Vault::open(root.clone()).unwrap();
        assert_eq!(v.duplicate("List.md").unwrap(), "List copy.md");
        assert_eq!(v.duplicate("List.md").unwrap(), "List copy 1.md");
        let copied = v.copy_to("List.md", "A").unwrap();
        assert_eq!(copied, "A/List.md");
        let text = fs::read_to_string(root.join(&copied)).unwrap();
        assert!(text.contains("![](../.attachments/p.jpg)"), "{text}");
        assert!(root.join("List.md").exists(), "the original stays");

        let hits = v.search("#home", 10);
        let names: Vec<&str> = hits.iter().map(|h| v.notes[h.note].rel.as_str()).collect();
        assert_eq!(names.len(), 5);
        assert_eq!(names.last(), Some(&"Other.md"), "pinned notes first: {names:?}");
        assert_eq!(v.search("tag:work", 10).len(), 1);
        assert_eq!(v.all_tags()[0], ("home".to_string(), 4));
    }

    #[test]
    fn move_keeps_relative_links_working() {
        let root = temp_dir("move");
        fs::create_dir_all(root.join(".attachments")).unwrap();
        fs::write(root.join(".attachments/p.jpg"), "img").unwrap();
        fs::write(
            root.join("Trip.md"),
            "![](.attachments/p.jpg) [web](https://x.y/a) [[Other]] [top](#h)\n```\n![](.attachments/p.jpg)\n```\n",
        )
        .unwrap();
        let mut v = Vault::open(root.clone()).unwrap();
        let a = v.create_folder("", "A").unwrap();
        let b = v.create_folder(&a, "B").unwrap();

        // Note one level down: image link gains "../", everything else untouched.
        let moved = v.move_to("Trip.md", &b).unwrap();
        assert_eq!(moved, "A/B/Trip.md");
        let text = fs::read_to_string(root.join(&moved)).unwrap();
        assert!(text.starts_with("![](../../.attachments/p.jpg) [web](https://x.y/a) [[Other]] [top](#h)"), "{text}");
        assert!(text.contains("```\n![](.attachments/p.jpg)\n```"), "code is left alone: {text}");

        // Moving the whole folder back up keeps links valid too.
        let moved_dir = v.move_to(&b, "").unwrap();
        assert_eq!(moved_dir, "B");
        let text = fs::read_to_string(root.join("B/Trip.md")).unwrap();
        assert!(text.starts_with("![](../.attachments/p.jpg)"), "{text}");

        // Name clashes get numbered; folders can't go inside themselves.
        fs::write(root.join("Trip.md"), "").unwrap();
        v.rescan();
        assert_eq!(v.move_to("B/Trip.md", "").unwrap(), "Trip 1.md");
        assert!(v.move_to(&a, &a).is_err());
        assert_eq!(v.move_to("Trip.md", "").unwrap(), "Trip.md", "same folder is a no-op");
    }

    #[test]
    fn trash_restores_to_original_place() {
        let root = temp_dir("trash");
        let mut v = Vault::open(root.clone()).unwrap();
        let folder = v.create_folder("", "Work").unwrap();
        let note = v.create_note(&folder, "Plan").unwrap();
        v.save(&note, "the plan").unwrap();
        v.delete(&note).unwrap();
        v.delete(&folder).unwrap();
        assert!(v.notes.is_empty());

        let items = v.trash();
        assert_eq!(items.len(), 2);
        let plan = items.iter().find(|t| t.name == "Plan.md").unwrap();
        assert_eq!(plan.original, "Work/Plan.md");
        assert!(!plan.is_dir);

        // The folder is gone, so restoring the note recreates it.
        assert_eq!(v.restore("Plan.md").unwrap(), "Work/Plan.md");
        assert_eq!(fs::read_to_string(root.join("Work/Plan.md")).unwrap(), "the plan");
        // The folder's name is taken now, so it comes back numbered.
        assert_eq!(v.restore("Work").unwrap(), "Work 1");
        assert!(v.trash().is_empty());

        // Same name deleted twice: both are kept and restored.
        let a = v.create_note("", "Dup").unwrap();
        v.delete(&a).unwrap();
        let b = v.create_note("", "Dup").unwrap();
        v.delete(&b).unwrap();
        assert_eq!(v.trash().len(), 2);
        v.delete_forever("1 Dup.md").unwrap();
        assert_eq!(v.trash().len(), 1);
        assert!(v.restore("../escape").is_err());
        v.empty_trash().unwrap();
        assert!(v.trash().is_empty());
    }

    #[test]
    fn search_finds_names_and_lines() {
        let root = temp_dir("search");
        fs::write(root.join("Ünïcode Plans.md"), "first line\n  Second LINE mentions ÄPFEL\r\nthird").unwrap();
        fs::write(root.join("Other.md"), "nothing here").unwrap();
        let long = format!("{} needle at the end", "word ".repeat(60));
        fs::write(root.join("Long.md"), &long).unwrap();
        let mut v = Vault::open(root.clone()).unwrap();

        let hits = v.search("äpfel", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "Second LINE mentions ÄPFEL");
        assert_eq!(v.search("ünïcode", 10).iter().filter(|h| h.text.is_empty()).count(), 1);
        assert!(v.search("missing", 10).is_empty());

        let hit = &v.search("NEEDLE", 10)[0];
        assert!(hit.text.starts_with('…') && hit.text.contains("needle at the end"), "{}", hit.text);

        // Saving keeps the search index current.
        v.save("Other.md", "now with apples").unwrap();
        assert_eq!(v.search("APPLES", 10).len(), 1);
    }

    #[test]
    fn fuzzy_prefers_tight_matches() {
        assert!(fuzzy("projects/roadmap", "road").unwrap() > fuzzy("random/other/amp-d", "road").unwrap_or(-1000));
        assert!(fuzzy("notes", "xyz").is_none());
        assert_eq!(stem("a/B.MD"), "B");
    }
}
