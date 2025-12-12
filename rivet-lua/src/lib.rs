//! Rivet Lua Infrastructure
//!
//! This crate provides shared Lua infrastructure for the Rivet CI/CD system.
//! It includes:
//! - Module trait and registry for Lua modules
//! - Two sandbox types: metadata evaluation and full execution
//! - Core module implementations
//! - Pipeline parsing and manifest extraction
//! - Stub generation for local development

pub mod module;
pub mod modules;
pub mod parser;
pub mod sandbox;

pub use module::{ModuleMetadata, ModuleRegistry, RivetModule};
pub use modules::{EnvModule, LogModule, LogSink, VarProvider};
pub use parser::parse_pipeline_metadata;
pub use sandbox::{create_execution_sandbox, create_metadata_sandbox};

pub use rivet_core::domain::pipeline::{InputDefinition, PipelineMetadata, StageMetadata};
