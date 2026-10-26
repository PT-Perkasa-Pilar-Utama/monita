//! `monita probe`, `monita probe install` and `monita probe uninstall`: a location's probe, run
//! by hand or as a systemd service (technical-specs 14, api-specs 10 section 10.5).

use std::{io::Write, path::Path, process::Command};

use anyhow::{Context, bail};
use clap::{Args, Subcommand};

pub const UNIT_PATH: &str = "/etc/systemd/system/monita-probe.service";
pub const ENV_PATH: &str = "/etc/monita/probe.env";
/// One source for the unit: deploy/monita-probe.service, with this host's binary.
const TEMPLATE: &str = include_str!("../../deploy/monita-probe.service");

#[derive(Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ProbeArgs {
    #[command(subcommand)]
    cmd: Option<ProbeCmd>,
    /// The Monita to report to, such as https://monita.company.co.id. Or MONITA_PROBE_SERVER.
    #[arg(long)]
    server: Option<String>,
    /// The location's token. Prefer MONITA_PROBE_TOKEN: a command line shows in `ps`.
    #[arg(long)]
    token: Option<String>,
    /// Checks at once, 1 to 50.
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u16).range(1..=50))]
    concurrency: u16,
}

#[derive(Subcommand)]
enum ProbeCmd {
    /// Run the probe as a systemd service that starts with the machine (Linux, as root).
    Install {
        /// The Monita to report to. Or MONITA_PROBE_SERVER.
        #[arg(long)]
        server: Option<String>,
        /// The location's token. Prefer MONITA_PROBE_TOKEN: a command line shows in `ps`.
        #[arg(long)]
        token: Option<String>,
    },
    /// Stop the service and remove it with its env file.
    Uninstall,
}

fn var(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.trim().is_empty())
}

/// The server and token, from the variables first for the token, the flags first for the server.
fn resolve(server: Option<String>, token: Option<String>) -> anyhow::Result<(String, String)> {
    let server = server
        .or_else(|| var("MONITA_PROBE_SERVER"))
        .context("Give --server or MONITA_PROBE_SERVER.")?;
    let token = var("MONITA_PROBE_TOKEN")
        .or(token)
        .context("Give MONITA_PROBE_TOKEN, or --token.")?;
    Ok((server, token))
}

pub async fn main(args: ProbeArgs) -> anyhow::Result<()> {
    match args.cmd {
        Some(ProbeCmd::Install { server, token }) => {
            let (server, token) = resolve(server, token)?;
            install(&server, &token)
        }
        Some(ProbeCmd::Uninstall) => uninstall(),
        None => {
            let filter = std::env::var("MONITA_LOG").unwrap_or_else(|_| "info".into());
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
                .init();
            let (server, token) = resolve(args.server, args.token)?;
            super::probe::run(&server, &token, args.concurrency).await
        }
    }
}

/// The unit for this host's binary.
pub fn unit(exe: &Path) -> anyhow::Result<String> {
    if !exe.is_absolute() || exe.to_string_lossy().contains(char::is_whitespace) {
        bail!("{} must be an absolute path without spaces", exe.display());
    }
    let mut seen = false;
    let mut out = String::new();
    for line in TEMPLATE.lines() {
        if line.starts_with("ExecStart=") {
            out += &format!("ExecStart={} probe\n", exe.display());
            seen = true;
        } else {
            out += &format!("{line}\n");
        }
    }
    if !seen {
        bail!("deploy/monita-probe.service lost its ExecStart line");
    }
    Ok(out)
}

/// The env file the unit reads. Refuses a server or token that would never work, before anything
/// is written.
pub fn env_file(server: &str, token: &str) -> anyhow::Result<String> {
    let server = server.trim().trim_end_matches('/');
    let token = token.trim();
    if !(server.starts_with("https://") || server.starts_with("http://"))
        || server.contains(char::is_whitespace)
    {
        bail!("--server must be your Monita's address, such as https://monita.company.co.id");
    }
    if !token.starts_with(crate::checks::locations::TOKEN_PREFIX)
        || token.contains(char::is_whitespace)
    {
        bail!("the token must be the ml_… token from Admin, Locations");
    }
    Ok(format!(
        "MONITA_PROBE_SERVER={server}\nMONITA_PROBE_TOKEN={token}\n"
    ))
}

fn linux() -> anyhow::Result<()> {
    if !cfg!(target_os = "linux") {
        bail!(
            "`monita probe install` needs Linux with systemd; elsewhere run the probe in Docker or with `monita probe`"
        );
    }
    Ok(())
}

fn install(server: &str, token: &str) -> anyhow::Result<()> {
    linux()?;
    let env = env_file(server, token)?;
    let exe = std::env::current_exe()?.canonicalize()?;
    if exe.starts_with("/home") || exe.starts_with("/root") {
        bail!(
            "{} is under a home folder, which the unit hides (ProtectHome); install it to /usr/local/bin",
            exe.display()
        );
    }
    let text = unit(&exe)?;
    std::fs::create_dir_all("/etc/monita").context("could not create /etc/monita; run as root")?;
    write_secret(Path::new(ENV_PATH), &env)?;
    println!("wrote {ENV_PATH}, readable by root only");
    std::fs::write(UNIT_PATH, text)
        .with_context(|| format!("could not write {UNIT_PATH}; run as root"))?;
    println!("wrote {UNIT_PATH}");
    for args in [
        &["daemon-reload"][..],
        &["enable", "monita-probe"],
        &["restart", "monita-probe"],
    ] {
        super::service::checked(Command::new("systemctl").args(args))?;
    }
    println!(
        "monita-probe is running, and Admin, Locations shows it as Running within a few seconds"
    );
    println!("follow it with: journalctl -u monita-probe -f");
    Ok(())
}

/// Writes the file with mode 0600 from the start, so the token is never readable by others.
fn write_secret(path: &Path, text: &str) -> anyhow::Result<()> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        opts.mode(0o600);
        let mut f = opts
            .open(path)
            .with_context(|| format!("could not write {}; run as root", path.display()))?;
        f.write_all(text.as_bytes())?;
        // An existing file keeps its old mode on open, so set it again.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    {
        let mut f = opts.open(path)?;
        f.write_all(text.as_bytes())?;
        Ok(())
    }
}

fn uninstall() -> anyhow::Result<()> {
    linux()?;
    super::service::checked(Command::new("systemctl").args(["disable", "--now", "monita-probe"]))?;
    for path in [UNIT_PATH, ENV_PATH] {
        match std::fs::remove_file(path) {
            Ok(()) => println!("removed {path}"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context(format!("could not remove {path}")),
        }
    }
    super::service::checked(Command::new("systemctl").arg("daemon-reload"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unit_runs_this_binary_and_the_env_file_holds_both_values() {
        let text = unit(Path::new("/opt/monita/monita")).unwrap();
        assert!(text.contains("ExecStart=/opt/monita/monita probe\n"));
        assert!(text.contains(&format!("EnvironmentFile={ENV_PATH}")));
        assert!(unit(Path::new("relative/monita")).is_err());
        let env = env_file("https://monita.example.com/ ", "ml_abc").unwrap();
        assert_eq!(
            env,
            "MONITA_PROBE_SERVER=https://monita.example.com\nMONITA_PROBE_TOKEN=ml_abc\n"
        );
        assert!(
            env_file("monita.example.com", "ml_abc").is_err(),
            "no scheme"
        );
        assert!(
            env_file("https://m.example.com", "mk_abc").is_err(),
            "an API key is not a location token"
        );
    }
}
