//! Lists and checklists: continuing a list when Enter is pressed, and ticking items.
//!
//! The Android app has a Kotlin twin of this (ui/Lists.kt) for its editor and widget.

/// What a new note made as a checklist starts with.
pub const NEW_CHECKLIST: &str = "1. [ ] ";

/// A list item's line: its marker (indent, bullet or number, checkbox) and what follows.
#[derive(Debug, PartialEq)]
pub struct Item<'a> {
    pub indent: &'a str,
    /// `-`, `*` or `+`; or `None` for a numbered item.
    pub bullet: Option<char>,
    pub number: Option<(u64, char)>,
    /// `Some(done)` for a checklist item.
    pub task: Option<bool>,
    /// Byte length of the whole marker, up to the item's text.
    pub prefix: usize,
}

impl Item<'_> {
    /// The marker for the item after this one: the next number, an unticked box.
    pub fn next_marker(&self) -> String {
        let head = match (self.bullet, self.number) {
            (Some(b), _) => b.to_string(),
            (None, Some((n, delim))) => format!("{}{delim}", n + 1),
            (None, None) => String::new(),
        };
        let task = if self.task.is_some() { " [ ]" } else { "" };
        format!("{}{head}{task} ", self.indent)
    }
}

/// Parses a list item at the start of `line` (which may still end in a newline).
pub fn item(line: &str) -> Option<Item<'_>> {
    let body = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - body.len()];
    let (bullet, number, after) = match body.chars().next()? {
        b @ ('-' | '*' | '+') => (Some(b), None, &body[1..]),
        _ => {
            let digits = body.bytes().take_while(u8::is_ascii_digit).count();
            if !(1..10).contains(&digits) {
                return None;
            }
            let delim = body[digits..].chars().next().filter(|c| *c == '.' || *c == ')')?;
            (None, Some((body[..digits].parse().ok()?, delim)), &body[digits + 1..])
        }
    };
    // The marker must be followed by a space (or end the line: an empty item).
    let rest = after.strip_prefix(' ').or_else(|| after.trim_end_matches(['\n', '\r']).is_empty().then_some(""))?;
    let (task, rest) = match rest.get(..3) {
        Some("[ ]") => (Some(false), &rest[3..]),
        Some("[x]" | "[X]") => (Some(true), &rest[3..]),
        _ => (None, rest),
    };
    let rest = if task.is_some() { rest.strip_prefix(' ').unwrap_or(rest) } else { rest };
    let prefix = line.len() - rest.len();
    Some(Item { indent, bullet, number, task, prefix })
}

/// After a newline was typed at byte `nl` of `text` (the `\n` itself is at `nl`): continues
/// the list the previous line is in, or ends it if that item was left empty. Returns the
/// new text and the cursor's new byte position, or `None` if the line isn't a list item.
pub fn continue_list(text: &str, nl: usize) -> Option<(String, usize)> {
    if text.as_bytes().get(nl) != Some(&b'\n') {
        return None;
    }
    let start = text[..nl].rfind('\n').map_or(0, |i| i + 1);
    let line = &text[start..nl];
    let item = item(line)?;
    if line[item.prefix..].trim().is_empty() {
        // Enter on an empty item: drop its marker and stay on that (now empty) line.
        let out = format!("{}{}", &text[..start], &text[nl + 1..]);
        return Some((out, start));
    }
    let marker = item.next_marker();
    let out = format!("{}{marker}{}", &text[..=nl], &text[nl + 1..]);
    Some((out, nl + 1 + marker.len()))
}

/// Line numbers (0-based) of the checklist items in `text`, in order, outside code blocks.
/// Items in block quotes (`> - [ ] x`) count too, as markdown renders them as checkboxes.
pub fn task_lines(text: &str) -> Vec<usize> {
    let mut in_fence = false;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let unquoted = unquote(line);
        if item(unquoted).is_some_and(|i| i.task.is_some()) {
            out.push(n);
        }
    }
    out
}

fn unquote(mut line: &str) -> &str {
    loop {
        let t = line.trim_start();
        match t.strip_prefix('>') {
            Some(rest) => line = rest.strip_prefix(' ').unwrap_or(rest),
            None => return line,
        }
    }
}

/// Ticks or unticks the `nth` checklist item (counting as [`task_lines`] does).
pub fn toggle_task(text: &str, nth: usize) -> Option<String> {
    let line_no = *task_lines(text).get(nth)?;
    let mut out = String::with_capacity(text.len());
    for (n, line) in text.split_inclusive('\n').enumerate() {
        if n == line_no {
            let unquoted = unquote(line);
            let quote = line.len() - unquoted.len();
            let it = item(unquoted)?;
            // The box sits just before the item text: "[ ] " or "[ ]".
            let head = &unquoted[..it.prefix];
            let at = quote + head.rfind('[')?;
            let mark = if it.task == Some(true) { ' ' } else { 'x' };
            out.push_str(&line[..at + 1]);
            out.push(mark);
            out.push_str(&line[at + 2..]);
        } else {
            out.push_str(line);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_items() {
        let i = item("  12. [x] eggs").unwrap();
        assert_eq!((i.indent, i.number, i.task, i.prefix), ("  ", Some((12, '.')), Some(true), 10));
        assert_eq!(i.next_marker(), "  13. [ ] ");
        assert_eq!(item("- milk").unwrap().next_marker(), "- ");
        assert_eq!(item("3) bread").unwrap().next_marker(), "4) ");
        assert_eq!(item("1. [ ] ").unwrap().prefix, 7);
        assert_eq!(item("-").map(|i| i.prefix), Some(1));
        assert!(item("-not a list").is_none());
        assert!(item("2024. a year").is_some()); // markdown agrees: an ordered list item
        assert!(item("1234567890. too long").is_none());
        assert!(item("plain").is_none());
    }

    #[test]
    fn continues_lists() {
        let text = "1. [ ] milk\n";
        assert_eq!(continue_list(text, 11), Some(("1. [ ] milk\n2. [ ] ".to_string(), 19)));
        let text = "- [x] done\nafter";
        assert_eq!(continue_list(text, 10).unwrap().0, "- [x] done\n- [ ] after");
        // An empty item ends the list.
        let text = "1. [ ] milk\n2. [ ] \n";
        assert_eq!(continue_list(text, 19), Some(("1. [ ] milk\n".to_string(), 12)));
        assert_eq!(continue_list("plain\n", 5), None);
        assert_eq!(continue_list("- a", 1), None);
    }

    #[test]
    fn toggles_tasks() {
        let text = "# List\n1. [ ] milk\n2. [x] eggs\n```\n- [ ] code\n```\n> - [ ] quoted\n- plain\n";
        assert_eq!(task_lines(text), vec![1, 2, 6]);
        let once = toggle_task(text, 0).unwrap();
        assert!(once.contains("1. [x] milk"));
        let twice = toggle_task(&once, 1).unwrap();
        assert!(twice.contains("2. [ ] eggs"));
        assert!(toggle_task(text, 2).unwrap().contains("> - [x] quoted"));
        assert_eq!(toggle_task(text, 3), None);
        assert_eq!(toggle_task(&toggle_task(text, 0).unwrap(), 0).unwrap(), text);
    }
}
