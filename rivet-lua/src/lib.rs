//! Rivet Lua Infrastructure
//!
//! This crate provides shared Lua infrastructure for the Rivet CI/CD system.
//! It includes:
//! - Two sandbox types: metadata evaluation and full execution
//! - Pipeline parsing and manifest extraction
//!
//! Module implementations live in rivet-runner where they have access to
//! runtime dependencies (container runtime, orchestrator connection, etc.).

pub mod definition;
pub mod sandbox;

pub use definition::{PipelineDefinition, StageDefinition, parse_pipeline_definition};
pub use sandbox::create_sandbox;
