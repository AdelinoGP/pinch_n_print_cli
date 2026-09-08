# Implementation Plan: 243-object-scoped-overhang-annotation

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Field-type replacement + major schema bump

- Task IDs: `TASK-353`
- Objective: Change the two `SurfaceClassificationIR` map field types to object-scoped nested maps
  and bump `CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION` from `1.3.0` to `2.0.0`, updating the
  hard-asserting test in the same edit.
- Precondition: tree compiles at `1.3.0`; `cargo check -p slicer-ir` is green.
- Postcondition: `SurfaceClassificationIR.overhang_quartile_polygons` is
  `HashMap<ObjectId, HashMap<u32, Vec<QuartileBand>>>` and `prev_layer_boundaries` is
  `HashMap<ObjectId, HashMap<u32, Vec<ExPolygon>>>`; the constant is `SemVer { major: 2, minor: 0,
  patch: 0 }`; `ir_tests.rs` asserts `{ 2, 0, 0 }`. The tree is non-compiling until Step 2 lands
  (expected — the field-type change breaks the producer, marshal, and visual-debug consumers).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-ir/src/slice_ir.rs` - lines 180-200 and 685-715
  - `crates/slicer-ir/tests/ir_tests.rs` - lines 680-700
- Files allowed to edit (at most 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/tests/ir_tests.rs`
- Files explicitly out of bounds:
  - `crates/slicer-runtime/**`, `crates/slicer-wasm-host/**`, `crates/slicer-core/**` (consumers —
    Step 2/3)
  - `docs/02_ir_schemas.md` (Step 4)
- Blast-radius discipline (mandatory when adding a new struct field or schema constant):
  - This step changes two field types and bumps a public version constant. The **struct-literal /
    field-access blast radius** (every site that compiles against the two fields) is pre-baked and
    split across Steps 2-3: production consumers `overhang_annotation_producer.rs`, `in_.rs`,
    `visual_debug_render.rs` (Step 2); test consumers `visual_debug_blackboard_tap_tdd.rs`,
    `algo_prepass_slice_tdd.rs`, `overhang_pipeline_e2e_tdd.rs`,
    `prepass_overhang_annotation_stage_order_tdd.rs`,
    `slice_region_view_overhang_areas_non_empty_tdd.rs` (Step 3). The **test-assertion fallout** for
    the constant is exactly `ir_tests.rs::bridge_detector_schema_versions_are_constant_sourced`
    (pins `SemVer { major: 1, minor: 3, patch: 0 }`), edited here. Struct literals that use
    type-inferred `HashMap::new()` (e.g. `mesh_analysis.rs`, `bridge_detector_tdd.rs`,
    `surface_group_resolution_tdd.rs`, `translated_object_z_floor_tdd.rs`,
    `bridge_false_site_gating_tdd.rs`, `rotated_object_world_extent_tdd.rs`,
    `blackboard_layer_arena_tdd.rs`, `prepass_executor_tdd.rs`) do NOT break and are out of scope.
  - Dispatch a `LOCATIONS` worker for the struct-literal sites before authoring this step; cite the
    result inline below.
- Expected sub-agent dispatches:
  - Question: list every `.overhang_quartile_polygons` / `.prev_layer_boundaries` field read/insert
    and every `SurfaceClassificationIR {` literal across `crates/**/*.rs` not already in this plan's
    §Files in Scope; scope: `crates/**/*.rs`; return: `LOCATIONS` (≤ 20 entries)
  - Question: does any test other than `ir_tests.rs::bridge_detector_schema_versions_are_constant_sourced`
    hard-assert the `1.3.0` value; scope: `crates/**/tests/**/*.rs`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - §"IR Versioning Contract" (lines ~1633-1641) — field-type change is
    major
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-ir --test ir_tests -- bridge_detector_schema_versions_are_constant_sourced --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo check -p slicer-ir --all-targets` - FACT pass/fail (slicer-ir itself must compile; the
    downstream crates are expected to fail until Step 2)
- Exit condition: the schema test passes with `{ 2, 0, 0 }` and `cargo check -p slicer-ir --all-targets`
  is green.

### Step 2: Producer + marshal + visual-debug re-keying

- Task IDs: `TASK-353`
- Objective: Re-key the three production consumers of the two maps to the object-scoped shape.
- Precondition: Step 1 landed (field types + constant changed).
- Postcondition: `commit_overhang_annotation_builtin` inserts per-object maps keyed by `object.id`;
  `sliced_region_to_data` reads `(region.object_id, global_layer_index)`; `visual_debug_render.rs`
  iterates the nested map. `cargo check -p slicer-runtime -p slicer-wasm-host --all-targets` is green
  (test targets may still fail until Step 3).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` - lines 130-195
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - lines 440-500
  - `crates/slicer-runtime/src/visual_debug_render.rs` - lines 505-525 and 1065-1090
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
  - `crates/slicer-runtime/src/visual_debug_render.rs`
- Files explicitly out of bounds:
  - `crates/slicer-core/**`, `crates/slicer-sdk/**`, `crates/slicer-macros/**`, `modules/**`
  - any test file (Step 3)
- Blast-radius discipline: not applicable (no new field/constant here; this step consumes Step 1's
  change).
- Expected sub-agent dispatches:
  - none (all three edits are localized and already located)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` - §"Packet 1" change items 1-2 (nested maps,
    marshal reads `(view.object_id(), global_layer_index)`)
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo check -p slicer-runtime -p slicer-wasm-host --all-targets` - FACT pass/fail
- Exit condition: both crates compile with `--all-targets` (test targets may still fail; Step 3 owns
  them).

### Step 3: Test fallout + object-scoping tests

- Task IDs: `TASK-353`
- Objective: Re-key the five test files that read/insert the two maps, rename + rewrite the
  multi-object producer test (`overhang_annotation_merges_multi_object_bands_by_quartile` →
  `overhang_annotation_scopes_bands_by_object`) to assert object isolation instead of quartile
  merge, and add the marshal-side no-cross-object-leak contract test.
- Precondition: Steps 1-2 landed; production crates compile.
- Postcondition: `overhang_annotation_scopes_bands_by_object` (executor) and
  `overhang_areas_object_scoped_no_cross_object_leak` (contract) pass; the rewritten multi-object
  test asserts object isolation; all five fallout files compile.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs` - lines
    280-385 (existing multi-object test + `two_object_overhang_mesh()`)
  - `crates/slicer-runtime/tests/contract/slice_region_view_overhang_areas_non_empty_tdd.rs` - lines
    250-320 (marshal test + `sliced_region_to_data` call)
  - `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs` - lines 110-130 and 385-395
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs` - lines 210-245
  - `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` - lines 430-500
- Files allowed to edit (at most 3 primary; the five fallout files are mechanical re-keys):
  - `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs`
  - `crates/slicer-runtime/tests/contract/slice_region_view_overhang_areas_non_empty_tdd.rs`
  - `crates/slicer-runtime/tests/visual_debug_blackboard_tap_tdd.rs`
  - `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`
  - `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, `crates/slicer-runtime/src/**`, `crates/slicer-wasm-host/src/**`
  - `docs/02_ir_schemas.md` (Step 4)
- Blast-radius discipline: not applicable (no new field/constant; this step absorbs Step 1's test
  fallout, pre-baked in Step 1).
- Expected sub-agent dispatches:
  - none (all five files already located; edits are mechanical re-keys plus two new test functions)
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` - §"Tests" ("Multi-object overlapping-footprint
    fixture proves object-scoped anchors (Packet 1)")
- OrcaSlicer refs:
  - none
- Verification:
  - `cargo test -p slicer-runtime --test executor -- overhang_annotation_scopes_bands_by_object --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo test -p slicer-runtime --test contract -- overhang_areas_object_scoped_no_cross_object_leak --exact 2>&1 | tee target/test-output.log | grep -qE "^test result: ok\. 1 passed"` - FACT pass/fail
  - `cargo check --workspace --all-targets` - FACT pass/fail
- Exit condition: the two named tests pass and `cargo check --workspace --all-targets` is green.

### Step 4: Docs update

- Task IDs: `TASK-353`
- Objective: Update `docs/02_ir_schemas.md` §"IR 2 — SurfaceClassificationIR" to reflect the 2.0.0
  major bump and object-scoped keying.
- Precondition: Steps 1-3 landed; all tests green.
- Postcondition: the IR 2 section reads `Current schema_version: 2.0.0` and describes the maps as
  keyed by object id first, then global layer index; the packet-193 provenance note is amended.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/02_ir_schemas.md` - lines 385-410 only
- Files allowed to edit (at most 3):
  - `docs/02_ir_schemas.md`
- Files explicitly out of bounds:
  - `docs/07_implementation_status.md` (orchestrator-owned)
  - `docs/specs/wave-overhangs-bridge-fill-plan.md` (orchestrator-owned)
- Blast-radius discipline: not applicable.
- Expected sub-agent dispatches:
  - none
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - §"IR 2" and §"IR Versioning Contract" (already read in Step 1)
- OrcaSlicer refs:
  - none
- Verification:
  - `rg -q 'Current schema_version: 2\.0\.0' docs/02_ir_schemas.md && rg -q 'keyed by object id first' docs/02_ir_schemas.md && echo P243_DOCS_UPDATED` - FACT pass/fail
- Exit condition: both greps match.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | field-type replacement + major bump; owns the full blast radius |
| Step 2 | M | three production consumers re-keyed |
| Step 3 | M | five test files + two new tests |
| Step 4 | S | one doc section |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none for this packet).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
