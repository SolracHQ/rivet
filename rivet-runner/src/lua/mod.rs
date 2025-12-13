//! Lua execution infrastructure for the Rivet runner
//!
//! This module provides:
//! - Module implementations (log, input, output, process, container)
//! - Sandbox creation with registered modules
//! - Job parameter and log buffer integration

pub mod executor;
pub mod modules;
