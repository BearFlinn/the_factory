# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**the_factory** is a Bevy-based factory management simulation game written in Rust. Players construct production facilities, manage workers, and optimize production chains on an infinite procedurally-generated grid.

## Build & Development Commands

```bash
# Build
cargo build                              # Debug build
cargo build --release                    # Release build

# Run
cargo run                                # Launch the game

# Test
cargo test                               # Run all tests
cargo test --all-targets                 # Include doc and integration tests

# Code Quality (enforced by pre-commit hooks)
cargo fmt                                # Format code
cargo clippy --all-targets -- -D warnings  # Lint check (strict)
cargo deny check                         # Dependency security audit

# Git hooks setup (required for development)
git config core.hooksPath .githooks
```

Pre-commit hooks run fmt, clippy, and tests. Pre-push runs full test suite. Commits block on `dbg!()` macros and legacy/fallback code comments.

## Architecture

### System Execution Order

The game loop uses Bevy system sets that run in strict sequence (defined in `main.rs`):

```
GameplaySet::GridUpdate         → Grid cells, coordinates
GameplaySet::ResourceSpawning   → Ore node generation
GameplaySet::SystemsUpdate      → Power, compute, network, scanning
GameplaySet::DomainOperations   → Buildings, workers, tasks
GameplaySet::UIUpdate           → Rendering, interaction
```

### Module Structure

Each major subsystem is a Bevy Plugin:

- **GridPlugin** (`grid.rs`) - 2D coordinate system, cell management, world↔grid conversion
- **ResourcesPlugin** (`resources.rs`) - Procedural ore spawning (iron, copper, coal)
- **MaterialsPlugin** (`materials/`) - Item registry, recipes, inventory system
- **BuildingsPlugin** (`structures/`) - Building definitions, placement, construction, production
- **SystemsPlugin** (`systems/`) - Infrastructure: power grid, compute, network connectivity, scanning
- **WorkersPlugin** (`workers/`) - Worker spawning, pathfinding, task management
- **UIPlugin** (`ui/`) - Sidebar, menus, tooltips, building placement UI
- **CameraPlugin** (`camera.rs`) - Camera controls

### Task System (`workers/tasks/`)

The worker task system is the most complex subsystem, with its own system set ordering:

```
TaskSet::Interrupts   → Cancel/preempt tasks
TaskSet::Assignment   → Match workers to tasks (BFS-based)
TaskSet::Processing   → Execute current tasks
TaskSet::Generation   → Create new tasks from building needs
TaskSet::Cleanup      → Remove completed tasks
```

Task types: logistics (item transfer), construction, proactive gathering.

### Game Content (RON Assets)

Game data is defined in `src/assets/` using RON format:
- `items.ron` - Item definitions with tiers
- `recipes.ron` - Crafting recipes (inputs, outputs, time)
- `buildings.ron` - Building definitions with components

Buildings are composed of components: `PowerConsumer`, `ComputeGenerator`, `RecipeCrafter`, `Inventory`, `Scanner`, etc.

## Key Concepts

- **Network Connectivity**: Buildings must be connected (adjacent or via Connectors) for workers to pathfind between them
- **Operational Status**: Buildings require sufficient power/compute from connected network to function
- **Construction**: Buildings require materials and time; workers deliver construction materials
- **Scanning**: Radar buildings progressively reveal the grid in a clockwise pattern

# Workflow Requirements

## Branch Strategy

**Always create a new branch before starting work.** Never commit directly to `main`.

```bash
git checkout -b <descriptive-branch-name>
```

Branch names should describe the work: `feature/scanner-range`, `fix/worker-pathfinding`, `refactor/task-system`.

## Task Execution

**Prefer parallel sub-agents over direct implementation.** When a task involves multiple independent operations, launch multiple Task agents in parallel rather than executing sequentially. This applies to both research and implementation work.

Examples:
- Implementing changes across multiple unrelated modules
- Writing tests while implementing features (when test structure is clear)
- Refactoring several independent files
- Running searches, audits, or exploration in parallel
- Any work that can be decomposed into independent subtasks

## Completion Requirements

Before providing a completion summary to the user:
1. All changes must be committed
2. Branch must be merged to `main` (or PR created if requested)
3. All changes must be pushed to remote

Never leave work in an uncommitted or unpushed state.

# Commit Requirements, Linting, and Formatting. 

## Git Hooks

Pre-commit hooks enforce quality gates:
- **pre-commit**: `cargo fmt --check`, `cargo clippy`, `cargo test`, blocks `dbg!()`, blocks legacy/fallback comments
- **commit-msg**: validates message format
- **pre-push**: full test suite

Bypass is **FORBIDDEN**.

## Lint Rules

Clippy pedantic is enabled with strict error handling:
- `unsafe_code` - forbidden
- `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` - **denied**
- `missing_errors_doc`, `missing_panics_doc`, `must_use_candidate` - warnings

Test modules have `#[allow(clippy::unwrap_used)]` for readability.

**ALWAYS** request explicit approval from the user before changing this config or adding clippy allow macros.

# Style Guidelines

## Naming
- **Domain-specific names**: Prefer descriptive names that match the domain (`send_chat_completion` over generic `run`)
- **Common abbreviations OK**: `cfg`, `dir`, `msg`, `ctx`, `cmd` are fine; avoid obscure ones

## Error Messages
- Always include context: `"failed to parse config at {path}"` not just `"parse error"`
- Lowercase, no trailing period (Unix style, chains well with `anyhow` context)

## Comments
- Comments should be minimally used. Over-commented code reads as bloated.
- Assume contributors are intelligent and can reason through basic logic without comments.
- Function over form. Pretty comments waste everyone's time. 
- **DO NOT** leave comments like "Validates data" on a function called `validate_data`, or "Implements Car" on `impl Car`.

## Module Organization
- Group related types in one file (e.g., `Message`, `Role`, `ToolCall` together in `llm/mod.rs`)
- Tests: unit tests in `#[cfg(test)] mod tests` at file bottom; integration tests in `tests/`

## Visibility
- Private-first: start with no visibility modifier, add `pub(crate)` or `pub` only when needed
- Treat `pub` as a commitment — once public, it's API

## Function Signatures
- **Strings**: `&str` for read-only, `impl Into<String>` when storing, owned `String` when caller must give up ownership
- **Async**: async-first; only use sync for trivial or CPU-bound operations
- **Generics**: default to concrete types, generify at public API boundaries when flexibility is needed

## Construction
- Prefer `new()` with required args + `Default` trait for optional configuration
- Avoid builder pattern unless struct has many optional fields

## Logging (tracing)
- **error**: failures that stop an operation
- **warn**: recoverable issues, degraded behavior
- **info**: major operations (agent loop start/end, tool execution)
- **debug**: internal details, state transitions
- **trace**: verbose diagnostics (full payloads, timing)
- Use structured fields: `info!(tool = %name, "executing tool")` not string interpolation

# Refactoring Policy

**Complete refactors fully. No hybrid states.**

When refactoring or replacing a system:
- **Remove the old code entirely** — don't leave it commented out or behind a flag
- **No fallback logic** — if the new code breaks, it should break loudly
- **No "just in case" preservation** — this codebase is small enough to revert via git if needed
- **No migration scaffolding** — replace in place, don't layer new on old

Comments like these are **FORBIDDEN** and will be blocked by pre-commit hooks:
- `// legacy`, `// old system`, `// fallback`
- `// keep for now`, `// just in case`
- `// TODO: remove`, `// deprecated but kept`
- `// backward compat`, `// preserved for`

If you're uncertain about removing old code, **ask first** rather than keeping both systems. Hybrid states make the refactor impossible to test and validate.

# Misc Notes
- Testing is a first class operation, NEVER skip test implementation.
- Commits should be made frequently, especially for large multi-phase tasks.
