---@meta

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
