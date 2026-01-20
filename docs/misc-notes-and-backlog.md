# Miscellaneous Notes & Backlog

Lower priority items to revisit after the inventory/production refactor is complete.

---

## Scanning System

### Current Behavior

**Location**: `src/systems/scanning.rs`

The scanner uses **sector-based scanning** inspired by Factorio's radar:
- World divided into sectors on a grid (default: 5x5)
- Scanner picks unexplored sectors adjacent to explored ones
- Each scan reveals the full sector area (5x5 with default settings)
- Sectors prioritized by distance from scanner, then clockwise angle

### Configuration

Single configurable field:
- `sector_size: i32` - Both grid spacing AND reveal size (default: 5)

---

## Resource Generation

### Current Implementation

**Location**: `src/resources.rs`

- 3.8% chance to spawn ore on new grid cells
- Three ore types: Iron, Copper, Coal with weighted probabilities
- Purely random placement—no clustering

### Desired Improvement

**TODO at line 56**: Add clustering to ore spawning

Real ore deposits tend to cluster—finding iron should suggest more iron nearby. The current random spawn makes the world feel scattered and unrealistic.

### Potential Approaches

1. **Noise-based**: Use Perlin/Simplex noise to create ore "veins"
2. **Seed-based clustering**: When ore spawns, increase spawn chance for same type in adjacent cells
3. **Predefined deposits**: Generate deposit centers, ore spawns based on distance to nearest deposit

### Priority

**Medium** - Affects game feel significantly, but doesn't block basic gameplay loop.

---

## Multi-Cell Buildings

### Current State

**Location**: `src/structures/construction.rs`

The Hub is implemented as a 3×3 multi-cell building and works. Other multi-cell buildings don't work reliably.

**TODO at line 175**: "Improve Multi-cell building implementation"

### Issues

- Building placement validation may not correctly check all cells
- Network connectivity calculations may not account for all cells
- Worker pathfinding targets building position (center), may not work for all entry points

### Priority

**Low-Medium** - Only matters if you want more large buildings. Current single-cell buildings work fine.

---

## Storage Handling

### Known Issue

When all Storage buildings are full, `find_closest_storage_receiver` returns `None`, and pickup tasks are despawned without dropoff counterparts. Items have nowhere to go.

**Location**: `src/workers/tasks/creation.rs:350-379`

### Potential Fixes

1. **Overflow handling**: Create a special "overflow" state or drop items on ground
2. **Backpressure**: Don't create pickup tasks if there's nowhere to put items
3. **Storage priority**: Prefer emptier storage, spread load
4. **Warning system**: Alert player when storage is critically full

### Relationship to Refactor

With the buffer-based system, this becomes "what happens when an output buffer is full and there's no downstream capacity?" The answer might be:
- Production pauses (backpressure)
- Items queue in the buffer
- Player is notified to build more storage or sinks

---

## Gameplay Loop

### Discussion Summary

Factory games have three interlocking loops:

**Immediate Loop** (moment-to-moment):
```
Identify bottleneck → Build/adjust to fix → Watch throughput improve → New bottleneck
```
For this to work, production chains must FLOW reliably. The jam bug breaks this.

**Progression Loop** (the "why"):
```
Build toward goal → Achieve milestone → Unlock new capabilities → New goal
```
Currently missing—there's no sink, no research, no win condition. You make items but there's no purpose.

**Expansion Loop** (the "where"):
```
Scan territory → Find resources → Claim space → Repeat
```
Scanning system provides this, functional enough.

### Minimum Viable Loop

To have a *complete* playable loop:

1. **Fix jam bug** so production chains work reliably
2. **Add a Sink building** that consumes end products (circuits, gearboxes)
3. **Add scoring/progression** - even just "deliver X items to win" or a point counter

This creates: mine → smelt → assemble → deliver → see progress → expand to go faster

### Richer Version

Add a simple tech tree:
- Deliver 50 iron plates → Unlock Smelter
- Deliver 20 gears → Unlock Assembler
- Deliver 10 circuits → Unlock [next tier]

Now expansion has purpose—you're unlocking capabilities.

---

## UI System

### Current State

The UI is functional but has **no test coverage**:
- `src/ui/sidebar.rs`
- `src/ui/building_buttons.rs`
- `src/ui/building_menu.rs`
- `src/ui/interaction_handler.rs`
- `src/ui/spawn_worker_button.rs`
- `src/ui/tooltips.rs`
- `src/ui/sidebar_tabs.rs`
- `src/ui/production_display.rs`

Bevy UI testing is notoriously difficult, so this is somewhat expected.

### Priority

**Low** - UI works, testing is nice-to-have but not blocking.

---

## Code Quality Notes

### Test Coverage Gaps (High Risk)

| File | Risk Level | Notes |
|------|------------|-------|
| `workers/tasks/execution.rs` | **High** | Complex logic, drives core behavior |
| `workers/tasks/assignment.rs` | Medium | Task matching logic |
| `structures/placement.rs` | Medium | Building placement validation |
| `structures/production.rs` | Medium | Crafting and logistics |
| `workers/spawning.rs` | Low | Simple spawning |

### TODOs in Codebase

| Location | Item |
|----------|------|
| `resources.rs:56` | Add clustering to ore spawning |
| `structures/construction.rs:175` | Improve multi-cell building implementation |
| `workers/tasks/components.rs:9` | Implement priority system |
| `materials/items.rs:45` | Add methods for accessing individual item fields |
| `materials/items.rs:55` | Better system for buildings with inputs/outputs |
| `materials/items.rs:62` | Move Inventory to its own module |
| `materials/recipes.rs:18` | Dynamic recipes |
| `systems/network.rs:14` | Figure out if NetworkConnection component is needed |

---

## Parking Lot (Ideas for Later)

- **Central intelligence / hive mind feel**: Workers as extensions of a planner, not individual agents
- **Predictive logistics**: Fetch inputs before buildings run out
- **Bottleneck analysis**: Prioritize tasks that unblock downstream production
- **Worker pre-positioning**: Station workers near high-traffic areas
- **Visual production flow**: Show item movement paths, throughput rates
- **Blueprint system**: Save and stamp building configurations
- **Research tree**: Unlock buildings and recipes through production goals
