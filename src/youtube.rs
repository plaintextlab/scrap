//! YouTube links shown as video cards: the video's thumbnail with a play button.
//!
//! Thumbnails are fetched once with the system `curl` (like the updater) into a cache in
//! the temp folder and loaded from there, so no HTTP client is built into the app.

use crate::theme;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Ui};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Width-to-height of a video card.
pub const ASPECT: f32 = 16.0 / 9.0;
/// Widest a card is drawn.
pub const MAX_WIDTH: f32 = 560.0;

/// The video id in a YouTube link: `youtube.com/watch?v=ID`, `youtu.be/ID`,
/// `youtube.com/shorts/ID`, `/embed/ID` or `/live/ID` (any subdomain, http or https).
pub fn video_id(url: &str) -> Option<&str> {
    let url = url.trim().trim_start_matches('<').trim_end_matches('>');
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").or_else(|| host.strip_prefix("m.")).unwrap_or(&host);
    let id = match host {
        "youtu.be" => path.split(['?', '#', '/']).next(),
        "youtube.com" | "music.youtube.com" | "youtube-nocookie.com" => {
            if let Some(query) = path.strip_prefix("watch?") {
                query.split(['&', '#']).find_map(|kv| kv.strip_prefix("v="))
            } else {
                ["shorts/", "embed/", "live/", "v/"]
                    .iter()
                    .find_map(|p| path.strip_prefix(p))
                    .and_then(|s| s.split(['?', '#', '/']).next())
            }
        }
        _ => None,
    }?;
    let valid = id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    valid.then_some(id)
}

/// Size of a card that fits `width`.
pub fn card_size(width: f32) -> egui::Vec2 {
    let w = width.clamp(0.0, MAX_WIDTH);
    egui::vec2(w, (w / ASPECT).round())
}

enum Thumb {
    Loading,
    Ready(String),
    Failed,
}

fn cache() -> &'static Mutex<HashMap<String, Thumb>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Thumb>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn cache_dir() -> PathBuf {
    std::env::temp_dir().join("scrap-thumbnails")
}

/// The thumbnail's image URI once it's downloaded; starts the download the first time.
pub fn thumbnail(ctx: &egui::Context, id: &str) -> Option<String> {
    let mut thumbs = cache().lock().unwrap_or_else(|e| e.into_inner());
    match thumbs.get(id) {
        Some(Thumb::Ready(uri)) => return Some(uri.clone()),
        Some(_) => return None,
        None => {}
    }
    let file = cache_dir().join(format!("{id}.jpg"));
    if file.is_file() {
        let uri = crate::attach::file_uri(&file);
        thumbs.insert(id.to_string(), Thumb::Ready(uri.clone()));
        return Some(uri);
    }
    thumbs.insert(id.to_string(), Thumb::Loading);
    drop(thumbs);
    let (id, ctx) = (id.to_string(), ctx.clone());
    std::thread::spawn(move || {
        let state = match download(&id, &file) {
            Ok(()) => Thumb::Ready(crate::attach::file_uri(&file)),
            Err(_) => Thumb::Failed,
        };
        cache().lock().unwrap_or_else(|e| e.into_inner()).insert(id, state);
        ctx.request_repaint();
    });
    None
}

fn download(id: &str, file: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(cache_dir()).map_err(|e| e.to_string())?;
    let partial = file.with_extension("part");
    let url = format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg");
    crate::update::curl(&["-fsSL", "-o", &partial.to_string_lossy(), &url])?;
    std::fs::rename(&partial, file).map_err(|e| e.to_string())
}

/// Draws a card in `rect`: the thumbnail (or a placeholder while it loads) and a play button.
pub fn paint(ui: &Ui, rect: Rect, thumbnail: Option<&str>, hovered: bool) {
    let p = theme::palette(ui.ctx());
    let painter = ui.painter_at(rect);
    let radius = CornerRadius::same(10);
    painter.rect_filled(rect, radius, p.code_bg);
    if let Some(uri) = thumbnail {
        // hqdefault is 4:3 with black bars above and below a 16:9 video; show the middle.
        egui::Image::new(uri)
            .uv(Rect::from_min_max(Pos2::new(0.0, 0.125), Pos2::new(1.0, 0.875)))
            .corner_radius(radius)
            .paint_at(ui, rect);
    }
    painter.rect_stroke(rect, radius, Stroke::new(1.0, p.border), egui::StrokeKind::Inside);

    // A rounded red button with a white triangle, like YouTube's own.
    let h = (rect.height() * 0.2).clamp(28.0, 56.0);
    let button = Rect::from_center_size(rect.center(), egui::vec2(h * 1.42, h));
    let red = if hovered { Color32::from_rgb(255, 0, 0) } else { Color32::from_rgb(204, 0, 0) };
    painter.rect_filled(button, CornerRadius::same((h * 0.28) as u8), red);
    let c = button.center();
    let t = h * 0.22;
    painter.add(egui::Shape::convex_polygon(
        vec![Pos2::new(c.x - t * 0.8, c.y - t), Pos2::new(c.x - t * 0.8, c.y + t), Pos2::new(c.x + t, c.y)],
        Color32::WHITE,
        Stroke::NONE,
    ));
}

/// A clickable card for the Read view, sized to the available width. Returns true when clicked.
pub fn card(ui: &mut Ui, url: &str, id: &str) -> bool {
    let size = card_size(ui.available_width());
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let thumb = thumbnail(ui.ctx(), id);
        paint(ui, rect, thumb.as_deref(), resp.hovered());
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(url).clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_video_ids() {
        let id = Some("dQw4w9WgXcQ");
        assert_eq!(video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"), id);
        assert_eq!(video_id("https://youtube.com/watch?feature=share&v=dQw4w9WgXcQ&t=42"), id);
        assert_eq!(video_id("https://m.youtube.com/watch?v=dQw4w9WgXcQ#t=1"), id);
        assert_eq!(video_id("https://youtu.be/dQw4w9WgXcQ?si=abc"), id);
        assert_eq!(video_id("http://www.youtube.com/shorts/dQw4w9WgXcQ"), id);
        assert_eq!(video_id("https://www.youtube.com/embed/dQw4w9WgXcQ"), id);
        assert_eq!(video_id("<https://youtu.be/dQw4w9WgXcQ>"), id);
        assert_eq!(video_id("https://www.youtube.com/@channel"), None);
        assert_eq!(video_id("https://youtu.be/short"), None);
        assert_eq!(video_id("https://example.com/watch?v=dQw4w9WgXcQ"), None);
        assert_eq!(video_id("youtube.com/watch?v=dQw4w9WgXcQ"), None);
    }
}
