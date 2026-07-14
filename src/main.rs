mod app;
mod config;
mod i18n;
mod security;
mod services;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "doktui", version, about = "Terminal UI for remote server management")]
struct Cli {
    /// Use the specified theme instead of the configured one
    #[arg(short, long)]
    theme: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check for updates and replace the binary (script installs only)
    Update,
    /// Theme management
    Themes {
        #[command(subcommand)]
        action: ThemeCommands,
    },
}

#[derive(Subcommand)]
enum ThemeCommands {
    /// List installed themes
    List,
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
        Some(Commands::Themes { action: ThemeCommands::List }) => list_themes(),
        None => app::run_tui(cli.theme).await,
    }
}

fn list_themes() -> Result<()> {
    let reg = ui::theme::ThemeRegistry::load_all()?;
    for name in reg.names() {
        if let Some(theme) = reg.get(name) {
            println!("{:<20} {}", theme.meta.name, theme.meta.display_name);
        }
    }
    Ok(())
}
