//! Runner domain model
//!
//! Represents a runner that executes jobs from the orchestrator.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A runner that can execute jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    /// Unique identifier for the runner
    pub id: String,

    /// When this runner was first registered
    pub registered_at: DateTime<Utc>,

    /// Last time this runner sent a heartbeat
    pub last_heartbeat_at: DateTime<Utc>,

    /// Current status of the runner
    pub status: RunnerStatus,
}

/// Status of a runner
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnerStatus {
    /// Runner is online and ready to accept jobs
    Online,

    /// Runner hasn't sent a heartbeat recently
    Offline,

    /// Runner is currently executing a job
    Busy,
}

impl std::fmt::Display for RunnerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnerStatus::Online => write!(f, "Online"),
            RunnerStatus::Offline => write!(f, "Offline"),
            RunnerStatus::Busy => write!(f, "Busy"),
        }
    }
}
