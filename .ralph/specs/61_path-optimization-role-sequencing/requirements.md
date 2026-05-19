# Requirements: 61_path-optimization-role-sequencing

## Packet Metadata

- Grouped task IDs:
  - `TASK-152h` — move NN entity-ordering algorithm into path-optimization-default (the foundation this packet extends)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The current `path-optimization-default` module applies a pure distance-minimizing nearest-neighbor heuristic across ALL entities within a tool cluster, with zero awareness of extrusion-role priority. An InnerWall entity at (100, 50) and a SparseInfill entity at (10, 50) produces Infill → InnerWall if the infill is closer — this interleaves roles, increases travel moves and retractions, and degrades print quality. The only role-aware behavior is a BridgeInfill tie-break preference (lib.rs:73-78), which is a local hack, not a structural fix.

OrcaSlicer's `process_layer` (`GCode.cpp:5415-5429`) orders entities by role group before applying distance optimization: perimeters (inner → outer) first, then all infill types chained together, then ironing. The nearest-neighbor heuristic runs within each role group, not across them. This packet brings the same structural guarantee to `path-optimization-default`.

## In Scope

- Add `WallSequence` enum (`InnerOuter`, `OuterInner`) to `path-optimization-default/src/lib.rs`.
- Add `role_group(&self, role: &ExtrusionRole) -> u32` method on `PathOptimizationDefault`, mapping each extrusion role to a group number with wall_sequence-driven swap of InnerWall/OuterWall groups.
- Refactor `group_then_nearest_neighbor` to two-level grouping: tool cluster → role group → nearest_neighbor_permutation within each role group.
- Remove the BridgeInfill tie-break from `nearest_neighbor_permutation` (lib.rs lines 73-78) — dead after role grouping.
- Add `wall_sequence` config key to `path-optimization-default.toml` manifest (`type="enum"`, values `["inner_outer", "outer_inner"]`, default `"inner_outer"`).
- Parse `wall_sequence` in `on_print_start`; reject invalid values with `ModuleError::fatal`.
- Add unit tests (`#[cfg(test)] mod tests` in lib.rs) covering all ACs.
- Verify all existing path-optimization tests continue to pass.

## Out of Scope

- WIT interface changes — the output contract is unchanged.
- Host dispatch changes (`crates/slicer-host/src/dispatch.rs`, `layer_executor.rs`, `gcode_emit.rs`).
- IR schema changes (`crates/slicer-ir/`) — `ExtrusionRole::default_priority()` is not modified.
- SDK changes (`crates/slicer-sdk/`).
- Core-module manifest structural changes beyond the single new config key.
- Per-role configurable priority overrides (deferred to a future packet).
- Cross-layer tool rotation logic (`apply_cross_layer_tool_rotation` in gcode_emit.rs) — orthogonal.
- Cooling, fan-speed, seam placement, or travel policy — unchanged.
- `wipe-tower` module edits — WipeTower/PrimeTower roles are handled as group 8 but their generation logic is not touched.

## Authoritative Docs

- `docs/01_system_architecture.md` — PathOptimization stage ordering, module claim `"path-optimizer"`, retract/Z-hop policy. Delegate SUMMARY (> 300 lines).
- `docs/05_module_sdk.md` — `LayerModule::run_path_optimization` signature, `LayerCollectionBuilder` contract, config access pattern. Load directly (SDK docs are ≈ 100 relevant lines).
- `docs/04_host_scheduler.md` — DAG validation, Host carries no NN ordering fallback (packet-33 note). Delegate SUMMARY.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — `process_layer` extrusion ordering loop (lines ~5381-5430) and `extrude_infill` (lines ~6082-6107) which chains all non-ironing infill types together.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` — `wall_sequence` config application (line ~2761) controlling inner/outer wall order baked into perimeter generation.

## Acceptance Summary

- Positive cases: **AC-1** through **AC-7** from `packet.spec.md`. Refinements:
  - AC-3 (walls before infill) extends to all wall role groups (InnerWall, OuterWall, ThinWall) vs all infill role group (BottomSolidInfill, TopSolidInfill, SparseInfill, BridgeInfill).
  - AC-4 (infill chained together) confirms the single NN chain — entities within the infill group are ordered by distance, not by infill subtype.
  - AC-6 (global sequence) proves the full 10-group ordering for all 14 ExtrusionRole variants.
  - AC-7 (determinism) is a cross-AC invariant: all role ordering tests must be deterministic.
- Negative cases: **AC-N1** covers invalid wall_sequence config rejection.
- Cross-packet impact: This packet builds on TASK-152h (existing NN algorithm) but does not modify any other packet's files.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p path-optimization-default --lib -- role_orders_inner_before_outer` | AC-1: InnerWall precedes OuterWall (default config) | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p path-optimization-default --lib -- role_orders_outer_before_inner` | AC-2: OuterWall precedes InnerWall (outer_inner config) | FACT pass/fail |
| `cargo test -p path-optimization-default --lib -- role_orders_walls_before_infill` | AC-3: walls before infill | FACT pass/fail |
| `cargo test -p path-optimization-default --lib -- role_chains_infill_together` | AC-4: infill types in one NN chain | FACT pass/fail |
| `cargo test -p path-optimization-default --lib -- role_handles_all_extrusion_roles` | AC-5: no panic on any role variant | FACT pass/fail |
| `cargo test -p path-optimization-default --lib -- role_preserves_global_sequence` | AC-6: full 10-group ordering | FACT pass/fail |
| `cargo test -p path-optimization-default --lib -- role_ordering_is_deterministic` | AC-7: deterministic output | FACT pass/fail |
| `cargo test -p path-optimization-default --lib -- role_rejects_invalid_wall_sequence` | AC-N1: invalid wall_sequence returns fatal error | FACT pass/fail |
| `cargo test -p path-optimization-default` | All existing + new path-optimization tests pass | FACT pass/fail |
| `cargo check --workspace` | Type-check whole workspace | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | Lint compliance | FACT pass/fail |
| `./modules/core-modules/build-core-modules.sh --check` | Guest WASM freshness | FACT: CLEAN or STALE: <list> |

## Step Completion Expectations

- **Cross-step invariant**: No step may change the signature of `nearest_neighbor_permutation` — it must continue to accept `&[&OrderedEntityView]` and return `Vec<(u32, bool)>` so all existing callers and the host dispatch contract remain valid.
- **Step ordering rationale**: Tests (Step 1) must be red first to validate they detect the pre-fix interleaving bug. Implementation (Steps 2-4) then turns them green. Config and cleanup (Steps 3-4) can be parallelized by separate sub-agents if desired.
- **Shared scratch state**: None — each step edits disjoint code regions or the same function incrementally.

## Context Discipline Notes

- **Large files in read-only path**: `crates/slicer-host/src/gcode_emit.rs` (~1600 lines) — must be ranged or delegated. Only `apply_cross_layer_tool_rotation` (lines 1495-1579) is relevant for verifying no interaction issues.
- **Likely temptation reads**: `crates/slicer-ir/src/slice_ir.rs` ExtrusionRole enum (lines 1318-1372) — already read via sub-agent; do not re-load. `crates/slicer-sdk/src/views.rs` OrderedEntityView (lines 419-432) — already read; do not re-load.
- **Heaviest dispatch**: The `build-core-modules.sh --check` dispatch at completion gate — return FACT (CLEAN or STALE: <list>).
