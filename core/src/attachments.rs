//! Images embedded in notes. They live in the vault's hidden `.attachments` folder, so
//! they sync with the notes but stay out of the note list, and notes link to them with
//! ordinary relative markdown image links (the Android app uses the same layout).

use std::fs;
use std::io;
use std::path::Path;

pub const DIR: &str = ".attachments";

/// Relative link from the note at `note_rel` to the vault path `path`.
pub fn link_from(note_rel: &str, path: &str) -> String {
    "../".repeat(note_rel.matches('/').count()) + path
}

/// Vault path a relative link in the note at `note_rel` points to. None for URLs,
/// absolute paths and links that leave the vault.
pub fn resolve(note_rel: &str, link: &str) -> Option<String> {
    let link = link.trim().trim_start_matches('<').trim_end_matches('>');
    if link.is_empty() || link.contains(':') || link.starts_with(['/', '#']) {
        return None;
    }
    let link = link.replace("%20", " ");
    let mut parts: Vec<&str> = note_rel.split('/').collect();
    parts.pop();
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

/// A free file name in the attachments folder: `IMG-<stamp>.<ext>`, numbered if taken.
pub fn free_name(vault: &Path, stamp: &str, ext: &str) -> io::Result<String> {
    let dir = vault.join(DIR);
    fs::create_dir_all(&dir)?;
    let mut name = format!("IMG-{stamp}.{ext}");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("IMG-{stamp}-{n}.{ext}");
        n += 1;
    }
    Ok(format!("{DIR}/{name}"))
}

/// A free path in the attachments folder for a file called `name` (numbered if taken):
/// attached files keep their own names, unlike images.
pub fn free_path_for(vault: &Path, name: &str) -> io::Result<String> {
    let dir = vault.join(DIR);
    fs::create_dir_all(&dir)?;
    let clean: String = name
        .chars()
        .map(|c| if c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .collect();
    let clean = clean.trim().trim_start_matches('.');
    let clean = if clean.is_empty() { "file" } else { clean };
    let (stem, ext) = match clean.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s, format!(".{e}")),
        _ => (clean, String::new()),
    };
    let mut file = clean.to_string();
    let mut n = 2;
    while dir.join(&file).exists() {
        file = format!("{stem} {n}{ext}");
        n += 1;
    }
    Ok(format!("{DIR}/{file}"))
}

/// A markdown link to an attached file: `[name](link)`, with spaces in the link encoded.
pub fn file_link(note_rel: &str, path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    let label = name.replace(['[', ']'], "");
    format!("[{label}]({})", link_from(note_rel, path).replace(' ', "%20"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_round_trip() {
        assert_eq!(link_from("Note.md", ".attachments/a.jpg"), ".attachments/a.jpg");
        assert_eq!(link_from("a/b/Note.md", ".attachments/a.jpg"), "../../.attachments/a.jpg");
        assert_eq!(resolve("a/b/Note.md", "../../.attachments/a.jpg").as_deref(), Some(".attachments/a.jpg"));
        assert_eq!(resolve("Note.md", "pics/My%20cat.png").as_deref(), Some("pics/My cat.png"));
        assert_eq!(resolve("Note.md", "https://x.y/a.png"), None);
        assert_eq!(resolve("Note.md", "../outside.png"), None);
    }

    #[test]
    fn attached_files_keep_their_names() {
        let vault = std::env::temp_dir().join(format!("scrap-attach-names-{}", std::process::id()));
        let _ = fs::remove_dir_all(&vault);
        let first = free_path_for(&vault, "Tax return.pdf").unwrap();
        assert_eq!(first, ".attachments/Tax return.pdf");
        fs::write(vault.join(&first), "x").unwrap();
        assert_eq!(free_path_for(&vault, "Tax return.pdf").unwrap(), ".attachments/Tax return 2.pdf");
        assert_eq!(free_path_for(&vault, "../a:b").unwrap(), ".attachments/_a_b");
        assert_eq!(file_link("sub/Note.md", &first), "[Tax return.pdf](../.attachments/Tax%20return.pdf)");
        let _ = fs::remove_dir_all(&vault);
    }
}
