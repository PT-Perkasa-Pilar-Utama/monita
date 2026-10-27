//! Questions the CLI asks on the terminal, or reads as lines of stdin when it is not a terminal.

use std::io::{BufRead, IsTerminal};

/// One line from stdin, without its line break. A closed stdin is an error, not an empty answer.
pub fn read_line() -> anyhow::Result<String> {
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line)? == 0 {
        anyhow::bail!("stdin closed before the answer");
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// A password from the terminal without echo, or one line of stdin when it is not a terminal.
pub fn read_secret(prompt: &str) -> anyhow::Result<String> {
    if !std::io::stdin().is_terminal() {
        eprint!("{prompt}");
        return read_line();
    }
    // Echo goes off before the prompt shows, so nothing typed after the prompt is echoed.
    let hidden = NoEcho::on_stdin();
    eprint!("{prompt}");
    let line = read_line();
    drop(hidden);
    eprintln!();
    line
}

/// A new password. On a terminal it is asked twice, because a typo stays hidden; a script still
/// sends one line.
pub fn new_password(prompt: &str) -> anyhow::Result<String> {
    let first = read_secret(prompt)?;
    if std::io::stdin().is_terminal() && read_secret("The same password again: ")? != first {
        anyhow::bail!("the two passwords differ; nothing was changed");
    }
    Ok(first)
}

/// Turns the terminal's echo off until dropped. It sets the terminal itself, because the
/// container image has no `stty` to call.
#[cfg(unix)]
struct NoEcho(Option<libc::termios>);

#[cfg(unix)]
impl NoEcho {
    fn on_stdin() -> Self {
        // SAFETY: tcgetattr fills the zeroed struct, and both calls only read or set fd 0's mode.
        unsafe {
            let mut mode: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(0, &mut mode) != 0 {
                return Self(None);
            }
            let old = mode;
            mode.c_lflag &= !libc::ECHO;
            if libc::tcsetattr(0, libc::TCSANOW, &mode) != 0 {
                return Self(None);
            }
            Self(Some(old))
        }
    }
}

#[cfg(unix)]
impl Drop for NoEcho {
    fn drop(&mut self) {
        if let Some(old) = &self.0 {
            // SAFETY: restores the mode read in `on_stdin` on the same fd.
            unsafe { libc::tcsetattr(0, libc::TCSANOW, old) };
        }
    }
}

// ponytail: Windows echoes the password; SetConsoleMode through windows-sys would hide it.
#[cfg(not(unix))]
struct NoEcho;

#[cfg(not(unix))]
impl NoEcho {
    fn on_stdin() -> Self {
        Self
    }
}
