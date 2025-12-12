# rivet-lua

Shared Lua infrastructure for the Rivet CI/CD system.

## Purpose

This crate provides trait-based abstractions for Lua runtime and core modules, allowing different components (Runner, CLI, Orchestrator) to provide their own implementations while sharing the same Lua interface.

## Key Concepts

### Two-Sandbox Architecture

**Metadata Sandbox** - Safe evaluation of pipeline configuration
- Used by CLI and Orchestrator to parse pipeline definitions
- No I/O or side effects allowed
- Extracts: name, description, inputs, requirements

**Execution Sandbox** - Full pipeline execution with core modules
- Used by Runner to execute stage scripts
- Includes registered core modules (log, env, process, etc.)
- Operations controlled by module implementations

### Trait-Based Modules

Core modules are generic over trait bounds, allowing each component to provide appropriate implementations:

**LogSink trait** - Different logging strategies
- Runner: Buffered logs sent to orchestrator
- CLI: Write to stdout or collect for display
- Orchestrator: Validation-only (no-op or store)

**VarProvider trait** - Different variable sources
- Runner: Job parameters from orchestrator
- CLI: Mock data for local testing
- Orchestrator: Validation context

### Benefits

**Separation of Concerns**
- rivet-lua knows nothing about concrete implementations
- Easy to change sink/provider behavior without touching Lua code
- Each component owns its specific logic

**Flexibility**
- Runner can use buffered logging that flushes on interval
- CLI can use immediate stdout logging
- Easy to add new module types following the same pattern

**Security**
- Metadata sandbox prevents side effects during parsing
- Execution sandbox enforces capability-based access
- Core modules validate all inputs and enforce policy

## Component Usage

**CLI**: Parses pipelines with metadata sandbox, provides mock implementations for local testing

**Orchestrator**: Validates uploaded pipelines with metadata sandbox, never executes stage scripts

**Runner**: Executes stages with execution sandbox, provides real implementations for all core modules