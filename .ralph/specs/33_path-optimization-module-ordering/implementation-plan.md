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
  - `docs/07_implementation_status.md` `TASK-152h` row is `[x]` with a close note: *"Closed 2026-04-28 — packet 33 ports NN ordering into path-optimization-default and removes order_entities_by_nearest_neighbor from slicer-host."*
  - `TASK-152` parent stays `[~]` (152b/c/f remain open).
- Files expected to change:
  - `docs/07_implementation_status.md`
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
