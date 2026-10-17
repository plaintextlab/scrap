//! Adding images to notes: they're copied into the vault's `.attachments` folder,
//! downscaled and rotated upright the same way the Android app does it.

use crate::time;
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use scrap_core::attachments;
use std::io::Cursor;
use std::path::Path;

/// Longest side of a stored image; keeps photos from bloating the repository.
const MAX_SIDE: u32 = 2048;

pub const EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "jfif"];

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// `file:///` URI for a local path, in the form egui's loaders and the OS expect on every
/// platform (`file:///C:/dir/a.png` on Windows, `file:///home/me/a.png` elsewhere).
pub fn file_uri(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

/// Copies the image at `src` into the vault and returns its vault-relative path.
pub fn import(vault: &Path, src: &Path) -> Result<String, String> {
    let bytes = std::fs::read(src).map_err(|e| format!("Couldn't read {}: {e}", src.display()))?;
    import_bytes(vault, &bytes, &src.display().to_string())
}

/// Saves the image on the clipboard into the vault, if there is one. Screenshots stay PNG
/// so text in them stays sharp. `Ok(None)` means the clipboard holds no image (or holds text).
pub fn import_clipboard(vault: &Path) -> Result<Option<String>, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Couldn't open the clipboard: {e}"))?;
    // Text wins when there's both (an Excel range, say): egui has already pasted it.
    if clipboard.get_text().is_ok_and(|t| !t.is_empty()) {
        return Ok(None);
    }
    let Ok(data) = clipboard.get_image() else { return Ok(None) };
    import_rgba(vault, data.width as u32, data.height as u32, data.bytes.into_owned()).map(Some)
}

/// Saves raw RGBA pixels (as the clipboard gives them) as a PNG in the vault.
fn import_rgba(vault: &Path, width: u32, height: u32, pixels: Vec<u8>) -> Result<String, String> {
    let img = image::RgbaImage::from_raw(width, height, pixels).ok_or("The pasted image is malformed")?;
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(img).write_to(&mut Cursor::new(&mut png), ImageFormat::Png).map_err(|e| e.to_string())?;
    import_bytes(vault, &png, "the pasted image")
}

/// Stores an encoded image in the vault; `label` names it in errors.
fn import_bytes(vault: &Path, bytes: &[u8], label: &str) -> Result<String, String> {
    let format = image::guess_format(bytes).map_err(|_| format!("{label} isn't an image Scrap can read"))?;
    let stamp = time::stamp(time::now_millis());

    // GIFs keep their animation; everything else is re-encoded at a sensible size.
    if format == ImageFormat::Gif {
        let rel = attachments::free_name(vault, &stamp, "gif").map_err(|e| e.to_string())?;
        std::fs::write(vault.join(&rel), bytes).map_err(|e| e.to_string())?;
        return Ok(rel);
    }
    let img = decode(bytes).map_err(|e| format!("Couldn't decode {label}: {e}"))?;
    let img = if img.width().max(img.height()) > MAX_SIDE {
        img.resize(MAX_SIDE, MAX_SIDE, FilterType::Triangle)
    } else {
        img
    };
    let alpha = img.color().has_alpha() && format == ImageFormat::Png;
    let ext = if alpha { "png" } else { "jpg" };
    let rel = attachments::free_name(vault, &stamp, ext).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    if alpha {
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png).map_err(|e| e.to_string())?;
    } else {
        img.to_rgb8().write_with_encoder(JpegEncoder::new_with_quality(&mut out, 85)).map_err(|e| e.to_string())?;
    }
    // Write next to the target and rename, so a sync never picks up half a file.
    let target = vault.join(&rel);
    let partial = target.with_extension(format!("{ext}.part"));
    std::fs::write(&partial, out).map_err(|e| e.to_string())?;
    std::fs::rename(&partial, &target).map_err(|e| e.to_string())?;
    Ok(rel)
}

/// Decodes and applies the EXIF orientation, so phone photos come out upright.
fn decode(bytes: &[u8]) -> image::ImageResult<DynamicImage> {
    let mut decoder = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_and_downscales() {
        let dir = std::env::temp_dir().join(format!("scrap-attach-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("big.png");
        image::RgbImage::from_pixel(3000, 1000, image::Rgb([200, 10, 10])).save(&src).unwrap();

        let vault = dir.join("vault");
        let rel = import(&vault, &src).unwrap();
        assert!(rel.starts_with(".attachments/IMG-") && rel.ends_with(".jpg"), "{rel}");
        let saved = image::open(vault.join(&rel)).unwrap();
        assert_eq!((saved.width(), saved.height()), (2048, 683));
        // A second import in the same second gets its own name.
        let again = import(&vault, &src).unwrap();
        assert_ne!(rel, again);
        assert!(import(&vault, &dir.join("missing.png")).is_err());
    }

    #[test]
    fn pasted_pixels_become_a_png() {
        let vault = std::env::temp_dir().join(format!("scrap-paste-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&vault);
        let pixels = [10u8, 20, 30, 255].repeat(40 * 30);
        let rel = import_rgba(&vault, 40, 30, pixels).unwrap();
        assert!(rel.starts_with(".attachments/IMG-") && rel.ends_with(".png"), "{rel}");
        let saved = image::open(vault.join(&rel)).unwrap().to_rgba8();
        assert_eq!((saved.width(), saved.height()), (40, 30));
        assert_eq!(saved.get_pixel(5, 5).0, [10, 20, 30, 255]);
        assert!(import_rgba(&vault, 40, 30, vec![0; 7]).is_err());
        let _ = std::fs::remove_dir_all(&vault);
    }

    #[test]
    fn file_uris() {
        assert_eq!(file_uri(Path::new("/home/me/a b.png")), "file:///home/me/a b.png");
        assert_eq!(file_uri(Path::new(r"C:\notes\.attachments\a.jpg")), "file:///C:/notes/.attachments/a.jpg");
    }
}
