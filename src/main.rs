use clap::{Parser, Subcommand};
use monita::platform::{config::Config, db::Db, migrate};

#[derive(Parser)]
#[command(
    name = "monita",
    version,
    about = "Self-hosted uptime, incident and status service"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run pending migrations, then serve HTTP and the background loops.
    Serve,
    /// Run pending migrations and exit.
    Migrate,
    /// Manage accounts.
    Admin {
        #[command(subcommand)]
        cmd: AdminCmd,
    },
}

#[derive(Subcommand)]
enum AdminCmd {
    /// Create a super admin. Reads the password from stdin.
    Create {
        #[arg(long)]
        email: String,
    },
    /// Set a new password for an existing super admin. Reads it from stdin.
    ResetPassword {
        #[arg(long)]
        email: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::from_env()?;
    init_tracing(&config);

    match cli.cmd {
        Cmd::Serve => monita::app::serve(config).await,
        Cmd::Migrate => {
            let db = open_offline(&config).await?;
            migrate::run(&db).await
        }
        Cmd::Admin {
            cmd: AdminCmd::Create { email },
        } => {
            let db = open_offline(&config).await?;
            migrate::run(&db).await?;
            let password = read_password(&email)?;
            let user = monita::accounts::create_super_admin(&db, &email, &password).await?;
            println!("created super admin {}", user.email);
            Ok(())
        }
        Cmd::Admin {
            cmd: AdminCmd::ResetPassword { email },
        } => {
            let db = open_offline(&config).await?;
            migrate::run(&db).await?;
            let password = read_password(&email)?;
            monita::accounts::reset_super_admin_password(&db, &email, &password).await?;
            println!("new password set for {email}; their sessions are signed out");
            Ok(())
        }
    }
}

/// The CLI commands need the database file to themselves: turso locks it while `serve` runs.
async fn open_offline(config: &Config) -> anyhow::Result<Db> {
    use anyhow::Context;
    Db::open(&config.db_path).await.with_context(|| {
        format!(
            "could not open {}; if Monita is running, stop it first (systemctl stop monita)",
            config.db_path.display()
        )
    })
}

// ponytail: the password echoes on a TTY; add termios no-echo if admins run this over a shared screen.
fn read_password(email: &str) -> anyhow::Result<String> {
    eprint!("Password for {email}: ");
    let mut password = String::new();
    std::io::stdin().read_line(&mut password)?;
    Ok(password.trim_end_matches(['\r', '\n']).to_string())
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
