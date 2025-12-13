//! Log module implementation for the runner
//!
//! Provides logging functionality to Lua scripts with buffered output
//! that is sent to the orchestrator.

use mlua::prelude::*;
use rivet_core::domain::log::{LogEntry, LogLevel};
use std::sync::Arc;

use crate::service::LogBufferService;

/// Register the log module into a Lua context
///
/// Creates a `log` global table with functions: debug, info, warning, error
///
/// # Arguments
/// * `lua` - The Lua context to register into
/// * `buffer` - The log buffer service to write to
///
/// # Example
/// ```no_run
/// use rivet_runner::lua::modules::register_log_module;
/// use rivet_lua::create_execution_sandbox;
///
/// let lua = create_execution_sandbox()?;
/// let buffer = create_log_buffer_service();
/// register_log_module(&lua, buffer)?;
///
/// lua.load(r#"log.info("Hello from Lua")"#).exec()?;
/// # Ok::<(), mlua::Error>(())
/// ```
pub fn register_log_module(lua: &Lua, buffer: Arc<dyn LogBufferService>) -> LuaResult<()> {
    let log_table = lua.create_table()?;

    // log.debug(msg)
    {
        let buffer = buffer.clone();
        log_table.set(
            "debug",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Debug,
                    message: msg,
                };
                buffer.add_entry(entry);
                Ok(())
            })?,
        )?;
    }

    // log.info(msg)
    {
        let buffer = buffer.clone();
        log_table.set(
            "info",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Info,
                    message: msg,
                };
                buffer.add_entry(entry);
                Ok(())
            })?,
        )?;
    }

    // log.warning(msg)
    {
        let buffer = buffer.clone();
        log_table.set(
            "warning",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Warning,
                    message: msg,
                };
                buffer.add_entry(entry);
                Ok(())
            })?,
        )?;
    }

    // log.error(msg)
    {
        let buffer = buffer.clone();
        log_table.set(
            "error",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Error,
                    message: msg,
                };
                buffer.add_entry(entry);
                Ok(())
            })?,
        )?;
    }

    lua.globals().set("log", log_table)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::domain::log::LogLevel;
    use std::sync::Mutex;

    struct TestLogBuffer {
        entries: Arc<Mutex<Vec<LogEntry>>>,
    }

    impl TestLogBuffer {
        fn new() -> (Self, Arc<Mutex<Vec<LogEntry>>>) {
            let entries = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    entries: entries.clone(),
                },
                entries,
            )
        }
    }

    impl LogBufferService for TestLogBuffer {
        fn add_entry(&self, entry: LogEntry) {
            self.entries.lock().unwrap().push(entry);
        }

        fn drain(&self) -> Vec<LogEntry> {
            Vec::new()
        }
    }

    #[test]
    fn test_log_module_registration() {
        let lua = Lua::new();
        let (buffer, _entries) = TestLogBuffer::new();

        register_log_module(&lua, Arc::new(buffer)).unwrap();

        // Verify log table exists
        let has_log: bool = lua.load("return log ~= nil").eval().unwrap();
        assert!(has_log);

        // Verify functions exist
        let has_debug: bool = lua
            .load("return type(log.debug) == 'function'")
            .eval()
            .unwrap();
        assert!(has_debug);

        let has_info: bool = lua
            .load("return type(log.info) == 'function'")
            .eval()
            .unwrap();
        assert!(has_info);
    }

    #[test]
    fn test_log_collection() {
        let lua = Lua::new();
        let (buffer, entries) = TestLogBuffer::new();

        register_log_module(&lua, Arc::new(buffer)).unwrap();

        lua.load(r#"log.info("test message")"#).exec().unwrap();
        lua.load(r#"log.error("error message")"#).exec().unwrap();

        let logs = entries.lock().unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].level, LogLevel::Info);
        assert_eq!(logs[0].message, "test message");
        assert_eq!(logs[1].level, LogLevel::Error);
        assert_eq!(logs[1].message, "error message");
    }

    #[test]
    fn test_all_log_levels() {
        let lua = Lua::new();
        let (buffer, entries) = TestLogBuffer::new();

        register_log_module(&lua, Arc::new(buffer)).unwrap();

        lua.load(r#"log.debug("debug")"#).exec().unwrap();
        lua.load(r#"log.info("info")"#).exec().unwrap();
        lua.load(r#"log.warning("warning")"#).exec().unwrap();
        lua.load(r#"log.error("error")"#).exec().unwrap();

        let logs = entries.lock().unwrap();
        assert_eq!(logs.len(), 4);
        assert_eq!(logs[0].level, LogLevel::Debug);
        assert_eq!(logs[1].level, LogLevel::Info);
        assert_eq!(logs[2].level, LogLevel::Warning);
        assert_eq!(logs[3].level, LogLevel::Error);
    }
}
