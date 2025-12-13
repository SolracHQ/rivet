//! API Module
//!
//! HTTP API layer for the orchestrator.
//! Each submodule handles endpoints for a specific domain.

pub mod error;
pub mod health;
pub mod job;
pub mod pipeline;
pub mod runner;
pub mod stubs;

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
        .route("/api/health", get(health::health_check))
        // Runner endpoints
        .route("/api/runners/register", post(runner::register_runner))
        .route(
            "/api/runners/{id}/heartbeat",
            post(runner::runner_heartbeat),
        )
        .route("/api/runners", get(runner::list_runners))
        .route("/api/runners/{id}", get(runner::get_runner))
        .route("/api/runners/{id}", delete(runner::delete_runner))
        // Pipeline endpoints
        .route("/api/pipeline/create", post(pipeline::create_pipeline))
        .route("/api/pipeline/launch", post(job::launch_job))
        .route("/api/pipeline/list", get(pipeline::list_pipelines))
        .route("/api/pipeline/{id}", get(pipeline::get_pipeline))
        .route("/api/pipeline/{id}", delete(pipeline::delete_pipeline))
        // Job endpoints
        .route("/api/jobs", get(job::list_all_jobs))
        .route("/api/jobs/scheduled", get(job::list_scheduled_jobs))
        .route("/api/jobs/execute/{id}", post(job::execute_job))
        .route("/api/jobs/{id}", get(job::get_job))
        .route("/api/jobs/{id}/complete", post(job::complete_job))
        .route("/api/jobs/{id}/logs", get(job::get_job_logs))
        .route("/api/jobs/{id}/logs", post(job::add_job_logs))
        .route(
            "/api/jobs/pipeline/{pipeline_id}",
            get(job::list_jobs_by_pipeline),
        )
        // Stubs endpoints
        .route("/api/stubs", get(stubs::list_stubs))
        .route("/api/stubs/{name}", get(stubs::get_stub))
        // Add state and middleware
        .with_state(pool)
        .layer(TraceLayer::new_for_http())
}
