//! Execution service
//!
//! Handles pipeline job execution including:
//! - Setting up Lua execution sandbox with modules
//! - Executing pipeline stages one by one
//! - Managing execution lifecycle
//!
//! This service contains the core business logic for running pipelines.

use anyhow::{Context, Result};
use async_trait::async_trait;
use rivet_core::domain::job::JobResult;
use rivet_core::domain::log::{LogEntry, LogLevel};
use rivet_lua::{PipelineMetadata, create_execution_sandbox};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::lua::modules::{register_input_module, register_log_module};
use crate::service::log_buffer::LogBufferService;

/// Service trait for executing pipeline jobs
#[async_trait]
pub trait ExecutionService: Send + Sync {
    /// Executes a pipeline job
    ///
    /// # Arguments
    /// * `job_id` - The job ID
    /// * `metadata` - The parsed pipeline metadata
    /// * `pipeline_source` - The Lua source code of the pipeline
    /// * `parameters` - Job parameters to inject as input variables
    /// * `log_buffer` - Shared buffer for collecting logs
    ///
    /// # Returns
    /// The job result (success/failure)
    async fn execute_job(
        &self,
        job_id: Uuid,
        metadata: PipelineMetadata,
        pipeline_source: &str,
        parameters: HashMap<String, JsonValue>,
        log_buffer: Arc<dyn LogBufferService>,
    ) -> Result<JobResult>;
}

/// Standard implementation of ExecutionService
pub struct StandardExecutionService {}

impl StandardExecutionService {
    /// Creates a new standard execution service
    pub fn new() -> Self {
        Self {}
    }

    /// Creates an execution context with all necessary modules
    fn create_execution_context(
        &self,
        parameters: HashMap<String, JsonValue>,
        log_buffer: Arc<dyn LogBufferService>,
    ) -> Result<mlua::Lua> {
        // Create base execution sandbox
        let lua = create_execution_sandbox().context("Failed to create execution sandbox")?;

        // Register log module with buffered sink
        register_log_module(&lua, log_buffer).context("Failed to register log module")?;

        // Register input module with job parameters
        register_input_module(&lua, parameters).context("Failed to register input module")?;

        // TODO: Register output module
        // TODO: Register process module
        // TODO: Register container module

        Ok(lua)
    }

    /// Executes a single stage
    fn execute_stage(&self, lua: &mlua::Lua, stage_idx: usize, stage_name: &str) -> Result<()> {
        debug!("Executing stage: {}", stage_name);

        // Get the pipeline table
        let pipeline: mlua::Table = lua
            .globals()
            .get("pipeline")
            .context("Pipeline table not found in globals")?;

        // Get the stages array
        let stages: mlua::Table = pipeline
            .get("stages")
            .context("Stages array not found in pipeline")?;

        // Get this specific stage (Lua arrays are 1-indexed)
        let stage_table: mlua::Table = stages
            .get(stage_idx + 1)
            .context(format!("Stage at index {} not found", stage_idx))?;

        // Get and execute the script function
        let script: mlua::Function = stage_table.get("script").context(format!(
            "Script function not found for stage '{}'",
            stage_name
        ))?;

        // Execute the stage script
        script
            .call::<()>(())
            .map_err(|e| anyhow::anyhow!("Stage '{}' execution failed: {}", stage_name, e))?;

        debug!("Stage '{}' completed successfully", stage_name);
        Ok(())
    }
}

impl Default for StandardExecutionService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExecutionService for StandardExecutionService {
    async fn execute_job(
        &self,
        job_id: Uuid,
        metadata: PipelineMetadata,
        pipeline_source: &str,
        parameters: HashMap<String, JsonValue>,
        log_buffer: Arc<dyn LogBufferService>,
    ) -> Result<JobResult> {
        info!(
            "Starting execution of job {} - pipeline '{}'",
            job_id, metadata.name
        );

        // Add initial log entry
        log_buffer.add_entry(LogEntry {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Info,
            message: format!("Starting pipeline: {}", metadata.name),
        });

        // Create execution context with modules
        let lua = match self.create_execution_context(parameters, log_buffer.clone()) {
            Ok(lua) => lua,
            Err(e) => {
                error!("Failed to create execution context: {}", e);
                log_buffer.add_entry(LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Error,
                    message: format!("Failed to create execution context: {}", e),
                });
                return Ok(JobResult {
                    success: false,
                    exit_code: 1,
                    output: None,
                    error_message: Some(format!("Failed to create execution context: {}", e)),
                });
            }
        };

        // Load the pipeline into the sandbox
        // The pipeline should return a table with the pipeline definition
        let pipeline_table: mlua::Table =
            match lua.load(pipeline_source).set_name("pipeline").eval() {
                Ok(table) => table,
                Err(e) => {
                    error!("Failed to load pipeline: {}", e);
                    log_buffer.add_entry(LogEntry {
                        timestamp: chrono::Utc::now(),
                        level: LogLevel::Error,
                        message: format!("Failed to load pipeline: {}", e),
                    });
                    return Ok(JobResult {
                        success: false,
                        exit_code: 1,
                        output: None,
                        error_message: Some(format!("Failed to load pipeline: {}", e)),
                    });
                }
            };

        // Store the pipeline table in globals for stage access
        if let Err(e) = lua.globals().set("pipeline", pipeline_table) {
            error!("Failed to set pipeline global: {}", e);
            log_buffer.add_entry(LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Error,
                message: format!("Failed to set pipeline global: {}", e),
            });
            return Ok(JobResult {
                success: false,
                exit_code: 1,
                output: None,
                error_message: Some(format!("Failed to set pipeline global: {}", e)),
            });
        }

        // Execute each stage
        for (idx, stage) in metadata.stages.iter().enumerate() {
            info!(
                "Executing stage {}/{}: {}",
                idx + 1,
                metadata.stages.len(),
                stage.name
            );

            log_buffer.add_entry(LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Info,
                message: format!("Starting stage: {}", stage.name),
            });

            if let Err(e) = self.execute_stage(&lua, idx, &stage.name) {
                error!("Stage '{}' failed: {}", stage.name, e);
                log_buffer.add_entry(LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Error,
                    message: format!("Stage '{}' failed: {}", stage.name, e),
                });
                return Ok(JobResult {
                    success: false,
                    exit_code: 1,
                    output: None,
                    error_message: Some(format!("Stage '{}' failed: {}", stage.name, e)),
                });
            }

            log_buffer.add_entry(LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Info,
                message: format!("Stage '{}' completed", stage.name),
            });
        }

        info!("Job {} completed successfully", job_id);
        log_buffer.add_entry(LogEntry {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Info,
            message: "Pipeline completed successfully".to_string(),
        });

        Ok(JobResult {
            success: true,
            exit_code: 0,
            output: None,
            error_message: None,
        })
    }
}
