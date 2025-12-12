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
            .context("Failed to fetch scheduled jobs")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch jobs: {} - {}", status, body);
        }

        let jobs = response
            .json::<Vec<Job>>()
            .await
            .context("Failed to parse job list")?;

        Ok(jobs)
    }

    async fn claim_job(&self, job_id: Uuid) -> Result<JobExecutionInfo> {
        let url = format!("{}/api/jobs/{}/claim", self.orchestrator_url, job_id);

        let response = self
            .client
            .post(&url)
            .json(&ClaimJobRequest {
                runner_id: self.runner_id.clone(),
            })
            .send()
            .await
            .context("Failed to claim job")?;

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

        let response = self
            .client
            .post(&url)
            .json(&CompleteJobRequest { result })
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
struct ClaimJobRequest {
    runner_id: String,
}

#[derive(Debug, Serialize)]
struct UpdateStatusRequest {
    status: JobStatus,
}

#[derive(Debug, Serialize)]
struct CompleteJobRequest {
    result: JobResult,
}
