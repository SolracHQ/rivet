//! Logging module for Rivet Lua scripts
//!
//! This module provides a trait-based abstraction for logging that allows
//! different components to provide their own implementations:
//! - Runner: Buffered logging to orchestrator
//! - CLI: Stdout logging or no-op for parsing
//! - Orchestrator: Validation-only logging

use crate::module::RivetModule;
use mlua::prelude::*;
use rivet_core::domain::log::LogLevel;

/// Trait for log sinks
///
/// Implement this trait to provide custom logging behavior.
/// The LogModule is generic over this trait, allowing different
/// components to provide their own implementations.
///
/// # Thread Safety
/// Implementations must be Send to work with Lua's threading model.
pub trait LogSink: Send + Sync {
    /// Write a log message
    ///
    /// # Arguments
    /// * `level` - The log level (Debug, Info, Warning, Error)
    /// * `message` - The log message content
    fn write(&mut self, level: LogLevel, message: &str);
}

/// Logging module for Rivet Lua scripts
///
/// Generic over LogSink trait to allow different implementations
/// depending on the execution context.
pub struct LogModule<S: LogSink> {
    sink: std::sync::Arc<std::sync::Mutex<S>>,
}

impl<S: LogSink> LogModule<S> {
    /// Creates a new LogModule with the provided sink
    ///
    /// # Arguments
    /// * `sink` - Implementation of LogSink trait
    pub fn new(sink: S) -> Self {
        Self {
            sink: std::sync::Arc::new(std::sync::Mutex::new(sink)),
        }
    }
}

impl<S: LogSink + 'static> RivetModule for LogModule<S> {
    fn id(&self) -> &'static str {
        "log"
    }

    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let log_table = lua.create_table()?;

        // Debug level logging
        {
            let sink = self.sink.clone();
            log_table.set(
                "debug",
                lua.create_function(move |_, msg: String| {
                    sink.lock()
                        .map_err(|e| LuaError::RuntimeError(format!("Failed to lock sink: {}", e)))?
                        .write(LogLevel::Debug, &msg);
                    Ok(())
                })?,
            )?;
        }

        // Info level logging
        {
            let sink = self.sink.clone();
            log_table.set(
                "info",
                lua.create_function(move |_, msg: String| {
                    sink.lock()
                        .map_err(|e| LuaError::RuntimeError(format!("Failed to lock sink: {}", e)))?
                        .write(LogLevel::Info, &msg);
                    Ok(())
                })?,
            )?;
        }

        // Warning level logging
        {
            let sink = self.sink.clone();
            log_table.set(
                "warning",
                lua.create_function(move |_, msg: String| {
                    sink.lock()
                        .map_err(|e| LuaError::RuntimeError(format!("Failed to lock sink: {}", e)))?
                        .write(LogLevel::Warning, &msg);
                    Ok(())
                })?,
            )?;
        }

        // Error level logging
        {
            let sink = self.sink.clone();
            log_table.set(
                "error",
                lua.create_function(move |_, msg: String| {
                    sink.lock()
                        .map_err(|e| LuaError::RuntimeError(format!("Failed to lock sink: {}", e)))?
                        .write(LogLevel::Error, &msg);
                    Ok(())
                })?,
            )?;
        }

        // Set the log table as a global
        lua.globals().set(self.id(), log_table)?;
        Ok(())
    }

    fn stubs(&self) -> String {
        r#"---@meta

---Logging module for Rivet pipelines
---@class log
log = {}

---Log a debug message
---@param msg string The message to log
function log.debug(msg) end

---Log an info message
---@param msg string The message to log
function log.info(msg) end

---Log a warning message
---@param msg string The message to log
function log.warning(msg) end

---Log an error message
---@param msg string The message to log
function log.error(msg) end
"#
        .to_string()
    }

    fn metadata(&self) -> crate::module::ModuleMetadata {
        crate::module::ModuleMetadata {
            id: self.id(),
            version: "1.0.0",
            description: "Logging functionality for Rivet pipelines",
            author: "Rivet",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::RivetModule;
    use std::sync::{Arc, Mutex};

    // Test implementation of LogSink
    struct TestLogSink {
        messages: Arc<Mutex<Vec<(LogLevel, String)>>>,
    }

    impl TestLogSink {
        fn new() -> (Self, Arc<Mutex<Vec<(LogLevel, String)>>>) {
            let messages = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    messages: messages.clone(),
                },
                messages,
            )
        }
    }

    impl LogSink for TestLogSink {
        fn write(&mut self, level: LogLevel, message: &str) {
            self.messages
                .lock()
                .unwrap()
                .push((level, message.to_string()));
        }
    }

    #[test]
    fn test_log_module_registration() {
        let (sink, _messages) = TestLogSink::new();
        let lua = Lua::new();
        let module = LogModule::new(sink);

        assert_eq!(module.id(), "log");
        assert!(module.register(&lua).is_ok());

        // Verify the log table exists
        let result: LuaResult<bool> = lua.load("return log ~= nil").eval();
        assert!(result.unwrap());

        // Verify functions exist
        let result: LuaResult<bool> = lua.load("return type(log.debug) == 'function'").eval();
        assert!(result.unwrap());

        let result: LuaResult<bool> = lua.load("return type(log.info) == 'function'").eval();
        assert!(result.unwrap());
    }

    #[test]
    fn test_log_collection() {
        let (sink, messages) = TestLogSink::new();
        let lua = Lua::new();
        let module = LogModule::new(sink);

        module.register(&lua).unwrap();

        // Log some messages
        lua.load(r#"log.info("test info message")"#).exec().unwrap();
        lua.load(r#"log.error("test error message")"#)
            .exec()
            .unwrap();

        // Check messages
        let logs = messages.lock().unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].0, LogLevel::Info);
        assert_eq!(logs[0].1, "test info message");
        assert_eq!(logs[1].0, LogLevel::Error);
        assert_eq!(logs[1].1, "test error message");
    }

    #[test]
    fn test_log_all_levels() {
        let (sink, messages) = TestLogSink::new();
        let lua = Lua::new();
        let module = LogModule::new(sink);

        module.register(&lua).unwrap();

        lua.load(r#"log.debug("debug")"#).exec().unwrap();
        lua.load(r#"log.info("info")"#).exec().unwrap();
        lua.load(r#"log.warning("warning")"#).exec().unwrap();
        lua.load(r#"log.error("error")"#).exec().unwrap();

        let logs = messages.lock().unwrap();
        assert_eq!(logs.len(), 4);
        assert_eq!(logs[0].0, LogLevel::Debug);
        assert_eq!(logs[1].0, LogLevel::Info);
        assert_eq!(logs[2].0, LogLevel::Warning);
        assert_eq!(logs[3].0, LogLevel::Error);
    }

    #[test]
    fn test_log_module_stubs() {
        let (sink, _messages) = TestLogSink::new();
        let module = LogModule::new(sink);
        let stubs = module.stubs();

        assert!(stubs.contains("---@meta"));
        assert!(stubs.contains("log = {}"));
        assert!(stubs.contains("function log.debug"));
        assert!(stubs.contains("function log.info"));
        assert!(stubs.contains("function log.warning"));
        assert!(stubs.contains("function log.error"));
    }
}
