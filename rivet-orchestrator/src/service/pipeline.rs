//! Pipeline Service
//!
//! Business logic for pipeline management.

use rivet_core::domain::pipeline::Pipeline;
use rivet_core::dto::pipeline::CreatePipeline;
use rivet_lua::{create_sandbox, parse_pipeline_definition};
use sqlx::PgPool;
use uuid::Uuid;

use crate::repository::pipeline_repository;

/// Service error type
#[derive(Debug)]
pub enum PipelineError {
    NotFound(Uuid),
    ValidationError(String),
    DatabaseError(sqlx::Error),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::NotFound(id) => write!(f, "Pipeline not found: {}", id),
            PipelineError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            PipelineError::DatabaseError(err) => write!(f, "Database error: {}", err),
        }
    }
}

impl std::error::Error for PipelineError {}

impl From<sqlx::Error> for PipelineError {
    fn from(err: sqlx::Error) -> Self {
        PipelineError::DatabaseError(err)
    }
}

pub type Result<T> = std::result::Result<T, PipelineError>;

/// Create a new pipeline
pub async fn create_pipeline(pool: &PgPool, req: CreatePipeline) -> Result<Pipeline> {
    // Validate request
    validate_pipeline_request(&req)?;

    // Create pipeline in database
    let pipeline = pipeline_repository::create(pool, req).await?;

    tracing::info!("Pipeline created: {} ({})", pipeline.name, pipeline.id);

    Ok(pipeline)
}

/// Get a pipeline by ID
pub async fn get_pipeline(pool: &PgPool, id: Uuid) -> Result<Pipeline> {
    let pipeline = pipeline_repository::find_by_id(pool, id)
        .await?
        .ok_or(PipelineError::NotFound(id))?;

    Ok(pipeline)
}

/// List all pipelines
pub async fn list_pipelines(pool: &PgPool) -> Result<Vec<Pipeline>> {
    let pipelines = pipeline_repository::list_all(pool).await?;
    Ok(pipelines)
}

/// Update a pipeline
pub async fn update_pipeline(pool: &PgPool, id: Uuid, req: CreatePipeline) -> Result<Pipeline> {
    // Validate request
    validate_pipeline_request(&req)?;

    // Check if pipeline exists
    let _existing = pipeline_repository::find_by_id(pool, id)
        .await?
        .ok_or(PipelineError::NotFound(id))?;

    // Update pipeline
    let updated = pipeline_repository::update(pool, id, req).await?;

    if !updated {
        return Err(PipelineError::NotFound(id));
    }

    // Return updated pipeline
    get_pipeline(pool, id).await
}

/// Delete a pipeline
pub async fn delete_pipeline(pool: &PgPool, id: Uuid) -> Result<()> {
    let deleted = pipeline_repository::delete(pool, id).await?;

    if !deleted {
        return Err(PipelineError::NotFound(id));
    }

    tracing::info!("Pipeline deleted: {}", id);

    Ok(())
}

// =============================================================================
// Validation
// =============================================================================

fn validate_pipeline_request(req: &CreatePipeline) -> Result<()> {
    if req.script.trim().is_empty() {
        return Err(PipelineError::ValidationError(
            "Pipeline script cannot be empty".to_string(),
        ));
    }

    // Validate pipeline structure using definition parser
    // This validates Lua syntax, pipeline structure, and required fields
    let lua = create_sandbox()
        .map_err(|e| PipelineError::ValidationError(format!("Failed to create sandbox: {}", e)))?;

    let definition = parse_pipeline_definition(&lua, &req.script).map_err(|e| {
        PipelineError::ValidationError(format!("Invalid pipeline definition: {}", e))
    })?;

    // Verify at least one stage is defined
    if definition.stages.is_empty() {
        return Err(PipelineError::ValidationError(
            "Pipeline must have at least one stage".to_string(),
        ));
    }

    Ok(())
}
