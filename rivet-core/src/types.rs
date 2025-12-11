//! Core types and DTOs for Rivet
//!
//! This module contains:
//! - Shared domain types (Pipeline, Job, etc.) - structure only
//! - DTOs for inter-service communication
//!
//! Note: Persistence logic lives in orchestrator, execution logic in runner.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// Shared Domain Types
// =============================================================================

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
    pub env_vars: std::collections::HashMap<String, String>,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: Some(3600),
            max_retries: 0,
            env_vars: std::collections::HashMap::new(),
        }
    }
}

/// Job execution record
///
/// Structure shared between orchestrator (persists) and runner (updates).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub status: JobStatus,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub runner_id: Option<String>,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
    pub result: Option<JobResult>,
}

/// Job execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

/// Result of a job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub success: bool,
    pub exit_code: i32,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

/// A log entry from job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

// =============================================================================
// DTOs (Data Transfer Objects)
// =============================================================================

/// Lightweight pipeline summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
}

impl From<Pipeline> for PipelineDto {
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
pub struct CreatePipelineRequest {
    pub name: String,
    pub description: Option<String>,
    pub script: String,
    pub required_modules: Vec<String>,
    pub tags: Vec<String>,
    pub config: Option<PipelineConfig>,
}

/// Job summary for listing and status updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDto {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub status: JobStatus,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub runner_id: Option<String>,
}

impl From<Job> for JobDto {
    fn from(job: Job) -> Self {
        Self {
            id: job.id,
            pipeline_id: job.pipeline_id,
            status: job.status,
            requested_at: job.requested_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            runner_id: job.runner_id,
        }
    }
}

/// Request to create/trigger a new job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub pipeline_id: Uuid,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

/// Job status update from runner to orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusUpdate {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub result: Option<JobResult>,
}

/// Log batch sent from runner to orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogBatch {
    pub job_id: Uuid,
    pub entries: Vec<LogEntry>,
}

/// Module information for registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub version: String,
    pub description: String,
    pub author: String,
}

impl From<crate::module::ModuleMetadata> for ModuleInfo {
    fn from(meta: crate::module::ModuleMetadata) -> Self {
        Self {
            id: meta.id.to_string(),
            version: meta.version.to_string(),
            description: meta.description.to_string(),
            author: meta.author.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_dto_conversion() {
        let pipeline = Pipeline {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            description: Some("desc".to_string()),
            script: "-- lua code".to_string(),
            required_modules: vec!["log".to_string()],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            config: PipelineConfig::default(),
        };

        let dto: PipelineDto = pipeline.clone().into();
        assert_eq!(dto.id, pipeline.id);
        assert_eq!(dto.name, pipeline.name);
    }

    #[test]
    fn test_job_dto_conversion() {
        let job = Job {
            id: Uuid::new_v4(),
            pipeline_id: Uuid::new_v4(),
            status: JobStatus::Running,
            requested_at: chrono::Utc::now(),
            started_at: Some(chrono::Utc::now()),
            completed_at: None,
            runner_id: Some("runner-1".to_string()),
            parameters: std::collections::HashMap::new(),
            result: None,
        };

        let dto: JobDto = job.clone().into();
        assert_eq!(dto.id, job.id);
        assert_eq!(dto.status, job.status);
    }
}
