//! Job Service
//!
//! Business logic for job management and lifecycle.

use rivet_core::domain::job::{Job, JobResult, JobStatus};
use rivet_core::domain::pipeline::Pipeline;
use rivet_core::dto::job::CreateJob;
use rivet_lua::{create_sandbox, parse_pipeline_definition};
use sqlx::PgPool;
use uuid::Uuid;

use crate::repository::{job_repository, pipeline_repository};

/// Service error type
#[derive(Debug)]
pub enum JobError {
    NotFound(Uuid),
    PipelineNotFound(Uuid),
    InvalidState(String),
    ValidationError(String),
    DatabaseError(sqlx::Error),
}

impl From<sqlx::Error> for JobError {
    fn from(err: sqlx::Error) -> Self {
        JobError::DatabaseError(err)
    }
}

/// Create and schedule a new job
pub async fn launch_job(pool: &PgPool, req: CreateJob) -> Result<Job, JobError> {
    // Verify pipeline exists
    let pipeline = pipeline_repository::find_by_id(pool, req.pipeline_id)
        .await?
        .ok_or(JobError::PipelineNotFound(req.pipeline_id))?;

    // Parse pipeline definition to validate and enrich parameters
    let lua = create_sandbox()
        .map_err(|e| JobError::ValidationError(format!("Failed to create sandbox: {}", e)))?;

    let definition = parse_pipeline_definition(&lua, &pipeline.script)
        .map_err(|e| JobError::ValidationError(format!("Failed to parse pipeline: {}", e)))?;

    // Validate and enrich parameters with defaults
    let enriched_params = validate_and_enrich_parameters(&definition, req.parameters)?;

    // Create enriched request
    let enriched_req = CreateJob {
        pipeline_id: req.pipeline_id,
        parameters: enriched_params,
    };

    // Create job in database
    let job = job_repository::create(pool, enriched_req).await?;

    tracing::info!("Job created: {} for pipeline: {}", job.id, job.pipeline_id);

    Ok(job)
}

/// Get a job by ID
pub async fn get_job(pool: &PgPool, id: Uuid) -> Result<Job, JobError> {
    let job = job_repository::find_by_id(pool, id)
        .await?
        .ok_or(JobError::NotFound(id))?;

    Ok(job)
}

/// List jobs by status
pub async fn list_jobs_by_status(pool: &PgPool, status: JobStatus) -> Result<Vec<Job>, JobError> {
    let jobs = job_repository::find_by_status(pool, status).await?;
    Ok(jobs)
}

/// List all jobs
pub async fn list_all_jobs(pool: &PgPool) -> Result<Vec<Job>, JobError> {
    let jobs = job_repository::list_all(pool).await?;
    Ok(jobs)
}

/// List jobs by pipeline
pub async fn list_jobs_by_pipeline(pool: &PgPool, pipeline_id: Uuid) -> Result<Vec<Job>, JobError> {
    // Verify pipeline exists
    let _pipeline = pipeline_repository::find_by_id(pool, pipeline_id)
        .await?
        .ok_or(JobError::PipelineNotFound(pipeline_id))?;

    let jobs = job_repository::find_by_pipeline(pool, pipeline_id).await?;
    Ok(jobs)
}

/// Reserve a job for execution by a runner
pub async fn reserve_job_for_execution(
    pool: &PgPool,
    job_id: Uuid,
    runner_id: String,
) -> Result<(Job, Pipeline), JobError> {
    // Get the job
    let job = job_repository::find_by_id(pool, job_id)
        .await?
        .ok_or(JobError::NotFound(job_id))?;

    // Check if job is in the right state
    if job.status != JobStatus::Queued {
        return Err(JobError::InvalidState(format!(
            "Job {} is not in Queued state (current: {:?})",
            job_id, job.status
        )));
    }

    // Get the pipeline
    let pipeline = pipeline_repository::find_by_id(pool, job.pipeline_id)
        .await?
        .ok_or(JobError::PipelineNotFound(job.pipeline_id))?;

    // Update job status to Running
    job_repository::update_status_to_running(pool, job_id, runner_id).await?;

    tracing::info!("Job {} reserved and started", job_id);

    // Return updated job
    let updated_job = job_repository::find_by_id(pool, job_id)
        .await?
        .ok_or(JobError::NotFound(job_id))?;

    Ok((updated_job, pipeline))
}

/// Complete a job with final status and result
pub async fn complete_job(
    pool: &PgPool,
    job_id: Uuid,
    status: JobStatus,
    result: Option<JobResult>,
) -> Result<(), JobError> {
    // Verify job exists
    let job = job_repository::find_by_id(pool, job_id)
        .await?
        .ok_or(JobError::NotFound(job_id))?;

    // Validate status transition
    validate_completion_status(status)?;

    // Ensure job is in running state
    if job.status != JobStatus::Running {
        tracing::warn!(
            "Completing job {} that is not in Running state (current: {:?})",
            job_id,
            job.status
        );
    }

    // Update job status
    job_repository::update_status_to_completed(pool, job_id, status).await?;

    // If there's a result, update it
    if let Some(result) = result {
        job_repository::update_result(pool, job_id, result).await?;
    }

    tracing::info!("Job {} completed with status: {:?}", job_id, status);

    Ok(())
}

/// Cancel a job
pub async fn cancel_job(pool: &PgPool, job_id: Uuid) -> Result<(), JobError> {
    let job = job_repository::find_by_id(pool, job_id)
        .await?
        .ok_or(JobError::NotFound(job_id))?;

    // Can only cancel queued or running jobs
    match job.status {
        JobStatus::Queued | JobStatus::Running => {
            job_repository::update_status_to_completed(pool, job_id, JobStatus::Cancelled).await?;
            tracing::info!("Job {} cancelled", job_id);
            Ok(())
        }
        _ => Err(JobError::InvalidState(format!(
            "Cannot cancel job {} in state {:?}",
            job_id, job.status
        ))),
    }
}

// =============================================================================
// Validation
// =============================================================================

fn validate_completion_status(status: JobStatus) -> Result<(), JobError> {
    match status {
        JobStatus::Succeeded | JobStatus::Failed | JobStatus::TimedOut | JobStatus::Cancelled => {
            Ok(())
        }
        _ => Err(JobError::ValidationError(format!(
            "Invalid completion status: {:?}",
            status
        ))),
    }
}

/// Validate and enrich job parameters with pipeline defaults
fn validate_and_enrich_parameters(
    definition: &rivet_lua::PipelineDefinition,
    mut parameters: std::collections::HashMap<String, serde_json::Value>,
) -> Result<std::collections::HashMap<String, serde_json::Value>, JobError> {
    // Check all required inputs are provided
    for (key, input_def) in &definition.inputs {
        if !parameters.contains_key(key) {
            if let Some(default) = &input_def.default {
                // Apply default value
                parameters.insert(key.clone(), default.clone());
            } else if input_def.required {
                return Err(JobError::ValidationError(format!(
                    "Missing required input '{}' (type: {})",
                    key, input_def.input_type
                )));
            }
        } else {
            // Validate type
            let value = &parameters[key];
            validate_input_type(key, value, &input_def.input_type)?;

            // Validate options if provided
            if let Some(options) = &input_def.options {
                let value_matches = options.iter().any(|opt| match (value, opt) {
                    (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                        a.as_f64() == b.as_f64()
                    }
                    (serde_json::Value::String(a), serde_json::Value::String(b)) => a == b,
                    (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => a == b,
                    _ => false,
                });

                if !value_matches {
                    let options_str = options
                        .iter()
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            _ => format!("{:?}", v),
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    return Err(JobError::ValidationError(format!(
                        "Invalid value for input '{}'. Must be one of: {}",
                        key, options_str
                    )));
                }
            }
        }
    }

    Ok(parameters)
}

/// Validate that a parameter value matches the expected type
fn validate_input_type(
    name: &str,
    value: &serde_json::Value,
    expected_type: &str,
) -> Result<(), JobError> {
    let matches = match expected_type {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "bool" => value.is_boolean(),
        _ => {
            return Err(JobError::ValidationError(format!(
                "Unknown input type: {}",
                expected_type
            )));
        }
    };

    if !matches {
        return Err(JobError::ValidationError(format!(
            "Input '{}' expected type '{}', but got: {:?}",
            name, expected_type, value
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_completion_status_valid() {
        assert!(validate_completion_status(JobStatus::Succeeded).is_ok());
        assert!(validate_completion_status(JobStatus::Failed).is_ok());
        assert!(validate_completion_status(JobStatus::TimedOut).is_ok());
        assert!(validate_completion_status(JobStatus::Cancelled).is_ok());
    }

    #[test]
    fn test_validate_completion_status_invalid() {
        assert!(validate_completion_status(JobStatus::Queued).is_err());
        assert!(validate_completion_status(JobStatus::Running).is_err());
    }
}
