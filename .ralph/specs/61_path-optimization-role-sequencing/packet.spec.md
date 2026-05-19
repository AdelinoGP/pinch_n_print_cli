---
status: draft
packet: 61_path-optimization-role-sequencing
task_ids:
  - TASK-152h
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 61_path-optimization-role-sequencing

## Goal

Add extrusion-role-priority grouping to `path-optimization-default`'s nearest-neighbor sequencing, ordering entities by role (InnerWall → OuterWall → Infill → Ironing → Support) before applying distance optimization within each role group, matching OrcaSlicer canonical behavior expressed by `GCode.cpp:5415-5429`.

## Scope Boundaries

The change modifies `group_then_nearest_neighbor` in `path-optimization-default/src/lib.rs` to introduce a two-level grouping: entities are first grouped by tool index, then subdivided by role priority via a new `role_group` method, with nearest-neighbor applied within each role sub-group. Adds a `wall_sequence` config key (`inner_outer` | `outer_inner`) to control InnerWall/OuterWall order, defaulting to `inner_outer`. Removes the now-redundant BridgeInfill tie-break from `nearest_neighbor_permutation`. No WIT, dispatch, IR, or SDK changes are required — the module's output contract (`Vec<(u32, bool)>` permutation + `ToolChangeRecord`) remains unchanged.

## Prerequisites and Blockers

- Depends on: TASK-152h (existing NN entity ordering in path-optimization-default) — must be complete. All existing path-optimization tests must pass before starting.
- Unblocks: cross-object ordering refinement; configurable per-role priority override maps (future).
- Activation blockers: None.

## Acceptance Criteria

- **AC-1. Given** two entities with roles InnerWall at (0, 100) and OuterWall at (0, 10), **when** `group_then_nearest_neighbor` runs with default `wall_sequence="inner_outer"`, **then** InnerWall precedes OuterWall in the output permutation regardless of distance. | `cargo test -p path-optimization-default --lib -- role_orders_inner_before_outer`
- **AC-2. Given** two entities with roles OuterWall at (0, 100) and InnerWall at (0, 10), **when** `wall_sequence="outer_inner"`, **then** OuterWall precedes InnerWall. | `cargo test -p path-optimization-default --lib -- role_orders_outer_before_inner`
- **AC-3. Given** three entities with roles InnerWall at (0, 100), SparseInfill at (0, 5), TopSolidInfill at (0, 20), **when** path-optimization runs, **then** all wall-role entities precede all infill-role entities (InnerWall before both SparseInfill and TopSolidInfill). | `cargo test -p path-optimization-default --lib -- role_orders_walls_before_infill`
- **AC-4. Given** three entities with roles SparseInfill at (0, 5), BridgeInfill at (0, 100), TopSolidInfill at (0, 20), **when** path-optimization runs, **then** all three infill entities are ordered by a single nearest-neighbor chain (not separated by infill subtype). | `cargo test -p path-optimization-default --lib -- role_chains_infill_together`
- **AC-5. Given** one entity per every `ExtrusionRole` variant (InnerWall, OuterWall, ThinWall, BottomSolidInfill, TopSolidInfill, SparseInfill, BridgeInfill, Ironing, SupportMaterial, SupportInterface, WipeTower, PrimeTower, Skirt, Custom), **when** `role_group` is called on each, **then** no variant panics (every arm is covered). | `cargo test -p path-optimization-default --lib -- role_handles_all_extrusion_roles`
- **AC-6. Given** entities for Skirt, InnerWall, OuterWall, ThinWall, all infill types, Ironing, SupportMaterial, SupportInterface, WipeTower, and Custom, **when** path-optimization runs, **then** roles sort in ascending group order: Skirt(0) < InnerWall(1) < OuterWall(2) < ThinWall(3) < infill(4) < Ironing(5) < SupportMaterial(6) < SupportInterface(7) < WipeTower/PrimeTower(8) < Custom(9). | `cargo test -p path-optimization-default --lib -- role_preserves_global_sequence`
- **AC-7. Given** a deterministic input set (fixed entity positions and roles), **when** path-optimization runs twice, **then** the output permutation is byte-identical across both runs. | `cargo test -p path-optimization-default --lib -- role_ordering_is_deterministic`

## Negative Test Cases

- **AC-N1. Given** config key `wall_sequence` set to `"invalid_value"` in `ConfigView`, **when** `on_print_start` is called, **then** the module returns `ModuleError::fatal` whose message contains the substring `"invalid_value"`. | `cargo test -p path-optimization-default --lib -- role_rejects_invalid_wall_sequence`

## Verification

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `./modules/core-modules/build-core-modules.sh --check`

## Authoritative Docs

- `docs/01_system_architecture.md` — PathOptimization stage ordering, module claims (`"path-optimizer"`), entity-ordering ownership — delegate SUMMARY.
- `docs/05_module_sdk.md` — Layer::PathOptimization stage surface, `LayerCollectionBuilder` contract — load directly (≤ 100 relevant lines).
- `docs/04_host_scheduler.md` — DAG validation, fixed stage order, host carries no NN fallback — delegate SUMMARY.

## Doc Impact Statement (Required)

`none` — the change is internal to the path-optimization module; no IR fields, WIT types, scheduler rules, claim IDs, manifest schema, host service, or module SDK contracts change. The module's output contract (`Vec<(u32, bool)>` + `ToolChangeRecord`) is preserved.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `process_layer` extrusion ordering (perimeters → infill → infill(ironing)); `extrude_infill` chains all infill types together.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `wall_sequence` config controls inner/outer wall order baked into perimeter generation.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
