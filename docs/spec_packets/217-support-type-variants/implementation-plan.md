# Implementation Plan: support-type-variants

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-326`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently.

## Steps

### Step 1: Lock the mode input and regression fixtures

- Task IDs: `TASK-326`
- Objective: inventory config forwarding and add red tests for auto versus manual contact sources without changing claim selection.
- Precondition: scheduler resolver and planner contact helpers exist at the cited locations.
- Postcondition: tests fail specifically when manual mode still collects overhang facets or when `support_type` is not scoped to the planner.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - lines `133-220,389-470,1009-1078`
  - `modules/core-modules/support-planner/support-planner.toml` - lines `27-120`
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` - targeted test/module ranges only
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs`
  - `modules/core-modules/support-planner/support-planner.toml`
- Files explicitly out of bounds:
  - `crates/slicer-scheduler/src/execution_plan.rs`
  - `crates/slicer-wasm-host/src/execution_plan_live.rs`
  - generated WASM and `target/`
- Expected sub-agent dispatches:
  - Question: enumerate config schema and test construction sites; scope: `modules/core-modules/support-planner/**`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-generation-remediation-plan.md` - direct queue row 5
  - `docs/specs/support-generation-defect-verified-findings.md` - direct ranges `28-55,157-176`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` - delegate; auto/manual policy
- Verification:
  - `cargo test -p support-planner --all-targets --test to_buildplate_tdd -- --list` - FACT
- Exit condition: the red tests identify the exact missing manual-mode behavior and the config schema lists the exact `support_type` field shape.

### Step 2: Implement planner mode and preserve the scheduler split

- Task IDs: `TASK-326`
- Objective: parse `support_type` into `SupportGenerationMode` and make manual mode enforcers-only while retaining the unchanged claim resolver.
- Precondition: Step 1 identifies the config forwarding shape and focused test locations.
- Postcondition: auto collects detected overhangs plus enforcers; manual collects only enforcers; `tree/classic` remains a scheduler-only module choice.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - lines `60-220,389-470,1009-1078`
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` - focused fixtures/tests
  - `crates/slicer-scheduler/src/execution_plan.rs` - lines `219-252,1215-1386`
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs`
  - `modules/core-modules/support-planner/support-planner.toml` only if Step 1 did not edit it
- Files explicitly out of bounds:
  - scheduler resolver and live execution-plan code
  - fallback, IR, WIT, raft/interface, and G-code code
  - generated `support-planner.wasm`
- Expected sub-agent dispatches:
  - Question: verify every `SupportPlanner` constructor/fixture affected by the new mode field; scope: `modules/core-modules/support-planner/**`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - delegated SUMMARY for geometry boundary
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegate; mode comparison
- Verification:
  - `cargo xtask build-guests --check` - FACT; rebuild if stale
  - `cargo test -p support-planner --all-targets --test to_buildplate_tdd` - FACT
  - `cargo test -p slicer-scheduler --all-targets support_type_tree_manual_selects_tree_support_holder support_type_normal_falls_back_to_traditional_support_holder` - FACT
- Exit condition: focused tests prove both mode contact policies and scheduler tests prove no claim-resolution regression.

### Step 3: Run the visual-debug behavior gate

- Task IDs: `TASK-326`
- Objective: run auto and manual model-mode requests and assert manifest evidence for overhang/enforcer behavior.
- Precondition: guest artifacts are fresh and Step 2 targeted tests pass.
- Postcondition: auto has detected-overhang support, manual has enforcer-only support, and the output bundles are deterministic and inspectable via `manifest.json`.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` - lines `125-197,686-774`
  - `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs` - lines `70-245`
  - `docs/19_visual_debug.md` - lines `17-46,158-180`
 - Files allowed to edit (at most 3):
   - `tmp/support-config-manual.json`
   - `tmp/visual-debug-support-manual.json`
   - `tmp/visual-debug-tree.json` only to add the `PrePass::SupportGeometry` tap required by AC-1
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug.rs`
  - G-code emit/serialize code and all geometry-fix packet surfaces
- Expected sub-agent dispatches:
  - Question: confirm existing model visual-debug test can drive real module dispatch; scope: `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/19_visual_debug.md` - direct ranges `17-46,158-180`
- OrcaSlicer refs: none beyond packet-level obligations.
 - Verification:
   - `cargo test -p pnp-cli --all-targets --test visual_debug_typed_tap_capture_tdd visual_debug_forwards_support_tool_selection` - FACT
    - `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-tree.json --output target/vd-support-type-auto --overwrite >/dev/null && jq -e '. as $m | ([10,125,130] | all(.[]; . as $layer | any($m.images[]; .tap == "PrePass::SupportGeometry" and .layer_index == $layer and ((.png_path // "") | length > 0)))) and ([10,125,130] | all(.[]; . as $layer | any($m.images[]; .tap == "Layer::Support" and .layer_index == $layer and ((.typed_capture.value.support_paths // []) | length > 0))))' target/vd-support-type-auto/manifest.json` - bounded auto manifest assertion using planner PNG entries and consumer typed paths
    - `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-manual.json --output target/vd-support-type-manual --overwrite >/dev/null && jq -e '. as $m | ([0,10,30] | all(.[]; . as $layer | any($m.images[]; .tap == "PrePass::SupportGeometry" and .layer_index == $layer and ((.png_path // "") | length > 0)))) and ([0,10,30] | all(.[]; . as $layer | any($m.images[]; .tap == "Layer::Support" and .layer_index == $layer and ((.typed_capture.value.support_paths // []) | length > 0))))' target/vd-support-type-manual/manifest.json` - bounded manual manifest assertion using planner PNG entries and consumer typed paths
- Exit condition: both mode bundles have a non-empty `PrePass::SupportGeometry` PNG entry for every requested layer and non-empty `Layer::Support` `typed_capture.value.support_paths` for every requested layer; the planner PNGs are the geometry evidence because its typed capture is null.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Red tests and schema inventory |
| Step 2 | M | Guest planner implementation and blast-radius check |
| Step 3 | M | Real pipeline visual-debug evidence |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk, especially stale guest artifacts.
- Confirm context stayed within the standard band.
