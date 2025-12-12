# Rivet

> ⚠️ **WORK IN PROGRESS** - This is a learning project. Almost nothing works yet.

A CI/CD tool built from scratch to learn systems programming, distributed computing, and security sandboxing.

## Objective

Build a simple but functional CI/CD pipeline runner similar to Jenkins or GitHub Actions, focusing on:
- Security-first design (sandboxed execution, controlled access to system resources)
- Lua scripts for pipeline definitions
- Two-tier plugin system: secure Rust core modules + flexible Lua plugins
- Distributed job execution across runners with capability-based matching
- Understanding systems architecture and secure sandboxing

This is **not** production-ready. This is for learning.

## Architecture

```
CLI/Client → Orchestrator ← Runner(s)
                 ↓
          PostgreSQL + Redis
```

- **Orchestrator**: Manages state, stores pipelines/jobs, coordinates execution, matches jobs to capable runners
- **Runner**: Stateless workers that execute Lua scripts in sandboxed environments, reports capabilities on startup
- **Core**: Shared types, DTOs, and traits across components

## System Design

```
┌─────────────────┐
│       CLI       │  - Create Jobs & Pipelines
│                 │  - Request Executions
└────────┬────────┘
         │ HTTP
         │
┌────────▼────────┐
│  Orchestrator   │  - Job Queue & Matching
│                 │  - Pipeline Storage
│                 │  - Runner Registry
└────────┬────────┘
         │ HTTP Polling
         │
┌────────▼────────┐
│     Runner      │  ┌──────────────────────┐
│                 │  │  Lua Sandbox         │
│  Core Modules   │──│  - Pipeline Script   │
│  (Rust)         │  │  - Lua Plugins       │
│  - log          │  │  - Limited stdlib    │
│  - env          │  └──────────────────────┘
│  - process*     │
│  - http*        │  * = Requires capability
│  - filesystem*  │
│  - container*   │
└─────────────────┘
```

## Two-Tier Plugin System

Rivet uses a **security-focused two-tier architecture**:

### Tier 1: Rust Core Modules (Security Boundary)

Low-level, dangerous operations implemented in Rust with strict security enforcement:

- **log** - Buffered logging to orchestrator
- **env** - Access to environment variables and parameters
- **process** - Execute whitelisted binaries (requires `process` capability)
- **http** - Rate-limited HTTP client (requires `http` capability)
- **filesystem** - Workspace-jailed file operations (requires `filesystem` capability)
- **container** - Docker/Podman operations (requires `container` capability)

Core modules enforce:
- Command whitelisting
- Workspace isolation
- Rate limiting
- Input validation

### Tier 2: Lua Plugins (User Space)

High-level, domain-specific logic implemented in Lua using core modules:

```lua
-- plugins/git.lua
local git = {}

function git.clone(config)
    process.run({
        command = "git",
        args = {"clone", "--branch", config.branch, config.url}
    })
end

return git
```

**Key insight:** Lua plugins can't bypass security because they can only use core modules, which enforce policy.

### Plugin Examples

**Built-in plugins (included):**
- `git` - Clone, commit, push (uses `process`)
- `slack` - Notifications (uses `http`)
- `docker-compose` - Multi-container apps (uses `process` + `filesystem`)

**User plugins (you write):**
- Custom deployment scripts
- Internal API integrations
- Company-specific workflows

No Rust knowledge required. Just Lua + core module APIs.

## Pipeline Definition

```lua
return {
    name = "Build and Deploy",
    description = "Clones repo, runs tests, and notifies Slack",
    
    -- injects local git = require("plugin.git"), slack = require("plugin.slack")
    requires = {"plugin.git", "plugin.slack", "container"},
    
    inputs = {
        repo_url = {
            type = "string",
            description = "Git repository URL",
            required = true  -- Better than optional = false
        },
        slack_webhook = {
            type = "string", 
            description = "Slack webhook URL",
            required = true
        },
        branch = {
            type = "string",
            description = "Git branch to build",
            default = "main"  -- optional with default
        }
    },
    
    stages = {
        {
            name = "checkout",
            script = function()
                local branch = env.get("branch")
                git.clone({
                    url = env.get("repo_url"),
                    branch = branch
                })
            end
        },
        {
            name = "test",
            container = "python:3.11",
            script = function()
                process.run({command = "pytest", args = {"tests/"}})
            end
        },
        {
            name = "notify",
            script = function()
                slack.notify({
                    webhook = env.get("slack_webhook"),
                    message = "Build completed for " .. env.get("branch")
                })
            end
        }
    }
}
```

## Capability System

Runners announce their capabilities on startup. Orchestrator only assigns matching jobs.

**Runner capabilities:**
```json
{
  "runner_id": "runner-123",
  "capabilities": [
    "log", "env",              // Always available
    "process", "http",         // Core modules
    "plugin.git",              // Lua plugins with deps satisfied
    "plugin.slack",
    "container.docker"         // Container runtime available
  ],
  "labels": {"env": "prod", "region": "us-west"}
}
```

**Pipeline requirements:**
```lua
pipeline = {
    requires = {"plugin.git", "container"},  -- Only runs on capable runners
    -- ...
}
```

If a runner lacks `git` binary, it won't advertise `plugin.git` capability, and won't receive jobs requiring it.

## Current Status

- [x] Project structure (workspace with orchestrator, runner, core)
- [x] Module trait system (`RivetModule`)
- [x] Lua sandbox with restricted stdlib
- [x] Log module (buffered, batched sends to orchestrator)
- [x] Environment module
- [x] Orchestrator API (create pipeline, execute job, stream logs)
- [x] Runner job execution loop
- [ ] Capability registration and job matching
- [ ] Core modules: process, http, filesystem, container
- [ ] Lua plugins: git, slack, docker-compose
- [ ] Log batching (5s interval / 100 entries)
- [ ] Container stage support
- [ ] Job queue (Redis)

## Design Decisions

**Why Lua?**
- Small, embeddable, proven sandbox track record
- Full programming language (conditionals, loops, functions)
- Mature Rust integration (`mlua` crate)
- Users can write plugins without compiling Rust

**Why two-tier plugins?**
- **Security**: Dangerous ops (process, filesystem, network) controlled by Rust
- **Flexibility**: Common patterns (git, notifications) easy to add as Lua
- **Auditability**: Review Rust modules once, Lua plugins are safe by construction
- **Extensibility**: Users write plugins without touching core

**Why capability-based matching?**
- Not all runners have same tools (git, docker, k8s access)
- Fail fast at schedule time, not runtime
- Enables heterogeneous runner pools (dev vs prod, x86 vs ARM)

**Why Lua plugins over dylibs?**
- No compilation step, no ABI compatibility issues
- Easier to review and test
- Can't escape sandbox (unlike native code)
- Simpler distribution (just text files)

## Security Model

```
Pipeline Script (untrusted user code)
    ↓ can only use
Lua Plugins (community/user code)
    ↓ can only use
Rust Core Modules (audited once)
    ↓ enforce
Security Policy (whitelist, workspace jail, rate limits)
```

**Three layers of defense:**
1. Pipeline scripts can't access Lua stdlib (no `io`, `os`, `require` arbitrary files)
2. Plugins can only call core modules (no native code execution)
3. Core modules validate all inputs and enforce resource limits

**Attack surface:** Only the Rust core modules. If they're secure, everything above is automatically safe.

## Future Ideas

- Web UI for pipeline visualization and log streaming
- Kubernetes runner (stages as k8s Jobs)
- Secret management integration (Vault, k8s secrets)
- Artifact storage and caching
- Matrix builds (test across multiple versions/platforms)
- Plugin marketplace

But first: Get process + filesystem + http modules working, implement git plugin, run a real build end-to-end.

---

**This is a learning project.** Expect breaking changes, incomplete features, and exploration of different approaches.
