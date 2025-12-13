//! Execution context for pipeline jobs
//!
//! Contains all state needed during pipeline execution:
//! - Log buffer for collecting logs
//! - Workspace path for job files
//! - Job input parameters
//! - Container stack for tracking current execution context
//! - Container manager for executing commands

use rivet_core::domain::log::{LogEntry, LogLevel};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::podman::ContainerManager;

/// Execution context shared across pipeline execution
pub struct Context {
    /// Log buffer with entries
    log_buffer: Mutex<Vec<LogEntry>>,

    /// Job input parameters
    pub inputs: HashMap<String, JsonValue>,

    /// Container manager for this job
    /// Manages multiple containers and tracks the execution stack
    pub container_manager: ContainerManager,
}

impl Context {
    /// Creates a new execution context
    ///
    /// # Arguments
    /// * `job_id` - The job ID
    /// * `workspace_base` - Base directory for workspaces (e.g., /tmp)
    /// * `inputs` - Job input parameters
    pub fn new(
        job_id: Uuid,
        workspace_base: PathBuf,
        inputs: HashMap<String, JsonValue>,
    ) -> Arc<Self> {
        let workspace = workspace_base.join(job_id.to_string());
        let workspace_str = workspace.to_string_lossy().to_string();

        let container_manager = ContainerManager::new(job_id, workspace_str);

        Arc::new(Self {
            log_buffer: Mutex::new(Vec::new()),
            inputs,
            container_manager,
        })
    }

    /// Adds a log entry to the buffer
    pub fn add_log(&self, entry: LogEntry) {
        let mut buffer = self.log_buffer.lock().unwrap();
        buffer.push(entry);
    }

    /// Logs a debug message
    pub fn log_debug(&self, message: String) {
        self.add_log(LogEntry {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Debug,
            message,
        });
    }

    /// Logs an info message
    pub fn log_info(&self, message: String) {
        self.add_log(LogEntry {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Info,
            message,
        });
    }

    /// Logs a warning message
    pub fn log_warning(&self, message: String) {
        self.add_log(LogEntry {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Warning,
            message,
        });
    }

    /// Logs an error message
    pub fn log_error(&self, message: String) {
        self.add_log(LogEntry {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Error,
            message,
        });
    }

    /// Drains all log entries from the buffer
    ///
    /// Returns all buffered entries and clears the buffer
    pub fn drain_logs(&self) -> Vec<LogEntry> {
        let mut buffer = self.log_buffer.lock().unwrap();
        buffer.drain(..).collect()
    }
}
