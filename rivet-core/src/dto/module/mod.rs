//! Module DTOs for inter-service communication

use serde::{Deserialize, Serialize};

/// Module information for registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub version: String,
    pub description: String,
    pub author: String,
}
