//! Job DTOs for inter-service communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::job::{JobResult, JobStatus};

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

/// Request to execute/claim a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteJobRequest {
    pub runner_id: String,
}

/// Information needed to execute a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecutionInfo {
    /// The job ID
    pub job_id: Uuid,
    /// The pipeline ID
    pub pipeline_id: Uuid,
    /// The pipeline Lua source code
    pub pipeline_source: String,
    /// Job parameters to inject as environment variables
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

/// Request to update job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: JobStatus,
}

/// Request to complete a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteJobRequest {
    pub status: JobStatus,
    pub result: Option<JobResult>,
}
