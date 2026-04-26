# Implementation Plan: path-optimization-tool-order-and-cooling-policy

## Execution Rules

- One atomic step at a time, validated before moving on.
- Land mixed-tool tests before module changes (TDD red), then update the docs rejection path.
- All implementation work happens inside `modules/core-modules/path-optimization-default/src/lib.rs`. Do not edit `crates/slicer-host/src/layer_executor.rs` or `crates/slicer-host/src/dispatch.rs` — packets `32`/`33` already provide the host surfaces this packet consumes.
- Tests drive `path-optimization-default.wasm` through `WasmRuntimeDispatcher` (live dispatch), mirroring the pattern packet `33` established in `crates/slicer-host/tests/path_ordering_tdd.rs`.

## Steps

### Step 1: Add failing live-dispatch mixed-tool ordering tests

- Task IDs:
  - `TASK-152b`
- Objective:
  Freeze the exact grouped tool order and deferred `ToolChange` sequence on the live module path before the module-side helper is extended.
- Precondition:
  Packets `32` and `33` are `implemented`. Mixed-tool ordering is not yet computed inside `path-optimization-default`. The current `path-optimization-default.wasm` only does single-pass NN entity ordering.
- Postcondition:
  `crates/slicer-host/tests/tool_ordering_tdd.rs` exists with three tests — `mixed_tool_layer_emits_deterministic_tool_change_sequence`, `single_tool_layer_emits_no_synthetic_tool_changes`, `canonical_or_single_tool_sequences_emit_no_redundant_tool_changes` — each driving `path-optimization-default.wasm` through `WasmRuntimeDispatcher` and asserting on the resulting `LayerCollectionIR.ordered_entities[*]` tool sequence and `LayerCollectionIR.tool_changes` records. The mixed-tool test currently fails (the module's NN ordering ignores tool index); the single-tool and already-grouped tests already pass because they reduce to packet-`33` behavior.
- Files expected to change:
  - `crates/slicer-host/tests/tool_ordering_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp`
- Verification:
  - `cargo test -p slicer-host --test tool_ordering_tdd mixed_tool_layer_emits_deterministic_tool_change_sequence -- --exact --nocapture`
  - `cargo test -p slicer-host --test tool_ordering_tdd single_tool_layer_emits_no_synthetic_tool_changes -- --exact --nocapture`
  - `cargo test -p slicer-host --test tool_ordering_tdd canonical_or_single_tool_sequences_emit_no_redundant_tool_changes -- --exact --nocapture`
- Exit condition:
  All three tests compile. Mixed-tool test fails on raw output; single-tool and already-grouped tests pass.

### Step 2: Implement per-tool grouping inside `path-optimization-default`

- Task IDs:
  - `TASK-152`
  - `TASK-152b`
- Objective:
  Add the per-tool grouping step inside the module so the live dispatch produces the expected grouped permutation and emits the correct deferred `ToolChange` sequence.
- Precondition:
  Step 1 tests are in place. The module's packet-`33` NN helper is intact.
- Postcondition:
  `modules/core-modules/path-optimization-default/src/lib.rs` `run_path_optimization` performs:
  1. cluster the input regions by `tool_index` (preserving raw assembly order within each cluster as input to NN)
  2. for each cluster, compute a within-cluster NN permutation reusing the existing per-module helper
  3. concatenate cluster permutations in ascending `tool_index` order to form the final `Vec<(u32, bool)>` (reversal flag stays `false`)
  4. call `collection.set_entity_order(items)` exactly once
  5. iterate the final permutation; at each tool-index change between consecutive entries call `output.push_tool_change(prev_tool, next_tool)`
  Single-tool layers reduce to packet-`33` NN behavior with zero `push_tool_change` calls. Already-grouped multi-tool layers emit exactly one `push_tool_change` per real boundary and no synthetic redundant changes.
- Files expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
- Files explicitly **not** expected to change:
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/dispatch.rs`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`
  - `docs/04_host_scheduler.md`
  - `docs/05_module_sdk.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrderUtils.hpp`
- Verification:
  - `./modules/core-modules/build-core-modules.sh`
  - `cargo test -p slicer-host --test tool_ordering_tdd -- --nocapture`
  - `! grep -RIn "tool.*group\\|group.*by_tool\\|order_entities_by_tool" crates/slicer-host/src/layer_executor.rs`
  - `cargo test -p slicer-host --test path_ordering_tdd -- --nocapture` (regression on packet-`33` tests)
- Exit condition:
  Mixed-tool, single-tool, and redundant-change suppression tests are green. The grep for any host-side tool-grouping helper returns zero matches. All packet-`33` tests still pass.

### Step 3: Close cooling overrides explicitly on the documentation rejection path

- Task IDs:
  - `TASK-152c`
- Objective:
  Update the docs surfaces so they explicitly say live cooling overrides are intentionally unsupported on `Layer::PathOptimization`.
- Precondition:
  Tool-ordering implementation is green.
- Postcondition:
  Both `docs/05_module_sdk.md` and `docs/07_implementation_status.md` contain the literal phrase `"intentionally unsupported on the live Layer::PathOptimization surface"` in a context tied to fan-speed and cooling overrides, with a `TASK-152c` reference. No new live-path cooling override surface is introduced.
- Files expected to change:
  - `docs/05_module_sdk.md`
  - `docs/07_implementation_status.md`
- Authoritative docs:
  - `docs/05_module_sdk.md`
  - `docs/07_implementation_status.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/CoolingBuffer.hpp`
- Verification:
  - `rg -n "intentionally unsupported on the live Layer::PathOptimization surface|TASK-152c" docs/05_module_sdk.md docs/07_implementation_status.md`
- Exit condition:
  The exact rejection text appears in both docs surfaces.

### Step 4: Run the packet's full acceptance ceremony

- Task IDs:
  - `TASK-152`
  - `TASK-152b`
  - `TASK-152c`
- Objective:
  Re-run every acceptance command from `packet.spec.md` and prove the workspace is clean.
- Precondition:
  Steps 1–3 complete.
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
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `./modules/core-modules/build-core-modules.sh` succeeds.
- `! grep -RIn "tool.*group\\|group.*by_tool\\|order_entities_by_tool" crates/slicer-host/src/layer_executor.rs` returns zero matches.
- `docs/07_implementation_status.md` updated for TASK-152 / TASK-152b / TASK-152c.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm tool ordering lives inside `path-optimization-default` and no host-side helper was reintroduced.
- Confirm packet-`33` regression tests (`path_ordering_tdd`) still pass.
- Confirm no new cooling/fan live-path API was added in this packet.
- Record any remaining packet-local risk before status changes.
