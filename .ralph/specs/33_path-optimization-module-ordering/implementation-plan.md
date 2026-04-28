# Implementation Plan: path-optimization-module-ordering

## Execution Rules

- One atomic step at a time, validated before moving on.
- Land the module-side algorithm and verify a single live test before deleting the host helper.
- Do not change the packet-32 host helpers (`apply_entity_order_proposal`, `LayerCollectionBuilderData`, dispatch arm) — they are consumed as-is.
- Do not introduce reversal usage in this packet; the flag is `false` for every entry.

## Steps

### Step 1: Port the NN algorithm into `path-optimization-default`

- Task IDs:
  - `TASK-152h`
- Objective:
  Add the deterministic nearest-neighbor permutation builder to the default path-optimization module without removing the host helper yet.
- Precondition:
  Packet 32 implemented (the SDK `LayerCollectionBuilder::set_entity_order`, `LayerCollectionBuilder::get_ordered_entities`, the `OrderedEntityView` SDK type, and the `collection: &mut LayerCollectionBuilder` trait parameter are reachable). `order_entities_by_nearest_neighbor` is still present in `crates/slicer-host/src/layer_executor.rs`.
- Postcondition:
  `modules/core-modules/path-optimization-default/src/lib.rs` declares a private `fn nearest_neighbor_permutation(entities: &[OrderedEntityView]) -> Vec<(u32, bool)>` that mirrors the packet-18 algorithm: start at `(0.0, 0.0)`, Euclidean distance from current cursor to each unpicked entity's `start_point` (in mm), advance current cursor to the picked entity's `end_point`, equality within 0.001 mm prefers `view.role == ExtrusionRole::BridgeInfill`, further ties go to lower `view.original_index`, reversal flag always `false`. The output `Vec<(u32, bool)>` is keyed on `view.original_index`, not on the slice index. `run_path_optimization` calls `let snapshot = collection.get_ordered_entities();` exactly once at the top, then `let items = nearest_neighbor_permutation(snapshot);`, then `collection.set_entity_order(items)?` exactly once. If `snapshot.is_empty()`, the function skips the call to `set_entity_order`. The existing inter-region travel-retraction logic that consumes `regions: &[PerimeterRegionView]` runs unchanged after the `set_entity_order` call.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
- Files to read (read-only reference, do not edit):
  - `crates/slicer-host/src/layer_executor.rs` — `order_entities_by_nearest_neighbor` and its two `execute_single_layer` call sites (algorithm source; lines bounded to the helper body + caller blocks)
  - `crates/slicer-sdk` headers for `LayerCollectionBuilder::set_entity_order`, `LayerCollectionBuilder::get_ordered_entities`, and `OrderedEntityView` (signatures only — already landed by packet 32)
  - `modules/core-modules/path-optimization-default/src/lib.rs` (current `run_path_optimization` body, to identify the insertion point and confirm the existing `regions: &[PerimeterRegionView]` consumer is left unchanged)
- Expected sub-agent dispatches:
  - read-only worker (FACT/SNIPPETS, ≤ 30 lines) to extract the verbatim NN body — start cursor `(0.0, 0.0)`, Euclidean distance to `start_point`, advance to `end_point`, 0.001 mm equality, BridgeInfill prefer, lower `original_index` tiebreak, reversal `false`
  - one editing worker to add the private `nearest_neighbor_permutation` helper and the single `get_ordered_entities()` + single `set_entity_order(items)?` calls in `path-optimization-default/src/lib.rs` (with empty-snapshot early-exit)
  - one validation worker to run `cargo build -p path-optimization-default`, `./modules/core-modules/build-core-modules.sh`, and the two grep counts (PASS/FAIL summary only)
- Context cost: M
- Authoritative docs:
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/ShortestPath.cpp`
- Verification:
  - `cargo build -p path-optimization-default`
  - `./modules/core-modules/build-core-modules.sh`
  - `grep -c "collection.get_ordered_entities()" modules/core-modules/path-optimization-default/src/lib.rs | grep -E "^1$"`
  - `grep -c "collection.set_entity_order(" modules/core-modules/path-optimization-default/src/lib.rs | grep -E "^1$"`
- Exit condition:
  Module builds. WASM artifact rebuilds successfully. Both grep counts are exactly `1`.

### Step 2: Rewrite the same-object acceptance test against live dispatch

- Task IDs:
  - `TASK-152h`
- Objective:
  Prove the module-side ordering produces the packet-18 expected result end-to-end before deleting the host helper. This step pins the algorithm/index-space alignment.
- Precondition:
  Step 1 complete; `path-optimization-default.wasm` rebuilt against the new code.
- Postcondition:
  `crates/slicer-host/tests/path_ordering_tdd.rs` has `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` rewritten to:
  1. build `Blackboard` with a minimal mesh containing object `"obj"`
  2. build `ExecutionPlan` whose per-layer stages are `Layer::Infill` (mock seeds an `InfillIR` whose `sparse_infill` paths have start x `[0.0, 30.0, 10.0]`) followed by `Layer::PathOptimization` (real `WasmRuntimeDispatcher` loaded with `path-optimization-default.wasm`)
  3. run `execute_per_layer`
  4. assert the produced `LayerCollectionIR.ordered_entities` start-x sequence is `[0.0, 10.0, 30.0]` and `topo_order` is `[0, 1, 2]`
- Files expected to change:
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
- Files to read (read-only reference, do not edit):
  - `crates/slicer-host/tests/finalization_live_tdd.rs` — canonical live-WASM-dispatch pattern (`Blackboard` build, `ExecutionPlan` with `WasmRuntimeDispatcher`, `execute_per_layer`)
  - `crates/slicer-host/tests/path_ordering_tdd.rs` — current packet-18 assertions for `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` (preserve assertion content, replace the dispatch path)
- Expected sub-agent dispatches:
  - one editing worker to rewrite `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` to load `path-optimization-default.wasm` via `WasmRuntimeDispatcher`, drive `Layer::Infill` (mock seeds `InfillIR` with start-x `[0.0, 30.0, 10.0]`) followed by `Layer::PathOptimization` (real module), and assert post-dispatch start-x `[0.0, 10.0, 30.0]` and `topo_order [0, 1, 2]`
  - one validation worker to run the single targeted `cargo test` from § Verification (pass/fail summary; on FAIL, return failing assertion and ≤ 20 lines of relevant code only)
- Context cost: M
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- Verification:
  - `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
- Exit condition:
  Test green via the live module path. If the start-x sequence does not match, the algorithm's index-space alignment is wrong — fix Step 1 before continuing.

### Step 3: Rewrite the cross-object, bridge, determinism, and no-op acceptance tests

- Task IDs:
  - `TASK-152h`
- Objective:
  Convert the remaining four packet-18 acceptance tests to the live dispatch pattern.
- Precondition:
  Step 2 complete and green.
- Postcondition:
  `crates/slicer-host/tests/path_ordering_tdd.rs` has the following tests rewritten to the live-dispatch pattern from Step 2:
  - `cross_object_ordering_resequences_entities_by_travel_cost`
  - `bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill`
  - `path_ordering_is_deterministic_across_repeated_runs`
  - `single_or_already_optimal_sequence_is_left_unchanged`
  Each test's fixtures are identical to the packet-18 fixtures (start positions, roles, object IDs); only the dispatch path changes.
- Files expected to change:
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
- Files to read (read-only reference, do not edit):
  - `crates/slicer-host/tests/path_ordering_tdd.rs` — current packet-18 assertions for the four tests (preserve assertion content; replace the dispatch path)
  - the rewritten `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` from Step 2 — copy its scaffolding (Blackboard build, plan layout) per test
- Expected sub-agent dispatches:
  - one editing worker to rewrite the four tests in a single pass (they share fixture scaffolding from Step 2): cross-object `["A","B","B","A"]`, bridge-priority equidistant tiebreak, byte-identical determinism across two runs, single/already-optimal no-op
  - one validation worker to run all four targeted `cargo test`s and report pass/fail per test (no full logs)
- Context cost: M
- Authoritative docs:
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp`
- Verification:
  - `cargo test -p slicer-host --test path_ordering_tdd cross_object_ordering_resequences_entities_by_travel_cost -- --exact --nocapture`
  - `cargo test -p slicer-host --test path_ordering_tdd bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill -- --exact --nocapture`
  - `cargo test -p slicer-host --test path_ordering_tdd path_ordering_is_deterministic_across_repeated_runs -- --exact --nocapture`
  - `cargo test -p slicer-host --test path_ordering_tdd single_or_already_optimal_sequence_is_left_unchanged -- --exact --nocapture`
- Exit condition:
  All four tests green via live dispatch.

### Step 4: Add the fallback-removal proof test

- Task IDs:
  - `TASK-152h`
- Objective:
  Add an explicit assertion that the host no longer applies its own ordering when the module emits no proposal.
- Precondition:
  Step 3 complete.
- Postcondition:
  `crates/slicer-host/tests/path_ordering_tdd.rs` has a new test `no_module_proposal_leaves_raw_assembled_order` that:
  1. uses the same 3-entity fixture as `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` (raw start-x `[30.0, 0.0, 10.0]`)
  2. uses a stub `LayerStageRunner` for `Layer::PathOptimization` that returns `Success` without ever pushing a proposal (no `set_entity_order` call)
  3. asserts the produced `LayerCollectionIR.ordered_entities` start-x sequence is `[30.0, 0.0, 10.0]` (raw assembly order, NOT NN-reordered).
  This test currently fails because `order_entities_by_nearest_neighbor` is still active; that is intentional — it goes green in Step 5.
- Files expected to change:
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
- Files to read (read-only reference, do not edit):
  - `crates/slicer-host/tests/path_ordering_tdd.rs` — Step 2's `same_object_nearest_neighbor_ordering_is_applied_before_path_optimization` for fixture reuse
  - `crates/slicer-host/src/dispatch.rs` — to confirm what a "no-proposal" `LayerStageRunner` return value looks like (it must reach `apply_entity_order_proposal` without proposing, i.e., return `Success` without invoking `set_entity_order`)
- Expected sub-agent dispatches:
  - one editing worker to add `no_module_proposal_leaves_raw_assembled_order` reusing the 3-entity raw start-x `[30.0, 0.0, 10.0]` fixture and substituting a stub `LayerStageRunner` for `Layer::PathOptimization` that returns `Success` without ever calling `set_entity_order`
  - one validation worker to run the single targeted `cargo test`; expected to FAIL at this step (host fallback still active) — record the failing assertion text for confirmation that Step 5 is the right fix
- Context cost: S
- Verification:
  - `cargo test -p slicer-host --test path_ordering_tdd no_module_proposal_leaves_raw_assembled_order -- --exact --nocapture`
- Exit condition:
  Test compiles and runs. It is allowed (and expected) to fail at this step — failure means "host fallback still active," which is exactly what Step 5 removes.

### Step 5: Delete `order_entities_by_nearest_neighbor`, its call sites, and the obsolete packet-32 fallback test

- Task IDs:
  - `TASK-152h`
- Objective:
  Remove the host helper, update its two call sites in `execute_single_layer` to use raw assembled order directly, and delete the now-obsolete test that locked in the host-fallback contract.
- Precondition:
  Steps 1–4 complete; the live-dispatch tests are green; the no-proposal proof test is in place.
- Postcondition:
  - `crates/slicer-host/src/layer_executor.rs` no longer contains `pub fn order_entities_by_nearest_neighbor`.
  - The pre-PathOptimization staging block uses `assemble_ordered_entities(...)` directly: `let ordered_entities = assemble_ordered_entities(layer.index, arena.perimeter(), arena.infill(), arena.support());`.
  - The no-PathOptimization fallback block at the end of `execute_single_layer` uses the same direct call.
  - The `ExtrusionRole` import at the top of the file is removed if it became unused.
  - `crates/slicer-host/src/lib.rs` removes `order_entities_by_nearest_neighbor` from `pub use layer_executor::{...}`.
  - `crates/slicer-host/tests/path_ordering_tdd.rs` no longer contains the test `reordered_sequence_is_consumed_by_path_optimization_stage` or its supporting fixture (`LiveStageCapture` helper, if it is unused after the deletion). Any module-level comment referencing the deleted test is updated.
- Files expected to change:
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/lib.rs`
  - `crates/slicer-host/tests/path_ordering_tdd.rs`
- Files to read (read-only reference, do not edit):
  - `crates/slicer-host/src/layer_executor.rs` — confirm helper body, both call sites in `execute_single_layer`, and which imports (`ExtrusionRole`, etc.) become unused after the delete
  - `crates/slicer-host/src/lib.rs` — locate the `pub use layer_executor::{...}` line
  - `crates/slicer-host/tests/path_ordering_tdd.rs` — confirm `reordered_sequence_is_consumed_by_path_optimization_stage` and any `LiveStageCapture` helper are dead after removal
- Expected sub-agent dispatches:
  - one editing worker to delete the helper, update both call sites in `execute_single_layer` to use `assemble_ordered_entities` directly, drop unused imports, remove the re-export, and delete the obsolete test (and any unused helper) in one pass
  - one validation worker to run the two grep checks, `cargo build -p slicer-host`, and the three targeted `cargo test`s; the Step 4 fallback-removal test must now be GREEN; the Step 2 live-dispatch test must remain GREEN; packet-32 `layer_collection_builder_tdd` must remain GREEN
- Context cost: M
- Authoritative docs:
  - `docs/01_system_architecture.md`
- Verification:
  - `! grep -RIn "order_entities_by_nearest_neighbor" crates/slicer-host/`
  - `! grep -RIn "reordered_sequence_is_consumed_by_path_optimization_stage" crates/slicer-host/tests/`
  - `cargo build -p slicer-host`
  - `cargo test -p slicer-host --test path_ordering_tdd no_module_proposal_leaves_raw_assembled_order -- --exact --nocapture`
  - `cargo test -p slicer-host --test path_ordering_tdd same_object_nearest_neighbor_ordering_is_applied_before_path_optimization -- --exact --nocapture`
  - `cargo test -p slicer-host --test layer_collection_builder_tdd 2>&1 | grep "test result: ok"`
- Exit condition:
  Both greps return zero matches. Build is clean. The fallback-removal test is green. The live-dispatch test is still green. The packet-32 host validation tests (including the new read-projection tests) are still green.

### Step 6: Mark packet 18 superseded and record the deviation

- Task IDs:
  - `TASK-152h`
- Objective:
  Update the supersession metadata.
- Precondition:
  Step 5 complete; the migration is verifiably done.
- Postcondition:
  - `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md` frontmatter `status: implemented` is changed to `status: superseded`. A new top-level section `## Superseded By` is added with the body `Packet 33 (33_path-optimization-module-ordering) moved the entity-ordering algorithm from the host helper into path-optimization-default via the layer-collection-builder WIT surface introduced by packet 32. The algorithm is preserved; only its location moved.`
  - `docs/DEVIATION_LOG.md` gains an entry titled `path-optimization-module-ordering (2026-04-28)` summarizing the move, noting that packet 32 introduced the WIT surface and packet 33 consumed it, and noting that the NN algorithm and bridge-priority tiebreak are bit-identical to packet 18.
  - `docs/14_deviation_audit_history.md` cross-links the new deviation log entry.
- Files expected to change:
  - `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
- Files to read (read-only reference, do not edit):
  - `.ralph/specs/18_path-optimization-entity-ordering/packet.spec.md` — frontmatter and existing top-level headings (verify current `status: implemented`; locate the insertion point for `## Superseded By`)
  - `docs/DEVIATION_LOG.md` — newest-entry placement convention and existing entry format (mirror style)
  - `docs/14_deviation_audit_history.md` — cross-link table format and chronology
- Expected sub-agent dispatches:
  - one editing worker to: (a) flip packet 18 frontmatter to `status: superseded` and add the `## Superseded By` section pointing at packet 33, (b) add the `path-optimization-module-ordering (2026-04-28)` entry to `docs/DEVIATION_LOG.md` summarizing the move and noting the algorithm is bit-identical to packet 18, (c) cross-link the entry from `docs/14_deviation_audit_history.md`
  - one validation worker to run the three greps in § Verification (PASS/FAIL summary only)
- Context cost: S
- Verification:
  - `grep -E "^status:\\s*superseded" .ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`
  - `grep -E "33_path-optimization-module-ordering" .ralph/specs/18_path-optimization-entity-ordering/packet.spec.md`
  - `grep -E "path-optimization-module-ordering" docs/DEVIATION_LOG.md`
- Exit condition:
  All three greps match.

### Step 7: Close `TASK-152g` and `TASK-152h` in the backlog

- Task IDs:
  - `TASK-152h`
- Objective:
  Reflect the closed migration in `docs/07_implementation_status.md`.
- Precondition:
  Step 6 complete.
- Postcondition:
  - `docs/07_implementation_status.md` `TASK-152g` row is `[x]` with a close note pointing at packet 33: *"Closed 2026-04-28 — packet 33 consumes the layer-collection-builder surface end-to-end."*
  - `docs/07_implementation_status.md` gains a new `TASK-152h` row under the `TASK-152` group (TASK-152h does not exist in the backlog at packet-33 start; this step CREATES it, using the wording from `task-map.md § Backlog Delta Summary`) and that row is `[x]` with the close note: *"Closed 2026-04-28 — packet 33 ports NN ordering into path-optimization-default and removes order_entities_by_nearest_neighbor from slicer-host."*
  - `TASK-152` parent stays `[~]` (152b/c/f remain open).
- Files expected to change:
  - `docs/07_implementation_status.md`
- Files to read (read-only reference, do not edit):
  - `docs/07_implementation_status.md` — the rows for `TASK-152`, `TASK-152a..g` (locate the `TASK-152g` row to update and the correct insertion point for the new `TASK-152h` row under the same group)
  - `task-map.md § Backlog Delta Summary` — authoritative wording for the new `TASK-152h` row body
- Expected sub-agent dispatches:
  - one editing worker to (a) flip `TASK-152g` to `[x]` with the close note, (b) insert the new `TASK-152h` row directly under `TASK-152g` with the wording from `task-map.md § Backlog Delta Summary` and mark it `[x]`, (c) leave `TASK-152` parent at `[~]` and the other 152x rows untouched
  - one validation worker to run the three greps in § Verification (PASS/FAIL summary)
- Context cost: S
- Authoritative docs:
  - `docs/07_implementation_status.md`
- Verification:
  - `grep -E "^- \\[x\\] TASK-152g" docs/07_implementation_status.md`
  - `grep -E "^- \\[x\\] TASK-152h" docs/07_implementation_status.md`
  - `grep -E "^- \\[~\\] TASK-152 " docs/07_implementation_status.md`
- Exit condition:
  All three greps match.

### Step 8: Run the packet's full acceptance ceremony

- Task IDs:
  - `TASK-152h`
- Objective:
  Re-run every acceptance command from `packet.spec.md` and prove the workspace is clean.
- Precondition:
  Steps 1–7 complete.
- Postcondition:
  Every command in `packet.spec.md`'s Verification block passes; `cargo clippy --workspace -- -D warnings` is clean; `./modules/core-modules/build-core-modules.sh` succeeds.
- Files expected to change:
  - none
- Files to read (read-only reference, do not edit):
  - `packet.spec.md § Verification` — the authoritative command list
- Expected sub-agent dispatches:
  - one validation worker to run every command in `packet.spec.md § Verification` and return only a PASS/FAIL summary per command (no full logs); a single FAIL halts the ceremony and returns the failing assertion + ≤ 20 lines of relevant code
  - one targeted-fix editing worker, dispatched only if a command surfaces a regression — scope strictly to the originating step's files-to-edit; do NOT widen scope from this ceremony step
- Context cost: S
- Verification:
  - run every command in `packet.spec.md` § Verification
- Exit condition:
  Every command succeeds.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands in `packet.spec.md` pass.
- `! grep -RIn "order_entities_by_nearest_neighbor" crates/slicer-host/` returns zero matches.
- `cargo build --workspace` and `cargo clippy --workspace -- -D warnings` are clean.
- `./modules/core-modules/build-core-modules.sh` succeeds.
- Packet 18 frontmatter is `status: superseded`.
- `docs/DEVIATION_LOG.md` documents the move.
- `docs/07_implementation_status.md` has `TASK-152g` and `TASK-152h` closed.

## Acceptance Ceremony

- Re-run every acceptance command from `packet.spec.md`.
- Confirm packet-32 host validation tests (`layer_collection_builder_tdd`) still pass — the host helper API is unchanged.
- Confirm `path_ordering_tdd` tests are now driven through real WASM dispatch (no direct calls to a host helper).
- Confirm the no-proposal fallback yields raw assembly order, not NN ordering.
- Record any remaining packet-local risk before status changes.
