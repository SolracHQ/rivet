//! Pipeline command handlers
//!
//! Handles all pipeline-related CLI commands including creation,
//! listing, viewing, deletion, and launching jobs.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::*;
use rivet_core::domain::pipeline::{Pipeline, PipelineConfig};
use rivet_core::dto::job::CreateJob;
use rivet_core::dto::pipeline::CreatePipeline;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::config::Config;
use crate::id_resolver::resolve_pipeline_id;
use crate::types::IdOrPrefix;
use rivet_client::OrchestratorClient;

/// Pipeline subcommands
#[derive(Subcommand)]
pub enum PipelineCommands {
    /// Create a new pipeline from a Lua script
    Create {
        /// Path to Lua script file
        #[arg(short, long)]
        script: String,

        /// Override pipeline name from script
        #[arg(short, long)]
        name: Option<String>,

        /// Override description from script
        #[arg(short, long)]
        description: Option<String>,

        /// Additional tags (comma-separated)
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
        /// Pipeline ID or unambiguous prefix
        id: String,
    },
    /// Delete a pipeline
    Delete {
        /// Pipeline ID or unambiguous prefix
        id: String,
    },
    /// Launch a job from a pipeline
    Launch {
        /// Pipeline ID or unambiguous prefix
        id: String,

        /// Parameters as key=value pairs (e.g., branch=main repo=myrepo)
        #[arg(short, long, value_parser = parse_key_val)]
        param: Vec<(String, String)>,
    },
}

/// Parse a single key=value pair
fn parse_key_val(s: &str) -> Result<(String, String)> {
    let pos = s
        .find('=')
        .ok_or_else(|| anyhow::anyhow!("invalid KEY=value: no `=` found in `{}`", s))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

/// Handle pipeline commands
///
/// Routes pipeline subcommands to their respective handlers.
///
/// # Arguments
/// * `command` - The pipeline command to execute
/// * `config` - The CLI configuration
pub async fn handle_pipeline_command(command: PipelineCommands, config: &Config) -> Result<()> {
    let client = OrchestratorClient::new(&config.orchestrator_url);

    match command {
        PipelineCommands::Create {
            script,
            name,
            description,
            tags,
            timeout,
            max_retries,
        } => {
            create_pipeline(
                &client,
                &script,
                name,
                description,
                tags,
                timeout,
                max_retries,
            )
            .await
        }
        PipelineCommands::List => list_pipelines(&client).await,
        PipelineCommands::Get { id } => get_pipeline(&client, &id).await,
        PipelineCommands::Delete { id } => delete_pipeline(&client, &id).await,
        PipelineCommands::Launch { id, param } => launch_job(&client, &id, param).await,
    }
}

/// Create a new pipeline from a Lua script
///
/// Parses the Lua script to extract metadata and creates the pipeline.
async fn create_pipeline(
    client: &OrchestratorClient,
    script_path: &str,
    name_override: Option<String>,
    description_override: Option<String>,
    additional_tags: Vec<String>,
    timeout: Option<u64>,
    max_retries: u32,
) -> Result<()> {
    // Read the script file
    let script_content = std::fs::read_to_string(script_path)
        .with_context(|| format!("Failed to read script file: {}", script_path))?;

    // Parse the pipeline metadata using rivet-lua
    let metadata = rivet_lua::parser::parse_pipeline_metadata(&script_content)
        .context("Failed to parse pipeline metadata from Lua script")?;

    // Use overrides or values from metadata
    let name = name_override.unwrap_or(metadata.name);
    let description = description_override.or(metadata.description);

    // Combine tags from script and CLI
    let tags = additional_tags;
    // Note: metadata.requires are treated as required modules, not tags

    // Build the pipeline config
    let config = PipelineConfig {
        timeout_seconds: timeout,
        max_retries,
        env_vars: HashMap::new(),
    };

    // Create the pipeline request
    let req = CreatePipeline {
        name: name.clone(),
        description,
        script: script_content,
        required_modules: metadata.requires,
        tags,
        config: Some(config),
    };

    // Send the request
    let pipeline = client.create_pipeline(req).await?;

    // Display success message
    println!("{}", "✓ Pipeline created successfully!".green().bold());
    println!("  ID:     {}", pipeline.id.to_string().cyan());
    println!("  Name:   {}", pipeline.name.bold());
    println!(
        "  Stages: {}",
        metadata
            .stages
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
            .dimmed()
    );

    if !metadata.inputs.is_empty() {
        println!("  Inputs: {}", metadata.inputs.len().to_string().dimmed());
        for (key, input_def) in metadata.inputs {
            let required = if input_def.required { "*" } else { "" };
            println!(
                "    - {}{}: {} {}",
                key.cyan(),
                required.red(),
                input_def.input_type.dimmed(),
                input_def
                    .description
                    .as_ref()
                    .map(|d| format!("({})", d))
                    .unwrap_or_default()
                    .dimmed()
            );
        }
    }

    Ok(())
}

/// List all pipelines
async fn list_pipelines(client: &OrchestratorClient) -> Result<()> {
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
            print_pipeline_summary(&pipeline);
        }
    }

    Ok(())
}

/// Get and display a single pipeline
async fn get_pipeline(client: &OrchestratorClient, id: &str) -> Result<()> {
    let id_or_prefix = IdOrPrefix::parse(id);
    let uuid = resolve_pipeline_id(client, &id_or_prefix).await?;

    let pipeline = client.get_pipeline(uuid).await?;

    print_pipeline_details(&pipeline);

    Ok(())
}

/// Delete a pipeline
async fn delete_pipeline(client: &OrchestratorClient, id: &str) -> Result<()> {
    let id_or_prefix = IdOrPrefix::parse(id);
    let uuid = resolve_pipeline_id(client, &id_or_prefix).await?;

    client.delete_pipeline(uuid).await?;

    println!(
        "{}",
        format!("✓ Pipeline {} deleted successfully!", uuid)
            .green()
            .bold()
    );

    Ok(())
}

/// Launch a job from a pipeline
async fn launch_job(
    client: &OrchestratorClient,
    id: &str,
    params: Vec<(String, String)>,
) -> Result<()> {
    let id_or_prefix = IdOrPrefix::parse(id);
    let uuid = resolve_pipeline_id(client, &id_or_prefix).await?;

    // Convert parameters from Vec to HashMap<String, JsonValue>
    let parameters: HashMap<String, JsonValue> = params
        .into_iter()
        .map(|(k, v)| (k, JsonValue::String(v)))
        .collect();

    let req = CreateJob {
        pipeline_id: uuid,
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

/// Print a pipeline summary
fn print_pipeline_summary(pipeline: &Pipeline) {
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
        println!("    Description: {}", desc.dimmed());
    }
    if !pipeline.tags.is_empty() {
        println!("    Tags:    {}", pipeline.tags.join(", ").dimmed());
    }
    println!();
}

/// Print detailed pipeline information
fn print_pipeline_details(pipeline: &Pipeline) {
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
    if !pipeline.required_plugins.is_empty() {
        println!("  Modules:     {}", pipeline.required_plugins.join(", "));
    }

    println!("\n{}", "Script:".bold());
    println!("{}", "─".repeat(80).dimmed());
    println!("{}", pipeline.script);
    println!("{}", "─".repeat(80).dimmed());
}
