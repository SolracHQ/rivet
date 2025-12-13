//! Input module implementation for the runner
//!
//! Provides access to job input parameters in Lua scripts.

use mlua::prelude::*;
use std::collections::HashMap;

/// Register the input module into a Lua context
///
/// Creates an `input` global table with functions: get, require, has, all, keys
///
/// # Arguments
/// * `lua` - The Lua context to register into
/// * `parameters` - Job parameters from the orchestrator
///
/// # Example
/// ```no_run
/// use rivet_runner::lua::modules::register_input_module;
/// use rivet_lua::create_execution_sandbox;
/// use std::collections::HashMap;
///
/// let lua = create_execution_sandbox()?;
/// let mut params = HashMap::new();
/// params.insert("branch".to_string(), serde_json::Value::String("main".to_string()));
/// register_input_module(&lua, params)?;
///
/// lua.load(r#"local branch = input.get("branch", "main")"#).exec()?;
/// # Ok::<(), mlua::Error>(())
/// ```
pub fn register_input_module(
    lua: &Lua,
    parameters: HashMap<String, serde_json::Value>,
) -> LuaResult<()> {
    // Convert JSON values to strings for Lua consumption
    let vars: HashMap<String, String> = parameters
        .into_iter()
        .map(|(key, value)| {
            let value_str = match value {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => String::new(),
                // For complex types, serialize to JSON string
                other => serde_json::to_string(&other).unwrap_or_default(),
            };
            (key, value_str)
        })
        .collect();

    let input_table = lua.create_table()?;

    // input.get(name, default?)
    {
        let vars = vars.clone();
        input_table.set(
            "get",
            lua.create_function(move |_, (name, default): (String, Option<String>)| {
                Ok(vars.get(&name).cloned().or(default))
            })?,
        )?;
    }

    // input.require(name)
    {
        let vars = vars.clone();
        input_table.set(
            "require",
            lua.create_function(move |_, name: String| {
                vars.get(&name).cloned().ok_or_else(|| {
                    LuaError::RuntimeError(format!(
                        "Required input parameter '{}' is not set",
                        name
                    ))
                })
            })?,
        )?;
    }

    // input.has(name)
    {
        let vars = vars.clone();
        input_table.set(
            "has",
            lua.create_function(move |_, name: String| Ok(vars.contains_key(&name)))?,
        )?;
    }

    // input.all()
    {
        let vars = vars.clone();
        input_table.set(
            "all",
            lua.create_function(move |lua, ()| {
                let table = lua.create_table()?;
                for (key, value) in &vars {
                    table.set(key.as_str(), value.as_str())?;
                }
                Ok(table)
            })?,
        )?;
    }

    // input.keys()
    {
        let vars = vars.clone();
        input_table.set(
            "keys",
            lua.create_function(move |lua, ()| {
                let table = lua.create_table()?;
                let keys: Vec<String> = vars.keys().cloned().collect();
                for (i, key) in keys.iter().enumerate() {
                    table.set(i + 1, key.as_str())?;
                }
                Ok(table)
            })?,
        )?;
    }

    lua.globals().set("input", input_table)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_params() -> HashMap<String, serde_json::Value> {
        let mut params = HashMap::new();
        params.insert(
            "branch".to_string(),
            serde_json::Value::String("main".to_string()),
        );
        params.insert(
            "count".to_string(),
            serde_json::Value::Number(serde_json::Number::from(42)),
        );
        params.insert("enabled".to_string(), serde_json::Value::Bool(true));
        params
    }

    #[test]
    fn test_input_module_registration() {
        let lua = Lua::new();
        let params = create_test_params();

        register_input_module(&lua, params).unwrap();

        // Verify input table exists
        let has_input: bool = lua.load("return input ~= nil").eval().unwrap();
        assert!(has_input);

        // Verify functions exist
        let has_get: bool = lua
            .load("return type(input.get) == 'function'")
            .eval()
            .unwrap();
        assert!(has_get);

        let has_require: bool = lua
            .load("return type(input.require) == 'function'")
            .eval()
            .unwrap();
        assert!(has_require);
    }

    #[test]
    fn test_input_get() {
        let lua = Lua::new();
        let params = create_test_params();

        register_input_module(&lua, params).unwrap();

        // Get existing parameter
        let result: String = lua.load(r#"return input.get("branch")"#).eval().unwrap();
        assert_eq!(result, "main");

        // Get with default
        let result: String = lua
            .load(r#"return input.get("missing", "default")"#)
            .eval()
            .unwrap();
        assert_eq!(result, "default");

        // Get missing without default
        let result: Option<String> = lua.load(r#"return input.get("missing")"#).eval().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_input_require() {
        let lua = Lua::new();
        let params = create_test_params();

        register_input_module(&lua, params).unwrap();

        // Require existing parameter
        let result: String = lua
            .load(r#"return input.require("branch")"#)
            .eval()
            .unwrap();
        assert_eq!(result, "main");

        // Require missing parameter
        let result: LuaResult<String> = lua.load(r#"return input.require("missing")"#).eval();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Required input"));
    }

    #[test]
    fn test_input_has() {
        let lua = Lua::new();
        let params = create_test_params();

        register_input_module(&lua, params).unwrap();

        let exists: bool = lua.load(r#"return input.has("branch")"#).eval().unwrap();
        assert!(exists);

        let missing: bool = lua.load(r#"return input.has("missing")"#).eval().unwrap();
        assert!(!missing);
    }

    #[test]
    fn test_input_all() {
        let lua = Lua::new();
        let params = create_test_params();

        register_input_module(&lua, params).unwrap();

        let script = r#"
            local all = input.all()
            return all["branch"], all["count"], all["enabled"]
        "#;
        let (branch, count, enabled): (String, String, String) = lua.load(script).eval().unwrap();
        assert_eq!(branch, "main");
        assert_eq!(count, "42");
        assert_eq!(enabled, "true");
    }

    #[test]
    fn test_input_keys() {
        let lua = Lua::new();
        let params = create_test_params();

        register_input_module(&lua, params).unwrap();

        let script = r#"
            local keys = input.keys()
            local count = 0
            for _, _ in ipairs(keys) do
                count = count + 1
            end
            return count
        "#;
        let count: i32 = lua.load(script).eval().unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_input_empty() {
        let lua = Lua::new();
        let params = HashMap::new();

        register_input_module(&lua, params).unwrap();

        let has_any: bool = lua.load(r#"return input.has("anything")"#).eval().unwrap();
        assert!(!has_any);

        let keys_count: i32 = lua
            .load(
                r#"
            local keys = input.keys()
            return #keys
        "#,
            )
            .eval()
            .unwrap();
        assert_eq!(keys_count, 0);
    }

    #[test]
    fn test_input_type_conversions() {
        let lua = Lua::new();
        let params = create_test_params();

        register_input_module(&lua, params).unwrap();

        // Number converted to string
        let count: String = lua.load(r#"return input.get("count")"#).eval().unwrap();
        assert_eq!(count, "42");

        // Boolean converted to string
        let enabled: String = lua.load(r#"return input.get("enabled")"#).eval().unwrap();
        assert_eq!(enabled, "true");
    }
}
