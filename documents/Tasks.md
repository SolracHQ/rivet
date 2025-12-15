# Tasks

Just a simple place to put my ideas.

**WARNING:** This is an experimental/pre-alpha project. Do NOT use in production. Things break, APIs change, features disappear. You've been warned.

## Current State

Rivet is a CI/CD system where pipelines are Lua scripts and everything runs in containers. Pipelines are fully Lua-defined, parsed and validated at creation time. The CLI now has interactive input collection with type validation. Conditional stages work properly. It's getting less bare bones.

**What's actually working:**
- Orchestrator API (create pipelines, launch jobs, view logs)
- Runner polls orchestrator, executes jobs
- Pipelines defined in Lua with two APIs:
  - `pipeline.define({...})` - declarative table structure
  - `pipeline.builder():name(...):stage(...)...` - fluent builder API
- Pipeline validation with LSP support (stub file provides autocomplete)
- Input system with proper types (string, number, bool)
- Input validation (required fields, default values, enum options)
- Interactive CLI input collection (prompts for missing inputs with validation)
- Conditional stages (stages can have condition functions that determine if they run)
- Core modules: log, input, process, container
- Container stack (container.with() pushes/pops from stack in runner, not nested Podman)
- CLI for basic operations (create, launch, list, logs, check)
- Podman integration (spawn containers on demand in runner process)
- Default Alpine container per job
- Workspace sharing between containers via /workspace mount

**Current bugs/annoyances:**
- Error messages could be better (but improved from before)
- No way to see what's happening without tailing logs manually
- Container images with custom entrypoints need workarounds

## What I'm Focusing On Now

Building out the core modules so pipelines can actually do useful things. Right now you can log and run commands, but you can't read files properly, make HTTP calls, or use secrets. Need to fix that before anything else makes sense.

**Immediate work:**
- Filesystem module (read/write files in workspace, jailed operations)
- HTTP module (make requests, probably with rate limiting)
- Secret module (store secrets, inject into pipelines, redact from logs)

## Wishlist (May or May Not Happen)

Features I want but might never implement because life is short and this is a toy project:

- **Kubernetes runner support** - Run jobs as K8s pods instead of Podman containers. Mounting workspace across pods is annoying (EFS? NFS? who knows). This would be killer for production use but it's a massive headache.
- **Web UI** - Dashboard to see pipelines, watch logs live, launch jobs. I suck at design so it'll probably look terrible but at least people can click buttons instead of CLI.
- **Plugin marketplace** - Let people share plugins. Probably overkill but would be cool.
- **Distributed tracing** - See exactly what happened across orchestrators/runners. OpenTelemetry integration maybe?
- **Caching layer** - Cache dependencies, build artifacts, whatever. Redis-backed probably.
- **Webhook triggers** - GitHub pushes trigger pipeline runs automatically. Not hard but not priority.
- **Pipeline templates** - Reusable pipeline fragments. DRY for CI/CD configs.
- **Container build/push in core module** - Use runner's Podman to build, tag, and push images. Avoids Podman-in-Podman mess. Would be in container module, not a plugin.

## Feature Roadmap (Loose Priority Order)

These are what I think I'll actually build, roughly in this order, but who knows what I'll feel like doing tomorrow:

### 1. Core Modules (Security Edition)

The current modules work but have zero security. Need to lock them down:

- **Process module security:**
  - Whitelist/blacklist for allowed binaries (no random curl | bash nonsense)
  - Resource limits (memory, CPU) per process
  - Timeout enforcement

- **Container module security:**
  - Image whitelist/blacklist (or registry restrictions)
  - Pull policy enforcement (always verify signatures?)
  - Network isolation controls
  - Maybe add build/tag/push support (using runner's Podman, not Podman-in-Podman)

- **Secret module:**
  - Encrypted storage in PostgreSQL
  - Access control (which pipelines can read which secrets)
  - Automatic redaction in logs (pattern matching for API keys, tokens, etc.)
  - Secret injection via environment variables or files

- **Filesystem module security:**
  - Workspace jailing (can't escape /workspace)
  - Read-only mode for sensitive stages
  - File size limits (no filling up disk)

### 2. Input Module Improvements

Input system is solid now with proper validation:

- [x] Booleans (true/false, yes/no, 1/0)
- [x] Numbers (validated as f64)
- [x] Strings
- [x] Enums via options field (pick from allowed values)
- [x] Required vs optional with defaults
- [x] Interactive CLI prompts with validation
- [x] Type checking in orchestrator before job creation

Still want:
- Arrays/lists (multiple values)
- Validation rules (regex, ranges, custom validators)

### 3. Plugin System

Turn common operations into clean APIs. Plugins use containers internally but users don't care:

- **Git plugin** - clone, checkout, commit, push (using alpine/git container)
- **Archive plugin** - tar/zip operations for artifacts
- **Terraform plugin** - plan, apply (using terraform container)

Plugins should be easy to write. Probably just Lua modules that get injected into sandbox with access to core modules.

**Note:** No webhooks/notification plugins (Teams, Slack, etc.). HTTP module exists, people can write their own if they need it. I'm not maintaining that.

### 4. Security Context

Can't have multiple users without auth. Need runner authentication and user authentication:

- **Runner registration:**
  - Admin creates registration token
  - Runner uses token to register and gets permanent token
  - All runner API calls require valid token
  - Runners live wherever they want, just need to poll orchestrator
  - This allows runners in intranets that can't be reached by orchestrator (like GitHub Actions)

- **User authentication:**
  - Simple username/password (hashed with argon2 or whatever)
  - JWT tokens for API access
  - CLI stores token in ~/.rivet/config
  - API key alternative for automation

- **Authorization:**
  - Basic RBAC (admin, user, viewer roles)
  - Pipeline permissions (who can launch what)
  - Secret permissions (who can read which secrets)
  - Runner permissions (which runners can execute which pipelines)

### 5. High Availability / Resilience

Right now orchestrator and PostgreSQL are single points of failure. My DevOps soul screams at this:

- **Multiple orchestrators:**
  - All orchestrators are stateless (state lives in PostgreSQL)
  - Use PostgreSQL as distributed job queue with row locks
  - `SELECT FOR UPDATE SKIP LOCKED` for job claiming
  - Load balancer (ALB/nginx) in front of orchestrators
  - Runners can hit any orchestrator instance via load balancer

- **Job reliability:**
  - Detect stale jobs (runner crashed, orchestrator died)
  - Automatic retry with exponential backoff
  - Job timeout enforcement
  - Orphaned container cleanup

- **PostgreSQL HA:**
  - Not implementing myself, just accept reader/writer endpoints
  - Auto-scaling is Aurora/RDS problem, not mine
  - Connection pooling (pgbouncer?)
  - For tests, SPOF is fine. I'm just presenting tools to prevent it.

### 6. Kubernetes Runner Support

Big one. Running jobs as K8s pods instead of Podman containers:

- **Pod execution:**
  - Each job runs in a pod
  - Containers are pod containers (not nested Podman)
  - Workspace sharing via PVC (probably needs ReadWriteMany - EFS on AWS, NFS, or Ceph)

- **Challenges:**
  - Volume mounts across multiple containers in a pod (doable)
  - Container stack in pod (each container.with() adds sidecar? or new pod?)
  - Performance (pod startup is slower than Podman)
  - Cleanup (delete pods after job completes)

- **Benefits:**
  - Run Rivet in Kubernetes clusters
  - Use K8s resource management (limits, requests)
  - Auto-scaling runners (HPA)
  - Better for production deployments

**Architecture decision:** Keeping polling model. Runners poll orchestrator, not the other way around. This allows runners in intranets/private networks that can't be reached by orchestrator. Easier architecture for me, same model as GitHub Actions.

### 7. Observability

Need to see what's happening without SSHing into things:

- **Better logging:**
  - Structured logs (JSON) from orchestrator/runner
  - Log levels configurable per component
  - Log aggregation (stdout to CloudWatch/Loki/whatever)

- **Metrics:**
  - Prometheus metrics (job count, duration, success rate)
  - Runner metrics (CPU, memory, active jobs)
  - Queue depth, job latency

- **Live log streaming:**
  - SSE endpoint for real-time log streaming
  - CLI can `rivet job logs --follow <id>`
  - Web UI (if it exists) shows live logs

### 8. Web UI (Maybe)

If I get motivated or people complain about CLI-only:

- **Pages needed:**
  - Dashboard (active jobs, recent completions, failed jobs)
  - Pipeline list (with launch button)
  - Job detail (status, logs, artifacts)
  - Runner list (active runners, status)

- **Technology:**
  - Probably htmx (server-rendered, less JS bullshit)
  - Or just vivecoded React or even flutter web
  - Will look ugly but functional

- **Features:**
  - Live log streaming (SSE or WebSocket)
  - Launch pipelines with input parameters
  - View job history
  - Maybe edit pipelines? (scary)

## Implementation Notes

**Container execution model:**
- Lua code runs in runner process, NOT in containers
- Only `process.run()` calls execute inside containers
- State (variables, plugin instances) lives in runner memory
- Containers are execution contexts, not isolation for Lua
- Container stack lives in runner (push/pop with container.with()), not nested Podman

**Workspace:**
- Runner creates workspace directory per job
- Mounted at /workspace in all containers
- Persists across container spawns (shared via filesystem)
- Cleaned up after job completes

**Podman specifics:**
- Using Podman (not Docker) because it's daemonless
- Using podman command-line (not API) for simplicity
- Containers named with image hash to avoid collisions
- Entrypoint override for images with custom entrypoints
- Default image: alpine:latest (configurable via env var)
- Containers spawn in runner process, stack managed in runner
- Prefer "container" and "containerfile" terminology over "docker" and "dockerfile"

**Architecture decisions:**
- Orchestrator is stateless (all state in PostgreSQL)
- Runners poll orchestrator (no push model, allows intranet runners)
- Jobs are immutable once created (no editing)
- Logs are append-only (no truncation)
- Load balancer (ALB/nginx) goes in front of orchestrators, not my problem

**Things I'll probably change my mind about:**
- Runner auto-registration vs manual token creation
- Plugin API design (might need versioning)
- Secret storage (maybe use Vault instead of PostgreSQL?)
- K8s support architecture (pods vs jobs vs deployments?)
- Web UI framework (htmx vs React vs just skip it)

## Other Ideas

- [x] Stage-level container declarations (`container = "rust:latest"` in stage definition)
- [x] Conditional stages (custom condition functions per stage)
- Artifacts storage (save build outputs, make available to later stages)
- Pipeline composition (call other pipelines as stages)
- Approval gates (pause pipeline, wait for human approval)
- Scheduled pipelines (cron-style triggers)
- Pipeline versioning (track changes over time)
- Rollback mechanism (revert to previous pipeline version)
- generic parallel execution `process.parallel(items, fn)` (runner executes several process.run() in parallel, collects results)
