//! Log Repository
//!
//! Handles all database operations related to job logs.

use rivet_core::domain::log::{LogEntry, LogLevel};
use sqlx::PgPool;
use uuid::Uuid;

/// Add log entries for a job
pub async fn add_entries(
    pool: &PgPool,
    job_id: Uuid,
    entries: Vec<LogEntry>,
) -> Result<(), sqlx::Error> {
    for entry in entries {
        let level_str = level_to_string(entry.level);

        sqlx::query(
            r#"
            INSERT INTO job_logs (job_id, timestamp, level, message)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(job_id)
        .bind(entry.timestamp)
        .bind(level_str)
        .bind(&entry.message)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Get all log entries for a job
pub async fn find_by_job(pool: &PgPool, job_id: Uuid) -> Result<Vec<LogEntry>, sqlx::Error> {
    let rows = sqlx::query_as::<_, LogRow>(
        r#"
        SELECT timestamp, level, message
        FROM job_logs
        WHERE job_id = $1
        ORDER BY timestamp ASC
        "#,
    )
    .bind(job_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Delete all logs for a job
pub async fn delete_by_job(pool: &PgPool, job_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM job_logs WHERE job_id = $1")
        .bind(job_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Get log count for a job
pub async fn count_by_job(pool: &PgPool, job_id: Uuid) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM job_logs WHERE job_id = $1
        "#,
    )
    .bind(job_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

// =============================================================================
// Helper Functions
// =============================================================================

fn level_to_string(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "Debug",
        LogLevel::Info => "Info",
        LogLevel::Warning => "Warning",
        LogLevel::Error => "Error",
    }
}

fn string_to_level(s: &str) -> LogLevel {
    match s {
        "Debug" => LogLevel::Debug,
        "Info" => LogLevel::Info,
        "Warning" => LogLevel::Warning,
        "Error" => LogLevel::Error,
        _ => LogLevel::Info,
    }
}

// =============================================================================
// Database Row Types
// =============================================================================

#[derive(sqlx::FromRow)]
struct LogRow {
    timestamp: chrono::DateTime<chrono::Utc>,
    level: String,
    message: String,
}

impl From<LogRow> for LogEntry {
    fn from(row: LogRow) -> Self {
        let level = string_to_level(&row.level);

        LogEntry {
            timestamp: row.timestamp,
            level,
            message: row.message,
        }
    }
}
