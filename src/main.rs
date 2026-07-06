mod app;
mod config;
mod security;
mod services;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "doktui", version, about = "Terminal UI for remote server management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check for updates and replace the binary (script installs only)
    Update,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("doktui=info".parse()?))
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Update) => app::run_update().await,
        None => app::run_tui().await,
    }
}
