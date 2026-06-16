# Implementation Plan: 56c_threemf-negative-and-support-subtype-routing

## Execution Rules

- One atomic step at a time.
- Each step maps back to TASK-192b, TASK-192c, or TASK-193.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble. The fields below are not optional.
- Aggregate context cost is **M**. Dispatch heavier steps (Steps 2, 3) to fresh workers.
- This packet depends on Packets 56, 56b, AND 64 being `status: implemented`. Step 0 verifies the precondition.
- This packet is the terminal closure of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice. Step 7 runs `cargo test --workspace` exactly once.
- **Implementation note**: All steps have been executed. The tests use IR-level builders (no 3MF archive parsing; see design.md §Selected Approach for rationale). The `paint_segmentation.rs` piggyback was implemented alongside Packet 64's host-native migration.

## Steps

### Step 0: Precondition gate

- Task IDs:
  - `TASK-192b` (precursor)
- Objective: Verify Packets 56 and 56b are both `status: implemented`. WIT-mirror gate is NOT re-run (clean confirmed by 56b; this packet introduces no IR types).
- Precondition: Packet activated.
- Postcondition: Both predecessor packets confirmed implemented OR halt and tell the user.
- Files allowed to read: none directly. Pure dispatch step.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: everything (dispatch only).
- Expected sub-agent dispatches:
  - Question: "What is the `status:` value in the frontmatter of `.ralph/specs/56_threemf-sidecar-parser/packet.spec.md`? Return FACT one-line value." → FACT. Expected: `implemented`.
  - Question: "What is the `status:` value in the frontmatter of `.ralph/specs/56b_threemf-modifier-part-ir-routing/packet.spec.md`? Return FACT one-line value." → FACT. Expected: `implemented`.
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: Both FACTs return `implemented`.
- Exit condition: Step 1 may begin.

### Step 1: Synthetic-fixture E2E test scaffolding (TDD-RED)

- Task IDs:
  - `TASK-193`
- Objective: Author the failing E2E TDD `threemf_subtypes_synthetic_e2e_tdd.rs`. Tests cover (10 functions): `negative_part_removes_layer_polygon_area`, `negative_part_area_reduction_matches_cube_cross_section`, `negative_part_above_parent_no_subtract`, `empty_negative_part_no_subtract`, `support_enforcer_emits_paint_region`, `support_blocker_emits_paint_region`, `empty_support_enforcer_emits_nothing`, `empty_support_blocker_emits_nothing`, `negative_part_subtract_runs_before_paint_segmentation`, `support_enforcer_flows_through_paint_overrides`. Build IR struct fixtures directly: `box_mesh()` for IndexedTriangleSet, `modifier_volume_with_subtype()` for ModifierVolume, `mesh_ir_with_modifier()` for MeshIR assembly — no 3MF archive parsing.
- Precondition: Step 0 clean.
- Postcondition: Test file compiles. The negative_part tests fail (no subtract stage exists yet). The support_* tests fail (no synthetic emission exists yet).
- Files allowed to read:
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — define IR builder helpers: `box_mesh()`, `modifier_volume_with_subtype()`, `mesh_ir_with_modifier()`, `layer_plan_with_z_values()` (pattern: build meshes and modifier volumes directly, no 3MF parsing).
  - `crates/slicer-host/src/layer_executor.rs` — narrow read around `run_paint_annotation` (≈ line 525) and the `arena.take_slice()` site. This is the per-layer insertion point for Step 2.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — NEW.
- Files explicitly out-of-bounds: everything else.
- Expected sub-agent dispatches:
  - Question: "Show the IR builder helpers (`box_mesh`, `modifier_volume_with_subtype`, `mesh_ir_with_modifier`) from `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs`. SNIPPETS, ≤ 30 lines." → SNIPPETS.
  - Question: "Run `cargo check -p slicer-host --tests` after Step 1's edits. FACT pass/fail." → FACT.
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — scaled integer units.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_removes_layer_polygon_area -- --exact --nocapture` — RED.
  - `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_emits_paint_region -- --exact --nocapture` — RED.
- Exit condition: All tests compile; primary RED tests fail as expected.

### Step 2: Implement `apply_negative_part_subtract` host stage

- Task IDs:
  - `TASK-192b`
- Objective: Create `crates/slicer-host/src/negative_part_subtract.rs` with `apply_negative_part_subtract(slice_ir: &mut SliceIR, modifier_volumes: &[ModifierVolume])`. Insert call inside `crates/slicer-host/src/layer_executor.rs::run_paint_annotation` after `arena.take_slice()` and BEFORE the paint annotation loop begins. For each `negative_part` modifier volume, project the modifier mesh at `slice_ir.z` via `slice_mesh_ex` and apply `slicer_core::polygon_ops::difference` per `SlicedRegion.polygons`. Modifiers outside the layer's Z extent are skipped.
- Precondition: Step 1 RED.
- Postcondition: `negative_part_removes_layer_polygon_area`, `negative_part_area_reduction_matches_cube_cross_section`, `negative_part_above_parent_no_subtract`, `empty_negative_part_no_subtract`, `negative_part_subtract_runs_before_paint_segmentation` are GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/layer_executor.rs` — narrow read at `run_paint_annotation` (≈ line 525), the `arena.take_slice()` site, and the existing `slice_mesh_ex` call (`layer_executor.rs:559-562`) for the Packet 56b projection pattern.
  - `crates/slicer-ir/src/slice_ir.rs` — narrow reads at `SliceIR` (≈ line 1102), `SlicedRegion` (≈ line 1068), `ModifierVolume` (≈ line 252), `ConfigDelta` (≈ line 231).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/negative_part_subtract.rs` — NEW.
  - `crates/slicer-host/src/layer_executor.rs` — insert per-layer call inside `run_paint_annotation`, after `arena.take_slice()` and before the paint annotation loop begins.
  - `crates/slicer-host/src/lib.rs` (or `mod.rs` — the host crate's module root) — declare the new `pub mod negative_part_subtract`. Step 2 FACT dispatch returns the correct module-root file.
- Files explicitly out-of-bounds: `model_loader.rs`, `region_mapping.rs`, `pipeline.rs`, `prepass.rs`, `paint_segmentation.rs` (Step 3 territory), macros, WIT, SDK, IR (read-only narrow).
- Expected sub-agent dispatches:
  - Question: "Return the exact line in `crates/slicer-host/src/layer_executor.rs::run_paint_annotation` where `arena.take_slice()` returns the layer's `SliceIR`, immediately before the paint annotation loop begins. FACT with file:line." → FACT.
  - Question: "Return the exact place in `crates/slicer-host/src/layer_executor.rs::run_paint_annotation` where the current object's `&[ModifierVolume]` is in scope alongside the mutable `SliceIR`, BEFORE the paint annotation loop runs. SNIPPETS, ≤ 15 lines." → SNIPPETS.
  - Question: "Which file declares the host crate's module roots (e.g., `pub mod negative_part_subtract`)? FACT with file path." → FACT.
  - Question: "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` (or sibling) that perform negative-part per-layer subtract. LOCATIONS, ≤ 5 entries. No source." → LOCATIONS.
  - Question: "Run `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd negative_part_removes_layer_polygon_area negative_part_area_reduction_matches_cube_cross_section negative_part_above_parent_no_subtract empty_negative_part_no_subtract negative_part_subtract_runs_before_paint_segmentation`. FACT pass/fail per test." → FACT.

  Note: `SliceIR` shape (`SliceIR { z, global_layer_index, regions: Vec<SlicedRegion> }` per-layer), `SlicedRegion.polygons: Vec<ExPolygon>`, `polygon_ops::difference(&[ExPolygon], &[ExPolygon]) -> Vec<ExPolygon>`, and `slice_mesh_ex(&IndexedTriangleSet, &[f32]) -> Vec<Vec<ExPolygon>>` are all already verified during the spec-review of this packet — no further FACT dispatches needed for those shapes.
- Context cost: `M`
- Authoritative docs:
  - `docs/04_host_scheduler.md` — prepass / region-mapping ordering. Delegate SUMMARY.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — LOCATIONS dispatch.
- Verification:
  - Five negative_part tests GREEN.
  - `cargo test -p slicer-host --test benchy_painted_e2e_tdd` → must stay GREEN.
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` → must stay GREEN.
- Exit condition: Five negative_part tests GREEN; regressions stay GREEN.

### Step 3: Implement support enforcer/blocker paint-segmentation piggyback

- Task IDs:
  - `TASK-192c`
- Objective: Augment `paint_segmentation.rs::execute_paint_segmentation` to read `mesh_ir.objects[].modifier_volumes` directly (no new parameter on the function signature). For each `support_enforcer` / `support_blocker` modifier volume, project per layer via `slice_mesh_ex` and insert synthetic `SemanticRegion` entries into `LayerPaintMap.semantic_regions` (`HashMap<PaintSemantic, Vec<SemanticRegion>>`) under the matching `PaintSemantic` variant. Use `entry(layer).or_insert_with(LayerPaintMap::default).semantic_regions.entry(semantic).or_default().push(...)` for layers without prior entries, and `polygon_ops::union` to merge polygons when prior `SemanticRegion`s already exist at the same `(layer, semantic)`.
- Precondition: Step 2 GREEN.
- Postcondition: `support_enforcer_emits_paint_region`, `support_blocker_emits_paint_region`, `empty_support_enforcer_emits_nothing`, `empty_support_blocker_emits_nothing`, `support_enforcer_flows_through_paint_overrides` GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/paint_segmentation.rs` — full (verified ~400 lines at review time; entry point `execute_paint_segmentation` at line 50).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/paint_segmentation.rs` — augment with synthetic-volume emission helper that reads `mesh_ir.objects[].modifier_volumes` directly.
- Files explicitly out-of-bounds: WIT, SDK, macros, `model_loader.rs`, `region_mapping.rs`, `pipeline.rs`, `prepass.rs`, `layer_executor.rs` (already touched at Step 2), `negative_part_subtract.rs` (already complete).
- Expected sub-agent dispatches:
  - Question: "Return the existing `execute_paint_segmentation` entry-point function body in `crates/slicer-host/src/paint_segmentation.rs` — specifically the place where the final `PaintRegionIR` is constructed/returned. SNIPPETS, ≤ 30 lines." → SNIPPETS.
  - Question: "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or sibling) that emit `support_enforcer` / `support_blocker` geometry into the slicer's paint pipeline. LOCATIONS, ≤ 5 entries. No source." → LOCATIONS.
  - Question: "Run `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd support_enforcer_emits_paint_region support_blocker_emits_paint_region empty_support_enforcer_emits_nothing empty_support_blocker_emits_nothing support_enforcer_flows_through_paint_overrides`. FACT pass/fail per test." → FACT.

  Note: `PaintRegionIR.per_layer: HashMap<u32, LayerPaintMap>`, `LayerPaintMap.semantic_regions: HashMap<PaintSemantic, Vec<SemanticRegion>>`, `SemanticRegion.polygons: Vec<ExPolygon>`, `PaintSemantic::SupportEnforcer`/`SupportBlocker`, and `polygon_ops::union(&[ExPolygon], &[ExPolygon]) -> Vec<ExPolygon>` are all already verified during the spec-review of this packet — no further FACT dispatches needed for those shapes.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `PaintRegionIR`, `PaintSemantic` block search (narrow read).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — LOCATIONS dispatch.
- Verification:
  - Five support_* tests GREEN.
  - `cargo test -p slicer-host --test benchy_painted_e2e_tdd painted_benchy_3mf_reaches_paint_segmentation` → must stay GREEN.
- Exit condition: All ten `threemf_subtypes_synthetic_e2e_tdd` tests GREEN; regressions stay GREEN.

### Step 4: Regression sweep + clippy

- Task IDs:
  - `TASK-193`
- Objective: Re-run all regression-defense suites and assert clean. Confirm clippy + check.
- Precondition: Step 3 GREEN.
- Postcondition: All regression suites GREEN; clippy clean.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): any source file the lint pass demands (sticking to files-in-scope from earlier steps).
- Files explicitly out-of-bounds: macros, WIT, SDK, IR, files owned by Packets 56 / 56b.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd && cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd && cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd`. Return FACT pass/fail per file." → FACT.
  - Question: "Run `cargo clippy --workspace -- -D warnings`. FACT pass/fail with first warning if fail." → FACT.
  - Question: "Run `cargo check --workspace`. FACT pass/fail with first error if fail." → FACT.
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: All three FACTs GREEN.
- Exit condition: Clean workspace; no regressions.

### Step 5: Doc registration

- Task IDs:
  - `TASK-193`
- Objective: Append TASK-192b, TASK-192c, TASK-193 rows in `docs/07_implementation_status.md` naming this packet. No deviation registration (this packet registers none).
- Precondition: Step 4 clean.
- Postcondition: `docs/07` reflects packet outcome.
- Files allowed to read:
  - `docs/07_implementation_status.md` — delegate FACT for the append position only.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md`
- Files explicitly out-of-bounds: all source; `docs/DEVIATION_LOG.md` (no new DEVs in this packet); `docs/14_deviation_audit_history.md` (no chronology entries).
- Expected sub-agent dispatches:
  - Question: "Append `[x] TASK-192b`, `[x] TASK-192c`, and `[x] TASK-193` rows to `docs/07_implementation_status.md` immediately after the TASK-192a row registered by Packet 56b, each naming packet `56c_threemf-negative-and-support-subtype-routing`. Return the resulting three lines verbatim. SNIPPETS, ≤ 5 lines." → SNIPPETS.
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q '\[x\] TASK-192b.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md` → exit 0.
  - `rg -q '\[x\] TASK-192c.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md` → exit 0.
  - `rg -q '\[x\] TASK-193.*56c_threemf-negative-and-support-subtype-routing' docs/07_implementation_status.md` → exit 0.
- Exit condition: All three `rg` checks pass.

### Step 6: Pre-ceremony verification

- Task IDs:
  - `TASK-193`
- Objective: Re-run every pipe-suffixed AC command from `packet.spec.md` to confirm GREEN before the workspace gate.
- Precondition: Step 5 complete.
- Postcondition: All AC commands return PASS.
- Files allowed to read: `packet.spec.md` (this packet).
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: every source file.
- Expected sub-agent dispatches:
  - One dispatch per AC command, each returning FACT pass/fail.
- Context cost: `S`
- Authoritative docs: this packet's `packet.spec.md`.
- OrcaSlicer refs: none.
- Verification:
  - All AC commands return PASS.
- Exit condition: All FACTs GREEN.

### Step 7: Acceptance ceremony + workspace gate

- Task IDs:
  - All packet TASK ids.
- Objective: Run `cargo test --workspace` exactly once via worker FACT dispatch (per CLAUDE.md Test Discipline acceptance-ceremony allowance). This is the terminal closure of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` three-way slice; a workspace-wide gate is justified.
- Precondition: Step 6 GREEN.
- Postcondition: `cargo test --workspace` reports all-pass; `packet.spec.md` ready to flip to `status: implemented`.
- Files allowed to read: `packet.spec.md` (this packet).
- Files allowed to edit (≤ 3): `packet.spec.md` (status flip on success only).
- Files explicitly out-of-bounds: every source file.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test --workspace`. Return FACT pass-count vs total. Do not paste failing test output unless count > 0; in that case return the first failing test name + ≤ 5 lines of assertion." → FACT.
- Context cost: `M`
- Authoritative docs: this packet's `packet.spec.md`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test --workspace` returns all-pass.
- Exit condition: Status flippable to `implemented`. Three-way split complete.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| Step 0 | S | Precondition check (two FACTs). |
| Step 1 | S | Synthetic-fixture E2E test scaffolding + RED. |
| Step 2 | M | `apply_negative_part_subtract` stage + pipeline insertion + module declaration. |
| Step 3 | M | Paint-segmentation piggyback for support subtypes. |
| Step 4 | S | Regression + clippy + check sweep dispatches. |
| Step 5 | S | Doc registration. |
| Step 6 | S | Pre-ceremony AC verification dispatches. |
| Step 7 | M | Workspace test dispatch (acceptance ceremony). |

Aggregate: **M** (3 M + 5 S).

## Packet Completion Gate

- All 8 steps complete.
- Every step exit condition met.
- All AC commands GREEN (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-192b, TASK-192c, TASK-193 via worker dispatch.
- No new DEVs registered (DEV-047, DEV-048, DEV-049 are closed by Packets 56 / 56b).
- `cargo test --workspace` GREEN at acceptance ceremony.
- `packet.spec.md` ready to move to `status: implemented`.
- Three-way split complete: Packets 56, 56b, 56c all `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md` (Step 6).
- Run `cargo test --workspace` exactly once via worker FACT dispatch (Step 7) — the only packet in the three-way split that does so.
- Confirm peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.
- The closure of this packet marks the end of the original `56_threemf-modifier-and-subtype-sidecar-ingestion` slice. The 3MF loader now handles all five OrcaSlicer / Bambu Studio `<part subtype>` values end-to-end.
