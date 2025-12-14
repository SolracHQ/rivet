---@meta

---Pipeline definition module for Rivet
---
---Provides types and helper functions for defining CI/CD pipelines.
---This module exists purely for LSP/type checking support and is not
---available at pipeline runtime.
---
---LSP SETUP:
---To enable autocomplete and type checking in your pipeline scripts,
---run the following command to generate stub files and LSP configuration:
---
---  rivet-cli init lua
---
---This will create .luarc.json and fetch stub files into .rivet/stubs/
---The configuration is editor-agnostic and works with any Lua language server.
---
---@class pipeline
pipeline = {}

---Input type enumeration
---@alias InputType "string" | "number" | "bool"

---Input definition for pipeline parameters
---@class InputDefinition
---@field type InputType The type of the input parameter
---@field description string? Human-readable description of the input
---@field default string|number|boolean? Default value if not provided
---@field options (string|number|boolean)[]? Valid options for enum-like inputs
---@field required boolean? Whether this input is required (default: true)

---Stage condition function
---@alias StageCondition fun(): boolean

---Stage script function
---@alias StageScript fun(): nil

---Stage definition
---@class StageDefinition
---@field name string Unique identifier for this stage
---@field container string? Container image to use for this stage (e.g., "rust:latest")
---@field condition StageCondition? Function that returns true if stage should run
---@field script StageScript The stage implementation function

---Runner tag for capability matching
---@class Tag
---@field key string Tag key (e.g., "os", "arch", "capability")
---@field value string Tag value (e.g., "linux", "x86_64", "docker")

---Complete pipeline definition
---@class PipelineDefinition
---@field name string Pipeline name (must be unique)
---@field description string? Human-readable description of what this pipeline does
---@field inputs table<string, InputDefinition>? Input parameter definitions
---@field runner Tag[]? Runner requirements as key-value tags
---@field plugins string[]? Plugin names required by this pipeline
---@field stages StageDefinition[] Ordered list of stages to execute

---Define a pipeline with the given configuration
---
---This is the primary way to define a pipeline. Returns the pipeline
---definition table for the Rivet runtime to parse and execute.
---
---@param definition PipelineDefinition The complete pipeline configuration
---@return PipelineDefinition definition The same definition (for chaining)
---
---@usage
---return pipeline.define({
---  name = "Build and Test",
---  description = "Builds the project and runs tests",
---  inputs = {
---    branch = {
---      type = "string",
---      description = "Git branch to build",
---      default = "main"
---    },
---    verbose = {
---      type = "bool",
---      description = "Enable verbose logging",
---      default = false,
---      required = false
---    }
---  },
---  runner = {
---    { key = "os", value = "linux" },
---    { key = "capability", value = "docker" }
---  },
---  plugins = { "git", "docker" },
---  stages = {
---    {
---      name = "checkout",
---      script = function()
---        log.info("Checking out code...")
---      end
---    },
---    {
---      name = "build",
---      container = "rust:latest",
---      script = function()
---        log.info("Building project...")
---        process.run({ "cargo", "build", "--release" })
---      end
---    },
---    {
---      name = "test",
---      container = "rust:latest",
---      condition = function()
---        return input.get("skip_tests") ~= "true"
---      end,
---      script = function()
---        log.info("Running tests...")
---        process.run({ "cargo", "test" })
---      end
---    }
---  }
---})
function pipeline.define(definition) end

---Pipeline builder for fluent API construction
---@class PipelineBuilder
local PipelineBuilder = {}

---Set the pipeline name
---@param name string Pipeline name (required)
---@return PipelineBuilder self
function PipelineBuilder:name(name) end

---Set the pipeline description
---@param description string Human-readable description
---@return PipelineBuilder self
function PipelineBuilder:description(description) end

---Add an input parameter definition
---
---Can be called multiple times to add multiple inputs.
---@param name string Input parameter name
---@param definition InputDefinition Input configuration
---@return PipelineBuilder self
---
---@usage
---builder:input("branch", {
---  type = "string",
---  description = "Git branch",
---  default = "main"
---})
function PipelineBuilder:input(name, definition) end

---Add a runner requirement tag
---
---Can be called multiple times to add multiple tags.
---@param tag Tag Runner requirement tag
---@return PipelineBuilder self
---
---@usage
---builder:tag({ key = "os", value = "linux" })
---builder:tag({ key = "capability", value = "docker" })
function PipelineBuilder:tag(tag) end

---Add a required plugin
---
---Can be called multiple times to add multiple plugins.
---@param plugin_name string Plugin name (e.g., "git", "docker")
---@return PipelineBuilder self
---
---@usage
---builder:plugin("git")
---builder:plugin("docker")
function PipelineBuilder:plugin(plugin_name) end

---Add a stage definition
---
---Can be called multiple times to add multiple stages.
---Stages execute in the order they are added.
---@param stage StageDefinition Stage configuration
---@return PipelineBuilder self
---
---@usage
---builder:stage({
---  name = "build",
---  container = "rust:latest",
---  script = function()
---    process.run({ "cargo", "build" })
---  end
---})
function PipelineBuilder:stage(stage) end

---Build and return the final pipeline definition
---
---@return PipelineDefinition definition Complete pipeline definition
function PipelineBuilder:build() end

---Create a new pipeline builder for fluent API construction
---
---Use this if you prefer a builder pattern over the table-based definition.
---@return PipelineBuilder builder A new pipeline builder instance
---
---@usage
---return pipeline.builder()
---  :name("My Pipeline")
---  :description("Does awesome things")
---  :input("repo_url", {
---    type = "string",
---    description = "Repository URL",
---    required = true
---  })
---  :tag({ key = "os", value = "linux" })
---  :plugin("git")
---  :stage({
---    name = "checkout",
---    script = function()
---      log.info("Checking out...")
---    end
---  })
---  :stage({
---    name = "build",
---    container = "rust:latest",
---    script = function()
---      process.run({ "cargo", "build" })
---    end
---  })
---  :build()
function pipeline.builder() end

---Helper: Create an input definition
---
---Convenience function for creating properly typed input definitions.
---@param config InputDefinition Input configuration
---@return InputDefinition definition The input definition
---
---@usage
---inputs = {
---  my_param = pipeline.input({
---    type = "string",
---    description = "My parameter",
---    default = "value"
---  })
---}
function pipeline.input(config) end

---Helper: Create a stage definition
---
---Convenience function for creating properly typed stage definitions.
---@param config StageDefinition Stage configuration
---@return StageDefinition definition The stage definition
---
---@usage
---stages = {
---  pipeline.stage({
---    name = "build",
---    container = "rust:latest",
---    script = function()
---      process.run({ "cargo", "build" })
---    end
---  })
---}
function pipeline.stage(config) end

---Helper: Create a tag
---
---Convenience function for creating properly typed runner tags.
---@param key string Tag key
---@param value string Tag value
---@return Tag tag The tag definition
---
---@usage
---runner = {
---  pipeline.tag("os", "linux"),
---  pipeline.tag("arch", "x86_64")
---}
function pipeline.tag(key, value) end
