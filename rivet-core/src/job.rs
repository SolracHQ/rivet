use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    id: uuid::Uuid,
    pipeline_id: uuid::Uuid,
    requested_at: chrono::DateTime<chrono::Utc>,
}
