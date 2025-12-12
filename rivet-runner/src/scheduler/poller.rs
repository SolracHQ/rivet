//! Job poller
//!
//! Polls the orchestrator for scheduled jobs and executes them using the
//! execution service. Manages concurrent job execution and periodic log sending.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::repository::{JobRepository, LogRepository, RunnerRepository};
use crate::service::{ExecutionService, InMemoryLogBuffer, LogBufferService};
use rivet_lua::parse_pipeline_metadata;

/// Job poller that continuously polls for and executes jobs
pub struct JobPoller {
    config: Config,
    job_repo: Arc<dyn JobRepository>,
    runner_repo: Arc<dyn RunnerRepository>,
    log_repo: Arc<dyn LogRepository>,
    execution_service: Arc<dyn ExecutionService>,
}

impl JobPoller {
    /// Creates a new job poller
    ///
    /// # Arguments
    /// * `config` - Runner configuration
    /// * `job_repo` - Repository for job operations
    /// * `runner_repo` - Repository for runner operations
    /// * `log_repo` - Repository for log operations
    /// * `execution_service` - Service for executing jobs
    pub fn new(
        config: Config,
        job_repo: Arc<dyn JobRepository>,
        runner_repo: Arc<dyn RunnerRepository>,
        log_repo: Arc<dyn LogRepository>,
        execution_service: Arc<dyn ExecutionService>,
    ) -> Self {
        Self {
            config,
            job_repo,
            runner_repo,
            log_repo,
            execution_service,
        }
    }

    /// Starts the polling loop
    ///
    /// This method runs indefinitely, polling the orchestrator at the
    /// configured interval for new jobs to execute.
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting job poller (interval: {:?})",
            self.config.poll_interval
        );

        // Start heartbeat task
        let _heartbeat_handle = self.start_heartbeat_loop();

        let mut interval = time::interval(self.config.poll_interval);

        loop {
            interval.tick().await;

            debug!("Polling for scheduled jobs");

            match self.poll_and_execute_once().await {
                Ok(executed) => {
                    if executed > 0 {
                        info!("Executed {} job(s) this cycle", executed);
                    }
                }
                Err(e) => {
                    error!("Error during poll cycle: {:#}", e);
                    // Continue polling even if one cycle fails
                }
            }
        }
    }

    /// Performs a single poll cycle
    ///
    /// Fetches scheduled jobs, claims them, and executes them concurrently.
    /// Returns the number of jobs executed.
    async fn poll_and_execute_once(&self) -> Result<usize> {
        // Fetch jobs from orchestrator
        let jobs = self
            .job_repo
            .fetch_scheduled_jobs()
            .await
            .context("Failed to fetch scheduled jobs")?;

        if jobs.is_empty() {
            debug!("No jobs available");
            return Ok(0);
        }

        info!("Found {} job(s) to execute", jobs.len());

        // Spawn a task for each job
        let mut handles = Vec::new();

        for job in jobs {
            let job_id = job.id;
            let handle = self.spawn_job_task(job_id);
            handles.push(handle);
        }

        let num_jobs = handles.len();

        // Wait for all jobs to complete
        for handle in handles {
            if let Err(e) = handle.await {
                warn!("Job task panicked: {}", e);
            }
        }

        Ok(num_jobs)
    }

    /// Spawns a tokio task to execute a single job
    fn spawn_job_task(&self, job_id: Uuid) -> tokio::task::JoinHandle<()> {
        let job_repo = Arc::clone(&self.job_repo);
        let log_repo = Arc::clone(&self.log_repo);
        let execution_service = Arc::clone(&self.execution_service);
        let log_send_interval = self.config.log_send_interval;

        tokio::spawn(async move {
            if let Err(e) = Self::execute_job_with_logging(
                job_id,
                job_repo,
                log_repo,
                execution_service,
                log_send_interval,
            )
            .await
            {
                error!("Failed to execute job {}: {:#}", job_id, e);
            }
        })
    }

    /// Executes a single job with periodic log sending
    async fn execute_job_with_logging(
        job_id: Uuid,
        job_repo: Arc<dyn JobRepository>,
        log_repo: Arc<dyn LogRepository>,
        execution_service: Arc<dyn ExecutionService>,
        log_send_interval: Duration,
    ) -> Result<()> {
        info!("Starting execution of job {}", job_id);

        // Claim the job
        let exec_info = job_repo
            .claim_job(job_id)
            .await
            .context("Failed to claim job")?;

        info!(
            "Claimed job {} (pipeline {})",
            exec_info.job_id, exec_info.pipeline_id
        );

        // Update job status to running
        if let Err(e) = job_repo
            .update_job_status(job_id, rivet_core::domain::job::JobStatus::Running)
            .await
        {
            warn!("Failed to update job status to running: {:#}", e);
            // Continue anyway - execution is more important
        }

        // Parse pipeline metadata
        let metadata = match parse_pipeline_metadata(&exec_info.pipeline_source) {
            Ok(meta) => meta,
            Err(e) => {
                error!("Failed to parse pipeline metadata: {:#}", e);
                let result = rivet_core::domain::job::JobResult {
                    success: false,
                    exit_code: 1,
                    output: None,
                    error_message: Some(format!("Failed to parse pipeline: {:#}", e)),
                };
                let _ = job_repo.complete_job(job_id, result).await;
                return Err(e);
            }
        };

        info!(
            "Executing pipeline '{}' with {} stages",
            metadata.name,
            metadata.stages.len()
        );

        // Create log buffer for this job
        let log_buffer: Arc<dyn LogBufferService> = Arc::new(InMemoryLogBuffer::new());

        // Start periodic log sending task
        let log_sender = Self::start_log_sender(
            job_id,
            Arc::clone(&log_buffer),
            Arc::clone(&log_repo),
            log_send_interval,
        );

        // Execute the job
        let result = execution_service
            .execute_job(
                exec_info.job_id,
                metadata,
                &exec_info.pipeline_source,
                exec_info.parameters,
                Arc::clone(&log_buffer),
            )
            .await
            .context("Job execution failed")?;

        // Stop the periodic log sender
        log_sender.abort();

        // Send any remaining logs
        let remaining_logs = log_buffer.drain();
        if !remaining_logs.is_empty() {
            info!(
                "Sending {} remaining logs for job {}",
                remaining_logs.len(),
                job_id
            );
            if let Err(e) = log_repo.send_logs(job_id, remaining_logs).await {
                warn!("Failed to send final logs: {:#}", e);
                // Don't fail the job just because log sending failed
            }
        }

        info!(
            "Job {} completed with status: {}",
            job_id,
            if result.success { "success" } else { "failure" }
        );

        // Report completion to orchestrator
        job_repo
            .complete_job(job_id, result)
            .await
            .context("Failed to complete job")?;

        Ok(())
    }

    /// Starts a background task that periodically sends logs to the orchestrator
    fn start_log_sender(
        job_id: Uuid,
        log_buffer: Arc<dyn LogBufferService>,
        log_repo: Arc<dyn LogRepository>,
        interval: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = time::interval(interval);

            loop {
                ticker.tick().await;

                // Drain logs from buffer
                let logs = log_buffer.drain();

                if logs.is_empty() {
                    debug!("No logs to send for job {}", job_id);
                    continue;
                }

                debug!("Sending {} logs for job {}", logs.len(), job_id);

                if let Err(e) = log_repo.send_logs(job_id, logs).await {
                    error!("Failed to send logs for job {}: {:#}", job_id, e);
                    // Continue trying on next interval
                }
            }
        })
    }

    /// Starts a background task that sends periodic heartbeats to the orchestrator
    fn start_heartbeat_loop(&self) -> tokio::task::JoinHandle<()> {
        let runner_repo = Arc::clone(&self.runner_repo);
        let heartbeat_interval = Duration::from_secs(30);

        tokio::spawn(async move {
            let mut ticker = time::interval(heartbeat_interval);

            loop {
                ticker.tick().await;

                debug!("Sending heartbeat");

                if let Err(e) = runner_repo.send_heartbeat().await {
                    warn!("Failed to send heartbeat: {:#}", e);
                    // Continue trying on next interval
                }
            }
        })
    }
}
