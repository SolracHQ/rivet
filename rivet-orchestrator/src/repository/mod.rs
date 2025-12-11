//! Repository Module
//!
//! Data access layer for the orchestrator.
//! Each repository handles database operations for a specific domain entity.

pub mod job;
pub mod log;
pub mod pipeline;

// Re-export for convenience
pub use job as job_repository;
pub use log as log_repository;
pub use pipeline as pipeline_repository;
