//! Pipeline API Handlers
//!
//! HTTP endpoints for pipeline management.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use rivet_core::domain::pipeline::Pipeline;
use rivet_core::dto::pipeline::{CreatePipeline, PipelineSummary};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::error::{ApiError, ApiResult};
use crate::service::pipeline_service;

/// POST /pipeline/create
/// Create a new pipeline
pub async fn create_pipeline(
    State(pool): State<PgPool>,
    Json(req): Json<CreatePipeline>,
) -> ApiResult<Json<Pipeline>> {
    tracing::info!("Creating pipeline: {}", req.name);

    let pipeline = pipeline_service::create_pipeline(&pool, req)
        .await
        .map_err(|e| match e {
            pipeline_service::PipelineError::ValidationError(msg) => ApiError::BadRequest(msg),
            pipeline_service::PipelineError::DatabaseError(err) => ApiError::DatabaseError(err),
            pipeline_service::PipelineError::NotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
        })?;

    Ok(Json(pipeline))
}

/// GET /pipeline/list
/// List all pipelines
pub async fn list_pipelines(State(pool): State<PgPool>) -> ApiResult<Json<Vec<PipelineSummary>>> {
    tracing::debug!("Listing all pipelines");

    let pipelines = pipeline_service::list_pipelines(&pool)
        .await
        .map_err(|e| match e {
            pipeline_service::PipelineError::DatabaseError(err) => ApiError::DatabaseError(err),
            pipeline_service::PipelineError::NotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
            pipeline_service::PipelineError::ValidationError(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(Json(pipelines))
}

/// GET /pipeline/{id}
/// Get pipeline by ID
pub async fn get_pipeline(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Pipeline>> {
    tracing::debug!("Getting pipeline: {}", id);

    let pipeline = pipeline_service::get_pipeline(&pool, id)
        .await
        .map_err(|e| match e {
            pipeline_service::PipelineError::NotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
            pipeline_service::PipelineError::DatabaseError(err) => ApiError::DatabaseError(err),
            pipeline_service::PipelineError::ValidationError(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(Json(pipeline))
}

/// DELETE /pipeline/{id}
/// Delete a pipeline
pub async fn delete_pipeline(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    tracing::info!("Deleting pipeline: {}", id);

    pipeline_service::delete_pipeline(&pool, id)
        .await
        .map_err(|e| match e {
            pipeline_service::PipelineError::NotFound(id) => {
                ApiError::NotFound(format!("Pipeline {} not found", id))
            }
            pipeline_service::PipelineError::DatabaseError(err) => ApiError::DatabaseError(err),
            pipeline_service::PipelineError::ValidationError(msg) => ApiError::BadRequest(msg),
        })?;

    Ok(StatusCode::NO_CONTENT)
}
