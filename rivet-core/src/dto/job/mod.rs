//! Job DTOs for inter-service communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::job::{Job, JobResult, JobStatus};

/// Request to create/trigger a new job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJob {
    pub pipeline_id: Uuid,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

/// Job status update from runner to orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub result: Option<JobResult>,
}
