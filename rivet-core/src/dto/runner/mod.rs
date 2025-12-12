//! Runner DTOs
//!
//! Data transfer objects for runner-related operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::runner::{Runner, RunnerStatus};

/// Request to register a runner with the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRunner {
    /// Unique identifier for the runner
    pub runner_id: String,

    /// List of capabilities this runner supports
    pub capabilities: Vec<String>,
}

/// Summary information about a runner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerSummary {
    /// Unique identifier for the runner
    pub id: String,

    /// Number of capabilities this runner supports
    pub capability_count: usize,

    /// When this runner was first registered
    pub registered_at: DateTime<Utc>,

    /// Last time this runner sent a heartbeat
    pub last_heartbeat_at: DateTime<Utc>,

    /// Current status of the runner
    pub status: RunnerStatus,
}

impl From<Runner> for RunnerSummary {
    fn from(runner: Runner) -> Self {
        RunnerSummary {
            id: runner.id,
            capability_count: runner.capabilities.len(),
            registered_at: runner.registered_at,
            last_heartbeat_at: runner.last_heartbeat_at,
            status: runner.status,
        }
    }
}
