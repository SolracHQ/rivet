//! Module implementations for the runner
//!
//! These modules provide Lua API bindings for pipeline scripts.
//! Each module is registered directly into the Lua sandbox by the runner.
//!
//! Unlike the old trait-based abstraction, these are concrete implementations
//! that live only in the runner where they have access to:
//! - Container runtime (podman/kubectl)
//! - Orchestrator connection (for logging)
//! - Job parameters and state

pub mod container;
pub mod input;
pub mod log;
pub mod process;

pub use container::register_container_module;
pub use input::register_input_module;
pub use log::register_log_module;
pub use process::register_process_module;
