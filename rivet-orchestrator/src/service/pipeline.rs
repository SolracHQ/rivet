//! Pipeline Service
//!
//! Business logic for pipeline management.

use rivet_core::domain::pipeline::Pipeline;
use rivet_core::dto::pipeline::CreatePipeline;
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
    if req.name.trim().is_empty() {
        return Err(PipelineError::ValidationError(
            "Pipeline name cannot be empty".to_string(),
        ));
    }

    if req.name.len() > 255 {
        return Err(PipelineError::ValidationError(
            "Pipeline name is too long (max 255 characters)".to_string(),
        ));
    }

    if req.script.trim().is_empty() {
        return Err(PipelineError::ValidationError(
            "Pipeline script cannot be empty".to_string(),
        ));
    }

    // Validate Lua syntax by attempting to parse
    if let Err(e) = mlua::Lua::new().load(&req.script).exec() {
        return Err(PipelineError::ValidationError(format!(
            "Invalid Lua script: {}",
            e
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_name() {
        let req = CreatePipeline {
            name: "".to_string(),
            description: None,
            script: "log.info('test')".to_string(),
            required_modules: vec![],
            tags: vec![],
            config: None,
        };

        let result = validate_pipeline_request(&req);
        assert!(matches!(result, Err(PipelineError::ValidationError(_))));
    }

    #[test]
    fn test_validate_empty_script() {
        let req = CreatePipeline {
            name: "Test".to_string(),
            description: None,
            script: "".to_string(),
            required_modules: vec![],
            tags: vec![],
            config: None,
        };

        let result = validate_pipeline_request(&req);
        assert!(matches!(result, Err(PipelineError::ValidationError(_))));
    }

    #[test]
    fn test_validate_valid_request() {
        let req = CreatePipeline {
            name: "Test Pipeline".to_string(),
            description: Some("A test pipeline".to_string()),
            // TODO: replace for a valid pipeline script definition
            script: "return {}".to_string(),
            required_modules: vec!["log".to_string()],
            tags: vec!["test".to_string()],
            config: None,
        };

        let result = validate_pipeline_request(&req);
        println!("{result:?}");
        assert!(result.is_ok());
    }
}
