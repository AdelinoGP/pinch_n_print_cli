# Implementation Plan: top-surface-ironing

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-168.
- TDD first (Step 1 sets up failing tests), then module skeleton, then implementation, then host gcode-emit verification, then acceptance.
- Each step honors the context-discipline preamble.

## Steps

### Step 0: FACT-confirm Orca defaults, gcode-emit mapping, support-ironing template, and SDK accessors

- Task IDs:
  - `TASK-168`
- Objective: read-only discovery — confirm (a) Orca defaults for `ironing_*` keys, (b) `ExtrusionRole::Ironing` → `;TYPE:Ironing` map presence, (c) `support-surface-ironing` skeleton pattern, (d) SDK accessors for adjacent-layer top-surface awareness.
- Precondition: Step 0 not yet run.
- Postcondition: four FACTs/SUMMARY recorded.
- Files allowed to read: none directly (delegate only).
- Files allowed to edit (≤ 3): none.
- Expected sub-agent dispatches:
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp::make_ironing` algorithm and defaults for `ironing_spacing`, `ironing_flow`, `ironing_speed` in ≤ 200 words. Return SUMMARY."
  - "Does `crates/slicer-host/src/gcode_emit.rs` map `ExtrusionRole::Ironing` to `;TYPE:Ironing`? Return FACT yes/no with file:line."
  - "Summarize `modules/core-modules/support-surface-ironing/` directory + module-skeleton pattern in ≤ 200 words. Return SUMMARY."
  - "Does `SliceRegionView` (or any sibling SDK type) expose adjacent-layer surface-flag lookups, or do we need a different mechanism for 'this is the topmost top-surface layer' detection? Return FACT yes/no with file:line if yes."
- Context cost: `S`.
- Authoritative docs: `docs/05_module_sdk.md`.
- OrcaSlicer refs: `Fill/Fill.cpp::make_ironing`.
- Verification: the four returns.
- Exit condition: defaults known; gcode-emit mapping known; template known; topmost-layer detection mechanism chosen.

### Step 1: Author failing TDD file

- Task IDs:
  - `TASK-168`
- Objective: create `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` with the AC + negative cases. Append `benchy_gcode_contains_ironing_evidence` to `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`. Tests fail until later steps land (the module doesn't exist yet — these tests will fail to compile until Step 2 creates the module crate).
- Precondition: Step 0 complete.
- Postcondition: tests authored; `cargo test -p top-surface-ironing` returns "package not found" (expected — Step 2 fixes); host E2E test compiles and FAILS.
- Files allowed to read:
  - `modules/core-modules/support-surface-ironing/tests/` — SUMMARY-only via Step 0; or a single full read of one test file (small).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — only the existing top-surface-evidence test (lines `1160-1200`).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/top-surface-ironing/tests/top_surface_ironing_emission_tdd.rs` (NEW; under a directory that doesn't yet exist — create the directory).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (append).
- Files explicitly out-of-bounds for this step: production source code in any module.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence -- --nocapture`; return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md` (delegate FACT for the test-fixture pattern).
- OrcaSlicer refs: none.
- Verification:
  - host E2E test compiles + FAILS.
- Exit condition: TDD scaffolding present.

### Step 2: Create the `top-surface-ironing` module skeleton + manifest + Cargo membership

- Task IDs:
  - `TASK-168`
- Objective: create `modules/core-modules/top-surface-ironing/{Cargo.toml, manifest.toml, src/lib.rs}` mirroring `support-surface-ironing` skeleton. The `src/lib.rs` is a stub `Layer::InfillPostProcess` callback that does nothing (returns input unchanged). Add the module to the workspace members if needed. Verify `./modules/core-modules/build-core-modules.sh` discovers the new module.
- Precondition: Step 1 complete.
- Postcondition: `top-surface-ironing` package builds; rebuild script succeeds.
- Files allowed to read:
  - `modules/core-modules/support-surface-ironing/Cargo.toml`, `manifest.toml`, `src/lib.rs` — full read (each is small).
  - `Cargo.toml` (workspace root) — full read.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/top-surface-ironing/Cargo.toml` (NEW)
  - `modules/core-modules/top-surface-ironing/manifest.toml` (NEW)
  - `modules/core-modules/top-surface-ironing/src/lib.rs` (NEW; minimal stub)
- Files explicitly out-of-bounds for this step: production source of unrelated modules; `wit/`; `crates/slicer-host/src/`.
- Expected sub-agent dispatches:
  - "Run `cargo build -p top-surface-ironing`; return FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail with failing module name on fail."
- Context cost: `M`.
- Authoritative docs: `docs/03_wit_and_manifest.md`, `docs/05_module_sdk.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build -p top-surface-ironing`
  - `./modules/core-modules/build-core-modules.sh`
- Exit condition: module package builds + WASM rebuild succeeds.

### Step 3: Verify (and conditionally add) gcode-emit mapping for `ExtrusionRole::Ironing`

- Task IDs:
  - `TASK-168`
- Objective: based on Step 0 FACT, either confirm the existing `ExtrusionRole::Ironing` → `;TYPE:Ironing` mapping (no change) or add the one line to `crates/slicer-host/src/gcode_emit.rs`.
- Precondition: Step 2 complete.
- Postcondition: G-code emitter produces `;TYPE:Ironing` markers when an `Ironing` path is committed.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` — only the role → marker map (FACT-narrowed, ≤ 60 lines).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs` (only if needed).
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT pass/fail." (only if a change was made).
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `cargo build --workspace`.
- Exit condition: mapping verified or added.

### Step 4: Implement ironing path generation

- Task IDs:
  - `TASK-168`
- Objective: implement the body of `top-surface-ironing/src/lib.rs`. Read `is_top_surface` flag and `top_solid_layers` config from the SDK view. Detect topmost-of-stack via the mechanism chosen in Step 0. Compute bounding ExPolygon of `TopSolidInfill` paths. Generate rectilinear zigzag at `ironing_spacing`. Append paths with `role = ExtrusionRole::Ironing` and `flow_factor = ironing_flow`.
- Precondition: Step 3 complete.
- Postcondition: module-level tests in `top_surface_ironing_emission_tdd.rs` PASS (`top_layer_emits_ironing_path_with_reduced_flow`, `non_topmost_layer_emits_no_ironing`, `interior_top_solid_layer_emits_no_ironing`, `disabled_config_emits_no_ironing_preserves_input`, `ironing_spacing_controls_stroke_count`, `bottom_only_layer_emits_no_ironing`, `zero_ironing_flow_is_config_error`).
- Files allowed to read:
  - `modules/core-modules/top-surface-ironing/src/lib.rs` — full.
  - `crates/slicer-sdk/src/views.rs` — only the relevant SDK accessors (FACT-narrowed range from Step 0).
  - `crates/slicer-helpers/src/lib.rs` — public API only via symbol search (zigzag generation utility if available).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/top-surface-ironing/src/lib.rs`
- Expected sub-agent dispatches:
  - "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail (post-edit rebuild)."
  - "Run `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd`; return FACT pass/fail per test."
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md`.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp::make_ironing` — already SUMMARY'd in Step 0.
- Verification:
  - rebuild script
  - `cargo test -p top-surface-ironing --test top_surface_ironing_emission_tdd -- --nocapture`
- Exit condition: module-level tests PASS; rebuild succeeds.

### Step 5: Acceptance — Benchy E2E + workspace gates

- Task IDs:
  - `TASK-168`
- Objective: confirm `benchy_gcode_contains_ironing_evidence` PASSES; full workspace test + clippy PASS; update `docs/07_implementation_status.md` with TASK-168.
- Precondition: Step 4 complete.
- Postcondition: every AC PASSES; backlog updated.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` (delegate row insertion).
  - `docs/05_module_sdk.md` (if module SDK pattern documentation needs updating).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_ironing_evidence -- --nocapture`; return FACT pass/fail."
  - "Run `cargo test --workspace`; return FACT pass/fail with failing test list (max 20 lines)."
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail."
  - "Insert TASK-168 row into `docs/07_implementation_status.md`; return FACT confirming the new line:line."
- Context cost: `S`.
- Authoritative docs: `docs/07_implementation_status.md`.
- OrcaSlicer refs: none.
- Verification: every AC command from `packet.spec.md`.
- Exit condition: every AC PASSES; `docs/07` carries TASK-168.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Four FACT/SUMMARY dispatches. |
| Step 1 | M | TDD scaffolding (new module test directory + host E2E append). |
| Step 2 | M | Module skeleton creation. |
| Step 3 | S | gcode-emit verification (often a no-op). |
| Step 4 | M | Ironing path generation logic. |
| Step 5 | S | Acceptance + doc updates. |

Aggregate: `M`. No single step is `L`.

## Packet Completion Gate

- All steps complete.
- Every AC verification command PASSES.
- `./modules/core-modules/build-core-modules.sh` PASSES.
- `docs/07_implementation_status.md` carries TASK-168.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command.
- Confirm `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `./modules/core-modules/build-core-modules.sh` PASS.
- Record any remaining packet-local risk (especially: any chosen defaults that diverge from Orca; topmost-layer detection edge cases for stepped objects).
- Confirm implementer's peak context usage stayed under 70%.
