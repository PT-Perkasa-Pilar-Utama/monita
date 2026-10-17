//! The env file `monita setup` writes: `KEY=value` lines that systemd's `EnvironmentFile=` and a
//! POSIX shell read the same way.

use std::path::Path;

use anyhow::{Context, bail};

/// The assignments in `text`, in file order. Comments, blank lines and lines without `=` are
/// skipped.
pub fn parse(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            Some((k.trim().to_string(), unquote(v.trim())))
        })
        .filter(|(k, _)| !k.is_empty())
        .collect()
}

fn unquote(v: &str) -> String {
    let inner = |q: char| v.strip_prefix(q).and_then(|s| s.strip_suffix(q));
    if let Some(s) = inner('\'').filter(|_| v.len() >= 2) {
        return s.to_string();
    }
    if let Some(s) = inner('"').filter(|_| v.len() >= 2) {
        return s.replace("\\\"", "\"").replace("\\\\", "\\");
    }
    v.to_string()
}

/// A value as it is written: bare when every character is plain, else in single quotes, which
/// both readers take literally.
fn quote(key: &str, v: &str) -> anyhow::Result<String> {
    let plain = |c: char| c.is_ascii_alphanumeric() || "_./:@,+-=%".contains(c);
    if v.chars().all(plain) {
        return Ok(v.to_string());
    }
    if v.contains(['\'', '\n', '\r']) {
        bail!("{key} cannot hold a single quote or a line break");
    }
    Ok(format!("'{v}'"))
}

/// `text` with each key in `set` given its new value. A key already in the file keeps its line
/// and the comments around it; a new key is added at the end.
pub fn update(text: &str, set: &[(&str, String)]) -> anyhow::Result<String> {
    let mut out = String::new();
    let mut done = vec![false; set.len()];
    for line in text.lines() {
        let key = line
            .split_once('=')
            .map(|(k, _)| k.trim())
            .filter(|_| !line.trim_start().starts_with('#'));
        match key.and_then(|k| set.iter().position(|(s, _)| *s == k)) {
            Some(i) if done[i] => {}
            Some(i) => {
                out += &format!("{}={}\n", set[i].0, quote(set[i].0, &set[i].1)?);
                done[i] = true;
            }
            None => out += &format!("{line}\n"),
        }
    }
    for ((k, v), _) in set.iter().zip(&done).filter(|(_, d)| !**d) {
        out += &format!("{k}={}\n", quote(k, v)?);
    }
    Ok(out)
}

/// Writes `text` through a temporary file and a rename, so a reader never sees half a file.
/// The file is readable by its owner only: it can hold the SMTP password.
pub fn write(path: &Path, text: &str) -> anyhow::Result<()> {
    use std::io::Write;
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).with_context(|| format!("could not create {}", dir.display()))?;
    let name = path.file_name().context("the env file needs a file name")?;
    let tmp = dir.join(format!(".{}.tmp", name.to_string_lossy()));
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    let mut f = open
        .open(&tmp)
        .with_context(|| format!("could not write {}", tmp.display()))?;
    f.write_all(text.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(&tmp, path).with_context(|| format!("could not replace {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{parse, update};

    #[test]
    fn a_written_value_reads_back_the_same() {
        let values = [
            ("A", "plain-value_1.2:3@x/y".to_string()),
            ("B", "Monita <monita@x.co.id>".to_string()),
            ("C", "with $dollar and \"quotes\"".to_string()),
            ("D", String::new()),
        ];
        let text = update("", &values).unwrap();
        let read: Vec<(String, String)> = values
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        assert_eq!(parse(&text), read);
        assert!(text.contains("A=plain-value_1.2:3@x/y\n"), "{text}");
        assert!(text.contains("C='with $dollar and \"quotes\"'\n"), "{text}");
    }

    #[test]
    fn a_double_quoted_value_from_the_example_file_is_read() {
        let got = parse("# note\nMONITA_MAIL_FROM=\"Monita <m@x.id>\"\n\nnot a line\n");
        assert_eq!(
            got,
            [(
                "MONITA_MAIL_FROM".to_string(),
                "Monita <m@x.id>".to_string()
            )]
        );
    }

    #[test]
    fn an_update_keeps_comments_and_other_keys_and_adds_new_ones_last() {
        let old = "# head\nMONITA_BIND=1\nMONITA_LOG=info\n# MONITA_BIND=commented\n";
        let set = [
            ("MONITA_BIND", "127.0.0.1:9000".to_string()),
            ("MONITA_SMTP_URL", "smtp://h".to_string()),
        ];
        let new = update(old, &set).unwrap();
        assert_eq!(
            new,
            "# head\nMONITA_BIND=127.0.0.1:9000\nMONITA_LOG=info\n# MONITA_BIND=commented\nMONITA_SMTP_URL=smtp://h\n"
        );
    }

    #[test]
    fn a_value_with_a_single_quote_is_refused() {
        let err = update("", &[("MONITA_SMTP_URL", "it's".to_string())]).unwrap_err();
        assert!(err.to_string().contains("MONITA_SMTP_URL"), "{err}");
    }
}
