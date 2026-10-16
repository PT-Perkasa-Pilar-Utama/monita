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
