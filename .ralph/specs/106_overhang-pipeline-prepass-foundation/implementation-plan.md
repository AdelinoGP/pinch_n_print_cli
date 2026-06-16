# Implementation Plan: 106_overhang-pipeline-prepass-foundation

## Execution Rules

- One atomic step at a time.
- Each step must map back to the packet's grouped task IDs.
- TDD first (write the failing test before the production change), then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`.

## Steps

### Step 1: O-T001/O-T002 — ADR-0012 + close roadmap open decisions

- Task IDs:
  - `O-T001` — Author ADR-0012
  - `O-T002` — Close O-1..O-8 inline in overhang roadmap
- Objective: write the ADR; mark O-1..O-8 CLOSED with the documented defaults.
- Precondition: workspace builds clean.
- Postcondition: AC-1 + AC-2 verification commands pass.
- Files allowed to read:
  - `docs/adr/0008-overhang-as-finalization-module.md` — read full.
  - `docs/specs/overhang-pipeline-restructuring.md` — read full.
- Files allowed to edit (≤ 3):
  - `docs/adr/0012-overhang-classification-at-prepass.md` (NEW)
  - `docs/specs/overhang-pipeline-restructuring.md` (close O-1..O-8)
- Files explicitly out-of-bounds: all source files.
- Expected sub-agent dispatches: none.
- Context cost: `S`
- Authoritative docs: `docs/adr/0008-overhang-as-finalization-module.md`, `docs/specs/overhang-pipeline-restructuring.md`.
- OrcaSlicer refs: none.
- Verification: AC-1 + AC-2 greps.
- Exit condition: ADR exists with supersession language; all 8 O-decisions show CLOSED.

### Step 2: O-T010/O-T011 — IR additions

- Task IDs:
  - `O-T010` — `OverhangRegion.xy_footprint` + `MeshAnalysis` populator
  - `O-T011` — `SurfaceClassificationIR.overhang_quartile_polygons` + `QuartileBand` type
- Objective: extend the IR with the two additions, mirror in WIT, populate `xy_footprint` at the existing `OverhangRegion` construction site, bump schema additively.
- Precondition: Step 1 exit condition met.
- Postcondition: AC-3 verification grep passes; `cargo xtask build-guests --check` no STALE.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — range-read by `rg -n 'OverhangRegion|SurfaceClassificationIR|CURRENT_SLICE_IR_SCHEMA_VERSION|BridgeRegion'`.
  - `crates/slicer-schema/wit/deps/ir-types.wit` — full.
  - `crates/slicer-core/src/algos/mesh_analysis.rs` — range-read around line 206 (existing `OverhangRegion` construction).
- Files allowed to edit (≤ 3 per sub-step):
  - 2a (IR + WIT): `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-schema/wit/deps/ir-types.wit`.
  - 2b (populator): `crates/slicer-core/src/algos/mesh_analysis.rs`, `crates/slicer-runtime/tests/unit/mesh_analysis_overhang_xy_footprint_tdd.rs` (NEW).
- Files explicitly out-of-bounds: `slicer-core/algos/overhang_annotation.rs` (Step 4 — does not exist yet).
- Expected sub-agent dispatches:
  - "Find the existing OverhangRegion construction site at `crates/slicer-core/src/algos/mesh_analysis.rs` around line 206; confirm field set + facet-cluster pattern; return FACT (field list + line range)."
  - "Find `BridgeRegion.xy_footprint` populator pattern; return SNIPPETS ≤ 30 lines (the `compute_xy_footprint` function or analogous)."
  - "Run `cargo build --tests --workspace`; return FACT (pass/fail)."
  - "Run `cargo xtask build-guests --check`; return FACT (clean / STALE list)."
- Context cost: `M`
- Authoritative docs: `docs/02_ir_schemas.md` (delegate SUMMARY), `docs/03_wit_and_manifest.md` §"WIT/Type Changes Checklist".
- OrcaSlicer refs: none for Step 2.
- Verification:
  - `rg -q 'pub xy_footprint: Vec<ExPolygon>' crates/slicer-ir/src/slice_ir.rs` — exit 0.
  - `rg -q 'pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>' crates/slicer-ir/src/slice_ir.rs` — exit 0.
  - `cargo build --tests --workspace 2>&1 | tee target/test-output.log` — FACT.
  - `cargo xtask build-guests --check` — no STALE.
- Exit condition: AC-3 grep passes; build clean; no STALE guests.

### Step 3: O-T012 — Promote mesh cross-section helper

- Task IDs:
  - `O-T012` — Extract plane-triangle intersection to `crates/slicer-core/src/algos/mesh_cross_section.rs`
- Objective: extract the plane-triangle intersection helper from `support_geometry.rs` to a new shared module; verify `support_geometry` consumes the promoted helper and its existing tests stay green.
- Precondition: Step 2 exit condition met.
- Postcondition: AC-4 verification passes.
- Files allowed to read:
  - `crates/slicer-core/src/algos/support_geometry.rs` — range-read by `rg -n 'plane_triangle|cross_section|slice_at_z'`.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/mesh_cross_section.rs` (NEW)
  - `crates/slicer-core/src/algos/support_geometry.rs` (consume the promoted helper)
  - `crates/slicer-core/src/algos/mod.rs` (declare `pub mod mesh_cross_section`)
- Files explicitly out-of-bounds: `overhang_annotation.rs` (Step 4).
- Expected sub-agent dispatches:
  - "Find the plane-triangle intersection function in `crates/slicer-core/src/algos/support_geometry.rs`; return LOCATIONS (function name + line range)."
  - "Run `cargo test -p slicer-core --test prepass_support_geometry_tdd`; return FACT pass/fail."
- Context cost: `M`
- Authoritative docs: align with `slicer-core/src/algos/` existing conventions (this is `slicer-core`, not `slicer-helpers` — `docs/13_slicer_helpers_crate.md` does not apply).
- OrcaSlicer refs: none for Step 3.
- Verification:
  - `rg -q 'pub fn cross_section_at_z' crates/slicer-core/src/algos/mesh_cross_section.rs` — exit 0.
  - `cargo test -p slicer-core --test prepass_support_geometry_tdd 2>&1 | tee target/test-output.log` — FACT pass.
- Exit condition: AC-4 green; existing `support_geometry` test passes after promotion.

### Step 4: O-T021/O-T022 — Implement classifier algorithm

- Task IDs:
  - `O-T021` — Classifier algorithm in `overhang_annotation.rs`
  - `O-T022` — Wire quartile thresholds to config (`line_width × {0.5, 1.0, 1.5, 2.0}`)
- Objective: implement the per-layer quartile classifier as a pure function; write the overhang-ramp TDD; threshold config-read.
- Precondition: Step 3 exit condition met.
- Postcondition: AC-5 + AC-N1 verification commands pass.
- Files allowed to read:
  - `crates/slicer-core/src/algos/mesh_cross_section.rs` (just created in Step 3).
  - `crates/slicer-ir/src/slice_ir.rs` (range-read `QuartileBand`, `SurfaceClassificationIR`).
- Files allowed to edit (≤ 3 per sub-step):
  - 4a (algorithm): `crates/slicer-core/src/algos/overhang_annotation.rs` (NEW), `crates/slicer-core/src/algos/mod.rs` (declare `pub mod overhang_annotation`).
  - 4b (positive TDD): `crates/slicer-core/tests/overhang_annotation_ramp_tdd.rs` (NEW; AC-5).
  - 4c (negative TDD): `crates/slicer-core/tests/overhang_annotation_no_overhang_case.rs` (NEW; AC-N1 — separate test binary so the AC-N1 verification command `cargo test --test overhang_annotation_no_overhang_case` resolves).
- Files explicitly out-of-bounds: stage wiring (Step 5).
- Expected sub-agent dispatches:
  - "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:159-199 for `detect_steep_overhang` algorithm; SUMMARY ≤ 150 words. Confirm: input is slice polygons; threshold formula uses `extrusion_width × multiplier`; quartile band count is 4."
  - "Run `cargo test -p slicer-core --test overhang_annotation_ramp_tdd` and `cargo test -p slicer-core --test overhang_annotation_no_overhang_case`; FACT pass/fail per case."
- Context cost: `M` (largest step — algorithm + reference TDD)
- Authoritative docs: `docs/specs/overhang-pipeline-restructuring.md` Phase 2 rows.
- OrcaSlicer refs: `PerimeterGenerator.cpp:159-199` (delegate SUMMARY).
- Verification:
  - `cargo test -p slicer-core --test overhang_annotation_ramp_tdd 2>&1 | tee target/test-output.log` — FACT.
  - `cargo test -p slicer-core --test overhang_annotation_no_overhang_case 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-5 + AC-N1 green.

### Step 5: O-T020/O-T023 — Stage declaration + host runner

- Task IDs:
  - `O-T020` — Declare `PrePass::OverhangAnnotation` in stage order
  - `O-T023` — Host stage runner: invoke after MeshAnalysis + LayerPlanning commit; write to Blackboard
- Objective: declare the new stage with explicit precondition dependencies; wire the host stage runner to invoke the Step 4 classifier and commit the result to the Blackboard's `SurfaceClassificationIR`.
- Precondition: Step 4 exit condition met.
- Postcondition: AC-6 + AC-N2 verification commands pass.
- Files allowed to read:
  - `crates/slicer-scheduler/src/execution_plan.rs` — range-read by `rg -n 'PrePass|MeshAnalysis|LayerPlanning'`.
  - `crates/slicer-runtime/src/prepass.rs` — range-read by `rg -n 'MeshAnalysis|LayerPlanning|commit'`.
  - `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` — full (pattern reference).
- Files allowed to edit (≤ 3 per sub-step):
  - 5a (scheduler): `crates/slicer-scheduler/src/execution_plan.rs`.
  - 5b (runtime): `crates/slicer-runtime/src/prepass.rs`, `crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs` (NEW; if pattern mandates; else inline).
  - 5c (test): `crates/slicer-runtime/tests/executor/prepass_overhang_annotation_stage_order_tdd.rs` (NEW; covers AC-6 positive + AC-N2 violation case).
- Files explicitly out-of-bounds: any other source file.
- Expected sub-agent dispatches:
  - "Find an existing prepass stage's scheduler declaration + builtin producer wiring (e.g., RegionMapping producer); return LOCATIONS ≤ 5 entries (the pattern to mirror)."
  - "Run `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd`; FACT pass/fail per case."
- Context cost: `M`
- Authoritative docs: `docs/04_host_scheduler.md` (delegate SUMMARY if needed for stage-ordering rules).
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --test executor prepass_overhang_annotation_stage_order_tdd 2>&1 | tee target/test-output.log` — FACT.
- Exit condition: AC-6 + AC-N2 green; stage runs after MeshAnalysis + LayerPlanning; violation rejected.

### Step 6: Doc impact landing

- Task IDs:
  - Doc impact for this packet (`docs/01_system_architecture.md`, `docs/02_ir_schemas.md`).
- Objective: land architecture + IR doc updates.
- Precondition: Step 5 exit condition met.
- Postcondition: all Doc Impact Statement greps pass.
- Files allowed to read:
  - `docs/01_system_architecture.md` (range-read §"Tier 1 — PrePass").
  - `docs/02_ir_schemas.md` (range-read SurfaceClassificationIR section).
- Files allowed to edit (≤ 4):
  - `docs/04_host_scheduler.md` (EDIT — register `PrePass::OverhangAnnotation` in STAGE_ORDER after `PrePassLayerPlanning`, add stage description paragraph, add Stage Prerequisites table entry)
  - `docs/01_system_architecture.md` (EDIT — register `PrePass::OverhangAnnotation` in PrePass Stage Order list, prose block, and Stage I/O Contract table)
  - `docs/02_ir_schemas.md`
- Files explicitly out-of-bounds: source files.
- Expected sub-agent dispatches:
  - "For each Doc Impact grep, run `rg -q`; return FACT pass/fail per grep."
- Context cost: `S`
- Authoritative docs: the three files being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'PrePass::OverhangAnnotation' docs/04_host_scheduler.md` — exit 0.
  - `rg -q 'OverhangAnnotation' docs/01_system_architecture.md` — exit 0.
  - `rg -q 'OverhangRegion.*xy_footprint' docs/02_ir_schemas.md` — exit 0.
  - `rg -q 'overhang_quartile_polygons' docs/02_ir_schemas.md` — exit 0.
- Falsifying check after edits: `rg -q 'PrePassOverhangAnnotation' docs/04_host_scheduler.md && rg -q 'OverhangAnnotation' docs/01_system_architecture.md` — both must exit 0; if either fails the doc edits are incomplete.
- Exit condition: all Doc Impact Statement greps pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | ADR write + roadmap close. |
| Step 2 | M | IR + WIT + populator; guest-WASM gate. |
| Step 3 | M | Helper promotion + regression check. |
| Step 4 | M | Largest step — classifier algorithm + reference TDD; OrcaSlicer SUMMARY. |
| Step 5 | M | Scheduler + runtime + stage-order TDD. |
| Step 6 | S | Two doc edits. |

Aggregate context cost: `M`. No step `L`. Per-step file edit count ≤ 3 (Steps 2 and 5 split into sub-steps).

## Packet Completion Gate

- All six steps complete; each exit condition met.
- AC-1 through AC-6 + AC-N1 + AC-N2 all PASS via worker dispatch.
- `cargo check --workspace --all-targets` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo xtask build-guests --check` reports no STALE guests.
- `docs/07_implementation_status.md` updated for each O-T001..O-T023 entry — via worker dispatch.
- `packet.spec.md` ready to move `draft` → `implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm gate commands green.
- Record the schema-bump direction (vs P105 sequencing) in the closure log.
- If the OrcaSlicer SUMMARY surfaced a threshold formula different from `× {0.5, 1.0, 1.5, 2.0}`, record the deviation.
- Confirm implementer's peak context usage < 70%.
