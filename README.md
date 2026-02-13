# You Are The Factory

A factory management simulation built with Bevy, where players construct production facilities, manage workers, and optimize production chains on an infinite procedurally-generated grid.

## Overview

You Are The Factory is a personal project built to sharpen Rust and system design skills. The game drops you onto a procedurally-generated grid with ore deposits and a single Hub building. From there, you expand outward: mining raw materials, smelting them into ingots, assembling components, and launching finished goods for score.

Current state: playable core loop with 10 buildings, 10 items across 4 tiers, 21 recipes, and automated worker logistics. ~6,000 lines of Rust on Bevy 0.18.

## Gameplay

### Core Loop

**Explore** — Radar buildings progressively scan outward, revealing ore deposits in a clockwise sweep.

**Extract** — Mining Drills placed on ore nodes produce raw materials (iron, copper, coal).

**Process** — Smelters convert ores into ingots. Generators burn coal to power the network.

**Manufacture** — Assemblers craft components from processed materials through multi-tier recipes.

**Launch** — Launchpads consume items and convert them to score. Higher-tier items score exponentially more: `10 * (tier + 1)^2` points per launch.

### Buildings

| Building | Function | Requirements |
|---|---|---|
| Hub | Starting building, network anchor, spawns workers | Placed at game start |
| Mining Drill | Extracts ore from deposits | Must be on a resource node, adjacent to network |
| Generator | Burns coal to produce power | Adjacent to network |
| Datacenter | Generates compute capacity | Power, adjacent to network |
| Smelter | Converts ores + coal into ingots | Power, adjacent to network |
| Assembler | Crafts components from processed materials | Power, adjacent to network |
| Storage | Buffers items (200 capacity) | Adjacent to network |
| Connector | Extends the building network | Adjacent to network |
| Radar | Scans and reveals the grid | Power, compute, adjacent to network |
| Launchpad | Launches items for score | Power, adjacent to network |

### Production Tiers

| Tier | Items | Source |
|---|---|---|
| 0 | Iron Ore, Copper Ore, Coal | Mining Drills |
| 1 | Iron Ingot, Copper Ingot | Smelter |
| 2 | Gear, Copper Wire, Iron Plate | Assembler |
| 3 | Gearbox, Electronic Circuit | Assembler |

### Network Connectivity

Buildings must be connected through adjacency or Connectors to form a network. Workers can only pathfind between buildings on the same network. Power, compute, and operational status propagate through connected buildings.

## Architecture

### System Execution Order

The game loop runs through five ordered phases per frame:

```
GameplaySet::GridUpdate         → Grid cells, coordinates
GameplaySet::ResourceSpawning   → Procedural ore generation
GameplaySet::SystemsUpdate      → Power, compute, network, scanning
GameplaySet::DomainOperations   → Buildings, workers, workflows
GameplaySet::UIUpdate           → Rendering, interaction
```

Each phase contains its own ordered sub-sets. For example, `SystemsUpdate` chains `Infrastructure → Operational → Display`, and `DomainOperations` chains `Lifecycle → TaskManagement → Movement → Interaction`.

### Plugin Architecture

The codebase is organized as eight Bevy plugins, each owning its domain:

- **GridPlugin** — 2D coordinate system, cell management, world-to-grid conversion
- **ResourcesPlugin** — Procedural ore deposit spawning (Perlin-style distribution)
- **MaterialsPlugin** — Item registry, recipe registry, inventory system
- **SystemsPlugin** — Power grid, compute grid, network connectivity, scanning
- **BuildingsPlugin** — Building definitions, placement validation, construction, production
- **WorkersPlugin** — Worker spawning, pathfinding, movement, workflow execution
- **CameraPlugin** — Pan and zoom controls
- **UIPlugin** — Sidebar panels, building placement, tooltips, status displays

### Data-Driven Content

All game content is defined in RON asset files rather than hardcoded:

- `items.ron` — Item definitions with tier assignments
- `recipes.ron` — Crafting recipes (inputs, outputs, crafting time)
- `buildings.ron` — Building definitions composed from reusable components

Buildings are assembled from components (`PowerConsumer`, `PowerGenerator`, `ComputeGenerator`, `RecipeCrafter`, `Scanner`, `InputPort`, `OutputPort`, `StoragePort`, etc.), making new buildings trivial to add without code changes.

### Worker Workflow System

Workers operate through a multi-phase workflow system:

```
WorkflowSystemSet::Management   → Create/delete/pause workflows, assign workers
WorkflowSystemSet::Processing   → Execute current workflow steps
WorkflowSystemSet::Arrivals     → Handle workers arriving at destinations
WorkflowSystemSet::Waiting      → Recheck waiting workers (items available / space freed)
WorkflowSystemSet::Cleanup      → Clean up invalid references, emergency dropoffs
```

Workers are assigned to workflows that define pickup and dropoff patterns between buildings. They pathfind using BFS across the building network and automatically wait when source buildings have no items or destination buildings are full.

### Runtime Invariant Checking

In debug builds, an `InvariantPlugin` runs every frame in `PostUpdate` and validates:

- Workers have all required components (Speed, Position, Path, Cargo, Transform)
- Buildings have all required components (Name, Position, Operational, Transform)
- Workflow assignment references point to live entities
- No worker has conflicting wait states simultaneously
- Construction sites have required structural components

In tests, invariant violations panic immediately. In dev builds, they log errors.

## Testing

**206 unit tests** covering individual systems, registries, pathfinding, and component logic.

**22 integration scenarios** exercising the full Bevy system loop across four categories:

| Category | Coverage |
|---|---|
| Network | Connectivity, pathfinding, operational status propagation |
| Production | Crafting pipelines, recipe execution, output generation |
| Logistics | Worker workflows, item transfers, waiting states |
| Construction | Building placement, material delivery, construction completion |

The integration test harness (`tests/integration/harness/`) provides:

- **`headless_app()`** — Full gameplay loop without rendering (includes `InvariantPlugin`)
- **Entity builders** — `spawn_building`, `spawn_worker` using the real `BuildingRegistry`
- **Time control** — Deterministic frame advancement via `tick()`, `tick_n()`, `tick_seconds()`, `tick_until()`
- **Domain assertions** — `assert_operational`, `assert_worker_at`, etc. with entity context on failure

## Code Quality

**Clippy pedantic** with strict error handling:
- `unsafe_code` — forbidden
- `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` — denied
- Full pedantic lint set as warnings

**Git hooks** enforce quality on every commit:
- `pre-commit` — `cargo fmt --check`, `cargo clippy`, `cargo test`, blocks `dbg!()` macros
- `commit-msg` — validates message format
- `pre-push` — full test suite

**Dependency auditing** via `cargo-deny` checks for known vulnerabilities and license compliance.

## Getting Started

### Prerequisites

- Rust (stable toolchain)
- `cargo-deny` (`cargo install cargo-deny`)

### Build and Run

```bash
cargo run                    # Launch the game
cargo build --release        # Optimized build
```

### Development

```bash
git config core.hooksPath .githooks    # Enable git hooks (required)

cargo test                             # Run all tests
cargo clippy --all-targets -- -D warnings
cargo deny check
```

The dev profile uses `opt-level = 1` for game code and `opt-level = 3` for dependencies, balancing compile times with playable frame rates during development.

## Project Goals

This project exists to build real competence in Rust and ECS architecture through a non-trivial system design problem. The factory simulation domain forces engagement with ownership patterns, system ordering, event-driven architecture, and data-driven design in ways that smaller projects don't.
