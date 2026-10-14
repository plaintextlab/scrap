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
    let format = image::guess_format(&bytes).map_err(|_| format!("{} isn't an image Scrap can read", src.display()))?;
    let stamp = time::stamp(time::now_millis());

    // GIFs keep their animation; everything else is re-encoded at a sensible size.
    if format == ImageFormat::Gif {
        let rel = attachments::free_name(vault, &stamp, "gif").map_err(|e| e.to_string())?;
        std::fs::write(vault.join(&rel), &bytes).map_err(|e| e.to_string())?;
        return Ok(rel);
    }
    let img = decode(&bytes).map_err(|e| format!("Couldn't decode {}: {e}", src.display()))?;
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
    fn file_uris() {
        assert_eq!(file_uri(Path::new("/home/me/a b.png")), "file:///home/me/a b.png");
        assert_eq!(file_uri(Path::new(r"C:\notes\.attachments\a.jpg")), "file:///C:/notes/.attachments/a.jpg");
    }
}
