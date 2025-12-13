//! Rivet Lua Infrastructure
//!
//! This crate provides shared Lua infrastructure for the Rivet CI/CD system.
//! It includes:
//! - Two sandbox types: metadata evaluation and full execution
//! - Pipeline parsing and manifest extraction
//!
//! Module implementations live in rivet-runner where they have access to
//! runtime dependencies (container runtime, orchestrator connection, etc.).

pub mod parser;
pub mod sandbox;

pub use parser::parse_pipeline_metadata;
pub use sandbox::{create_execution_sandbox, create_metadata_sandbox};

pub use rivet_core::domain::pipeline::{InputDefinition, PipelineMetadata, StageMetadata};
