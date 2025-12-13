//! Stubs API endpoints
//!
//! Serves Lua Language Server stub files for Rivet modules.
//! These stubs provide type hints and documentation for pipeline development.

use axum::{
    Json,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Response containing a stub file
#[derive(Serialize)]
pub struct StubResponse {
    pub name: String,
    pub content: String,
}

/// List all available stub files
pub async fn list_stubs() -> Json<Vec<String>> {
    Json(vec![
        "log".to_string(),
        "input".to_string(),
        "output".to_string(),
        "process".to_string(),
        "container".to_string(),
    ])
}

/// Get a specific stub file by name
pub async fn get_stub(Path(name): Path<String>) -> Response {
    let content = match name.as_str() {
        "log" => include_str!("../../stubs/log.lua"),
        "input" => include_str!("../../stubs/input.lua"),
        "output" => include_str!("../../stubs/output.lua"),
        "process" => include_str!("../../stubs/process.lua"),
        "container" => include_str!("../../stubs/container.lua"),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Stub '{}' not found", name)
                })),
            )
                .into_response();
        }
    };

    Json(StubResponse {
        name: format!("{}.lua", name),
        content: content.to_string(),
    })
    .into_response()
}
