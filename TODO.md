# TODO

Centralized tracking of planned work, known issues, and improvement ideas.

## High Priority

### Gameplay Loop Completion

The game needs a complete playable loop to be meaningful:

1. **Add Sink Building** - Consumes end products (circuits, gearboxes), provides a goal
2. **Add Scoring/Progression** - Even simple "deliver X items to win" or point counter
3. **Simple Tech Tree** (optional but valuable):
   - Deliver 50 iron plates → Unlock Smelter
   - Deliver 20 gears → Unlock Assembler
   - Deliver 10 circuits → Unlock next tier

### Test Coverage

High-risk modules lacking tests:

| File | Risk | Notes |
|------|------|-------|
| `workers/tasks/execution.rs` | **High** | Complex logic, drives core behavior |
| `workers/tasks/assignment.rs` | Medium | Task matching logic |
| `structures/placement.rs` | Medium | Building placement validation |
| `structures/production.rs` | Medium | Crafting and logistics |

## Medium Priority

### Resource Generation Clustering

**Location**: `src/resources.rs:56`

Current: 3.8% random ore spawn, no clustering.
Desired: Ore deposits should cluster realistically.

Approaches:
- Noise-based (Perlin/Simplex for ore veins)
- Seed-based clustering (finding ore increases nearby spawn chance)
- Predefined deposit centers

### Storage Overflow Handling

**Location**: `src/workers/tasks/creation.rs:350-379`

When all Storage is full, pickup tasks are despawned without dropoff counterparts.

Options:
- Backpressure: Don't create pickup tasks if no destination
- Overflow state: Drop items on ground
- Warning system: Alert player when storage is critical

### Multi-Cell Buildings

**Location**: `src/structures/construction.rs:159`

Hub (3x3) works, but other multi-cell buildings have issues:
- Placement validation may not check all cells
- Network connectivity may not account for all cells
- Worker pathfinding targets center, may miss entry points

## Low Priority

### Scanning System Polish

**Location**: `src/systems/scanning.rs`

Issues:
- Not pure clockwise sweep (sorts by distance first, then angle)
- `dedup()` may not remove all duplicates (only adjacent ones)

Fix: Use HashSet for deduplication, consider true angular sweep.

### Code Organization

**From TODOs in codebase:**

| Location | Item |
|----------|------|
| `workers/tasks/components.rs:9` | Implement task priority system |
| `materials/items.rs:45` | Add methods for accessing individual item fields |
| `materials/items.rs:55` | Better system for buildings with inputs/outputs |
| `materials/items.rs:62` | Move Inventory to its own module |
| `materials/recipes.rs:18` | Dynamic recipes |
| `systems/network.rs:14` | Determine if NetworkConnection component is needed |
| `ui/building_buttons.rs:8` | Remove Option wrapper |

### UI Test Coverage

UI modules have no test coverage:
- `ui/sidebar.rs`
- `ui/building_buttons.rs`
- `ui/building_menu.rs`
- `ui/interaction_handler.rs`
- `ui/spawn_worker_button.rs`
- `ui/tooltips.rs`
- `ui/sidebar_tabs.rs`
- `ui/production_display.rs`

Bevy UI testing is difficult; this is nice-to-have.

## Ideas (Parking Lot)

Future features to consider:

- **Predictive logistics**: Fetch inputs before buildings run out
- **Bottleneck analysis**: Prioritize tasks that unblock downstream production
- **Worker pre-positioning**: Station workers near high-traffic areas
- **Visual production flow**: Show item movement paths, throughput rates
- **Blueprint system**: Save and stamp building configurations
- **Research tree**: Full tech tree with production-based unlocks
- **Central intelligence**: Workers as extensions of a hive-mind planner
