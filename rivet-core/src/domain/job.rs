//! Job domain types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
