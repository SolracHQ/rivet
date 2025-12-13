//! Runner command handlers
//!
//! Handles all runner-related CLI commands including listing runners.

use anyhow::Result;
use clap::Subcommand;
use colored::*;
use rivet_core::domain::runner::{Runner, RunnerStatus};

use crate::config::Config;
use rivet_client::OrchestratorClient;

/// Runner subcommands
#[derive(Subcommand)]
pub enum RunnerCommands {
    /// List all registered runners
    List,
}

/// Handle runner commands
///
/// Routes runner subcommands to their respective handlers.
///
/// # Arguments
/// * `command` - The runner command to execute
/// * `config` - The CLI configuration
pub async fn handle_runner_command(command: RunnerCommands, config: &Config) -> Result<()> {
    let client = OrchestratorClient::new(&config.orchestrator_url);

    match command {
        RunnerCommands::List => list_runners(&client).await,
    }
}

/// List all registered runners
async fn list_runners(client: &OrchestratorClient) -> Result<()> {
    let runners = client.list_runners().await?;

    if runners.is_empty() {
        println!("{}", "No runners registered.".yellow());
    } else {
        println!(
            "{}",
            format!("Found {} registered runner(s):", runners.len()).bold()
        );
        println!();
        for runner in runners {
            print_runner_summary(&runner);
        }
    }

    Ok(())
}

/// Print a runner summary
fn print_runner_summary(runner: &Runner) {
    let status_colored = colorize_status(&runner.status);

    println!("  {} Runner {}", "▸".cyan(), runner.id.bold());
    println!("    Status:       {}", status_colored);
    println!(
        "    Registered:   {}",
        runner
            .registered_at
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
            .dimmed()
    );
    println!(
        "    Last Seen:    {}",
        runner
            .last_heartbeat_at
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
            .dimmed()
    );
    println!();
}

/// Colorize runner status for display
fn colorize_status(status: &RunnerStatus) -> colored::ColoredString {
    let status_str = format!("{:?}", status);
    match status {
        RunnerStatus::Online => status_str.green(),
        RunnerStatus::Offline => status_str.red(),
        RunnerStatus::Busy => status_str.yellow(),
    }
}
