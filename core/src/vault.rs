//! A vault is a plain folder of markdown files. Everything lives on disk;
//! this module keeps a small in-memory index for the tree, links and search.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Deleted notes are moved here instead of being destroyed.
pub const TRASH: &str = ".trash";

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

    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let q = query.trim().to_lowercase();
        let mut hits = Vec::new();
        if q.is_empty() {
            return hits;
        }
        for (i, n) in self.notes.iter().enumerate() {
            if n.name.to_lowercase().contains(&q) {
                hits.push(Hit { note: i, text: String::new() });
            }
            for line in n.content.lines() {
                if line.to_lowercase().contains(&q) {
                    hits.push(Hit { note: i, text: snippet(line.trim(), 140) });
                    if hits.len() >= limit {
                        return hits;
                    }
                }
            }
        }
        hits
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

    /// Moves a note or folder into the vault's `.trash` folder.
    pub fn delete(&mut self, rel: &str) -> io::Result<()> {
        let trash = self.root.join(TRASH);
        fs::create_dir_all(&trash)?;
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let mut dst = trash.join(name);
        let mut n = 1;
        while dst.exists() {
            dst = trash.join(format!("{n} {name}"));
            n += 1;
        }
        fs::rename(self.path(rel), dst)?;
        self.rescan();
        Ok(())
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
    }

    #[test]
    fn fuzzy_prefers_tight_matches() {
        assert!(fuzzy("projects/roadmap", "road").unwrap() > fuzzy("random/other/amp-d", "road").unwrap_or(-1000));
        assert!(fuzzy("notes", "xyz").is_none());
        assert_eq!(stem("a/B.MD"), "B");
    }
}
