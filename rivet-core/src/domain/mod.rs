//! Core domain types
//!
//! This module contains the core domain structures used across Rivet services.
//! These types represent the fundamental business entities and are shared between
//! orchestrator (for persistence) and runner (for execution).

pub mod job;
pub mod log;
pub mod pipeline;
pub mod runner;
