use mlua::prelude::*;

/// Trait for Rivet Lua modules.
///
/// Each module provides functionality to Lua scripts running in the sandbox.
/// Modules must have a unique identifier and can register functions, tables,
/// and other values into the Lua global scope.
///
/// # Example
///
/// ```rust
/// use rivet_core::module::RivetModule;
/// use mlua::prelude::*;
///
/// struct LogModule;
///
/// impl RivetModule for LogModule {
///     fn id(&self) -> &'static str {
///         "log"
///     }
///
///     fn register(&self, lua: &Lua) -> LuaResult<()> {
///         let log_table = lua.create_table()?;
///
///         log_table.set("info", lua.create_function(|_, msg: String| {
///             println!("[INFO] {}", msg);
///             Ok(())
///         })?)?;
///
///         lua.globals().set(self.id(), log_table)?;
///         Ok(())
///     }
///
///     fn stubs(&self) -> String {
///         r#"---@meta
///
/// ---Logging module for Rivet pipelines
/// ---@class log
/// log = {}
///
/// ---Log an info message
/// ---@param msg string The message to log
/// function log.info(msg) end
/// "#.to_string()
///     }
/// }
/// ```
pub trait RivetModule: Send + Sync {
    /// Returns the unique identifier for this module.
    ///
    /// This identifier will be used as the global variable name in Lua.
    /// For example, if `id()` returns `"log"`, the module will be accessible
    /// in Lua scripts as `log.function_name()`.
    ///
    /// # Requirements
    /// - Must be a valid Lua identifier (alphanumeric + underscore, no leading digit)
    /// - Must be unique across all modules
    /// - Should be lowercase by convention
    fn id(&self) -> &'static str;

    /// Registers this module's functions and values into the Lua context.
    ///
    /// This method is called when setting up the sandbox. Implementations should:
    /// 1. Create a table for the module (if needed)
    /// 2. Add functions and values to the table
    /// 3. Set the table as a global with the module's `id()`
    ///
    /// # Arguments
    /// * `lua` - The Lua context to register into
    ///
    /// # Errors
    /// Returns `LuaError` if registration fails (e.g., invalid function, type error)
    fn register(&self, lua: &Lua) -> LuaResult<()>;

    /// Generates Lua Language Server stubs for this module.
    ///
    /// Returns a string containing LuaLS annotations that provide:
    /// - Type information
    /// - Function signatures
    /// - Documentation
    ///
    /// The stub should start with `---@meta` to mark it as a definition file.
    ///
    /// # Returns
    /// A string containing the complete stub file content
    fn stubs(&self) -> String;

    /// Optional: Returns module metadata (version, description, etc.)
    ///
    /// This can be used for documentation, debugging, or plugin management.
    fn metadata(&self) -> ModuleMetadata {
        ModuleMetadata {
            id: self.id(),
            version: "0.1.0",
            description: "",
            author: "",
        }
    }
}

/// Metadata about a Rivet module
#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    /// Module identifier
    pub id: &'static str,
    /// Module version (semver)
    pub version: &'static str,
    /// Brief description of module functionality
    pub description: &'static str,
    /// Module author
    pub author: &'static str,
}

/// Registry for managing Rivet modules
///
/// Provides a central place to register and retrieve modules.
/// Used by the runner to load modules into the Lua sandbox.
pub struct ModuleRegistry {
    modules: Vec<Box<dyn RivetModule>>,
}

impl ModuleRegistry {
    /// Creates a new empty module registry
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    /// Registers a module
    ///
    /// # Panics
    /// Panics if a module with the same ID is already registered
    pub fn register<M: RivetModule + 'static>(&mut self, module: M) {
        let id = module.id();
        if self.modules.iter().any(|m| m.id() == id) {
            panic!("Module with id '{}' is already registered", id);
        }
        self.modules.push(Box::new(module));
    }

    /// Gets a module by its ID
    pub fn get(&self, id: &str) -> Option<&dyn RivetModule> {
        self.modules
            .iter()
            .find(|m| m.id() == id)
            .map(|m| m.as_ref())
    }

    /// Returns all registered modules
    pub fn modules(&self) -> &[Box<dyn RivetModule>] {
        &self.modules
    }

    /// Registers all modules into a Lua context
    ///
    /// # Arguments
    /// * `lua` - The Lua context to register modules into
    ///
    /// # Errors
    /// Returns the first error encountered during registration
    pub fn register_all(&self, lua: &Lua) -> LuaResult<()> {
        for module in &self.modules {
            module.register(lua)?;
        }
        Ok(())
    }

    /// Generates a combined stub file for all registered modules
    ///
    /// # Returns
    /// A string containing stubs for all modules, suitable for saving as a `.lua` file
    pub fn generate_stubs(&self) -> String {
        let mut stubs = String::new();
        for module in &self.modules {
            stubs.push_str(&module.stubs());
            stubs.push_str("\n\n");
        }
        stubs
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestModule;

    impl RivetModule for TestModule {
        fn id(&self) -> &'static str {
            "test"
        }

        fn register(&self, lua: &Lua) -> LuaResult<()> {
            let table = lua.create_table()?;
            table.set("value", 42)?;
            lua.globals().set(self.id(), table)?;
            Ok(())
        }

        fn stubs(&self) -> String {
            "---@meta\n---@class test\ntest = {}".to_string()
        }
    }

    #[test]
    fn test_module_registration() {
        let mut registry = ModuleRegistry::new();
        registry.register(TestModule);

        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    #[should_panic(expected = "already registered")]
    fn test_duplicate_registration() {
        let mut registry = ModuleRegistry::new();
        registry.register(TestModule);
        registry.register(TestModule);
    }

    #[test]
    fn test_stub_generation() {
        let mut registry = ModuleRegistry::new();
        registry.register(TestModule);

        let stubs = registry.generate_stubs();
        assert!(stubs.contains("---@meta"));
        assert!(stubs.contains("test = {}"));
    }
}
