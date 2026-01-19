# Inventory & Production System Refactor

## Current State Analysis

### The Problem

The current inventory and production system is difficult to reason about, which has downstream effects on the task system and makes bugs hard to diagnose. The foundational confusion makes everything built on top more complex than it needs to be.

### Current InventoryTypes

The system currently uses five inventory types defined in `src/materials/items.rs`:

```rust
pub enum InventoryTypes {
    Storage,    // General storage buildings
    Sender,     // Output-only (Mining Drills)
    Requester,  // Input-only (Generator)
    Carrier,    // Workers
    Producer,   // Both input and output (Smelter, Assembler)
}
```

**The core issue**: `Producer` is trying to be both `Requester` AND `Sender`, which creates the most complex handling in `crafter_logistics_requests` (`src/structures/production.rs:127-193`). The branching logic treats each type differently, and Producer gets the longest, most tangled branch.

### Single Inventory Problem

Buildings like Smelter have ONE inventory (100 slots) that holds both incoming ore AND outgoing ingots. This causes:

1. **Inventory jams**: The inventory fills with a mix of inputs and outputs, blocking both delivery and production
2. **Confusing crafting logic**: The system has to reason about "do I have space AND inputs" in a shared pool

The crafting condition in `update_recipe_crafters` (`src/structures/production.rs:29-33`):

```rust
let can_craft = !inventory.is_full()
    || recipe
        .inputs
        .iter()
        .all(|(item_name, quantity)| inventory.has_at_least(item_name, *quantity));
```

This OR logic is confusing. The apparent intent: "craft if there's space OR if you have inputs (because transforming inputs to outputs is net-zero space)." But the logic is hard to follow and may not handle edge cases correctly.

### Current Building Inventory Configurations

From `src/assets/buildings.ron`:

| Building | InventoryType | Capacity | Role |
|----------|---------------|----------|------|
| Mining Drill | Sender | 100 | Extracts ore from nodes |
| Generator | Requester | 100 | Consumes coal for power |
| Smelter | Producer | 100 | Ore → Ingots |
| Assembler | Producer | 100 | Ingots → Components |
| Storage | Storage | 200 | General buffer |

---

## Proposed Refactor

### Separate Input/Output Buffers

Instead of one inventory, production buildings should have **two distinct inventories**:

- **Input Buffer**: Holds items waiting to be processed
- **Output Buffer**: Holds items that have been produced

Benefits:
- **No mixing**: Inputs and outputs can't compete for the same slots
- **Clear jam diagnosis**: If output buffer is full, outputs aren't being collected. If input buffer is empty, inputs aren't being delivered.
- **Simpler crafting logic**: Pull from input buffer, push to output buffer. No complex space calculations.
- **Simpler logistics**: Task system doesn't need to understand recipes—just "move items from output buffers to input buffers"

Example flow for Smelter:
```
[Mining Drill Output] → Worker → [Smelter Input Buffer]
                                         ↓
                                    (Crafting)
                                         ↓
                                 [Smelter Output Buffer] → Worker → [Storage or next Processor]
```

### Building Archetypes

Replace inventory-type-based categorization with **role-based archetypes** that map to production graph topology:

#### Source
- **Role**: Entry point where items enter the production chain
- **Examples**: Mining Drill (extracts ore from world)
- **Buffers**: Output only
- **Graph topology**: Only outgoing edges
- **Logistics concern**: "Where do outputs go?"

#### Processor
- **Role**: Transforms items according to recipes
- **Examples**: Smelter, Assembler
- **Buffers**: Input AND Output (separate)
- **Graph topology**: Incoming and outgoing edges
- **Logistics concerns**: "Where do inputs come from?" + "Where do outputs go?"

#### Storage
- **Role**: Buffer node that holds items in transit
- **Examples**: Storage building
- **Buffers**: Single general-purpose inventory (or could be split for organization)
- **Graph topology**: Pass-through node
- **Logistics concern**: Balancing, overflow handling

#### Sink
- **Role**: Exit point where items leave the system for a purpose
- **Examples**: Not yet implemented—needed for gameplay loop
- **Buffers**: Input only
- **Graph topology**: Only incoming edges
- **Logistics concern**: "Where do inputs come from?"
- **Purpose**: Consumes items for research, victory points, progression

### Buffer Policies (Declarative Approach)

Instead of reactive event-driven requests (`Changed<Inventory>` triggers), buffers should express **standing policies**:

```rust
// Conceptual - not exact implementation
struct InputBuffer {
    inventory: Inventory,
    request_threshold: f32,  // Request more when below this % full (e.g., 0.5)
}

struct OutputBuffer {
    inventory: Inventory,
    offer_threshold: f32,    // Offer items when above this % full (e.g., 0.2)
}
```

The logistics planner **polls** all buffers periodically:
1. Collect all "requests" (input buffers below threshold)
2. Collect all "offers" (output buffers above threshold)
3. Match requests to offers based on item type, distance, priority
4. Create transfer tasks

Benefits:
- Decouples "what buildings want" from "when we check"
- No race conditions from rapid inventory changes
- Easier to reason about—the system state is evaluated holistically
- Can add smarter planning later (prioritization, batching) without changing building logic

---

## Implementation Considerations

### Data Structure Changes

Current:
```rust
// Single inventory component
#[derive(Component)]
pub struct Inventory { ... }

#[derive(Component)]
pub struct InventoryType(pub InventoryTypes);
```

Proposed:
```rust
// Separate buffer components
#[derive(Component)]
pub struct InputBuffer {
    pub inventory: Inventory,
    pub request_threshold: f32,
}

#[derive(Component)]
pub struct OutputBuffer {
    pub inventory: Inventory,
    pub offer_threshold: f32,
}

// Building archetype marker components
#[derive(Component)]
pub struct Source;

#[derive(Component)]
pub struct Processor;

#[derive(Component)]
pub struct Sink;

// Storage might just use a regular Inventory without the archetype markers
```

### Migration Path

1. **Add new buffer components** alongside existing Inventory
2. **Update crafting logic** to use input/output buffers
3. **Update task generation** to use buffer policies
4. **Update building definitions** in RON files
5. **Remove old InventoryType system** once everything is migrated

### Files to Modify

- `src/materials/items.rs` - Add buffer types, potentially deprecate InventoryTypes
- `src/structures/production.rs` - Update crafting and logistics request logic
- `src/structures/building_config.rs` - Update BuildingComponentDef for new buffer system
- `src/workers/tasks/creation.rs` - Update task creation to work with buffers
- `src/assets/buildings.ron` - Update building definitions

---

## Open Questions

1. **Should Storage buildings have input/output buffers or a single inventory?**
   - Single inventory is simpler for pure storage
   - Split could enable "input side" vs "output side" for logistics
A: Single inventory.

2. **How should workers' Carrier inventory work with the new system?**
   - Workers pick up from output buffers, drop off to input buffers
   - Worker inventory is transient, probably stays as single inventory
A: Minimal changes should be needed.

3. **What about buildings that don't fit the archetypes cleanly?**
   - Generator consumes coal but doesn't "produce" items (produces power)
   - Could model power as a pseudo-item, or treat Generator as a Sink for coal
A: Generators fall neatly under the Sink role, as would any building that takes items as input and produces a non-item output

4. **Buffer size ratios**
   - Should input and output buffers be equal size?
   - Should they be configurable per building type?
   - Smaller output buffers would force more frequent collection
A: Configurable by building.

5. **Threshold tuning**
   - What are good default request/offer thresholds?
   - Should these be configurable per building or global?
A: Global for now.
