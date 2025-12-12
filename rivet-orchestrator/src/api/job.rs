//! Job API Handlers
//!
//! HTTP endpoints for job lifecycle management.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use rivet_core::domain::job::{Job, JobResult, JobStatus};
use rivet_core::domain::log::LogEntry;
use rivet_core::dto::job::CreateJob;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::error::{ApiError, ApiResult};
use crate::service::{job_service, log_service};

// =============================================================================
// Job Lifecycle Endpoints
// =============================================================================

/// POST /pipeline/launch
/// Create and launch a new job for a pipeline
pub async fn launch_job(
    State(pool): State<PgPool>,
    Json(req): Json<CreateJob>,
) -> ApiResult<Json<Job>> {
    tracing::info!("Launching job for pipeline: {}", req.pipeline_id);

    let job = job_service::launch_job(&pool, req)
        .await
        .map_err(|e| match e {
            job_service::JobError::PipelineNotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
            job_service::JobError::ValidationError(msg) => ApiError::BadRequest(msg),
            job_service::JobError::DatabaseError(err) => ApiError::DatabaseError(err),
            job_service::JobError::NotFound(id) => {
                ApiError::NotFound(format!("Job {} not found", id))
            }
            job_service::JobError::InvalidState(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(Json(job))
}

/// GET /job/{id}
/// Get job details by ID
pub async fn get_job(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> ApiResult<Json<Job>> {
    tracing::debug!("Getting job: {}", id);

    let job = job_service::get_job(&pool, id).await.map_err(|e| match e {
        job_service::JobError::NotFound(id) => ApiError::NotFound(format!("Job {} not found", id)),
        job_service::JobError::DatabaseError(err) => ApiError::DatabaseError(err),
        job_service::JobError::PipelineNotFound(id) => {
            ApiError::NotFound(format!("Pipeline {} not found", id))
        }
        job_service::JobError::ValidationError(msg) => ApiError::BadRequest(msg),
        job_service::JobError::InvalidState(msg) => ApiError::BadRequest(msg),
    })?;

    Ok(Json(job))
}

/// GET /jobs
/// List all jobs
pub async fn list_all_jobs(State(pool): State<PgPool>) -> ApiResult<Json<Vec<Job>>> {
    tracing::debug!("Listing all jobs");

    let jobs = job_service::list_all_jobs(&pool)
        .await
        .map_err(|e| match e {
            job_service::JobError::DatabaseError(err) => ApiError::DatabaseError(err),
            job_service::JobError::NotFound(id) => {
                ApiError::NotFound(format!("Job {} not found", id))
            }
            job_service::JobError::PipelineNotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
            job_service::JobError::ValidationError(msg) => ApiError::BadRequest(msg),
            job_service::JobError::InvalidState(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(Json(jobs))
}

/// GET /jobs/scheduled
/// List all scheduled (queued) jobs
///
/// Query parameters:
/// - `runner_id` (optional): Filter jobs to only those compatible with this runner
pub async fn list_scheduled_jobs(
    State(pool): State<PgPool>,
    Query(params): Query<ScheduledJobsQuery>,
) -> ApiResult<Json<Vec<Job>>> {
    if let Some(runner_id) = &params.runner_id {
        tracing::debug!("Listing scheduled jobs for runner: {}", runner_id);
    } else {
        tracing::debug!("Listing all scheduled jobs");
    }

    let jobs = job_service::list_jobs_by_status(&pool, JobStatus::Queued)
        .await
        .map_err(|e| match e {
            job_service::JobError::DatabaseError(err) => ApiError::DatabaseError(err),
            job_service::JobError::NotFound(id) => {
                ApiError::NotFound(format!("Job {} not found", id))
            }
            job_service::JobError::PipelineNotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
            job_service::JobError::ValidationError(msg) => ApiError::BadRequest(msg),
            job_service::JobError::InvalidState(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(Json(jobs))
}

#[derive(Debug, Deserialize)]
pub struct ScheduledJobsQuery {
    pub runner_id: Option<String>,
}

/// GET /job/pipeline/{pipeline_id}
/// List all jobs for a specific pipeline
pub async fn list_jobs_by_pipeline(
    State(pool): State<PgPool>,
    Path(pipeline_id): Path<Uuid>,
) -> ApiResult<Json<Vec<Job>>> {
    tracing::debug!("Listing jobs for pipeline: {}", pipeline_id);

    let jobs = job_service::list_jobs_by_pipeline(&pool, pipeline_id)
        .await
        .map_err(|e| match e {
            job_service::JobError::PipelineNotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
            job_service::JobError::DatabaseError(err) => ApiError::DatabaseError(err),
            job_service::JobError::NotFound(id) => {
                ApiError::NotFound(format!("Job {} not found", id))
            }
            job_service::JobError::ValidationError(msg) => ApiError::BadRequest(msg),
            job_service::JobError::InvalidState(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(Json(jobs))
}

/// POST /job/execute/{id}
/// Reserve a job for execution by a runner
pub async fn execute_job(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<ExecuteJobRequest>,
) -> ApiResult<Json<ExecuteJobResponse>> {
    tracing::info!("Runner {} executing job: {}", req.runner_id, id);

    let (job, pipeline) = job_service::reserve_job_for_execution(&pool, id, req.runner_id)
        .await
        .map_err(|e| match e {
            job_service::JobError::NotFound(id) => {
                ApiError::NotFound(format!("Job {} not found", id))
            }
            job_service::JobError::PipelineNotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
            job_service::JobError::InvalidState(msg) => ApiError::BadRequest(msg),
            job_service::JobError::DatabaseError(err) => ApiError::DatabaseError(err),
            job_service::JobError::ValidationError(msg) => ApiError::BadRequest(msg),
        })?;

    let response = ExecuteJobResponse {
        job_id: job.id,
        pipeline_id: pipeline.id,
        pipeline_source: pipeline.script,
        parameters: job.parameters,
    };

    Ok(Json(response))
}

/// POST /job/{id}/complete
/// Mark a job as complete with final status and result
pub async fn complete_job(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(req): Json<CompleteJobRequest>,
) -> ApiResult<StatusCode> {
    tracing::info!("Completing job: {} with status {:?}", id, req.status);

    job_service::complete_job(&pool, id, req.status, req.result)
        .await
        .map_err(|e| match e {
            job_service::JobError::NotFound(id) => {
                ApiError::NotFound(format!("Job {} not found", id))
            }
            job_service::JobError::ValidationError(msg) => ApiError::BadRequest(msg),
            job_service::JobError::InvalidState(msg) => ApiError::BadRequest(msg),
            job_service::JobError::DatabaseError(err) => ApiError::DatabaseError(err),
            job_service::JobError::PipelineNotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Log Endpoints
// =============================================================================

/// GET /job/{id}/logs
/// Get all logs for a job
pub async fn get_job_logs(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<LogEntry>>> {
    tracing::debug!("Getting logs for job: {}", id);

    // Verify job exists first
    job_service::get_job(&pool, id).await.map_err(|e| match e {
        job_service::JobError::NotFound(id) => ApiError::NotFound(format!("Job {} not found", id)),
        job_service::JobError::DatabaseError(err) => ApiError::DatabaseError(err),
        _ => ApiError::InternalError("Failed to verify job".to_string()),
    })?;

    let logs = log_service::get_job_logs(&pool, id)
        .await
        .map_err(|e| match e {
            log_service::LogError::DatabaseError(err) => ApiError::DatabaseError(err),
            log_service::LogError::JobNotFound(id) => {
                ApiError::NotFound(format!("Job {} not found", id))
            }
            log_service::LogError::ValidationError(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(Json(logs))
}

/// POST /job/{id}/logs
/// Add log entries to a job
pub async fn add_job_logs(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(logs): Json<Vec<LogEntry>>,
) -> ApiResult<StatusCode> {
    tracing::debug!("Adding {} log entries for job: {}", logs.len(), id);

    log_service::add_log_entries(&pool, id, logs)
        .await
        .map_err(|e| match e {
            log_service::LogError::ValidationError(msg) => ApiError::BadRequest(msg),
            log_service::LogError::DatabaseError(err) => ApiError::DatabaseError(err),
            log_service::LogError::JobNotFound(id) => {
                ApiError::NotFound(format!("Job {} not found", id))
            }
        })?;

    Ok(StatusCode::CREATED)
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct ExecuteJobRequest {
    pub runner_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ExecuteJobResponse {
    pub job_id: Uuid,
    pub pipeline_id: Uuid,
    pub pipeline_source: String,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CompleteJobRequest {
    pub status: JobStatus,
    pub result: Option<JobResult>,
}
