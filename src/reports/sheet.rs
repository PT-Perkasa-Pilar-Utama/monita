//! A report's layout on A4: a cursor that moves down the page, breaks to a new page when the
//! next block does not fit, repeats a table's header on each page, and numbers the pages.

use crate::platform::pdf::{Font, PAGE_H, PAGE_W, Pdf, Rgb, fit, wrap};

const MARGIN: f32 = 48.0;
const BOTTOM: f32 = PAGE_H - 56.0;
pub const TEXT_W: f32 = PAGE_W - 2.0 * MARGIN;

const INK: Rgb = Rgb(0.11, 0.12, 0.13);
const MUTED: Rgb = Rgb(0.40, 0.42, 0.45);
const RULE: Rgb = Rgb(0.85, 0.86, 0.87);

/// A table column: its heading, its share of the text width, and whether its text wraps
/// (otherwise it is cut to fit) and sits on the right.
pub struct Col {
    pub head: &'static str,
    pub share: f32,
    pub wraps: bool,
    pub right: bool,
}

impl Col {
    pub const fn left(head: &'static str, share: f32) -> Self {
        Self {
            head,
            share,
            wraps: false,
            right: false,
        }
    }
    pub const fn right(head: &'static str, share: f32) -> Self {
        Self {
            head,
            share,
            wraps: false,
            right: true,
        }
    }
    pub const fn wrapping(head: &'static str, share: f32) -> Self {
        Self {
            head,
            share,
            wraps: true,
            right: false,
        }
    }
}

pub struct Sheet {
    pdf: Pdf,
    y: f32,
    footer: String,
}

impl Sheet {
    /// `footer` prints at the foot of every page, before the page number.
    pub fn new(title: &str, footer: &str) -> Self {
        Self {
            pdf: Pdf::new(title),
            y: MARGIN,
            footer: footer.to_string(),
        }
    }

    /// Starts a new page when `h` more points do not fit on this one.
    fn room(&mut self, h: f32) {
        if self.y + h > BOTTOM {
            self.pdf.new_page();
            self.y = MARGIN;
        }
    }

    /// The report's title, wrapped: a long incident title is never cut.
    pub fn title(&mut self, s: &str) {
        for line in wrap(s, Font::Bold, 20.0, TEXT_W) {
            self.room(26.0);
            self.y += 24.0;
            self.pdf.text(MARGIN, self.y, 20.0, Font::Bold, INK, &line);
        }
        self.y += 6.0;
    }

    /// Plain text in the muted colour, wrapped to the width.
    pub fn note(&mut self, s: &str) {
        for line in wrap(s, Font::Regular, 9.5, TEXT_W) {
            self.room(14.0);
            self.y += 13.0;
            self.pdf
                .text(MARGIN, self.y, 9.5, Font::Regular, MUTED, &line);
        }
    }

    /// Plain text in the ink colour, wrapped to the width.
    pub fn para(&mut self, s: &str) {
        for line in wrap(s, Font::Regular, 10.0, TEXT_W) {
            self.room(15.0);
            self.y += 14.0;
            self.pdf
                .text(MARGIN, self.y, 10.0, Font::Regular, INK, &line);
        }
    }

    /// A section heading, kept on the page with at least two lines under it.
    pub fn heading(&mut self, s: &str) {
        self.room(60.0);
        self.y += 28.0;
        self.pdf.text(MARGIN, self.y, 13.0, Font::Bold, INK, s);
        self.y += 4.0;
    }

    /// A bold line that names the block under it, such as one incident.
    pub fn subheading(&mut self, s: &str) {
        let lines = wrap(s, Font::Bold, 10.5, TEXT_W);
        self.room(16.0 * lines.len() as f32 + 30.0);
        self.y += 8.0;
        for line in lines {
            self.y += 14.0;
            self.pdf.text(MARGIN, self.y, 10.5, Font::Bold, INK, &line);
        }
    }

    /// One bar per value across the text width, each in its colour, with the first and the
    /// last label under the row.
    pub fn bars(&mut self, colours: &[Rgb], first: &str, last: &str) {
        const H: f32 = 28.0;
        self.room(H + 30.0);
        self.y += 10.0;
        let n = colours.len().max(1) as f32;
        let step = TEXT_W / n;
        let gap = (step * 0.2).min(2.0);
        for (i, c) in colours.iter().enumerate() {
            self.pdf
                .rect(MARGIN + i as f32 * step, self.y, step - gap, H, *c);
        }
        self.y += H + 12.0;
        self.pdf
            .text(MARGIN, self.y, 8.5, Font::Regular, MUTED, first);
        let w = crate::platform::pdf::width(last, Font::Regular, 8.5);
        self.pdf
            .text(MARGIN + TEXT_W - w, self.y, 8.5, Font::Regular, MUTED, last);
    }

    /// Up to four numbers in a row: each value large, its name under it.
    pub fn figures(&mut self, items: &[(&str, String)]) {
        self.room(52.0);
        let w = TEXT_W / items.len().max(1) as f32;
        for (i, (label, value)) in items.iter().enumerate() {
            let x = MARGIN + i as f32 * w;
            let v = fit(value, Font::Bold, 18.0, w - 8.0);
            self.pdf.text(x, self.y + 28.0, 18.0, Font::Bold, INK, &v);
            let l = fit(label, Font::Regular, 9.0, w - 8.0);
            self.pdf
                .text(x, self.y + 42.0, 9.0, Font::Regular, MUTED, &l);
        }
        self.y += 50.0;
    }

    /// Label and value pairs, one a line.
    pub fn facts(&mut self, items: &[(&str, String)]) {
        let label_w = 110.0;
        for (label, value) in items {
            let lines = wrap(value, Font::Regular, 10.0, TEXT_W - label_w);
            self.room(15.0 * lines.len() as f32);
            self.y += 15.0;
            self.pdf
                .text(MARGIN, self.y, 10.0, Font::Regular, MUTED, label);
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    self.y += 14.0;
                }
                self.pdf
                    .text(MARGIN + label_w, self.y, 10.0, Font::Regular, INK, line);
            }
        }
    }

    /// A table with a rule under the header and between rows. The header repeats on each page.
    pub fn table(&mut self, cols: &[Col], rows: &[Vec<String>]) {
        const SIZE: f32 = 9.0;
        const LINE: f32 = 12.0;
        const PAD: f32 = 6.0;
        let widths: Vec<f32> = cols.iter().map(|c| c.share * TEXT_W).collect();
        let header = |sheet: &mut Self| {
            sheet.y += 16.0;
            let mut x = MARGIN;
            for (c, w) in cols.iter().zip(&widths) {
                let at = if c.right {
                    x + w - 2.0 * PAD - crate::platform::pdf::width(c.head, Font::Bold, SIZE)
                } else {
                    x
                };
                sheet.pdf.text(at, sheet.y, SIZE, Font::Bold, MUTED, c.head);
                x += w;
            }
            sheet.y += 5.0;
            sheet.pdf.hline(MARGIN, sheet.y, TEXT_W, 0.75, MUTED);
        };
        self.room(21.0 + LINE + PAD);
        header(self);
        for row in rows {
            let cells: Vec<Vec<String>> = cols
                .iter()
                .zip(&widths)
                .zip(row)
                .map(|((c, w), text)| {
                    if c.wraps {
                        wrap(text, Font::Regular, SIZE, w - PAD)
                    } else {
                        vec![fit(text, Font::Regular, SIZE, w - PAD)]
                    }
                })
                .collect();
            let lines = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
            // A row starts where at least two of its lines fit. A row taller than the rest of
            // the page goes on line by line, under the header again on the next page.
            let first = lines.min(2) as f32 * LINE + PAD;
            if self.y + first > BOTTOM {
                self.pdf.new_page();
                self.y = MARGIN;
                header(self);
            }
            for i in 0..lines {
                if self.y + LINE > BOTTOM {
                    self.pdf.new_page();
                    self.y = MARGIN;
                    header(self);
                }
                self.y += LINE;
                let mut x = MARGIN;
                for ((c, w), cell) in cols.iter().zip(&widths).zip(&cells) {
                    if let Some(line) = cell.get(i) {
                        let at = if c.right {
                            x + w
                                - 2.0 * PAD
                                - crate::platform::pdf::width(line, Font::Regular, SIZE)
                        } else {
                            x
                        };
                        self.pdf.text(at, self.y, SIZE, Font::Regular, INK, line);
                    }
                    x += w;
                }
            }
            self.y += PAD;
            self.pdf.hline(MARGIN, self.y, TEXT_W, 0.5, RULE);
        }
    }

    /// The file, with the footer and "Page 1 of 3" on every page.
    pub fn finish(mut self) -> anyhow::Result<Vec<u8>> {
        let n = self.pdf.page_count();
        for i in 0..n {
            self.pdf.select(i);
            // The page number always prints; a long project name is cut before it.
            let page = format!("   Page {} of {n}", i + 1);
            let room = TEXT_W - crate::platform::pdf::width(&page, Font::Regular, 8.0);
            let foot = format!("{}{page}", fit(&self.footer, Font::Regular, 8.0, room));
            self.pdf
                .text(MARGIN, PAGE_H - 30.0, 8.0, Font::Regular, MUTED, &foot);
        }
        self.pdf.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;

    fn text_of(pdf: &[u8]) -> String {
        let mut out = String::new();
        let mut at = 0;
        let find = |hay: &[u8], n: &[u8]| hay.windows(n.len()).position(|w| w == n);
        while let Some(i) = find(&pdf[at..], b">>\nstream\n") {
            let start = at + i + 10;
            let end = start + find(&pdf[start..], b"\nendstream").unwrap();
            flate2::read::ZlibDecoder::new(&pdf[start..end])
                .read_to_string(&mut out)
                .unwrap();
            at = end + 10;
        }
        out
    }

    #[test]
    fn a_long_table_breaks_across_pages_and_repeats_its_header() {
        let mut sheet = Sheet::new("Long", "Payments");
        sheet.title("Payments report");
        let rows: Vec<Vec<String>> = (0..120)
            .map(|i| vec![format!("Monitor {i}"), "99.00%".into()])
            .collect();
        sheet.table(
            &[Col::left("Name", 0.7), Col::right("Availability", 0.3)],
            &rows,
        );
        let pages = sheet.pdf.page_count();
        assert!(pages >= 3, "{pages} pages");
        let text = text_of(&sheet.finish().unwrap());
        assert_eq!(
            text.matches("(Name)").count(),
            pages,
            "the header on every page"
        );
        assert!(text.contains(&format!("(Payments   Page {pages} of {pages})")));
        assert!(text.contains("(Monitor 119)"), "no row is lost at a break");
    }

    #[test]
    fn a_long_project_name_never_pushes_the_page_number_off_the_footer() {
        let sheet = Sheet::new("Long", &"WIDE PROJECT NAME ".repeat(8));
        let text = text_of(&sheet.finish().unwrap());
        assert!(text.contains("...   Page 1 of 1)"), "{text}");
    }

    #[test]
    fn a_row_taller_than_a_page_goes_on_over_the_next_pages() {
        let mut sheet = Sheet::new("Long", "Payments");
        let words: Vec<String> = (0..1500).map(|i| format!("w{i}")).collect();
        sheet.table(
            &[Col::left("Time", 0.3), Col::wrapping("Message", 0.7)],
            &[vec!["10:00".into(), words.join(" ")]],
        );
        let pages = sheet.pdf.page_count();
        assert!(pages >= 2, "{pages} pages");
        let text = text_of(&sheet.finish().unwrap());
        assert!(text.contains("w1499"), "the end of the message is drawn");
        // Every line sits above the footer: no Td lower than the bottom margin.
        for y in text
            .split(" Td")
            .filter_map(|t| t.rsplit(' ').next()?.parse::<f32>().ok())
        {
            assert!(
                !(40.0..PAGE_H - BOTTOM - 1.0).contains(&y),
                "a line at {y} runs into the footer"
            );
        }
    }
}
