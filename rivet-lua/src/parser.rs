//! Pipeline metadata parser
//!
//! This module provides functionality to parse Lua pipeline definitions
//! and extract metadata (name, description, inputs, requires, stages)
//! without executing any stage scripts.
//!
//! Uses the metadata sandbox to safely evaluate pipeline structure.

use anyhow::{Context, Result};
use mlua::{Table, Value};
use rivet_core::domain::pipeline::{InputDefinition, PipelineMetadata, StageMetadata};
use std::collections::HashMap;

use crate::sandbox::create_metadata_sandbox;

/// Parse pipeline metadata from Lua source code
///
/// This function evaluates the pipeline definition in a metadata sandbox
/// and extracts structural information without executing any stage scripts.
///
/// # Arguments
/// * `source` - The Lua source code defining the pipeline
///
/// # Returns
/// The parsed pipeline metadata
///
/// # Errors
/// Returns an error if:
/// - The Lua source is invalid
/// - Required fields are missing (name, stages)
/// - Field types are incorrect
///
/// # Example
/// ```no_run
/// use rivet_lua::parser::parse_pipeline_metadata;
///
/// let source = r#"
///     return {
///         name = "Build Pipeline",
///         description = "Builds and tests the project",
///         requires = {"process", "plugin.git"},
///         inputs = {
///             branch = {
///                 type = "string",
///                 description = "Git branch to build",
///                 required = false,
///                 default = "main"
///             }
///         },
///         stages = {
///             { name = "checkout", script = function() end },
///             { name = "test", script = function() end }
///         }
///     }
/// "#;
///
/// let metadata = parse_pipeline_metadata(source)?;
/// assert_eq!(metadata.name, "Build Pipeline");
/// assert_eq!(metadata.stages.len(), 2);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_pipeline_metadata(source: &str) -> Result<PipelineMetadata> {
    let lua = create_metadata_sandbox().context("Failed to create metadata sandbox")?;

    // Evaluate the pipeline definition
    let pipeline: Table = lua
        .load(source)
        .eval()
        .context("Failed to evaluate pipeline definition")?;

    // Extract required field: name
    let name: String = pipeline
        .get("name")
        .context("Pipeline must have a 'name' field")?;

    // Extract optional field: description
    let description: Option<String> = pipeline.get("description").ok();

    // Extract requires array (optional, defaults to empty)
    let requires = parse_requires(&pipeline)?;

    // Extract inputs table (optional, defaults to empty)
    let inputs = parse_inputs(&pipeline)?;

    // Extract required field: stages
    let stages = parse_stages(&pipeline)?;

    Ok(PipelineMetadata {
        name,
        description,
        requires,
        inputs,
        stages,
    })
}

/// Parse the 'requires' field from pipeline table
fn parse_requires(pipeline: &Table) -> Result<Vec<String>> {
    let requires_value: Value = pipeline.get("requires").unwrap_or(Value::Nil);

    match requires_value {
        Value::Nil => Ok(Vec::new()),
        Value::Table(table) => {
            let mut requires = Vec::new();
            for pair in table.sequence_values::<String>() {
                let req = pair.context("Failed to read requires entry")?;
                requires.push(req);
            }
            Ok(requires)
        }
        _ => Err(anyhow::anyhow!(
            "Field 'requires' must be an array of strings"
        )),
    }
}

/// Parse the 'inputs' field from pipeline table
fn parse_inputs(pipeline: &Table) -> Result<HashMap<String, InputDefinition>> {
    let inputs_value: Value = pipeline.get("inputs").unwrap_or(Value::Nil);

    match inputs_value {
        Value::Nil => Ok(HashMap::new()),
        Value::Table(table) => {
            let mut inputs = HashMap::new();

            for pair in table.pairs::<String, Table>() {
                let (key, input_table) = pair.context("Failed to read input entry")?;

                let input_type: String = input_table
                    .get("type")
                    .context(format!("Input '{}' must have a 'type' field", key))?;

                let description: Option<String> = input_table.get("description").ok();

                // Parse 'required' field
                let required: bool = match input_table.get("required") {
                    Ok(val) => val,
                    Err(_) => {
                        // For backward compatibility, check 'optional' field
                        let optional: bool = input_table.get("optional").unwrap_or(false);
                        !optional
                    }
                };

                let default: Option<String> = input_table.get("default").ok();

                inputs.insert(
                    key,
                    InputDefinition {
                        input_type,
                        description,
                        required,
                        default,
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

/// Parse the 'stages' field from pipeline table
fn parse_stages(pipeline: &Table) -> Result<Vec<StageMetadata>> {
    let stages_table: Table = pipeline
        .get("stages")
        .context("Pipeline must have a 'stages' field")?;

    let mut stages = Vec::new();

    for pair in stages_table.sequence_values::<Table>() {
        let stage_table = pair.context("Failed to read stage entry")?;

        let name: String = stage_table
            .get("name")
            .context("Stage must have a 'name' field")?;

        let container: Option<String> = stage_table.get("container").ok();

        stages.push(StageMetadata { name, container });
    }

    if stages.is_empty() {
        return Err(anyhow::anyhow!("Pipeline must have at least one stage"));
    }

    Ok(stages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_pipeline() {
        let source = r#"
            return {
                name = "Minimal Pipeline",
                stages = {
                    { name = "stage1", script = function() end }
                }
            }
        "#;

        let metadata = parse_pipeline_metadata(source).unwrap();
        assert_eq!(metadata.name, "Minimal Pipeline");
        assert_eq!(metadata.description, None);
        assert_eq!(metadata.requires.len(), 0);
        assert_eq!(metadata.inputs.len(), 0);
        assert_eq!(metadata.stages.len(), 1);
        assert_eq!(metadata.stages[0].name, "stage1");
    }

    #[test]
    fn test_parse_full_pipeline() {
        let source = r#"
            return {
                name = "Full Pipeline",
                description = "A complete example",
                requires = {"process", "plugin.git"},
                inputs = {
                    repo_url = {
                        type = "string",
                        description = "Repository URL",
                        required = true
                    },
                    branch = {
                        type = "string",
                        required = false,
                        default = "main"
                    }
                },
                stages = {
                    { name = "checkout", script = function() end },
                    { name = "build", script = function() end, container = "rust:latest" }
                }
            }
        "#;

        let metadata = parse_pipeline_metadata(source).unwrap();
        assert_eq!(metadata.name, "Full Pipeline");
        assert_eq!(metadata.description, Some("A complete example".to_string()));
        assert_eq!(metadata.requires, vec!["process", "plugin.git"]);
        assert_eq!(metadata.inputs.len(), 2);
        assert!(metadata.inputs.contains_key("repo_url"));
        assert!(metadata.inputs.contains_key("branch"));
        assert_eq!(metadata.inputs["repo_url"].input_type, "string");
        assert_eq!(metadata.inputs["repo_url"].required, true);
        assert_eq!(metadata.inputs["branch"].required, false);
        assert_eq!(metadata.inputs["branch"].default, Some("main".to_string()));
        assert_eq!(metadata.stages.len(), 2);
        assert_eq!(metadata.stages[0].name, "checkout");
        assert_eq!(metadata.stages[0].container, None);
        assert_eq!(metadata.stages[1].name, "build");
        assert_eq!(
            metadata.stages[1].container,
            Some("rust:latest".to_string())
        );
    }

    #[test]
    fn test_parse_pipeline_missing_name() {
        let source = r#"
            return {
                stages = {
                    { name = "stage1", script = function() end }
                }
            }
        "#;

        let result = parse_pipeline_metadata(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[test]
    fn test_parse_pipeline_missing_stages() {
        let source = r#"
            return {
                name = "No Stages"
            }
        "#;

        let result = parse_pipeline_metadata(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("stages"));
    }

    #[test]
    fn test_parse_pipeline_empty_stages() {
        let source = r#"
            return {
                name = "Empty Stages",
                stages = {}
            }
        "#;

        let result = parse_pipeline_metadata(source);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least one stage")
        );
    }

    #[test]
    fn test_parse_stage_missing_name() {
        let source = r#"
            return {
                name = "Bad Stage",
                stages = {
                    { script = function() end }
                }
            }
        "#;

        let result = parse_pipeline_metadata(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_input_missing_type() {
        let source = r#"
            return {
                name = "Bad Input",
                inputs = {
                    param1 = {
                        description = "Missing type"
                    }
                },
                stages = {
                    { name = "stage1", script = function() end }
                }
            }
        "#;

        let result = parse_pipeline_metadata(source);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("type"));
    }

    #[test]
    fn test_parse_invalid_lua() {
        let source = "this is not valid lua!!!";

        let result = parse_pipeline_metadata(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pipeline_not_returning_table() {
        let source = r#"return "not a table""#;

        let result = parse_pipeline_metadata(source);
        assert!(result.is_err());
    }
}
