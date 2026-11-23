//! A small PDF writer for reports (v1.2 Q58): A4 pages of text, lines and filled rectangles in
//! the built-in Helvetica fonts, so no font file is embedded and no crate is added.
//!
//! ponytail: WinAnsi printable ASCII only; any other character prints as `?`. Swap in a
//! font-embedding crate that passes `cargo deny` when reports need other scripts.

use std::fmt::Write as _;
use std::io::Write as _;

/// A4 in points (1/72 inch).
pub const PAGE_W: f32 = 595.0;
pub const PAGE_H: f32 = 842.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    Regular,
    Bold,
}

/// A colour as red, green and blue from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb(pub f32, pub f32, pub f32);

// Advance widths in 1/1000 em for codes 32 to 126, read from Adobe's Core 14 AFM files
// (Helvetica.afm and Helvetica-Bold.afm, as pdfkit 0.15.0 ships them in lib/font/data). Codes 39
// and 96 take the WinAnsi glyphs `quotesingle` and `grave`, not StandardEncoding's quotes.
const REGULAR: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];
const BOLD: [u16; 95] = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, 975, 722, 722, 722, 722, 667,
    611, 778, 722, 278, 556, 722, 611, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 333, 278, 333, 584, 556, 333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556,
    278, 889, 611, 611, 611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584,
];

/// The byte a character prints as: itself when it is printable ASCII, else `?`.
fn byte(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        _ => b'?',
    }
}

/// The width of `s` in points at `size`.
pub fn width(s: &str, font: Font, size: f32) -> f32 {
    let table = match font {
        Font::Regular => &REGULAR,
        Font::Bold => &BOLD,
    };
    let units: u32 = s
        .chars()
        .map(|c| u32::from(table[usize::from(byte(c) - b' ')]))
        .sum();
    units as f32 * size / 1000.0
}

/// `s` cut to fit `max` points, ending in `...` when cut.
pub fn fit(s: &str, font: Font, size: f32, max: f32) -> String {
    if width(s, font, size) <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for c in s.chars() {
        let next = format!("{out}{c}...");
        if width(&next, font, size) > max {
            break;
        }
        out.push(c);
    }
    format!("{}...", out.trim_end())
}

/// `s` broken into lines of at most `max` points at spaces and line breaks. A word longer than a
/// line is cut to fit.
pub fn wrap(s: &str, font: Font, size: f32, max: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in s.lines() {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let next = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if width(&next, font, size) <= max {
                line = next;
            } else {
                if !line.is_empty() {
                    lines.push(line);
                }
                line = fit(word, font, size, max);
            }
        }
        lines.push(line);
    }
    lines
}

/// A PDF string literal: `(`, `)` and `\` escaped, so text from users cannot end the string.
fn literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('(');
    for b in s.chars().map(byte) {
        if matches!(b, b'(' | b')' | b'\\') {
            out.push('\\');
        }
        out.push(char::from(b));
    }
    out.push(')');
    out
}

/// Pages drawn from the top left, in points. `finish` returns the file.
pub struct Pdf {
    title: String,
    pages: Vec<String>,
    /// The page that drawing goes to.
    current: usize,
}

impl Pdf {
    /// One empty page. `title` is the document's title in the viewer.
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            pages: vec![String::new()],
            current: 0,
        }
    }

    /// Adds a page and draws on it.
    pub fn new_page(&mut self) {
        self.pages.push(String::new());
        self.current = self.pages.len() - 1;
    }

    /// Draws on page `i` (from 0) again, for a footer that needs the page count.
    pub fn select(&mut self, i: usize) {
        self.current = i.min(self.pages.len() - 1);
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    fn ops(&mut self) -> &mut String {
        &mut self.pages[self.current]
    }

    /// Text whose baseline sits `y` points from the top of the page.
    pub fn text(&mut self, x: f32, y: f32, size: f32, font: Font, color: Rgb, s: &str) {
        let f = match font {
            Font::Regular => "F1",
            Font::Bold => "F2",
        };
        let Rgb(r, g, b) = color;
        let s = literal(s);
        let _ = writeln!(
            self.ops(),
            "BT /{f} {size:.1} Tf {r:.3} {g:.3} {b:.3} rg {x:.2} {:.2} Td {s} Tj ET",
            PAGE_H - y
        );
    }

    /// A filled rectangle whose top left corner is `(x, y)` from the top of the page.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Rgb) {
        let Rgb(r, g, b) = color;
        let _ = writeln!(
            self.ops(),
            "{r:.3} {g:.3} {b:.3} rg {x:.2} {:.2} {w:.2} {h:.2} re f",
            PAGE_H - y - h
        );
    }

    /// A horizontal line `w` points long at `y` from the top.
    pub fn hline(&mut self, x: f32, y: f32, w: f32, thickness: f32, color: Rgb) {
        let Rgb(r, g, b) = color;
        let y = PAGE_H - y;
        let _ = writeln!(
            self.ops(),
            "{r:.3} {g:.3} {b:.3} RG {thickness:.2} w {x:.2} {y:.2} m {:.2} {y:.2} l S",
            x + w
        );
    }

    /// The file: a catalog, the page tree, two fonts, then a page and its compressed content
    /// stream for each page, and the cross-reference table that points at every object.
    pub fn finish(self) -> anyhow::Result<Vec<u8>> {
        let n = self.pages.len();
        let mut objects: Vec<Vec<u8>> = Vec::with_capacity(5 + 2 * n);
        let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 6 + 2 * i)).collect();
        objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
        objects
            .push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes());
        for name in ["Helvetica", "Helvetica-Bold"] {
            objects.push(
                format!(
                    "<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding /WinAnsiEncoding >>"
                )
                .into_bytes(),
            );
        }
        objects
            .push(format!("<< /Title {} /Producer (Monita) >>", literal(&self.title)).into_bytes());
        for (i, ops) in self.pages.iter().enumerate() {
            objects.push(
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
                     /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                    7 + 2 * i
                )
                .into_bytes(),
            );
            let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            z.write_all(ops.as_bytes())?;
            let data = z.finish()?;
            let mut stream = format!(
                "<< /Length {} /Filter /FlateDecode >>\nstream\n",
                data.len()
            )
            .into_bytes();
            stream.extend_from_slice(&data);
            stream.extend_from_slice(b"\nendstream");
            objects.push(stream);
        }

        let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = Vec::with_capacity(objects.len());
        for (i, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
        for o in &offsets {
            let _ = writeln!(table, "{o:010} 00000 n ");
        }
        let _ = write!(
            table,
            "trailer\n<< /Size {} /Root 1 0 R /Info 5 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        );
        out.extend_from_slice(table.as_bytes());
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_from_users_cannot_end_its_string_and_other_scripts_print_as_question_marks() {
        assert_eq!(literal(r"a (b) \ c"), r"(a \(b\) \\ c)");
        assert_eq!(literal("Reykjavík ✓"), "(Reykjav?k ?)");
        assert_eq!(literal(")\nET"), r"(\)?ET)");
    }

    #[test]
    fn widths_come_from_the_font_metrics() {
        // "Hi" in Helvetica: H 722 + i 222 = 944 units, 9.44 pt at 10 pt.
        assert!((width("Hi", Font::Regular, 10.0) - 9.44).abs() < 1e-4);
        // Bold: H 722 + i 278.
        assert!((width("Hi", Font::Bold, 10.0) - 10.0).abs() < 1e-4);
        assert!((width("é", Font::Regular, 10.0) - width("?", Font::Regular, 10.0)).abs() < 1e-6);
    }

    #[test]
    fn long_text_is_cut_or_wrapped_to_the_width() {
        let cut = fit("Payments gateway in Jakarta", Font::Regular, 10.0, 60.0);
        assert!(
            cut.ends_with("...") && width(&cut, Font::Regular, 10.0) <= 60.0,
            "{cut}"
        );
        assert_eq!(fit("API", Font::Regular, 10.0, 60.0), "API");
        let lines = wrap(
            "one two three four five six\nseven",
            Font::Regular,
            10.0,
            50.0,
        );
        assert!(lines.len() >= 3, "{lines:?}");
        assert_eq!(lines.last().unwrap(), "seven", "a line break starts a line");
        assert!(lines.iter().all(|l| width(l, Font::Regular, 10.0) <= 50.0));
    }

    #[test]
    fn the_cross_reference_table_points_at_every_object() {
        let mut pdf = Pdf::new("Report");
        pdf.text(40.0, 60.0, 12.0, Font::Bold, Rgb(0.0, 0.0, 0.0), "Hello");
        pdf.new_page();
        pdf.rect(40.0, 40.0, 100.0, 10.0, Rgb(0.2, 0.6, 0.3));
        let out = pdf.finish().unwrap();
        let tail = |from: usize| String::from_utf8_lossy(&out[from..]).into_owned();
        assert!(out.starts_with(b"%PDF-1.4"));
        assert!(out.ends_with(b"%%EOF\n"));
        let at = out.windows(10).rposition(|w| w == b"startxref\n").unwrap();
        let start: usize = tail(at + 10).lines().next().unwrap().parse().unwrap();
        let table = tail(start);
        assert!(
            table.starts_with("xref\n0 10\n"),
            "2 pages: 5 + 2 * 2 objects"
        );
        for (i, line) in table.lines().skip(3).take(9).enumerate() {
            let at: usize = line[..10].parse().unwrap();
            assert!(
                out[at..].starts_with(format!("{} 0 obj", i + 1).as_bytes()),
                "object {} sits where the table says",
                i + 1
            );
        }
        assert!(tail(0).contains("/Count 2"));
    }
}
