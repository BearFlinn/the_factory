# Integration Test Suite

## Purpose

Verify runtime behavior across frames — not unit-level correctness. These tests exercise the full Bevy system loop: plugins, system sets, component composition, and cross-system interactions that unit tests cannot cover.

## Structure

```
tests/integration/
  main.rs              # Entry point, clippy allows for test code
  harness/
    app.rs             # headless_app() — full gameplay loop without rendering
    builders.rs        # Entity factories: spawn_building, spawn_worker, add_items_*
    assertions.rs      # Domain-specific asserts with entity context in failures
    time.rs            # Frame advancement: tick, tick_n, tick_seconds, tick_until
  scenarios/
    network.rs         # Network connectivity, pathfinding
    production.rs      # Crafting pipelines, operational status
    logistics.rs       # Worker workflows, item transfers, waiting states
    construction.rs    # Building placement, construction completion, auto-pull
```

`harness/` is infrastructure (rarely changes). `scenarios/` is test code (grows with features).

## Rules

- **Use builders** — never hand-roll entity component bundles. `spawn_building` uses `BuildingRegistry` so tests break when building definitions change.
- **Use time helpers** — never call `app.update()` directly. `tick()`, `tick_n()`, `tick_seconds()`, `tick_until()` handle deterministic time via `TimeUpdateStrategy::ManualDuration`.
- **Use assertion helpers** — `assert_operational`, `assert_not_operational`, `assert_worker_at`, etc. provide entity context on failure.
- **No println/dbg** — assertion messages are the sole failure output. Passing tests produce zero output.

## Naming

`subject_verb_expected_outcome` — e.g., `worker_completes_pickup_dropoff_cycle`, `operational_crafter_produces_output`.

## Invariants

`InvariantPlugin` runs automatically on every frame via `headless_app()`. Checks:
- Workers have all required components (Speed, Position, WorkerPath, Cargo, Transform)
- Buildings have all required components (Name, Position, Operational, Transform)
- WorkflowAssignment references point to live entities with Workflow component
- No worker has both WaitingForItems and WaitingForSpace simultaneously
- ConstructionSites have InputPort, BuildingCost, Position, Transform

If an invariant fails, fix the production code or test setup — never disable the invariant.

## Adding New Tests

1. Add test function in the appropriate `scenarios/*.rs` file
2. Use `headless_app()` to create the app
3. Use `ensure_grid_coordinates` before spawning buildings
4. Spawn entities with `spawn_building`, `spawn_worker`
5. Advance time with `tick_n` or `tick_seconds`
6. Assert with domain helpers or standard `assert!`

New scenario files: add `mod new_file;` in `scenarios/mod.rs`.
