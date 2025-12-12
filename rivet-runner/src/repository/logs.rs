//! Logs repository
//!
//! Handles sending logs to the orchestrator.
//! This is a stateless HTTP client - log buffering is handled by the service layer.

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use rivet_core::domain::log::LogEntry;
use uuid::Uuid;

/// Repository trait for log-related operations with the orchestrator
#[async_trait]
pub trait LogRepository: Send + Sync {
    /// Sends logs to the orchestrator for a specific job
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job these logs belong to
    /// * `entries` - The log entries to send
    async fn send_logs(&self, job_id: Uuid, entries: Vec<LogEntry>) -> Result<()>;
}

/// HTTP implementation of LogRepository
pub struct HttpLogRepository {
    client: Client,
    orchestrator_url: String,
}

impl HttpLogRepository {
    /// Creates a new HTTP log repository
    ///
    /// # Arguments
    /// * `orchestrator_url` - Base URL of the orchestrator (e.g., "http://localhost:8080")
    pub fn new(orchestrator_url: String) -> Self {
        Self {
            client: Client::new(),
            orchestrator_url,
        }
    }
}

#[async_trait]
impl LogRepository for HttpLogRepository {
    async fn send_logs(&self, job_id: Uuid, entries: Vec<LogEntry>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let url = format!("{}/api/jobs/{}/logs", self.orchestrator_url, job_id);

        let response = self
            .client
            .post(&url)
            .json(&entries)
            .send()
            .await
            .context("Failed to send logs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to send logs: {} - {}", status, body);
        }

        Ok(())
    }
}
