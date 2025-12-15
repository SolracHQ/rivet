//! Rivet HTTP Client
//!
//! A simple, type-safe HTTP client for communicating with the Rivet orchestrator API.
//!
//! This crate provides a unified interface for both CLI and runner components to interact
//! with the orchestrator, eliminating code duplication and ensuring consistency.
//!
//! # Example
//!
//! ```no_run
//! use rivet_client::OrchestratorClient;
//! use rivet_core::dto::pipeline::CreatePipeline;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = OrchestratorClient::new("http://localhost:8080");
//!
//!     // Create a pipeline
//!     let pipeline = client.create_pipeline(CreatePipeline {
//!         script: "return { name = 'test', stages = {} }".to_string(),
//!     }).await?;
//!
//!     println!("Created pipeline: {}", pipeline.id);
//!     Ok(())
//! }
//! ```

pub mod error;
mod jobs;
mod pipelines;
mod runners;

// Re-export commonly used types
pub use error::{ClientError, Result};
pub use rivet_core::dto::job::JobExecutionInfo;

use reqwest::Client;
use serde::de::DeserializeOwned;

/// HTTP client for the Rivet orchestrator API
///
/// This client provides methods for all orchestrator API endpoints, organized
/// into logical groups:
/// - Pipeline management (create, list, get, delete)
/// - Job lifecycle (launch, claim, complete, status updates)
/// - Runner registration and heartbeats
/// - Log streaming
#[derive(Debug, Clone)]
pub struct OrchestratorClient {
    /// Base URL of the orchestrator (e.g., "http://localhost:8080")
    base_url: String,
    /// HTTP client instance
    client: Client,
}

impl OrchestratorClient {
    /// Create a new orchestrator client
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the orchestrator API (e.g., "http://localhost:8080")
    ///
    /// # Example
    /// ```
    /// use rivet_client::OrchestratorClient;
    ///
    /// let client = OrchestratorClient::new("http://localhost:8080");
    /// ```
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    /// Create a new orchestrator client with a custom HTTP client
    ///
    /// This allows you to configure timeouts, proxies, TLS settings, etc.
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the orchestrator API
    /// * `client` - A configured reqwest Client
    ///
    /// # Example
    /// ```
    /// use rivet_client::OrchestratorClient;
    /// use reqwest::Client;
    /// use std::time::Duration;
    ///
    /// let http_client = Client::builder()
    ///     .timeout(Duration::from_secs(30))
    ///     .build()
    ///     .unwrap();
    ///
    /// let client = OrchestratorClient::with_client("http://localhost:8080", http_client);
    /// ```
    pub fn with_client(base_url: impl Into<String>, client: Client) -> Self {
        let base_url = base_url.into();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Get the base URL of the orchestrator
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // =============================================================================
    // Response Handlers
    // =============================================================================

    /// Handle an API response and deserialize JSON
    ///
    /// This method checks the status code and returns an appropriate error if
    /// the request failed, or deserializes the response body if successful.
    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClientError::api_error(status.as_u16(), error_text));
        }

        response
            .json()
            .await
            .map_err(|e| ClientError::ParseError(format!("Failed to parse JSON response: {}", e)))
    }

    /// Handle an API response that returns no content (e.g., DELETE operations)
    ///
    /// This method checks the status code and returns an error if the request failed.
    async fn handle_empty_response(&self, response: reqwest::Response) -> Result<()> {
        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClientError::api_error(status.as_u16(), error_text));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OrchestratorClient::new("http://localhost:8080");
        assert_eq!(client.base_url(), "http://localhost:8080");
    }

    #[test]
    fn test_client_trims_trailing_slash() {
        let client = OrchestratorClient::new("http://localhost:8080/");
        assert_eq!(client.base_url(), "http://localhost:8080");
    }

    #[test]
    fn test_client_with_custom_client() {
        let http_client = Client::new();
        let client = OrchestratorClient::with_client("http://localhost:8080", http_client);
        assert_eq!(client.base_url(), "http://localhost:8080");
    }
}
