# Rivet

> **WORK IN PROGRESS** - This is an experimental learning project for exploring systems programming, distributed computing, and container orchestration. Expect things to break and change frequently.

A CI/CD tool where pipelines are Lua scripts that run in containers. Pipelines are fully validated at creation time with proper type checking and LSP support.

## What Currently Works

Rivet can now execute pipelines with multiple stages, conditional execution, typed inputs with validation, and interactive CLI input collection. Check out the examples directory for working pipelines.

## Architecture

```
CLI/Client → Orchestrator ← Runner(s)
                 ↓
          PostgreSQL + Redis
```

- **Orchestrator**: Manages state, stores pipelines/jobs, coordinates execution
- **Runner**: Stateless workers that execute Lua scripts in sandboxed environments
- **Core**: Shared types, DTOs, and traits across components

## Development Setup

### Prerequisites

- Rust (stable)
- Podman
- Python 3

### Quick Start

Use the `dev.py` script to manage the development environment:

```bash
# Start all services (PostgreSQL, orchestrator, runner)
./dev.py start

# View logs
./dev.py logs

# Stop all services
./dev.py stop

# Restart services
./dev.py restart

# Stop and remove all data
./dev.py clean
```

The script will:
1. Start PostgreSQL in a Podman container
2. Build the project with `cargo build`
3. Start the orchestrator (logs to `logs/orchestrator.log`)
4. Start the runner (logs to `logs/runner.log`)

All logs are written to the `logs/` directory.

## Pipeline Definition

Rivet supports two ways to define pipelines:

### Declarative API

Using `pipeline.define()` with a table structure:

```lua
return pipeline.define({
    name = "Build and Test",
    description = "Builds the project and runs tests",
    
    inputs = {
        branch = {
            type = "string",
            description = "Git branch to build",
            default = "main"
        },
        parallel_jobs = {
            type = "number",
            description = "Number of parallel jobs",
            default = 4,
            options = { 1, 2, 4, 8, 16 }
        },
        skip_tests = {
            type = "bool",
            description = "Skip test stage",
            default = false,
            required = false
        }
    },
    
    runner = {
        { key = "os", value = "linux" },
        { key = "arch", value = "x86_64" }
    },
    
    stages = {
        {
            name = "build",
            container = "rust:latest",
            script = function()
                log.info("Building project...")
                local jobs = input.get("parallel_jobs", "4")
                log.info("Using " .. jobs .. " parallel jobs")
            end
        },
        {
            name = "test",
            container = "rust:latest",
            condition = function()
                return input.get("skip_tests") ~= "true"
            end,
            script = function()
                log.info("Running tests...")
            end
        }
    }
})
```

### Builder API

Using `pipeline.builder()` for a fluent interface:

```lua
return pipeline.builder()
    :name("Docker Build Pipeline")
    :description("Builds and pushes a Docker image")
    :input("image_name", {
        type = "string",
        description = "Docker image name",
        required = true
    })
    :input("push_image", {
        type = "bool",
        description = "Push to registry",
        default = true
    })
    :tag({ key = "capability", value = "docker" })
    :stage({
        name = "build",
        container = "docker:latest",
        script = function()
            log.info("Building image...")
        end
    })
    :build()
```

### LSP Support

Run `rivet-cli init lua` to generate stub files and LSP configuration for autocomplete and type checking in your editor. This creates `.luarc.json` and fetches stubs into `.rivet/stubs/` - works with any Lua language server.

## Features

- **Typed Inputs**: String, number, and bool types with validation
- **Default Values**: Inputs can have defaults, applied automatically
- **Enum Options**: Restrict inputs to specific allowed values
- **Interactive CLI**: Prompts for missing inputs with validation
- **Conditional Stages**: Stages can have condition functions to control execution
- **Container-per-Stage**: Each stage can specify its own container image
- **Input Validation**: Type checking and option validation before job execution

## Current Implementation Status

- [x] Project structure (workspace with orchestrator, runner, core)
- [x] Lua sandbox with restricted stdlib
- [x] Pipeline definition APIs (declarative and builder)
- [x] LSP support with type stubs
- [x] Typed input system (string, number, bool)
- [x] Input validation with defaults and options
- [x] Interactive CLI input collection
- [x] Conditional stage execution
- [x] Log module (buffered, batched sends to orchestrator)
- [x] Input module (read-only access to job parameters)
- [x] Basic orchestrator API (create pipeline, execute job, view logs)
- [x] Runner job execution loop with polling
- [ ] Process module with container execution
- [ ] Container module (stack-based container management)
- [ ] Plugin system (create, register, and use plugins)
- [ ] HTTP module
- [ ] Filesystem module
- [ ] Secret management
- [ ] Security context and authentication

## Design Philosophy

**Container-First Architecture**

Rivet is moving toward a container-first model similar to GitHub Actions. Runners only need container runtime (Podman/Kubernetes) to execute any pipeline. Instead of installing git, Python, Node.js, etc., runners spawn ephemeral containers with the right tools.

**Two-Tier Plugin System**

- **Rust Core Modules**: Low-level operations (process, http, filesystem) with security enforcement
- **Lua Plugins**: High-level domain logic (git, notifications) using core modules

Lua plugins can't bypass security because they only use core modules, which enforce policy.

**Security Model**

```
Pipeline Script (untrusted user code)
    ↓ can only use
Lua Plugins (community/user code)
    ↓ can only use
Rust Core Modules (audited once)
    ↓ enforce
Security Policy (container isolation, workspace jail, rate limits)
```

## Why Lua?

- Small, embeddable, proven sandbox track record
- Full programming language (conditionals, loops, functions)
- Mature Rust integration (`mlua` crate)
- Users can write plugins without compiling Rust
- LSP support available for type checking and autocomplete

## Next Steps

See `documents/Tasks.md` for detailed roadmap and current objectives.

---

**This is a learning project.** Expect breaking changes, incomplete features, and exploration of different approaches. My priorities may change frequently as I experiment with different ideas.