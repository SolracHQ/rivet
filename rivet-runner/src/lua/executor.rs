//! Lua executor service
//!
//! Handles all Lua-related execution logic including:
//! - Creating execution sandboxes
//! - Registering core modules
//! - Loading and executing pipelines
//! - Running individual stages

use anyhow::{Context as AnyhowContext, Result};
use rivet_core::domain::job::JobResult;
use rivet_lua::{PipelineMetadata, create_execution_sandbox};
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
    /// * `metadata` - Parsed pipeline metadata
    /// * `pipeline_source` - The Lua source code
    ///
    /// # Returns
    /// The job result (success or error)
    pub async fn execute_pipeline(
        &self,
        job_id: Uuid,
        metadata: PipelineMetadata,
        pipeline_source: &str,
    ) -> JobResult {
        self.context
            .log_info(format!("Starting pipeline: {}", metadata.name));

        // Create Lua sandbox
        let lua = match self.create_sandbox() {
            Ok(lua) => lua,
            Err(e) => {
                return self.log_and_fail("Failed to create execution sandbox", e);
            }
        };

        // Load pipeline
        if let Err(e) = self.load_pipeline(&lua, pipeline_source) {
            return self.log_and_fail("Failed to load pipeline", e);
        }

        // Execute stages
        for (idx, stage) in metadata.stages.iter().enumerate() {
            info!(
                "Executing stage {}/{}: {}",
                idx + 1,
                metadata.stages.len(),
                stage.name
            );

            self.context
                .log_info(format!("Starting stage: {}", stage.name));

            if let Err(e) = self.execute_stage(&lua, idx, &stage.name) {
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
        let lua = create_execution_sandbox().context("Failed to create base sandbox")?;

        // Register log module
        register_log_module(&lua, Arc::clone(&self.context))
            .context("Failed to register log module")?;

        // Register input module
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

    /// Loads a pipeline into the Lua sandbox
    fn load_pipeline(&self, lua: &mlua::Lua, pipeline_source: &str) -> Result<()> {
        let pipeline_table: mlua::Table = lua
            .load(pipeline_source)
            .set_name("pipeline")
            .eval()
            .context("Failed to evaluate pipeline source")?;

        lua.globals()
            .set("pipeline", pipeline_table)
            .context("Failed to set pipeline global")?;

        Ok(())
    }

    /// Executes a single stage
    fn execute_stage(&self, lua: &mlua::Lua, stage_idx: usize, stage_name: &str) -> Result<()> {
        debug!("Executing stage: {}", stage_name);

        let pipeline: mlua::Table = lua
            .globals()
            .get("pipeline")
            .context("Pipeline table not found in globals")?;

        let stages: mlua::Table = pipeline
            .get("stages")
            .context("Stages array not found in pipeline")?;

        // Lua arrays are 1-indexed
        let stage_table: mlua::Table = stages
            .get(stage_idx + 1)
            .context(format!("Stage at index {} not found", stage_idx))?;

        let script: mlua::Function = stage_table.get("script").context(format!(
            "Script function not found for stage '{}'",
            stage_name
        ))?;

        script
            .call::<()>(())
            .map_err(|e| anyhow::anyhow!("Stage '{}' execution failed: {}", stage_name, e))?;

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
