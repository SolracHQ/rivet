//! Pipeline Repository
//!
//! Handles all database operations related to pipelines.

use rivet_core::domain::pipeline::{Pipeline, PipelineConfig};
use rivet_core::dto::pipeline::CreatePipeline;
use sqlx::PgPool;
use uuid::Uuid;

/// Create a new pipeline in the database
pub async fn create(pool: &PgPool, req: CreatePipeline) -> Result<Pipeline, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let config = req.config.unwrap_or_default();

    let pipeline = Pipeline {
        id,
        name: req.name.clone(),
        description: req.description.clone(),
        script: req.script.clone(),
        required_modules: req.required_modules.clone(),
        created_at: now,
        updated_at: now,
        tags: req.tags.clone(),
        config: config.clone(),
    };

    sqlx::query(
        r#"
        INSERT INTO pipelines (
            id, name, description, script, required_modules,
            created_at, updated_at, tags, timeout_seconds, max_retries, env_vars
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.script)
    .bind(&req.required_modules)
    .bind(now)
    .bind(now)
    .bind(&req.tags)
    .bind(config.timeout_seconds.map(|t| t as i64))
    .bind(config.max_retries as i32)
    .bind(serde_json::to_value(&config.env_vars).unwrap())
    .execute(pool)
    .await?;

    Ok(pipeline)
}

/// Find a pipeline by ID
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Pipeline>, sqlx::Error> {
    let row = sqlx::query_as::<_, PipelineRow>(
        r#"
        SELECT id, name, description, script, required_modules,
               created_at, updated_at, tags, timeout_seconds, max_retries, env_vars
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
        SELECT id, name, description, script, required_modules,
               created_at, updated_at, tags, timeout_seconds, max_retries, env_vars
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
    let config = req.config.unwrap_or_default();

    let result = sqlx::query(
        r#"
        UPDATE pipelines
        SET name = $1, description = $2, script = $3, required_modules = $4,
            updated_at = $5, tags = $6, timeout_seconds = $7, max_retries = $8, env_vars = $9
        WHERE id = $10
        "#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.script)
    .bind(&req.required_modules)
    .bind(now)
    .bind(&req.tags)
    .bind(config.timeout_seconds.map(|t| t as i64))
    .bind(config.max_retries as i32)
    .bind(serde_json::to_value(&config.env_vars).unwrap())
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
    required_modules: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    tags: Vec<String>,
    timeout_seconds: Option<i64>,
    max_retries: i32,
    env_vars: serde_json::Value,
}

impl From<PipelineRow> for Pipeline {
    fn from(row: PipelineRow) -> Self {
        let env_vars = serde_json::from_value(row.env_vars).unwrap_or_default();

        Pipeline {
            id: row.id,
            name: row.name,
            description: row.description,
            script: row.script,
            required_modules: row.required_modules,
            created_at: row.created_at,
            updated_at: row.updated_at,
            tags: row.tags,
            config: PipelineConfig {
                timeout_seconds: row.timeout_seconds.map(|t| t as u64),
                max_retries: row.max_retries as u32,
                env_vars,
            },
        }
    }
}
