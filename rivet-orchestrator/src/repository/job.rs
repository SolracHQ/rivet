//! Job Repository
//!
//! Handles all database operations related to jobs.

use rivet_core::domain::job::{Job, JobResult, JobStatus};
use rivet_core::dto::job::CreateJob;
use sqlx::PgPool;
use uuid::Uuid;

/// Create a new job in the database
pub async fn create(pool: &PgPool, req: CreateJob) -> Result<Job, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let job = Job {
        id,
        pipeline_id: req.pipeline_id,
        status: JobStatus::Queued,
        requested_at: now,
        started_at: None,
        completed_at: None,
        runner_id: None,
        parameters: req.parameters.clone(),
        result: None,
    };

    sqlx::query(
        r#"
        INSERT INTO jobs (id, pipeline_id, status, requested_at, parameters)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id)
    .bind(req.pipeline_id)
    .bind("Queued")
    .bind(now)
    .bind(serde_json::to_value(&req.parameters).unwrap())
    .execute(pool)
    .await?;

    Ok(job)
}

/// Find a job by ID
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Job>, sqlx::Error> {
    let row = sqlx::query_as::<_, JobRow>(
        r#"
        SELECT id, pipeline_id, status, requested_at, started_at, completed_at,
               runner_id, parameters, result_success, result_exit_code,
               result_output, result_error_message
        FROM jobs
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into()))
}

/// Find jobs by status
pub async fn find_by_status(pool: &PgPool, status: JobStatus) -> Result<Vec<Job>, sqlx::Error> {
    let status_str = status_to_string(status);

    let rows = sqlx::query_as::<_, JobRow>(
        r#"
        SELECT id, pipeline_id, status, requested_at, started_at, completed_at,
               runner_id, parameters, result_success, result_exit_code,
               result_output, result_error_message
        FROM jobs
        WHERE status = $1
        ORDER BY requested_at ASC
        "#,
    )
    .bind(status_str)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Find jobs by pipeline ID
pub async fn find_by_pipeline(pool: &PgPool, pipeline_id: Uuid) -> Result<Vec<Job>, sqlx::Error> {
    let rows = sqlx::query_as::<_, JobRow>(
        r#"
        SELECT id, pipeline_id, status, requested_at, started_at, completed_at,
               runner_id, parameters, result_success, result_exit_code,
               result_output, result_error_message
        FROM jobs
        WHERE pipeline_id = $1
        ORDER BY requested_at DESC
        "#,
    )
    .bind(pipeline_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Update job status and runner assignment (for starting execution)
/// List all jobs
pub async fn list_all(pool: &PgPool) -> Result<Vec<Job>, sqlx::Error> {
    let rows = sqlx::query_as::<_, JobRow>(
        r#"
        SELECT id, pipeline_id, status, requested_at, started_at, completed_at,
               runner_id, parameters, result_success, result_exit_code,
               result_output, result_error_message
        FROM jobs
        ORDER BY requested_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

pub async fn update_status_to_running(
    pool: &PgPool,
    job_id: Uuid,
    runner_id: String,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now();

    sqlx::query(
        r#"
        UPDATE jobs
        SET status = $1, started_at = $2, runner_id = $3
        WHERE id = $4
        "#,
    )
    .bind("Running")
    .bind(now)
    .bind(runner_id)
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update job status to completed state
pub async fn update_status_to_completed(
    pool: &PgPool,
    job_id: Uuid,
    status: JobStatus,
) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now();
    let status_str = status_to_string(status);

    sqlx::query(
        r#"
        UPDATE jobs
        SET status = $1, completed_at = $2
        WHERE id = $3
        "#,
    )
    .bind(status_str)
    .bind(now)
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Update job result
pub async fn update_result(
    pool: &PgPool,
    job_id: Uuid,
    result: JobResult,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE jobs
        SET result_success = $1, result_exit_code = $2, result_output = $3, result_error_message = $4
        WHERE id = $5
        "#,
    )
    .bind(result.success)
    .bind(result.exit_code)
    .bind(result.output)
    .bind(&result.error_message)
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a job by ID
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM jobs WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// =============================================================================
// Helper Functions
// =============================================================================

fn status_to_string(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "Queued",
        JobStatus::Running => "Running",
        JobStatus::Succeeded => "Succeeded",
        JobStatus::Failed => "Failed",
        JobStatus::Cancelled => "Cancelled",
        JobStatus::TimedOut => "TimedOut",
    }
}

fn string_to_status(s: &str) -> JobStatus {
    match s {
        "Queued" => JobStatus::Queued,
        "Running" => JobStatus::Running,
        "Succeeded" => JobStatus::Succeeded,
        "Failed" => JobStatus::Failed,
        "Cancelled" => JobStatus::Cancelled,
        "TimedOut" => JobStatus::TimedOut,
        _ => JobStatus::Queued,
    }
}

// =============================================================================
// Database Row Types
// =============================================================================

#[derive(sqlx::FromRow)]
struct JobRow {
    id: Uuid,
    pipeline_id: Uuid,
    status: String,
    requested_at: chrono::DateTime<chrono::Utc>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    runner_id: Option<String>,
    parameters: serde_json::Value,
    result_success: Option<bool>,
    result_exit_code: Option<i32>,
    result_output: Option<serde_json::Value>,
    result_error_message: Option<String>,
}

impl From<JobRow> for Job {
    fn from(row: JobRow) -> Self {
        let status = string_to_status(&row.status);

        let result = if let Some(success) = row.result_success {
            Some(JobResult {
                success,
                exit_code: row.result_exit_code.unwrap_or(0),
                output: row.result_output,
                error_message: row.result_error_message,
            })
        } else {
            None
        };

        let parameters = serde_json::from_value(row.parameters).unwrap_or_default();

        Job {
            id: row.id,
            pipeline_id: row.pipeline_id,
            status,
            requested_at: row.requested_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
            runner_id: row.runner_id,
            parameters,
            result,
        }
    }
}
