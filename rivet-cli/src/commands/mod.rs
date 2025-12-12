//! Commands module
//!
//! Defines all CLI commands and their handlers.

mod init;
mod job;
mod pipeline;
mod runner;

pub use init::InitCommands;
pub use job::JobCommands;
pub use pipeline::PipelineCommands;
pub use runner::RunnerCommands;

use anyhow::Result;
use clap::Subcommand;

use crate::config::Config;

/// Top-level CLI commands
#[derive(Subcommand)]
pub enum Commands {
    /// Pipeline management
    Pipeline {
        #[command(subcommand)]
        command: PipelineCommands,
    },
    /// Job management
    Job {
        #[command(subcommand)]
        command: JobCommands,
    },
    /// Runner management
    Runner {
        #[command(subcommand)]
        command: RunnerCommands,
    },
    /// Initialize development environment
    Init {
        #[command(subcommand)]
        command: InitCommands,
    },
}

/// Handle a CLI command
///
/// Routes the command to the appropriate handler module.
///
/// # Arguments
/// * `command` - The command to execute
/// * `config` - The CLI configuration
///
/// # Returns
/// Result indicating success or failure
pub async fn handle_command(command: Commands, config: &Config) -> Result<()> {
    match command {
        Commands::Pipeline { command } => pipeline::handle_pipeline_command(command, config).await,
        Commands::Job { command } => job::handle_job_command(command, config).await,
        Commands::Runner { command } => runner::handle_runner_command(command, config).await,
        Commands::Init { command } => init::handle_init_command(command, config).await,
    }
}
