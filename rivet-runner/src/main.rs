use anyhow::{Context, Result};
use rivet_core::types::{JobResult, JobStatus, LogEntry, Pipeline};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

pub mod execution;
pub mod lua;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rivet_runner=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Rivet Runner...");

    // Get configuration from environment
    let orchestrator_url =
        std::env::var("ORCHESTRATOR_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let runner_id =
        std::env::var("RUNNER_ID").unwrap_or_else(|_| format!("runner-{}", Uuid::new_v4()));
    let poll_interval = std::env::var("POLL_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5u64);

    tracing::info!("Runner ID: {}", runner_id);
    tracing::info!("Orchestrator URL: {}", orchestrator_url);
    tracing::info!("Poll interval: {}s", poll_interval);

    let client = OrchestratorClient::new(&orchestrator_url, &runner_id);

    // Main polling loop
    loop {
        match poll_and_execute(&client).await {
            Ok(executed) => {
                if !executed {
                    tracing::debug!("No jobs available, waiting...");
                }
            }
            Err(e) => {
                tracing::error!("Error in poll/execute cycle: {:?}", e);
            }
        }

        tokio::time::sleep(Duration::from_secs(poll_interval)).await;
    }
}

async fn poll_and_execute(client: &OrchestratorClient) -> Result<bool> {
    // Get list of scheduled jobs
    let scheduled_jobs = client.list_scheduled_jobs().await?;

    if scheduled_jobs.is_empty() {
        return Ok(false);
    }

    tracing::info!("Found {} scheduled job(s)", scheduled_jobs.len());

    // Try to claim and execute the first available job
    for job_dto in scheduled_jobs {
        match execute_job(client, job_dto.id).await {
            Ok(_) => {
                tracing::info!("Successfully executed job: {}", job_dto.id);
                return Ok(true);
            }
            Err(e) => {
                tracing::warn!("Failed to execute job {}: {:?}", job_dto.id, e);
                // Continue to next job
            }
        }
    }

    Ok(false)
}

async fn execute_job(client: &OrchestratorClient, job_id: Uuid) -> Result<()> {
    tracing::info!("Attempting to execute job: {}", job_id);

    // Reserve the job by calling execute endpoint
    let execute_response = client.execute_job(job_id).await?;

    tracing::info!(
        "Job {} reserved, executing pipeline: {}",
        execute_response.job_id,
        execute_response.pipeline.name
    );

    // Execute the pipeline script
    let result = execute_pipeline_script(
        client,
        &execute_response.job_id,
        &execute_response.pipeline,
        execute_response.parameters,
    )
    .await;

    // Report completion to orchestrator
    match result {
        Ok(execution_result) => {
            let job_result = execution_result.into_job_result();
            client
                .complete_job(job_id, JobStatus::Succeeded, Some(job_result))
                .await?;
            tracing::info!("Job {} completed successfully", job_id);
        }
        Err(e) => {
            let error_message = format!("Execution failed: {:?}", e);
            tracing::error!("{}", error_message);

            let job_result = JobResult {
                success: false,
                exit_code: 1,
                output: None,
                error_message: Some(error_message),
            };

            client
                .complete_job(job_id, JobStatus::Failed, Some(job_result))
                .await?;
        }
    }

    Ok(())
}

async fn execute_pipeline_script(
    client: &OrchestratorClient,
    job_id: &Uuid,
    pipeline: &Pipeline,
    parameters: HashMap<String, serde_json::Value>,
) -> Result<execution::ExecutionResult> {
    tracing::debug!("Creating execution context for job: {}", job_id);

    // Create execution context
    let log_buffer = Arc::new(Mutex::new(Vec::new()));
    let lua = mlua::Lua::new();

    let ctx = execution::ExecutionContext {
        lua,
        job_id: *job_id,
        pipeline_id: pipeline.id,
        parameters: parameters.clone(),
        metadata: execution::ExecutionMetadata {
            runner_id: client.runner_id.clone(),
            started_at: chrono::Utc::now(),
            loaded_modules: pipeline.required_modules.clone(),
        },
        log_buffer: log_buffer.clone(),
    };

    // Load modules into Lua context
    load_modules(&ctx)?;

    // Add a log entry for script start
    add_log(
        &log_buffer,
        rivet_core::types::LogLevel::Info,
        "Starting pipeline execution",
    );

    // First, execute the Lua script to define the pipeline table
    tracing::debug!("Loading pipeline definition...");
    if let Err(e) = ctx.lua.load(&pipeline.script).exec() {
        let error_msg = format!("Failed to load pipeline definition: {}", e);
        tracing::error!("{}", error_msg);
        add_log(&log_buffer, rivet_core::types::LogLevel::Error, &error_msg);

        let logs = ctx.drain_logs().map_err(|e| anyhow::anyhow!(e))?;
        if !logs.is_empty() {
            if let Err(e) = client.send_logs(*job_id, logs.clone()).await {
                tracing::warn!("Failed to send error logs: {:?}", e);
            }
        }

        return Ok(execution::ExecutionResult::Failure {
            error: error_msg,
            logs,
        });
    }

    // Now extract and execute each stage
    tracing::debug!("Extracting pipeline stages...");
    let globals = ctx.lua.globals();

    let pipeline_table: mlua::Table = match globals.get("pipeline") {
        Ok(table) => table,
        Err(e) => {
            let error_msg = format!("Pipeline table not found in script: {}", e);
            tracing::error!("{}", error_msg);
            add_log(&log_buffer, rivet_core::types::LogLevel::Error, &error_msg);

            let logs = ctx.drain_logs().map_err(|e| anyhow::anyhow!(e))?;
            if !logs.is_empty() {
                if let Err(e) = client.send_logs(*job_id, logs.clone()).await {
                    tracing::warn!("Failed to send error logs: {:?}", e);
                }
            }

            return Ok(execution::ExecutionResult::Failure {
                error: error_msg,
                logs,
            });
        }
    };

    let stages: mlua::Table = match pipeline_table.get("stages") {
        Ok(stages) => stages,
        Err(e) => {
            let error_msg = format!("Pipeline stages not found: {}", e);
            tracing::error!("{}", error_msg);
            add_log(&log_buffer, rivet_core::types::LogLevel::Error, &error_msg);

            let logs = ctx.drain_logs().map_err(|e| anyhow::anyhow!(e))?;
            if !logs.is_empty() {
                if let Err(e) = client.send_logs(*job_id, logs.clone()).await {
                    tracing::warn!("Failed to send error logs: {:?}", e);
                }
            }

            return Ok(execution::ExecutionResult::Failure {
                error: error_msg,
                logs,
            });
        }
    };

    // Execute each stage
    let stage_count: usize = stages.len()? as usize;
    tracing::info!("Found {} stage(s) to execute", stage_count);

    for i in 1..=stage_count {
        let stage: mlua::Table = match stages.get(i) {
            Ok(stage) => stage,
            Err(e) => {
                let error_msg = format!("Failed to get stage {}: {}", i, e);
                tracing::error!("{}", error_msg);
                add_log(&log_buffer, rivet_core::types::LogLevel::Error, &error_msg);

                let logs = ctx.drain_logs().map_err(|e| anyhow::anyhow!(e))?;
                if !logs.is_empty() {
                    if let Err(e) = client.send_logs(*job_id, logs.clone()).await {
                        tracing::warn!("Failed to send error logs: {:?}", e);
                    }
                }

                return Ok(execution::ExecutionResult::Failure {
                    error: error_msg,
                    logs,
                });
            }
        };

        let stage_name: String = stage.get("name").unwrap_or_else(|_| format!("stage-{}", i));

        add_log(
            &log_buffer,
            rivet_core::types::LogLevel::Info,
            &format!("Executing stage: {}", stage_name),
        );
        tracing::info!("Executing stage {}/{}: {}", i, stage_count, stage_name);

        let script_fn: mlua::Function = match stage.get("script") {
            Ok(func) => func,
            Err(e) => {
                let error_msg = format!("Stage '{}' has no script function: {}", stage_name, e);
                tracing::error!("{}", error_msg);
                add_log(&log_buffer, rivet_core::types::LogLevel::Error, &error_msg);

                let logs = ctx.drain_logs().map_err(|e| anyhow::anyhow!(e))?;
                if !logs.is_empty() {
                    if let Err(e) = client.send_logs(*job_id, logs.clone()).await {
                        tracing::warn!("Failed to send error logs: {:?}", e);
                    }
                }

                return Ok(execution::ExecutionResult::Failure {
                    error: error_msg,
                    logs,
                });
            }
        };

        // Execute the stage script
        if let Err(e) = script_fn.call::<()>(()) {
            let error_msg = format!("Stage '{}' failed: {}", stage_name, e);
            tracing::error!("{}", error_msg);
            add_log(&log_buffer, rivet_core::types::LogLevel::Error, &error_msg);

            let logs = ctx.drain_logs().map_err(|e| anyhow::anyhow!(e))?;
            if !logs.is_empty() {
                if let Err(e) = client.send_logs(*job_id, logs.clone()).await {
                    tracing::warn!("Failed to send error logs: {:?}", e);
                }
            }

            return Ok(execution::ExecutionResult::Failure {
                error: error_msg,
                logs,
            });
        }

        add_log(
            &log_buffer,
            rivet_core::types::LogLevel::Info,
            &format!("Stage '{}' completed successfully", stage_name),
        );

        // Send logs periodically after each stage
        let logs = ctx.drain_logs().map_err(|e| anyhow::anyhow!(e))?;
        if !logs.is_empty() {
            if let Err(e) = client.send_logs(*job_id, logs).await {
                tracing::warn!("Failed to send logs after stage '{}': {:?}", stage_name, e);
            }
        }
    }

    add_log(
        &log_buffer,
        rivet_core::types::LogLevel::Info,
        "Pipeline execution completed",
    );

    // Send final logs to orchestrator
    let logs = ctx.drain_logs().map_err(|e| anyhow::anyhow!(e))?;
    if !logs.is_empty() {
        if let Err(e) = client.send_logs(*job_id, logs.clone()).await {
            tracing::warn!("Failed to send final logs: {:?}", e);
        }
    }

    Ok(execution::ExecutionResult::Success { output: None, logs })
}

fn load_modules(ctx: &execution::ExecutionContext) -> Result<()> {
    tracing::debug!("Loading modules into Lua context");

    // Create a log module
    let log_buffer = ctx.log_buffer.clone();
    let globals = ctx.lua.globals();

    let log_table = ctx.lua.create_table()?;

    // log.info(message)
    let log_buffer_info = log_buffer.clone();
    let info_fn = ctx.lua.create_function(move |_, message: String| {
        add_log(
            &log_buffer_info,
            rivet_core::types::LogLevel::Info,
            &message,
        );
        Ok(())
    })?;
    log_table.set("info", info_fn)?;

    // log.debug(message)
    let log_buffer_debug = log_buffer.clone();
    let debug_fn = ctx.lua.create_function(move |_, message: String| {
        add_log(
            &log_buffer_debug,
            rivet_core::types::LogLevel::Debug,
            &message,
        );
        Ok(())
    })?;
    log_table.set("debug", debug_fn)?;

    // log.warning(message)
    let log_buffer_warn = log_buffer.clone();
    let warn_fn = ctx.lua.create_function(move |_, message: String| {
        add_log(
            &log_buffer_warn,
            rivet_core::types::LogLevel::Warning,
            &message,
        );
        Ok(())
    })?;
    log_table.set("warning", warn_fn)?;

    // log.error(message)
    let log_buffer_error = log_buffer.clone();
    let error_fn = ctx.lua.create_function(move |_, message: String| {
        add_log(
            &log_buffer_error,
            rivet_core::types::LogLevel::Error,
            &message,
        );
        Ok(())
    })?;
    log_table.set("error", error_fn)?;

    globals.set("log", log_table)?;

    // Create an env module for accessing parameters
    let env_table = ctx.lua.create_table()?;
    let params_clone = ctx.parameters.clone();
    let get_fn = ctx.lua.create_function(move |lua, key: String| {
        if let Some(value) = params_clone.get(&key) {
            // Convert serde_json::Value to Lua value
            let lua_value = json_to_lua(lua, value)?;
            Ok(Some(lua_value))
        } else {
            Ok(None)
        }
    })?;
    env_table.set("get", get_fn)?;
    globals.set("env", env_table)?;

    tracing::debug!("Modules loaded successfully");
    Ok(())
}

fn add_log(
    log_buffer: &Arc<Mutex<Vec<LogEntry>>>,
    level: rivet_core::types::LogLevel,
    message: &str,
) {
    let entry = LogEntry {
        timestamp: chrono::Utc::now(),
        level,
        message: message.to_string(),
    };

    if let Ok(mut buffer) = log_buffer.lock() {
        buffer.push(entry);
    } else {
        tracing::error!("Failed to lock log buffer");
    }
}

fn json_to_lua(lua: &mlua::Lua, value: &serde_json::Value) -> mlua::Result<mlua::Value> {
    match value {
        serde_json::Value::Null => Ok(mlua::Value::Nil),
        serde_json::Value::Bool(b) => Ok(mlua::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(mlua::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(mlua::Value::Number(f))
            } else {
                Ok(mlua::Value::Nil)
            }
        }
        serde_json::Value::String(s) => Ok(mlua::Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua(lua, item)?)?;
            }
            Ok(mlua::Value::Table(table))
        }
        serde_json::Value::Object(obj) => {
            let table = lua.create_table()?;
            for (key, val) in obj {
                table.set(key.as_str(), json_to_lua(lua, val)?)?;
            }
            Ok(mlua::Value::Table(table))
        }
    }
}

// =============================================================================
// Orchestrator Client
// =============================================================================

struct OrchestratorClient {
    base_url: String,
    runner_id: String,
    client: reqwest::Client,
}

impl OrchestratorClient {
    fn new(base_url: &str, runner_id: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            runner_id: runner_id.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn list_scheduled_jobs(&self) -> Result<Vec<rivet_core::types::JobDto>> {
        let url = format!("{}/job/list/scheduled", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to list scheduled jobs")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response
            .json()
            .await
            .context("Failed to parse scheduled jobs response")
    }

    async fn execute_job(&self, job_id: Uuid) -> Result<ExecuteJobResponse> {
        let url = format!("{}/job/execute/{}", self.base_url, job_id);
        let request_body = serde_json::json!({
            "runner_id": self.runner_id
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to execute job")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response
            .json()
            .await
            .context("Failed to parse execute job response")
    }

    async fn complete_job(
        &self,
        job_id: Uuid,
        status: JobStatus,
        result: Option<JobResult>,
    ) -> Result<()> {
        let url = format!("{}/job/{}/complete", self.base_url, job_id);
        let request_body = serde_json::json!({
            "status": status,
            "result": result
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to complete job")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        Ok(())
    }

    async fn send_logs(&self, job_id: Uuid, logs: Vec<LogEntry>) -> Result<()> {
        if logs.is_empty() {
            return Ok(());
        }

        let url = format!("{}/job/{}/logs", self.base_url, job_id);
        let response = self
            .client
            .post(&url)
            .json(&logs)
            .send()
            .await
            .context("Failed to send logs")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
struct ExecuteJobResponse {
    job_id: Uuid,
    pipeline: Pipeline,
    parameters: HashMap<String, serde_json::Value>,
}
