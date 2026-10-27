//! `monita service`: the systemd unit on a Linux host. Contract: docs/api-specs/10-cli.md 10.3.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, bail};

use crate::platform::config::Config;

pub const UNIT_PATH: &str = "/etc/systemd/system/monita.service";
/// One source for the unit: deploy/monita.service, filled in with this host's paths.
const TEMPLATE: &str = include_str!("../../deploy/monita.service");
const USER: &str = "monita";

/// The template with this host's binary, env file and folders.
pub fn unit(exe: &Path, env_file: &Path, config: &Config) -> anyhow::Result<String> {
    let folders = folders(config)?;
    for p in folders.iter().map(PathBuf::as_path).chain([exe, env_file]) {
        if !p.is_absolute() || p.to_string_lossy().contains(char::is_whitespace) {
            bail!("{} must be an absolute path without spaces", p.display());
        }
    }
    let rw: Vec<String> = folders.iter().map(|p| p.display().to_string()).collect();
    let fill = [
        ("ExecStart=", format!("{} serve", exe.display())),
        ("EnvironmentFile=", env_file.display().to_string()),
        ("ReadWritePaths=", rw.join(" ")),
    ];
    let mut seen = [false; 3];
    let mut out = String::new();
    for line in TEMPLATE.lines() {
        match fill.iter().position(|(key, _)| line.starts_with(key)) {
            Some(i) => {
                out += &format!("{}{}\n", fill[i].0, fill[i].1);
                seen[i] = true;
            }
            None => out += &format!("{line}\n"),
        }
    }
    if seen.contains(&false) {
        bail!("deploy/monita.service lost a line that `service install` fills in");
    }
    Ok(out)
}

/// The data folder, then the backup folder if there is one.
fn folders(config: &Config) -> anyhow::Result<Vec<PathBuf>> {
    let data = config
        .db_path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .context("MONITA_DB_PATH needs a folder")?;
    Ok([Some(data.to_path_buf()), config.backup_dir.clone()]
        .into_iter()
        .flatten()
        .collect())
}

pub async fn install(config: &Config, env_file: Option<&Path>) -> anyhow::Result<()> {
    linux()?;
    let env_file =
        env_file.context("no env file to point the unit at: run `sudo monita setup` first")?;
    let env_file = std::fs::canonicalize(env_file)?;
    let exe = std::env::current_exe()?.canonicalize()?;
    if exe.starts_with("/home") || exe.starts_with("/root") {
        bail!(
            "{} is under a home folder, which the unit hides (ProtectHome); install it to /usr/local/bin",
            exe.display()
        );
    }
    let text = unit(&exe, &env_file, config)?;

    if !run("id", &["-u", USER])?.status.success() {
        let data = &folders(config)?[0];
        checked(
            Command::new("useradd")
                .args([
                    "--system",
                    "--user-group",
                    "--shell",
                    "/usr/sbin/nologin",
                    "--home-dir",
                ])
                .arg(data)
                .arg(USER),
        )?;
        println!("created the system user {USER}");
    }
    let uid = id_of("-u")?;
    let gid = id_of("-g")?;
    for dir in folders(config)? {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("could not create {}", dir.display()))?;
        hand_over(&dir, uid, gid)?;
    }
    std::fs::write(UNIT_PATH, text)
        .with_context(|| format!("could not write {UNIT_PATH}; run as root"))?;
    println!("wrote {UNIT_PATH}");
    for args in [
        &["daemon-reload"][..],
        &["enable", "monita"],
        &["restart", "monita"],
    ] {
        checked(Command::new("systemctl").args(args))?;
    }
    wait_healthy(config.bind).await
}

/// Gives the folder and what it holds to the service user, mode 0700. `lchown` changes a link
/// itself, never what it points at.
#[cfg(unix)]
fn hand_over(dir: &Path, uid: u32, gid: u32) -> anyhow::Result<()> {
    use std::os::unix::fs::{PermissionsExt, chown, lchown};
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    chown(dir, Some(uid), Some(gid))?;
    for entry in std::fs::read_dir(dir)? {
        lchown(entry?.path(), Some(uid), Some(gid))?;
    }
    Ok(())
}

/// Never reached: `install` stops at `linux()` first. It lets the binary build for Windows.
#[cfg(not(unix))]
fn hand_over(_: &Path, _: u32, _: u32) -> anyhow::Result<()> {
    bail!("monita service runs on Linux with systemd only")
}

pub fn uninstall() -> anyhow::Result<()> {
    linux()?;
    checked(Command::new("systemctl").args(["disable", "--now", "monita"]))?;
    match std::fs::remove_file(UNIT_PATH) {
        Ok(()) => println!("removed {UNIT_PATH}"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).context(format!("could not remove {UNIT_PATH}")),
    }
    checked(Command::new("systemctl").arg("daemon-reload"))?;
    println!("kept the env file, the data, the backups and the {USER} user");
    Ok(())
}

pub fn status() -> anyhow::Result<()> {
    linux()?;
    pass_through(Command::new("systemctl").args(["status", "monita", "--no-pager"]))
}

pub fn logs(follow: bool) -> anyhow::Result<()> {
    linux()?;
    let mut cmd = Command::new("journalctl");
    cmd.args(["-u", "monita", "--no-pager", "-n", "200"]);
    if follow {
        cmd.arg("-f");
    }
    pass_through(&mut cmd)
}

fn linux() -> anyhow::Result<()> {
    if !cfg!(target_os = "linux") {
        bail!("`monita service` needs Linux with systemd; on other systems run `monita serve`");
    }
    Ok(())
}

fn run(program: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
    Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("could not run {program}"))
}

fn id_of(flag: &str) -> anyhow::Result<u32> {
    let out = run("id", &[flag, USER])?;
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .with_context(|| format!("`id {flag} {USER}` gave no number"))
}

pub(super) fn checked(cmd: &mut Command) -> anyhow::Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("could not run {:?}", cmd.get_program()))?;
    if !status.success() {
        bail!("{:?} failed with {status}", cmd);
    }
    Ok(())
}

/// Runs a viewer command with this terminal and exits with its code.
fn pass_through(cmd: &mut Command) -> anyhow::Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("could not run {:?}", cmd.get_program()))?;
    std::process::exit(status.code().unwrap_or(1));
}

/// Waits up to 20 s for `/healthz` on the listen address.
async fn wait_healthy(bind: SocketAddr) -> anyhow::Result<()> {
    let mut addr = bind;
    if addr.ip().is_unspecified() {
        addr.set_ip(match addr.ip() {
            IpAddr::V4(_) => Ipv4Addr::LOCALHOST.into(),
            IpAddr::V6(_) => Ipv6Addr::LOCALHOST.into(),
        });
    }
    let url = format!("http://{addr}/healthz");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    for _ in 0..40 {
        if let Ok(res) = client.get(&url).send().await
            && res.status().is_success()
        {
            println!("monita is running: {url}");
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    bail!("monita did not answer {url} within 20 s; see `journalctl -u monita`")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::unit;
    use crate::platform::config::Config;

    fn config(pairs: &[(&str, &str)]) -> Config {
        Config::from_lookup(|k| {
            pairs
                .iter()
                .find(|(p, _)| *p == k)
                .map(|(_, v)| v.to_string())
        })
        .unwrap()
    }

    #[test]
    fn the_unit_points_at_this_binary_env_file_and_folders() {
        let c = config(&[
            ("MONITA_ENV", "prod"),
            ("MONITA_BASE_URL", "https://m.example.com"),
            ("MONITA_DB_PATH", "/srv/monita/monita.db"),
            ("MONITA_BACKUP_DIR", "/srv/backups"),
        ]);
        let text = unit(Path::new("/opt/bin/monita"), Path::new("/etc/m.env"), &c).unwrap();
        assert!(
            text.contains("\nExecStart=/opt/bin/monita serve\n"),
            "{text}"
        );
        assert!(text.contains("\nEnvironmentFile=/etc/m.env\n"), "{text}");
        assert!(
            text.contains("\nReadWritePaths=/srv/monita /srv/backups\n"),
            "{text}"
        );
        assert!(
            text.contains("\nUser=monita\n"),
            "the rest of the template stays: {text}"
        );
    }

    #[test]
    fn a_relative_database_path_is_refused() {
        let c = config(&[("MONITA_ENV", "dev"), ("MONITA_DB_PATH", "data/monita.db")]);
        let err = unit(Path::new("/opt/bin/monita"), Path::new("/etc/m.env"), &c).unwrap_err();
        assert!(err.to_string().contains("data"), "{err}");
    }
}
