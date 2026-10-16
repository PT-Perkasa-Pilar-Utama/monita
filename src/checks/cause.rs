//! The cause text a `/fail` ping keeps from its body. Spec: api-specs 04 section 4.2.

/// The cause a `/fail` ping keeps: its first 1 024 bytes after leading whitespace, cut on a
/// character boundary. A longer body is cut, never refused, so the failure itself is always
/// recorded (api-specs 04 section 4.2). Fed chunk by chunk, so it never holds more than that.
#[derive(Default)]
pub struct Cause {
    kept: Vec<u8>,
    /// Bytes of a character split across chunks while still in the leading whitespace.
    pending: Vec<u8>,
    started: bool,
}

impl Cause {
    const KEEP: usize = 1024;

    pub fn push(&mut self, chunk: &[u8]) {
        let joined;
        let mut chunk = chunk;
        if !self.started {
            // Leading Unicode whitespace is skipped before it can use up the budget.
            joined = [std::mem::take(&mut self.pending).as_slice(), chunk].concat();
            let valid = match std::str::from_utf8(&joined) {
                Ok(s) => s,
                Err(e) => std::str::from_utf8(&joined[..e.valid_up_to()]).unwrap_or_default(),
            };
            let skip = valid.len() - valid.trim_start().len();
            let rest = &joined[skip..];
            let cut_char = matches!(std::str::from_utf8(rest), Err(e) if e.error_len().is_none());
            if skip == valid.len() && cut_char {
                // Only whitespace so far, then a character cut at the chunk's end: wait for it.
                self.pending = rest.to_vec();
                return;
            }
            if rest.is_empty() {
                return;
            }
            self.started = true;
            chunk = rest;
        }
        let room = Self::KEEP.saturating_sub(self.kept.len());
        self.kept.extend_from_slice(&chunk[..chunk.len().min(room)]);
    }

    /// True once the cause has all it keeps; the rest of the body can be dropped unread.
    pub fn is_full(&self) -> bool {
        self.kept.len() >= Self::KEEP
    }

    pub fn finish(mut self) -> Option<String> {
        // A character cut at the 1 KB mark is dropped, not turned into U+FFFD.
        if let Err(e) = std::str::from_utf8(&self.kept)
            && e.error_len().is_none()
        {
            self.kept.truncate(e.valid_up_to());
        }
        let text = String::from_utf8_lossy(&self.kept);
        let text = text.trim();
        let mut end = text.len().min(1024);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        (end > 0).then(|| text[..end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Cause;

    fn cause(chunks: &[&[u8]]) -> Option<String> {
        let mut c = Cause::default();
        for chunk in chunks {
            c.push(chunk);
        }
        c.finish()
    }

    #[test]
    fn a_cause_is_cut_to_1_kb_on_a_character_boundary() {
        assert_eq!(cause(&[b"disk full"]).as_deref(), Some("disk full"));
        let long = "x".repeat(1023) + "\u{e9}";
        assert_eq!(
            cause(&[long.as_bytes()]).unwrap().len(),
            1023,
            "the two-byte character goes whole"
        );
        assert_eq!(cause(&[&[b'y'; 5000]]).unwrap().len(), 1024);
        let four_byte = "x".repeat(1021) + "\u{1f525}";
        assert_eq!(
            cause(&[four_byte.as_bytes()]).as_deref(),
            Some("x".repeat(1021).as_str()),
            "a character cut at 1 KB is dropped, not replaced"
        );
    }

    #[test]
    fn leading_whitespace_does_not_use_up_the_cause() {
        let blank = vec![b'\n'; 1100];
        assert_eq!(cause(&[&blank, b"Error: x"]).as_deref(), Some("Error: x"));
        assert_eq!(cause(&[b"  \n "]), None, "a blank body has no cause");
        let unicode = "\u{a0}\u{2003}\x0bError: y".as_bytes();
        assert_eq!(cause(&[unicode]).as_deref(), Some("Error: y"));
        let wide = "\u{2003}".repeat(600) + "Error: db down";
        let (head, tail) = wide.as_bytes().split_at(1000); // splits one em space across chunks
        assert_eq!(cause(&[head, tail]).as_deref(), Some("Error: db down"));
        assert_eq!(
            cause(&[b"\n\xFFab"]).as_deref(),
            Some("\u{fffd}ab"),
            "an invalid byte is kept, not held as a cut character"
        );
    }
}
