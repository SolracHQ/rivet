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
