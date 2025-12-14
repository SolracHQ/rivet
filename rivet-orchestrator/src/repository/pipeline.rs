//! Pipeline Repository
//!
//! Handles all database operations related to pipelines.

use rivet_core::domain::pipeline::Pipeline;
use rivet_core::dto::pipeline::CreatePipeline;
use rivet_lua::{create_sandbox, parse_pipeline_definition};
use sqlx::PgPool;
use uuid::Uuid;

/// Create a new pipeline in the database
pub async fn create(pool: &PgPool, req: CreatePipeline) -> Result<Pipeline, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Parse script to extract name and description
    let lua = create_sandbox()
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to create sandbox: {}", e)))?;

    let definition = parse_pipeline_definition(&lua, &req.script)
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to parse pipeline: {}", e)))?;

    // Convert definition tags to domain tags
    let tags: Vec<rivet_core::domain::pipeline::Tag> = definition
        .runner
        .iter()
        .map(|t| rivet_core::domain::pipeline::Tag {
            key: t.key.clone(),
            value: t.value.clone(),
        })
        .collect();

    let pipeline = Pipeline {
        id,
        name: definition.name.clone(),
        description: definition.description.clone(),
        script: req.script.clone(),
        created_at: now,
        updated_at: now,
        tags: tags.clone(),
    };

    let tags_json = serde_json::to_value(&tags)
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize tags: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO pipelines (id, name, description, script, created_at, updated_at, tags)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(id)
    .bind(&definition.name)
    .bind(&definition.description)
    .bind(&req.script)
    .bind(now)
    .bind(now)
    .bind(tags_json)
    .execute(pool)
    .await?;

    Ok(pipeline)
}

/// Find a pipeline by ID
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Pipeline>, sqlx::Error> {
    let row = sqlx::query_as::<_, PipelineRow>(
        r#"
        SELECT id, name, description, script, created_at, updated_at, tags::text as tags
        FROM pipelines
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into()))
}

/// List all pipelines
pub async fn list_all(pool: &PgPool) -> Result<Vec<Pipeline>, sqlx::Error> {
    let rows = sqlx::query_as::<_, PipelineRow>(
        r#"
        SELECT id, name, description, script, created_at, updated_at, tags::text as tags
        FROM pipelines
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Update a pipeline
pub async fn update(pool: &PgPool, id: Uuid, req: CreatePipeline) -> Result<bool, sqlx::Error> {
    let now = chrono::Utc::now();

    // Parse script to extract name and description
    let lua = create_sandbox()
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to create sandbox: {}", e)))?;

    let definition = parse_pipeline_definition(&lua, &req.script)
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to parse pipeline: {}", e)))?;

    // Convert definition tags to domain tags
    let tags: Vec<rivet_core::domain::pipeline::Tag> = definition
        .runner
        .iter()
        .map(|t| rivet_core::domain::pipeline::Tag {
            key: t.key.clone(),
            value: t.value.clone(),
        })
        .collect();

    let tags_json = serde_json::to_value(&tags)
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize tags: {}", e)))?;

    let result = sqlx::query(
        r#"
        UPDATE pipelines
        SET name = $1, description = $2, script = $3, updated_at = $4, tags = $5
        WHERE id = $6
        "#,
    )
    .bind(&definition.name)
    .bind(&definition.description)
    .bind(&req.script)
    .bind(now)
    .bind(tags_json)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Delete a pipeline by ID
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM pipelines WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

// =============================================================================
// Database Row Types
// =============================================================================

#[derive(sqlx::FromRow)]
struct PipelineRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    script: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    tags: String,
}

impl From<PipelineRow> for Pipeline {
    fn from(row: PipelineRow) -> Self {
        let tags: Vec<rivet_core::domain::pipeline::Tag> =
            serde_json::from_str(&row.tags).unwrap_or_else(|_| vec![]);

        Pipeline {
            id: row.id,
            name: row.name,
            description: row.description,
            script: row.script,
            created_at: row.created_at,
            updated_at: row.updated_at,
            tags,
        }
    }
}
