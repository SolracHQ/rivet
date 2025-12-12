//! Pipeline DTOs for inter-service communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::pipeline::{Pipeline, PipelineConfig};

/// Lightweight pipeline summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

impl From<Pipeline> for PipelineSummary {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id,
            name: pipeline.name,
            description: pipeline.description,
            created_at: pipeline.created_at,
            updated_at: pipeline.updated_at,
            tags: pipeline.tags,
        }
    }
}

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
