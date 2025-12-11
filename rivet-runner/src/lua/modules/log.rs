use mlua::prelude::*;
use rivet_core::module::RivetModule;
use rivet_core::types::{LogEntry, LogLevel};
use std::sync::{Arc, Mutex};

/// Logging module for Rivet Lua scripts
///
/// Collects logs in memory and sends them to the orchestrator.
/// Logs are buffered to avoid too many HTTP requests.
pub struct LogModule {
    /// Shared log buffer for collecting log entries
    log_buffer: Arc<Mutex<Vec<LogEntry>>>,
}

impl LogModule {
    /// Creates a new LogModule with a shared log buffer
    ///
    /// # Arguments
    /// * `log_buffer` - Shared buffer where logs will be collected
    pub fn new(log_buffer: Arc<Mutex<Vec<LogEntry>>>) -> Self {
        Self { log_buffer }
    }
}

impl RivetModule for LogModule {
    fn id(&self) -> &'static str {
        "log"
    }

    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let log_table = lua.create_table()?;

        // Debug level logging
        let buffer = self.log_buffer.clone();
        log_table.set(
            "debug",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Debug,
                    message: msg,
                };
                buffer
                    .lock()
                    .map_err(|e| {
                        LuaError::RuntimeError(format!("Failed to lock log buffer: {}", e))
                    })?
                    .push(entry);
                Ok(())
            })?,
        )?;

        // Info level logging
        let buffer = self.log_buffer.clone();
        log_table.set(
            "info",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Info,
                    message: msg,
                };
                buffer
                    .lock()
                    .map_err(|e| {
                        LuaError::RuntimeError(format!("Failed to lock log buffer: {}", e))
                    })?
                    .push(entry);
                Ok(())
            })?,
        )?;

        // Warning level logging
        let buffer = self.log_buffer.clone();
        log_table.set(
            "warning",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Warning,
                    message: msg,
                };
                buffer
                    .lock()
                    .map_err(|e| {
                        LuaError::RuntimeError(format!("Failed to lock log buffer: {}", e))
                    })?
                    .push(entry);
                Ok(())
            })?,
        )?;

        // Error level logging
        let buffer = self.log_buffer.clone();
        log_table.set(
            "error",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Error,
                    message: msg,
                };
                buffer
                    .lock()
                    .map_err(|e| {
                        LuaError::RuntimeError(format!("Failed to lock log buffer: {}", e))
                    })?
                    .push(entry);
                Ok(())
            })?,
        )?;

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

    fn metadata(&self) -> rivet_core::module::ModuleMetadata {
        rivet_core::module::ModuleMetadata {
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
    use rivet_core::module::RivetModule;

    #[test]
    fn test_log_module_registration() {
        let log_buffer = Arc::new(Mutex::new(Vec::new()));
        let lua = Lua::new();
        let module = LogModule::new(log_buffer);

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
        let log_buffer = Arc::new(Mutex::new(Vec::new()));
        let lua = Lua::new();
        let module = LogModule::new(log_buffer.clone());

        module.register(&lua).unwrap();

        // Log some messages
        lua.load(r#"log.info("test info message")"#).exec().unwrap();
        lua.load(r#"log.error("test error message")"#)
            .exec()
            .unwrap();

        // Check buffer
        let logs = log_buffer.lock().unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].level, LogLevel::Info);
        assert_eq!(logs[0].message, "test info message");
        assert_eq!(logs[1].level, LogLevel::Error);
        assert_eq!(logs[1].message, "test error message");
    }

    #[test]
    fn test_log_module_stubs() {
        let log_buffer = Arc::new(Mutex::new(Vec::new()));
        let module = LogModule::new(log_buffer);
        let stubs = module.stubs();

        assert!(stubs.contains("---@meta"));
        assert!(stubs.contains("log = {}"));
        assert!(stubs.contains("function log.debug"));
        assert!(stubs.contains("function log.info"));
        assert!(stubs.contains("function log.warning"));
        assert!(stubs.contains("function log.error"));
    }
}
