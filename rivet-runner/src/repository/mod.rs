//! Repository layer
//!
//! Repositories are stateless HTTP clients that abstract communication
//! with the orchestrator. They provide simple, focused interfaces for
//! different API endpoints without any business logic.
//!
//! All repositories are trait-based to enable testing and mocking.

mod jobs;
mod logs;
mod runners;

// Re-export traits
pub use jobs::JobRepository;
pub use logs::LogRepository;
pub use runners::RunnerRepository;

// Re-export implementations
pub use jobs::HttpJobRepository;
pub use logs::HttpLogRepository;
pub use runners::HttpRunnerRepository;
