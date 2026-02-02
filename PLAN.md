# UI Refactor: State Machine Architecture

## Goal
Replace ad-hoc UI state management with a `UiMode` state machine, adopt Bevy 0.18 headless widgets, unify visual style across all UI elements, and reorganize files into panels/popups/modes directories.

## Branch
`refactor/ui-state-machine`

## Progress
- [x] Phase 1: UiMode State Machine + Escape Unification
- [x] Phase 2: style.rs + Headless Widget Migration + Style Unification
- [x] Phase 3: Move Placement Ghost to modes/
- [x] Phase 4: File Migration to Subdirectories
- [x] Phase 5: Unified Scroll System
- [x] Phase 6: BuildingClickEvent Cleanup + Final Polish

## File Structure (final)
```
src/ui/
├── mod.rs                          # UIPlugin, UiMode enum, UISystemSet, unified escape, scroll
├── style.rs                        # ButtonStyle component + global apply_button_styles system
├── panels/
│   ├── mod.rs                      # PanelsPlugin
│   ├── sidebar/
│   │   ├── mod.rs                  # SidebarPlugin, Sidebar component, toggle/layout
│   │   ├── tabs.rs                 # SidebarTab, tab switching (RadioGroup)
│   │   └── building_buttons.rs     # BuildingButton, SelectedBuilding, selection (RadioGroup)
│   ├── workflow_list.rs            # WorkflowListPlugin (from workflow_panel.rs)
│   └── production_hud.rs           # ProductionHudPlugin (merges production_display + score_display)
├── popups/
│   ├── mod.rs                      # PopupsPlugin
│   ├── building_menu.rs            # BuildingMenu, BuildingClickEvent
│   ├── tooltip.rs                  # Tooltip system
│   └── workflow_action.rs          # WorkflowActionPopup (extracted from workflow_creation.rs)
└── modes/
    ├── mod.rs                      # ModesPlugin, OnEnter/OnExit hooks
    ├── placement.rs                # PlacementGhost (from systems/display.rs), error messages
    └── workflow_create.rs          # WorkflowCreationState, creation panel, controls
```

## Phases

### Phase 2: style.rs + Headless Widget Migration + Style Unification
Replace `InteractiveUI`/`DynamicStyles`/`Selectable` with Bevy 0.18 headless widgets. Unify all UI visuals to the workflow panel's blue-tinted dark palette.

**src/ui/style.rs** — already created with:
- Canonical color palette constants (PANEL_BG, CARD_BG, POPUP_BG, PANEL_BORDER, HEADER_COLOR, TEXT_COLOR, DIM_TEXT, BUTTON_BG, BUTTON_HOVER, CONFIRM_BG, CONFIRM_HOVER, CANCEL_BG, CANCEL_HOVER, SELECTED_BG, SELECTED_BORDER)
- `ButtonStyle` component with presets: `default_button()`, `confirm()`, `cancel()`, `close()`, `tab()`, `building_button()`
- `apply_button_styles` system using `Has<Pressed>`, `&Hovered`, `Has<Checked>` (headless widget API)
- `apply_button_styles_on_uncheck` system for `RemovedComponents<Checked>` / `RemovedComponents<Pressed>`
- `StylePlugin` registered in `UISystemSet::VisualUpdates`

**Still TODO for Phase 2:**
1. Register `UiWidgetsPlugins` in main.rs (or UIPlugin)
2. Register `style` module and `StylePlugin` in `ui/mod.rs`
3. Migrate each consumer file from `InteractiveUI`/`Selectable`/old `Button` to headless widgets + `ButtonStyle`:
   - `building_buttons.rs`: replace `InteractiveUI`/`Selectable(Exclusive)` with headless `Button` + `RadioButton` in a `RadioGroup`. `Checked` replaces `is_selected`. Adopt `ButtonStyle::building_button()`.
   - `sidebar_tabs.rs`: replace `Selectable` tabs with `RadioGroup`/`RadioButton`. `Checked` replaces `is_active`. Adopt `ButtonStyle::tab()`.
   - `building_menu.rs`: replace `InteractiveUI` on close button and recipe selectors with headless `Button`/`RadioButton` + `ButtonStyle::close()`. Use `PANEL_BG`/`PANEL_BORDER` for menu surfaces.
   - `sidebar.rs`: replace `InteractiveUI` on toggle/close buttons. Use `PANEL_BG`/`PANEL_BORDER` for container. Use `HEADER_COLOR` for title.
   - `workflow_panel.rs`: replace `update_panel_button_hover_visuals` with `ButtonStyle` presets. Replace inline color consts with imports from `style.rs`.
   - `workflow_creation.rs`: replace `update_button_hover_visuals` with `ButtonStyle` presets. Replace inline color consts with imports from `style.rs`.
   - `spawn_worker_button.rs`: replace inline hover logic with `ButtonStyle::default_button()`.
   - `production_display.rs`: adopt `PANEL_BG`, `TEXT_COLOR`, `HEADER_COLOR`. Reduce font sizes for consistency.
   - `score_display.rs`: adopt `HEADER_COLOR` for score text. Reduce font size.
4. Delete `interaction_handler.rs` entirely
5. Delete all per-file color constants (duplicated in workflow_panel.rs and workflow_creation.rs)

**Style changes per element:**

| Element | Current | New |
|---------|---------|-----|
| Sidebar bg | neutral `(0.1, 0.1, 0.1)` | blue-tinted `PANEL_BG` |
| Sidebar border | neutral gray `(0.3, 0.3, 0.3)` | blue `PANEL_BORDER` |
| Sidebar header text | default white | `HEADER_COLOR` (light blue-white) |
| Sidebar close button | red `(0.4, 0.2, 0.2)` | `CANCEL_BG` pattern |
| Sidebar toggle button | neutral gray `(0.2, 0.2, 0.2)` | `BUTTON_BG` pattern |
| Tab default bg | neutral `(0.15, 0.15, 0.15)` | `BUTTON_BG` |
| Tab selected bg | green `(0.2, 0.3, 0.2)` | `SELECTED_BG` (blue-tinted) |
| Building button bg | neutral `(0.2, 0.2, 0.2)` | `BUTTON_BG` |
| Building button selected | green `(0.3, 0.4, 0.2)` | `SELECTED_BG` |
| Building button cost text | `(0.8, 0.8, 0.8)` | `DIM_TEXT` |
| Production HUD bg | neutral `(0.1, 0.1, 0.1, 0.8)` | `PANEL_BG` |
| Production HUD text | default white, 24px | `TEXT_COLOR`, 14px (consistent sizing) |
| Score text | gold `(0.9, 0.85, 0.2)`, 28px | `HEADER_COLOR`, 14px |
| Spawn worker button | inline gray hover | `ButtonStyle::default_button()` |
| Building menu bg | inline values | `PANEL_BG` / `POPUP_BG` |
| Workflow panels | already correct | no change (source of truth) |

**Headless widget key patterns:**
- `bevy::ui_widgets::Button` (NOT `bevy::prelude::Button`) — handles Press/Release/Click, emits `Activate`, requires `AccessibilityNode(Role::Button)`
- `RadioGroup` + `RadioButton` — arrow key navigation, emits `ValueChange<Entity>`. Requires `Checkable`. App must respond to events to insert/remove `Checked`.
- `Hovered::default()` must be added to entities that need hover detection
- `Pressed` is a marker component (not the old `Interaction::Pressed`)
- `RemovedComponents<Checked>` and `RemovedComponents<Pressed>` needed for style cleanup when markers are removed

### Phase 3: Move Placement Ghost to modes/
Move placement-related code from `systems/display.rs` to `ui/modes/placement.rs`.

**src/ui/modes/placement.rs** — move here:
- `PlacementGhost`, `PlacementErrorMessage` components
- `update_placement_ghost` (gated with `.run_if(in_state(UiMode::Place))`)
- `display_placement_error`, `cleanup_placement_errors`

**src/systems/display.rs** — remove the 3 systems and 2 components listed above. Keep `update_inventory_display`, `update_operational_indicators`, `InventoryDisplay`, `NonOperationalIndicator`.

**src/systems/mod.rs** — remove re-exports of moved items; remove from `SystemsSet::Display` registration.

**src/ui/modes/mod.rs** — register `PlacementPlugin` systems in `UISystemSet::VisualUpdates` (safe to move from `SystemsSet::Display` since ghost is visual-only; grid coords computed in earlier `GridUpdate` set).

The ghost-during-workflow bug is now structurally impossible.

### Phase 4: File Migration to Subdirectories
Move all remaining flat UI files into the subdirectory structure. This is the largest phase — parallelize across sub-agents.

**Parallel work streams:**

**Stream A — Panels:**
- `sidebar.rs` → `panels/sidebar/mod.rs`
- `sidebar_tabs.rs` → `panels/sidebar/tabs.rs`
- `building_buttons.rs` → `panels/sidebar/building_buttons.rs`
- `production_display.rs` + `score_display.rs` → `panels/production_hud.rs` (merge into `ProductionHudPlugin`)
- `workflow_panel.rs` → `panels/workflow_list.rs` (rename plugin to `WorkflowListPlugin`)
- `spawn_worker_button.rs` → `panels/production_hud.rs` (fold in, or keep as separate file under panels/)

**Stream B — Popups:**
- `building_menu.rs` → `popups/building_menu.rs`
- `tooltips.rs` → `popups/tooltip.rs`
- Extract `WorkflowActionPopup` + `handle_action_selection` from `workflow_creation.rs` → `popups/workflow_action.rs`

**Stream C — Modes:**
- `workflow_creation.rs` (remainder after popup extraction) → `modes/workflow_create.rs`
- Remove `WorkflowCreationState.active` field (all consumers now use `in_state()`)

**After all streams:**
- Update `src/ui/mod.rs`: replace flat `pub mod` + `pub use *` with new module tree + targeted re-exports for types used externally (`SelectedBuilding`, `BuildingClickEvent`, `PlacementGhost`, `WorkflowCreationState`)
- Update external imports in `structures/placement.rs` and `systems/display.rs`
- Delete all old flat files

### Phase 5: Unified Scroll System
Replace per-panel manual scroll with a single observer-based system.

**src/ui/mod.rs** (or a dedicated `scroll.rs`):
- Define `ScrollEvent` as `EntityEvent` with propagation
- `send_scroll_events` system: reads `MouseWheel`, finds hovered scroll nodes, triggers `ScrollEvent` on them
- `on_scroll_handler` observer: updates `ScrollPosition` clamped to content bounds, consumes event to prevent propagation

**Remove:**
- `handle_building_menu_scroll` from `popups/building_menu.rs`
- `handle_workflow_scroll` from `panels/workflow_list.rs`

### Phase 6: BuildingClickEvent Cleanup + Final Polish
- Remove `click_events.clear()` from workflow create mode handler (no longer needed since consumers are mode-gated)
- Remove any remaining dead code (`Selectable`, `SelectionBehavior`, etc.)
- Ensure all `pub use` re-exports in `ui/mod.rs` are minimal
- Run full `cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test --all-targets`

**Commit. Merge to main. Push.**

## Key Decisions
- `UiMode` is a top-level `States` enum, not a `SubStates` — there's no parent state to scope it under
- `SelectedBuilding` resource is kept; mode transitions are driven by changes to it (not the reverse)
- `WorkflowCreationState.active` is kept temporarily in Phase 1 for compat, removed in Phase 4
- Placement ghost moves from `SystemsSet::Display` to `UISystemSet::VisualUpdates` — safe because it's visual-only
- `handle_building_input` (in `BuildingSystemSet::Input` / `DomainOperations`) gets `not(in_state(WorkflowCreate))` run condition — it needs to work in both Observe (right-click remove) and Place (left-click place)
- Full headless widget migration chosen over simpler Interaction-based approach for: accessibility, keyboard navigation, proper press/release semantics, Activate events, InteractionDisabled, ValueChange on RadioGroup, future-proofing

## Verification
After each phase:
1. `cargo fmt --check` passes
2. `cargo clippy --all-targets -- -D warnings` passes
3. `cargo test --all-targets` passes
4. Manual smoke test:
   - Select building → ghost appears, click to place, Escape deselects
   - Press N → workflow creation panel, click buildings to add steps, confirm/cancel
   - Click placed building → building menu opens with status/storage/crafting
   - Tab → workflow panel toggles
   - B → sidebar toggles, 1/2/3 switch tabs
   - Verify: no ghost during workflow creation, Escape doesn't break sidebar tabs, menus close properly on mode transitions

## Files Modified Outside UI
- `Cargo.toml` — add `experimental_bevy_ui_widgets` feature (done in Phase 1)
- `src/structures/placement.rs` — remove `WorkflowCreationState` import/param, add run condition (done in Phase 1)
- `src/structures/mod.rs` — add `in_state` run condition to `handle_building_input` (done in Phase 1)
- `src/systems/display.rs` — remove placement ghost code (Phase 3)
- `src/systems/mod.rs` — remove placement ghost re-exports and registrations (Phase 3)
