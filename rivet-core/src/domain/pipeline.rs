//! Pipeline domain types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Pipeline definition
///
/// Structure shared between orchestrator (persists) and runner (executes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub script: String,
    pub required_modules: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub config: PipelineConfig,
}

/// Pipeline configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub timeout_seconds: Option<u64>,
    pub max_retries: u32,
    pub env_vars: HashMap<String, String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(3600),
            max_retries: 0,
            env_vars: HashMap::new(),
        }
    }
}

/// Pipeline metadata extracted from Lua definition
///
/// This structure contains the parsed metadata from a pipeline definition,
/// including inputs, requirements, and stage information (but not the executable code).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineMetadata {
    pub name: String,
    pub description: Option<String>,
    pub requires: Vec<String>,
    pub inputs: HashMap<String, InputDefinition>,
    pub stages: Vec<StageMetadata>,
}

/// Input definition for a pipeline parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDefinition {
    #[serde(rename = "type")]
    pub input_type: String,
    pub description: Option<String>,
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

/// Stage metadata (name and optional container)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMetadata {
    pub name: String,
    pub container: Option<String>,
}
