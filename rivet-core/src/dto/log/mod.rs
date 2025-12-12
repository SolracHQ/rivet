//! Log DTOs for inter-service communication

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::log::LogEntry;

/// Log batch sent from runner to orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogBatch {
    pub job_id: Uuid,
    pub entries: Vec<LogEntry>,
}
