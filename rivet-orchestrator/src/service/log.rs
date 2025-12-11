//! Log Service
//!
//! Business logic for job log management.

use rivet_core::types::LogEntry;
use sqlx::PgPool;
use uuid::Uuid;

use crate::repository::log_repository;

/// Service error type
#[derive(Debug)]
pub enum LogError {
    JobNotFound(Uuid),
    ValidationError(String),
    DatabaseError(sqlx::Error),
}

impl From<sqlx::Error> for LogError {
    fn from(err: sqlx::Error) -> Self {
        LogError::DatabaseError(err)
    }
}

pub type Result<T> = std::result::Result<T, LogError>;

/// Add log entries for a job
pub async fn add_log_entries(pool: &PgPool, job_id: Uuid, entries: Vec<LogEntry>) -> Result<()> {
    // Validate entries
    validate_log_entries(&entries)?;

    if entries.is_empty() {
        return Ok(());
    }

    // Add entries to database
    log_repository::add_entries(pool, job_id, entries).await?;

    tracing::debug!("Added log entries for job: {}", job_id);

    Ok(())
}

/// Get all log entries for a job
pub async fn get_job_logs(pool: &PgPool, job_id: Uuid) -> Result<Vec<LogEntry>> {
    let logs = log_repository::find_by_job(pool, job_id).await?;

    Ok(logs)
}

/// Get log count for a job
pub async fn get_log_count(pool: &PgPool, job_id: Uuid) -> Result<i64> {
    let count = log_repository::count_by_job(pool, job_id).await?;

    Ok(count)
}

/// Delete all logs for a job
pub async fn delete_job_logs(pool: &PgPool, job_id: Uuid) -> Result<u64> {
    let deleted = log_repository::delete_by_job(pool, job_id).await?;

    tracing::info!("Deleted {} log entries for job: {}", deleted, job_id);

    Ok(deleted)
}

// =============================================================================
// Validation
// =============================================================================

fn validate_log_entries(entries: &[LogEntry]) -> Result<()> {
    const MAX_MESSAGE_LENGTH: usize = 10_000;
    const MAX_BATCH_SIZE: usize = 1000;

    if entries.len() > MAX_BATCH_SIZE {
        return Err(LogError::ValidationError(format!(
            "Too many log entries in batch (max: {})",
            MAX_BATCH_SIZE
        )));
    }

    for (i, entry) in entries.iter().enumerate() {
        if entry.message.len() > MAX_MESSAGE_LENGTH {
            return Err(LogError::ValidationError(format!(
                "Log entry {} message too long (max: {} chars)",
                i, MAX_MESSAGE_LENGTH
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::types::LogLevel;

    #[test]
    fn test_validate_log_entries_valid() {
        let entries = vec![
            LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Info,
                message: "Test message".to_string(),
            },
            LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Error,
                message: "Error message".to_string(),
            },
        ];

        let result = validate_log_entries(&entries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_log_entries_too_many() {
        let entries: Vec<LogEntry> = (0..1001)
            .map(|i| LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Info,
                message: format!("Message {}", i),
            })
            .collect();

        let result = validate_log_entries(&entries);
        assert!(matches!(result, Err(LogError::ValidationError(_))));
    }

    #[test]
    fn test_validate_log_entries_message_too_long() {
        let entries = vec![LogEntry {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Info,
            message: "x".repeat(10_001),
        }];

        let result = validate_log_entries(&entries);
        assert!(matches!(result, Err(LogError::ValidationError(_))));
    }
}
