//! Runner Repository
//!
//! Handles all database operations related to runners.

use rivet_core::domain::runner::{Runner, RunnerStatus};
use rivet_core::dto::runner::RegisterRunner;
use sqlx::PgPool;

/// Create or update a runner registration in the database
pub async fn register(pool: &PgPool, req: RegisterRunner) -> Result<Runner, sqlx::Error> {
    let now = chrono::Utc::now();

    let runner = Runner {
        id: req.runner_id.clone(),
        registered_at: now,
        last_heartbeat_at: now,
        status: RunnerStatus::Online,
    };

    sqlx::query(
        r#"
        INSERT INTO runners (id, registered_at, last_heartbeat_at, status)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO UPDATE SET
            last_heartbeat_at = EXCLUDED.last_heartbeat_at,
            status = EXCLUDED.status
        "#,
    )
    .bind(&req.runner_id)
    .bind(now)
    .bind(now)
    .bind("Online")
    .execute(pool)
    .await?;

    Ok(runner)
}

/// Update the last heartbeat time for a runner
pub async fn update_heartbeat(pool: &PgPool, runner_id: &str) -> Result<bool, sqlx::Error> {
    let now = chrono::Utc::now();

    let result = sqlx::query(
        r#"
        UPDATE runners
        SET last_heartbeat_at = $1, status = $2
        WHERE id = $3
        "#,
    )
    .bind(now)
    .bind("Online")
    .bind(runner_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Find a runner by ID
pub async fn find_by_id(pool: &PgPool, id: &str) -> Result<Option<Runner>, sqlx::Error> {
    let row = sqlx::query_as::<_, RunnerRow>(
        r#"
        SELECT id, registered_at, last_heartbeat_at, status
        FROM runners
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into()))
}

/// List all runners
pub async fn list_all(pool: &PgPool) -> Result<Vec<Runner>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RunnerRow>(
        r#"
        SELECT id, registered_at, last_heartbeat_at, status
        FROM runners
        ORDER BY registered_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

/// Delete a runner by ID
pub async fn delete(pool: &PgPool, id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM runners WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Mark runners as offline if they haven't sent a heartbeat recently
/// Returns the number of runners marked as offline
pub async fn mark_stale_runners_offline(
    pool: &PgPool,
    timeout_seconds: i64,
) -> Result<u64, sqlx::Error> {
    let cutoff_time = chrono::Utc::now() - chrono::Duration::seconds(timeout_seconds);

    let result = sqlx::query(
        r#"
        UPDATE runners
        SET status = $1
        WHERE last_heartbeat_at < $2 AND status != $3
        "#,
    )
    .bind("Offline")
    .bind(cutoff_time)
    .bind("Offline")
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

// =============================================================================
// Database Row Types
// =============================================================================

#[derive(sqlx::FromRow)]
struct RunnerRow {
    id: String,
    registered_at: chrono::DateTime<chrono::Utc>,
    last_heartbeat_at: chrono::DateTime<chrono::Utc>,
    status: String,
}

impl From<RunnerRow> for Runner {
    fn from(row: RunnerRow) -> Self {
        let status = match row.status.as_str() {
            "Online" => RunnerStatus::Online,
            "Offline" => RunnerStatus::Offline,
            "Busy" => RunnerStatus::Busy,
            _ => RunnerStatus::Offline, // Default to offline for unknown status
        };

        Runner {
            id: row.id,
            registered_at: row.registered_at,
            last_heartbeat_at: row.last_heartbeat_at,
            status,
        }
    }
}
