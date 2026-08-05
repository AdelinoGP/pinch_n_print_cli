---
status: implemented
packet: 61_path-optimization-role-sequencing
task_ids:
  - TASK-152h
---

# 61_path-optimization-role-sequencing

## Goal

Add extrusion-role-priority grouping to `path-optimization-default`'s nearest-neighbor sequencing, ordering entities by role (InnerWall → OuterWall → Infill → Ironing → Support) before applying distance optimization within each role group, matching OrcaSlicer canonical behavior expressed by `GCode.cpp:5415-5429`.

## Problem Statement

The current `path-optimization-default` module applies a pure distance-minimizing nearest-neighbor heuristic across ALL entities within a tool cluster, with zero awareness of extrusion-role priority. An InnerWall entity at (100, 50) and a SparseInfill entity at (10, 50) produces Infill → InnerWall if the infill is closer — this interleaves roles, increases travel moves and retractions, and degrades print quality. The only role-aware behavior is a BridgeInfill tie-break preference (lib.rs:73-78), which is a local hack, not a structural fix.

OrcaSlicer's `process_layer` (`GCode.cpp:5415-5429`) orders entities by role group before applying distance optimization: perimeters (inner → outer) first, then all infill types chained together, then ironing. The nearest-neighbor heuristic runs within each role group, not across them. This packet brings the same structural guarantee to `path-optimization-default`.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **Packet-specific**: The output contract of `run_path_optimization` is `-> Result<(), ModuleError>` with mutations applied through `LayerCollectionBuilder::set_entity_order` and `GcodeOutputBuilder::push_tool_change`. This contract must not change — the host dispatch at `crates/slicer-host/src/dispatch.rs` commits the builder state after the module returns, and any signature change here would require synchronized host and WASM ABI edits in scope of this packet.
- **Packet-specific**: `nearest_neighbor_permutation` must continue to accept `&[&OrderedEntityView]` and return `Vec<(u32, bool)>`. Its internal simplification (bridge tie-break removal) does not alter the signature.
- **Packet-specific**: Config key strings use `snake_case` per `CLAUDE.md` convention. The new key is `wall_sequence`, not `wall-sequence`.

## Data and Contract Notes

- **Output contract**: The module emits `Vec<(u32, bool)>` via `LayerCollectionBuilder::set_entity_order` and `Vec<ToolChangeRecord>` via `GcodeOutputBuilder::push_tool_change`. These signatures are unchanged. The host dispatch commits these into `LayerCollectionIR.ordered_entities` and `LayerCollectionIR.tool_changes` at `dispatch.rs:2884`.
- **Tool-cluster rotation interaction**: Host's `apply_cross_layer_tool_rotation` rotates clusters by tool index at layer boundaries (post-dispatch). Role grouping operates within each tool cluster — the rotation only changes which tool cluster is first. No interaction issue.
- **Determinism**: `BTreeMap` gives deterministic key ordering. Nearest-neighbor is deterministic (no randomness, only greedy distance comparison). Ties fall back to `original_index` ordering. Output is deterministic for a given input.

## Locked Assumptions and Invariants

- The `role_group` method MUST cover all `ExtrusionRole` variants. A new variant added to `ExtrusionRole` later would require a corresponding update to `role_group`.
- The module's `wall_sequence` config default (`"inner_outer"`) means InnerWall precedes OuterWall in the output permutation by default. This differs from the existing `ExtrusionRole::default_priority()` ordering (OuterWall=1000 < InnerWall=1500), which is intentional and not a bug — `default_priority()` serves a different purpose.
- Infill types (BottomSolidInfill, TopSolidInfill, SparseInfill, BridgeInfill) share the same group number and are chained together. Adding a new infill-type role (e.g., `InternalBridgeInfill`) would belong to the same group.
- `nearest_neighbor_permutation` tie-break for equidistant entities within a role group falls back to `i < best_idx` (lower original index wins). This is deterministic and matches the existing tie-break behavior minus the BridgeInfill preference.

## Risks and Tradeoffs

- **OrcaSlicer divergence**: OrcaSlicer's `wall_sequence` is a print config that controls perimeter generation, not emission-time reordering. This module applies it at path-optimization time because the upstream perimeter generation does not yet implement configurable inner/outer ordering. If perimeter generation gains this feature later, the module's `wall_sequence` may become a no-op for walls (but remains useful for other role groups).
- **Performance**: `BTreeMap` per tool cluster adds O(n log g) where g is the number of distinct role groups (max 10). This is negligible compared to the O(n²) nearest-neighbor pass. No measurable performance regression expected.
- **Chaining quality**: Combining all infill types into one nearest-neighbor pass means a SparseInfill entity may be followed by a TopSolidInfill entity if geometry dictates. This matches OrcaSlicer behavior. Users who want strict subtype ordering within infill would need a future `infill_sequence` config.
