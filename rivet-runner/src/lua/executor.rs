//! Lua executor service
//!
//! Handles all Lua-related execution logic including:
//! - Creating execution sandboxes
//! - Registering core modules
//! - Parsing and executing pipelines with PipelineDefinition
//! - Running individual stages

use anyhow::{Context as AnyhowContext, Result};
use rivet_core::domain::job::JobResult;
use rivet_lua::{create_sandbox, parse_pipeline_definition};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::context::Context;
use crate::lua::modules::{
    register_container_module, register_input_module, register_log_module, register_process_module,
};

/// Lua executor service
pub struct LuaExecutor {
    context: Arc<Context>,
}

impl LuaExecutor {
    /// Creates a new Lua executor with the given context
    pub fn new(context: Arc<Context>) -> Self {
        Self { context }
    }

    /// Executes a pipeline from source code
    ///
    /// # Arguments
    /// * `job_id` - The job ID for logging
    /// * `pipeline_source` - The Lua source code
    ///
    /// # Returns
    /// The job result (success or error)
    pub async fn execute_pipeline(&self, job_id: Uuid, pipeline_source: &str) -> JobResult {
        // Create Lua sandbox with modules registered
        let lua = match self.create_sandbox() {
            Ok(lua) => lua,
            Err(e) => {
                return self.log_and_fail("Failed to create execution sandbox", e);
            }
        };

        // Parse the full pipeline definition (includes functions)
        let definition = match parse_pipeline_definition(&lua, pipeline_source) {
            Ok(def) => def,
            Err(e) => {
                return self.log_and_fail("Failed to parse pipeline definition", e);
            }
        };

        self.context
            .log_info(format!("Starting pipeline: {}", definition.name));

        info!(
            "Executing pipeline '{}' with {} stages",
            definition.name,
            definition.stages.len()
        );

        // Execute stages
        for (idx, stage) in definition.stages.iter().enumerate() {
            info!(
                "Executing stage {}/{}: {}",
                idx + 1,
                definition.stages.len(),
                stage.name
            );

            self.context
                .log_info(format!("Starting stage: {}", stage.name));

            // Check condition if present
            if let Some(ref condition) = stage.condition {
                match self.evaluate_condition(condition, &stage.name) {
                    Ok(true) => {
                        debug!("Stage '{}' condition passed", stage.name);
                    }
                    Ok(false) => {
                        info!("Stage '{}' skipped (condition returned false)", stage.name);
                        self.context.log_info(format!(
                            "Stage '{}' skipped (condition not met)",
                            stage.name
                        ));
                        continue;
                    }
                    Err(e) => {
                        error!("Stage '{}' condition evaluation failed: {}", stage.name, e);
                        self.context.log_error(format!(
                            "Stage '{}' condition evaluation failed: {}",
                            stage.name, e
                        ));
                        return JobResult::error(
                            format!("Stage '{}' condition failed: {}", stage.name, e),
                            1,
                        );
                    }
                }
            }

            // Execute stage script
            if let Err(e) = self.execute_stage(&stage.script, &stage.name) {
                error!("Stage '{}' failed: {}", stage.name, e);
                self.context
                    .log_error(format!("Stage '{}' failed: {}", stage.name, e));
                return JobResult::error(format!("Stage '{}' failed: {}", stage.name, e), 1);
            }

            self.context
                .log_info(format!("Stage '{}' completed", stage.name));
        }

        info!("Job {} completed successfully", job_id);
        self.context
            .log_info("Pipeline completed successfully".to_string());

        JobResult::success()
    }

    /// Creates and configures a Lua execution sandbox
    fn create_sandbox(&self) -> Result<mlua::Lua> {
        let lua = create_sandbox().context("Failed to create base sandbox")?;

        // Register log module
        register_log_module(&lua, Arc::clone(&self.context))
            .context("Failed to register log module")?;

        // Register input module with proper input definitions
        register_input_module(&lua, self.context.inputs.clone())
            .context("Failed to register input module")?;

        // Register process module
        register_process_module(&lua, Arc::clone(&self.context))
            .context("Failed to register process module")?;

        // Register container module
        register_container_module(&lua, Arc::clone(&self.context))
            .context("Failed to register container module")?;

        // TODO: Register output module

        Ok(lua)
    }

    /// Evaluates a stage condition function
    fn evaluate_condition(&self, condition: &mlua::Function, stage_name: &str) -> Result<bool> {
        debug!("Evaluating condition for stage: {}", stage_name);

        let result: bool = condition
            .call(())
            .map_err(|e| anyhow::anyhow!("Condition evaluation failed: {}", e))?;

        Ok(result)
    }

    /// Executes a single stage script function
    fn execute_stage(&self, script: &mlua::Function, stage_name: &str) -> Result<()> {
        debug!("Executing stage: {}", stage_name);

        script
            .call::<()>(())
            .map_err(|e| anyhow::anyhow!("Stage execution failed: {}", e))?;

        debug!("Stage '{}' completed successfully", stage_name);
        Ok(())
    }

    /// Logs an error and returns a failed JobResult
    fn log_and_fail(&self, message: &str, error: anyhow::Error) -> JobResult {
        let full_message = format!("{}: {}", message, error);
        error!("{}", full_message);
        self.context.log_error(full_message.clone());
        JobResult::failed(full_message)
    }
}
