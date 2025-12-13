//! Pipeline-related API endpoints

use crate::OrchestratorClient;
use crate::error::Result;
use rivet_core::domain::pipeline::Pipeline;
use rivet_core::dto::pipeline::CreatePipeline;
use uuid::Uuid;

impl OrchestratorClient {
    // =============================================================================
    // Pipeline Management
    // =============================================================================

    /// Create a new pipeline
    ///
    /// # Arguments
    /// * `req` - The pipeline creation request
    ///
    /// # Returns
    /// The created pipeline
    ///
    /// # Example
    /// ```no_run
    /// # use rivet_client::OrchestratorClient;
    /// # use rivet_core::dto::pipeline::CreatePipeline;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = OrchestratorClient::new("http://localhost:8080");
    /// let pipeline = client.create_pipeline(CreatePipeline {
    ///     name: "my-pipeline".to_string(),
    ///     script: "-- Lua script here".to_string(),
    ///     schedule: None,
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_pipeline(&self, req: CreatePipeline) -> Result<Pipeline> {
        let url = format!("{}/api/pipeline/create", self.base_url);
        let response = self.client.post(&url).json(&req).send().await?;

        self.handle_response(response).await
    }

    /// List all pipelines
    ///
    /// # Returns
    /// A list of all pipelines
    pub async fn list_pipelines(&self) -> Result<Vec<Pipeline>> {
        let url = format!("{}/api/pipeline/list", self.base_url);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// Get a pipeline by ID
    ///
    /// # Arguments
    /// * `pipeline_id` - The pipeline UUID
    ///
    /// # Returns
    /// The pipeline details
    pub async fn get_pipeline(&self, pipeline_id: Uuid) -> Result<Pipeline> {
        let url = format!("{}/api/pipeline/{}", self.base_url, pipeline_id);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// Delete a pipeline
    ///
    /// # Arguments
    /// * `pipeline_id` - The pipeline UUID to delete
    pub async fn delete_pipeline(&self, pipeline_id: Uuid) -> Result<()> {
        let url = format!("{}/api/pipeline/{}", self.base_url, pipeline_id);
        let response = self.client.delete(&url).send().await?;

        self.handle_empty_response(response).await
    }
}
