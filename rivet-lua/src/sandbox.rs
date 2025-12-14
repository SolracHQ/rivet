//! Lua sandbox creation
//!
//! This module provides a restricted Lua sandbox that prevents access to
//! dangerous operations like filesystem I/O, network access, and process execution.
//!
//! The pipeline module is always injected as it's needed for parsing definitions.
//! Core modules (log, input, process, container, etc.) are registered by the caller
//! after creating the sandbox, typically in the runner.

use mlua::{Lua, LuaOptions, Result as LuaResult, StdLib, Table};

/// Create a restricted Lua sandbox
///
/// This sandbox includes only basic Lua functionality (tables, strings, math, coroutines)
/// and does NOT include any I/O capabilities or the ability to load external code.
///
/// # Use Cases
/// - CLI: Parse pipeline.lua to extract metadata for registration
/// - Orchestrator: Validate uploaded pipeline definitions
/// - Runner: Execute pipeline stage scripts (after registering core modules)
///
/// # Security
/// This sandbox prevents:
/// - Network access
/// - File system access
/// - Process execution
/// - Loading external modules via require()
///
/// # Example
/// ```no_run
/// use rivet_lua::sandbox::create_sandbox;
///
/// let lua = create_sandbox()?;
///
/// // For metadata parsing (CLI/orchestrator):
/// let pipeline_source = r#"
///     return {
///         name = "My Pipeline",
///         description = "Does something cool",
///         inputs = { repo_url = { type = "string", required = true } },
///         stages = { -- ... stages here ... }
///     }
/// "#;
/// let pipeline_table: mlua::Table = lua.load(pipeline_source).eval()?;
/// let name: String = pipeline_table.get("name")?;
///
/// // For execution (runner would register modules first):
/// // register_log_module(&lua)?;
/// // register_input_module(&lua)?;
/// // lua.load(r#"log.info("Starting build")"#).exec()?;
/// # Ok::<(), mlua::Error>(())
/// ```
pub fn create_sandbox() -> LuaResult<Lua> {
    // Create Lua with restricted standard libraries
    // Only allow: TABLE, STRING, MATH, COROUTINE
    // Explicitly exclude: IO, OS, PACKAGE, DEBUG
    let lua = unsafe {
        Lua::unsafe_new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::COROUTINE,
            LuaOptions::default(),
        )
    };

    // Remove dangerous globals
    lua.globals().set("require", mlua::Nil)?;
    lua.globals().set("dofile", mlua::Nil)?;
    lua.globals().set("loadfile", mlua::Nil)?;

    // Register pipeline module (always available for definition parsing)
    register_pipeline_module(&lua)?;

    Ok(lua)
}

/// Register the pipeline module
///
/// This module provides helper functions for defining pipelines.
/// Currently it's just a passthrough - pipeline.define() returns the table as-is.
fn register_pipeline_module(lua: &Lua) -> LuaResult<()> {
    let pipeline = lua.create_table()?;

    // pipeline.define(definition) - returns the definition table as-is
    let define_fn = lua.create_function(|_, definition: Table| Ok(definition))?;
    pipeline.set("define", define_fn)?;

    // pipeline.builder() - returns a builder metatable
    let builder_fn = lua.create_function(|lua, ()| create_pipeline_builder(lua))?;
    pipeline.set("builder", builder_fn)?;

    // pipeline.input(config) - returns the config table as-is
    let input_fn = lua.create_function(|_, config: Table| Ok(config))?;
    pipeline.set("input", input_fn)?;

    // pipeline.stage(config) - returns the config table as-is
    let stage_fn = lua.create_function(|_, config: Table| Ok(config))?;
    pipeline.set("stage", stage_fn)?;

    // pipeline.tag(key, value) - returns a tag table
    let tag_fn = lua.create_function(|lua, (key, value): (String, String)| {
        let tag = lua.create_table()?;
        tag.set("key", key)?;
        tag.set("value", value)?;
        Ok(tag)
    })?;
    pipeline.set("tag", tag_fn)?;

    lua.globals().set("pipeline", pipeline)?;

    Ok(())
}

/// Create a pipeline builder instance with fluent API methods
fn create_pipeline_builder(lua: &Lua) -> LuaResult<Table> {
    let builder = lua.create_table()?;
    let metatable = lua.create_table()?;

    // Builder methods return self for chaining
    let name_fn = lua.create_function(|_, (builder, name): (Table, String)| {
        builder.set("_name", name)?;
        Ok(builder)
    })?;
    metatable.set("name", name_fn)?;

    let description_fn = lua.create_function(|_, (builder, desc): (Table, String)| {
        builder.set("_description", desc)?;
        Ok(builder)
    })?;
    metatable.set("description", description_fn)?;

    let input_fn = lua.create_function(|lua, (builder, name, def): (Table, String, Table)| {
        let inputs: Table = match builder.get("_inputs") {
            Ok(t) => t,
            Err(_) => {
                let t = lua.create_table()?;
                builder.set("_inputs", t.clone())?;
                t
            }
        };
        inputs.set(name, def)?;
        Ok(builder)
    })?;
    metatable.set("input", input_fn)?;

    let tag_fn = lua.create_function(|lua, (builder, tag): (Table, Table)| {
        let runner: Table = match builder.get("_runner") {
            Ok(t) => t,
            Err(_) => {
                let t = lua.create_table()?;
                builder.set("_runner", t.clone())?;
                t
            }
        };
        let len = runner.len()? + 1;
        runner.set(len, tag)?;
        Ok(builder)
    })?;
    metatable.set("tag", tag_fn)?;

    let plugin_fn = lua.create_function(|lua, (builder, plugin): (Table, String)| {
        let plugins: Table = match builder.get("_plugins") {
            Ok(t) => t,
            Err(_) => {
                let t = lua.create_table()?;
                builder.set("_plugins", t.clone())?;
                t
            }
        };
        let len = plugins.len()? + 1;
        plugins.set(len, plugin)?;
        Ok(builder)
    })?;
    metatable.set("plugin", plugin_fn)?;

    let stage_fn = lua.create_function(|lua, (builder, stage): (Table, Table)| {
        let stages: Table = match builder.get("_stages") {
            Ok(t) => t,
            Err(_) => {
                let t = lua.create_table()?;
                builder.set("_stages", t.clone())?;
                t
            }
        };
        let len = stages.len()? + 1;
        stages.set(len, stage)?;
        Ok(builder)
    })?;
    metatable.set("stage", stage_fn)?;

    // build() converts builder to pipeline definition table
    let build_fn = lua.create_function(|lua, builder: Table| {
        let definition = lua.create_table()?;

        if let Ok(name) = builder.get::<String>("_name") {
            definition.set("name", name)?;
        }
        if let Ok(desc) = builder.get::<String>("_description") {
            definition.set("description", desc)?;
        }
        if let Ok(inputs) = builder.get::<Table>("_inputs") {
            definition.set("inputs", inputs)?;
        }
        if let Ok(runner) = builder.get::<Table>("_runner") {
            definition.set("runner", runner)?;
        }
        if let Ok(plugins) = builder.get::<Table>("_plugins") {
            definition.set("plugins", plugins)?;
        }
        if let Ok(stages) = builder.get::<Table>("_stages") {
            definition.set("stages", stages)?;
        }

        Ok(definition)
    })?;
    metatable.set("build", build_fn)?;

    // Set __index to the metatable itself so methods are accessible
    metatable.set("__index", metatable.clone())?;
    builder.set_metatable(Some(metatable))?;

    Ok(builder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_basic_lua() {
        let lua = create_sandbox().unwrap();

        // Should be able to create tables and use strings
        let result: i32 = lua
            .load(
                r#"
                local t = {a = 1, b = 2}
                return t.a + t.b
            "#,
            )
            .eval()
            .unwrap();
        assert_eq!(result, 3);

        // Should be able to use string manipulation
        let result: String = lua.load(r#"return string.upper("hello")"#).eval().unwrap();
        assert_eq!(result, "HELLO");

        // Should be able to use math
        let result: f64 = lua.load(r#"return math.sqrt(16)"#).eval().unwrap();
        assert_eq!(result, 4.0);
    }

    #[test]
    fn test_sandbox_no_io() {
        let lua = create_sandbox().unwrap();

        // Should NOT have io module
        let has_io: bool = lua.load(r#"return io ~= nil"#).eval().unwrap();
        assert!(!has_io);

        // Should NOT have os module
        let has_os: bool = lua.load(r#"return os ~= nil"#).eval().unwrap();
        assert!(!has_os);
    }

    #[test]
    fn test_sandbox_no_require() {
        let lua = create_sandbox().unwrap();

        // require should not be available
        let result: LuaResult<()> = lua.load(r#"require("os")"#).exec();
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_no_core_modules_by_default() {
        let lua = create_sandbox().unwrap();

        // Core modules should not be available until registered
        let has_log: bool = lua.load(r#"return log ~= nil"#).eval().unwrap();
        assert!(!has_log);

        let has_input: bool = lua.load(r#"return input ~= nil"#).eval().unwrap();
        assert!(!has_input);

        let has_process: bool = lua.load(r#"return process ~= nil"#).eval().unwrap();
        assert!(!has_process);
    }

    #[test]
    fn test_sandbox_can_parse_pipeline() {
        let lua = create_sandbox().unwrap();

        let pipeline_def = r#"
            return {
                name = "Test Pipeline",
                description = "A test pipeline",
                inputs = {
                    repo_url = {
                        type = "string",
                        required = true
                    }
                },
                plugins = {"git"},
                stages = {}
            }
        "#;

        let pipeline: mlua::Table = lua.load(pipeline_def).eval().unwrap();
        let name: String = pipeline.get("name").unwrap();
        assert_eq!(name, "Test Pipeline");

        let plugins: mlua::Table = pipeline.get("plugins").unwrap();
        let first_plugin: String = plugins.get(1).unwrap();
        assert_eq!(first_plugin, "git");
    }

    #[test]
    fn test_sandbox_can_register_globals() {
        let lua = create_sandbox().unwrap();

        // Caller can register modules as globals
        let test_table = lua.create_table().unwrap();
        test_table.set("value", 42).unwrap();
        lua.globals().set("test", test_table).unwrap();

        let result: i32 = lua.load("return test.value").eval().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_sandbox_has_pipeline_module() {
        let lua = create_sandbox().unwrap();

        // Pipeline module should be available
        let has_pipeline: bool = lua.load(r#"return pipeline ~= nil"#).eval().unwrap();
        assert!(has_pipeline);

        // Test pipeline.define() - should return the table as-is
        let result: String = lua
            .load(
                r#"
            local def = pipeline.define({ name = "test" })
            return def.name
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(result, "test");

        // Test pipeline.tag()
        let key: String = lua
            .load(
                r#"
            local tag = pipeline.tag("os", "linux")
            return tag.key
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(key, "os");
    }
}
