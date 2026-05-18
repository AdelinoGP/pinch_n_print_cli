# Implementation Plan: external-surface-classification-at-slice

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-164.
- TDD first (Step 2), then schema (Step 1) and implementation (Steps 3–4), then mechanical fix-ups (Step 5), then narrow validation (Step 6).
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are the budget contract.

## Steps

### Step 0: Confirm `bridge_regions` is populated by `PrePass::MeshAnalysis`

- Task IDs:
  - `TASK-164`
- Objective: read-only discovery — verify `crates/slicer-host/src/mesh_analysis.rs` populates `SurfaceClassificationIR.per_object[*].bridge_regions[*].facet_indices` for at least one realistic object. If it does not, expand the Step-3 scope to include a minimal classifier-side bridge population fix (else defer all bridge work to packet 36 and ship this packet with `is_bridge` always `false`).
- Precondition: Step 0 not yet run.
- Postcondition: a single FACT recorded — "bridge_regions populated" or "bridge_regions empty for typical objects".
- Files allowed to read:
  - `crates/slicer-host/src/mesh_analysis.rs` — lines `113-330`
  - `crates/slicer-ir/src/slice_ir.rs` — lines `336-380` (BridgeRegion + ObjectSurfaceData)
- Files allowed to edit (≤ 3): none (read-only step).
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/`, `target/`, `wit/`, `crates/slicer-host/src/dispatch.rs`.
- Expected sub-agent dispatches:
  - "Does `mesh_analysis.rs::execute_mesh_analysis_with` ever push entries into `bridge_regions`? Return FACT yes/no with the file:line of the push site."
- Context cost: `S`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `BridgeRegion` definition (read directly; one section).
- OrcaSlicer refs:
  - none for this step.
- Verification:
  - the dispatched FACT.
- Exit condition: a recorded FACT determining whether bridge classification is in scope for this packet. If "no", update Step 3 to skip bridge logic and document in the deviation log; if "yes", proceed with full bridge logic.

### Step 1: Schema additive-minor bump on `SlicedRegion` and `SliceIR`

- Task IDs:
  - `TASK-164`
- Objective: add `is_top_surface: bool, is_bottom_surface: bool, is_bridge: bool` to `SlicedRegion`; bump `SliceIR.schema_version` from `1.0.0` to `1.1.0`; update the workspace's literal `SlicedRegion {}` constructors with the three new fields defaulted to `false`.
- Precondition: Step 0 complete.
- Postcondition: workspace compiles; existing tests still green; new fields default-false; one new schema-version test passes.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — lines `1000-1040`
  - `docs/02_ir_schemas.md` — `SliceIR` section only
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-ir/tests/ir_tests.rs` (one literal `SlicedRegion` site at line `439` + new schema-version test)
  - one mechanical-fix-up file at a time (re-enter Step 1 per file): `crates/slicer-host/tests/dispatch_tdd.rs`, `crates/slicer-host/tests/live_layer_support_tdd.rs`, `crates/slicer-host/tests/macro_all_worlds_roundtrip_tdd.rs`, `crates/slicer-host/tests/slice_postprocess_paint_annotation_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs`, `wit/`, `OrcaSlicerDocumented/`.
- Expected sub-agent dispatches:
  - "Find every literal `SlicedRegion {` constructor in the workspace; return LOCATIONS." (run once; reuse list across the mechanical edits)
  - "Run `cargo build --workspace`; return FACT pass / SNIPPETS on fail."
  - "Run `cargo test -p slicer-ir slice_ir_schema_version_is_one_one_zero -- --exact`; return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — additive-minor rule (delegate one-sentence FACT confirming the rule).
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo build --workspace`
  - `cargo test -p slicer-ir slice_ir_schema_version_is_one_one_zero -- --exact`
- Exit condition: workspace builds; schema-version test passes; literal `SlicedRegion {}` constructors all carry the three new fields defaulted to `false`.

### Step 2: Author the failing TDD file

- Task IDs:
  - `TASK-164`
- Objective: create `crates/slicer-host/tests/external_surface_classification_tdd.rs` with the exact test names listed in `packet.spec.md` (Acceptance and Negative Test Cases). Tests fail until Step 3 / Step 4 land. No production code changes in this step.
- Precondition: Step 1 complete (schema fields present).
- Postcondition: 7+ failing tests for cases enumerated in `packet.spec.md`. Each test asserts on `(is_top_surface, is_bottom_surface, is_bridge)` exactly.
- Files allowed to read:
  - `crates/slicer-host/src/layer_slice.rs` — full file (≤ 100 lines)
  - `crates/slicer-host/src/mesh_analysis.rs` — lines `113-330` only
  - `crates/slicer-ir/src/slice_ir.rs` — lines `230-380, 1000-1040`
  - `packet.spec.md` from this packet
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/external_surface_classification_tdd.rs` (new)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/wit_host.rs`.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test external_surface_classification_tdd`; return FACT pass/fail with the per-test list of failures."
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `SurfaceClassificationIR` section.
  - `docs/08_coordinate_system.md` — read directly.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` — delegate FACT confirming `stTop`/`stBottom`/`stBottomBridge` are the canonical role names this packet's tests mirror.
- Verification:
  - `cargo test -p slicer-host --test external_surface_classification_tdd` — must FAIL at this stage; pass count must be 0.
- Exit condition: tests compile and run; every test in the file is in the FAIL state for the right reason (helper / extended `execute_layer_slice` not yet implemented).

### Step 3: Implement `classify_region_surfaces` private helper

- Task IDs:
  - `TASK-164`
- Objective: add the private helper inside `crates/slicer-host/src/layer_slice.rs` matching the signature in `design.md`. World-Z facet computation reuses `apply_transform` from `mesh_analysis.rs`; XY conversion uses `Point2::from_mm` / `mm_to_units` per `docs/08`. Bridge classification uses `bridge_regions[*].facet_indices` per Step 0's FACT.
- Precondition: Step 2 complete with failing tests pinned to the planned helper signature.
- Postcondition: the unit-level helper tests in Step 2 (top, bottom, bridge, mixed, out-of-window, centroid-outside-polygon) pass.
- Files allowed to read:
  - `crates/slicer-host/src/mesh_analysis.rs` — lines `113-330` (re-read `apply_transform` + `triangle_normal_area`).
  - `crates/slicer-helpers` — public API only via symbol search; do not load files in full.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_slice.rs` (helper added inside this file).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/wit_host.rs`.
- Expected sub-agent dispatches:
  - "Find the public point-in-polygon helper in `crates/slicer-helpers/`; return FACT with the function path and signature."
  - "Run `cargo test -p slicer-host --test external_surface_classification_tdd top_surface_facet_within_window_flags_top bottom_surface_facet_within_window_flags_bottom bridge_facet_in_z_span_flags_bridge top_facet_outside_polygon_does_not_flag_top top_facet_outside_z_window_does_not_flag_top -- --exact`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs:
  - `docs/08_coordinate_system.md` — read directly.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/LayerRegion.cpp` — delegate FACT: confirm `process_external_surfaces` does not perform a per-vertex containment test (we deliberately diverge); record divergence in `docs/DEVIATION_LOG.md`.
- Verification:
  - `cargo test -p slicer-host --test external_surface_classification_tdd <unit-test-set> -- --exact`
- Exit condition: all unit-level helper tests PASS; integration tests (Step 4 territory) still FAIL.

### Step 4: Extend `execute_layer_slice` and wire the production caller

- Task IDs:
  - `TASK-164`
- Objective: extend `execute_layer_slice` signature with `surface_class: Option<&SurfaceClassificationIR>, next_layer_z: Option<f32>, prev_layer_z: Option<f32>`; populate the three flags per region using the Step-3 helper. Update `crates/slicer-host/src/layer_executor.rs:295-310` to pass `blackboard.surface_classification()` and adjacent-layer Z values from `blackboard.layer_plan().global_layers`. Replace `wit_host.rs:2545-2547` hardcoded `false`s with `region.is_top_surface` / `is_bottom_surface` / `is_bridge`.
- Precondition: Step 3 complete; helper tests green.
- Postcondition: `execute_layer_slice` test cases in `external_surface_classification_tdd.rs` PASS; `live_top_bottom_fill_tdd.rs` remains green; `cargo build --workspace` succeeds.
- Files allowed to read:
  - `crates/slicer-host/src/blackboard.rs` — lines `192-220` (accessor surface).
  - `crates/slicer-host/src/wit_host.rs` — lines `2517-2580` only.
  - `crates/slicer-host/src/layer_executor.rs` — lines `280-360`.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_slice.rs`
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/wit_host.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/prepass.rs`, `wit/`, `crates/slicer-sdk/`.
- Expected sub-agent dispatches:
  - "Find every caller of `execute_layer_slice` in `crates/`; return LOCATIONS."
  - "Run `cargo test -p slicer-host --test external_surface_classification_tdd execute_layer_slice_writes_top_flag_on_sliced_region execute_layer_slice_without_classification_keeps_flags_false -- --exact`; return FACT pass/fail."
  - "Run `cargo test -p slicer-host --test live_top_bottom_fill_tdd`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs:
  - `docs/04_host_scheduler.md` — § Per-Layer Execution + Blackboard Structure (delegate SUMMARY ≤ 200 words).
- OrcaSlicer refs:
  - none for this step.
- Verification:
  - `cargo test -p slicer-host --test external_surface_classification_tdd -- --nocapture`
  - `cargo test -p slicer-host --test live_top_bottom_fill_tdd -- --nocapture`
- Exit condition: every test in `external_surface_classification_tdd.rs` PASS; `live_top_bottom_fill_tdd.rs` PASS; build green.

### Step 5: Update remaining `execute_layer_slice` test callers

- Task IDs:
  - `TASK-164`
- Objective: pass `None, None, None` to every existing `execute_layer_slice` test caller in `crates/slicer-host/tests/layer_slice_tdd.rs` (lines `145, 159, 256, 257, 319, 369, 370, 444`). These tests do not exercise classification.
- Precondition: Step 4 complete.
- Postcondition: `cargo test --workspace` succeeds.
- Files allowed to read:
  - `crates/slicer-host/tests/layer_slice_tdd.rs` — only the listed line ranges (range-read 8 windows of ±10 lines around each call site).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/layer_slice_tdd.rs`
- Files explicitly out-of-bounds for this step:
  - all production code (this is a mechanical test-file edit).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test layer_slice_tdd`; return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs:
  - none.
- OrcaSlicer refs:
  - none.
- Verification:
  - `cargo test -p slicer-host --test layer_slice_tdd -- --nocapture`
- Exit condition: file compiles; every test in `layer_slice_tdd.rs` PASS.

### Step 6: Acceptance — Benchy E2E + workspace gates

- Task IDs:
  - `TASK-164`
- Objective: confirm the two user-facing failing tests now PASS, and no other test in the workspace regressed; clippy stays clean.
- Precondition: Step 5 complete.
- Postcondition: every `packet.spec.md` AC verification command returns PASS; `docs/02_ir_schemas.md` and `docs/DEVIATION_LOG.md` carry the schema bump and deviation entries; `docs/07_implementation_status.md` carries the new TASK-164 line.
- Files allowed to read:
  - `docs/02_ir_schemas.md` — relevant sections only.
  - `docs/DEVIATION_LOG.md` — full file (small).
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md` (delegate the row insertion via worker; do NOT load the full backlog).
- Files explicitly out-of-bounds for this step:
  - `OrcaSlicerDocumented/`, `target/`, `crates/slicer-host/src/dispatch.rs`.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence benchy_feature_evidence_failures_name_the_missing_family -- --nocapture`; return FACT pass/fail."
  - "Run `cargo test --workspace`; return FACT pass/fail with failing test list (max 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail."
  - "Insert TASK-164 row into `docs/07_implementation_status.md` for the live-top-bottom surface-classification wiring; return FACT confirming the new line:line."
- Context cost: `S`.
- Authoritative docs:
  - `docs/02_ir_schemas.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
- OrcaSlicer refs:
  - none for this step.
- Verification:
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_top_and_bottom_surface_evidence benchy_feature_evidence_failures_name_the_missing_family -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
- Exit condition: every AC command PASS; deviation log carries one entry for the any-vertex-in-polygon approximation; `docs/07` carries TASK-164 line.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Read-only discovery; one delegated FACT. |
| Step 1 | S | Schema bump + mechanical fix-ups (small per-file). |
| Step 2 | M | New TDD file; writes 7+ tests. |
| Step 3 | M | Helper implementation. |
| Step 4 | M | Three coupled file edits with one signature change. |
| Step 5 | S | Mechanical test-file edits. |
| Step 6 | S | Verification dispatches + 3 doc edits. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete.
- Every step exit condition is met.
- Every pipe-suffixed AC verification command in `packet.spec.md` returns PASS.
- `docs/07_implementation_status.md` carries TASK-164 (delegated edit).
- `docs/02_ir_schemas.md` documents `SliceIR.schema_version = 1.1.0` and the three new `SlicedRegion` fields.
- `docs/DEVIATION_LOG.md` carries the any-vertex-in-polygon approximation entry.
- `.ralph/specs/12_live-top-bottom-surface-fill/packet.spec.md` carries a header pointer to this packet (delegated edit; do not flip its status).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` are PASS.
- Record any remaining packet-local risk (especially: Benchy bridge evidence — not asserted in this packet; tracked in packet 36).
- Confirm implementer's peak context usage stayed under 70%.
