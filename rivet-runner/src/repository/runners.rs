//! Runners repository
//!
//! Handles communication with the orchestrator for runner-related operations:
//! - Registering runner capabilities
//! - Sending heartbeats to maintain runner status

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;

/// Repository trait for runner-related operations with the orchestrator
#[async_trait]
pub trait RunnerRepository: Send + Sync {
    /// Registers this runner's capabilities with the orchestrator
    ///
    /// This should be called when the runner starts up to inform the
    /// orchestrator which capabilities this runner has available.
    ///
    /// # Arguments
    /// * `capabilities` - List of capability strings (e.g., "process", "plugin.git", "container.docker")
    async fn register_capabilities(&self, capabilities: Vec<String>) -> Result<()>;

    /// Sends a heartbeat to the orchestrator
    ///
    /// This keeps the runner marked as "alive" in the orchestrator's registry.
    /// Should be called periodically (e.g., every 30 seconds).
    async fn send_heartbeat(&self) -> Result<()>;
}

/// HTTP implementation of RunnerRepository
pub struct HttpRunnerRepository {
    client: Client,
    orchestrator_url: String,
    runner_id: String,
}

impl HttpRunnerRepository {
    /// Creates a new HTTP runner repository
    ///
    /// # Arguments
    /// * `orchestrator_url` - Base URL of the orchestrator (e.g., "http://localhost:8080")
    /// * `runner_id` - Unique identifier for this runner
    pub fn new(orchestrator_url: String, runner_id: String) -> Self {
        Self {
            client: Client::new(),
            orchestrator_url,
            runner_id,
        }
    }
}

#[async_trait]
impl RunnerRepository for HttpRunnerRepository {
    async fn register_capabilities(&self, capabilities: Vec<String>) -> Result<()> {
        let url = format!("{}/api/runners/register", self.orchestrator_url);

        let request = RegisterRequest {
            runner_id: self.runner_id.clone(),
            capabilities,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to register capabilities")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to register capabilities: {} - {}", status, body);
        }

        Ok(())
    }

    async fn send_heartbeat(&self) -> Result<()> {
        let url = format!(
            "{}/api/runners/{}/heartbeat",
            self.orchestrator_url, self.runner_id
        );

        let response = self
            .client
            .post(&url)
            .send()
            .await
            .context("Failed to send heartbeat")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to send heartbeat: {} - {}", status, body);
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct RegisterRequest {
    runner_id: String,
    capabilities: Vec<String>,
}
