//! Helpers for turning `[[wiki links]]` into something markdown renderers understand.

/// How the `wiki:` link destination is written.
#[derive(Clone, Copy)]
pub enum LinkStyle {
    /// `[label](<wiki:My Note>)`: CommonMark angle brackets, used by pulldown-cmark.
    AngleBrackets,
    /// `[label](wiki:My%20Note)`: percent-encoded, for renderers without angle-bracket support.
    Encoded,
}

/// Turns `[[Target|Alias]]` into regular markdown links with a `wiki:` scheme,
/// leaving fenced code blocks alone.
pub fn wikilinks_to_md(src: &str, style: LinkStyle) -> String {
    let mut out = String::with_capacity(src.len() + 64);
    let mut in_fence = false;
    for line in src.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
        }
        if in_fence || !line.contains("[[") {
            out.push_str(line);
            continue;
        }
        let mut rest = line;
        while let Some(s) = rest.find("[[") {
            let Some(e) = rest[s + 2..].find("]]") else { break };
            let inner = &rest[s + 2..s + 2 + e];
            let (target, label) = match inner.split_once('|') {
                Some((t, l)) => (t.trim(), l.trim()),
                None => (inner.trim(), inner.trim()),
            };
            let label = label.replace('[', r"\[").replace(']', r"\]");
            out.push_str(&rest[..s]);
            match style {
                LinkStyle::AngleBrackets => {
                    out.push_str(&format!("[{label}](<wiki:{}>)", target.replace(['<', '>'], "")));
                }
                LinkStyle::Encoded => out.push_str(&format!("[{label}](wiki:{})", encode(target))),
            }
            rest = &rest[s + 2 + e + 2..];
        }
        out.push_str(rest);
    }
    out
}

fn encode(target: &str) -> String {
    let mut out = String::with_capacity(target.len());
    for c in target.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '%' => out.push_str("%25"),
            '(' => out.push_str("%28"),
            ')' => out.push_str("%29"),
            '<' | '>' => {}
            _ => out.push(c),
        }
    }
    out
}

/// The link under a cursor position (in chars), as a URL; wiki links become `wiki:Target`.
pub fn link_at(text: &str, char_idx: usize) -> Option<String> {
    let pos = text.char_indices().nth(char_idx).map_or(text.len(), |(b, _)| b);
    let start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    let end = text[pos..].find('\n').map_or(text.len(), |i| pos + i);
    let line = &text[start..end];
    let p = pos - start;

    let mut i = 0;
    while let Some(s) = line[i..].find("[[") {
        let s = i + s;
        let Some(e) = line[s + 2..].find("]]") else { break };
        let e = s + 2 + e;
        if p >= s && p <= e + 2 {
            let target = line[s + 2..e].split('|').next().unwrap_or("").trim();
            return Some(format!("wiki:{target}"));
        }
        i = e + 2;
    }

    let mut i = 0;
    while let Some(m) = line[i..].find("](") {
        let m = i + m;
        let Some(e) = line[m..].find(')') else { break };
        let e = m + e;
        let open = line[..m].rfind('[').unwrap_or(m);
        if p >= open && p <= e {
            return Some(line[m + 2..e].trim_matches(['<', '>']).to_string());
        }
        i = e + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wikilinks_become_markdown_links() {
        let src = "a [[My Note|alias]] b";
        assert_eq!(wikilinks_to_md(src, LinkStyle::AngleBrackets), "a [alias](<wiki:My Note>) b");
        assert_eq!(wikilinks_to_md(src, LinkStyle::Encoded), "a [alias](wiki:My%20Note) b");
        assert_eq!(wikilinks_to_md("```\n[[x]]\n```\n", LinkStyle::Encoded), "```\n[[x]]\n```\n");
    }

    #[test]
    fn link_under_cursor() {
        let text = "first line\nsee [[Target#h|T]] or [site](https://example.com)";
        let wiki = text.find("Target").unwrap();
        assert_eq!(link_at(text, wiki).as_deref(), Some("wiki:Target#h"));
        let md = text.find("site").unwrap();
        assert_eq!(link_at(text, md).as_deref(), Some("https://example.com"));
        assert_eq!(link_at(text, 2), None);
    }
}
