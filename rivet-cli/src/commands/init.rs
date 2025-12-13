//! Init command handlers
//!
//! Handles initialization of development environment including
//! generation of stub files for modules and .luarc.json configuration.
//!
//! Stubs are fetched from the orchestrator to ensure they're always in sync
//! with the actual module implementations running in the runner.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::*;
use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::config::Config;

/// Init subcommands
#[derive(Subcommand)]
pub enum InitCommands {
    /// Generate Lua development files (.luarc.json and stubs)
    Lua {
        /// Output directory for generated files
        #[arg(short, long, default_value = ".")]
        output: String,

        /// Generate only .luarc.json
        #[arg(long)]
        config_only: bool,

        /// Generate only stub files
        #[arg(long)]
        stubs_only: bool,
    },
}

/// Response from the orchestrator's stub endpoint
#[derive(Deserialize)]
struct StubResponse {
    name: String,
    content: String,
}

/// Handle init commands
///
/// Routes init subcommands to their respective handlers.
///
/// # Arguments
/// * `command` - The init command to execute
/// * `config` - The CLI configuration
pub async fn handle_init_command(command: InitCommands, config: &Config) -> Result<()> {
    match command {
        InitCommands::Lua {
            output,
            config_only,
            stubs_only,
        } => generate_lua_dev_files(&output, config_only, stubs_only, config).await,
    }
}

/// Generate Lua development files
///
/// Creates .luarc.json for LSP configuration and fetches stub files from orchestrator.
async fn generate_lua_dev_files(
    output_dir: &str,
    config_only: bool,
    stubs_only: bool,
    config: &Config,
) -> Result<()> {
    let output_path = Path::new(output_dir);

    // Determine what to generate
    let generate_config = !stubs_only;
    let generate_stubs = !config_only;

    if generate_config {
        generate_luarc_json(output_path)?;
    }

    if generate_stubs {
        fetch_and_save_stubs(output_path, config).await?;
    }

    println!("{}", "✓ Lua development files generated!".green().bold());
    println!();
    println!("{}", "Next steps:".bold());
    println!("  1. Install Lua Language Server in your editor");
    println!("  2. Open your pipeline script to see autocomplete and type hints");
    println!(
        "  3. Use {} to create a pipeline",
        "rivet pipeline create".cyan()
    );

    Ok(())
}

/// Generate .luarc.json for Lua LSP configuration
fn generate_luarc_json(output_path: &Path) -> Result<()> {
    let luarc_path = output_path.join(".luarc.json");

    let luarc_content = r#"{
  "$schema": "https://raw.githubusercontent.com/sumneko/vscode-lua/master/setting/schema.json",
  "runtime": {
    "version": "Lua 5.4"
  },
  "diagnostics": {
    "globals": ["log", "input", "output", "process", "container"]
  },
  "workspace": {
    "library": [".rivet/stubs"],
    "checkThirdParty": false
  },
  "completion": {
    "callSnippet": "Both"
  }
}
"#;

    fs::write(&luarc_path, luarc_content)
        .with_context(|| format!("Failed to write .luarc.json to {:?}", luarc_path))?;

    println!("  {} .luarc.json", "Created".green());

    Ok(())
}

/// Fetch stub files from orchestrator and save them locally
async fn fetch_and_save_stubs(output_path: &Path, config: &Config) -> Result<()> {
    let stubs_dir = output_path.join(".rivet").join("stubs");
    fs::create_dir_all(&stubs_dir)
        .with_context(|| format!("Failed to create stubs directory at {:?}", stubs_dir))?;

    let client = reqwest::Client::new();
    let orchestrator_url = &config.orchestrator_url;

    // Get list of available stubs
    let stubs_list: Vec<String> = client
        .get(format!("{}/api/stubs", orchestrator_url))
        .send()
        .await
        .context("Failed to fetch stubs list from orchestrator")?
        .json()
        .await
        .context("Failed to parse stubs list response")?;

    // Fetch each stub file
    for stub_name in stubs_list {
        let stub_response: StubResponse = client
            .get(format!("{}/api/stubs/{}", orchestrator_url, stub_name))
            .send()
            .await
            .with_context(|| format!("Failed to fetch stub '{}'", stub_name))?
            .json()
            .await
            .with_context(|| format!("Failed to parse stub '{}' response", stub_name))?;

        let stub_path = stubs_dir.join(&stub_response.name);
        fs::write(&stub_path, stub_response.content)
            .with_context(|| format!("Failed to write stub file {:?}", stub_path))?;

        println!("  {} {}", "Fetched".green(), stub_response.name);
    }

    println!(
        "  {} in {}",
        "Stubs ready".green(),
        stubs_dir.display().to_string().cyan()
    );

    Ok(())
}
