use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    monita::cli::run(monita::cli::Cli::parse()).await
}
