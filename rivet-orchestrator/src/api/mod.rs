//! API Module
//!
//! HTTP API layer for the orchestrator.
//! Each submodule handles endpoints for a specific domain.

pub mod error;
pub mod health;
pub mod job;
pub mod pipeline;

use axum::{
    Router,
    routing::{delete, get, post},
};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

/// Create the main API router with all endpoints
pub fn create_router(pool: PgPool) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health::health_check))
        // Pipeline endpoints
        .route("/pipeline/create", post(pipeline::create_pipeline))
        .route("/pipeline/launch", post(job::launch_job))
        .route("/pipeline/list", get(pipeline::list_pipelines))
        .route("/pipeline/{id}", get(pipeline::get_pipeline))
        .route("/pipeline/{id}", delete(pipeline::delete_pipeline))
        // Job endpoints
        .route("/job/list/scheduled", get(job::list_scheduled_jobs))
        .route("/job/execute/{id}", post(job::execute_job))
        .route("/job/{id}", get(job::get_job))
        .route("/job/{id}/complete", post(job::complete_job))
        .route("/job/{id}/logs", get(job::get_job_logs))
        .route("/job/{id}/logs", post(job::add_job_logs))
        .route(
            "/job/pipeline/{pipeline_id}",
            get(job::list_jobs_by_pipeline),
        )
        // Add state and middleware
        .with_state(pool)
        .layer(TraceLayer::new_for_http())
}
