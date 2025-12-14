//! Pipeline definition for runtime execution
//!
//! This module provides the full pipeline definition structure that includes
//! Lua functions for stage execution. Unlike PipelineMetadata (which is serializable),
//! PipelineDefinition contains actual Lua function references and is used during execution.

use anyhow::Result;
use mlua::{Function, Lua, Table, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Tag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct InputDefinition {
    pub input_type: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub options: Option<Vec<serde_json::Value>>,
}

/// Full pipeline definition with executable Lua functions
///
/// This structure contains everything needed to execute a pipeline,
/// including the actual Lua functions for stage scripts and conditions.
pub struct PipelineDefinition {
    pub name: String,
    pub description: Option<String>,
    pub inputs: HashMap<String, InputDefinition>,
    pub runner: Vec<Tag>,
    pub plugins: Vec<String>,
    pub stages: Vec<StageDefinition>,
}

/// Stage definition with executable Lua functions
pub struct StageDefinition {
    pub name: String,
    pub container: Option<String>,
    pub condition: Option<Function>,
    pub script: Function,
}

/// Parse a pipeline definition from Lua source code in an execution sandbox
///
/// This function evaluates the pipeline in a Lua execution sandbox and extracts
/// the full definition including stage script functions and condition functions.
///
/// # Arguments
/// * `lua` - The Lua execution sandbox (must have core modules registered)
/// * `source` - The Lua source code defining the pipeline
///
/// # Returns
/// The full pipeline definition with executable functions
///
/// # Errors
/// Returns an error if:
/// - The Lua source is invalid
/// - Required fields are missing (name, stages)
/// - Field types are incorrect
pub fn parse_pipeline_definition(lua: &Lua, source: &str) -> Result<PipelineDefinition> {
    // Evaluate the pipeline definition
    let pipeline: Table = lua
        .load(source)
        .eval()
        .map_err(|e| anyhow::anyhow!("Failed to evaluate pipeline definition: {}", e))?;

    // Extract required field: name
    let name: String = pipeline
        .get("name")
        .map_err(|e| anyhow::anyhow!("Pipeline must have a 'name' field: {}", e))?;

    // Extract optional field: description
    let description: Option<String> = pipeline.get("description").ok();

    // Extract inputs
    let inputs = parse_inputs_from_table(&pipeline)?;

    // Extract runner tags
    let runner = parse_runner_tags_from_table(&pipeline)?;

    // Extract plugins
    let plugins = parse_plugins_from_table(&pipeline)?;

    // Extract stages with functions
    let stages = parse_stages_from_table(&pipeline)?;

    Ok(PipelineDefinition {
        name,
        description,
        inputs,
        runner,
        plugins,
        stages,
    })
}

/// Parse inputs from pipeline table
fn parse_inputs_from_table(pipeline: &Table) -> Result<HashMap<String, InputDefinition>> {
    let inputs_value: Value = pipeline.get("inputs").unwrap_or(Value::Nil);

    match inputs_value {
        Value::Nil => Ok(HashMap::new()),
        Value::Table(table) => {
            let mut inputs = HashMap::new();

            for pair in table.pairs::<String, Table>() {
                let (key, input_table) =
                    pair.map_err(|e| anyhow::anyhow!("Failed to read input entry: {}", e))?;

                let input_type: String = input_table.get("type").map_err(|e| {
                    anyhow::anyhow!("Input '{}' must have a 'type' field: {}", key, e)
                })?;

                let description: Option<String> = input_table.get("description").ok();
                let required: bool = input_table.get("required").unwrap_or(true);

                let default: Option<serde_json::Value> = match input_table.get::<Value>("default") {
                    Ok(ref val) if !matches!(val, Value::Nil) => {
                        Some(lua_value_to_json(val).map_err(|e| {
                            anyhow::anyhow!("Input '{}' has invalid default value type: {}", key, e)
                        })?)
                    }
                    _ => None,
                };

                let options: Option<Vec<serde_json::Value>> = match input_table
                    .get::<Value>("options")
                {
                    Ok(Value::Table(opts_table)) => {
                        let mut opts = Vec::new();
                        for pair in opts_table.sequence_values::<Value>() {
                            let val = pair.map_err(|e| {
                                anyhow::anyhow!("Failed to read option entry: {}", e)
                            })?;
                            let json_val = lua_value_to_json(&val).map_err(|e| {
                                anyhow::anyhow!(
                                    "Input '{}' has invalid option value type: {}",
                                    key,
                                    e
                                )
                            })?;
                            opts.push(json_val);
                        }
                        Some(opts)
                    }
                    Ok(Value::Nil) | Err(_) => None,
                    _ => return Err(anyhow::anyhow!("Input '{}' options must be an array", key)),
                };

                inputs.insert(
                    key,
                    InputDefinition {
                        input_type,
                        description,
                        required,
                        default,
                        options,
                    },
                );
            }

            Ok(inputs)
        }
        _ => Err(anyhow::anyhow!(
            "Field 'inputs' must be a table of input definitions"
        )),
    }
}

/// Parse runner tags from pipeline table
fn parse_runner_tags_from_table(pipeline: &Table) -> Result<Vec<Tag>> {
    let runner_value: Value = pipeline.get("runner").unwrap_or(Value::Nil);

    match runner_value {
        Value::Nil => Ok(Vec::new()),
        Value::Table(table) => {
            let mut tags = Vec::new();
            for pair in table.sequence_values::<Table>() {
                let tag_table =
                    pair.map_err(|e| anyhow::anyhow!("Failed to read runner tag entry: {}", e))?;

                let key: String = tag_table
                    .get("key")
                    .map_err(|e| anyhow::anyhow!("Runner tag must have a 'key' field: {}", e))?;

                let value: String = tag_table
                    .get("value")
                    .map_err(|e| anyhow::anyhow!("Runner tag must have a 'value' field: {}", e))?;

                tags.push(Tag { key, value });
            }
            Ok(tags)
        }
        _ => Err(anyhow::anyhow!(
            "Field 'runner' must be an array of tag tables"
        )),
    }
}

/// Parse plugins from pipeline table
fn parse_plugins_from_table(pipeline: &Table) -> Result<Vec<String>> {
    let plugins_value: Value = pipeline.get("plugins").unwrap_or(Value::Nil);

    match plugins_value {
        Value::Nil => Ok(Vec::new()),
        Value::Table(table) => {
            let mut plugins = Vec::new();
            for pair in table.sequence_values::<String>() {
                let plugin =
                    pair.map_err(|e| anyhow::anyhow!("Failed to read plugins entry: {}", e))?;
                plugins.push(plugin);
            }
            Ok(plugins)
        }
        _ => Err(anyhow::anyhow!(
            "Field 'plugins' must be an array of strings"
        )),
    }
}

/// Parse stages from pipeline table
fn parse_stages_from_table(pipeline: &Table) -> Result<Vec<StageDefinition>> {
    let stages_table: Table = pipeline
        .get("stages")
        .map_err(|e| anyhow::anyhow!("Pipeline must have a 'stages' field: {}", e))?;

    let mut stages = Vec::new();

    for pair in stages_table.sequence_values::<Table>() {
        let stage_table = pair.map_err(|e| anyhow::anyhow!("Failed to read stage entry: {}", e))?;

        let name: String = stage_table
            .get("name")
            .map_err(|e| anyhow::anyhow!("Stage must have a 'name' field: {}", e))?;

        let container: Option<String> = stage_table.get("container").ok();

        let condition: Option<Function> = stage_table.get("condition").ok();

        let script: Function = stage_table.get("script").map_err(|e| {
            anyhow::anyhow!("Stage '{}' must have a 'script' function: {}", name, e)
        })?;

        stages.push(StageDefinition {
            name,
            container,
            condition,
            script,
        });
    }

    if stages.is_empty() {
        return Err(anyhow::anyhow!("Pipeline must have at least one stage"));
    }

    Ok(stages)
}

/// Convert mlua Value to serde_json Value
fn lua_value_to_json(val: &Value) -> Result<serde_json::Value> {
    match val {
        Value::String(s) => Ok(serde_json::Value::String(s.to_str()?.to_string())),
        Value::Number(n) => {
            if let Some(num) = serde_json::Number::from_f64(*n) {
                Ok(serde_json::Value::Number(num))
            } else {
                Err(anyhow::anyhow!("Invalid number value"))
            }
        }
        Value::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
        Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Nil => Ok(serde_json::Value::Null),
        _ => Err(anyhow::anyhow!(
            "Unsupported Lua value type for JSON conversion"
        )),
    }
}
