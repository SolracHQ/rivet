//! Container module implementation for the runner
//!
//! Provides container context management for Lua scripts.
//! Implements container.run(image, fn) which pushes a container onto the stack,
//! executes the function, then pops the container.

use mlua::prelude::*;
use std::sync::Arc;
use tracing::{debug, error};

use crate::context::Context;

/// Register the container module into a Lua context
///
/// Creates a `container` global table with the `run` function
///
/// # Arguments
/// * `lua` - The Lua context to register into
/// * `context` - The execution context with container manager
pub fn register_container_module(lua: &Lua, context: Arc<Context>) -> LuaResult<()> {
    let container_table = lua.create_table()?;

    // container.run(image, fn)
    {
        let context = context.clone();
        container_table.set(
            "run",
            lua.create_function(move |_lua_ctx, (image, func): (String, LuaFunction)| {
                debug!("Entering container.run with image: {}", image);

                // Push container onto stack
                let container_name =
                    context
                        .container_manager
                        .push_container(&image)
                        .map_err(|e| {
                            error!("Failed to push container for image {}: {}", image, e);
                            context.log_error(format!(
                                "Failed to start container for image {}: {}",
                                image, e
                            ));
                            LuaError::RuntimeError(format!("Failed to start container: {}", e))
                        })?;

                context.log_debug(format!(
                    "Container {} pushed to stack for image {}",
                    container_name, image
                ));

                // Execute the function
                let result = func.call::<()>(());

                // Always pop the container, even if function failed
                context.container_manager.pop_container();
                context.log_debug(format!(
                    "Container {} popped from stack for image {}",
                    container_name, image
                ));

                // Propagate any error from the function
                result?;

                Ok(())
            })?,
        )?;
    }

    lua.globals().set("container", container_table)?;
    Ok(())
}
