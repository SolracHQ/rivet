use mlua::prelude::*;
use rivet_core::module::RivetModule;
use std::collections::HashMap;

/// Environment module for accessing pipeline environment variables
///
/// Provides controlled access to environment variables and pipeline parameters.
/// Variables must be explicitly allowed in the pipeline configuration.
pub struct EnvModule {
    /// Allowed environment variables for this execution
    allowed_vars: HashMap<String, String>,
}

impl EnvModule {
    /// Creates a new EnvModule with the specified allowed variables
    ///
    /// # Arguments
    /// * `allowed_vars` - Map of variable names to values that can be accessed
    pub fn new(allowed_vars: HashMap<String, String>) -> Self {
        Self { allowed_vars }
    }

    /// Creates an EnvModule with no accessible variables
    pub fn empty() -> Self {
        Self {
            allowed_vars: HashMap::new(),
        }
    }
}

impl RivetModule for EnvModule {
    fn id(&self) -> &'static str {
        "env"
    }

    fn register(&self, lua: &Lua) -> LuaResult<()> {
        let env_table = lua.create_table()?;

        // Clone the allowed vars to move into closures
        let vars_for_get = self.allowed_vars.clone();
        let vars_for_has = self.allowed_vars.clone();
        let vars_for_all = self.allowed_vars.clone();

        // env.get(name, default?) - Get an environment variable
        env_table.set(
            "get",
            lua.create_function(move |_, (name, default): (String, Option<String>)| {
                match vars_for_get.get(&name) {
                    Some(value) => Ok(Some(value.clone())),
                    None => Ok(default),
                }
            })?,
        )?;

        // env.require(name) - Get a required environment variable (errors if missing)
        let vars_for_require = self.allowed_vars.clone();
        env_table.set(
            "require",
            lua.create_function(move |_, name: String| {
                vars_for_require.get(&name).cloned().ok_or_else(|| {
                    LuaError::RuntimeError(format!(
                        "Required environment variable '{}' is not set",
                        name
                    ))
                })
            })?,
        )?;

        // env.has(name) - Check if an environment variable exists
        env_table.set(
            "has",
            lua.create_function(move |_, name: String| Ok(vars_for_has.contains_key(&name)))?,
        )?;

        // env.all() - Get all available environment variables as a table
        env_table.set(
            "all",
            lua.create_function(move |lua, ()| {
                let table = lua.create_table()?;
                for (key, value) in &vars_for_all {
                    table.set(key.as_str(), value.as_str())?;
                }
                Ok(table)
            })?,
        )?;

        // env.keys() - Get all available environment variable names
        let vars_for_keys = self.allowed_vars.clone();
        env_table.set(
            "keys",
            lua.create_function(move |lua, ()| {
                let table = lua.create_table()?;
                for (i, key) in vars_for_keys.keys().enumerate() {
                    table.set(i + 1, key.as_str())?;
                }
                Ok(table)
            })?,
        )?;

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

    fn metadata(&self) -> rivet_core::module::ModuleMetadata {
        rivet_core::module::ModuleMetadata {
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

    #[test]
    fn test_env_module_get() {
        let lua = Lua::new();
        let mut vars = HashMap::new();
        vars.insert("TEST_VAR".to_string(), "test_value".to_string());
        vars.insert("ANOTHER_VAR".to_string(), "another_value".to_string());

        let module = EnvModule::new(vars);
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

        let module = EnvModule::new(vars);
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

        let module = EnvModule::new(vars);
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

        let module = EnvModule::new(vars);
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

        let module = EnvModule::new(vars);
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
        let module = EnvModule::empty();
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
        let module = EnvModule::empty();
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
