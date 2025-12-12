//! Configuration module
//!
//! Handles CLI configuration including orchestrator URL and other settings.

/// CLI configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// URL of the orchestrator service
    pub orchestrator_url: String,
}
