use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Create pipelines table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS pipelines (
            id UUID PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            description TEXT,
            script TEXT NOT NULL,
            required_modules TEXT[] NOT NULL DEFAULT '{}',
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL,
            tags TEXT[] NOT NULL DEFAULT '{}',
            timeout_seconds BIGINT,
            max_retries INTEGER NOT NULL DEFAULT 0,
            env_vars JSONB NOT NULL DEFAULT '{}'
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create jobs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id UUID PRIMARY KEY,
            pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
            status VARCHAR(50) NOT NULL,
            requested_at TIMESTAMPTZ NOT NULL,
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ,
            runner_id VARCHAR(255),
            parameters JSONB NOT NULL DEFAULT '{}',
            result_success BOOLEAN,
            result_exit_code INTEGER,
            result_output JSONB,
            result_error_message TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create logs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS job_logs (
            id SERIAL PRIMARY KEY,
            job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
            timestamp TIMESTAMPTZ NOT NULL,
            level VARCHAR(20) NOT NULL,
            message TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for better query performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_pipeline_id ON jobs(pipeline_id)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_requested_at ON jobs(requested_at DESC)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_job_logs_job_id ON job_logs(job_id, timestamp)")
        .execute(pool)
        .await?;

    // Create runners table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS runners (
            id VARCHAR(255) PRIMARY KEY,
            capabilities TEXT[] NOT NULL,
            registered_at TIMESTAMPTZ NOT NULL,
            last_heartbeat_at TIMESTAMPTZ NOT NULL,
            status VARCHAR(50) NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for runner queries
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_runners_status ON runners(status)")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_runners_last_heartbeat ON runners(last_heartbeat_at)",
    )
    .execute(pool)
    .await?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}
