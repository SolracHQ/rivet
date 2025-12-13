//! Log module implementation for the runner
//!
//! Provides logging functionality to Lua scripts with buffered output
//! that is sent to the orchestrator.

use mlua::prelude::*;
use rivet_core::domain::log::{LogEntry, LogLevel};
use std::sync::Arc;

use crate::context::Context;

/// Register the log module into a Lua context
///
/// Creates a `log` global table with functions: debug, info, warning, error
///
/// # Arguments
/// * `lua` - The Lua context to register into
/// * `context` - The execution context to write logs to
pub fn register_log_module(lua: &Lua, context: Arc<Context>) -> LuaResult<()> {
    let log_table = lua.create_table()?;

    // log.debug(msg)
    {
        let context = context.clone();
        log_table.set(
            "debug",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Debug,
                    message: msg,
                };
                context.add_log(entry);
                Ok(())
            })?,
        )?;
    }

    // log.info(msg)
    {
        let context = context.clone();
        log_table.set(
            "info",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Info,
                    message: msg,
                };
                context.add_log(entry);
                Ok(())
            })?,
        )?;
    }

    // log.warning(msg)
    {
        let context = context.clone();
        log_table.set(
            "warning",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Warning,
                    message: msg,
                };
                context.add_log(entry);
                Ok(())
            })?,
        )?;
    }

    // log.error(msg)
    {
        let context = context.clone();
        log_table.set(
            "error",
            lua.create_function(move |_, msg: String| {
                let entry = LogEntry {
                    timestamp: chrono::Utc::now(),
                    level: LogLevel::Error,
                    message: msg,
                };
                context.add_log(entry);
                Ok(())
            })?,
        )?;
    }

    lua.globals().set("log", log_table)?;
    Ok(())
}
