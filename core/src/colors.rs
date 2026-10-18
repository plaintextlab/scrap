//! Coloured text. Markdown has no colour syntax, so colours are stored the way other
//! editors (Obsidian, VS Code) understand them: `<span style="color: blue">text</span>`.
//! The names are the theme accents, which are also CSS colour names, so a note looks
//! right in Scrap's themes and sensible anywhere else. [`strip`] turns a note back into
//! plain markdown for copying and sharing.

/// The colours offered for text, in the order they're shown.
pub const NAMES: [&str; 8] = ["red", "orange", "yellow", "green", "aqua", "blue", "purple", "gray"];

pub const CLOSE: &str = "</span>";

/// The opening tag for `name`.
pub fn open(name: &str) -> String {
    format!("<span style=\"color: {name}\">")
}

/// A `<span ...>` tag at the start of `s`: its length in bytes and, if its style sets a
/// colour, that colour's value (`blue`, `#3366ff`, ...).
pub fn span_tag(s: &str) -> Option<(usize, Option<&str>)> {
    let head = s.get(..5)?;
    if !head.eq_ignore_ascii_case("<span") || !s[5..].starts_with(|c: char| c == '>' || c.is_ascii_whitespace()) {
        return None;
    }
    let end = s.find('>')? + 1;
    let tag = &s[..end];
    if tag.contains('\n') || tag[1..].contains('<') {
        return None;
    }
    let lower = tag.to_ascii_lowercase();
    let color = lower.match_indices("color").find_map(|(i, _)| {
        // Not `background-color`.
        if lower[..i].ends_with('-') {
            return None;
        }
        let rest = lower[i + 5..].trim_start().strip_prefix(':')?;
        let start = end - rest.len();
        let value = &tag[start..];
        let len = value.find([';', '"', '\'', '>']).unwrap_or(value.len());
        Some(value[..len].trim()).filter(|v| !v.is_empty())
    });
    Some((end, color))
}

/// A colour span opening at the start of `s`: (tag length, colour).
pub fn open_tag(s: &str) -> Option<(usize, &str)> {
    match span_tag(s)? {
        (len, Some(color)) => Some((len, color)),
        _ => None,
    }
}

/// `text` without colour spans (their text is kept), for exporting as plain markdown.
/// Code blocks and inline code are left alone, as are other `<span>`s.
pub fn strip(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // For each open <span>: whether it's a colour span (and so removed).
    let mut spans: Vec<bool> = Vec::new();
    let mut in_fence = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        }
        if in_fence || trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            out.push_str(line);
            continue;
        }
        let mut in_code = false;
        let mut i = 0;
        while i < line.len() {
            let rest = &line[i..];
            if rest.starts_with('`') {
                in_code = !in_code;
            } else if !in_code && rest.starts_with('<') {
                if let Some((len, color)) = span_tag(rest) {
                    spans.push(color.is_some());
                    if color.is_none() {
                        out.push_str(&rest[..len]);
                    }
                    i += len;
                    continue;
                }
                if rest.get(..CLOSE.len()).is_some_and(|h| h.eq_ignore_ascii_case(CLOSE)) {
                    if !spans.pop().unwrap_or(false) {
                        out.push_str(&rest[..CLOSE.len()]);
                    }
                    i += CLOSE.len();
                    continue;
                }
            }
            let ch = rest.chars().next().map_or(1, char::len_utf8);
            out.push_str(&rest[..ch]);
            i += ch;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_span_tags() {
        assert_eq!(open_tag(r#"<span style="color: blue">hi</span>"#), Some((26, "blue")));
        assert_eq!(open_tag(r#"<span style='color:#FF0000;font-weight:bold'>x"#).map(|t| t.1), Some("#FF0000"));
        assert_eq!(open_tag(r#"<SPAN STYLE="Color: Red">x"#).map(|t| t.1), Some("Red"));
        assert_eq!(open_tag(r#"<span style="background-color: red">x"#), None);
        assert_eq!(span_tag(r#"<span class="x">"#), Some((16, None)));
        assert_eq!(open_tag("<spanner>"), None);
        assert_eq!(open_tag("<span style=\"color: red\""), None);
        assert_eq!(open(NAMES[5]), r#"<span style="color: blue">"#);
    }

    #[test]
    fn strips_colours_only() {
        let src = "# Title\n\nSome <span style=\"color: red\">red **bold**</span> text and \
                   <span class=\"k\">kept <span style=\"color: blue\">blue</span></span>.\n\
                   `<span style=\"color: red\">code</span>`\n\
                   ```\n<span style=\"color: red\">fenced</span>\n```\n\
                   <span style=\"color: green\">across\nlines</span> ünïcode\n";
        let want = "# Title\n\nSome red **bold** text and <span class=\"k\">kept blue</span>.\n\
                    `<span style=\"color: red\">code</span>`\n\
                    ```\n<span style=\"color: red\">fenced</span>\n```\n\
                    across\nlines ünïcode\n";
        assert_eq!(strip(src), want);
        assert_eq!(strip("no colours"), "no colours");
    }
}
