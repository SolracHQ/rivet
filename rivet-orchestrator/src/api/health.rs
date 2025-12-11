//! Health Check API Handler
//!
//! Simple health check endpoint for monitoring.

use axum::{http::StatusCode, response::IntoResponse};

/// GET /health
/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
