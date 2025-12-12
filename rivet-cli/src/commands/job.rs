//! Job command handlers
//!
//! Handles all job-related CLI commands including listing,
//! viewing details, and accessing logs.

use anyhow::Result;
use clap::Subcommand;
use colored::*;
use rivet_core::domain::job::{Job, JobStatus};
use rivet_core::domain::log::{LogEntry, LogLevel};

use crate::api::ApiClient;
use crate::config::Config;
use crate::id_resolver::{resolve_job_id, resolve_job_id_in_pipeline, resolve_pipeline_id};
use crate::types::IdOrPrefix;

/// Job subcommands
#[derive(Subcommand)]
pub enum JobCommands {
    /// List all jobs
    List,
    /// List scheduled jobs
    Scheduled,
    /// Get job details
    Get {
        /// Job ID or unambiguous prefix
        id: String,
    },
    /// Get job logs
    Logs {
        /// Job ID or unambiguous prefix
        id: String,

        /// Follow logs (not yet implemented)
        #[arg(short, long)]
        follow: bool,
    },
    /// List jobs for a pipeline
    Pipeline {
        /// Pipeline ID or unambiguous prefix
        pipeline_id: String,

        /// Also resolve job IDs by prefix within this pipeline
        #[arg(long)]
        job: Option<String>,
    },
}

/// Handle job commands
///
/// Routes job subcommands to their respective handlers.
///
/// # Arguments
/// * `command` - The job command to execute
/// * `config` - The CLI configuration
pub async fn handle_job_command(command: JobCommands, config: &Config) -> Result<()> {
    let client = ApiClient::new(&config.orchestrator_url);

    match command {
        JobCommands::List => list_all_jobs(&client).await,
        JobCommands::Scheduled => list_scheduled_jobs(&client).await,
        JobCommands::Get { id } => get_job(&client, &id).await,
        JobCommands::Logs { id, follow } => get_job_logs(&client, &id, follow).await,
        JobCommands::Pipeline { pipeline_id, job } => {
            list_pipeline_jobs(&client, &pipeline_id, job).await
        }
    }
}

/// List all jobs
async fn list_all_jobs(client: &ApiClient) -> Result<()> {
    let jobs = client.list_all_jobs().await?;

    if jobs.is_empty() {
        println!("{}", "No jobs found.".yellow());
    } else {
        println!("{}", format!("Found {} job(s):", jobs.len()).bold());
        println!();
        for job in jobs {
            print_job_summary(&job);
        }
    }

    Ok(())
}

/// List all scheduled jobs
async fn list_scheduled_jobs(client: &ApiClient) -> Result<()> {
    let jobs = client.list_scheduled_jobs().await?;

    if jobs.is_empty() {
        println!("{}", "No scheduled jobs found.".yellow());
    } else {
        println!(
            "{}",
            format!("Found {} scheduled job(s):", jobs.len()).bold()
        );
        println!();
        for job in jobs {
            print_job_summary(&job);
        }
    }

    Ok(())
}

/// Get and display a single job
async fn get_job(client: &ApiClient, id: &str) -> Result<()> {
    let id_or_prefix = IdOrPrefix::parse(id);
    let uuid = resolve_job_id(client, &id_or_prefix).await?;

    let job = client.get_job(uuid).await?;

    print_job_details(&job);

    Ok(())
}

/// Get and display job logs
async fn get_job_logs(client: &ApiClient, id: &str, follow: bool) -> Result<()> {
    let id_or_prefix = IdOrPrefix::parse(id);
    let uuid = resolve_job_id(client, &id_or_prefix).await?;

    if follow {
        println!("{}", "⚠ Log following not yet implemented".yellow());
        println!("{}", "  Showing current logs only...".dimmed());
        println!();
    }

    let logs = client.get_job_logs(uuid).await?;

    if logs.is_empty() {
        println!("{}", "No logs found for this job.".yellow());
    } else {
        println!("{}", format!("Logs for job {}:", uuid).bold());
        println!("{}", "─".repeat(80).dimmed());
        for log in logs {
            print_log_entry(&log);
        }
        println!("{}", "─".repeat(80).dimmed());
    }

    Ok(())
}

/// List jobs for a specific pipeline
async fn list_pipeline_jobs(
    client: &ApiClient,
    pipeline_id: &str,
    job_id: Option<String>,
) -> Result<()> {
    let pipeline_id_or_prefix = IdOrPrefix::parse(pipeline_id);
    let pipeline_uuid = resolve_pipeline_id(client, &pipeline_id_or_prefix).await?;

    // If a specific job ID is provided, resolve and show just that job
    if let Some(job_id_str) = job_id {
        let job_id_or_prefix = IdOrPrefix::parse(&job_id_str);
        let job_uuid = resolve_job_id_in_pipeline(client, pipeline_uuid, &job_id_or_prefix).await?;

        let job = client.get_job(job_uuid).await?;
        print_job_details(&job);
        return Ok(());
    }

    // Otherwise, list all jobs for the pipeline
    let jobs = client.list_jobs_by_pipeline(pipeline_uuid).await?;

    if jobs.is_empty() {
        println!(
            "{}",
            format!("No jobs found for pipeline {}.", pipeline_uuid).yellow()
        );
    } else {
        println!(
            "{}",
            format!(
                "Found {} job(s) for pipeline {}:",
                jobs.len(),
                pipeline_uuid
            )
            .bold()
        );
        println!();
        for job in jobs {
            print_job_summary(&job);
        }
    }

    Ok(())
}

/// Print a job summary from a full Job object
fn print_job_summary(job: &Job) {
    let status_colored = colorize_status(&job.status);

    println!("  {} Job {}", "▸".cyan(), job.id.to_string().dimmed());
    println!("    Pipeline: {}", job.pipeline_id.to_string().dimmed());
    println!("    Status:   {}", status_colored);
    println!(
        "    Created:  {}",
        job.requested_at
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
            .dimmed()
    );
    if let Some(runner) = &job.runner_id {
        println!("    Runner:   {}", runner.dimmed());
    }
    println!();
}

/// Print detailed job information
fn print_job_details(job: &Job) {
    let status_colored = colorize_status(&job.status);

    println!("{}", "Job Details:".bold());
    println!("  ID:          {}", job.id.to_string().cyan());
    println!("  Pipeline ID: {}", job.pipeline_id.to_string().dimmed());
    println!("  Status:      {}", status_colored);
    println!(
        "  Requested:   {}",
        job.requested_at.format("%Y-%m-%d %H:%M:%S")
    );

    if let Some(started) = job.started_at {
        println!("  Started:     {}", started.format("%Y-%m-%d %H:%M:%S"));
    }

    if let Some(completed) = job.completed_at {
        println!("  Completed:   {}", completed.format("%Y-%m-%d %H:%M:%S"));

        // Calculate duration
        if let Some(started) = job.started_at {
            let duration = completed.signed_duration_since(started);
            let seconds = duration.num_seconds();
            println!("  Duration:    {}s", seconds);
        }
    }

    if let Some(runner) = &job.runner_id {
        println!("  Runner:      {}", runner);
    }

    if !job.parameters.is_empty() {
        println!("\n{}", "Parameters:".bold());
        for (key, value) in &job.parameters {
            println!("  {} = {}", key.cyan(), value);
        }
    }

    if let Some(result) = &job.result {
        println!("\n{}", "Result:".bold());
        println!(
            "  Success:    {}",
            if result.success {
                "✓".green()
            } else {
                "✗".red()
            }
        );
        println!("  Exit Code:  {}", result.exit_code);

        if let Some(output) = &result.output {
            println!("\n{}", "Output:".bold());
            if let Ok(pretty) = serde_json::to_string_pretty(output) {
                println!("{}", pretty);
            } else {
                println!("{:?}", output);
            }
        }

        if let Some(error) = &result.error_message {
            println!("\n{}", "Error:".bold());
            println!("{}", error.red());
        }
    }
}

/// Print a log entry
fn print_log_entry(log: &LogEntry) {
    let level_str = format!("{:?}", log.level).to_uppercase();
    let level_colored = match log.level {
        LogLevel::Debug => level_str.dimmed(),
        LogLevel::Info => level_str.cyan(),
        LogLevel::Warning => level_str.yellow(),
        LogLevel::Error => level_str.red(),
    };

    println!(
        "{} [{}] {}",
        log.timestamp.format("%H:%M:%S").to_string().dimmed(),
        level_colored,
        log.message
    );
}

/// Colorize job status for display
fn colorize_status(status: &JobStatus) -> colored::ColoredString {
    let status_str = format!("{:?}", status);
    match status {
        JobStatus::Queued => status_str.yellow(),
        JobStatus::Running => status_str.cyan(),
        JobStatus::Succeeded => status_str.green(),
        JobStatus::Failed => status_str.red(),
        JobStatus::Cancelled => status_str.dimmed(),
        JobStatus::TimedOut => status_str.red(),
    }
}
