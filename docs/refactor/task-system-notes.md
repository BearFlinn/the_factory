# Task System Notes

## Current Architecture Overview

The task system is the most complex subsystem, managing how workers receive and execute jobs.

### System Set Ordering

Defined in `src/workers/tasks/mod.rs`:

```
TaskSet::Interrupts   → Cancel/preempt tasks
TaskSet::Assignment   → Match workers to tasks (BFS-based)
TaskSet::Processing   → Execute current tasks
TaskSet::Generation   → Create new tasks from building needs
TaskSet::Cleanup      → Remove completed tasks
```

### Key Components

From `src/workers/tasks/components.rs`:

- **Task**: Individual action (pickup or dropoff at a location)
- **TaskBundle**: Task with position, action, priority, status
- **TaskSequence**: Ordered list of tasks (e.g., pickup then dropoff)
- **TaskAction**: `Pickup(Option<items>)` or `Dropoff(Option<items>)`
- **Priority**: High, Medium, Low (defined but not actually used in assignment)
- **TaskStatus**: Pending, Queued, InProgress, Completed

### Current Flow

1. **Task Generation** (`src/structures/production.rs` and `src/workers/tasks/creation.rs`):
   - `crafter_logistics_requests` fires on `Changed<Inventory>`
   - Creates `CrafterLogisticsRequest` events for buildings that need inputs or have outputs
   - `create_logistics_tasks` turns these events into TaskSequences

2. **Task Assignment** (`src/workers/tasks/assignment.rs`):
   - BFS-based matching of workers to tasks
   - Considers network connectivity and distance

3. **Task Execution** (`src/workers/tasks/execution.rs`):
   - `process_worker_sequences`: Handles pathfinding and movement
   - `handle_sequence_task_arrivals`: Executes pickup/dropoff on arrival

---

## Known Issues

### The Production Jam Bug

**Location**: `src/structures/production.rs:127-193` (Producer handling in `crafter_logistics_requests`)

**The Problem**: Both input requests AND output removal requests are gated by the same check:
```rust
&& !existing_priorities.contains(&Priority::Medium)
```

**Failure Sequence**:
1. Smelter needs iron ore → sends request for inputs (Medium priority)
2. Task is created, worker starts bringing inputs
3. Smelter keeps producing outputs while waiting
4. System checks: "should I request output removal?"
5. Sees Medium priority task already exists → skips output removal
6. Outputs pile up, inventory fills with mix of inputs and outputs
7. Worker arrives with iron ore but can't deliver—inventory full
8. **Jam**

**Additional Factor**: Output removal only triggers when `produced_items.values().sum() >= 20` (WORKER_CAPACITY). Small/slow recipes might never hit this threshold before jamming.

### Output Destination Failure

**Location**: `src/workers/tasks/creation.rs:350-379` (`find_closest_storage_receiver`)

**The Problem**: If all Storage buildings are full, function returns `None`, and the pickup task is despawned without creating a dropoff (line 116). Outputs have nowhere to go.

### Priority System Unused

**Location**: `src/workers/tasks/components.rs:9`

The `Priority` enum (High, Medium, Low) is defined and attached to tasks, but task assignment doesn't actually sort by priority. Everything effectively gets FIFO/distance-based selection. The TODO at line 9 acknowledges this.

### Reactive vs Proactive

The current system is reactive—it responds to inventory changes rather than anticipating needs. This leads to:
- Always playing catch-up
- Race conditions when inventory changes rapidly
- Complex event handling logic

---

## Design Philosophy Discussion

### "Central Intelligence" vs Individual Agents

**User's Vision**: Workers should feel like extensions of a central intelligence (hive mind), not individual agents with their own goals.

**Current Reality**: The system is more "buildings broadcast needs, workers respond"—reactive rather than orchestrated.

**What Central Intelligence Would Look Like**:
- Global view of all production chains
- Predictive task creation (fetch inputs BEFORE building runs out)
- Bottleneck-aware prioritization (if smelter is starving downstream assemblers, prioritize it)
- Worker pre-positioning near high-traffic areas
- Coordinated multi-worker operations

**Current Priority**: Getting to a playable prototype first. Central intelligence improvements can come later once the foundation is solid.

---

## Relationship to Inventory Refactor

The task system's complexity is partly a symptom of the inventory system's complexity. With separate input/output buffers and declarative policies:

### Task Generation Simplifies

Current (reactive, complex branching):
```
Inventory changed →
  What type is this building? →
    If Producer: Check inputs AND outputs with complex threshold logic
    If Sender: Check outputs
    If Requester: Check inputs
  → Fire events if conditions met
```

With buffers (polling, uniform):
```
For each output buffer above offer threshold:
  → Create "items available" record
For each input buffer below request threshold:
  → Create "items needed" record
Match available to needed → Create transfer tasks
```

### Task Execution Simplifies

Current: Tasks specify items to transfer, but the source/destination inventory type affects behavior.

With buffers: Always "pickup from output buffer" or "dropoff to input buffer". The buffer type tells you everything.

### Priority Becomes Meaningful

With the declarative approach, priority can be derived from:
- How far below threshold an input buffer is (urgency)
- How backed up an output buffer is (pressure)
- Position in production chain (downstream starvation)

---

## Proactive Task System (`create_proactive_tasks`)

**Location**: `src/workers/tasks/creation.rs:394-496`

This system runs every 2 seconds and tries to create optimization tasks for idle workers:

1. **Sender → Storage**: Move items from full senders to storage
2. **Storage → Requester**: Proactively restock requesters running low
3. **Storage Balancing**: Move items between storage buildings to balance load

This is a good foundation for more intelligent behavior, but currently:
- Only runs when workers are idle
- Limited to `idle_count / 2` tasks per category
- Uses Low priority (but priority isn't used in assignment anyway)

---

## Files Reference

| File | Purpose |
|------|---------|
| `src/workers/tasks/mod.rs` | Plugin setup, system set ordering |
| `src/workers/tasks/components.rs` | Task, TaskSequence, Priority definitions |
| `src/workers/tasks/creation.rs` | Task generation from requests |
| `src/workers/tasks/assignment.rs` | Worker-to-task matching |
| `src/workers/tasks/execution.rs` | Task execution on arrival |
| `src/structures/production.rs` | Crafting logic and logistics request generation |

---

## Post-Refactor Considerations

Once the inventory refactor is complete, revisit:

1. **Implement actual priority sorting** in task assignment
2. **Derive priority dynamically** from buffer states
3. **Consider pull-based model**: Input buffers "pull" from upstream outputs, rather than push-based
4. **Batch task creation**: Instead of one sequence per transfer, batch nearby pickups
5. **Worker specialization**: Some workers handle certain routes/areas (future optimization)

---

## Test Coverage Gaps

From the codebase analysis:

- `src/workers/tasks/execution.rs` - **No tests** (complex, drives core behavior)
- `src/workers/tasks/assignment.rs` - **No tests**
- `src/workers/spawning.rs` - **No tests**

These should be prioritized after the refactor to prevent regressions.
