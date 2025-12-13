//! Runner-related API endpoints

use crate::OrchestratorClient;
use crate::error::Result;
use rivet_core::domain::runner::Runner;
use rivet_core::dto::runner::RegisterRunner;

impl OrchestratorClient {
    // =============================================================================
    // Runner Registration & Lifecycle
    // =============================================================================

    /// Register a runner with the orchestrator
    ///
    /// # Arguments
    /// * `runner_id` - Unique identifier for this runner
    /// * `capabilities` - List of capability strings (e.g., "process", "plugin.git", "container.docker")
    ///
    /// # Returns
    /// The registered runner
    ///
    /// # Example
    /// ```no_run
    /// # use rivet_client::OrchestratorClient;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = OrchestratorClient::new("http://localhost:8080");
    /// let runner = client.register_runner(
    ///     "my-runner-001",
    ///     vec!["process".to_string(), "plugin.git".to_string()]
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_runner(
        &self,
        runner_id: &str,
        capabilities: Vec<String>,
    ) -> Result<Runner> {
        let url = format!("{}/api/runners/register", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&RegisterRunner {
                runner_id: runner_id.to_string(),
                capabilities,
            })
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Send a heartbeat to the orchestrator
    ///
    /// This keeps the runner marked as "alive" in the orchestrator's registry.
    /// Should be called periodically (e.g., every 30 seconds).
    ///
    /// # Arguments
    /// * `runner_id` - The ID of the runner sending the heartbeat
    pub async fn send_heartbeat(&self, runner_id: &str) -> Result<()> {
        let url = format!("{}/api/runners/{}/heartbeat", self.base_url, runner_id);
        let response = self.client.post(&url).send().await?;

        self.handle_empty_response(response).await
    }

    // =============================================================================
    // Runner Query
    // =============================================================================

    /// List all registered runners
    ///
    /// # Returns
    /// A list of all runners
    pub async fn list_runners(&self) -> Result<Vec<Runner>> {
        let url = format!("{}/api/runners", self.base_url);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// Get details for a specific runner
    ///
    /// # Arguments
    /// * `runner_id` - The runner ID
    ///
    /// # Returns
    /// The runner details
    pub async fn get_runner(&self, runner_id: &str) -> Result<Runner> {
        let url = format!("{}/api/runners/{}", self.base_url, runner_id);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// Delete a runner registration
    ///
    /// # Arguments
    /// * `runner_id` - The runner ID to delete
    pub async fn delete_runner(&self, runner_id: &str) -> Result<()> {
        let url = format!("{}/api/runners/{}", self.base_url, runner_id);
        let response = self.client.delete(&url).send().await?;

        self.handle_empty_response(response).await
    }
}
