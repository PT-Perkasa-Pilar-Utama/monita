//! A small picture as the browser sends it (a project icon, a profile picture): base64 of a PNG
//! or WebP that the page already scaled down. The server checks the type and the size; it never
//! decodes the picture.

use base64::{Engine, engine::general_purpose::STANDARD};

/// The most bytes a stored picture may hold. The page sends a 128 px square, a few KB.
pub const MAX_BYTES: usize = 32 * 1024;

pub struct Image {
    pub mime: &'static str,
    pub data: Vec<u8>,
}

/// `None` for an empty field. A `data:` URL prefix is allowed. Anything but PNG or WebP, such
/// as SVG, is refused, because the picture is served back to browsers. The error is the form
/// field's message.
// ponytail: picture dimensions are not read here; the browser resizes. Parse the PNG IHDR or the
// WebP VP8X header if oversized pictures show up.
pub fn parse(field: &str) -> Result<Option<Image>, &'static str> {
    let field = field.trim();
    if field.is_empty() {
        return Ok(None);
    }
    let b64 = field.split_once(";base64,").map_or(field, |(_, b)| b);
    // Base64 is 4 characters per 3 bytes; a longer field is over the cap before decoding.
    if b64.len() > MAX_BYTES.div_ceil(3) * 4 {
        return Err("The picture must be 32 KB or less.");
    }
    let data = STANDARD
        .decode(b64)
        .map_err(|_| "The picture did not arrive whole, so pick it again.")?;
    if data.len() > MAX_BYTES {
        return Err("The picture must be 32 KB or less.");
    }
    let mime = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if data.len() > 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "image/webp"
    } else {
        return Err("Use a PNG, JPEG or WebP picture.");
    };
    Ok(Some(Image { mime, data }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

    #[test]
    fn a_png_or_webp_is_kept_with_its_type_and_an_empty_field_is_no_picture() {
        assert!(parse("  ").unwrap().is_none());
        let png = parse(&format!("data:image/png;base64,{PNG_1X1}"))
            .unwrap()
            .unwrap();
        assert_eq!((png.mime, png.data.len()), ("image/png", 70));
        let webp = STANDARD.encode(b"RIFF\x10\x00\x00\x00WEBPVP8 ");
        assert_eq!(parse(&webp).unwrap().unwrap().mime, "image/webp");
    }

    #[test]
    fn svg_bad_base64_and_oversized_pictures_are_refused() {
        let svg = STANDARD.encode(b"<svg xmlns='http://www.w3.org/2000/svg'/>");
        assert!(parse(&svg).is_err());
        assert!(parse("not base64!").is_err());
        let mut big = b"\x89PNG\r\n\x1a\n".to_vec();
        big.resize(MAX_BYTES + 1, 0);
        assert!(parse(&STANDARD.encode(big)).is_err());
    }
}
