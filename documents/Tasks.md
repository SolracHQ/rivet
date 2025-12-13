# Tasks

Just a simple place to put my ideas.

WARNING: This is an experimental project. I'm testing things, my priorities can change any time soon, so this file will change a lot.

## Current Objective

Implement the process module with a container-first strategy that will replace capabilities (like GitHub Actions).

For now, the initial idea is to have function-scoped containers managed with Podman that are launched with something like `sleep 99999999` and run the commands with `exec -it`. I need to think about how to combine this with logging - maybe accepting logging level as table input, or sending stderr/stdout to different logging levels based on user request.

## Priority Tasks

### High Priority (In Order)

1. Process module with container execution
   - Only `process.run()` calls execute inside containers, all Lua code runs in the runner
   - By default, a long-lived bare Alpine container can be available for process execution
   - Support ephemeral containers spawned per plugin operation
   - Support stage-level persistent containers for multi-step operations
   - Figure out how to integrate with logging (stdout/stderr capture)

2. Plugin system
   - Create plugin API and structure
   - Register plugins in runner
   - Inject plugins into Lua sandbox
   - Create initial git plugin using process module

### Medium Priority (No Specific Order)

- Implement HTTP module
  - Rate limiting
  - HTTP client for external API calls
  
- Implement filesystem module
  - Workspace-jailed operations
  - File read/write within container context
  
- Implement secret module
  - Secure secret storage
  - Controlled access from pipelines

- Implement archive module
  - Tar/zip operations
  - Artifact handling

### Low Priority

- Evaluate PostgreSQL as queue or Redis to make orchestrator scalable
  - Use row locks to have multiple orchestrator instances
  - All state lives in the database
  - Not too different from what we're doing now, but taking advantage of database features

- Security context implementation
  - Login system
  - Permissions and configurations
  - Only runners with orchestrator-generated token should pull jobs
  - Only authorized users should run certain pipelines
  - Only authorized pipelines should fetch certain secrets

- Add redact secret logic to logging module
  - Prevent secrets from appearing in logs
  - Pattern-based secret detection

- Improve error messages
  - More context and specificity
  - If polling fails, show "polling failed because orchestrator is unreachable" not just "polling fail"
  - Better error propagation from Rust to Lua

- Remove capabilities system
  - No longer useful with container-first approach
  - Simplify runner registration
  - Remove capability matching logic

- Add CLI option to fetch plugins in the init system
  - Users get completions in their IDE
  - Similar to what we already do with stubs for core modules
  - Better developer experience

## Architecture Vision

### Container-First Execution Model

All Lua code runs in the runner process (not in containers). Containers are execution contexts for `process.run()` calls. State (variables, plugin instances) lives in the runner and persists across container spawns.

### Plugin Usage (Ephemeral Containers)

Plugins internally use containers, but users never see this complexity.

Example - git plugin implementation:
```lua
function git.clone(config)
    container.run("docker.io/alpine/git:latest", function()
        process.run({command = "git", args = {"clone", config.url}})
    end)
end
```

User just writes:
```lua
local git = require("git")
git.clone({url = "https://github.com/user/repo.git", branch = "main"})
```

The plugin spawns alpine/git container, executes git clone, destroys container. User sees clean API, no container management.

### User-Controlled Containers

For multi-step operations in the same environment, users can explicitly request persistent containers:

```lua
container.run("docker.io/python:3.11", function()
    process.run({command = "pip", args = {"install", "-r", "requirements.txt"}})
    process.run({command = "pytest", args = {"tests/"}})
    process.run({command = "python", args = {"setup.py", "bdist_wheel"}})
end)
```

Container lives for the duration of the function, then is destroyed.

### Stage-Level Containers (Declarative)

For clarity, stages can declare their container. The Lua script function still runs in the runner, but all `process.run()` calls within that stage automatically execute inside the specified container:

```lua
{
    name = "build",
    container = "rust:latest",
    script = function()
        -- This Lua code runs in the runner
        log.info("Starting build")
        -- Only these process.run() calls execute inside rust:latest container
        process.run({command = "cargo", args = {"build"}})
        process.run({command = "cargo", args = {"test"}})
    end
}
```

All `process.run()` calls in this stage automatically execute inside rust:latest container, but the Lua script itself (logging, variable assignments, conditionals, etc.) runs in the runner process.

### Workspace Sharing

The runner's workspace directory is mounted into every container at `/workspace`. This means:
- Files written by one container are visible to the next
- `git.clone()` in alpine/git writes files
- pytest in python:3.11 reads those same files
- State is shared through filesystem, not memory

### Performance Considerations

Each plugin operation spawns an ephemeral container (~178ms overhead measured with podman on my machine). For typical pipelines with 5-10 plugin calls, total overhead is 1-2 seconds. Build/test operations take minutes, so this is acceptable.

For performance-critical operations, use stage-level containers or explicit `container.run()` blocks to amortize the spawn cost across multiple commands.

### Security Model

Containers provide isolation boundaries. Even if a user tries malicious operations, the container can't escape its mount namespace. The worst case is corrupting the workspace, which is ephemeral per-job.

## Pipeline Definition Changes

The `requires` block should be changed to `plugins` block just to let the runner know it needs to inject those plugins.

Old:
```lua
requires = {"plugin.git", "plugin.slack", "container"}
```

New:
```lua
plugins = {"git", "slack"}
```

Container runtime is mandatory for all runners, so no need to declare it.

## Notes

- Container runtime (Podman or Kubernetes) is now mandatory for runners
- No more capability system - if you have container runtime, you can run anything
- GitHub Actions model: runners just need Docker, everything else comes from images
- Capability system added unnecessary complexity that containerization solves naturally
