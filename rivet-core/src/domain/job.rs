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

impl JobResult {
    /// Creates a successful job result
    pub fn success() -> Self {
        Self {
            success: true,
            exit_code: 0,
            output: None,
            error_message: None,
        }
    }

    /// Creates a successful job result with output
    pub fn success_with_output(output: serde_json::Value) -> Self {
        Self {
            success: true,
            exit_code: 0,
            output: Some(output),
            error_message: None,
        }
    }

    /// Creates a failed job result with error message and exit code
    pub fn error(error_message: String, exit_code: i32) -> Self {
        Self {
            success: false,
            exit_code,
            output: None,
            error_message: Some(error_message),
        }
    }

    /// Creates a failed job result with default exit code of 1
    pub fn failed(error_message: String) -> Self {
        Self::error(error_message, 1)
    }
}
