---@meta

---Logging module for Rivet pipelines
---
---Provides structured logging at different severity levels.
---Logs are buffered and sent to the orchestrator for centralized collection.
---
---All log functions accept a single string message parameter.
---For formatted output, use Lua's string.format() or concatenation.
---
---@class log
log = {}

---Log a debug message
---
---Debug messages are used for detailed diagnostic information useful during development.
---These may be filtered out in production environments.
---
---@param msg string The message to log
---
---@usage
---log.debug("Starting processing of item " .. item_id)
---log.debug(string.format("Processing item %d of %d", current, total))
function log.debug(msg) end

---Log an informational message
---
---Info messages are used for general informational messages about application progress.
---These typically indicate successful operations or important state changes.
---
---@param msg string The message to log
---
---@usage
---log.info("Build completed successfully")
---log.info("Connected to database")
function log.info(msg) end

---Log a warning message
---
---Warning messages indicate potentially harmful situations that don't prevent
---the pipeline from continuing but may require attention.
---
---@param msg string The message to log
---
---@usage
---log.warning("API rate limit approaching")
---log.warning("Using deprecated configuration option")
function log.warning(msg) end

---Log an error message
---
---Error messages indicate serious problems that may cause the pipeline to fail.
---Logging an error does not stop execution - use Lua's error() function for that.
---
---@param msg string The message to log
---
---@usage
---log.error("Failed to connect to database")
---log.error("Invalid configuration: missing required field 'api_key'")
function log.error(msg) end
