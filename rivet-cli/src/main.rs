//! Rivet CLI
//!
//! Command-line interface for interacting with the Rivet orchestrator.

mod api;
mod commands;
mod config;
mod id_resolver;
mod types;

use anyhow::Result;
use clap::Parser;
use commands::{Commands, handle_command};
use config::Config;

#[derive(Parser)]
#[command(name = "rivet")]
#[command(about = "Rivet CI/CD Pipeline CLI", long_about = None)]
struct Cli {
    /// Orchestrator URL
    #[arg(
        long,
        env = "RIVET_ORCHESTRATOR_URL",
        default_value = "http://localhost:8080"
    )]
    orchestrator_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = Config {
        orchestrator_url: cli.orchestrator_url,
    };

    handle_command(cli.command, &config).await
}
