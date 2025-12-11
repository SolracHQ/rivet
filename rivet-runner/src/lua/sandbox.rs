use mlua::StdLib;
use mlua::prelude::*;

/// Create a new Lua sandbox with restricted standard libraries.
/// This sandbox only includes the TABLE, STRING, MATH, and COROUTINE libraries.
///
/// # Returns
/// A `Lua` instance configured as a sandbox.
pub fn create_sandbox() -> LuaResult<Lua> {
    // Start with NOTHING
    let lua = unsafe {
        Lua::unsafe_new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::COROUTINE,
            Default::default(),
        )
    };

    Ok(lua)
}
