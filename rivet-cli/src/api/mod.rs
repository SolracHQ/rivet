//! API client module
//!
//! HTTP client for communicating with the Rivet orchestrator API.

use anyhow::{Context, Result};
use reqwest::Client;
use rivet_core::domain::job::Job;
use rivet_core::domain::log::LogEntry;
use rivet_core::domain::pipeline::Pipeline;
use rivet_core::dto::job::{CreateJob, JobSummary};
use rivet_core::dto::pipeline::{CreatePipeline, PipelineSummary};
use rivet_core::dto::runner::RunnerSummary;
use uuid::Uuid;

/// HTTP client for the Rivet orchestrator API
pub struct ApiClient {
    base_url: String,
    client: Client,
}

impl ApiClient {
    /// Create a new API client
    ///
    /// # Arguments
    /// * `base_url` - The base URL of the orchestrator API
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    /// Create a new pipeline
    ///
    /// # Arguments
    /// * `req` - The pipeline creation request
    ///
    /// # Returns
    /// The created pipeline
    pub async fn create_pipeline(&self, req: CreatePipeline) -> Result<Pipeline> {
        let url = format!("{}/api/pipeline/create", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to send create pipeline request")?;

        self.handle_response(response).await
    }

    /// List all pipelines
    ///
    /// # Returns
    /// A list of pipeline summaries
    pub async fn list_pipelines(&self) -> Result<Vec<PipelineSummary>> {
        let url = format!("{}/api/pipeline/list", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send list pipelines request")?;

        self.handle_response(response).await
    }

    /// Get a pipeline by ID
    ///
    /// # Arguments
    /// * `id` - The pipeline UUID
    ///
    /// # Returns
    /// The pipeline details
    pub async fn get_pipeline(&self, id: Uuid) -> Result<Pipeline> {
        let url = format!("{}/api/pipeline/{}", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send get pipeline request")?;

        self.handle_response(response).await
    }

    /// Delete a pipeline
    ///
    /// # Arguments
    /// * `id` - The pipeline UUID to delete
    pub async fn delete_pipeline(&self, id: Uuid) -> Result<()> {
        let url = format!("{}/api/pipeline/{}", self.base_url, id);
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .context("Failed to send delete pipeline request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        Ok(())
    }

    /// Launch a new job from a pipeline
    ///
    /// # Arguments
    /// * `req` - The job creation request
    ///
    /// # Returns
    /// The created job
    pub async fn launch_job(&self, req: CreateJob) -> Result<Job> {
        let url = format!("{}/api/pipeline/launch", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .context("Failed to send launch job request")?;

        self.handle_response(response).await
    }

    /// List all scheduled jobs
    ///
    /// # Returns
    /// A list of scheduled job summaries
    pub async fn list_scheduled_jobs(&self) -> Result<Vec<JobSummary>> {
        let url = format!("{}/api/jobs/scheduled", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send list scheduled jobs request")?;

        self.handle_response(response).await
    }

    /// Get a job by ID
    ///
    /// # Arguments
    /// * `id` - The job UUID
    ///
    /// # Returns
    /// The job details
    pub async fn get_job(&self, id: Uuid) -> Result<Job> {
        let url = format!("{}/api/jobs/{}", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send get job request")?;

        self.handle_response(response).await
    }

    /// Get logs for a job
    ///
    /// # Arguments
    /// * `id` - The job UUID
    ///
    /// # Returns
    /// A list of log entries for the job
    pub async fn get_job_logs(&self, id: Uuid) -> Result<Vec<LogEntry>> {
        let url = format!("{}/api/jobs/{}/logs", self.base_url, id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send get job logs request")?;

        self.handle_response(response).await
    }

    /// List all jobs for a specific pipeline
    ///
    /// # Arguments
    /// * `pipeline_id` - The pipeline UUID
    ///
    /// # Returns
    /// A list of job summaries for the pipeline
    pub async fn list_jobs_by_pipeline(&self, pipeline_id: Uuid) -> Result<Vec<JobSummary>> {
        let url = format!("{}/api/jobs/pipeline/{}", self.base_url, pipeline_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send list jobs by pipeline request")?;

        self.handle_response(response).await
    }

    /// List all registered runners
    ///
    /// # Returns
    /// A list of runner summaries
    pub async fn list_runners(&self) -> Result<Vec<RunnerSummary>> {
        let url = format!("{}/api/runners", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send list runners request")?;

        self.handle_response(response).await
    }

    /// Handle API response and deserialize JSON
    ///
    /// # Arguments
    /// * `response` - The HTTP response
    ///
    /// # Returns
    /// The deserialized response body
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, error_text);
        }

        response
            .json()
            .await
            .context("Failed to parse response JSON")
    }
}
