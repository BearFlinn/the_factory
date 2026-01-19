# Refactor Planning Documents

Documentation from exploration session discussing the current state of the codebase and planned refactoring work.

## Context

The game has a functional but confusing inventory/production system that makes the task system harder to reason about. The decision was made to focus on simplifying and streamlining these foundational systems before pursuing other improvements.

## Documents

### [inventory-production-refactor.md](./inventory-production-refactor.md)
**Primary focus of the refactor.**

- Analysis of current InventoryTypes system and why it's confusing
- The single-inventory problem that causes production jams
- Proposed solution: Separate input/output buffers
- Building archetypes: Source, Processor, Storage, Sink
- Buffer policies as a declarative approach
- Implementation considerations and migration path

### [task-system-notes.md](./task-system-notes.md)
**Secondary—will likely simplify naturally after inventory refactor.**

- Current architecture and system ordering
- Known bugs (jam bug root cause, output destination failure)
- Priority system exists but unused
- Relationship between task complexity and inventory complexity
- Notes on "central intelligence" design philosophy (future work)

### [misc-notes-and-backlog.md](../misc-notes-and-backlog.md)
**Lower priority items for later.**

- Scanning system quirks (not pure clockwise, dedup issue)
- Resource clustering (TODO)
- Multi-cell building issues
- Storage overflow handling
- Gameplay loop discussion (need a Sink for progression)
- Test coverage gaps
- All TODOs found in codebase
- Parking lot of future ideas

## Recommended Approach

1. **Start with inventory refactor** - Separate input/output buffers, implement building archetypes
2. **Update crafting logic** - Simple: pull from input buffer, push to output buffer
3. **Simplify task generation** - Buffer policies replace reactive Changed<Inventory> events
4. **Test the loop** - With reliable production chains, evaluate if the game "feels" right
5. **Add a Sink** - Something to consume end products and provide progression
6. **Revisit backlog** - Scanning, clustering, multi-cell buildings, etc.

## Key Files to Modify

| File | Changes |
|------|---------|
| `src/materials/items.rs` | Add InputBuffer, OutputBuffer components |
| `src/structures/production.rs` | Update crafting to use buffers |
| `src/structures/building_config.rs` | Update BuildingComponentDef for buffers |
| `src/workers/tasks/creation.rs` | Replace reactive task generation with buffer polling |
| `src/assets/buildings.ron` | Update building definitions |

## Session Date

2026-01-19
