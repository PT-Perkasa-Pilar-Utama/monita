//! A QR code as an SVG document, for a link a phone must open: a two-factor key, a Telegram
//! connect link.

use qrcode::{QrCode, render::svg};

/// Dark modules on white, 4 px each, so they land on whole pixels at the size pages show them.
/// A phone camera reads dark on light only, so the colours are fixed.
pub fn svg(text: &str) -> anyhow::Result<String> {
    let code = QrCode::new(text.as_bytes())?;
    Ok(code
        .render::<svg::Color<'_>>()
        .module_dimensions(4, 4)
        .dark_color(svg::Color("#000000"))
        .light_color(svg::Color("#ffffff"))
        .build())
}
