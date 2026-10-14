//! Markdown in two flavours: syntax highlighting for the editor (the raw text is
//! kept intact so the cursor maps 1:1) and a rendered preview.

use eframe::egui::{
    self, text::LayoutJob, Color32, FontFamily, FontId, RichText, Stroke, TextFormat, Ui, Visuals,
};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};
use scrap_core::links::{wikilinks_to_md, LinkStyle};

pub use scrap_core::links::link_at;

const HEADING_SCALE: [f32; 6] = [1.7, 1.45, 1.25, 1.1, 1.0, 0.95];

struct Colors {
    text: Color32,
    strong: Color32,
    weak: Color32,
    link: Color32,
    accent: Color32,
    code_bg: Color32,
}

impl Colors {
    fn new(v: &Visuals) -> Self {
        Colors {
            text: v.text_color(),
            strong: v.strong_text_color(),
            weak: v.weak_text_color(),
            link: v.hyperlink_color,
            accent: if v.dark_mode { Color32::from_rgb(0xa8, 0x8b, 0xfa) } else { Color32::from_rgb(0x6d, 0x4a, 0xd8) },
            code_bg: v.code_bg_color,
        }
    }
}

fn prop(size: f32, color: Color32) -> TextFormat {
    TextFormat { font_id: FontId::proportional(size), color, ..Default::default() }
}

fn mono(size: f32, color: Color32, background: Color32) -> TextFormat {
    TextFormat { font_id: FontId::monospace(size * 0.9), color, background, ..Default::default() }
}

// ---------------------------------------------------------------------------
// Editor highlighting
// ---------------------------------------------------------------------------

pub fn highlight(text: &str, visuals: &Visuals, size: f32) -> LayoutJob {
    let c = Colors::new(visuals);
    let base = prop(size, c.text);
    let mut job = LayoutJob::default();
    let mut in_fence = false;

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            job.append(line, 0.0, mono(size, c.weak, Color32::TRANSPARENT));
            continue;
        }
        if in_fence {
            job.append(line, 0.0, mono(size, c.text, c.code_bg));
            continue;
        }

        let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with([' ', '\t']) {
            let scaled = size * HEADING_SCALE[hashes - 1];
            let marker = indent + hashes;
            job.append(&line[..marker], 0.0, prop(scaled, c.weak));
            inline(&mut job, &line[marker..], &prop(scaled, c.strong), &c);
            continue;
        }

        if is_rule(trimmed) {
            job.append(line, 0.0, prop(size, c.weak));
            continue;
        }

        let marker = indent + list_prefix(trimmed);
        let mut body = base.clone();
        if trimmed.starts_with('>') {
            body.color = c.weak;
            body.italics = true;
        }
        if marker > indent {
            job.append(&line[..marker], 0.0, prop(size, c.accent));
        } else if marker > 0 {
            job.append(&line[..marker], 0.0, base.clone());
        }
        inline(&mut job, &line[marker..], &body, &c);
    }
    job
}

fn is_rule(t: &str) -> bool {
    let compact: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3 && ['-', '*', '_'].iter().any(|&m| compact.chars().all(|c| c == m))
}

fn list_prefix(t: &str) -> usize {
    for p in ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "- ", "* ", "+ ", "> "] {
        if t.starts_with(p) {
            return p.len();
        }
    }
    if t.starts_with('>') {
        return 1;
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if (1..10).contains(&digits) && (t[digits..].starts_with(". ") || t[digits..].starts_with(") ")) {
        return digits + 2;
    }
    0
}

fn inline(job: &mut LayoutJob, s: &str, f: &TextFormat, c: &Colors) {
    let mut i = 0;
    let mut plain = 0;
    while i < s.len() {
        if let Some((end, parts)) = token(s, i, f, c) {
            if plain < i {
                job.append(&s[plain..i], 0.0, f.clone());
            }
            for (a, z, fmt) in parts {
                if a < z {
                    job.append(&s[a..z], 0.0, fmt);
                }
            }
            i = end;
            plain = end;
            continue;
        }
        i += s[i..].chars().next().map_or(1, char::len_utf8);
    }
    if plain < s.len() {
        job.append(&s[plain..], 0.0, f.clone());
    }
}

type Parts = Vec<(usize, usize, TextFormat)>;

/// Recognises one inline token starting at byte `i`; returns its end and styled pieces.
fn token(s: &str, i: usize, f: &TextFormat, c: &Colors) -> Option<(usize, Parts)> {
    let rest = &s[i..];
    let weak = TextFormat { color: c.weak, ..f.clone() };
    let link = TextFormat { color: c.link, ..f.clone() };

    if let Some(after) = rest.strip_prefix('`') {
        let close = after.find('`')? + 1;
        let code = TextFormat {
            font_id: FontId::monospace(f.font_id.size * 0.9),
            background: c.code_bg,
            ..f.clone()
        };
        return Some((i + close + 1, vec![(i, i + close + 1, code)]));
    }
    if rest.starts_with("[[") {
        let close = rest.find("]]")?;
        return Some((
            i + close + 2,
            vec![(i, i + 2, weak.clone()), (i + 2, i + close, link), (i + close, i + close + 2, weak)],
        ));
    }
    for marker in ["**", "__", "~~"] {
        if rest.starts_with(marker) {
            let close = rest[2..].find(marker)?;
            if close == 0 {
                return None;
            }
            let (a, z) = (i + 2, i + 2 + close);
            let mut inner = f.clone();
            if marker == "~~" {
                inner.strikethrough = Stroke::new(1.0_f32, c.weak);
            } else {
                inner.color = c.strong;
            }
            return Some((z + 2, vec![(i, a, weak.clone()), (a, z, inner), (z, z + 2, weak)]));
        }
    }
    if rest.starts_with('*') && rest[1..].starts_with(|ch: char| !ch.is_whitespace() && ch != '*') {
        let close = rest[1..].find('*')? + 1;
        let italic = TextFormat { italics: true, ..f.clone() };
        let z = i + close;
        return Some((z + 1, vec![(i, i + 1, weak.clone()), (i + 1, z, italic), (z, z + 1, weak)]));
    }
    if rest.starts_with('[') {
        let mid = rest.find("](")?;
        if rest[1..mid].contains('[') {
            return None;
        }
        let end = mid + rest[mid..].find(')')?;
        return Some((
            i + end + 1,
            vec![(i, i + 1, weak.clone()), (i + 1, i + mid, link), (i + mid, i + end + 1, weak)],
        ));
    }
    if rest.starts_with('#') && (i == 0 || s.as_bytes()[i - 1] == b' ') {
        let len = rest[1..]
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '-' | '/')))
            .unwrap_or(rest.len() - 1);
        if len > 0 {
            return Some((i + 1 + len, vec![(i, i + 1 + len, TextFormat { color: c.accent, ..f.clone() })]));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Preview
// ---------------------------------------------------------------------------

/// Renders markdown; returns the URL of a clicked link, if any.
pub fn preview(ui: &mut Ui, src: &str, size: f32) -> Option<String> {
    let md = wikilinks_to_md(src, LinkStyle::AngleBrackets);
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    let mut r = Renderer::new(size);
    for event in Parser::new_ext(&md, opts) {
        r.event(ui, event);
    }
    r.flush(ui);
    r.clicked
}

#[derive(Clone, Default)]
struct SpanStyle {
    strong: bool,
    emph: bool,
    strike: bool,
    code: bool,
    link: Option<String>,
}

struct Span {
    text: String,
    style: SpanStyle,
}

enum Open {
    Para,
    Heading(usize),
    Quote,
    List,
    Item,
    Code,
    Emph,
    Strong,
    Strike,
    Link,
    Table,
    Row,
    Cell,
    Skip,
    Other,
}

#[derive(Default)]
struct Table {
    rows: Vec<Vec<Vec<Span>>>,
    row: Vec<Vec<Span>>,
}

struct Renderer {
    size: f32,
    spans: Vec<Span>,
    style: SpanStyle,
    stack: Vec<Open>,
    lists: Vec<Option<u64>>,
    quote: usize,
    bullet: Option<String>,
    heading: Option<usize>,
    code: Option<String>,
    table: Option<Table>,
    skip: usize,
    tables: usize,
    clicked: Option<String>,
}

impl Renderer {
    fn new(size: f32) -> Self {
        Renderer {
            size,
            spans: Vec::new(),
            style: SpanStyle::default(),
            stack: Vec::new(),
            lists: Vec::new(),
            quote: 0,
            bullet: None,
            heading: None,
            code: None,
            table: None,
            skip: 0,
            tables: 0,
            clicked: None,
        }
    }

    fn push(&mut self, text: &str) {
        if self.skip > 0 {
            return;
        }
        self.spans.push(Span { text: text.to_string(), style: self.style.clone() });
    }

    fn event(&mut self, ui: &mut Ui, event: Event) {
        match event {
            Event::Start(tag) => self.start(ui, tag),
            Event::End(_) => self.end(ui),
            Event::Text(t) => match &mut self.code {
                Some(code) => code.push_str(&t),
                None => self.push(&t),
            },
            Event::Code(t) => {
                if self.skip == 0 {
                    let style = SpanStyle { code: true, ..self.style.clone() };
                    self.spans.push(Span { text: t.to_string(), style });
                }
            }
            Event::Html(t) | Event::InlineHtml(t) => match &mut self.code {
                Some(code) => code.push_str(&t),
                None => self.push(&t),
            },
            Event::SoftBreak => self.push(" "),
            Event::HardBreak => self.push("\n"),
            Event::Rule => {
                self.flush(ui);
                ui.separator();
            }
            Event::TaskListMarker(done) => self.push(if done { "âœ… " } else { "â¬œ " }),
            Event::FootnoteReference(t) => self.push(&format!("[^{t}]")),
            _ => {}
        }
    }

    fn start(&mut self, ui: &mut Ui, tag: Tag) {
        let open = match tag {
            Tag::Paragraph => Open::Para,
            Tag::Heading { level, .. } => {
                self.flush(ui);
                let level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                self.heading = Some(level);
                ui.add_space(self.size * 0.5);
                Open::Heading(level)
            }
            Tag::BlockQuote(..) => {
                self.flush(ui);
                self.quote += 1;
                Open::Quote
            }
            Tag::CodeBlock(_) => {
                self.flush(ui);
                self.code = Some(String::new());
                Open::Code
            }
            Tag::List(start) => {
                self.flush(ui);
                self.lists.push(start);
                Open::List
            }
            Tag::Item => {
                self.flush(ui);
                self.bullet = Some(match self.lists.last_mut() {
                    Some(Some(n)) => {
                        let b = format!("{n}. ");
                        *n += 1;
                        b
                    }
                    _ => "â€¢  ".to_string(),
                });
                Open::Item
            }
            Tag::Emphasis => {
                self.style.emph = true;
                Open::Emph
            }
            Tag::Strong => {
                self.style.strong = true;
                Open::Strong
            }
            Tag::Strikethrough => {
                self.style.strike = true;
                Open::Strike
            }
            Tag::Link { dest_url, .. } => {
                self.style.link = Some(dest_url.to_string());
                Open::Link
            }
            Tag::Image { dest_url, .. } => {
                self.style.link = Some(dest_url.to_string());
                self.push("ðŸ–¼ ");
                Open::Link
            }
            Tag::Table(_) => {
                self.flush(ui);
                self.table = Some(Table::default());
                Open::Table
            }
            Tag::TableHead | Tag::TableRow => Open::Row,
            Tag::TableCell => {
                self.spans.clear();
                Open::Cell
            }
            Tag::MetadataBlock(_) => {
                self.skip += 1;
                Open::Skip
            }
            _ => Open::Other,
        };
        self.stack.push(open);
    }

    fn end(&mut self, ui: &mut Ui) {
        match self.stack.pop() {
            Some(Open::Para) => {
                self.flush(ui);
                ui.add_space(if self.lists.is_empty() { self.size * 0.5 } else { 2.0 });
            }
            Some(Open::Heading(level)) => {
                self.flush(ui);
                self.heading = None;
                if level <= 2 {
                    ui.separator();
                }
                ui.add_space(self.size * 0.3);
            }
            Some(Open::Quote) => {
                self.flush(ui);
                self.quote = self.quote.saturating_sub(1);
            }
            Some(Open::List) => {
                self.flush(ui);
                self.lists.pop();
                if self.lists.is_empty() {
                    ui.add_space(self.size * 0.5);
                }
            }
            Some(Open::Item) => self.flush(ui),
            Some(Open::Code) => {
                let code = self.code.take().unwrap_or_default();
                self.code_block(ui, &code);
            }
            Some(Open::Emph) => self.style.emph = false,
            Some(Open::Strong) => self.style.strong = false,
            Some(Open::Strike) => self.style.strike = false,
            Some(Open::Link) => self.style.link = None,
            Some(Open::Cell) => {
                let cell = std::mem::take(&mut self.spans);
                if let Some(t) = &mut self.table {
                    t.row.push(cell);
                }
            }
            Some(Open::Row) => {
                if let Some(t) = &mut self.table {
                    let row = std::mem::take(&mut t.row);
                    t.rows.push(row);
                }
            }
            Some(Open::Table) => {
                if let Some(t) = self.table.take() {
                    self.table_ui(ui, t);
                }
            }
            Some(Open::Skip) => self.skip = self.skip.saturating_sub(1),
            _ => {}
        }
    }

    /// Lays out the pending inline spans as one wrapped block.
    fn flush(&mut self, ui: &mut Ui) {
        if self.table.is_some() {
            return;
        }
        let bullet = self.bullet.take();
        if self.spans.is_empty() && bullet.is_none() {
            return;
        }
        let spans = std::mem::take(&mut self.spans);
        let list_indent = self.lists.len().saturating_sub(1) as f32 * 20.0;
        let indent = list_indent + self.quote as f32 * 14.0;
        let size = self.heading.map_or(self.size, |h| self.size * HEADING_SCALE[h - 1]);
        let heading = self.heading.is_some();
        let quote = self.quote;
        let mut clicked = None;

        let resp = ui.horizontal(|ui| {
            ui.add_space(indent);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if let Some(b) = &bullet {
                    ui.label(RichText::new(b).size(size).color(ui.visuals().weak_text_color()));
                }
                for span in &spans {
                    if let Some(url) = span_ui(ui, span, size, heading, quote > 0) {
                        clicked = Some(url);
                    }
                }
            });
        });

        let rect = resp.response.rect;
        let stroke = Stroke::new(3.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color);
        for q in 0..quote {
            let x = rect.left() + list_indent + q as f32 * 14.0 + 3.0;
            ui.painter().vline(x, rect.y_range(), stroke);
        }
        if clicked.is_some() {
            self.clicked = clicked;
        }
    }

    fn code_block(&self, ui: &mut Ui, code: &str) {
        egui::Frame::default()
            .fill(ui.visuals().code_bg_color)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    RichText::new(code.trim_end_matches('\n'))
                        .family(FontFamily::Monospace)
                        .size(self.size * 0.9),
                );
            });
        ui.add_space(self.size * 0.5);
    }

    fn table_ui(&mut self, ui: &mut Ui, table: Table) {
        self.tables += 1;
        let size = self.size;
        let mut clicked = None;
        egui::Grid::new(("md_table", self.tables))
            .striped(true)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                for (r, row) in table.rows.iter().enumerate() {
                    for cell in row {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            for span in cell {
                                if let Some(url) = span_ui(ui, span, size, r == 0, false) {
                                    clicked = Some(url);
                                }
                            }
                        });
                    }
                    ui.end_row();
                }
            });
        if clicked.is_some() {
            self.clicked = clicked;
        }
        ui.add_space(self.size * 0.5);
    }
}

fn span_ui(ui: &mut Ui, span: &Span, size: f32, heading: bool, quote: bool) -> Option<String> {
    let st = &span.style;
    let mut rt = RichText::new(&span.text).size(size);
    if heading || st.strong {
        rt = rt.strong();
    }
    if st.emph || quote {
        rt = rt.italics();
    }
    if st.strike {
        rt = rt.strikethrough();
    }
    if st.code {
        rt = rt
            .family(FontFamily::Monospace)
            .size(size * 0.9)
            .background_color(ui.visuals().code_bg_color);
    }
    match &st.link {
        Some(url) => {
            let hover = url.strip_prefix("wiki:").unwrap_or(url);
            ui.link(rt).on_hover_text(hover).clicked().then(|| url.clone())
        }
        None => {
            ui.label(rt);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlighting_keeps_every_byte() {
        let src = "# Title **x**\n- [ ] task with `code` and [[Link|alias]]\n> quote *em*\n```rust\nlet a = 1;\n```\n1. [md](http://x) #tag ~~gone~~ é**ü**\n**unclosed and [broken\n";
        let job = highlight(src, &Visuals::dark(), 14.0);
        assert_eq!(job.text, src);
    }
}
