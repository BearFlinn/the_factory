# Inventory & Production System Refactor (v2)

## Background

This document supersedes the original refactor plan. The first attempt at this refactor introduced `InputBuffer`/`OutputBuffer` components and archetype markers (`Source`, `Processor`, `Sink`), but the migration was incomplete. The result was two parallel systems (legacy `InventoryType` and new buffers) coexisting awkwardly, making the codebase harder to reason about than before.

This revision takes a cleaner approach: **port components** that follow Bevy-idiomatic ECS patterns.

---

## Current State Analysis

### The Original Problem (Still Valid)

Buildings like Smelter have ONE inventory that holds both incoming ore AND outgoing ingots. This causes:

1. **Inventory jams**: The inventory fills with a mix of inputs and outputs, blocking both delivery and production
2. **Confusing crafting logic**: The system has to reason about "do I have space AND inputs" in a shared pool

### The Partial Refactor Problem (New)

The first refactor attempt created a hybrid state:

- Some buildings use `Inventory` + `InventoryType` (Storage)
- Some buildings use `InputBuffer`/`OutputBuffer` + archetype markers (Smelter, Mining Drill)
- Task creation code queries both patterns with fallback logic
- Transfer validation checks buffers first, then falls back to legacy inventory
- UI queries `Inventory` only, so buffer-based buildings disappeared from tooltips

This hybrid state is worse than either pure approach would be.

---

## Proposed Solution: Port Components

### Core Concept

Instead of inventory "types" that encode behavior in an enum, use **separate component types** that represent logistics roles. The presence of a component IS the type information—no runtime checks needed.

### Port Component Definitions

```rust
/// Shared behavior for all inventory-like components
pub trait InventoryAccess {
    fn items(&self) -> &HashMap<ItemName, u32>;
    fn items_mut(&mut self) -> &mut HashMap<ItemName, u32>;
    fn capacity(&self) -> u32;

    // Default implementations for common operations
    fn add_item(&mut self, name: &str, quantity: u32) -> u32 { ... }
    fn remove_item(&mut self, name: &str, quantity: u32) -> u32 { ... }
    fn get_item_quantity(&self, name: &str) -> u32 { ... }
    fn get_total_quantity(&self) -> u32 { ... }
    fn is_full(&self) -> bool { ... }
    fn is_empty(&self) -> bool { ... }
    fn has_space_for(&self, items: &HashMap<ItemName, u32>) -> bool { ... }
    // ... etc
}

/// Items can be picked up from here (Mining Drills, Smelter outputs)
#[derive(Component)]
pub struct OutputPort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

/// Items can be delivered here (Generators, Smelter inputs)
#[derive(Component)]
pub struct InputPort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

/// Bidirectional storage - accepts deliveries and provides pickups
#[derive(Component)]
pub struct StoragePort {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

/// Transient carrying capacity for workers
#[derive(Component)]
pub struct Cargo {
    pub items: HashMap<ItemName, u32>,
    pub capacity: u32,
}

impl InventoryAccess for OutputPort { ... }
impl InventoryAccess for InputPort { ... }
impl InventoryAccess for StoragePort { ... }
impl InventoryAccess for Cargo { ... }
```

### Building Compositions

| Building | Components | Logistics Role |
|----------|------------|----------------|
| Mining Drill | `OutputPort` | Provides items only |
| Generator | `InputPort` | Accepts items only |
| Smelter | `InputPort` + `OutputPort` | Accepts inputs, provides outputs |
| Assembler | `InputPort` + `OutputPort` | Accepts inputs, provides outputs |
| Storage | `StoragePort` | Bidirectional buffer |
| Worker | `Cargo` | Transient carrying |

### Why This Is Better

1. **No `InventoryType` enum.** Component presence IS the type. A building with `OutputPort` is a supplier by structure, not by checking a field.

2. **No archetype markers.** We don't need `Source`, `Processor`, `Sink` components. A building with only `OutputPort` is a source. A building with both `InputPort` and `OutputPort` is a processor. The component combination encodes the archetype.

3. **Bevy-idiomatic queries.** Systems can efficiently query exactly what they need:
   ```rust
   // Find all suppliers
   fn find_suppliers(
       outputs: Query<(Entity, &Position, &OutputPort)>,
       storage: Query<(Entity, &Position, &StoragePort)>,
   ) { ... }

   // Find all destinations
   fn find_destinations(
       inputs: Query<(Entity, &Position, &InputPort)>,
       storage: Query<(Entity, &Position, &StoragePort)>,
   ) { ... }

   // Crafting only matches buildings with both ports
   fn update_crafters(
       crafters: Query<(&mut InputPort, &mut OutputPort, &RecipeCrafter, &Operational)>,
   ) { ... }
   ```

4. **Fine-grained change detection.** `Changed<OutputPort>` won't trigger systems watching `Changed<InputPort>`.

5. **System parallelism.** Systems reading only `InputPort` can run in parallel with systems reading only `OutputPort`.

6. **UI works naturally.** Query all port types, display contents:
   ```rust
   fn building_tooltip(
       entity: Entity,
       inputs: Query<&InputPort>,
       outputs: Query<&OutputPort>,
       storage: Query<&StoragePort>,
   ) -> String {
       let mut lines = Vec::new();
       if let Ok(input) = inputs.get(entity) {
           lines.push(format!("Input: {} items", input.get_total_quantity()));
       }
       if let Ok(output) = outputs.get(entity) {
           lines.push(format!("Output: {} items", output.get_total_quantity()));
       }
       if let Ok(store) = storage.get(entity) {
           lines.push(format!("Storage: {} items", store.get_total_quantity()));
       }
       lines.join("\n")
   }
   ```

---

## System Changes

### Crafting System

Before (complex single-inventory logic):
```rust
let can_craft = !inventory.is_full()
    || recipe.inputs.iter()
        .all(|(item, qty)| inventory.has_at_least(item, *qty));
```

After (clean separated logic):
```rust
fn update_crafters(
    mut crafters: Query<(&mut InputPort, &mut OutputPort, &RecipeCrafter, &Operational)>,
    recipes: Res<RecipeRegistry>,
    time: Res<Time>,
) {
    for (mut input, mut output, crafter, operational) in &mut crafters {
        if !operational.is_active() { continue; }
        if !crafter.timer.tick(time.delta()).just_finished() { continue; }

        let Some(recipe) = recipes.get(crafter.current_recipe()) else { continue };

        // Simple checks: do we have inputs? do we have output space?
        let has_inputs = recipe.inputs.iter()
            .all(|(item, qty)| input.get_item_quantity(item) >= *qty);
        let has_space = output.has_space_for(&recipe.outputs);

        if has_inputs && has_space {
            // Consume from input
            for (item, qty) in &recipe.inputs {
                input.remove_item(item, *qty);
            }
            // Produce to output
            for (item, qty) in &recipe.outputs {
                output.add_item(item, *qty);
            }
        }
    }
}
```

### Task Creation

```rust
/// Find buildings that have items available for pickup
fn find_available_pickups(
    outputs: Query<(Entity, &Position, &OutputPort), With<Building>>,
    storage: Query<(Entity, &Position, &StoragePort), With<Building>>,
) -> Vec<(Entity, Position, HashMap<ItemName, u32>)> {
    let mut available = Vec::new();

    for (entity, pos, output) in &outputs {
        if !output.is_empty() {
            available.push((entity, *pos, output.items.clone()));
        }
    }

    for (entity, pos, storage) in &storage {
        if !storage.is_empty() {
            available.push((entity, *pos, storage.items.clone()));
        }
    }

    available
}

/// Find buildings that need items delivered
fn find_delivery_destinations(
    inputs: Query<(Entity, &Position, &InputPort, Option<&RecipeCrafter>), With<Building>>,
    storage: Query<(Entity, &Position, &StoragePort), With<Building>>,
    recipes: Res<RecipeRegistry>,
) -> Vec<(Entity, Position, HashMap<ItemName, u32>)> {
    let mut destinations = Vec::new();

    for (entity, pos, input, maybe_crafter) in &inputs {
        if input.is_full() { continue; }

        // If building has a recipe, only request recipe inputs
        let needed = if let Some(crafter) = maybe_crafter {
            calculate_recipe_needs(input, crafter, &recipes)
        } else {
            // No recipe - accept anything (e.g., Generator with fixed fuel type)
            HashMap::new() // or specific item filter
        };

        if !needed.is_empty() {
            destinations.push((entity, *pos, needed));
        }
    }

    for (entity, pos, storage) in &storage {
        if !storage.is_full() {
            destinations.push((entity, *pos, HashMap::new())); // Accepts anything
        }
    }

    destinations
}
```

### Transfer Execution

```rust
fn execute_pickup(
    entity: Entity,
    items: &HashMap<ItemName, u32>,
    outputs: &mut Query<&mut OutputPort>,
    storage: &mut Query<&mut StoragePort>,
    cargo: &mut Query<&mut Cargo>,
    worker: Entity,
) {
    // Try OutputPort first, then StoragePort
    let source = outputs.get_mut(entity)
        .map(|o| o.into_inner() as &mut dyn InventoryAccess)
        .or_else(|_| storage.get_mut(entity)
            .map(|s| s.into_inner() as &mut dyn InventoryAccess));

    let Ok(mut worker_cargo) = cargo.get_mut(worker) else { return };

    if let Ok(source) = source {
        for (item, qty) in items {
            let removed = source.remove_item(item, *qty);
            worker_cargo.add_item(item, removed);
        }
    }
}

fn execute_dropoff(
    entity: Entity,
    inputs: &mut Query<&mut InputPort>,
    storage: &mut Query<&mut StoragePort>,
    cargo: &mut Query<&mut Cargo>,
    worker: Entity,
) {
    // Try InputPort first, then StoragePort
    let destination = inputs.get_mut(entity)
        .map(|i| i.into_inner() as &mut dyn InventoryAccess)
        .or_else(|_| storage.get_mut(entity)
            .map(|s| s.into_inner() as &mut dyn InventoryAccess));

    let Ok(mut worker_cargo) = cargo.get_mut(worker) else { return };

    if let Ok(dest) = destination {
        for (item, qty) in worker_cargo.items().clone() {
            let space = dest.capacity() - dest.get_total_quantity();
            let transfer = qty.min(space);
            if transfer > 0 {
                worker_cargo.remove_item(&item, transfer);
                dest.add_item(&item, transfer);
            }
        }
    }
}
```

---

## Building Configuration

### RON Definitions

```ron
(
    name: "Mining Drill",
    category: Production,
    appearance: ( ... ),
    placement: ( ... ),
    components: [
        PowerConsumer(amount: 10),
        ViewRange(radius: 2),
        RecipeCrafter(recipe_name: None, available_recipes: None, interval: 1.0),
        OutputPort(capacity: 100),
    ]
),

(
    name: "Generator",
    category: Production,
    appearance: ( ... ),
    placement: ( ... ),
    components: [
        PowerGenerator(amount: 40),
        RecipeCrafter(recipe_name: Some("Power"), available_recipes: None, interval: 2.0),
        ViewRange(radius: 2),
        InputPort(capacity: 100),
    ]
),

(
    name: "Smelter",
    category: Production,
    appearance: ( ... ),
    placement: ( ... ),
    components: [
        PowerConsumer(amount: 60),
        RecipeCrafter(recipe_name: None, available_recipes: Some(["Iron Ingot", "Copper Ingot"]), interval: 2.0),
        ViewRange(radius: 2),
        InputPort(capacity: 50),
        OutputPort(capacity: 50),
    ]
),

(
    name: "Storage",
    category: Logistics,
    appearance: ( ... ),
    placement: ( ... ),
    components: [
        StoragePort(capacity: 200),
        ViewRange(radius: 2),
    ]
),
```

### BuildingComponentDef Updates

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BuildingComponentDef {
    PowerConsumer { amount: i32 },
    PowerGenerator { amount: i32 },
    ComputeGenerator { amount: i32 },
    ComputeConsumer { amount: i32 },
    ViewRange { radius: i32 },
    NetWorkComponent,
    RecipeCrafter { ... },
    Scanner { ... },

    // New port components (replacing Inventory, InventoryType, InputBuffer, OutputBuffer, Source, Processor, Sink)
    InputPort { capacity: u32 },
    OutputPort { capacity: u32 },
    StoragePort { capacity: u32 },
}
```

---

## Implementation Plan

### Phase 1: Add New Components

1. Define `InputPort`, `OutputPort`, `StoragePort`, `Cargo` components
2. Define `InventoryAccess` trait with shared behavior
3. Implement the trait for all port types
4. Add to `BuildingComponentDef` enum
5. Update `spawn_building` to handle new component types

### Phase 2: Update Core Systems

1. Rewrite crafting systems to use `InputPort`/`OutputPort`
2. Rewrite transfer validation/execution to use port components
3. Update task creation to query port components
4. Update worker systems to use `Cargo`

### Phase 3: Update UI

1. Update tooltip systems to query all port types
2. Update any inventory display widgets
3. Test that all buildings appear correctly

### Phase 4: Update Building Definitions

1. Convert all buildings in `buildings.ron` to use port components
2. Remove `Inventory`, `InventoryType`, `InputBuffer`, `OutputBuffer`, `Source`, `Processor`, `Sink` from definitions

### Phase 5: Remove Legacy Code

1. Delete `InventoryType` enum and component
2. Delete `InputBuffer`, `OutputBuffer` components
3. Delete `Source`, `Processor`, `Sink` marker components
4. Delete `Inventory` component (replaced by port-specific types)
5. Delete legacy task creation code (`create_proactive_tasks` using InventoryTypes)
6. Delete buffer polling system (`poll_buffer_logistics`)
7. Clean up any remaining fallback logic

### Files to Modify

**Core inventory system:**
- `src/materials/items.rs` - Define port components and trait
- `src/materials/mod.rs` - Update exports

**Building system:**
- `src/structures/building_config.rs` - Update BuildingComponentDef
- `src/structures/production.rs` - Rewrite crafting systems
- `src/assets/buildings.ron` - Update all building definitions

**Task system:**
- `src/workers/tasks/creation.rs` - Rewrite task creation
- `src/workers/tasks/execution.rs` - Update transfer logic

**Worker system:**
- `src/workers/mod.rs` - Add Cargo component to workers

**UI:**
- `src/ui/` - Update tooltip and inventory display systems

---

## Optional Future Enhancements

### Threshold Policies

If we want threshold-based logistics (request when low, offer when high), add optional policy components:

```rust
#[derive(Component)]
pub struct InputPolicy {
    pub request_threshold: f32,  // Request more when below this % full
}

#[derive(Component)]
pub struct OutputPolicy {
    pub offer_threshold: f32,  // Offer items when above this % full
}
```

These are separate from the ports themselves, keeping the core model clean.

### Item Filtering

If specific ports should only accept certain items:

```rust
#[derive(Component)]
pub struct ItemFilter {
    pub allowed_items: HashSet<ItemName>,
}
```

Attach to buildings that need filtering; task creation checks for it.

---

## Summary

The key insight is that **component presence should encode logistics role**, not runtime field checks. This aligns with Bevy's ECS philosophy and results in:

- Cleaner queries
- Better change detection
- Potential system parallelism
- No hybrid/fallback code paths
- Self-documenting building definitions

The migration is a full replacement rather than incremental addition, which avoids the hybrid state that made the first attempt problematic.
