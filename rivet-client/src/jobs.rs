//! Job-related API endpoints

use crate::OrchestratorClient;
use crate::error::Result;
use rivet_core::domain::job::{Job, JobResult, JobStatus};
use rivet_core::domain::log::LogEntry;
use rivet_core::dto::job::{
    CompleteJobRequest, CreateJob, ExecuteJobRequest, JobExecutionInfo, UpdateStatusRequest,
};
use uuid::Uuid;

impl OrchestratorClient {
    // =============================================================================
    // Job Lifecycle
    // =============================================================================

    /// Launch a new job from a pipeline
    ///
    /// # Arguments
    /// * `req` - The job creation request
    ///
    /// # Returns
    /// The created job
    ///
    /// # Example
    /// ```no_run
    /// # use rivet_client::OrchestratorClient;
    /// # use rivet_core::dto::job::CreateJob;
    /// # use uuid::Uuid;
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = OrchestratorClient::new("http://localhost:8080");
    /// let job = client.launch_job(CreateJob {
    ///     pipeline_id: Uuid::new_v4(),
    ///     parameters: Default::default(),
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn launch_job(&self, req: CreateJob) -> Result<Job> {
        let url = format!("{}/api/pipeline/launch", self.base_url);
        let response = self.client.post(&url).json(&req).send().await?;

        self.handle_response(response).await
    }

    /// Get a job by ID
    ///
    /// # Arguments
    /// * `job_id` - The job UUID
    ///
    /// # Returns
    /// The job details
    pub async fn get_job(&self, job_id: Uuid) -> Result<Job> {
        let url = format!("{}/api/jobs/{}", self.base_url, job_id);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// List all jobs
    ///
    /// # Returns
    /// A list of all jobs
    pub async fn list_all_jobs(&self) -> Result<Vec<Job>> {
        let url = format!("{}/api/jobs", self.base_url);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// List all scheduled (queued) jobs
    ///
    /// # Returns
    /// A list of scheduled jobs
    pub async fn list_scheduled_jobs(&self) -> Result<Vec<Job>> {
        let url = format!("{}/api/jobs/scheduled", self.base_url);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// List all jobs for a specific pipeline
    ///
    /// # Arguments
    /// * `pipeline_id` - The pipeline UUID
    ///
    /// # Returns
    /// A list of jobs for the pipeline
    pub async fn list_jobs_by_pipeline(&self, pipeline_id: Uuid) -> Result<Vec<Job>> {
        let url = format!("{}/api/jobs/pipeline/{}", self.base_url, pipeline_id);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    // =============================================================================
    // Job Execution (Runner-specific)
    // =============================================================================

    /// Claim a job for execution by a runner
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job to claim
    /// * `runner_id` - The ID of the runner claiming the job
    ///
    /// # Returns
    /// Information needed to execute the job
    pub async fn claim_job(&self, job_id: Uuid, runner_id: &str) -> Result<JobExecutionInfo> {
        let url = format!("{}/api/jobs/execute/{}", self.base_url, job_id);
        let response = self
            .client
            .post(&url)
            .json(&ExecuteJobRequest {
                runner_id: runner_id.to_string(),
            })
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Update the status of a job
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job to update
    /// * `status` - The new status
    pub async fn update_job_status(&self, job_id: Uuid, status: JobStatus) -> Result<()> {
        let url = format!("{}/api/jobs/{}/status", self.base_url, job_id);
        let response = self
            .client
            .put(&url)
            .json(&UpdateStatusRequest { status })
            .send()
            .await?;

        self.handle_empty_response(response).await
    }

    /// Complete a job with the execution result
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job that completed
    /// * `result` - The execution result (success/failure)
    pub async fn complete_job(&self, job_id: Uuid, result: JobResult) -> Result<()> {
        let url = format!("{}/api/jobs/{}/complete", self.base_url, job_id);

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
            .await?;

        self.handle_empty_response(response).await
    }

    // =============================================================================
    // Job Logs
    // =============================================================================

    /// Get logs for a job
    ///
    /// # Arguments
    /// * `job_id` - The job UUID
    ///
    /// # Returns
    /// A list of log entries for the job
    pub async fn get_job_logs(&self, job_id: Uuid) -> Result<Vec<LogEntry>> {
        let url = format!("{}/api/jobs/{}/logs", self.base_url, job_id);
        let response = self.client.get(&url).send().await?;

        self.handle_response(response).await
    }

    /// Send logs to the orchestrator for a specific job
    ///
    /// # Arguments
    /// * `job_id` - The ID of the job these logs belong to
    /// * `entries` - The log entries to send
    pub async fn send_logs(&self, job_id: Uuid, entries: Vec<LogEntry>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let url = format!("{}/api/jobs/{}/logs", self.base_url, job_id);
        let response = self.client.post(&url).json(&entries).send().await?;

        self.handle_empty_response(response).await
    }
}
