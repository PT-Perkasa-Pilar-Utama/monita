//! The `monita` command line. Contract: docs/api-specs/02-authentication.md 2.1 and
//! docs/api-specs/10-cli.md.

mod envfile;
mod service;
mod setup;

use std::{
    io::BufRead,
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};

use crate::platform::{config::Config, db::Db, migrate};

/// Where `setup` writes and every other command looks when `MONITA_ENV` is unset.
pub const ENV_FILE: &str = "/etc/monita/monita.env";

#[derive(Parser)]
#[command(
    name = "monita",
    version,
    about = "Self-hosted uptime, incident and status service"
)]
pub struct Cli {
    /// Read MONITA_* variables from this file; variables already set win. Without it, Monita
    /// reads /etc/monita/monita.env when MONITA_ENV is unset.
    #[arg(long, global = true, value_name = "PATH")]
    env_file: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run pending migrations, then serve HTTP and the background loops.
    Serve,
    /// Run pending migrations and exit.
    Migrate,
    /// Write the env file for production and create the first super admin.
    Setup {
        /// Ask nothing: take each value from MONITA_*, then the file, then the default.
        #[arg(long)]
        yes: bool,
    },
    /// Install or inspect the systemd service (Linux, as root).
    Service {
        #[command(subcommand)]
        cmd: ServiceCmd,
    },
    /// Print where the env file, the data and the logs are.
    Whereis { what: Option<Place> },
    /// Manage accounts.
    Admin {
        #[command(subcommand)]
        cmd: AdminCmd,
    },
}

#[derive(Subcommand)]
enum ServiceCmd {
    /// Create the monita user and folders, write the unit, start it and wait for /healthz.
    Install,
    /// Stop and remove the unit. The data, the env file and the user stay.
    Uninstall,
    /// systemctl status monita.
    Status,
    /// The service log from journald.
    Logs {
        #[arg(short, long)]
        follow: bool,
    },
}

#[derive(Subcommand)]
enum AdminCmd {
    /// Create a super admin. Reads the password from a prompt, or stdin when not a terminal.
    Create {
        #[arg(long)]
        email: String,
    },
    /// Set a new password for an existing super admin.
    ResetPassword {
        #[arg(long)]
        email: String,
    },
}

#[derive(ValueEnum, Clone, Copy, PartialEq)]
enum Place {
    Env,
    Data,
    Backups,
    Bin,
    Service,
    Logs,
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    if let Cmd::Setup { yes } = cli.cmd {
        let path = cli.env_file.unwrap_or_else(|| ENV_FILE.into());
        return setup::run(&path, yes).await;
    }
    let (config, env_path) = load(cli.env_file)?;
    init_tracing(&config);
    match cli.cmd {
        Cmd::Setup { .. } => unreachable!("handled above"),
        Cmd::Serve => crate::app::serve(config).await,
        Cmd::Migrate => offline(&config, async |_| Ok(())).await,
        Cmd::Service { cmd } => match cmd {
            ServiceCmd::Install => service::install(&config, env_path.as_deref()).await,
            ServiceCmd::Uninstall => service::uninstall(),
            ServiceCmd::Status => service::status(),
            ServiceCmd::Logs { follow } => service::logs(follow),
        },
        Cmd::Whereis { what } => {
            whereis(&config, env_path.as_deref(), what);
            Ok(())
        }
        Cmd::Admin {
            cmd: AdminCmd::Create { email },
        } => {
            let password = read_secret(&format!("Password for {email}: "))?;
            offline(&config, async |db| {
                let user = crate::accounts::create_super_admin(db, &email, &password).await?;
                println!("created super admin {}", user.email);
                Ok(())
            })
            .await
        }
        Cmd::Admin {
            cmd: AdminCmd::ResetPassword { email },
        } => {
            let password = read_secret(&format!("New password for {email}: "))?;
            offline(&config, async |db| {
                crate::accounts::reset_super_admin_password(db, &email, &password).await?;
                println!("new password set for {email}; their sessions are signed out");
                Ok(())
            })
            .await
        }
    }
}

/// The config from the process env, then the env file. Returns the file it read, if any.
fn load(env_file: Option<PathBuf>) -> anyhow::Result<(Config, Option<PathBuf>)> {
    let path = env_file.or_else(|| {
        let default = Path::new(ENV_FILE);
        (std::env::var_os("MONITA_ENV").is_none() && default.exists()).then(|| default.into())
    });
    let file = match &path {
        Some(p) => envfile::parse(&std::fs::read_to_string(p).with_context(|| {
            format!(
                "could not read {}; it is readable by root only",
                p.display()
            )
        })?),
        None => Vec::new(),
    };
    let config = Config::from_lookup(|k| {
        std::env::var(k)
            .ok()
            .or_else(|| file.iter().find(|(f, _)| f == k).map(|(_, v)| v.clone()))
    })?;
    Ok((config, path))
}

/// Opens the database without the server, migrates it and runs `f`. turso locks the file
/// while `serve` runs, so the service must be stopped first.
async fn offline(
    config: &Config,
    f: impl AsyncFnOnce(&Db) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let db = Db::open(&config.db_path).await.with_context(|| {
        format!(
            "could not open {}; if Monita is running, stop it first (systemctl stop monita)",
            config.db_path.display()
        )
    })?;
    migrate::run(&db).await?;
    let result = f(&db).await;
    drop(db);
    own_like_folder(&config.db_path);
    result
}

/// Gives the database files the owner of their folder. Root runs these commands; the service
/// runs as `monita` and must still open what root created.
fn own_like_folder(db_path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let Some(dir) = db_path.parent().and_then(|d| std::fs::metadata(d).ok()) else {
            return;
        };
        let name = db_path.as_os_str().to_string_lossy();
        for path in [
            name.to_string(),
            format!("{name}-wal"),
            format!("{name}-shm"),
        ] {
            let Ok(m) = std::fs::metadata(&path) else {
                continue;
            };
            if (m.uid(), m.gid()) != (dir.uid(), dir.gid())
                && let Err(e) = std::os::unix::fs::chown(&path, Some(dir.uid()), Some(dir.gid()))
            {
                eprintln!("warning: could not give {path} to the owner of its folder: {e}");
            }
        }
    }
}

fn whereis(config: &Config, env_path: Option<&Path>, what: Option<Place>) {
    let bin = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());
    let rows = [
        (
            Place::Env,
            "env",
            env_path.map_or("none: MONITA_* come from the environment".into(), |p| {
                p.display().to_string()
            }),
        ),
        (Place::Data, "data", config.db_path.display().to_string()),
        (
            Place::Backups,
            "backups",
            config
                .backup_dir
                .as_ref()
                .map_or("none".into(), |d| d.display().to_string()),
        ),
        (Place::Bin, "bin", bin),
        (Place::Service, "service", service::UNIT_PATH.into()),
        (Place::Logs, "logs", "journalctl -u monita".into()),
    ];
    for (place, label, value) in rows {
        match what {
            Some(w) if w == place => println!("{value}"),
            Some(_) => {}
            None => println!("{label:<8} {value}"),
        }
    }
}

/// One line from stdin, without its line break. A closed stdin is an error, not an empty answer.
fn read_line() -> anyhow::Result<String> {
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line)? == 0 {
        anyhow::bail!("stdin closed before the answer");
    }
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// A password from the terminal without echo, or one line of stdin when it is not a terminal.
// ponytail: `stty -echo` stays on if the prompt is killed; termios through a crate would restore it.
fn read_secret(prompt: &str) -> anyhow::Result<String> {
    use std::io::IsTerminal;
    eprint!("{prompt}");
    let tty = std::io::stdin().is_terminal();
    let echo = |on: bool| {
        let _ = std::process::Command::new("stty")
            .arg(if on { "echo" } else { "-echo" })
            .status();
    };
    if tty {
        echo(false);
    }
    let line = read_line();
    if tty {
        echo(true);
        eprintln!();
    }
    line
}

fn init_tracing(config: &Config) {
    let filter = tracing_subscriber::EnvFilter::new(&config.log_filter);
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    if config.log_json {
        builder.json().init();
    } else {
        builder.init();
    }
}
