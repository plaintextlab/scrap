//! Markdown in two flavours: syntax highlighting for the editor (the raw text is
//! kept intact so the cursor maps 1:1) and a rendered reader view.

use crate::theme::{self, icon, Palette};
use eframe::egui::{
    self, text::LayoutJob, Color32, CornerRadius, FontFamily, FontId, Margin, RichText, Stroke, TextFormat, Ui,
    Visuals,
};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};
use scrap_core::links::{wikilinks_to_md, LinkStyle};
use std::path::Path;

pub use scrap_core::links::link_at;

const HEADING_SCALE: [f32; 6] = [1.75, 1.4, 1.2, 1.08, 1.0, 0.95];

fn colors(v: &Visuals) -> Palette {
    theme::palette_for(v.dark_mode)
}

fn prop(size: f32, color: Color32) -> TextFormat {
    TextFormat { font_id: FontId::proportional(size), color, ..Default::default() }
}

fn bold(size: f32, color: Color32) -> TextFormat {
    TextFormat { font_id: FontId::new(size, theme::semibold()), color, ..Default::default() }
}

fn mono(size: f32, color: Color32, background: Color32) -> TextFormat {
    TextFormat { font_id: FontId::monospace(size * 0.88), color, background, ..Default::default() }
}

// ---------------------------------------------------------------------------
// Editor highlighting
// ---------------------------------------------------------------------------

pub fn highlight(text: &str, visuals: &Visuals, size: f32) -> LayoutJob {
    let c = colors(visuals);
    let base = prop(size, c.text);
    let mut job = LayoutJob::default();
    let mut in_fence = false;

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            job.append(line, 0.0, mono(size, c.faint, Color32::TRANSPARENT));
            continue;
        }
        if in_fence {
            job.append(line, 0.0, mono(size, c.green, c.code_bg));
            continue;
        }

        let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with([' ', '\t']) {
            let scaled = size * HEADING_SCALE[hashes - 1];
            let marker = indent + hashes;
            job.append(&line[..marker], 0.0, prop(scaled, c.faint));
            inline(&mut job, &line[marker..], &bold(scaled, c.yellow), &c);
            continue;
        }

        if is_rule(trimmed) {
            job.append(line, 0.0, prop(size, c.faint));
            continue;
        }

        let marker = indent + list_prefix(trimmed);
        let mut body = base.clone();
        if trimmed.starts_with('>') {
            body.color = c.gray;
            body.italics = true;
        }
        if marker > indent {
            job.append(&line[..marker], 0.0, prop(size, c.orange));
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

fn inline(job: &mut LayoutJob, s: &str, f: &TextFormat, c: &Palette) {
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
fn token(s: &str, i: usize, f: &TextFormat, c: &Palette) -> Option<(usize, Parts)> {
    let rest = &s[i..];
    let weak = TextFormat { color: c.faint, ..f.clone() };
    let link = TextFormat { color: c.blue, ..f.clone() };

    if let Some(after) = rest.strip_prefix('`') {
        let close = after.find('`')? + 1;
        let code = TextFormat {
            font_id: FontId::monospace(f.font_id.size * 0.88),
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
                inner.strikethrough = Stroke::new(1.0_f32, c.muted);
                inner.color = c.muted;
            } else {
                inner.font_id = FontId::new(f.font_id.size, theme::semibold());
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
    // Links and images: [text](url), ![alt](url).
    let bang = rest.starts_with("![");
    if rest.starts_with('[') || bang {
        let open = usize::from(bang);
        let mid = rest.find("](")?;
        if rest[open + 1..mid].contains('[') {
            return None;
        }
        let end = mid + rest[mid..].find(')')?;
        return Some((
            i + end + 1,
            vec![
                (i, i + open + 1, weak.clone()),
                (i + open + 1, i + mid, link),
                (i + mid, i + end + 1, weak),
            ],
        ));
    }
    if rest.starts_with('#') && (i == 0 || s.as_bytes()[i - 1] == b' ') {
        let len = rest[1..]
            .find(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '-' | '/')))
            .unwrap_or(rest.len() - 1);
        if len > 0 {
            return Some((i + 1 + len, vec![(i, i + 1 + len, TextFormat { color: c.aqua, ..f.clone() })]));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Where relative image links in a note point.
pub struct Images<'a> {
    pub root: &'a Path,
    pub note_rel: &'a str,
}

/// Renders markdown; returns the URL of a clicked link, if any.
pub fn preview(ui: &mut Ui, src: &str, size: f32, images: &Images) -> Option<String> {
    let md = wikilinks_to_md(src, LinkStyle::AngleBrackets);
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    ui.spacing_mut().item_spacing.y = 4.0;
    let mut r = Renderer::new(size, colors(ui.visuals()));
    for event in Parser::new_ext(&md, opts) {
        r.event(ui, event, images);
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
    color: Option<Color32>,
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
    Image(String),
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
    c: Palette,
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
    fn new(size: f32, c: Palette) -> Self {
        Renderer {
            size,
            c,
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

    fn event(&mut self, ui: &mut Ui, event: Event, images: &Images) {
        match event {
            Event::Start(tag) => self.start(ui, tag),
            Event::End(_) => self.end(ui, images),
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
                ui.add_space(self.size * 0.4);
                let r = ui.available_rect_before_wrap();
                ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, self.c.border));
                ui.add_space(self.size * 0.6);
            }
            Event::TaskListMarker(done) => {
                if self.skip == 0 {
                    // The list bullet is replaced by the checkbox.
                    self.bullet = Some(String::new());
                    let (glyph, color) = if done { (icon::CHECK_SQUARE, self.c.accent) } else { (icon::SQUARE, self.c.faint) };
                    let style = SpanStyle { color: Some(color), ..SpanStyle::default() };
                    self.spans.push(Span { text: format!("{glyph}  "), style });
                    if done {
                        self.style.strike = true;
                        self.style.color = Some(self.c.muted);
                    }
                }
            }
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
                ui.add_space(self.size * if level <= 2 { 0.9 } else { 0.6 });
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
                        let b = format!("{n}.  ");
                        *n += 1;
                        b
                    }
                    _ => "•  ".to_string(),
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
                // Images are drawn as blocks of their own; their alt text is skipped.
                self.flush(ui);
                self.skip += 1;
                Open::Image(dest_url.to_string())
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

    fn end(&mut self, ui: &mut Ui, images: &Images) {
        match self.stack.pop() {
            Some(Open::Para) => {
                self.flush(ui);
                ui.add_space(if self.lists.is_empty() { self.size * 0.7 } else { 2.0 });
            }
            Some(Open::Heading(level)) => {
                self.flush(ui);
                self.heading = None;
                if level <= 2 {
                    ui.add_space(4.0);
                    let r = ui.available_rect_before_wrap();
                    ui.painter().hline(r.x_range(), r.top(), Stroke::new(1.0, self.c.border));
                }
                ui.add_space(self.size * 0.45);
            }
            Some(Open::Quote) => {
                self.flush(ui);
                self.quote = self.quote.saturating_sub(1);
            }
            Some(Open::List) => {
                self.flush(ui);
                self.lists.pop();
                if self.lists.is_empty() {
                    ui.add_space(self.size * 0.6);
                }
            }
            Some(Open::Item) => {
                self.flush(ui);
                self.style.strike = false;
                self.style.color = None;
            }
            Some(Open::Code) => {
                let code = self.code.take().unwrap_or_default();
                self.code_block(ui, &code);
            }
            Some(Open::Emph) => self.style.emph = false,
            Some(Open::Strong) => self.style.strong = false,
            Some(Open::Strike) => self.style.strike = false,
            Some(Open::Link) => self.style.link = None,
            Some(Open::Image(url)) => {
                self.skip = self.skip.saturating_sub(1);
                self.image(ui, &url, images);
            }
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

    fn image(&mut self, ui: &mut Ui, url: &str, images: &Images) {
        let local = scrap_core::attachments::resolve(images.note_rel, url).map(|rel| images.root.join(rel));
        match local {
            Some(path) if path.exists() => {
                ui.add_space(4.0);
                let uri = crate::attach::file_uri(&path);
                let width = ui.available_width();
                ui.add(
                    egui::Image::new(uri)
                        .fit_to_original_size(1.0)
                        .max_width(width)
                        .corner_radius(CornerRadius::same(10))
                        .show_loading_spinner(true),
                );
                ui.add_space(self.size * 0.6);
            }
            _ => {
                // Web images and missing files: show a link instead.
                let label = format!("{}  {}", icon::IMAGE, url.rsplit('/').next().unwrap_or(url));
                let resp = ui.link(RichText::new(label).size(self.size * 0.9));
                if resp.on_hover_text(url).clicked() && url.contains("://") {
                    self.clicked = Some(url.to_string());
                }
            }
        }
    }

    /// Lays out the pending inline spans as one wrapped block.
    fn flush(&mut self, ui: &mut Ui) {
        if self.table.is_some() {
            return;
        }
        let bullet = self.bullet.take();
        if self.spans.is_empty() && bullet.as_deref().is_none_or(str::is_empty) {
            return;
        }
        let spans = std::mem::take(&mut self.spans);
        let list_indent = self.lists.len().saturating_sub(1) as f32 * 22.0;
        let indent = list_indent + self.quote as f32 * 16.0;
        let size = self.heading.map_or(self.size, |h| self.size * HEADING_SCALE[h - 1]);
        let heading = self.heading.is_some();
        let quote = self.quote;
        let c = self.c;
        let mut clicked = None;

        let resp = ui.horizontal(|ui| {
            ui.add_space(indent);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.spacing_mut().item_spacing.y = size * 0.3;
                if let Some(b) = bullet.as_deref().filter(|b| !b.is_empty()) {
                    ui.label(RichText::new(b).size(size).color(c.orange));
                }
                for span in &spans {
                    if let Some(url) = span_ui(ui, span, size, heading, quote > 0, &c) {
                        clicked = Some(url);
                    }
                }
            });
        });

        let rect = resp.response.rect;
        for q in 0..quote {
            let x = rect.left() + list_indent + q as f32 * 16.0 + 3.0;
            ui.painter().vline(x, rect.y_range(), Stroke::new(3.0, c.gray.gamma_multiply(0.6)));
        }
        if clicked.is_some() {
            self.clicked = clicked;
        }
    }

    fn code_block(&self, ui: &mut Ui, code: &str) {
        egui::Frame::new()
            .fill(self.c.code_bg)
            .stroke(Stroke::new(1.0, self.c.border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(
                    RichText::new(code.trim_end_matches('\n'))
                        .family(FontFamily::Monospace)
                        .size(self.size * 0.86),
                );
            });
        ui.add_space(self.size * 0.7);
    }

    fn table_ui(&mut self, ui: &mut Ui, table: Table) {
        self.tables += 1;
        let size = self.size;
        let c = self.c;
        let mut clicked = None;
        egui::Frame::new()
            .stroke(Stroke::new(1.0, c.border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                egui::Grid::new(("md_table", self.tables)).spacing([24.0, 8.0]).show(ui, |ui| {
                    for (r, row) in table.rows.iter().enumerate() {
                        for cell in row {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                for span in cell {
                                    if let Some(url) = span_ui(ui, span, size * 0.95, r == 0, false, &c) {
                                        clicked = Some(url);
                                    }
                                }
                            });
                        }
                        ui.end_row();
                    }
                });
            });
        if clicked.is_some() {
            self.clicked = clicked;
        }
        ui.add_space(self.size * 0.7);
    }
}

fn span_ui(ui: &mut Ui, span: &Span, size: f32, heading: bool, quote: bool, c: &Palette) -> Option<String> {
    let st = &span.style;
    let mut rt = RichText::new(&span.text).size(size);
    if heading || st.strong {
        rt = rt.family(theme::semibold());
    }
    if heading {
        rt = rt.color(c.yellow);
    }
    if st.emph || quote {
        rt = rt.italics();
    }
    if quote {
        rt = rt.color(c.gray);
    }
    if st.strike {
        rt = rt.strikethrough();
    }
    if let Some(color) = st.color {
        rt = rt.color(color);
    }
    if st.code {
        rt = rt.family(FontFamily::Monospace).size(size * 0.86).color(c.green).background_color(c.code_bg);
    }
    match &st.link {
        Some(url) => {
            let hover = url.strip_prefix("wiki:").unwrap_or(url);
            ui.link(rt.color(c.blue)).on_hover_text(hover).clicked().then(|| url.clone())
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
        let src = "# Title **x**\n- [ ] task with `code` and [[Link|alias]]\n> quote *em*\n```rust\nlet a = 1;\n```\n1. [md](http://x) ![](a.png) #tag ~~gone~~ é**ü**\n**unclosed and [broken\n";
        let job = highlight(src, &Visuals::dark(), 14.0);
        assert_eq!(job.text, src);
    }
}
