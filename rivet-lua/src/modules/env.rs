//! Environment module for accessing pipeline environment variables
//!
//! This module provides a trait-based abstraction for variable access that allows
//! different components to provide their own implementations:
//! - Runner: Restricted variable access from job parameters
//! - CLI: Mock variables for parsing and testing
//! - Orchestrator: Validation-only variable access

use crate::module::RivetModule;
use mlua::prelude::*;

/// Trait for providing environment variables
///
/// Implement this trait to provide custom variable access behavior.
/// The EnvModule is generic over this trait, allowing different
/// components to provide their own implementations.
///
/// # Thread Safety
/// Implementations must be Send + Sync to work with Lua's threading model.
pub trait VarProvider: Send + Sync {
    /// Get a variable by name
    ///
    /// # Arguments
    /// * `name` - The name of the variable to retrieve
    ///
    /// # Returns
    /// The variable value if it exists, otherwise None
    fn get(&self, name: &str) -> Option<String>;

    /// Get all available variable names
    ///
    /// # Returns
    /// A vector of all variable names that can be accessed
    fn keys(&self) -> Vec<String>;
}

/// Environment module for accessing pipeline environment variables
///
/// Generic over VarProvider trait to allow different implementations
/// depending on the execution context.
pub struct EnvModule<V: VarProvider> {
    provider: std::sync::Arc<std::sync::Mutex<V>>,
}

impl<V: VarProvider> EnvModule<V> {
    /// Creates a new EnvModule with the provided variable provider
    ///
    /// # Arguments
    /// * `provider` - Implementation of VarProvider trait
    pub fn new(provider: V) -> Self {
        Self {
            provider: std::sync::Arc::new(std::sync::Mutex::new(provider)),
        }
    }
}

impl<V: VarProvider + 'static> RivetModule for EnvModule<V> {
    fn id(&self) -> &'static str {
        "env"
    }

    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let env_table = lua.create_table()?;

        // env.get(name, default?) - Get an environment variable
        {
            let provider = self.provider.clone();
            env_table.set(
                "get",
                lua.create_function(move |_, (name, default): (String, Option<String>)| {
                    let value = provider
                        .lock()
                        .map_err(|e| {
                            LuaError::RuntimeError(format!("Failed to lock provider: {}", e))
                        })?
                        .get(&name);
                    Ok(value.or(default))
                })?,
            )?;
        }

        // env.require(name) - Get a required environment variable (errors if missing)
        {
            let provider = self.provider.clone();
            env_table.set(
                "require",
                lua.create_function(move |_, name: String| {
                    provider
                        .lock()
                        .map_err(|e| {
                            LuaError::RuntimeError(format!("Failed to lock provider: {}", e))
                        })?
                        .get(&name)
                        .ok_or_else(|| {
                            LuaError::RuntimeError(format!(
                                "Required environment variable '{}' is not set",
                                name
                            ))
                        })
                })?,
            )?;
        }

        // env.has(name) - Check if an environment variable exists
        {
            let provider = self.provider.clone();
            env_table.set(
                "has",
                lua.create_function(move |_, name: String| {
                    Ok(provider
                        .lock()
                        .map_err(|e| {
                            LuaError::RuntimeError(format!("Failed to lock provider: {}", e))
                        })?
                        .get(&name)
                        .is_some())
                })?,
            )?;
        }

        // env.all() - Get all available environment variables as a table
        {
            let provider = self.provider.clone();
            env_table.set(
                "all",
                lua.create_function(move |lua, ()| {
                    let table = lua.create_table()?;
                    let provider = provider.lock().map_err(|e| {
                        LuaError::RuntimeError(format!("Failed to lock provider: {}", e))
                    })?;

                    for key in provider.keys() {
                        if let Some(value) = provider.get(&key) {
                            table.set(key.as_str(), value.as_str())?;
                        }
                    }
                    Ok(table)
                })?,
            )?;
        }

        // env.keys() - Get all available environment variable names
        {
            let provider = self.provider.clone();
            env_table.set(
                "keys",
                lua.create_function(move |lua, ()| {
                    let table = lua.create_table()?;
                    let keys = provider
                        .lock()
                        .map_err(|e| {
                            LuaError::RuntimeError(format!("Failed to lock provider: {}", e))
                        })?
                        .keys();

                    for (i, key) in keys.iter().enumerate() {
                        table.set(i + 1, key.as_str())?;
                    }
                    Ok(table)
                })?,
            )?;
        }

        lua.globals().set(self.id(), env_table)?;
        Ok(())
    }

    fn stubs(&self) -> String {
        r#"---@meta

---Environment variable access module
---Provides controlled access to environment variables configured in the pipeline
---@class env
env = {}

---Get an environment variable with an optional default value
---Returns the variable value if it exists, otherwise returns the default value or nil
---@param name string The name of the environment variable
---@param default? string The default value to return if the variable is not set
---@return string? value The value of the environment variable or the default
---
---@usage
---local api_key = env.get("API_KEY", "default-key")
---local optional = env.get("OPTIONAL_VAR")  -- returns nil if not set
function env.get(name, default) end

---Get a required environment variable
---Throws an error if the variable is not set
---@param name string The name of the environment variable
---@return string value The value of the environment variable
---
---@usage
---local api_key = env.require("API_KEY")  -- errors if API_KEY is not set
function env.require(name) end

---Check if an environment variable exists
---@param name string The name of the environment variable
---@return boolean exists True if the variable exists, false otherwise
---
---@usage
---if env.has("DEBUG") then
---  log.debug("Debug mode enabled")
---end
function env.has(name) end

---Get all available environment variables as a table
---@return table<string, string> vars A table mapping variable names to values
---
---@usage
---local all_vars = env.all()
---for key, value in pairs(all_vars) do
---  log.debug(key .. " = " .. value)
---end
function env.all() end

---Get all available environment variable names
---@return string[] keys An array of variable names
---
---@usage
---local keys = env.keys()
---for i, key in ipairs(keys) do
---  log.info("Variable: " .. key)
---end
function env.keys() end
"#
        .to_string()
    }

    fn metadata(&self) -> crate::module::ModuleMetadata {
        crate::module::ModuleMetadata {
            id: self.id(),
            version: "1.0.0",
            description: "Environment variable access for pipeline scripts",
            author: "Rivet",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Test implementation of VarProvider
    struct TestVarProvider {
        vars: HashMap<String, String>,
    }

    impl TestVarProvider {
        fn new(vars: HashMap<String, String>) -> Self {
            Self { vars }
        }
    }

    impl VarProvider for TestVarProvider {
        fn get(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }

        fn keys(&self) -> Vec<String> {
            self.vars.keys().cloned().collect()
        }
    }

    #[test]
    fn test_env_module_get() {
        let lua = Lua::new();
        let mut vars = HashMap::new();
        vars.insert("TEST_VAR".to_string(), "test_value".to_string());
        vars.insert("ANOTHER_VAR".to_string(), "another_value".to_string());

        let provider = TestVarProvider::new(vars);
        let module = EnvModule::new(provider);
        module.register(&lua).unwrap();

        // Test getting existing variable
        let result: String = lua.load(r#"return env.get("TEST_VAR")"#).eval().unwrap();
        assert_eq!(result, "test_value");

        // Test getting non-existent variable with default
        let result: String = lua
            .load(r#"return env.get("MISSING", "default")"#)
            .eval()
            .unwrap();
        assert_eq!(result, "default");

        // Test getting non-existent variable without default
        let result: Option<String> = lua.load(r#"return env.get("MISSING")"#).eval().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_module_require() {
        let lua = Lua::new();
        let mut vars = HashMap::new();
        vars.insert("REQUIRED_VAR".to_string(), "required_value".to_string());

        let provider = TestVarProvider::new(vars);
        let module = EnvModule::new(provider);
        module.register(&lua).unwrap();

        // Test requiring existing variable
        let result: String = lua
            .load(r#"return env.require("REQUIRED_VAR")"#)
            .eval()
            .unwrap();
        assert_eq!(result, "required_value");

        // Test requiring missing variable (should error)
        let result: LuaResult<String> = lua.load(r#"return env.require("MISSING")"#).eval();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Required environment variable")
        );
    }

    #[test]
    fn test_env_module_has() {
        let lua = Lua::new();
        let mut vars = HashMap::new();
        vars.insert("EXISTS".to_string(), "value".to_string());

        let provider = TestVarProvider::new(vars);
        let module = EnvModule::new(provider);
        module.register(&lua).unwrap();

        let exists: bool = lua.load(r#"return env.has("EXISTS")"#).eval().unwrap();
        assert!(exists);

        let missing: bool = lua.load(r#"return env.has("MISSING")"#).eval().unwrap();
        assert!(!missing);
    }

    #[test]
    fn test_env_module_all() {
        let lua = Lua::new();
        let mut vars = HashMap::new();
        vars.insert("VAR1".to_string(), "value1".to_string());
        vars.insert("VAR2".to_string(), "value2".to_string());

        let provider = TestVarProvider::new(vars);
        let module = EnvModule::new(provider);
        module.register(&lua).unwrap();

        let script = r#"
            local all = env.all()
            return all["VAR1"], all["VAR2"]
        "#;
        let (v1, v2): (String, String) = lua.load(script).eval().unwrap();
        assert_eq!(v1, "value1");
        assert_eq!(v2, "value2");
    }

    #[test]
    fn test_env_module_keys() {
        let lua = Lua::new();
        let mut vars = HashMap::new();
        vars.insert("KEY1".to_string(), "value1".to_string());
        vars.insert("KEY2".to_string(), "value2".to_string());

        let provider = TestVarProvider::new(vars);
        let module = EnvModule::new(provider);
        module.register(&lua).unwrap();

        let script = r#"
            local keys = env.keys()
            local count = 0
            for _, _ in ipairs(keys) do
                count = count + 1
            end
            return count
        "#;
        let count: i32 = lua.load(script).eval().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_env_module_empty() {
        let lua = Lua::new();
        let provider = TestVarProvider::new(HashMap::new());
        let module = EnvModule::new(provider);
        module.register(&lua).unwrap();

        let has_any: bool = lua.load(r#"return env.has("ANYTHING")"#).eval().unwrap();
        assert!(!has_any);

        let script = r#"
            local keys = env.keys()
            local count = 0
            for _, _ in ipairs(keys) do
                count = count + 1
            end
            return count
        "#;
        let count: i32 = lua.load(script).eval().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_stubs_generation() {
        let provider = TestVarProvider::new(HashMap::new());
        let module = EnvModule::new(provider);
        let stubs = module.stubs();

        assert!(stubs.contains("---@meta"));
        assert!(stubs.contains("env = {}"));
        assert!(stubs.contains("function env.get"));
        assert!(stubs.contains("function env.require"));
        assert!(stubs.contains("function env.has"));
        assert!(stubs.contains("function env.all"));
        assert!(stubs.contains("function env.keys"));
    }
}
