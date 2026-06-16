# Design: 61_path-optimization-role-sequencing

## Controlling Code Paths

- **Primary code path**: `group_then_nearest_neighbor` (`lib.rs:133-179`) — the single function that splits entities by tool index and applies nearest-neighbor. This is refactored to add a second level of grouping by role priority within each tool cluster.
- **Secondary code path**: `nearest_neighbor_permutation` (`lib.rs:43-103`) — the pure NN heuristic. BridgeInfill tie-break (lines 73-78) is removed; the function otherwise unchanged.
- **Config surface**: `PathOptimizationDefault::on_print_start` (`lib.rs:196-233`) — parses the new `wall_sequence` config key.
- **Manifest**: `path-optimization-default.toml` — adds `[config.schema.wall_sequence]` table.
- **Neighboring tests**: `#[cfg(test)] mod tests` at bottom of `lib.rs` — existing tests must continue to pass. New role-ordering unit tests added to the same module.
- **OrcaSlicer comparison surface**: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `./modules/core-modules/build-core-modules.sh --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- **Packet-specific**: The output contract of `run_path_optimization` is `-> Result<(), ModuleError>` with mutations applied through `LayerCollectionBuilder::set_entity_order` and `GcodeOutputBuilder::push_tool_change`. This contract must not change — the host dispatch at `crates/slicer-host/src/dispatch.rs` commits the builder state after the module returns, and any signature change here would require synchronized host and WASM ABI edits in scope of this packet.
- **Packet-specific**: `nearest_neighbor_permutation` must continue to accept `&[&OrderedEntityView]` and return `Vec<(u32, bool)>`. Its internal simplification (bridge tie-break removal) does not alter the signature.
- **Packet-specific**: Config key strings use `snake_case` per `CLAUDE.md` convention. The new key is `wall_sequence`, not `wall-sequence`.

## Code Change Surface

**Selected approach**: Add a `WallSequence` enum and `role_group(&self, role: &ExtrusionRole) -> u32` method on `PathOptimizationDefault`. Refactor `group_then_nearest_neighbor` into a method that, within each tool cluster, builds a `BTreeMap<u32, Vec<&OrderedEntityView>>` keyed by role group, then applies `nearest_neighbor_permutation` to each group's entities in ascending group order. This preserves the existing tool-cluster-first structure while adding role-priority subdivision.

**Exact changes**:

1. **`lib.rs`** — Add `WallSequence` enum (lines ~31-35 after `const DEFAULT_TRAVEL_Z_HOP`).
2. **`lib.rs`** — Add `role_group(&self, role: &ExtrusionRole) -> u32` method (near `tool_index_of`, lines ~40-52).
3. **`lib.rs`** — Refactor `group_then_nearest_neighbor` from free function to `PathOptimizationDefault` method; insert role-group BTreeMap inside the per-tool loop (lines ~140-180).
4. **`lib.rs`** — Simplify `nearest_neighbor_permutation` tie-break: replace lines 72-81 with a single `i < best_idx` fallback.
5. **`lib.rs`** — Add `wall_sequence` field to `PathOptimizationDefault` struct; parse in `on_print_start`.
6. **`lib.rs`** — Update `run_path_optimization` to call `self.group_then_nearest_neighbor()` instead of the free function.
7. **`lib.rs`** — Add `#[cfg(test)]` tests for all ACs.
8. **`path-optimization-default.toml`** — Add `[config.schema.wall_sequence]` table.

**Rejected alternatives**:

- **Reuse `ExtrusionRole::default_priority()`** — Rejected because that method is already consumed by the finalization builder (`traits.rs`) and wipe-tower for a different purpose (slot-based insertion ordering), and changing its values to match path-optimization semantics would break those consumers. The standalone `role_group()` avoids coupling.
- **Add a full `HashMap<ExtrusionRole, u32>` config for per-role overrides** — Rejected as over-scope. Deferred to a future packet. Only `wall_sequence` is configurable now.
- **Split infill types into separate groups (SolidInfill group, SparseInfill group, BridgeInfill group)** — Rejected in favor of matching OrcaSlicer's `extrude_infill` which chains all infill together in one nearest-neighbor pass.

## Files in Scope (read + edit)

- `modules/core-modules/path-optimization-default/src/lib.rs` — primary edit target: `WallSequence` enum, `role_group` method, `group_then_nearest_neighbor` refactor, `nearest_neighbor_permutation` tie-break removal, config parsing, tests.
- `modules/core-modules/path-optimization-default/path-optimization-default.toml` — add `wall_sequence` config schema entry.
- `crates/slicer-ir/src/slice_ir.rs` — read-only reference for `ExtrusionRole` enum variants (lines 1318-1347).

## Read-Only Context

- `crates/slicer-ir/src/slice_ir.rs` — lines 1318-1371 — `ExtrusionRole` enum and `default_priority()` (do not edit; read to confirm variant list for `role_group` match arms).
- `crates/slicer-sdk/src/views.rs` — lines 419-432 — `OrderedEntityView` struct (confirm fields `original_index`, `role`, `start_point`, `end_point`, `region_key`).
- `docs/05_module_sdk.md` — Layer::PathOptimization stage docs (~100 relevant lines; load directly).
- `crates/slicer-host/src/gcode_emit.rs` — lines 1495-1579 — `apply_cross_layer_tool_rotation` (confirm no interaction issues with role grouping).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate parity checks; never load.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` — host commitment of builder state; structure is read through delegation only if dispatch tests break.
- `crates/slicer-sdk/src/traits.rs` — finalization builder priority sorting; delegate a FACT check if needed.
- `wit/**/*.wit` — no WIT changes in scope.

## Expected Sub-Agent Dispatches

- "Run `cargo test -p path-optimization-default --lib`; return FACT pass/fail or SNIPPETS with failing test name + assertion + ≤ 20 lines" — purpose: validate existing tests pass before starting.
- "Run `cargo test -p path-optimization-default --lib -- role_orders_inner_before_outer`; return FACT" — purpose: validate AC-1.
- "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail" — purpose: lint gate after all edits.
- "Run `./modules/core-modules/build-core-modules.sh --check`; return FACT: CLEAN or STALE: <list>" — purpose: WASM freshness gate.
- "Search `crates/slicer-host/src/dispatch.rs` for calls to `set_entity_order` or `push_tool_change`; return LOCATIONS" — purpose: confirm no dispatch surface change needed.

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

## Context Cost Estimate

- **Aggregate**: M — 4 steps, all S except Step 2 (M for refactoring).
- **Largest single step**: Step 2 (M) — refactoring `group_then_nearest_neighbor` and adding `role_group`.
- **Highest-risk dispatch**: The `build-core-modules.sh --check` at completion — if stale, must rebuild WASM and re-run tests. Return format: FACT (CLEAN or STALE: <path list>).

## Open Questions

None — all design decisions resolved via user input during packet generation: inner/outer configurable via `wall_sequence` only, infill types chained together matching OrcaSlicer, bridge tie-break removed.
