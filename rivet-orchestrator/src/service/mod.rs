//! Service Module
//!
//! Business logic layer for the orchestrator.
//! Services orchestrate between repositories and contain domain logic.

pub mod job;
pub mod log;
pub mod pipeline;

// Re-export for convenience
pub use job as job_service;
pub use log as log_service;
pub use pipeline as pipeline_service;
