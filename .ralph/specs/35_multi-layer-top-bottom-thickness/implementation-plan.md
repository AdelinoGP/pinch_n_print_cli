# Implementation Plan: multi-layer-top-bottom-thickness

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-165.
- TDD first (Step 2), then implementation (Steps 3–4), then mechanical fix-ups + acceptance.
- Each step honors the context-discipline preamble.

## Steps

### Step 0: FACT-confirm config schema and Orca defaults

- Task IDs:
  - `TASK-165`
- Objective: read-only discovery — confirm whether `top_solid_layers` and `bottom_solid_layers` config keys exist in the central config schema, and confirm Orca's numeric defaults.
- Precondition: Step 0 not yet run.
- Postcondition: two recorded FACTs — (a) "keys present in `<file>:<line>`" or "keys missing"; (b) "Orca default = 3 for both".
- Files allowed to read: none directly (delegate dispatches only).
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds for this step: all production code.
- Expected sub-agent dispatches:
  - "Are `top_solid_layers` and `bottom_solid_layers` declared in `crates/slicer-ir/src/` or `crates/slicer-host/src/` config schemas? Return FACT yes/no with file:line each."
  - "Confirm Orca defaults for `top_solid_layers` and `bottom_solid_layers` from `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` and config-option defaults. Return FACT numeric values only."
- Context cost: `S`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — config-key declaration rules (delegate FACT).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `discover_horizontal_shells()`; defaults section.
- Verification: the two FACTs.
- Exit condition: both FACTs recorded; Step 1 plan firmed up based on outcomes.

### Step 1: Add config keys (if absent) and update doc

- Task IDs:
  - `TASK-165`
- Objective: if Step 0 FACT (a) reported the keys absent, add `top_solid_layers: u32` and `bottom_solid_layers: u32` to the central config schema. Document defaults in `docs/02_ir_schemas.md` or `docs/03_wit_and_manifest.md` per repository convention. If Step 0 FACT (a) reported keys present, this step is a no-op; record that and proceed.
- Precondition: Step 0 complete.
- Postcondition: keys present in the schema with Orca defaults; workspace builds.
- Files allowed to read:
  - the schema file identified in Step 0 (range-read; ≤ 60 lines).
- Files allowed to edit (≤ 3):
  - the schema file.
  - `docs/02_ir_schemas.md` or `docs/03_wit_and_manifest.md` (whichever owns config schema).
- Files explicitly out-of-bounds for this step: all unrelated crates.
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md`.
- OrcaSlicer refs: none.
- Verification: `cargo build --workspace`.
- Exit condition: workspace builds; keys exist; defaults documented.

### Step 2: Author the failing TDD file

- Task IDs:
  - `TASK-165`
- Objective: create `crates/slicer-host/tests/multi_layer_thickness_tdd.rs` with the exact test names from `packet.spec.md` (Acceptance and Negative cases). Tests fail until Steps 3–4 land. Add `benchy_multi_layer_top_bottom_evidence` test to `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`.
- Precondition: Step 1 complete.
- Postcondition: every new test compiles and FAILS.
- Files allowed to read:
  - `crates/slicer-host/src/layer_slice.rs` — full file (small post-12-rev1).
  - `crates/slicer-host/tests/external_surface_classification_tdd.rs` — full file (test patterns to mirror).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — read only the existing top/bottom-surface-evidence test (lines `1160-1200`).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/multi_layer_thickness_tdd.rs` (new).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (append one new test).
- Files explicitly out-of-bounds for this step: all production code.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test multi_layer_thickness_tdd`; return FACT (every test FAIL)."
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md` (RegionMapIR section).
- OrcaSlicer refs: none.
- Verification: tests compile + every new test in FAIL state.
- Exit condition: TDD file present; tests fail for the right reason (window logic not yet wide).

### Step 3: Widen the Z window in `classify_region_surfaces`

- Task IDs:
  - `TASK-165`
- Objective: in `crates/slicer-host/src/layer_slice.rs`, extend `classify_region_surfaces` to take `top_solid_layers: u32, bottom_solid_layers: u32` and walk `LayerPlanIR.global_layers` to compute the multi-layer Z window. Pass `LayerPlanIR` borrow into the helper or thread it via a helper struct.
- Precondition: Step 2 complete.
- Postcondition: helper-level multi-layer tests in `multi_layer_thickness_tdd.rs` PASS; `external_surface_classification_tdd.rs` (12-rev1) remains green.
- Files allowed to read:
  - `crates/slicer-host/src/layer_slice.rs` — full file.
  - `crates/slicer-ir/src/slice_ir.rs` — lines `680-740` (LayerPlanIR / GlobalLayer).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_slice.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs`, `wit/`, `crates/slicer-sdk/`.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test multi_layer_thickness_tdd top_solid_layers_three_flags_three_layers bottom_solid_layers_three_flags_three_layers window_truncates_at_object_extent missing_config_uses_default_three zero_top_solid_layers_disables_flag none_region_map_uses_orca_defaults -- --exact`; return FACT pass/fail per test."
  - "Run `cargo test -p slicer-host --test external_surface_classification_tdd`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs:
  - `docs/04_host_scheduler.md` — § RegionMapIR (delegate SUMMARY).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — already FACTed in Step 0; no re-read.
- Verification:
  - `cargo test -p slicer-host --test multi_layer_thickness_tdd <unit-tests> -- --exact`
  - `cargo test -p slicer-host --test external_surface_classification_tdd`
- Exit condition: helper tests PASS; 12-rev1 tests still PASS.

### Step 4: Thread `RegionMapIR` through `execute_layer_slice` and the production caller

- Task IDs:
  - `TASK-165`
- Objective: extend `execute_layer_slice` signature with `region_map: Option<&RegionMapIR>` (and `layer_plan: Option<&LayerPlanIR>` if not already present from Step 3). Inside the region loop, look up `(top_solid_layers, bottom_solid_layers)` per `(layer_idx, object_id, region_id)` from `region_map.entries[*].config` with Orca defaults when absent. Update the production caller `crates/slicer-host/src/layer_executor.rs:295-310` to forward `blackboard.region_map()` and `blackboard.layer_plan()`.
- Precondition: Step 3 complete (helper supports the wide window).
- Postcondition: `execute_layer_slice_honors_region_map_top_solid_layers` PASSES; `none_region_map_uses_orca_defaults` PASSES; `cargo build --workspace` succeeds.
- Files allowed to read:
  - `crates/slicer-host/src/blackboard.rs` — lines `192-220` only.
  - `crates/slicer-host/src/region_mapping.rs` — public API only (range-read).
  - `crates/slicer-host/src/layer_executor.rs` — lines `280-360`.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_slice.rs`
  - `crates/slicer-host/src/layer_executor.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/prepass.rs`, `wit/`.
- Expected sub-agent dispatches:
  - "Find every caller of `execute_layer_slice` in `crates/`; return LOCATIONS."
  - "Run `cargo test -p slicer-host --test multi_layer_thickness_tdd execute_layer_slice_honors_region_map_top_solid_layers none_region_map_uses_orca_defaults -- --exact`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs:
  - `docs/04_host_scheduler.md` — § Per-Layer Execution + § Blackboard Structure.
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo test -p slicer-host --test multi_layer_thickness_tdd -- --nocapture`
  - `cargo build --workspace`
- Exit condition: every test in `multi_layer_thickness_tdd.rs` PASSES; build green.

### Step 5: Update remaining `execute_layer_slice` test callers

- Task IDs:
  - `TASK-165`
- Objective: pass `None` (and `None` for `layer_plan` if added in Step 3) to existing `execute_layer_slice` callers in `crates/slicer-host/tests/layer_slice_tdd.rs` (8 sites enumerated by 12-rev1 plan).
- Precondition: Step 4 complete.
- Postcondition: workspace tests PASS.
- Files allowed to read:
  - `crates/slicer-host/tests/layer_slice_tdd.rs` — only the 8 line ranges enumerated.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/layer_slice_tdd.rs`
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test layer_slice_tdd`; return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-host --test layer_slice_tdd`.
- Exit condition: every test PASSES.

### Step 6: Acceptance — Benchy E2E + workspace gates

- Task IDs:
  - `TASK-165`
- Objective: confirm `benchy_multi_layer_top_bottom_evidence` PASSES at `top_solid_layers = 4`, `bottom_solid_layers = 4`; confirm `cargo test --workspace` PASSES; confirm `cargo clippy --workspace -- -D warnings` PASSES; update `docs/07_implementation_status.md`.
- Precondition: Step 5 complete.
- Postcondition: every AC verification command in `packet.spec.md` PASSES; backlog updated.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` (delegate the row insertion; do NOT load the full backlog).
  - `docs/02_ir_schemas.md` if Step 1 added config keys.
- Files explicitly out-of-bounds for this step: production code (no further changes).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_multi_layer_top_bottom_evidence -- --nocapture`; return FACT pass/fail."
  - "Run `cargo test --workspace`; return FACT pass/fail with failing test list (max 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail."
  - "Insert TASK-165 row into `docs/07_implementation_status.md`; return FACT confirming the new line:line."
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_multi_layer_top_bottom_evidence -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
- Exit condition: every AC PASSES; `docs/07` carries TASK-165.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Two FACT dispatches. |
| Step 1 | S | Conditional schema-key add + small doc edit. |
| Step 2 | M | New TDD file + 1 Benchy E2E test. |
| Step 3 | M | Window-widening logic. |
| Step 4 | M | Signature extension + production caller. |
| Step 5 | S | Mechanical test fix-ups. |
| Step 6 | S | Acceptance dispatches + doc edits. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every pipe-suffixed AC verification command in `packet.spec.md` PASSES.
- `docs/07_implementation_status.md` carries TASK-165 (delegated edit).
- `docs/02_ir_schemas.md` documents the new config keys (if Step 1 added them).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command.
- Confirm `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` PASS.
- Record any remaining packet-local risk.
- Confirm implementer's peak context usage stayed under 70%.
