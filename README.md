# The Factory

A sophisticated factory simulation game built in Rust using the Bevy game engine. Features autonomous worker AI, complex production chains, network infrastructure, and dynamic resource management systems.

![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Bevy](https://img.shields.io/badge/bevy-0.15.3-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)

## Overview

**The Factory** is a grid-based factory simulation game that demonstrates advanced game architecture patterns and complex systems coordination. Players build and manage interconnected production facilities, coordinate autonomous workers, and scale their factory operations through progressive automation.

## Technical Highlights

This project showcases production-quality game development practices with several technically impressive systems:

### 1. Advanced Pathfinding System
- **BFS Algorithm**: Breadth-first search implementation for optimal path finding
- **Network-Aware Navigation**: Workers traverse only connected network infrastructure
- **Emergency Displacement**: Automatic relocation of stranded workers using expanding square search patterns
- **Performance Optimized**: Pre-validation of network connectivity before pathfinding execution

### 2. Intelligent Task Assignment System
- **Priority-Based Queuing**: Tasks sorted by Critical → High → Medium → Low priority
- **Multi-Task Sequences**: Support for complex workflows (pickup → transport → dropoff chains)
- **State Machine Pattern**: Clean worker state management (Idle/Working)
- **Orphaned Reference Cleanup**: Automatic validation and cleanup of broken entity references
- **Dynamic Reassignment**: Mid-execution task interruption with proper cleanup

### 3. Network Connectivity System
- **Dual-Layer Network Model**:
  - Core Network: Hubs + Connectors (pathfinding backbone)
  - Extended Network: Core + Adjacent buildings (operational validation)
- **Flood Fill Algorithm**: BFS-based connectivity propagation from network hubs
- **Dynamic Recalculation**: Real-time updates on building placement/removal

### 4. Progressive Scanning System
- **Systematic Exploration**: Reveals 3×3 tile clusters progressively
- **Distance-Scaled Timing**: Scan duration increases with distance to target
- **Angular Progression**: Clockwise scanning pattern for organized map discovery
- **Smart Target Selection**: Prioritizes closest unexplored tiles adjacent to known areas

### 5. Modular Building System
- **Data-Driven Configuration**: RON format for building definitions
- **Component-Based Assembly**: Dynamic component composition at spawn time
  - Power generation/consumption with grid balancing
  - Compute resource management
  - Inventory systems (Storage/Sender/Requester/Producer)
  - Recipe crafters (single or multi-recipe support)
  - Network connectivity components
- **Multi-Cell Support**: Buildings spanning multiple grid cells

### 6. Production & Crafting System
- **Recipe-Based Manufacturing**: Timer-driven crafting with input/output validation
- **Smart Inventory Management**: Transfer validation with capacity limits
- **Automatic Logistics**: Task generation based on inventory state and production requirements
- **Multi-Recipe Crafters**: Buildings can switch between multiple production recipes

### 7. Power & Compute Grid Systems
- **Real-Time Resource Tracking**: Global capacity, usage, and availability monitoring
- **Operational Status Integration**: Only functional buildings contribute to generation
- **Three-Tier Architecture**: Producers → Consumers → Global Grid

### 8. Sophisticated UI System
- **Interactive Framework**: Selection, hover, and click state management
- **Dynamic Styling**: Three-state visual feedback system
- **Exclusive Selection Groups**: Radio button-like behavior for UI elements
- **Context-Aware Menus**: Building-specific interaction panels
- **Real-Time Displays**: Production status and resource visualization

## Architecture

### Plugin-Based Design
The project uses Bevy's plugin architecture for clean module separation:
- **GridPlugin**: Grid-based world with coordinate transformations
- **ResourcesPlugin**: Global resource and asset management
- **MaterialsPlugin**: Items, recipes, and inventory systems
- **SystemsPlugin**: Infrastructure (power, compute, network, scanning)
- **BuildingsPlugin**: Building lifecycle and production
- **WorkersPlugin**: Worker AI, pathfinding, and task execution
- **UIPlugin**: User interface and interaction handling
- **CameraPlugin**: Camera controls and viewport management

### System Set Ordering
Precise control over system execution order:
```
GridUpdate → ResourceSpawning → SystemsUpdate → DomainOperations → UIUpdate
```

### Key Design Patterns
- **Entity-Component-System (ECS)**: Bevy's core paradigm, heavily leveraged throughout
- **Event-Driven Architecture**: Decoupled systems communicating via events
- **State Machines**: Worker states, task statuses, UI selection behavior
- **Data-Driven Design**: Configuration-based building and recipe definitions
- **Validation Pipeline**: Request → Validate → Execute pattern for all state changes

## Gameplay Features

- **Grid-Based Building**: Place structures on a 64px grid system
- **Worker Management**: Spawn and coordinate autonomous carrier workers
- **Production Chains**: Mining → Smelting → Manufacturing progression
- **Network Infrastructure**: Connect buildings with hubs and connectors for logistics
- **Resource Management**: Balance power generation and compute allocation
- **Map Exploration**: Deploy scanners to progressively reveal the world
- **Logistics Automation**: Configure sender/requester/storage buildings for automatic material flow

## Technologies

- **[Bevy 0.15.3](https://bevyengine.org/)**: ECS game engine
- **[Rust 2021 Edition](https://www.rust-lang.org/)**: Systems programming language
- **[Serde](https://serde.rs/)**: Serialization framework for data-driven configuration
- **Rand**: Randomization utilities

## Building and Running

### Prerequisites
- Rust 1.70 or later
- Cargo (comes with Rust)

### Build Instructions
```bash
# Clone the repository
git clone https://github.com/yourusername/the_factory.git
cd the_factory

# Run in development mode
cargo run

# Build optimized release
cargo build --release
```

### Performance Notes
The project uses custom optimization profiles for faster development iteration:
- Dev profile: Level 1 optimization for quick compilation
- Dev dependencies: Level 3 optimization for reasonable runtime performance

## Project Structure

```
src/
├── main.rs              # Application entry point and plugin configuration
├── camera.rs            # Camera controls and viewport management
├── constants.rs         # Game constants and configuration values
├── grid.rs              # Grid system and coordinate transformations
├── resources.rs         # Global resources and asset management
├── materials/           # Items, recipes, and inventory
│   ├── items.rs
│   ├── recipes.rs
│   └── mod.rs
├── structures/          # Building system
│   ├── building_config.rs
│   ├── construction.rs
│   ├── placement.rs
│   ├── production.rs
│   ├── validation.rs
│   └── mod.rs
├── systems/             # Infrastructure systems
│   ├── compute.rs
│   ├── display.rs
│   ├── network.rs
│   ├── operational.rs
│   ├── power.rs
│   ├── scanning.rs
│   └── mod.rs
├── workers/             # Worker AI and task management
│   ├── pathfinding.rs
│   ├── spawning.rs
│   ├── tasks/
│   │   ├── assignment.rs
│   │   ├── components.rs
│   │   ├── creation.rs
│   │   ├── execution.rs
│   │   └── mod.rs
│   └── mod.rs
└── ui/                  # User interface
    ├── building_buttons.rs
    ├── building_menu.rs
    ├── interaction_handler.rs
    ├── production_display.rs
    ├── sidebar.rs
    ├── sidebar_tabs.rs
    ├── spawn_worker_button.rs
    ├── tooltips.rs
    └── mod.rs
```

## Code Quality

- **Type Safety**: Strong typing prevents common runtime errors
- **Error Handling**: Comprehensive validation and error reporting
- **Entity Safety**: Validation checks prevent orphaned entity references
- **Documentation**: Clear naming conventions and code organization

## Development Status

Version 0.1.3 - Active Development

This project is a demonstration of advanced game development concepts and serves as a portfolio piece showcasing:
- Complex systems design and coordination
- ECS architecture patterns
- Real-time resource management
- Autonomous agent AI implementation
- Large-scale Rust application development

## License

MIT License - See LICENSE file for details

## Contributing

This is currently a personal portfolio project. Feel free to fork and experiment, but please note that pull requests may not be actively reviewed.

## Contact

For questions or discussions about the technical implementation, feel free to open an issue.

---

**Built with** ❤️ **and Rust**