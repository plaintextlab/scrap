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
}
