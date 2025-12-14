//! Job poller
//!
//! Polls the orchestrator for scheduled jobs and executes them.
//! Each job runs in its own task with a context containing logs, workspace, and container stack.

use anyhow::{Context as AnyhowContext, Result};
use rivet_core::domain::job::JobResult;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::context::Context;
use crate::lua::executor::LuaExecutor;
use rivet_client::OrchestratorClient;

/// Job poller that continuously polls for and executes jobs
pub struct JobPoller {
    config: Config,
    client: Arc<OrchestratorClient>,
    semaphore: Arc<Semaphore>,
}

impl JobPoller {
    /// Creates a new job poller
    pub fn new(config: Config, client: Arc<OrchestratorClient>) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_parallel_jobs));
        Self {
            config,
            client,
            semaphore,
        }
    }

    /// Starts the polling loop
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting job poller (interval: {:?})",
            self.config.poll_interval
        );

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
                }
            }
        }
    }

    /// Performs a single poll cycle
    async fn poll_and_execute_once(&self) -> Result<usize> {
        let jobs = self
            .client
            .list_scheduled_jobs()
            .await
            .context("Failed to fetch scheduled jobs")?;

        if jobs.is_empty() {
            debug!("No jobs available");
            return Ok(0);
        }

        info!("Found {} job(s) to execute", jobs.len());

        let mut handles = Vec::new();

        for job in jobs {
            let job_id = job.id;

            // Try to acquire semaphore permit, skip if at max capacity
            if let Ok(permit) = self.semaphore.clone().try_acquire_owned() {
                let handle = self.spawn_job_task(job_id, permit);
                handles.push(handle);
            } else {
                debug!("Max parallel jobs reached, skipping job {} for now", job_id);
            }
        }

        let num_jobs = handles.len();

        for handle in handles {
            if let Err(e) = handle.await {
                warn!("Job task panicked: {}", e);
            }
        }

        Ok(num_jobs)
    }

    /// Spawns a task to execute a single job
    fn spawn_job_task(
        &self,
        job_id: Uuid,
        _permit: tokio::sync::OwnedSemaphorePermit,
    ) -> tokio::task::JoinHandle<()> {
        let client = Arc::clone(&self.client);
        let config = self.config.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::execute_job(job_id, config, client).await {
                error!("Failed to execute job {}: {:#}", job_id, e);
            }
            // Permit is automatically released when dropped
        })
    }

    /// Executes a single job with log streaming
    async fn execute_job(
        job_id: Uuid,
        config: Config,
        client: Arc<OrchestratorClient>,
    ) -> Result<()> {
        info!("Starting execution of job {}", job_id);

        // Claim the job
        let exec_info = client
            .claim_job(job_id, &config.runner_id)
            .await
            .context("Failed to claim job")?;

        info!(
            "Claimed job {} (pipeline {})",
            exec_info.job_id, exec_info.pipeline_id
        );

        // Create execution context
        let context = Context::new(job_id, config.workspace_base.clone(), exec_info.parameters);

        // Start the default container
        context.log_info("Starting default container...".to_string());
        if let Err(e) = context
            .container_manager
            .start_default(&config.default_container_image)
        {
            error!("Failed to start default container: {:#}", e);
            context.log_error(format!("Failed to start default container: {}", e));
            let result = JobResult::failed(format!("Failed to start default container: {}", e));
            let _ = client.complete_job(job_id, result).await;
            return Err(e);
        }
        context.log_info("Default container started successfully".to_string());

        // Spawn log sender task
        let log_sender = Self::spawn_log_sender(
            job_id,
            Arc::clone(&context),
            Arc::clone(&client),
            config.log_send_interval,
        );

        // Create executor and execute pipeline
        let executor = LuaExecutor::new(Arc::clone(&context));
        let result = executor
            .execute_pipeline(job_id, &exec_info.pipeline_source)
            .await;

        // Always abort log sender
        log_sender.abort();

        // Send remaining logs
        let remaining_logs = context.drain_logs();
        if !remaining_logs.is_empty() {
            info!(
                "Sending {} remaining logs for job {}",
                remaining_logs.len(),
                job_id
            );
            if let Err(e) = client.send_logs(job_id, remaining_logs).await {
                warn!("Failed to send final logs: {:#}", e);
            }
        }

        info!(
            "Job {} completed with status: {}",
            job_id,
            if result.success { "success" } else { "failure" }
        );

        // Cleanup container
        context.log_info("Cleaning up container...".to_string());
        if let Err(e) = context.container_manager.cleanup() {
            warn!("Failed to cleanup container: {:#}", e);
            context.log_warning(format!("Failed to cleanup container: {}", e));
        } else {
            context.log_info("Container cleaned up successfully".to_string());
        }

        // Report completion
        client
            .complete_job(job_id, result)
            .await
            .context("Failed to complete job")?;

        Ok(())
    }

    /// Spawns a background task to send logs periodically
    fn spawn_log_sender(
        job_id: Uuid,
        context: Arc<Context>,
        client: Arc<OrchestratorClient>,
        interval: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = time::interval(interval);

            loop {
                ticker.tick().await;

                let logs = context.drain_logs();

                if logs.is_empty() {
                    debug!("No logs to send for job {}", job_id);
                    continue;
                }

                debug!("Sending {} logs for job {}", logs.len(), job_id);

                if let Err(e) = client.send_logs(job_id, logs).await {
                    error!("Failed to send logs for job {}: {:#}", job_id, e);
                }
            }
        })
    }

    /// Starts a background task to send heartbeats
    fn start_heartbeat_loop(&self) -> tokio::task::JoinHandle<()> {
        let client = Arc::clone(&self.client);
        let runner_id = self.config.runner_id.clone();
        let heartbeat_interval = Duration::from_secs(30);

        tokio::spawn(async move {
            let mut ticker = time::interval(heartbeat_interval);

            loop {
                ticker.tick().await;

                debug!("Sending heartbeat");

                if let Err(e) = client.send_heartbeat(&runner_id).await {
                    warn!("Failed to send heartbeat: {:#}", e);
                }
            }
        })
    }
}
