//! Init command handlers
//!
//! Handles initialization of development environment including
//! generation of stub files for modules and .luarc.json configuration.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::*;
use rivet_lua::module::RivetModule;
use rivet_lua::modules::{EnvModule, LogModule};
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

/// Handle init commands
///
/// Routes init subcommands to their respective handlers.
///
/// # Arguments
/// * `command` - The init command to execute
/// * `config` - The CLI configuration
pub async fn handle_init_command(command: InitCommands, _config: &Config) -> Result<()> {
    match command {
        InitCommands::Lua {
            output,
            config_only,
            stubs_only,
        } => generate_lua_dev_files(&output, config_only, stubs_only).await,
    }
}

/// Generate Lua development files
///
/// Creates .luarc.json for LSP configuration and stub files for module autocompletion.
async fn generate_lua_dev_files(
    output_dir: &str,
    config_only: bool,
    stubs_only: bool,
) -> Result<()> {
    let output_path = Path::new(output_dir);

    // Determine what to generate
    let generate_config = !stubs_only;
    let generate_stubs = !config_only;

    if generate_config {
        generate_luarc_json(output_path)?;
    }

    if generate_stubs {
        generate_stub_files(output_path)?;
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
    "globals": ["log", "env"]
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

/// Generate stub files for Rivet modules
///
/// Uses the actual module implementations to generate stubs,
/// ensuring they stay in sync with the real modules.
fn generate_stub_files(output_path: &Path) -> Result<()> {
    let stubs_dir = output_path.join(".rivet").join("stubs");
    fs::create_dir_all(&stubs_dir)
        .with_context(|| format!("Failed to create stubs directory at {:?}", stubs_dir))?;

    // Create stub implementations of the modules
    // These use no-op providers since we only need the stub generation
    let log_module = create_log_module();
    let env_module = create_env_module();

    // Generate individual stub files
    let modules: Vec<(&str, Box<dyn RivetModule>)> =
        vec![("log", Box::new(log_module)), ("env", Box::new(env_module))];

    for (name, module) in modules {
        let stub_content = module.stubs();
        let stub_path = stubs_dir.join(format!("{}.lua", name));

        fs::write(&stub_path, stub_content)
            .with_context(|| format!("Failed to write stub file {:?}", stub_path))?;

        println!("  {} {}.lua", "Created".green(), name);
    }

    println!(
        "  {} in {}",
        "Stubs ready".green(),
        stubs_dir.display().to_string().cyan()
    );

    Ok(())
}

/// Create a log module instance for stub generation
///
/// Uses a no-op sink since we only need the stub output
fn create_log_module() -> LogModule<NoOpLogSink> {
    LogModule::new(NoOpLogSink)
}

/// Create an env module instance for stub generation
///
/// Uses a no-op provider since we only need the stub output
fn create_env_module() -> EnvModule<NoOpVarProvider> {
    EnvModule::new(NoOpVarProvider)
}

/// No-op log sink for stub generation
struct NoOpLogSink;

impl rivet_lua::modules::LogSink for NoOpLogSink {
    fn write(&mut self, _level: rivet_core::domain::log::LogLevel, _message: &str) {
        // No-op: we only need this for stub generation
    }
}

/// No-op variable provider for stub generation
struct NoOpVarProvider;

impl rivet_lua::modules::VarProvider for NoOpVarProvider {
    fn get(&self, _name: &str) -> Option<String> {
        None
    }

    fn keys(&self) -> Vec<String> {
        Vec::new()
    }
}
