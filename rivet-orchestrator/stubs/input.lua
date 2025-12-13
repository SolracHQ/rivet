---@meta

---Input module for accessing pipeline input parameters
---
---Provides controlled access to input parameters defined in the pipeline configuration.
---Input parameters are passed when executing a job and are available throughout all stages.
---
---This module does NOT provide access to OS environment variables - only pipeline inputs.
---
---@class input
input = {}

---Get an input parameter with an optional default value
---
---Returns the parameter value if it exists, otherwise returns the default value or nil.
---This is useful for optional parameters where you want to provide a fallback value.
---
---@param name string The name of the input parameter
---@param default? string The default value to return if the parameter is not set
---@return string? value The value of the input parameter or the default
---
---@usage
---local branch = input.get("branch", "main")
---local optional_tag = input.get("tag")  -- returns nil if not set
---
---if optional_tag then
---  log.info("Building tag: " .. optional_tag)
---end
function input.get(name, default) end

---Get a required input parameter
---
---Throws an error if the parameter is not set. Use this for mandatory parameters
---that must be provided for the pipeline to function correctly.
---
---@param name string The name of the input parameter
---@return string value The value of the input parameter
---
---@usage
---local repo_url = input.require("repo_url")  -- errors if repo_url is not set
---local api_token = input.require("api_token")
function input.require(name) end

---Check if an input parameter exists
---
---Returns true if the parameter was provided, false otherwise.
---Useful for conditional logic based on optional parameters.
---
---@param name string The name of the input parameter
---@return boolean exists True if the parameter exists, false otherwise
---
---@usage
---if input.has("debug") then
---  log.debug("Debug mode enabled")
---  log.debug("Additional diagnostic information...")
---end
---
---if not input.has("api_key") then
---  log.warning("No API key provided, using limited mode")
---end
function input.has(name) end

---Get all available input parameters as a table
---
---Returns a table mapping parameter names to their values.
---Useful for debugging or logging all inputs at once.
---
---@return table<string, string> params A table mapping parameter names to values
---
---@usage
---local all_inputs = input.all()
---for key, value in pairs(all_inputs) do
---  log.debug(key .. " = " .. value)
---end
---
---log.info("Running with " .. #input.keys() .. " input parameters")
function input.all() end

---Get all available input parameter names
---
---Returns an array of parameter names that were provided to this job.
---The order is not guaranteed.
---
---@return string[] keys An array of parameter names
---
---@usage
---local keys = input.keys()
---for i, key in ipairs(keys) do
---  log.info("Input parameter: " .. key)
---end
---
---if #keys == 0 then
---  log.warning("No input parameters provided")
---end
function input.keys() end
