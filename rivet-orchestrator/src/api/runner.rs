//! Runner API Handlers
//!
//! HTTP endpoints for runner management and lifecycle.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use rivet_core::domain::runner::Runner;
use rivet_core::dto::runner::RegisterRunner;
use sqlx::PgPool;

use crate::api::error::{ApiError, ApiResult};
use crate::service::runner_service;

// =============================================================================
// Runner Registration & Lifecycle
// =============================================================================

/// POST /api/runners/register
/// Register a runner with the orchestrator
pub async fn register_runner(
    State(pool): State<PgPool>,
    Json(req): Json<RegisterRunner>,
) -> ApiResult<Json<Runner>> {
    tracing::info!("Registering runner: {}", req.runner_id);

    let runner = runner_service::register_runner(&pool, req)
        .await
        .map_err(|e| match e {
            runner_service::RunnerError::NotFound(id) => {
                ApiError::NotFound(format!("Runner {} not found", id))
            }
            runner_service::RunnerError::ValidationError(msg) => ApiError::BadRequest(msg),
            runner_service::RunnerError::DatabaseError(err) => ApiError::DatabaseError(err),
        })?;

    Ok(Json(runner))
}

/// POST /api/runners/{id}/heartbeat
/// Update heartbeat for a runner to keep it marked as online
pub async fn runner_heartbeat(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    tracing::debug!("Heartbeat from runner: {}", id);

    runner_service::update_heartbeat(&pool, &id)
        .await
        .map_err(|e| match e {
            runner_service::RunnerError::NotFound(id) => {
                ApiError::NotFound(format!("Runner {} not found", id))
            }
            runner_service::RunnerError::ValidationError(msg) => ApiError::BadRequest(msg),
            runner_service::RunnerError::DatabaseError(err) => ApiError::DatabaseError(err),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Runner Query Endpoints
// =============================================================================

/// GET /api/runners
/// List all registered runners
pub async fn list_runners(State(pool): State<PgPool>) -> ApiResult<Json<Vec<Runner>>> {
    tracing::debug!("Listing all runners");

    let runners = runner_service::list_runners(&pool)
        .await
        .map_err(|e| match e {
            runner_service::RunnerError::DatabaseError(err) => ApiError::DatabaseError(err),
            runner_service::RunnerError::NotFound(id) => {
                ApiError::NotFound(format!("Runner {} not found", id))
            }
            runner_service::RunnerError::ValidationError(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(Json(runners))
}

/// GET /api/runners/{id}
/// Get details for a specific runner
pub async fn get_runner(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> ApiResult<Json<Runner>> {
    tracing::debug!("Getting runner: {}", id);

    let runner = runner_service::get_runner(&pool, &id)
        .await
        .map_err(|e| match e {
            runner_service::RunnerError::NotFound(id) => {
                ApiError::NotFound(format!("Runner {} not found", id))
            }
            runner_service::RunnerError::ValidationError(msg) => ApiError::BadRequest(msg),
            runner_service::RunnerError::DatabaseError(err) => ApiError::DatabaseError(err),
        })?;

    Ok(Json(runner))
}

/// DELETE /api/runners/{id}
/// Delete a runner registration
pub async fn delete_runner(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    tracing::info!("Deleting runner: {}", id);

    runner_service::delete_runner(&pool, &id)
        .await
        .map_err(|e| match e {
            runner_service::RunnerError::NotFound(id) => {
                ApiError::NotFound(format!("Runner {} not found", id))
            }
            runner_service::RunnerError::ValidationError(msg) => ApiError::BadRequest(msg),
            runner_service::RunnerError::DatabaseError(err) => ApiError::DatabaseError(err),
        })?;

    Ok(StatusCode::NO_CONTENT)
}
