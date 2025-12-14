//! Pipeline command handlers
//!
//! Handles all pipeline-related CLI commands including creation,
//! listing, viewing, deletion, and launching jobs.

use anyhow::Result;
use clap::Subcommand;
use colored::*;
use rivet_core::domain::pipeline::Pipeline;
use rivet_core::dto::job::CreateJob;
use rivet_core::dto::pipeline::CreatePipeline;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::io::{self, Write};

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
        script: String,
    },
    /// Check pipeline syntax and display information
    Check {
        /// Path to Lua script file
        script: String,
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

        /// Skip interactive input prompts, use only provided params
        #[arg(long)]
        no_interactive: bool,
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
        PipelineCommands::Create { script } => create_pipeline(&client, &script).await,
        PipelineCommands::Check { script } => check_pipeline(&script).await,
        PipelineCommands::List => list_pipelines(&client).await,
        PipelineCommands::Get { id } => get_pipeline(&client, &id).await,
        PipelineCommands::Delete { id } => delete_pipeline(&client, &id).await,
        PipelineCommands::Launch {
            id,
            param,
            no_interactive,
        } => launch_job(&client, &id, param, no_interactive).await,
    }
}

/// Create a new pipeline from a Lua script
async fn create_pipeline(client: &OrchestratorClient, script_path: &str) -> Result<()> {
    let script_content = std::fs::read_to_string(script_path)
        .map_err(|e| anyhow::anyhow!("Failed to read script file '{}': {}", script_path, e))?;

    // Validate pipeline by parsing definition
    let lua = rivet_lua::create_sandbox()
        .map_err(|e| anyhow::anyhow!("Failed to create sandbox: {}", e))?;
    let definition = rivet_lua::parse_pipeline_definition(&lua, &script_content)?;

    let req = CreatePipeline {
        script: script_content,
    };

    let pipeline = client.create_pipeline(req).await?;

    println!("{}", "✓ Pipeline created successfully!".green().bold());
    println!("  ID:     {}", pipeline.id.to_string().cyan());
    println!("  Name:   {}", pipeline.name.bold());
    println!(
        "  Stages: {}",
        definition
            .stages
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
            .dimmed()
    );

    if !definition.inputs.is_empty() {
        println!("  Inputs: {}", definition.inputs.len().to_string().dimmed());
        for (key, input_def) in definition.inputs {
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

/// Check pipeline syntax and display information
async fn check_pipeline(script_path: &str) -> Result<()> {
    let script_content = std::fs::read_to_string(script_path)
        .map_err(|e| anyhow::anyhow!("Failed to read script file '{}': {}", script_path, e))?;

    let lua = rivet_lua::create_sandbox()
        .map_err(|e| anyhow::anyhow!("Failed to create sandbox: {}", e))?;
    let definition = rivet_lua::parse_pipeline_definition(&lua, &script_content)?;

    println!("{}", "✓ Pipeline is valid!".green().bold());
    println!();
    println!("{}", "Pipeline Information:".bold());
    println!("  Name:        {}", definition.name.cyan());
    if let Some(desc) = &definition.description {
        println!("  Description: {}", desc.dimmed());
    }

    if !definition.plugins.is_empty() {
        println!("  Plugins:     {}", definition.plugins.join(", ").yellow());
    }

    if !definition.runner.is_empty() {
        println!("  Runner tags:");
        for tag in &definition.runner {
            println!("    - {}={}", tag.key.cyan(), tag.value.dimmed());
        }
    }

    if !definition.inputs.is_empty() {
        println!();
        println!("{}", "Inputs:".bold());
        for (key, input_def) in &definition.inputs {
            let required = if input_def.required { "*" } else { "" };
            println!(
                "  - {}{}: {}",
                key.cyan(),
                required.red(),
                input_def.input_type.dimmed()
            );
            if let Some(desc) = &input_def.description {
                println!("      {}", desc.dimmed());
            }
            if let Some(default) = &input_def.default {
                let default_str = match default {
                    JsonValue::String(s) => s.clone(),
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Bool(b) => b.to_string(),
                    _ => format!("{:?}", default),
                };
                println!("      Default: {}", default_str.dimmed());
            }
        }
    }

    println!();
    println!(
        "{}",
        format!("Stages ({}):", definition.stages.len()).bold()
    );
    for (idx, stage) in definition.stages.iter().enumerate() {
        println!("  {}. {}", idx + 1, stage.name.cyan());
        if let Some(container) = &stage.container {
            println!("      Container: {}", container.yellow());
        }
        if stage.condition.is_some() {
            println!("      {}", "Has condition".dimmed());
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
    no_interactive: bool,
) -> Result<()> {
    let id_or_prefix = IdOrPrefix::parse(id);
    let uuid = resolve_pipeline_id(client, &id_or_prefix).await?;

    // Get pipeline to extract definition
    let pipeline = client.get_pipeline(uuid).await?;

    // Parse pipeline definition to get input schema
    let lua = rivet_lua::create_sandbox()
        .map_err(|e| anyhow::anyhow!("Failed to create sandbox: {}", e))?;
    let definition = rivet_lua::parse_pipeline_definition(&lua, &pipeline.script)?;

    // Convert CLI params to HashMap
    let mut provided_params: HashMap<String, String> = params.into_iter().collect();

    // Collect and validate inputs
    let parameters = if no_interactive {
        // Non-interactive mode: validate and apply defaults
        collect_params_non_interactive(&definition, provided_params)?
    } else {
        // Interactive mode: prompt for missing inputs
        collect_params_interactive(&definition, &mut provided_params)?
    };

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

/// Collect parameters in non-interactive mode (validate and apply defaults)
fn collect_params_non_interactive(
    definition: &rivet_lua::PipelineDefinition,
    provided: HashMap<String, String>,
) -> Result<HashMap<String, JsonValue>> {
    let mut parameters = HashMap::new();

    for (key, input_def) in &definition.inputs {
        if let Some(value) = provided.get(key) {
            // Validate and convert type
            let json_value = validate_and_convert_input(key, value, &input_def.input_type)?;
            parameters.insert(key.clone(), json_value);
        } else if let Some(default) = &input_def.default {
            // Use default value
            parameters.insert(key.clone(), default.clone());
        } else if input_def.required {
            return Err(anyhow::anyhow!(
                "Missing required input '{}' ({}). Use -p {}=<value> or run without --no-interactive",
                key,
                input_def.input_type,
                key
            ));
        }
    }

    Ok(parameters)
}

/// Collect parameters interactively (prompt user for missing inputs)
fn collect_params_interactive(
    definition: &rivet_lua::PipelineDefinition,
    provided: &mut HashMap<String, String>,
) -> Result<HashMap<String, JsonValue>> {
    let mut parameters = HashMap::new();

    if definition.inputs.is_empty() {
        return Ok(parameters);
    }

    println!();
    println!("{}", "Pipeline Inputs:".bold());
    println!();

    for (key, input_def) in &definition.inputs {
        // Check if already provided via CLI
        if let Some(value) = provided.get(key) {
            let json_value = validate_and_convert_input(key, value, &input_def.input_type)?;
            parameters.insert(key.clone(), json_value);
            println!(
                "  {} {} (from CLI: {})",
                "✓".green(),
                key.cyan(),
                value.dimmed()
            );
            continue;
        }

        // Show input information
        let required_mark = if input_def.required { "*" } else { "" };
        print!(
            "  {}{} ({}):",
            key.cyan(),
            required_mark.red(),
            input_def.input_type.dimmed()
        );

        if let Some(desc) = &input_def.description {
            print!(" {}", desc.dimmed());
        }
        println!();

        // Show default if available
        if let Some(default) = &input_def.default {
            let default_str = match default {
                JsonValue::String(s) => s.clone(),
                JsonValue::Number(n) => n.to_string(),
                JsonValue::Bool(b) => b.to_string(),
                _ => format!("{:?}", default),
            };
            println!("    Default: {}", default_str.dimmed());
        }

        // Show options if available
        if let Some(options) = &input_def.options {
            println!(
                "    Options: {}",
                options
                    .iter()
                    .map(|v| match v {
                        JsonValue::String(s) => s.clone(),
                        JsonValue::Number(n) => n.to_string(),
                        JsonValue::Bool(b) => b.to_string(),
                        _ => format!("{:?}", v),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
                    .dimmed()
            );
        }

        // Prompt for input
        print!("    Enter value");
        if !input_def.required {
            print!(" (or press Enter to skip)");
        }
        print!(": ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            if let Some(default) = &input_def.default {
                // Use default
                parameters.insert(key.clone(), default.clone());
                println!("    {} Using default", "→".dimmed());
            } else if input_def.required {
                return Err(anyhow::anyhow!("Input '{}' is required", key));
            }
        } else {
            // Validate and convert
            let json_value = validate_and_convert_input(key, input, &input_def.input_type)?;

            // Validate options if provided
            if let Some(options) = &input_def.options {
                let value_matches = options.iter().any(|opt| match (&json_value, opt) {
                    (JsonValue::Number(a), JsonValue::Number(b)) => a.as_f64() == b.as_f64(),
                    (JsonValue::String(a), JsonValue::String(b)) => a == b,
                    (JsonValue::Bool(a), JsonValue::Bool(b)) => a == b,
                    _ => false,
                });

                if !value_matches {
                    return Err(anyhow::anyhow!(
                        "Invalid value for '{}'. Must be one of: {}",
                        key,
                        options
                            .iter()
                            .map(|v| match v {
                                JsonValue::String(s) => s.clone(),
                                JsonValue::Number(n) => n.to_string(),
                                JsonValue::Bool(b) => b.to_string(),
                                _ => format!("{:?}", v),
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }

            parameters.insert(key.clone(), json_value);
        }
        println!();
    }

    Ok(parameters)
}

/// Validate and convert input string to appropriate JSON type
fn validate_and_convert_input(name: &str, value: &str, input_type: &str) -> Result<JsonValue> {
    match input_type {
        "string" => Ok(JsonValue::String(value.to_string())),
        "number" => {
            let num: f64 = value.parse().map_err(|_| {
                anyhow::anyhow!("Input '{}' must be a number, got: {}", name, value)
            })?;
            Ok(serde_json::json!(num))
        }
        "bool" => {
            let bool_val = match value.to_lowercase().as_str() {
                "true" | "yes" | "1" | "y" => true,
                "false" | "no" | "0" | "n" => false,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Input '{}' must be a boolean (true/false), got: {}",
                        name,
                        value
                    ));
                }
            };
            Ok(JsonValue::Bool(bool_val))
        }
        _ => Err(anyhow::anyhow!("Unknown input type: {}", input_type)),
    }
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
        println!(
            "    Tags:    {}",
            pipeline
                .tags
                .iter()
                .map(|t| format!("{}={}", t.key, t.value))
                .collect::<Vec<_>>()
                .join(", ")
                .dimmed()
        );
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
        println!("  Tags:        {} tags", pipeline.tags.len());
    }

    println!("\n{}", "Script:".bold());
    println!("{}", "─".repeat(80).dimmed());
    println!("{}", pipeline.script);
    println!("{}", "─".repeat(80).dimmed());
}
