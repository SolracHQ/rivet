//! Data Transfer Objects for inter-service communication
//!
//! This module contains DTOs used for communication between Rivet services
//! (orchestrator, runner, etc.). DTOs are lightweight representations of
//! domain entities optimized for network transfer.

pub mod job;
pub mod log;
pub mod module;
pub mod pipeline;
pub mod runner;
