# The Factory

A Bevy-based factory management simulation game where players construct production facilities, manage workers, and optimize production chains on an infinite procedurally-generated grid.

## Features

- **Infinite Grid World**: Procedurally-generated terrain with ore deposits (iron, copper, coal)
- **Building System**: Construct miners, smelters, assemblers, storage, and infrastructure
- **Worker Management**: Autonomous workers handle logistics, construction, and material transport
- **Production Chains**: Define recipes to transform raw materials into complex products
- **Infrastructure Systems**: Power grids, compute networks, and connectivity management
- **Progressive Exploration**: Radar-based scanning reveals the world in expanding arcs

## Requirements

- Rust (stable, see `rust-toolchain.toml`)
- Cargo

## Quick Start

```bash
# Clone and enter the repository
git clone <repo-url>
cd the_factory

# Set up git hooks (required for development)
git config core.hooksPath .githooks

# Build and run
cargo run
```

## Development

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
cargo test --all-targets

# Code quality (enforced by pre-commit hooks)
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo deny check
```

### Git Hooks

Pre-commit hooks enforce:
- `cargo fmt --check`
- `cargo clippy` (pedantic, strict error handling)
- `cargo test`
- Blocks `dbg!()` macros
- Blocks legacy/fallback code comments

## Architecture

### System Execution Order

The game loop runs in strict sequence via Bevy system sets:

```
GridUpdate → ResourceSpawning → SystemsUpdate → DomainOperations → UIUpdate
```

### Module Structure

| Module | Purpose |
|--------|---------|
| `grid` | 2D coordinate system, cell management, world/grid conversion |
| `resources` | Procedural ore spawning |
| `materials` | Item registry, recipes, inventory system |
| `structures` | Building definitions, placement, construction, production |
| `systems` | Power grid, compute, network connectivity, scanning |
| `workers` | Worker spawning, pathfinding, task management |
| `ui` | Sidebar, menus, tooltips, building placement |
| `camera` | Camera controls |

### Game Data (RON Assets)

Game content is defined in `src/assets/`:
- `items.ron` - Item definitions with tiers
- `recipes.ron` - Crafting recipes (inputs, outputs, duration)
- `buildings.ron` - Building definitions with components

## Key Concepts

- **Network Connectivity**: Buildings must be connected (adjacent or via Connectors) for workers to pathfind between them
- **Operational Status**: Buildings require sufficient power/compute from their connected network to function
- **Construction**: Buildings require materials and time; workers deliver construction materials
- **Scanning**: Radar buildings progressively reveal the grid in a clockwise pattern

## License

[Add license information]
