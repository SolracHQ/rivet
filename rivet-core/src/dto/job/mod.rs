//! Job DTOs for inter-service communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::job::{Job, JobResult, JobStatus};

/// Job summary for listing and status updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: Uuid,
    pub pipeline_id: Uuid,
    pub status: JobStatus,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub runner_id: Option<String>,
}

impl From<Job> for JobSummary {
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
