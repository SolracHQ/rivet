//! Pipeline DTOs for inter-service communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::pipeline::{Pipeline, PipelineConfig};

/// Lightweight pipeline summary for listing
/// Request to create a new pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipeline {
    pub name: String,
    pub description: Option<String>,
    pub script: String,
    pub required_modules: Vec<String>,
    pub tags: Vec<String>,
    pub config: Option<PipelineConfig>,
}
