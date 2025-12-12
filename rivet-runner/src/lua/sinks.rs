//! Concrete implementations of LogSink and VarProvider for the runner
//!
//! These implementations connect the Lua modules to the runner's
//! service layer, enabling logging through LogBufferService and
//! providing job parameters as environment variables.

use rivet_core::domain::log::{LogEntry, LogLevel};
use rivet_lua::{LogSink, VarProvider};
use std::collections::HashMap;
use std::sync::Arc;

use crate::service::LogBufferService;

/// Buffered log sink that writes to a LogBufferService
///
/// This sink connects the Lua log module to the runner's log buffer service,
/// allowing pipeline scripts to write logs that are collected and sent to
/// the orchestrator.
pub struct BufferedLogSink {
    buffer: Arc<dyn LogBufferService>,
}

impl BufferedLogSink {
    /// Creates a new buffered log sink
    ///
    /// # Arguments
    /// * `buffer` - The log buffer service to write to
    pub fn new(buffer: Arc<dyn LogBufferService>) -> Self {
        Self { buffer }
    }
}

impl LogSink for BufferedLogSink {
    fn write(&mut self, level: LogLevel, message: &str) {
        let entry = LogEntry {
            timestamp: chrono::Utc::now(),
            level,
            message: message.to_string(),
        };

        self.buffer.add_entry(entry);
    }
}

/// Variable provider that supplies job parameters as environment variables
///
/// When a job is executed, the orchestrator provides parameters that should
/// be accessible via the `env` module in Lua scripts. This provider makes
/// those parameters available.
pub struct JobVarProvider {
    vars: HashMap<String, String>,
}

impl JobVarProvider {
    /// Creates a new job variable provider
    ///
    /// # Arguments
    /// * `parameters` - Job parameters from the orchestrator
    pub fn new(parameters: HashMap<String, serde_json::Value>) -> Self {
        // Convert JSON values to strings for Lua consumption
        let vars = parameters
            .into_iter()
            .map(|(key, value)| {
                let value_str = match value {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => String::new(),
                    // For complex types, serialize to JSON string
                    other => serde_json::to_string(&other).unwrap_or_default(),
                };
                (key, value_str)
            })
            .collect();

        Self { vars }
    }
}

impl VarProvider for JobVarProvider {
    fn get(&self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }

    fn keys(&self) -> Vec<String> {
        self.vars.keys().cloned().collect()
    }
}
