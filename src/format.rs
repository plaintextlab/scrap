//! Formatting toolbar operations. They work on character (not byte) offsets, like
//! egui's text cursors, and return the new text with the new selection.

pub struct Edit {
    pub text: String,
    /// New selection as character offsets (start, end); equal for a plain cursor.
    pub selection: (usize, usize),
}

fn byte(text: &str, char_index: usize) -> usize {
    text.char_indices().nth(char_index).map_or(text.len(), |(b, _)| b)
}

fn chars(s: &str) -> usize {
    s.chars().count()
}

/// Wraps the selection in `before`/`after`, or inserts the empty pair at the cursor.
/// If the selection is already wrapped, the markers are removed instead.
pub fn wrap(text: &str, sel: (usize, usize), before: &str, after: &str) -> Edit {
    let (start, end) = (sel.0.min(sel.1), sel.0.max(sel.1));
    let (a, z) = (byte(text, start), byte(text, end));
    let nb = chars(before);
    // Toggle off: the markers sit just outside the selection.
    if start != end && text[..a].ends_with(before) && text[z..].starts_with(after) {
        let out = format!("{}{}{}", &text[..a - before.len()], &text[a..z], &text[z + after.len()..]);
        return Edit { text: out, selection: (start - nb, end - nb) };
    }
    let out = format!("{}{before}{}{after}{}", &text[..a], &text[a..z], &text[z..]);
    let selection = if start == end { (start + nb, start + nb) } else { (start + nb, end + nb) };
    Edit { text: out, selection }
}

/// Colours the selection (`Some(name)`), or removes its colour (`None`). A selection that
/// is already coloured, either just inside or including its `<span>` tags, is recoloured
/// rather than wrapped again. With no selection, the coloured text around the cursor is
/// recoloured, or else the cursor's line (after any list, heading or quote marker); on an
/// empty line an empty coloured span is inserted to type into.
pub fn color(text: &str, sel: (usize, usize), name: Option<&str>) -> Edit {
    use scrap_core::colors;
    let (start, end) = (sel.0.min(sel.1), sel.0.max(sel.1));
    let cursor = start == end;
    let (a, z) = if cursor {
        let at = byte(text, start);
        let span = enclosing_span(text, at);
        // An empty span (inserted earlier on an empty line, say) next to text on the line:
        // drop it and colour the line instead.
        if let Some((inner, _)) = span.filter(|&(ia, iz)| ia == iz && name.is_some()) {
            let open_at = text[..inner].rfind('<').unwrap_or(inner);
            let cleaned = format!("{}{}", &text[..open_at], &text[inner + colors::CLOSE.len()..]);
            let cur = chars(&text[..open_at]);
            let (la, lz) = line_text(&cleaned, open_at);
            if la < lz {
                return color(&cleaned, (cur, cur), name);
            }
        }
        span.unwrap_or_else(|| line_text(text, at))
    } else {
        (byte(text, start), byte(text, end))
    };
    // The span being replaced (byte range) and the text it holds.
    let selected = &text[a..z];
    let whole = colors::open_tag(selected)
        .filter(|_| selected.ends_with(colors::CLOSE))
        .map(|(len, _)| (a, z, a + len, z - colors::CLOSE.len()));
    let around = || {
        let before = &text[..a];
        let open_at = before.rfind('<')?;
        let (len, _) = colors::open_tag(&before[open_at..])?;
        (open_at + len == a && text[z..].starts_with(colors::CLOSE)).then_some((open_at, z + colors::CLOSE.len(), a, z))
    };
    let (outer_a, outer_z, inner_a, inner_z) = whole.or_else(around).unwrap_or((a, z, a, z));
    let inner = &text[inner_a..inner_z];
    if name.is_none() && (outer_a, outer_z) == (inner_a, inner_z) {
        return Edit { text: text.to_string(), selection: (start, end) };
    }
    let (open, close) = match name {
        Some(n) => (colors::open(n), colors::CLOSE),
        None => (String::new(), ""),
    };
    let out = format!("{}{open}{inner}{close}{}", &text[..outer_a], &text[outer_z..]);
    let first = chars(&text[..outer_a]) + chars(&open);
    let last = first + chars(inner);
    // A selection stays selected; a cursor goes to the end of the coloured text.
    Edit { text: out, selection: if cursor { (last, last) } else { (first, last) } }
}

/// The text inside the colour span the cursor (byte `at`) is in, on its line.
fn enclosing_span(text: &str, at: usize) -> Option<(usize, usize)> {
    use scrap_core::colors;
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    let line = &text[line_start..line_end];
    let mut best = None;
    for (i, _) in line.match_indices('<') {
        let Some((len, _)) = colors::open_tag(&line[i..]) else { continue };
        let inner = line_start + i + len;
        let Some(close) = text[inner..line_end].find(colors::CLOSE) else { continue };
        // Inside the tags, or right before or after them, counts as in the span.
        let outer_end = inner + close + colors::CLOSE.len();
        if line_start + i <= at && at <= outer_end {
            best = Some((inner, inner + close));
        }
    }
    best
}

/// The cursor's line without its list, heading or quote marker and surrounding spaces;
/// an empty range at the cursor if that leaves nothing.
fn line_text(text: &str, at: usize) -> (usize, usize) {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    let line = text[line_start..line_end].trim_end();
    let body = line.trim_start();
    let mut skip = line.len() - body.len();
    let hashes = body.bytes().take_while(|&b| b == b'#').count();
    let markers = ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "- ", "* ", "+ ", "> "];
    if (1..=6).contains(&hashes) && body[hashes..].starts_with(' ') {
        skip += hashes + 1;
    } else if let Some(m) = markers.iter().find(|m| body.starts_with(**m)) {
        skip += m.len();
    } else {
        let digits = body.bytes().take_while(u8::is_ascii_digit).count();
        if (1..10).contains(&digits) && (body[digits..].starts_with(". ") || body[digits..].starts_with(") ")) {
            skip += digits + 2;
        }
    }
    let rest = &line[skip..];
    let a = line_start + skip + (rest.len() - rest.trim_start().len());
    let z = line_start + line.len();
    if a >= z {
        (at, at)
    } else {
        (a, z)
    }
}

/// Adds `prefix` (such as "- " or "# ") to every line the selection touches, or removes
/// it if all of them already have it.
pub fn toggle_line_prefix(text: &str, sel: (usize, usize), prefix: &str) -> Edit {
    let (start, end) = (sel.0.min(sel.1), sel.0.max(sel.1));
    let (a, z) = (byte(text, start), byte(text, end));
    let first = text[..a].rfind('\n').map_or(0, |i| i + 1);
    let last_end = text[z..].find('\n').map_or(text.len(), |i| z + i);
    let block = &text[first..last_end];
    let lines: Vec<&str> = block.split('\n').collect();
    let all = lines.iter().all(|l| l.starts_with(prefix));
    let n = chars(prefix);

    let mut out_lines = Vec::with_capacity(lines.len());
    // How the selection ends move: by the change on their own line and the lines before.
    let mut shift_start: isize = 0;
    let mut shift_end: isize = 0;
    let mut pos = first;
    for line in &lines {
        let delta: isize = if all {
            -(n as isize)
        } else if line.starts_with(prefix) {
            0
        } else {
            n as isize
        };
        if pos <= a {
            shift_start += delta;
        }
        if pos <= z {
            shift_end += delta;
        }
        out_lines.push(if all {
            line[prefix.len()..].to_string()
        } else if line.starts_with(prefix) {
            line.to_string()
        } else {
            format!("{prefix}{line}")
        });
        pos += line.len() + 1;
    }
    let line_start = chars(&text[..first]);
    let clamp = |p: usize, s: isize| ((p as isize + s).max(line_start as isize)) as usize;
    let out = format!("{}{}{}", &text[..first], out_lines.join("\n"), &text[last_end..]);
    Edit { text: out, selection: (clamp(start, shift_start), clamp(end, shift_end)) }
}

/// Inserts `block` as a paragraph of its own (blank lines around it, so markdown
/// doesn't fold it into a neighbouring list item) and puts the cursor after it.
pub fn insert_block(text: &str, cursor: usize, block: &str) -> Edit {
    let at = byte(text, cursor);
    let (head, tail) = text.split_at(at);
    let before = if head.is_empty() || head.ends_with("\n\n") {
        ""
    } else if head.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let after = if tail.starts_with("\n\n") {
        ""
    } else if tail.starts_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let inserted = format!("{before}{block}{after}");
    let caret = cursor + chars(&inserted);
    Edit { text: format!("{head}{inserted}{tail}"), selection: (caret, caret) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_and_unwrap() {
        let e = wrap("say hi now", (4, 6), "**", "**");
        assert_eq!(e.text, "say **hi** now");
        assert_eq!(e.selection, (6, 8));
        let back = wrap(&e.text, e.selection, "**", "**");
        assert_eq!(back.text, "say hi now");
        assert_eq!(back.selection, (4, 6));
        let empty = wrap("ab", (1, 1), "[[", "]]");
        assert_eq!((empty.text.as_str(), empty.selection), ("a[[]]b", (3, 3)));
        // Character offsets, not bytes.
        assert_eq!(wrap("é x", (2, 3), "*", "*").text, "é *x*");
    }

    #[test]
    fn line_prefixes() {
        let e = toggle_line_prefix("one\ntwo\nthree", (5, 9), "- ");
        assert_eq!(e.text, "one\n- two\n- three");
        assert_eq!(e.selection, (7, 13));
        let back = toggle_line_prefix(&e.text, e.selection, "- ");
        assert_eq!(back.text, "one\ntwo\nthree");
        assert_eq!(toggle_line_prefix("# a", (3, 3), "# ").text, "a");
        assert_eq!(toggle_line_prefix("", (0, 0), "- [ ] ").text, "- [ ] ");
        // Mixed lines: everything gets the prefix.
        assert_eq!(toggle_line_prefix("- a\nb", (0, 5), "- ").text, "- a\n- b");
    }

    #[test]
    fn blocks_get_their_own_paragraph() {
        let e = insert_block("- [ ] task", 10, "![](x.png)");
        assert_eq!(e.text, "- [ ] task\n\n![](x.png)\n\n");
        assert_eq!(e.selection.0, e.text.chars().count());
        assert_eq!(insert_block("a\n\nb", 3, "X").text, "a\n\nX\n\nb");
        assert_eq!(insert_block("", 0, "X").text, "X\n\n");
    }

    #[test]
    fn colors_selection() {
        let blue = r#"<span style="color: blue">"#;
        // Wrap, keeping the words selected.
        let e = color("say hi now", (4, 6), Some("blue"));
        assert_eq!(e.text, format!("say {blue}hi</span> now"));
        let (s, z) = e.selection;
        assert_eq!(e.text.chars().skip(s).take(z - s).collect::<String>(), "hi");
        // Recolour with the words selected, or the whole span selected.
        let red = color(&e.text, e.selection, Some("red"));
        assert_eq!(red.text, r#"say <span style="color: red">hi</span> now"#);
        let all = (4, 4 + red.text[4..].find(" now").unwrap());
        assert_eq!(color(&red.text, all, Some("green")).text, r#"say <span style="color: green">hi</span> now"#);
        // Remove.
        assert_eq!(color(&red.text, red.selection, None).text, "say hi now");
        assert_eq!(color("plain", (0, 5), None).text, "plain");
        // At a cursor: the line is coloured, after any marker, and the cursor ends up after it.
        let e = color("one\n- two words \nthree", (9, 9), Some("blue"));
        assert_eq!(e.text, format!("one\n- {blue}two words</span> \nthree"));
        let end = e.text.find("</span>").unwrap();
        assert_eq!(e.selection, (end, end));
        assert_eq!(color("## Title", (3, 3), Some("red")).text, r#"## <span style="color: red">Title</span>"#);
        // In (or just after) coloured text: that span is recoloured or uncoloured.
        let coloured = format!("say {blue}hi</span> now");
        let inside = 4 + blue.len() + 1;
        assert_eq!(color(&coloured, (inside, inside), Some("red")).text, r#"say <span style="color: red">hi</span> now"#);
        let after = coloured.find(" now").unwrap();
        assert_eq!(color(&coloured, (after, after), None).text, "say hi now");
        // An empty span (from an empty line) can be removed too.
        let empty = format!("x{blue}</span>");
        assert_eq!(color(&empty, (1, 1), None).text, "x");
        // Colouring there replaces the empty span by colouring the line.
        let typed = r#"Call now.<span style="color: red"></span>"#;
        let end = typed.chars().count();
        assert_eq!(color(typed, (end, end), Some("red")).text, r#"<span style="color: red">Call now.</span>"#);
        // On an empty line: an empty span with the cursor inside, to type into.
        let e = color("a\n\nb", (2, 2), Some("blue"));
        assert_eq!(e.text, format!("a\n{blue}</span>\nb"));
        assert_eq!(e.selection, (2 + blue.len(), 2 + blue.len()));
    }
}
