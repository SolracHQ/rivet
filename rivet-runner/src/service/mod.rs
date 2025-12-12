//! Service layer
//!
//! Services contain business logic for the runner. They orchestrate
//! operations using repositories and implement core functionality like
//! job execution, log buffering, and capability discovery.
//!
//! All services are trait-based to enable testing and dependency injection.

mod capabilities;
mod execution;
mod log_buffer;

// Re-export traits
pub use capabilities::CapabilitiesService;
pub use execution::ExecutionService;
pub use log_buffer::LogBufferService;

// Re-export implementations
pub use capabilities::StandardCapabilitiesService;
pub use execution::StandardExecutionService;
pub use log_buffer::InMemoryLogBuffer;
