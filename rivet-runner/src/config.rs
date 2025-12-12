//! Runner configuration
//!
//! Defines all configurable parameters for the runner including
//! polling intervals, logging configuration, and orchestrator connection settings.

use std::time::Duration;

/// Runner configuration
///
/// All timeouts and intervals are configurable to allow tuning
/// for different deployment scenarios (dev vs prod, fast vs slow networks).
#[derive(Debug, Clone)]
pub struct Config {
    /// Unique identifier for this runner instance
    pub runner_id: String,

    /// Orchestrator base URL (e.g., "http://localhost:8080")
    pub orchestrator_url: String,

    /// How often to poll the orchestrator for new jobs
    pub poll_interval: Duration,

    /// How often to send buffered logs to the orchestrator
    pub log_send_interval: Duration,

    /// Maximum number of log entries to buffer before forcing a send
    #[allow(dead_code)]
    pub log_buffer_size: usize,

    /// Maximum time a job can run before timing out
    #[allow(dead_code)]
    pub job_timeout: Duration,

    /// Labels for capability matching (e.g., env=prod, region=us-west)
    #[allow(dead_code)]
    pub labels: std::collections::HashMap<String, String>,

    /// Max parallel jobs the runner can handle
    #[allow(dead_code)]
    pub max_parallel_jobs: usize,
}

impl Config {
    /// Creates a new configuration with defaults
    pub fn new(runner_id: String, orchestrator_url: String) -> Self {
        Self {
            runner_id,
            orchestrator_url,
            poll_interval: Duration::from_secs(5),
            log_send_interval: Duration::from_secs(30),
            log_buffer_size: 100,
            job_timeout: Duration::from_secs(300), // 5 minutes
            labels: std::collections::HashMap::new(),
            max_parallel_jobs: 2,
        }
    }

    /// Creates configuration from environment variables
    ///
    /// Expected environment variables:
    /// - RUNNER_ID (required)
    /// - ORCHESTRATOR_URL (required)
    /// - POLL_INTERVAL (optional, seconds, default: 5)
    /// - LOG_SEND_INTERVAL (optional, seconds, default: 30)
    /// - LOG_BUFFER_SIZE (optional, default: 100)
    /// - JOB_TIMEOUT (optional, seconds, default: 300)
    /// - MAX_PARALLEL_JOBS (optional, default: 2)
    pub fn from_env() -> anyhow::Result<Self> {
        let runner_id = std::env::var("RUNNER_ID")
            .map_err(|_| anyhow::anyhow!("RUNNER_ID environment variable not set"))?;

        let orchestrator_url = std::env::var("ORCHESTRATOR_URL")
            .map_err(|_| anyhow::anyhow!("ORCHESTRATOR_URL environment variable not set"))?;

        let poll_interval = std::env::var("POLL_INTERVAL")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(5));

        let log_send_interval = std::env::var("LOG_SEND_INTERVAL")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30));

        let log_buffer_size = std::env::var("LOG_BUFFER_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(100);

        let job_timeout = std::env::var("JOB_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(300));

        let max_parallel_jobs = std::env::var("MAX_PARALLEL_JOBS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(2);

        Ok(Self {
            runner_id,
            orchestrator_url,
            poll_interval,
            log_send_interval,
            log_buffer_size,
            job_timeout,
            labels: std::collections::HashMap::new(),
            max_parallel_jobs,
        })
    }

    /// Adds a label for capability matching
    #[allow(dead_code)]
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    /// Validates the configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.runner_id.is_empty() {
            anyhow::bail!("runner_id cannot be empty");
        }

        if self.orchestrator_url.is_empty() {
            anyhow::bail!("orchestrator_url cannot be empty");
        }

        if !self.orchestrator_url.starts_with("http://")
            && !self.orchestrator_url.starts_with("https://")
        {
            anyhow::bail!("orchestrator_url must start with http:// or https://");
        }

        if self.poll_interval.as_secs() == 0 {
            anyhow::bail!("poll_interval must be greater than 0");
        }

        if self.log_send_interval.as_secs() == 0 {
            anyhow::bail!("log_send_interval must be greater than 0");
        }

        if self.log_buffer_size == 0 {
            anyhow::bail!("log_buffer_size must be greater than 0");
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string(),
            "http://localhost:8080".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.poll_interval, Duration::from_secs(5));
        assert_eq!(config.log_send_interval, Duration::from_secs(30));
        assert_eq!(config.log_buffer_size, 100);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Empty runner_id should fail
        config.runner_id = String::new();
        assert!(config.validate().is_err());

        config.runner_id = "test".to_string();

        // Invalid URL should fail
        config.orchestrator_url = "not-a-url".to_string();
        assert!(config.validate().is_err());

        config.orchestrator_url = "http://localhost:8080".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_with_label() {
        let config = Config::default()
            .with_label("env".to_string(), "prod".to_string())
            .with_label("region".to_string(), "us-west".to_string());

        assert_eq!(config.labels.get("env"), Some(&"prod".to_string()));
        assert_eq!(config.labels.get("region"), Some(&"us-west".to_string()));
    }
}
