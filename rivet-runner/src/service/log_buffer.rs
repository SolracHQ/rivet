//! Log buffer service
//!
//! Manages in-memory log collection for job execution.
//! This service provides thread-safe access to a log buffer that can be
//! written to during job execution and periodically drained to send to the orchestrator.

use rivet_core::domain::log::LogEntry;
use std::sync::{Arc, Mutex};

/// Service for managing log buffers
///
/// This service wraps a thread-safe buffer for collecting log entries
/// during job execution. It provides methods to add entries and drain
/// the buffer for sending to the orchestrator.
pub trait LogBufferService: Send + Sync {
    /// Adds a log entry to the buffer
    ///
    /// # Arguments
    /// * `entry` - The log entry to add
    fn add_entry(&self, entry: LogEntry);

    /// Drains all log entries from the buffer
    ///
    /// This returns all buffered entries and clears the buffer.
    ///
    /// # Returns
    /// A vector of all log entries that were in the buffer
    fn drain(&self) -> Vec<LogEntry>;
}

/// In-memory implementation of LogBufferService
///
/// Uses Arc<Mutex<Vec<LogEntry>>> for thread-safe access across tasks.
#[derive(Clone)]
pub struct InMemoryLogBuffer {
    buffer: Arc<Mutex<Vec<LogEntry>>>,
}

impl InMemoryLogBuffer {
    /// Creates a new in-memory log buffer
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for InMemoryLogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBufferService for InMemoryLogBuffer {
    fn add_entry(&self, entry: LogEntry) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push(entry);
    }

    fn drain(&self) -> Vec<LogEntry> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.drain(..).collect()
    }
}
