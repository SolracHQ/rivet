//! Pipeline DTOs for inter-service communication

use serde::{Deserialize, Serialize};

/// Request to create a new pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipeline {
    pub script: String,
}
