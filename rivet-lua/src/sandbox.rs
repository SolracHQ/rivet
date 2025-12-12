//! Lua sandbox creation for different evaluation contexts
//!
//! This module provides two types of sandboxes:
//! 1. **Metadata Sandbox**: For extracting pipeline configuration (name, description, inputs, requires)
//!    - No core modules available
//!    - Cannot perform I/O or side effects
//!    - Used by CLI and Orchestrator for pipeline parsing
//!
//! 2. **Execution Sandbox**: For running pipeline stage scripts
//!    - Full core modules available (log, env, process, etc.)
//!    - Can perform I/O and system operations (within capability constraints)
//!    - Used by Runner for actual job execution

use crate::module::ModuleRegistry;
use mlua::{Lua, LuaOptions, Result as LuaResult, StdLib};

/// Create a metadata-only Lua sandbox
///
/// This sandbox is designed for safe evaluation of pipeline configuration.
/// It includes only basic Lua functionality (tables, strings, math, coroutines)
/// and does NOT include any core modules or I/O capabilities.
///
/// # Use Cases
/// - CLI: Parse pipeline.lua to extract metadata for registration
/// - Orchestrator: Validate uploaded pipeline definitions
/// - Pre-execution: Extract requirements and capabilities needed
///
/// # Security
/// This sandbox prevents:
/// - Network access
/// - File system access
/// - Process execution
/// - Loading external modules
///
/// # Example
/// ```no_run
/// use rivet_lua::sandbox::create_metadata_sandbox;
///
/// let lua = create_metadata_sandbox()?;
/// let pipeline_source = r#"
///     return {
///         name = "My Pipeline",
///         description = "Does something cool",
///         inputs = { repo_url = { type = "string", required = true } },
///         requires = {"process", "plugin.git"},
///         stages = { -- ... stages here ... }
///     }
/// "#;
///
/// let pipeline_table: mlua::Table = lua.load(pipeline_source).eval()?;
/// let name: String = pipeline_table.get("name")?;
/// # Ok::<(), mlua::Error>(())
/// ```
pub fn create_metadata_sandbox() -> LuaResult<Lua> {
    // Create Lua with restricted standard libraries
    // Only allow: TABLE, STRING, MATH, COROUTINE
    // Explicitly exclude: IO, OS, PACKAGE, DEBUG
    let lua = unsafe {
        Lua::unsafe_new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::COROUTINE,
            LuaOptions::default(),
        )
    };

    // Verify dangerous globals are not accessible
    lua.globals().set("require", mlua::Nil)?;
    lua.globals().set("dofile", mlua::Nil)?;
    lua.globals().set("loadfile", mlua::Nil)?;

    Ok(lua)
}

/// Create a full execution Lua sandbox with core modules
///
/// This sandbox includes all core Rivet modules and is used for actual
/// pipeline execution. Modules must be provided via a ModuleRegistry.
///
/// # Arguments
/// * `registry` - A ModuleRegistry containing all modules to register
///
/// # Use Cases
/// - Runner: Execute pipeline stage scripts with full capabilities
/// - Local testing: Run pipelines with mock implementations
///
/// # Security
/// While this sandbox has access to core modules, those modules enforce:
/// - Command whitelisting (process module)
/// - Workspace isolation (filesystem module)
/// - Rate limiting (http module)
/// - Input validation (all modules)
///
/// # Example
/// ```no_run
/// use rivet_lua::sandbox::create_execution_sandbox;
/// use rivet_lua::modules::{LogModule, EnvModule, LogSink, VarProvider};
/// use rivet_lua::RivetModule;
/// use rivet_lua::ModuleRegistry;
/// use rivet_core::domain::log::LogLevel;
///
/// // Implement LogSink for your use case
/// struct MyLogSink;
/// impl LogSink for MyLogSink {
///     fn write(&mut self, level: LogLevel, message: &str) {
///         println!("[{:?}] {}", level, message);
///     }
/// }
///
/// // Implement VarProvider for your use case
/// struct MyVarProvider;
/// impl VarProvider for MyVarProvider {
///     fn get(&self, name: &str) -> Option<String> {
///         if name == "BRANCH" { Some("main".to_string()) } else { None }
///     }
///     fn keys(&self) -> Vec<String> {
///         vec!["BRANCH".to_string()]
///     }
/// }
///
/// let mut registry = ModuleRegistry::new();
/// registry.register(LogModule::new(MyLogSink));
/// registry.register(EnvModule::new(MyVarProvider));
///
/// let lua = create_execution_sandbox(&registry)?;
///
/// // Now can execute stage scripts
/// lua.load(r#"
///     log.info("Starting build on branch: " .. env.get("BRANCH"))
/// "#).exec()?;
/// # Ok::<(), mlua::Error>(())
/// ```
pub fn create_execution_sandbox(registry: &ModuleRegistry) -> LuaResult<Lua> {
    // Create Lua with the same restricted stdlib as metadata sandbox
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

    // Register all modules from registry
    registry.register_all(&lua)?;

    Ok(lua)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_sandbox_basic_lua() {
        let lua = create_metadata_sandbox().unwrap();

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
    fn test_metadata_sandbox_no_io() {
        let lua = create_metadata_sandbox().unwrap();

        // Should NOT have io module
        let has_io: bool = lua.load(r#"return io ~= nil"#).eval().unwrap();
        assert!(!has_io);

        // Should NOT have os module
        let has_os: bool = lua.load(r#"return os ~= nil"#).eval().unwrap();
        assert!(!has_os);
    }

    #[test]
    fn test_metadata_sandbox_no_require() {
        let lua = create_metadata_sandbox().unwrap();

        // require should not be available
        let result: LuaResult<()> = lua.load(r#"require("os")"#).exec();
        assert!(result.is_err());
    }

    #[test]
    fn test_metadata_sandbox_no_core_modules() {
        let lua = create_metadata_sandbox().unwrap();

        // Core modules should not be available in metadata sandbox
        let has_log: bool = lua.load(r#"return log ~= nil"#).eval().unwrap();
        assert!(!has_log);

        let has_env: bool = lua.load(r#"return env ~= nil"#).eval().unwrap();
        assert!(!has_env);

        let has_process: bool = lua.load(r#"return process ~= nil"#).eval().unwrap();
        assert!(!has_process);
    }

    #[test]
    fn test_metadata_sandbox_can_parse_pipeline() {
        let lua = create_metadata_sandbox().unwrap();

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
                requires = {"process", "plugin.git"},
                stages = {}
            }
        "#;

        let pipeline: mlua::Table = lua.load(pipeline_def).eval().unwrap();
        let name: String = pipeline.get("name").unwrap();
        assert_eq!(name, "Test Pipeline");

        let requires: mlua::Table = pipeline.get("requires").unwrap();
        let first_req: String = requires.get(1).unwrap();
        assert_eq!(first_req, "process");
    }

    #[test]
    fn test_execution_sandbox_empty_registry() {
        let registry = ModuleRegistry::new();
        let lua = create_execution_sandbox(&registry).unwrap();

        // Should still have basic Lua
        let result: i32 = lua.load("return 1 + 1").eval().unwrap();
        assert_eq!(result, 2);

        // But no core modules since registry is empty
        let has_log: bool = lua.load(r#"return log ~= nil"#).eval().unwrap();
        assert!(!has_log);
    }

    #[test]
    fn test_execution_sandbox_no_io() {
        let registry = ModuleRegistry::new();
        let lua = create_execution_sandbox(&registry).unwrap();

        // Even execution sandbox should not have raw io/os access
        let has_io: bool = lua.load(r#"return io ~= nil"#).eval().unwrap();
        assert!(!has_io);

        let has_os: bool = lua.load(r#"return os ~= nil"#).eval().unwrap();
        assert!(!has_os);
    }

    #[test]
    fn test_execution_sandbox_no_require() {
        let registry = ModuleRegistry::new();
        let lua = create_execution_sandbox(&registry).unwrap();

        // require should not work even in execution sandbox
        // (plugins are loaded differently, not via require)
        let result: LuaResult<()> = lua.load(r#"require("os")"#).exec();
        assert!(result.is_err());
    }
}
