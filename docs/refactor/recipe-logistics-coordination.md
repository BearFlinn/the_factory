# Refactor Plan: Recipe Selection and Logistics Coordination

## 1. Current Flow Analysis

### Recipe Selection Flow

1. **UI Recipe Selection** (`src/ui/building_menu.rs`)
   - User clicks a recipe button in the building menu
   - `handle_recipe_selection()` detects the selection change and emits `RecipeChangeEvent`
   - `apply_recipe_changes()` calls `RecipeCrafter::set_recipe()` which directly mutates `current_recipe`
   - **No coordination** with logistics system occurs

2. **RecipeCrafter State** (`src/structures/construction.rs:28-57`)
   - Simple mutable state: `current_recipe: Option<RecipeName>` and `available_recipes: Vec<RecipeName>`
   - Recipe can be changed at any time via `set_recipe()`
   - No tracking of whether inputs are being delivered or what recipe was "active" when logistics started

### Logistics Request Flow

1. **Polling for Logistics Needs** (`src/structures/production.rs:172-268`)
   - `poll_port_logistics()` runs on a 1-second timer
   - For each building with an `InputPort` and `RecipeCrafter`, calls `emit_input_port_requests()`
   - `emit_input_port_requests()` reads `crafter.get_active_recipe()` at poll time
   - Creates `PortLogisticsRequest` events for items needed by the **current** recipe

2. **Task Creation** (`src/workers/tasks/creation.rs:64-136`)
   - `create_port_logistics_tasks()` consumes `PortLogisticsRequest` events
   - Finds suppliers via `find_port_suppliers()` and creates pickup/dropoff task sequences
   - Task contains the items to move (`TaskAction::Pickup(Some(items))`, `TaskAction::Dropoff(Some(items))`)
   - **Items are "locked in" at task creation time** based on the recipe active at that moment

3. **Task Execution** (`src/workers/tasks/execution.rs:283-330`)
   - When worker arrives at destination, `execute_task_action()` runs
   - Calls `request_transfer_specific_items()` to actually move items
   - Transfer happens regardless of what recipe the building currently has selected

### Production Consumption Flow

1. **Crafting Loop** (`src/structures/production.rs:42-88`)
   - `update_port_crafters()` checks `crafter.get_active_recipe()` at craft time
   - Only consumes items if `InputPort` has all required items for the **current** recipe
   - If recipe changed mid-delivery, wrong items may accumulate in `InputPort`

## 2. The Coordination Gap

The fundamental problem is that recipe selection and logistics operate on **different temporal boundaries** with no synchronization:

| Component | When Recipe is Read | Consequence |
|-----------|-------------------|-------------|
| UI Selection | User clicks | Immediately changes `current_recipe` |
| Logistics Polling | Every 1 second | Creates requests for recipe active at poll time |
| Task Creation | On `PortLogisticsRequest` event | Locks in items based on recipe at event time |
| Task Execution | Worker arrival | Delivers whatever items were in the task |
| Production | Every craft tick | Consumes based on recipe active at craft time |

**No component knows what the others are doing.** Specifically:

1. **No "commitment" concept**: The building doesn't track when it committed to needing certain inputs
2. **No task invalidation**: When recipe changes, existing tasks for the old recipe continue executing
3. **No buffer awareness**: Logistics doesn't know what items are already in transit
4. **Race condition**: Recipe can change between poll -> task creation -> delivery -> consumption

### Concrete Problem Scenarios

**Scenario A: Recipe Change Mid-Delivery**
1. Smelter has "Iron Ingot" recipe selected (needs Iron Ore + Coal)
2. Logistics polls, creates task to deliver Iron Ore
3. User changes recipe to "Copper Ingot" (needs Copper Ore + Coal)
4. Worker delivers Iron Ore to the Smelter
5. Iron Ore sits unused, Smelter waits for Copper Ore that wasn't requested

**Scenario B: Double Delivery**
1. Smelter needs 20 Iron Ore (has 0, target is 10x recipe = 20)
2. Logistics polls at t=0, creates task for 20 Iron Ore
3. Logistics polls at t=1, recipe still same, InputPort still empty (delivery in progress)
4. Creates another task for 20 Iron Ore (40 total incoming)
5. Only 50 capacity means overflow/waste

**Scenario C: Insufficient Inputs for Selected Recipe**
1. Assembler has "Electronic Circuit" selected (needs Copper Wire + Iron Plate)
2. No Copper Wire in network, but Iron Plate available
3. Logistics delivers Iron Plate repeatedly (it's needed)
4. Assembler never crafts because Copper Wire never arrives
5. Iron Plate fills InputPort, blocking any future deliveries

## 3. Proposed Solution: Recipe Commitment Contract

### Core Concept

Introduce a **recipe commitment** that:
1. Represents the recipe the building is "locked into" for logistics purposes
2. Only changes when safe (no active deliveries for different recipes)
3. Is the single source of truth for what inputs logistics should deliver

### New Components

```rust
// src/structures/components.rs (new or in construction.rs)

/// Tracks the recipe a building has committed to for logistics purposes.
/// Separate from UI selection to allow queued recipe changes.
#[derive(Component, Debug)]
pub struct RecipeCommitment {
    /// The recipe logistics is currently delivering for
    pub committed_recipe: Option<RecipeName>,
    /// Recipe the user wants to switch to (pending commitment)
    pub pending_recipe: Option<RecipeName>,
    /// Items currently in-transit for this building
    pub in_transit_items: HashMap<ItemName, u32>,
}

/// Marker for buildings that need their recipe commitment evaluated
#[derive(Component)]
pub struct NeedsRecipeCommitmentEvaluation;
```

### Modified RecipeCrafter Behavior

The `RecipeCrafter` component keeps its existing fields but its `set_recipe()` method changes:

```rust
impl RecipeCrafter {
    /// Sets the user's desired recipe. Does not immediately commit.
    pub fn set_desired_recipe(&mut self, recipe_name: RecipeName) -> Result<(), String> {
        if self.is_single_recipe() || self.available_recipes.contains(&recipe_name) {
            self.current_recipe = Some(recipe_name);
            Ok(())
        } else {
            Err(...)
        }
    }
}
```

### New System Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          RECIPE COMMITMENT FLOW                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  User selects recipe ──► RecipeCrafter.current_recipe updated               │
│                                 │                                           │
│                                 ▼                                           │
│                    NeedsRecipeCommitmentEvaluation marker added             │
│                                 │                                           │
│                                 ▼                                           │
│              ┌────────────────────────────────────────┐                     │
│              │  evaluate_recipe_commitments() system  │                     │
│              └────────────────────────────────────────┘                     │
│                                 │                                           │
│              ┌──────────────────┼───────────────────┐                       │
│              ▼                  ▼                   ▼                       │
│       [Same recipe?]    [No in-transit?]    [Different, has transit]        │
│              │                  │                   │                       │
│              ▼                  ▼                   ▼                       │
│         No change         Commit new          Set pending_recipe            │
│                           recipe              (wait for transit clear)      │
│                                                                             │
│                                                                             │
│  poll_port_logistics() ──► reads committed_recipe (NOT current_recipe)      │
│                                 │                                           │
│                                 ▼                                           │
│                    Creates PortLogisticsRequest                             │
│                    Updates in_transit_items                                 │
│                                                                             │
│                                                                             │
│  Task completes ──► Updates in_transit_items (decrement)                    │
│                     If pending_recipe exists and transit empty, commit      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 4. Detailed Implementation Plan

### Phase 1: Add RecipeCommitment Component

**Files to modify:**
- `src/structures/construction.rs` - Add `RecipeCommitment` component definition

**Changes:**
1. Define `RecipeCommitment` component with:
   - `committed_recipe: Option<RecipeName>`
   - `pending_recipe: Option<RecipeName>`
   - `in_transit_items: HashMap<ItemName, u32>`
2. Define `NeedsRecipeCommitmentEvaluation` marker component

### Phase 2: Modify Building Spawning

**Files to modify:**
- `src/structures/building_config.rs` - Spawn `RecipeCommitment` with `RecipeCrafter`

**Changes:**
1. In `spawn_building()`, when inserting `RecipeCrafter`, also insert `RecipeCommitment`:
   - If single-recipe building: `committed_recipe = current_recipe`
   - If multi-recipe building: `committed_recipe = None` (no commitment until user selects)

### Phase 3: Update Recipe Selection UI

**Files to modify:**
- `src/ui/building_menu.rs`

**Changes:**
1. `apply_recipe_changes()` should:
   - Update `RecipeCrafter.current_recipe` (user's desired recipe)
   - Add `NeedsRecipeCommitmentEvaluation` marker to the building entity
2. Display commitment status in building menu:
   - Show "Committed: X" vs "Selected: Y (pending commit)"
   - Visual indicator when recipes differ

### Phase 4: Create Recipe Commitment Evaluation System

**Files to create:**
- `src/structures/commitment.rs` (new file)

**New systems:**
1. `evaluate_recipe_commitments()`:
   - Query buildings with `NeedsRecipeCommitmentEvaluation`
   - If `current_recipe == committed_recipe`: remove marker, done
   - If `in_transit_items.is_empty()`: commit new recipe, remove marker
   - Else: set `pending_recipe = current_recipe`, remove marker (will commit when transit clears)

2. `commit_pending_recipes()`:
   - Query buildings where `pending_recipe.is_some()` and `in_transit_items.is_empty()`
   - Move `pending_recipe` to `committed_recipe`
   - Clear `pending_recipe`

### Phase 5: Update Logistics Polling

**Files to modify:**
- `src/structures/production.rs`

**Changes:**
1. `poll_port_logistics()` query must include `RecipeCommitment`
2. `emit_input_port_requests()` should:
   - Read `commitment.committed_recipe` instead of `crafter.get_active_recipe()`
   - Skip if no committed recipe
   - Account for `in_transit_items` when calculating deficit

```rust
fn emit_input_port_requests(
    entity: Entity,
    input_port: &InputPort,
    commitment: &RecipeCommitment,  // NEW parameter
    recipe_registry: &RecipeRegistry,
    events: &mut EventWriter<PortLogisticsRequest>,
) {
    let Some(recipe_name) = &commitment.committed_recipe else {
        return;  // No commitment = no requests
    };
    // ... rest of logic, but subtract in_transit_items from deficit calculation
}
```

### Phase 6: Track In-Transit Items

**Files to modify:**
- `src/workers/tasks/creation.rs`
- `src/workers/tasks/execution.rs`

**Changes in `creation.rs`:**
1. When creating logistics tasks for an `InputPort`:
   - Update `RecipeCommitment.in_transit_items` with requested quantities
   - Store building entity on the task (already done via `TaskTarget`)

2. Add new event `LogisticsTaskCreatedEvent`:
```rust
#[derive(Event)]
pub struct LogisticsTaskCreatedEvent {
    pub building: Entity,
    pub items: HashMap<ItemName, u32>,
}
```

**Changes in `execution.rs`:**
1. After `TaskAction::Dropoff` completes to a building with `RecipeCommitment`:
   - Emit `LogisticsTaskCompletedEvent`

2. Add new event and handler:
```rust
#[derive(Event)]
pub struct LogisticsTaskCompletedEvent {
    pub building: Entity,
    pub items: HashMap<ItemName, u32>,
}
```

**New system `update_in_transit_tracking()`:**
- On `LogisticsTaskCreatedEvent`: increment `in_transit_items`
- On `LogisticsTaskCompletedEvent`: decrement `in_transit_items`

### Phase 7: Handle Edge Cases

**7a. Recipe Change with Items In-Transit**
- Already handled: `pending_recipe` is set, commitment doesn't change until transit clears
- UI shows "pending" state

**7b. Task Cancellation / Worker Stranding**
- Current system already has `emergency_dropoff_idle_workers`
- Add: when emergency dropoff triggers for items destined for a building, decrement that building's `in_transit_items`

**7c. InputPort with Wrong Items (Pre-existing or Misdelivered)**
- Production system already handles this: won't craft if wrong items
- Add optional: `flush_wrong_items()` system that requests pickup of items not matching `committed_recipe`

**7d. No Valid Recipe Committed**
- Multi-recipe buildings start with `committed_recipe = None`
- No logistics requests generated until user commits
- Production won't run (already the case, `get_active_recipe()` returns None)

### Phase 8: System Ordering

**Add to `src/structures/mod.rs`:**

```rust
.add_systems(
    Update,
    (
        // Existing systems...
        evaluate_recipe_commitments
            .run_if(any_needs_evaluation)
            .after(apply_recipe_changes),  // After UI updates
        commit_pending_recipes
            .before(poll_port_logistics),  // Before logistics reads commitments
    )
)
```

**Add to `src/workers/tasks/mod.rs`:**

```rust
.add_systems(
    Update,
    (
        update_in_transit_tracking
            .in_set(TaskSystemSet::Cleanup)
            .after(handle_sequence_task_arrivals),
    )
)
```

## 5. Testing Strategy

### Unit Tests

1. **RecipeCommitment component tests:**
   - Test state transitions (no commitment -> committed -> pending -> new commitment)
   - Test in_transit_items accounting

2. **evaluate_recipe_commitments tests:**
   - Same recipe, no change
   - Different recipe, empty transit -> immediate commit
   - Different recipe, active transit -> pending set

### Integration Tests

1. **Recipe change mid-delivery:**
   - Set up Smelter with Iron Ingot recipe
   - Create delivery task
   - Change recipe to Copper Ingot
   - Verify pending_recipe set
   - Complete delivery
   - Verify commitment changes

2. **Double-delivery prevention:**
   - Set up building needing 20 items
   - Trigger logistics poll
   - Verify in_transit_items updated
   - Trigger second poll
   - Verify no duplicate task created

## 6. Migration Path

1. Add components and systems with feature flag or conditional compilation
2. Initially have `RecipeCommitment.committed_recipe` always mirror `RecipeCrafter.current_recipe` (backward compatible)
3. Gradually enable full commitment logic
4. Remove backward compatibility once validated

---

## Files Summary

| File | Changes |
|------|---------|
| `src/structures/construction.rs` | Add `RecipeCommitment` component, modify `RecipeCrafter` API |
| `src/structures/production.rs` | Modify `emit_input_port_requests()` to use commitment, update queries |
| `src/workers/tasks/creation.rs` | Track in-transit items when creating logistics tasks |
| `src/ui/building_menu.rs` | Update UI to show commitment state, trigger evaluation on recipe change |
| `src/structures/building_config.rs` | Spawn `RecipeCommitment` alongside `RecipeCrafter` |
| `src/structures/commitment.rs` | NEW: Commitment evaluation systems |
| `src/structures/mod.rs` | Register new systems and events |
| `src/workers/tasks/mod.rs` | Add in-transit tracking system |
