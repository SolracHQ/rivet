use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use rivet_core::types::{
    CreateJobRequest, CreatePipelineRequest, Job, JobDto, LogEntry, Pipeline, PipelineConfig,
    PipelineDto,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "rivet")]
#[command(about = "Rivet CI/CD CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Orchestrator URL
    #[arg(short, long, env, default_value = "http://localhost:8080")]
    orchestrator_url: String,
}

#[derive(Subcommand)]
enum Commands {
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
}

#[derive(Subcommand)]
enum PipelineCommands {
    /// Create a new pipeline
    Create {
        /// Pipeline name
        #[arg(short, long)]
        name: String,

        /// Pipeline description
        #[arg(short, long)]
        description: Option<String>,

        /// Path to Lua script file
        #[arg(short, long)]
        script: String,

        /// Required modules (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        modules: Vec<String>,

        /// Tags (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        tags: Vec<String>,

        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,

        /// Max retries
        #[arg(long, default_value = "0")]
        max_retries: u32,
    },
    /// List all pipelines
    List,
    /// Get pipeline details
    Get {
        /// Pipeline ID
        id: Uuid,
    },
    /// Delete a pipeline
    Delete {
        /// Pipeline ID
        id: Uuid,
    },
    /// Launch a job from a pipeline
    Launch {
        /// Pipeline ID
        id: Uuid,

        /// Parameters in JSON format (e.g., '{"key": "value"}')
        #[arg(short, long)]
        params: Option<String>,
    },
}

#[derive(Subcommand)]
enum JobCommands {
    /// List scheduled jobs
    Scheduled,
    /// Get job details
    Get {
        /// Job ID
        id: Uuid,
    },
    /// Get job logs
    Logs {
        /// Job ID
        id: Uuid,
    },
    /// List jobs for a pipeline
    Pipeline {
        /// Pipeline ID
        pipeline_id: Uuid,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = ApiClient::new(&cli.orchestrator_url);

    match cli.command {
        Commands::Pipeline { command } => handle_pipeline_command(client, command).await,
        Commands::Job { command } => handle_job_command(client, command).await,
    }
}

async fn handle_pipeline_command(client: ApiClient, command: PipelineCommands) -> Result<()> {
    match command {
        PipelineCommands::Create {
            name,
            description,
            script,
            modules,
            tags,
            timeout,
            max_retries,
        } => {
            let script_content = std::fs::read_to_string(&script)
                .with_context(|| format!("Failed to read script file: {}", script))?;

            let config = PipelineConfig {
                timeout_seconds: timeout,
                max_retries,
                env_vars: HashMap::new(),
            };

            let req = CreatePipelineRequest {
                name: name.clone(),
                description,
                script: script_content,
                required_modules: modules,
                tags,
                config: Some(config),
            };

            let pipeline = client.create_pipeline(req).await?;
            println!("{}", "✓ Pipeline created successfully!".green().bold());
            println!("  ID:   {}", pipeline.id.to_string().cyan());
            println!("  Name: {}", pipeline.name.bold());
            Ok(())
        }
        PipelineCommands::List => {
            let pipelines = client.list_pipelines().await?;
            if pipelines.is_empty() {
                println!("{}", "No pipelines found.".yellow());
            } else {
                println!(
                    "{}",
                    format!("Found {} pipeline(s):", pipelines.len()).bold()
                );
                println!();
                for pipeline in pipelines {
                    println!("  {} {}", "▸".cyan(), pipeline.name.bold());
                    println!("    ID:      {}", pipeline.id.to_string().dimmed());
                    println!(
                        "    Created: {}",
                        pipeline
                            .created_at
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                            .dimmed()
                    );
                    if let Some(desc) = &pipeline.description {
                        println!("    Desc:    {}", desc.dimmed());
                    }
                    if !pipeline.tags.is_empty() {
                        println!("    Tags:    {}", pipeline.tags.join(", ").dimmed());
                    }
                    println!();
                }
            }
            Ok(())
        }
        PipelineCommands::Get { id } => {
            let pipeline = client.get_pipeline(id).await?;
            println!("{}", "Pipeline Details:".bold());
            println!("  ID:          {}", pipeline.id.to_string().cyan());
            println!("  Name:        {}", pipeline.name.bold());
            if let Some(desc) = &pipeline.description {
                println!("  Description: {}", desc);
            }
            println!(
                "  Created:     {}",
                pipeline.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!(
                "  Updated:     {}",
                pipeline.updated_at.format("%Y-%m-%d %H:%M:%S")
            );
            if !pipeline.tags.is_empty() {
                println!("  Tags:        {}", pipeline.tags.join(", "));
            }
            if !pipeline.required_modules.is_empty() {
                println!("  Modules:     {}", pipeline.required_modules.join(", "));
            }
            println!("\n{}", "Script:".bold());
            println!("{}", "─".repeat(80).dimmed());
            println!("{}", pipeline.script);
            println!("{}", "─".repeat(80).dimmed());
            Ok(())
        }
        PipelineCommands::Delete { id } => {
            client.delete_pipeline(id).await?;
            println!("{}", "✓ Pipeline deleted successfully!".green().bold());
            Ok(())
        }
        PipelineCommands::Launch { id, params } => {
            let parameters = if let Some(params_str) = params {
                serde_json::from_str(&params_str).context("Failed to parse parameters JSON")?
            } else {
                HashMap::new()
            };

            let req = CreateJobRequest {
                pipeline_id: id,
                parameters,
            };

            let job = client.launch_job(req).await?;
            println!("{}", "✓ Job launched successfully!".green().bold());
            println!("  Job ID:      {}", job.id.to_string().cyan());
            println!("  Pipeline ID: {}", job.pipeline_id.to_string().dimmed());
            println!("  Status:      {}", format!("{:?}", job.status).yellow());
            println!(
                "  Requested:   {}",
                job.requested_at.format("%Y-%m-%d %H:%M:%S")
            );
            Ok(())
        }
    }
}

async fn handle_job_command(client: ApiClient, command: JobCommands) -> Result<()> {
    match command {
        JobCommands::Scheduled => {
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
                    println!("  {} Job {}", "▸".cyan(), job.id.to_string().dimmed());
                    println!("    Pipeline: {}", job.pipeline_id.to_string().dimmed());
                    println!("    Status:   {}", format!("{:?}", job.status).yellow());
                    println!(
                        "    Created:  {}",
                        job.requested_at
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                            .dimmed()
                    );
                    println!();
                }
            }
            Ok(())
        }
        JobCommands::Get { id } => {
            let job = client.get_job(id).await?;
            println!("{}", "Job Details:".bold());
            println!("  ID:          {}", job.id.to_string().cyan());
            println!("  Pipeline ID: {}", job.pipeline_id.to_string().dimmed());
            println!("  Status:      {}", format!("{:?}", job.status).yellow());
            println!(
                "  Requested:   {}",
                job.requested_at.format("%Y-%m-%d %H:%M:%S")
            );
            if let Some(started) = job.started_at {
                println!("  Started:     {}", started.format("%Y-%m-%d %H:%M:%S"));
            }
            if let Some(completed) = job.completed_at {
                println!("  Completed:   {}", completed.format("%Y-%m-%d %H:%M:%S"));
            }
            if let Some(runner) = &job.runner_id {
                println!("  Runner:      {}", runner);
            }
            if !job.parameters.is_empty() {
                println!(
                    "  Parameters:  {}",
                    serde_json::to_string_pretty(&job.parameters)?
                );
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
                    println!("  Output:     {}", serde_json::to_string_pretty(output)?);
                }
                if let Some(error) = &result.error_message {
                    println!("  Error:      {}", error.red());
                }
            }
            Ok(())
        }
        JobCommands::Logs { id } => {
            let logs = client.get_job_logs(id).await?;
            if logs.is_empty() {
                println!("{}", "No logs found for this job.".yellow());
            } else {
                println!("{}", format!("Logs for job {}:", id).bold());
                println!("{}", "─".repeat(80).dimmed());
                for log in logs {
                    let level_str = format!("{:?}", log.level).to_uppercase();
                    let level_colored = match log.level {
                        rivet_core::types::LogLevel::Debug => level_str.dimmed(),
                        rivet_core::types::LogLevel::Info => level_str.cyan(),
                        rivet_core::types::LogLevel::Warning => level_str.yellow(),
                        rivet_core::types::LogLevel::Error => level_str.red(),
                    };
                    println!(
                        "{} [{}] {}",
                        log.timestamp.format("%H:%M:%S").to_string().dimmed(),
                        level_colored,
                        log.message
                    );
                }
                println!("{}", "─".repeat(80).dimmed());
            }
            Ok(())
        }
        JobCommands::Pipeline { pipeline_id } => {
            let jobs = client.list_jobs_by_pipeline(pipeline_id).await?;
            if jobs.is_empty() {
                println!("{}", "No jobs found for this pipeline.".yellow());
            } else {
                println!(
                    "{}",
                    format!("Found {} job(s) for pipeline {}:", jobs.len(), pipeline_id).bold()
                );
                println!();
                for job in jobs {
                    let status_colored = match job.status {
                        rivet_core::types::JobStatus::Queued => {
                            format!("{:?}", job.status).yellow()
                        }
                        rivet_core::types::JobStatus::Running => format!("{:?}", job.status).cyan(),
                        rivet_core::types::JobStatus::Succeeded => {
                            format!("{:?}", job.status).green()
                        }
                        rivet_core::types::JobStatus::Failed => format!("{:?}", job.status).red(),
                        rivet_core::types::JobStatus::Cancelled => {
                            format!("{:?}", job.status).dimmed()
                        }
                        rivet_core::types::JobStatus::TimedOut => format!("{:?}", job.status).red(),
                    };
                    println!("  {} Job {}", "▸".cyan(), job.id.to_string().dimmed());
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
            }
            Ok(())
        }
    }
}

// =============================================================================
// API Client
// =============================================================================

struct ApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl ApiClient {
    fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn create_pipeline(&self, req: CreatePipelineRequest) -> Result<Pipeline> {
        let url = format!("{}/pipeline/create", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn list_pipelines(&self) -> Result<Vec<PipelineDto>> {
        let url = format!("{}/pipeline/list", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn get_pipeline(&self, id: Uuid) -> Result<Pipeline> {
        let url = format!("{}/pipeline/{}", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn delete_pipeline(&self, id: Uuid) -> Result<()> {
        let url = format!("{}/pipeline/{}", self.base_url, id);
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        Ok(())
    }

    async fn launch_job(&self, req: CreateJobRequest) -> Result<Job> {
        let url = format!("{}/pipeline/launch", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn list_scheduled_jobs(&self) -> Result<Vec<JobDto>> {
        let url = format!("{}/job/list/scheduled", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn get_job(&self, id: Uuid) -> Result<Job> {
        let url = format!("{}/job/{}", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn get_job_logs(&self, id: Uuid) -> Result<Vec<LogEntry>> {
        let url = format!("{}/job/{}/logs", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn list_jobs_by_pipeline(&self, pipeline_id: Uuid) -> Result<Vec<JobDto>> {
        let url = format!("{}/job/pipeline/{}", self.base_url, pipeline_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response.json().await.context("Failed to parse response")
    }
}
