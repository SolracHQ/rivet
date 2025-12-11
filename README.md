# Rivet

> ⚠️ **WORK IN PROGRESS** - This is a learning project. Almost nothing works yet.

A CI/CD tool built from scratch to learn systems programming, distributed computing, and security sandboxing.

## Objective

Build a simple but functional CI/CD pipeline runner similar to Jenkins or GitHub Actions, focusing on:
- Security-first design (sandboxed execution, no arbitrary shell access)
- Lua scripts for pipeline definitions with controlled module system
- Distributed job execution across runners
- State management via orchestrator
- Understanding systems architecture

This is **not** production-ready. This is for learning.

## Architecture

```
CLI/Client → Orchestrator ← Runner(s)
                 ↓
          PostgreSQL + Redis
```

- **Orchestrator**: Manages state, stores pipelines/jobs, coordinates execution
- **Runner**: Stateless workers that execute Lua scripts in sandboxed environments
- **Core**: Shared types, DTOs, and traits across components

## Basic Architecture

**Current design** (subject to change):

```
┌─────────────────┐
│       CLI       │  - Create Jobs
│                 │  - Create pipelines
│                 │  - Request executions
└────────┬────────┘
         │ CLI command to HTTP requests
         │
┌────────▼────────┐
│  Orchestrator   │  - Accepts jobs
│                 │  - Stores Lua scripts
│                 │  - Queues jobs
└────────┬────────┘
         │
         │ HTTP (Runner polls for jobs)
         │
┌────────▼────────┐
│     Runner      │  - Executes Lua scripts
│                 │  - Sandboxed environment
│                 │  - Streams logs back
└─────────────────┘
```

**Shared Core**: Common data structures (Job, JobStatus, etc.)

The CLI module is a temporal replacement for a web UI or API client.

## Pipeline Definition

Pipelines are defined in Lua with injected APIs for safe operations:

```lua
pipeline = {
  name = "Example Pipeline",
  stages = {
    {
      name = "checkout",
      script = function()
        log.info("Cloning repository...")
        git.clone({
          url = "https://github.com/user/repo.git",
          branch = "main"
        })
      end
    },
    {
      name = "test",
      script = function()
        log.info("Running tests...")
        process.run({
          command = "pytest",
          args = {"tests/"}
        })
      end
    }
  }
}
```

**Note**: The module system is now implemented. Modules are the only way to interact with the outside world from Lua scripts.

## Module System

All external functionality is provided through **modules** that implement the `RivetModule` trait:

- **log**: Collects logs and sends them to orchestrator
- **env**: Access to pipeline environment variables and parameters
- More modules coming (http, git, docker, etc.)

Modules are registered at runner startup and loaded into each sandbox. They buffer operations and communicate with the orchestrator to maintain stateless execution.

## Current Status

- [x] Project structure
- [x] Module trait system (`RivetModule`)
- [x] Lua sandbox (restricted stdlib)
- [x] Core types and DTOs
- [x] Log module (buffered, sends to orchestrator)
- [x] Environment module
- [x] Orchestrator API endpoints
- [x] Runner job execution loop
- [ ] Job queue (Redis)
- [ ] Actual end-to-end job execution

## Design Decisions

**Why Lua?**
- Small, embeddable, easy to sandbox
- Full programming language (not limited like YAML)
- Mature Rust integration (mlua)

**Why module system?**
- Security boundary: validate all inputs, limit resource usage
- Controlled external access: no arbitrary shell/filesystem operations
- Flexibility: modules can be added without changing core
- Future plugin support via dynamic loading

## Future Ideas

- Kubernetes runner support
- Secret management
- Docker-in-Docker builds
- Web UI (probably Flutter)
- Distributed runners

But first, we need to get a single job to execute locally.

---

**This is a learning project.** Expect breaking changes, incomplete features, and questionable decisions as I figure things out.
