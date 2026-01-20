# Worker State Management Refactor Plan

## Current Problem Analysis

### The Three Sources of Truth

Worker state management has three independent sources of truth that create race conditions:

1. **`AssignedSequence(Option<Entity>)`** - The "logical" assignment state
   - Modified by: `assign_available_sequences_to_workers`, `handle_worker_interrupts`, `validate_and_displace_stranded_workers`, `derive_worker_state_from_sequences`, `process_worker_sequences`, `handle_sequence_task_arrivals`

2. **`WorkerState` (Idle/Working)** - The "derived" state
   - Modified by: `derive_worker_state_from_sequences` (intended derivation), but ALSO directly modified by: `handle_worker_interrupts`, `validate_and_displace_stranded_workers`, `validate_and_process_sequence`, `initiate_pathfinding_or_complete_task`, `handle_sequence_task_arrivals`, `validate_arrival_context`

3. **`WorkerPath` (waypoints, current_target)** - Movement state
   - Modified by: `move_workers`, `handle_worker_interrupts`, `validate_and_displace_stranded_workers`, `process_worker_sequences`

### Root Cause: Scattered State Mutation

The fundamental problem is that `WorkerState` is SUPPOSED to be derived from `AssignedSequence`, but multiple systems set it directly. This creates scenarios where:

- A worker has `AssignedSequence(Some(...))` but `WorkerState::Idle`
- A worker has `AssignedSequence(None)` but `WorkerState::Working`
- `derive_worker_state_from_sequences` runs AFTER other systems have already made inconsistent changes

### Current System Ordering

```
DomainOperations
  └── WorkersSystemSet::TaskManagement
        └── TaskSystemSet::Interrupts
              - handle_worker_interrupts        [modifies ALL THREE]
              - debug_clear_all_workers
              - emergency_dropoff_idle_workers  [reads state, may fire interrupt]
        └── TaskSystemSet::Assignment
              - assign_available_sequences_to_workers [modifies AssignedSequence]
        └── TaskSystemSet::Processing
              - process_worker_sequences              [modifies ALL THREE]
              - derive_worker_state_from_sequences    [modifies WorkerState, AssignedSequence]
        └── TaskSystemSet::Generation
              - create_port_logistics_tasks
              - create_proactive_port_tasks
              - create_port_construction_logistics_tasks
        └── TaskSystemSet::Cleanup
              - handle_sequence_task_arrivals         [modifies AssignedSequence, WorkerState]
              - clear_completed_tasks
  └── WorkersSystemSet::Movement
        - validate_and_displace_stranded_workers      [modifies ALL THREE, NOT IN SET!]
        - move_workers                                [modifies WorkerPath]
```

Critical issue: `validate_and_displace_stranded_workers` has NO system set, so its ordering relative to `TaskManagement` is undefined.

---

## Proposed Solution: Single Source of Truth with Computed Derivation

### Core Principle

**`AssignedSequence` is the ONLY mutable source of truth for worker assignment.** `WorkerState` becomes a purely derived read-only marker that is computed atomically when needed.

### Design Option A: Remove `WorkerState` Component Entirely (Recommended)

Replace all `WorkerState` reads with inline derivation from `AssignedSequence`.

**Rationale:**
- The game only has 2 states (Idle/Working)
- Derivation is trivial: `state = if assigned_sequence.0.is_some() { Working } else { Idle }`
- Eliminates the entire class of synchronization bugs
- No lag frame - state is always consistent

**Implementation:**

1. **Create a helper trait/method for state derivation:**
   ```rust
   // In spawning.rs or a new utils module
   pub trait WorkerStateComputation {
       fn is_idle(&self) -> bool;
       fn is_working(&self) -> bool;
   }

   impl WorkerStateComputation for AssignedSequence {
       fn is_idle(&self) -> bool { self.0.is_none() }
       fn is_working(&self) -> bool { self.0.is_some() }
   }
   ```

2. **Remove `WorkerState` component from `WorkerBundle`:**
   ```rust
   pub struct WorkerBundle {
       pub worker: Worker,
       pub speed: Speed,
       pub position: Position,
       pub path: WorkerPath,
       pub assigned_sequence: AssignedSequence,
       // REMOVED: pub state: WorkerState,
       pub cargo: Cargo,
       pub compute_consumer: ComputeConsumer,
       pub sprite: Sprite,
       pub transform: Transform,
   }
   ```

3. **Delete `derive_worker_state_from_sequences` system entirely**

4. **Update all queries and checks to use the trait methods:**
   - `find_available_worker`: Change `*worker_state == WorkerState::Idle` to `assigned_sequence.is_idle()`
   - `emergency_dropoff_idle_workers`: Change `*worker_state != WorkerState::Idle` to `!assigned_sequence.is_idle()`
   - etc.

5. **Remove all direct `WorkerState` mutations from other systems** (they become unnecessary)

### Design Option B: Strict Derived State (Alternative)

Keep `WorkerState` but make it truly derived with strict system ordering.

**Implementation:**

1. **Single point of state derivation:** Move ALL state derivation to a dedicated system that runs LAST in `TaskSystemSet::Processing`

2. **Ban direct `WorkerState` writes:** All other systems may only modify `AssignedSequence`. They must not touch `WorkerState`.

3. **System reads derived state for next frame:** Accept 1-frame lag but guarantee consistency.

**This approach is NOT recommended** because the 1-frame lag creates subtle bugs and the code complexity remains high.

---

## Path State Consolidation

### Current Problem with `WorkerPath`

`WorkerPath` is cleared by multiple systems (interrupts, displacement, execution failure) without coordination. This creates scenarios where:
- A worker is mid-path but gets displaced, losing their assignment
- A worker path is cleared but assignment remains

### Proposed Solution: Path as Part of Worker State Machine

Create a unified `WorkerActivity` component that encapsulates both assignment AND movement state:

```rust
#[derive(Component)]
pub enum WorkerActivity {
    Idle,
    Assigned {
        sequence: Entity,
        path: WorkerPath,
    },
    Displaced {
        // Worker was forcibly moved, needs recovery
        from_position: (i32, i32),
    },
}
```

**Benefits:**
- Assignment and path state cannot diverge
- State transitions are explicit and atomic
- Displacement is a first-class state, not an edge case

**Trade-off:**
- Larger refactor scope
- Need to update all systems that query either component

### Simpler Alternative: Path Clearing Protocol

Keep components separate but establish a strict protocol:

1. **Only these systems may clear `WorkerPath`:**
   - `handle_worker_interrupts` (when clearing/replacing assignment)
   - `validate_and_displace_stranded_workers` (emergency only)

2. **`move_workers` system** advances through path but never clears it

3. **`process_worker_sequences`** sets path, never clears it

4. **Path clearing ALWAYS accompanies assignment clearing**

---

## System Ordering Fixes

### Fix 1: Place `validate_and_displace_stranded_workers` in a System Set

```rust
.add_systems(
    Update,
    (
        validate_and_displace_stranded_workers.in_set(WorkersSystemSet::Lifecycle),
        move_workers.in_set(WorkersSystemSet::Movement),
    ),
);
```

This ensures displacement runs BEFORE task management, not in an undefined order.

### Fix 2: Reorder Task Sets for Consistency

Current order has a problem: `Cleanup` runs AFTER `Processing`, but `handle_sequence_task_arrivals` (in Cleanup) modifies state that `process_worker_sequences` (in Processing) also modifies.

**Proposed new order:**
```
TaskSystemSet::Interrupts    → Handle external interrupts, displacement recovery
TaskSystemSet::Assignment    → Assign sequences to workers
TaskSystemSet::Processing    → Execute tasks, initiate pathfinding
TaskSystemSet::Arrivals      → NEW: Handle arrivals (moved from Cleanup)
TaskSystemSet::Generation    → Create new tasks
TaskSystemSet::Cleanup       → Remove completed tasks only
```

Moving arrival handling before generation ensures that workers completing tasks are available for new assignments in the same frame.

### Fix 3: Atomic State Transitions

Implement helper functions that modify all related state atomically:

```rust
fn clear_worker_assignment(
    assigned_sequence: &mut AssignedSequence,
    worker_path: &mut WorkerPath,
) {
    assigned_sequence.0 = None;
    worker_path.waypoints.clear();
    worker_path.current_target = None;
}

fn assign_worker_to_sequence(
    assigned_sequence: &mut AssignedSequence,
    worker_path: &mut WorkerPath,
    sequence_entity: Entity,
) {
    assigned_sequence.0 = Some(sequence_entity);
    worker_path.waypoints.clear();
    worker_path.current_target = None;
}
```

All systems use these helpers instead of directly mutating components.

---

## Implementation Phases

### Phase 1: Fix System Ordering (Low Risk)

1. Add `validate_and_displace_stranded_workers` to `WorkersSystemSet::Lifecycle`
2. Create `TaskSystemSet::Arrivals` and move `handle_sequence_task_arrivals` there
3. Verify no behavior changes with existing tests

### Phase 2: Consolidate State Mutation (Medium Risk)

1. Create atomic helper functions for state transitions
2. Refactor all systems to use helpers
3. Add assertions to detect direct mutation in debug builds

### Phase 3: Remove `WorkerState` Component (Higher Risk, Highest Payoff)

1. Create `WorkerStateComputation` trait
2. Update all queries to use trait methods instead of `WorkerState` component
3. Remove `derive_worker_state_from_sequences` system
4. Remove `WorkerState` from `WorkerBundle`
5. Update tests

### Phase 4: Optional - Unified `WorkerActivity` (Future Enhancement)

Only if Phase 3 doesn't fully solve the issues, consider the more comprehensive `WorkerActivity` enum approach.

---

## Files Requiring Changes

| File | Changes |
|------|---------|
| `src/workers/spawning.rs` | Remove `WorkerState`, add `WorkerStateComputation` trait |
| `src/workers/mod.rs` | Fix system ordering for `validate_and_displace_stranded_workers` |
| `src/workers/tasks/mod.rs` | Add `TaskSystemSet::Arrivals`, reorder sets |
| `src/workers/tasks/assignment.rs` | Remove `derive_worker_state_from_sequences`, update queries, remove direct `WorkerState` writes |
| `src/workers/tasks/execution.rs` | Update queries, remove direct `WorkerState` writes, use atomic helpers |
| `src/workers/pathfinding.rs` | Update `validate_and_displace_stranded_workers` to use helpers |
| `src/workers/tasks/creation.rs` | Update query for idle workers |

---

## Testing Strategy

1. **Unit tests for `WorkerStateComputation` trait** - Verify derivation logic
2. **Integration tests for state transitions** - Ensure assignment/unassignment works atomically
3. **Regression tests for current behavior** - Capture existing scenarios before refactor
4. **Stress tests** - Multiple workers, rapid task creation/completion/interruption

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking existing worker behavior | Extensive testing before and after each phase |
| Performance impact from inline derivation | Profile; derivation is O(1), unlikely to matter |
| Large diff making review difficult | Phase implementation, commit after each phase |
| Missing edge cases | Review all grep results for `WorkerState`, `AssignedSequence`, `WorkerPath` |
