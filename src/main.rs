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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::from_env()?;
    init_tracing(&config);

    match cli.cmd {
        Cmd::Serve => monita::app::serve(config).await,
        Cmd::Migrate => {
            let db = Db::open(&config.db_path).await?;
            migrate::run(&db).await
        }
        Cmd::Admin {
            cmd: AdminCmd::Create { email },
        } => {
            let db = Db::open(&config.db_path).await?;
            migrate::run(&db).await?;
            // ponytail: the password echoes on a TTY; add termios no-echo if admins run this over a shared screen.
            eprint!("Password for {email}: ");
            let mut password = String::new();
            std::io::stdin().read_line(&mut password)?;
            let user = monita::accounts::create_super_admin(
                &db,
                &email,
                password.trim_end_matches(['\r', '\n']),
            )
            .await?;
            println!("created super admin {}", user.email);
            Ok(())
        }
    }
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
