# Rivet

> ⚠️ **WORK IN PROGRESS** - This is an experimental learning project. Almost nothing works yet.

A CI/CD tool built from scratch to learn systems programming, distributed computing, and container orchestration.

## What Currently Works

Right now, Rivet can execute a basic pipeline with logging capabilities. See `examples/only_loggin.lua` for a working example.

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

## Running the Example

The `only_loggin.lua` example demonstrates the current functionality:

```lua
return {
    name = "Example Pipeline",
    description = "A simple pipeline that demonstrates logging",

    inputs = {
        message = {
            type = "string",
            description = "A message to log",
            default = "Hello from Rivet!"
        }
    },

    stages = {
        {
            name = "checkout",
            script = function()
                log.info("Starting checkout stage...")
                log.debug("This is a debug message")
                log.info("Checkout completed successfully")
            end
        },
        {
            name = "test",
            script = function()
                log.info("Starting test stage...")
                local message = env.get("message", "default message")
                log.info("Message from environment: " .. message)
                log.warning("This is a warning message")
                log.info("Tests completed successfully")
            end
        },
        {
            name = "deploy",
            script = function()
                log.info("Starting deploy stage...")
                log.info("Deploying application...")
                log.info("Deployment completed successfully")
            end
        }
    }
}
```

This pipeline demonstrates:
- Multiple stages executing sequentially
- Logging at different levels (debug, info, warning)
- Environment variable access through the `env` module
- Input parameters with defaults

## Current Implementation Status

- [x] Project structure (workspace with orchestrator, runner, core)
- [x] Module trait system (`RivetModule`)
- [x] Lua sandbox with restricted stdlib
- [x] Log module (buffered, batched sends to orchestrator)
- [x] Environment module (read-only access to job parameters)
- [x] Basic orchestrator API (create pipeline, execute job, stream logs)
- [x] Runner job execution loop with polling
- [ ] Process module with container execution
- [ ] Plugin system (create, register, and use plugins)
- [ ] HTTP module
- [ ] Filesystem module
- [ ] Secret management
- [ ] Security context and authentication
- [ ] Job queue improvements

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

## Next Steps

See `documents/Tasks.md` for detailed roadmap and current objectives.

---

**This is a learning project.** Expect breaking changes, incomplete features, and exploration of different approaches. My priorities may change frequently as I experiment with different ideas.