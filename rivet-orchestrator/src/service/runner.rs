//! Runner Service
//!
//! Business logic for runner management.

use rivet_core::domain::runner::Runner;
use rivet_core::dto::runner::RegisterRunner;
use sqlx::PgPool;

use crate::repository::runner_repository;

/// Service error type
#[derive(Debug)]
pub enum RunnerError {
    NotFound(String),
    ValidationError(String),
    DatabaseError(sqlx::Error),
}

impl From<sqlx::Error> for RunnerError {
    fn from(err: sqlx::Error) -> Self {
        RunnerError::DatabaseError(err)
    }
}

pub type Result<T> = std::result::Result<T, RunnerError>;

/// Register a runner with the orchestrator
///
/// This creates a new runner entry or updates an existing one.
/// When a runner re-registers, it updates its heartbeat.
pub async fn register_runner(pool: &PgPool, req: RegisterRunner) -> Result<Runner> {
    // Validate request
    validate_register_request(&req)?;

    // Register runner in database
    let runner = runner_repository::register(pool, req).await?;

    tracing::info!("Runner registered: {}", runner.id);

    Ok(runner)
}

/// Update heartbeat for a runner
///
/// Keeps the runner marked as online. Should be called periodically by runners.
pub async fn update_heartbeat(pool: &PgPool, runner_id: &str) -> Result<()> {
    let updated = runner_repository::update_heartbeat(pool, runner_id).await?;

    if !updated {
        return Err(RunnerError::NotFound(runner_id.to_string()));
    }

    tracing::debug!("Heartbeat received from runner: {}", runner_id);

    Ok(())
}

/// Get a runner by ID
pub async fn get_runner(pool: &PgPool, id: &str) -> Result<Runner> {
    let runner = runner_repository::find_by_id(pool, id)
        .await?
        .ok_or_else(|| RunnerError::NotFound(id.to_string()))?;

    Ok(runner)
}

/// List all runners
pub async fn list_runners(pool: &PgPool) -> Result<Vec<Runner>> {
    let runners = runner_repository::list_all(pool).await?;
    Ok(runners)
}

/// Delete a runner
pub async fn delete_runner(pool: &PgPool, id: &str) -> Result<()> {
    let deleted = runner_repository::delete(pool, id).await?;

    if !deleted {
        return Err(RunnerError::NotFound(id.to_string()));
    }

    tracing::info!("Runner deleted: {}", id);

    Ok(())
}

/// Mark stale runners as offline
///
/// This should be called periodically to mark runners that haven't
/// sent a heartbeat recently as offline.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `timeout_seconds` - How long to wait before marking a runner as offline
pub async fn mark_stale_runners_offline(pool: &PgPool, timeout_seconds: i64) -> Result<u64> {
    let count = runner_repository::mark_stale_runners_offline(pool, timeout_seconds).await?;

    if count > 0 {
        tracing::info!("Marked {} runner(s) as offline", count);
    }

    Ok(count)
}

// =============================================================================
// Validation
// =============================================================================

fn validate_register_request(req: &RegisterRunner) -> Result<()> {
    if req.runner_id.trim().is_empty() {
        return Err(RunnerError::ValidationError(
            "Runner ID cannot be empty".to_string(),
        ));
    }

    if req.runner_id.len() > 255 {
        return Err(RunnerError::ValidationError(
            "Runner ID is too long (max 255 characters)".to_string(),
        ));
    }

    Ok(())
}
