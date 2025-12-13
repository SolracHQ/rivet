//! Rivet Runner
//!
//! A stateless worker that executes pipeline jobs in sandboxed Lua environments.
//!
//! Architecture:
//! - Configuration: Load settings from environment or defaults
//! - Repositories: HTTP communication with orchestrator (jobs, logs, runners)
//! - Services: Business logic (execution, capabilities, log buffering)
//! - Scheduler: Job polling and lifecycle management
//!
//! The runner polls the orchestrator for scheduled jobs, executes them in
//! secure Lua sandboxes, and streams logs back periodically.

mod config;
mod lua;
mod scheduler;
mod service;

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::scheduler::JobPoller;
use crate::service::{
    CapabilitiesService, ExecutionService, StandardCapabilitiesService, StandardExecutionService,
};
use rivet_client::OrchestratorClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rivet_runner=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Rivet Runner");

    // Load configuration
    let config = load_config()?;
    info!(
        "Loaded configuration: runner_id={}, orchestrator_url={}",
        config.runner_id, config.orchestrator_url
    );

    // Initialize orchestrator client
    let client = Arc::new(OrchestratorClient::new(config.orchestrator_url.clone()));

    info!("Orchestrator client initialized");

    // Initialize services
    let capabilities_service = StandardCapabilitiesService::new(config.runner_id.clone());
    let capabilities = capabilities_service
        .discover()
        .context("Failed to discover capabilities")?;

    info!("Discovered {} capabilities", capabilities.len());
    for cap in &capabilities {
        info!("  - {}", cap);
    }

    // Register capabilities with orchestrator (with retry logic)
    info!("Registering capabilities with orchestrator");
    register_with_retry(&client, &config.runner_id, capabilities).await?;
    info!("Capabilities registered successfully");

    let execution_service: Arc<dyn ExecutionService> = Arc::new(StandardExecutionService::new());

    info!("Services initialized");

    // Create job poller
    let poller = JobPoller::new(config.clone(), client, execution_service);

    info!("Runner initialized successfully");
    info!(
        "Poll interval: {:?}, Log send interval: {:?}",
        config.poll_interval, config.log_send_interval
    );

    // Start polling loop
    info!("Starting job polling loop");
    if let Err(e) = poller.run().await {
        error!("Poller error: {}", e);
        return Err(e);
    }

    Ok(())
}

/// Loads configuration from environment variables with fallback to defaults
fn load_config() -> Result<Config> {
    match Config::from_env() {
        Ok(config) => {
            config.validate()?;
            Ok(config)
        }
        Err(_) => {
            info!("Failed to load config from environment, using defaults");
            let config = Config::default();
            config.validate()?;
            Ok(config)
        }
    }
}

/// Register with orchestrator with retry logic and exponential backoff
///
/// This handles the case where the orchestrator may not be ready yet when
/// the runner starts (common in container environments).
async fn register_with_retry(
    client: &Arc<OrchestratorClient>,
    runner_id: &str,
    capabilities: Vec<String>,
) -> Result<()> {
    const MAX_RETRIES: u32 = 10;
    const INITIAL_DELAY_MS: u64 = 500;
    const MAX_DELAY_MS: u64 = 30_000;

    let mut attempt = 0;
    let mut delay_ms = INITIAL_DELAY_MS;

    loop {
        attempt += 1;

        match client
            .register_runner(runner_id, capabilities.clone())
            .await
        {
            Ok(_) => {
                if attempt > 1 {
                    info!(
                        "Successfully registered with orchestrator after {} attempt(s)",
                        attempt
                    );
                }
                return Ok(());
            }
            Err(e) => {
                if attempt >= MAX_RETRIES {
                    error!(
                        "Failed to register with orchestrator after {} attempts",
                        MAX_RETRIES
                    );
                    return Err(anyhow::anyhow!(
                        "Failed to register capabilities with orchestrator: {}",
                        e
                    ));
                }

                warn!(
                    "Failed to register with orchestrator (attempt {}/{}): {}",
                    attempt, MAX_RETRIES, e
                );
                warn!("Retrying in {} ms...", delay_ms);

                tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                // Exponential backoff with cap
                delay_ms = (delay_ms * 2).min(MAX_DELAY_MS);
            }
        }
    }
}
