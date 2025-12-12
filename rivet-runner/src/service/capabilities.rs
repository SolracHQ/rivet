//! Capabilities service
//!
//! Discovers and validates runner capabilities including core modules,
//! plugins, and system tools. Used for capability-based job matching.

use anyhow::Result;
use std::collections::HashSet;
use tracing::info;

/// Service trait for capability discovery and validation
pub trait CapabilitiesService: Send + Sync {
    /// Discovers all capabilities available on this runner
    ///
    /// Returns a set of capability identifiers that can be reported
    /// to the orchestrator for job matching.
    fn discover(&self) -> Result<Vec<String>>;

    /// Checks if this runner has all required capabilities
    ///
    /// # Arguments
    /// * `requires` - List of required capability strings
    ///
    /// # Returns
    /// `true` if all required capabilities are available
    #[allow(dead_code)]
    fn check_compatibility(&self, requires: &[String]) -> bool;
}

/// Standard implementation of CapabilitiesService
pub struct StandardCapabilitiesService {}

impl StandardCapabilitiesService {
    /// Creates a new standard capabilities service
    ///
    /// # Arguments
    /// * `runner_id` - Unique identifier for this runner (currently unused but may be needed for logging)
    pub fn new(_runner_id: String) -> Self {
        Self {}
    }
}

impl CapabilitiesService for StandardCapabilitiesService {
    fn discover(&self) -> Result<Vec<String>> {
        info!("Discovering runner capabilities");

        let mut capabilities = HashSet::new();

        // Core modules that are always available
        capabilities.insert("log".to_string());
        capabilities.insert("env".to_string());

        // TODO: Detect additional capabilities
        // - Check for git binary -> "process.git"
        // - Check for docker -> "container.docker"
        // - Check for available Lua plugins
        // - etc.

        info!("Discovered {} capabilities", capabilities.len());

        Ok(capabilities.into_iter().collect())
    }

    fn check_compatibility(&self, _requires: &[String]) -> bool {
        // TODO: Implement capability checking
        // For now, assume all requirements are met
        true
    }
}
