---@meta

---Output module for inter-stage communication
---
---Provides a mechanism to pass data between pipeline stages.
---Unlike global variables, outputs are explicitly managed and provide
---a clear contract for stage dependencies.
---
---Outputs set in one stage are available to all subsequent stages.
---This enables clean data flow without relying on global state.
---
---@class output
output = {}

---Set an output value
---
---Stores a key-value pair that will be available to subsequent stages.
---If a key already exists, it will be overwritten.
---
---@param name string The name of the output parameter
---@param value string The value to store
---
---@usage
---output.set("commit_sha", "abc123def456")
---output.set("build_version", "1.2.3")
---output.set("artifact_url", "https://example.com/artifact.tar.gz")
---
---local version = string.format("%d.%d.%d", major, minor, patch)
---output.set("version", version)
function output.set(name, value) end

---Get an output value with an optional default
---
---Retrieves a value that was set by a previous stage.
---Returns the default value if the output doesn't exist.
---
---@param name string The name of the output parameter
---@param default? string The default value to return if the parameter is not set
---@return string? value The value of the output parameter or the default
---
---@usage
---local commit = output.get("commit_sha", "unknown")
---local version = output.get("build_version")
---
---if version then
---  log.info("Deploying version: " .. version)
---end
function output.get(name, default) end

---Get a required output value
---
---Retrieves a value that must have been set by a previous stage.
---Throws an error if the output doesn't exist.
---
---@param name string The name of the output parameter
---@return string value The value of the output parameter
---
---@usage
---local artifact = output.require("artifact_url")
---local sha = output.require("commit_sha")  -- errors if not set by previous stage
function output.require(name) end

---Check if an output value exists
---
---Returns true if a value has been set for the given name.
---Useful for conditional logic based on optional outputs from previous stages.
---
---@param name string The name of the output parameter
---@return boolean exists True if the parameter exists, false otherwise
---
---@usage
---if output.has("build_artifact") then
---  log.info("Artifact available for deployment")
---else
---  log.warning("No artifact produced, skipping deployment")
---end
---
---if not output.has("test_results") then
---  output.set("test_results", "skipped")
---end
function output.has(name) end

---Get all output values as a table
---
---Returns a table mapping all output names to their values.
---Useful for debugging or logging all outputs at once.
---
---@return table<string, string> outputs A table mapping output names to values
---
---@usage
---local all_outputs = output.all()
---for key, value in pairs(all_outputs) do
---  log.debug("Output: " .. key .. " = " .. value)
---end
---
---log.info("Stage produced " .. #output.keys() .. " outputs")
function output.all() end

---Get all output parameter names
---
---Returns an array of all output names that have been set.
---The order is not guaranteed.
---
---@return string[] keys An array of output names
---
---@usage
---local keys = output.keys()
---for i, key in ipairs(keys) do
---  log.info("Available output: " .. key)
---end
---
---if #keys == 0 then
---  log.info("No outputs produced yet")
---end
function output.keys() end

---Clear a specific output value
---
---Removes an output that was previously set.
---Useful for cleanup or resetting state between retries.
---
---@param name string The name of the output parameter to clear
---
---@usage
---output.clear("temporary_file")
---output.clear("build_cache")
function output.clear(name) end

---Clear all output values
---
---Removes all outputs that have been set.
---Use with caution - this will affect all subsequent stages.
---
---@usage
---output.clear_all()
---log.info("All outputs cleared")
function output.clear_all() end
