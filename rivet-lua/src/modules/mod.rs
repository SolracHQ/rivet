//! Core Rivet modules for Lua scripts
//!
//! This module contains trait-based abstractions for Rivet core modules.
//! Each module can be implemented differently depending on the context:
//! - Runner: Real implementations with I/O and system access
//! - CLI: Stub/no-op implementations for parsing and validation
//! - Orchestrator: Validation-only implementations

pub mod env;
pub mod log;

pub use env::{EnvModule, VarProvider};
pub use log::{LogModule, LogSink};
