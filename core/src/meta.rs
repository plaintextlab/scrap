//! Note properties kept in the note itself, in a YAML front matter block at the top:
//!
//! ```text
//! ---
//! pinned: true
//! color: blue
//! tags: [groceries, home]
//! ---
//! ```
//!
//! Obsidian and other editors read the same block, it syncs with the note and survives
//! renames and moves. Only simple `key: value` lines are written; other keys a note has
//! are kept as they are. Tags also come from `#hashtags` in the text.

/// The front matter's lines (without the `---` fences) and the byte where the body starts.
fn block(text: &str) -> Option<(&str, usize)> {
    let first = text.split_inclusive('\n').next()?;
    if first.trim_end() != "---" {
        return None;
    }
    let mut at = first.len();
    for line in text[at..].split_inclusive('\n') {
        if line.trim_end() == "---" || line.trim_end() == "..." {
            return Some((&text[first.len()..at], at + line.len()));
        }
        at += line.len();
    }
    None
}

/// The note without its front matter.
pub fn body(text: &str) -> &str {
    block(text).map_or(text, |(_, start)| &text[start..])
}

/// A front matter value, unquoted (`None` if the key isn't there or is empty).
pub fn get(text: &str, key: &str) -> Option<String> {
    let (lines, _) = block(text)?;
    lines.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        (k.trim() == key && !line.starts_with([' ', '\t'])).then(|| unquote(v.trim()).to_string())
    })
    .filter(|v| !v.is_empty())
}

fn unquote(v: &str) -> &str {
    v.strip_prefix('"').and_then(|v| v.strip_suffix('"'))
        .or_else(|| v.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
        .unwrap_or(v)
}

/// Sets `key` to `value`, or removes it (`None`). An emptied front matter is removed.
pub fn set(text: &str, key: &str, value: Option<&str>) -> String {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let (mut lines, body_start): (Vec<String>, usize) = match block(text) {
        Some((fm, start)) => (fm.lines().map(str::to_string).collect(), start),
        None => (Vec::new(), 0),
    };
    // Drop the key and any indented lines that belong to it (a YAML list).
    let mut out = Vec::with_capacity(lines.len() + 1);
    let mut skipping = false;
    let mut at = None;
    for line in lines.drain(..) {
        let indented = line.starts_with([' ', '\t']) || line.trim_start().starts_with("- ");
        if skipping && indented {
            continue;
        }
        skipping = false;
        if !indented && line.split_once(':').is_some_and(|(k, _)| k.trim() == key) {
            skipping = true;
            at.get_or_insert(out.len());
            continue;
        }
        out.push(line);
    }
    if let Some(v) = value {
        out.insert(at.unwrap_or(out.len()), format!("{key}: {v}"));
    }
    let body = &text[body_start..];
    if out.iter().all(|l| l.trim().is_empty()) {
        return body.to_string();
    }
    format!("---{nl}{}{nl}---{nl}{body}", out.join(nl))
}

pub fn pinned(text: &str) -> bool {
    get(text, "pinned").is_some_and(|v| v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
}

pub fn set_pinned(text: &str, pinned: bool) -> String {
    set(text, "pinned", pinned.then_some("true"))
}

/// The note's background colour: one of `colors::NAMES` (or whatever the file says).
pub fn color(text: &str) -> Option<String> {
    get(text, "color")
}

pub fn set_color(text: &str, color: Option<&str>) -> String {
    set(text, "color", color)
}

/// Tags written in the front matter (`tags: [a, b]`, `tags: a, b` or a YAML list).
pub fn front_tags(text: &str) -> Vec<String> {
    let Some((lines, _)) = block(text) else { return Vec::new() };
    let mut tags = Vec::new();
    let mut in_list = false;
    for line in lines.lines() {
        if in_list {
            match line.trim_start().strip_prefix("- ") {
                Some(item) if line.starts_with([' ', '\t', '-']) => {
                    tags.push(clean_tag(item));
                    continue;
                }
                _ => in_list = false,
            }
        }
        let Some((k, v)) = line.split_once(':') else { continue };
        if !matches!(k.trim(), "tags" | "tag") || line.starts_with([' ', '\t']) {
            continue;
        }
        let v = v.trim();
        if v.is_empty() {
            in_list = true;
            continue;
        }
        let v = v.trim_start_matches('[').trim_end_matches(']');
        tags.extend(v.split([',', ' ']).map(clean_tag));
    }
    tags.retain(|t| !t.is_empty());
    tags
}

fn clean_tag(t: &str) -> String {
    unquote(t.trim()).trim_start_matches('#').trim().to_string()
}

/// `#tags` in the note's text, outside code (headings' `# ` and URLs' `#anchors` aren't tags).
pub fn inline_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut in_fence = false;
    for line in body(text).lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut in_code = false;
        let mut prev = ' ';
        let chars = line.char_indices();
        for (i, c) in chars {
            if c == '`' {
                in_code = !in_code;
            } else if c == '#' && !in_code && (prev.is_whitespace() || prev == '(' || prev == ',') {
                let tag: String = line[i + 1..].chars().take_while(|&c| is_tag_char(c)).collect();
                let tag = tag.trim_end_matches(['/', '-']);
                if tag.chars().any(|c| !c.is_ascii_digit()) {
                    tags.push(tag.to_string());
                }
            }
            prev = c;
        }
    }
    tags
}

fn is_tag_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}

/// All of a note's tags, front matter first, without duplicates (ignoring case).
pub fn tags(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in front_tags(text).into_iter().chain(inline_tags(text)) {
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&t)) {
            out.push(t);
        }
    }
    out
}

/// Replaces the front matter tags (`#hashtags` in the text stay where they are).
pub fn set_tags(text: &str, tags: &[String]) -> String {
    let tags: Vec<String> = tags.iter().map(|t| clean_tag(t).replace(' ', "-")).filter(|t| !t.is_empty()).collect();
    let value = (!tags.is_empty()).then(|| format!("[{}]", tags.join(", ")));
    set(text, "tags", value.as_deref())
}

/// Whether the note has `tag`, or a tag nested under it (`home` matches `home/kitchen`).
pub fn has_tag(text: &str, tag: &str) -> bool {
    let tag = tag.trim().trim_start_matches('#');
    !tag.is_empty()
        && tags(text).iter().any(|t| {
            t.eq_ignore_ascii_case(tag)
                || t.len() > tag.len() && t[..tag.len()].eq_ignore_ascii_case(tag) && t.as_bytes()[tag.len()] == b'/'
        })
}

/// The tag a search query asks for: `#tag` or `tag:tag`.
pub fn tag_query(query: &str) -> Option<&str> {
    let q = query.trim();
    let tag = q.strip_prefix('#').or_else(|| q.strip_prefix("tag:"))?.trim();
    (!tag.is_empty() && !tag.contains(char::is_whitespace)).then_some(tag)
}

/// The note as plain text, for exporting as `.txt`: no front matter, colour tags or
/// formatting marks; links keep their text (and web links their address).
pub fn plain_text(text: &str) -> String {
    let text = crate::colors::strip(body(text));
    let mut out = String::with_capacity(text.len());
    let mut in_fence = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            out.push_str(line);
            continue;
        }
        let hashes = t.bytes().take_while(|&b| b == b'#').count();
        let line = if (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') { &t[hashes + 1..] } else { line };
        let line = t.strip_prefix("> ").unwrap_or(line);
        out.push_str(&plain_inline(line));
    }
    out
}

fn plain_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        // [[target|alias]] and [[target]]
        if let Some(inner) = rest.strip_prefix("[[").and_then(|r| r.split_once("]]")) {
            let (link, after) = inner;
            out.push_str(link.rsplit('|').next().unwrap_or(link));
            rest = after;
            continue;
        }
        // ![alt](url) and [text](url)
        let bang = rest.starts_with("![");
        if rest.starts_with('[') || bang {
            let open = usize::from(bang);
            if let Some(mid) = rest.find("](") {
                if let Some(end) = rest[mid..].find(')').map(|e| mid + e) {
                    let label = &rest[open + 1..mid];
                    let url = &rest[mid + 2..end];
                    if !label.contains('[') {
                        if bang {
                            out.push_str(&format!("[image: {}]", if label.is_empty() { url } else { label }));
                        } else if url.contains("://") && label != url {
                            out.push_str(&format!("{label} ({url})"));
                        } else {
                            out.push_str(label);
                        }
                        rest = &rest[end + 1..];
                        continue;
                    }
                }
            }
        }
        for marker in ["**", "__", "~~", "`"] {
            if let Some(r) = rest.strip_prefix(marker) {
                rest = r;
                break;
            }
        }
        if rest.len() < s.len() && !rest.starts_with(c) {
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_front_matter() {
        let plain = "# Shopping\n- [ ] milk\n";
        assert_eq!(get(plain, "color"), None);
        let pin = set_pinned(plain, true);
        assert_eq!(pin, "---\npinned: true\n---\n# Shopping\n- [ ] milk\n");
        assert!(pinned(&pin));
        let coloured = set_color(&pin, Some("blue"));
        assert_eq!(color(&coloured).as_deref(), Some("blue"));
        assert!(coloured.starts_with("---\npinned: true\ncolor: blue\n---\n"));
        // Removing every key removes the block.
        let back = set_color(&set_pinned(&coloured, false), None);
        assert_eq!(back, plain);
        // Other keys and their lists are kept; CRLF stays CRLF.
        let other = "---\r\naliases:\r\n  - x\r\ntitle: \"Hi\"\r\n---\r\nbody";
        let out = set_pinned(other, true);
        assert_eq!(out, "---\r\naliases:\r\n  - x\r\ntitle: \"Hi\"\r\npinned: true\r\n---\r\nbody");
        assert_eq!(get(other, "title").as_deref(), Some("Hi"));
        assert_eq!(body(other), "body");
        // A lone --- (a horizontal rule with no closing fence) isn't front matter.
        assert_eq!(get("---\nnot: closed\n", "not"), None);
    }

    #[test]
    fn finds_tags() {
        let text = "---\ntags: [groceries, \"home\"]\n---\n# Heading\nBuy #milk and #Groceries, \
                    not `#code` or https://x.y/#anchor or #123.\n```\n#fenced\n```\n(#paren) #home/kitchen.";
        assert_eq!(tags(text), ["groceries", "home", "milk", "paren", "home/kitchen"]);
        assert!(has_tag(text, "#MILK"));
        assert!(has_tag(text, "home"));
        assert!(has_tag("#home/kitchen", "home"));
        assert!(!has_tag(text, "kitchen"));
        assert!(!has_tag(text, "gro"));
        let list = "---\ntags:\n  - a\n  - b c\ntitle: x\n---\n";
        assert_eq!(front_tags(list), ["a", "b c"]);
        let set = set_tags(list, &["one".into(), "#two".into(), "three four".into()]);
        assert_eq!(set, "---\ntags: [one, two, three-four]\ntitle: x\n---\n");
        assert_eq!(set_tags(&set, &[]), "---\ntitle: x\n---\n");
        assert_eq!(tag_query(" #home "), Some("home"));
        assert_eq!(tag_query("tag:home"), Some("home"));
        assert_eq!(tag_query("home"), None);
        assert_eq!(tag_query("#"), None);
    }

    #[test]
    fn exports_plain_text() {
        let md = "---\ncolor: red\n---\n# Title\nSome **bold**, `code` and [[Other|alias]].\n\
                  > quoted [site](https://x.y) and [note](Note.md)\n![cat](cat.png)\n\
                  <span style=\"color: red\">red</span>\n- [ ] item\n```\n**kept**\n```\n";
        assert_eq!(
            plain_text(md),
            "Title\nSome bold, code and alias.\nquoted site (https://x.y) and note\n[image: cat]\nred\n- [ ] item\n**kept**\n"
        );
    }
}
