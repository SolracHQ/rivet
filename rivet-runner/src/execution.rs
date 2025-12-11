//! Execution types for the Rivet runner
//!
//! These types only exist at runtime during job execution.
//! They are not persisted or sent over the network.

use rivet_core::types::{JobResult, LogEntry};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Execution context for a Lua script
///
/// Created at runtime for each job execution.
/// Contains the Lua VM, loaded modules, and execution state.
pub struct ExecutionContext {
    /// The Lua sandbox instance
    pub lua: mlua::Lua,
    /// Job being executed
    pub job_id: Uuid,
    /// Pipeline being executed
    pub pipeline_id: Uuid,
    /// Job parameters available to the script
    pub parameters: HashMap<String, serde_json::Value>,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
    /// Collected logs (shared with LogModule)
    pub log_buffer: Arc<Mutex<Vec<LogEntry>>>,
}

impl ExecutionContext {
    /// Drains and returns all collected logs from the buffer
    ///
    /// Called periodically during execution to send logs to orchestrator.
    pub fn drain_logs(&self) -> Result<Vec<LogEntry>, String> {
        self.log_buffer
            .lock()
            .map_err(|e| format!("Failed to lock log buffer: {}", e))
            .map(|mut buffer| buffer.drain(..).collect())
    }

    /// Gets a snapshot of current logs without draining
    pub fn peek_logs(&self) -> Result<Vec<LogEntry>, String> {
        self.log_buffer
            .lock()
            .map_err(|e| format!("Failed to lock log buffer: {}", e))
            .map(|buffer| buffer.clone())
    }
}

/// Metadata about the execution environment
#[derive(Debug, Clone)]
pub struct ExecutionMetadata {
    pub runner_id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Loaded module IDs
    pub loaded_modules: Vec<String>,
}

/// Result of a pipeline execution
///
/// Returned by the runner after executing a job.
#[derive(Debug)]
pub enum ExecutionResult {
    Success {
        output: Option<serde_json::Value>,
        logs: Vec<LogEntry>,
    },
    Failure {
        error: String,
        logs: Vec<LogEntry>,
    },
    Timeout {
        logs: Vec<LogEntry>,
    },
}

impl ExecutionResult {
    /// Convert execution result to job result for persistence
    pub fn into_job_result(self) -> JobResult {
        match self {
            ExecutionResult::Success { output, .. } => JobResult {
                success: true,
                exit_code: 0,
                output,
                error_message: None,
            },
            ExecutionResult::Failure { error, .. } => JobResult {
                success: false,
                exit_code: 1,
                output: None,
                error_message: Some(error),
            },
            ExecutionResult::Timeout { .. } => JobResult {
                success: false,
                exit_code: 124, // Standard timeout exit code
                output: None,
                error_message: Some("Execution timed out".to_string()),
            },
        }
    }

    /// Get logs from the result
    pub fn logs(&self) -> &[LogEntry] {
        match self {
            ExecutionResult::Success { logs, .. } => logs,
            ExecutionResult::Failure { logs, .. } => logs,
            ExecutionResult::Timeout { logs } => logs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::types::LogLevel;

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::Success {
            output: Some(serde_json::json!({"key": "value"})),
            logs: vec![],
        };

        let job_result = result.into_job_result();
        assert!(job_result.success);
        assert_eq!(job_result.exit_code, 0);
        assert_eq!(job_result.output, Some(serde_json::json!({"key": "value"})));
    }

    #[test]
    fn test_execution_result_failure() {
        let result = ExecutionResult::Failure {
            error: "Something went wrong".to_string(),
            logs: vec![],
        };

        let job_result = result.into_job_result();
        assert!(!job_result.success);
        assert_eq!(job_result.exit_code, 1);
        assert_eq!(
            job_result.error_message,
            Some("Something went wrong".to_string())
        );
    }

    #[test]
    fn test_execution_result_timeout() {
        let result = ExecutionResult::Timeout { logs: vec![] };

        let job_result = result.into_job_result();
        assert!(!job_result.success);
        assert_eq!(job_result.exit_code, 124);
        assert!(job_result.error_message.unwrap().contains("timed out"));
    }

    #[test]
    fn test_execution_context_drain_logs() {
        let log_buffer = Arc::new(Mutex::new(vec![
            LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Info,
                message: "test1".to_string(),
            },
            LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Error,
                message: "test2".to_string(),
            },
        ]));

        let ctx = ExecutionContext {
            lua: mlua::Lua::new(),
            job_id: Uuid::new_v4(),
            pipeline_id: Uuid::new_v4(),
            parameters: HashMap::new(),
            metadata: ExecutionMetadata {
                runner_id: "test-runner".to_string(),
                started_at: chrono::Utc::now(),
                loaded_modules: vec![],
            },
            log_buffer: log_buffer.clone(),
        };

        let drained = ctx.drain_logs().unwrap();
        assert_eq!(drained.len(), 2);

        // Buffer should be empty after drain
        let remaining = log_buffer.lock().unwrap();
        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn test_execution_context_peek_logs() {
        let log_buffer = Arc::new(Mutex::new(vec![LogEntry {
            timestamp: chrono::Utc::now(),
            level: LogLevel::Info,
            message: "test".to_string(),
        }]));

        let ctx = ExecutionContext {
            lua: mlua::Lua::new(),
            job_id: Uuid::new_v4(),
            pipeline_id: Uuid::new_v4(),
            parameters: HashMap::new(),
            metadata: ExecutionMetadata {
                runner_id: "test-runner".to_string(),
                started_at: chrono::Utc::now(),
                loaded_modules: vec![],
            },
            log_buffer: log_buffer.clone(),
        };

        let peeked = ctx.peek_logs().unwrap();
        assert_eq!(peeked.len(), 1);

        // Buffer should still have the log
        let remaining = log_buffer.lock().unwrap();
        assert_eq!(remaining.len(), 1);
    }
}
