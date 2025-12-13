//! Process module implementation for the runner
//!
//! Provides process execution functionality to Lua scripts.
//! Commands are executed inside the container managed by the context.

use mlua::prelude::*;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::context::Context;

/// Register the process module into a Lua context
///
/// Creates a `process` global table with the `run` function
///
/// # Arguments
/// * `lua` - The Lua context to register into
/// * `context` - The execution context with container manager
pub fn register_process_module(lua: &Lua, context: Arc<Context>) -> LuaResult<()> {
    let process_table = lua.create_table()?;

    // process.run(options)
    {
        let context = context.clone();
        process_table.set(
            "run",
            lua.create_function(move |lua_ctx, options: LuaTable| {
                // Parse options
                let cmd: String = options.get("cmd").map_err(|_| {
                    LuaError::RuntimeError("process.run requires 'cmd' field".to_string())
                })?;

                let args: Vec<String> = options
                    .get::<Option<LuaTable>>("args")
                    .ok()
                    .flatten()
                    .map(|tbl| {
                        let mut args = Vec::new();
                        for pair in tbl.pairs::<i32, String>() {
                            if let Ok((_, arg)) = pair {
                                args.push(arg);
                            }
                        }
                        args
                    })
                    .unwrap_or_default();

                let capture_stdout: bool = options.get("capture_stdout").unwrap_or(false);
                let capture_stderr: bool = options.get("capture_stderr").unwrap_or(false);
                let stdout_level: String = options
                    .get("stdout_level")
                    .unwrap_or_else(|_| "info".to_string());
                let stderr_level: String = options
                    .get("stderr_level")
                    .unwrap_or_else(|_| "error".to_string());
                let cwd: Option<String> = options.get("cwd").ok();

                debug!("Executing process: {} {:?}", cmd, args);

                // Execute command in container
                let (stdout, stderr, exit_code) = context
                    .container_manager
                    .exec(&cmd, &args, cwd.as_deref())
                    .map_err(|e| {
                        LuaError::RuntimeError(format!("Failed to execute command: {}", e))
                    })?;

                // Log stdout if not captured
                if !capture_stdout && !stdout.is_empty() {
                    log_output(&context, &stdout, &stdout_level);
                }

                // Log stderr if not captured
                if !capture_stderr && !stderr.is_empty() {
                    log_output(&context, &stderr, &stderr_level);
                }

                // Create result table
                let result = lua_ctx.create_table()?;
                result.set("exit_code", exit_code)?;

                if capture_stdout {
                    result.set("stdout", stdout)?;
                }

                if capture_stderr {
                    result.set("stderr", stderr)?;
                }

                Ok(result)
            })?,
        )?;
    }

    lua.globals().set("process", process_table)?;
    Ok(())
}

/// Logs output with the specified level
fn log_output(context: &Context, output: &str, level: &str) {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return;
    }

    match level.to_lowercase().as_str() {
        "debug" => context.log_debug(trimmed.to_string()),
        "info" => context.log_info(trimmed.to_string()),
        "warning" | "warn" => context.log_warning(trimmed.to_string()),
        "error" => context.log_error(trimmed.to_string()),
        _ => {
            warn!("Unknown log level '{}', defaulting to info", level);
            context.log_info(trimmed.to_string());
        }
    }
}
