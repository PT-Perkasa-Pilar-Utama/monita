//! A project's icon as the browser sends it: base64 of a PNG or WebP that the page already
//! scaled down. The server checks the type and the size; it never decodes the picture.

use base64::{Engine, engine::general_purpose::STANDARD};

use super::ProjectError;

/// The most bytes a stored icon may hold. The page sends a 128 px square, a few KB.
pub const MAX_BYTES: usize = 32 * 1024;

pub struct Icon {
    pub mime: &'static str,
    pub data: Vec<u8>,
}

/// `None` for an empty field. A `data:` URL prefix is allowed. Anything but PNG or WebP, such
/// as SVG, is refused, because the icon is served back to browsers.
// ponytail: icon dimensions are not read here; the browser resizes. Parse the PNG IHDR or the
// WebP VP8X header if oversized pictures show up.
pub fn parse(field: &str) -> Result<Option<Icon>, ProjectError> {
    let field = field.trim();
    if field.is_empty() {
        return Ok(None);
    }
    let b64 = field.split_once(";base64,").map_or(field, |(_, b)| b);
    // Base64 is 4 characters per 3 bytes; a longer field is over the cap before decoding.
    if b64.len() > MAX_BYTES.div_ceil(3) * 4 {
        return Err(ProjectError::BadIcon("The icon must be 32 KB or less."));
    }
    let data = STANDARD
        .decode(b64)
        .map_err(|_| ProjectError::BadIcon("The icon did not arrive whole. Pick it again."))?;
    if data.len() > MAX_BYTES {
        return Err(ProjectError::BadIcon("The icon must be 32 KB or less."));
    }
    let mime = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if data.len() > 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "image/webp"
    } else {
        return Err(ProjectError::BadIcon("Use a PNG, JPEG or WebP picture."));
    };
    Ok(Some(Icon { mime, data }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

    #[test]
    fn a_png_or_webp_is_kept_with_its_type_and_an_empty_field_is_no_icon() {
        assert!(parse("  ").unwrap().is_none());
        let png = parse(&format!("data:image/png;base64,{PNG_1X1}"))
            .unwrap()
            .unwrap();
        assert_eq!((png.mime, png.data.len()), ("image/png", 70));
        let webp = STANDARD.encode(b"RIFF\x10\x00\x00\x00WEBPVP8 ");
        assert_eq!(parse(&webp).unwrap().unwrap().mime, "image/webp");
    }

    #[test]
    fn svg_bad_base64_and_oversized_icons_are_refused() {
        let svg = STANDARD.encode(b"<svg xmlns='http://www.w3.org/2000/svg'/>");
        assert!(matches!(parse(&svg), Err(ProjectError::BadIcon(_))));
        assert!(matches!(
            parse("not base64!"),
            Err(ProjectError::BadIcon(_))
        ));
        let mut big = b"\x89PNG\r\n\x1a\n".to_vec();
        big.resize(MAX_BYTES + 1, 0);
        assert!(matches!(
            parse(&STANDARD.encode(big)),
            Err(ProjectError::BadIcon(_))
        ));
    }
}
