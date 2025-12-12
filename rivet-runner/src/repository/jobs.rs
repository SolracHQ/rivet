//! Jobs repository
//!
//! Handles communication with the orchestrator for job-related operations:
//! - Fetching scheduled jobs
//! - Claiming jobs
//! - Updating job status
//! - Completing jobs with results

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use rivet_core::domain::job::{Job, JobResult, JobStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;
use uuid::Uuid;

/// Repository trait for job-related operations with the orchestrator
#[async_trait]
pub trait JobRepository: Send + Sync {
    /// Fetches scheduled jobs from the orchestrator
    ///
    /// Returns a list of jobs that are ready to be executed by this runner.
    /// The orchestrator performs capability matching to ensure only compatible
    /// jobs are returned.
    async fn fetch_scheduled_jobs(&self) -> Result<Vec<Job>>;

    /// Claims a job for execution
    ///
    /// Tells the orchestrator that this runner is starting to execute the job.
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job to claim
    async fn claim_job(&self, job_id: Uuid) -> Result<JobExecutionInfo>;

    /// Updates the job status
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job to update
    /// * `status` - The new status
    async fn update_job_status(&self, job_id: Uuid, status: JobStatus) -> Result<()>;

    /// Completes a job with the execution result
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job that completed
    /// * `result` - The execution result (success/failure)
    async fn complete_job(&self, job_id: Uuid, result: JobResult) -> Result<()>;
}

/// HTTP implementation of JobRepository
pub struct HttpJobRepository {
    client: Client,
    orchestrator_url: String,
    runner_id: String,
}

impl HttpJobRepository {
    /// Creates a new HTTP job repository
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
impl JobRepository for HttpJobRepository {
    async fn fetch_scheduled_jobs(&self) -> Result<Vec<Job>> {
        let url = format!("{}/api/jobs/scheduled", self.orchestrator_url);

        let response = self
            .client
            .get(&url)
            .query(&[("runner_id", &self.runner_id)])
            .send()
            .await
            .context("Failed to send request to orchestrator")?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Orchestrator returned error status {}: {}", status, body);
        }

        // Try to get the response body first for better error reporting
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        // Parse the JSON
        let jobs: Vec<Job> = serde_json::from_str(&body).map_err(|e| {
            error!(
                "Failed to parse job list. Status: {}, Body: {}",
                status, body
            );
            anyhow::anyhow!(
                "Failed to parse job list from orchestrator: {}. Response body: {}",
                e,
                body
            )
        })?;

        Ok(jobs)
    }

    async fn claim_job(&self, job_id: Uuid) -> Result<JobExecutionInfo> {
        let url = format!("{}/api/jobs/execute/{}", self.orchestrator_url, job_id);

        let response = self
            .client
            .post(&url)
            .json(&ExecuteJobRequest {
                runner_id: self.runner_id.clone(),
            })
            .send()
            .await
            .context("Failed to execute job")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to claim job: {} - {}", status, body);
        }

        let info = response
            .json::<JobExecutionInfo>()
            .await
            .context("Failed to parse job execution info")?;

        Ok(info)
    }

    async fn update_job_status(&self, job_id: Uuid, status: JobStatus) -> Result<()> {
        let url = format!("{}/api/jobs/{}/status", self.orchestrator_url, job_id);

        let response = self
            .client
            .put(&url)
            .json(&UpdateStatusRequest { status })
            .send()
            .await
            .context("Failed to update job status")?;

        if !response.status().is_success() {
            let status_code = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update job status: {} - {}", status_code, body);
        }

        Ok(())
    }

    async fn complete_job(&self, job_id: Uuid, result: JobResult) -> Result<()> {
        let url = format!("{}/api/jobs/{}/complete", self.orchestrator_url, job_id);

        // Determine status from result
        let status = if result.success {
            JobStatus::Succeeded
        } else {
            JobStatus::Failed
        };

        let response = self
            .client
            .post(&url)
            .json(&CompleteJobRequest {
                status,
                result: Some(result),
            })
            .send()
            .await
            .context("Failed to complete job")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to complete job: {} - {}", status, body);
        }

        Ok(())
    }
}

/// Information needed to execute a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecutionInfo {
    /// The job ID
    pub job_id: Uuid,
    /// The pipeline ID
    pub pipeline_id: Uuid,
    /// The pipeline Lua source code
    pub pipeline_source: String,
    /// Job parameters to inject as environment variables
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ExecuteJobRequest {
    runner_id: String,
}

#[derive(Debug, Serialize)]
struct UpdateStatusRequest {
    status: JobStatus,
}

#[derive(Debug, Serialize)]
struct CompleteJobRequest {
    status: JobStatus,
    result: Option<JobResult>,
}
