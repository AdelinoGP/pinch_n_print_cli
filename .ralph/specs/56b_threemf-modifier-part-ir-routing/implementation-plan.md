# Implementation Plan: 56b_threemf-modifier-part-ir-routing

## Execution Rules

- One atomic step at a time.
- Each step maps back to TASK-191 or TASK-192a.
- TDD first, then implementation, then the narrowest falsifying validation.
- Each step honors the context-discipline preamble. The fields below are not optional — they are the budget contract.
- Aggregate context cost is **M**. Dispatch heavier steps (Steps 2, 5) to fresh workers.
- This packet depends on **Packet 56 (`56_threemf-sidecar-parser`) being `status: implemented`**. Step 0 verifies the precondition.

## Steps

### Step 0: Precondition gate + WIT-mirror re-check

- Task IDs:
  - `TASK-191` (precursor)
- Objective: Verify Packet 56 is closed. Re-run the WIT-mirror gate because this packet widens the producer contract for `ObjectMesh.modifier_volumes` for the first time.
- Precondition: Packet activated. This packet is the active packet.
- Postcondition: Either (a) Packet 56 is implemented AND WIT is clean → continue; (b) Packet 56 is still draft/active → halt and tell the user; (c) WIT mirror discovered → halt and register DEV-043-style escalation.
- Files allowed to read: none directly. Pure dispatch step.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: everything (dispatch only).
- Expected sub-agent dispatches:
  - Question: "What is the `status:` value in the frontmatter of `.ralph/specs/56_threemf-sidecar-parser/packet.spec.md`? Return FACT one-line value." → FACT. Expected: `implemented`.
  - Question: "Does any guest-visible WIT type or its host mirror expose `ObjectMesh.modifier_volumes` or a `ModifierVolume` shape? Scope: `wit/**`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs`. Return FACT yes/no with file:line if yes; ≤ 8 lines." → FACT.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — delegate SUMMARY only if gate flips.
- OrcaSlicer refs: none.
- Verification: Both FACTs return clean.
- Exit condition: Step 1 may begin.

### Step 1: `resolve_object` branching TDD-RED + E2E test scaffolding

- Task IDs:
  - `TASK-191`
- Objective: Author the failing E2E TDD `benchy_4color_modifier_part_e2e_tdd.rs` covering: (a) triangle counts (225,240 in solid, 12 in modifier); (b) `modifier_volumes.len() == 1` with typed `config_delta`; (c) world-space AABB centroid within ±0.01 mm; (d) `MeshIR.schema_version == 1.1.0`. Author the failing DEV-052 negative case in `threemf_paint_drop_on_modifier_tdd.rs` (or fold into the parser suite at Step 2's discretion). Optionally add `empty_modifier_volume_stamps_no_regions` test stub.
- Precondition: Step 0 clean.
- Postcondition: New test files compile and fail on assertion (current `resolve_object` body merges everything; `modifier_volumes` is still empty).
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 130-552 (post-Packet-56 state).
  - `crates/slicer-ir/src/slice_ir.rs` — lines 230-295 (informational).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — NEW.
  - `crates/slicer-host/tests/threemf_paint_drop_on_modifier_tdd.rs` — NEW.
- Files explicitly out-of-bounds: everything else.
- Expected sub-agent dispatches:
  - Question: "Confirm `resources/benchy_4color.3mf` exists, is readable, and report file size. FACT one line." → FACT.
  - Question: "Compute the expected world-space AABB centroid of the modifier cube inside `resources/benchy_4color.3mf` using (a) the `<build>/<item>` transform from `3D/3dmodel.model` and (b) the `<component objectid="2">` row-major transform. Return FACT in the form `centroid: (x_mm, y_mm, z_mm)`; ≤ 2 lines. Use the OrcaSlicerDocumented matrix-composition pattern; do not load Orca source — the math is in `docs/08_coordinate_system.md` + the model XML structure." → FACT.
  - Question: "Run `cargo check -p slicer-host --tests` after Step 1's edits. FACT pass/fail." → FACT.
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — scaled integer units.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_part_excluded_from_solid_mesh -- --exact --nocapture` — expected RED (`225_252 != 225_240`).
  - `cargo test -p slicer-host --test threemf_paint_drop_on_modifier_tdd paint_on_modifier_part_dropped_with_warning -- --exact --nocapture` — expected RED.
- Exit condition: Both TDDs RED with the expected assertion lines.

### Step 2: Implement `resolve_object` branching + `ModifierVolume` construction + paint drop + schema bump

- Task IDs:
  - `TASK-191`
- Objective: Rename `_sidecar` → `sidecar` in `resolve_object`'s signature; thread classification through the component recursion. For each non-`NormalPart` part, construct a `ModifierVolume` with typed `config_delta`. Drop paint data on non-`NormalPart` rows with `log::warn!`. Bump `SemVer { 1, 0, 0 }` → `SemVer { 1, 1, 0 }` at lines 194-199.
- Precondition: Step 1 RED.
- Postcondition: All `benchy_4color_modifier_part_e2e_tdd` ACs covering triangle counts + `modifier_volumes` + schema bump + world AABB are GREEN. Paint-drop negative is GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — lines 130-552 (post-Packet-56 state).
  - `crates/slicer-ir/src/slice_ir.rs` — lines 192-295.
  - `crates/slicer-host/src/config_resolution.rs` — lines 80-220 (`ConfigKey` / `ConfigValue` helpers).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/model_loader.rs` — `resolve_object` body + schema bump.
- Files explicitly out-of-bounds: every other source file (Steps 5/6 touch `region_mapping.rs` and `pipeline.rs`).
- Expected sub-agent dispatches:
  - Question: "What is `ConfigKey`'s constructor signature in `crates/slicer-ir/`? Can it accept arbitrary strings? List the variants of `ConfigValue`. FACT, ≤ 6 lines." → FACT.
  - Question: "Return the one-line `ModifierId` derivation recipe used by Packet 39's `stable-entity-ids` precedent (search `crates/slicer-host/src/` for the existing pattern). SNIPPETS, ≤ 5 lines." → SNIPPETS.
  - Question: "Return the log target string used in the 5 most recent `log::warn!` calls in `crates/slicer-host/src/model_loader.rs` (post-Packet-56). SNIPPETS, ≤ 5 lines." → SNIPPETS.
  - Question: "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` that branch on `<part subtype>` and route geometry into the modifier-volume container. Return LOCATIONS, ≤ 5 entries. No source." → LOCATIONS.
  - Question: "Run `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd && cargo test -p slicer-host --test threemf_paint_drop_on_modifier_tdd`. FACT pass/fail per test." → FACT.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` lines 5, 192-211 (versioning rule + `ConfigDelta`/`ModifierVolume`). Read directly.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Format/bbs_3mf.cpp` — LOCATIONS dispatch.
- Verification:
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_part_excluded_from_solid_mesh -- --exact --nocapture` → GREEN.
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_volume_carries_typed_metadata -- --exact --nocapture` → GREEN.
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd modifier_world_aabb_matches_composition -- --exact --nocapture` → GREEN.
  - `cargo test -p slicer-host --test threemf_paint_drop_on_modifier_tdd paint_on_modifier_part_dropped_with_warning -- --exact --nocapture` → GREEN.
  - `cargo test -p slicer-host --test threemf_transform_tdd` → must stay GREEN.
  - `cargo test -p slicer-host --test threemf_sidecar_classification_tdd` → must stay GREEN.
- Exit condition: All four ACs above GREEN; regressions GREEN. `region_overlap_*` and `fuzzy_region_restricted_*` still RED (Steps 4/5 fix them).

### Step 3: Fuzzy-skin manifest schema confirmation gate

- Task IDs:
  - `TASK-192a`
- Objective: Verify `apply_to_all` is declared in `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`'s `[config.schema]`. If absent, add it (additive; no SemVer ripple). If present, no-op.
- Precondition: Step 2 GREEN.
- Postcondition: `fuzzy-skin` manifest declares `apply_to_all`.
- Files allowed to read:
  - `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`.
  - `modules/core-modules/fuzzy-skin/src/lib.rs` — lines 1-120 (read-only verification; already analyzed at packet-author time).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/fuzzy-skin/fuzzy-skin.toml` — additive only.
- Files explicitly out-of-bounds: all other files.
- Expected sub-agent dispatches:
  - Question: "Does `modules/core-modules/fuzzy-skin/fuzzy-skin.toml`'s `[config.schema]` block declare an entry whose name matches `apply_to_all` or `apply_to_all` (any spelling variant)? Return FACT yes/no with the verbatim name + file:line if yes." → FACT.
- Context cost: `S`
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — module manifest TOML schema. Delegate SUMMARY if needed for the additive-edit syntax.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test core_module_ir_access_contract_tdd` → GREEN.
  - `cargo build --workspace` → GREEN.
- Exit condition: `apply_to_all` (verbatim verified key) present in manifest; build clean. If the verbatim key differs from `apply_to_all`, update the AC literals in `packet.spec.md` to match.

### Step 4: Region-mapping overlap stamp TDD-RED

- Task IDs:
  - `TASK-192a`
- Objective: Author the failing TDD for `region_overlap_stamps_only_in_cube_zband` and `fuzzy_region_restricted_to_cube_and_painted_facets` in `benchy_4color_modifier_part_e2e_tdd.rs`. Also add `empty_modifier_volume_stamps_no_regions` if not stubbed at Step 1.
- Precondition: Step 3 GREEN.
- Postcondition: New tests compile and fail (current `execute_region_mapping` doesn't stamp `fuzzy_skin.apply_to_all`).
- Files allowed to read:
  - `crates/slicer-host/src/region_mapping.rs` — lines 1-260.
  - `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` — `count_perimeter_markers_in_z_band` helper.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — extend.
- Files explicitly out-of-bounds: WIT, SDK, macros, IR.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd region_overlap_stamps_only_in_cube_zband fuzzy_region_restricted_to_cube_and_painted_facets`. FACT pass/fail per test." → FACT.
- Context cost: `S`
- Authoritative docs: none new.
- OrcaSlicer refs: none.
- Verification: Both new tests RED.
- Exit condition: TDD-RED with expected assertion lines.

### Step 5: Implement region-mapping overlap stamp + pipeline thread

- Task IDs:
  - `TASK-192a`
- Objective: Extend `execute_region_mapping` to accept `&[ModifierVolume]` per object (or read from `ExecutionPlan`). Project each `modifier_part` per layer; compute `slicer_core::polygon_ops::intersection` against each region polygon; stamp `RegionPlan.config[ConfigKey::from("fuzzy_skin.apply_to_all")] = ConfigValue::Bool(true)` on non-empty overlap. Preserve the no-modifier fast path. Thread `modifier_volumes` from `pipeline.rs` into the call.
- Precondition: Step 4 RED.
- Postcondition: All four `benchy_4color_modifier_part_e2e_tdd` ACs GREEN: triangle counts, `modifier_volumes`, world AABB, region-overlap Z-band, fuzzy G-code restriction.
- Files allowed to read:
  - `crates/slicer-host/src/region_mapping.rs` — full (≤ 260 lines).
  - `crates/slicer-host/src/pipeline.rs` — search for `execute_region_mapping(` call site.
  - `crates/slicer-host/src/config_resolution.rs` — lines 80-220.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_executor.rs` — `run_paint_annotation` modifier projection loop.
  - `crates/slicer-host/src/slice_postprocess.rs` — `modifier_projections` field + overlap check.
  - `crates/slicer-core/src/paint_region.rs` — `ex_polygon_contains_point` made `pub`.
  - `crates/slicer-host/tests/benchy_4color_modifier_part_e2e_tdd.rs` — extend if needed for fuzz-marker assertions.
  - Note: `region_mapping.rs` received a structural refactor only (wrapping); `pipeline.rs` was unchanged.
- Files explicitly out-of-bounds: macros, WIT, SDK, IR.
- Expected sub-agent dispatches:
  - Question: "Which function in `slicer-core` slices an `IndexedTriangleSet` at a given Z plane and returns 2D polygons in scaled integer units? Return FACT with function path + signature." → FACT.
  - Question: "Return `slicer_core::polygon_ops::intersection` signature. SNIPPETS, ≤ 6 lines." → SNIPPETS.
  - Question: "Enumerate every call site of `execute_region_mapping` in the workspace. Return LOCATIONS with file:line; ≤ 10 entries." → LOCATIONS.
  - Question: "Name the function(s) in `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` (or sibling) that apply fuzzy-skin overlay to a region for `modifier_part`. Return LOCATIONS, ≤ 5 entries. No source." → LOCATIONS.
  - Question: "Run `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd region_overlap_stamps_only_in_cube_zband fuzzy_region_restricted_to_cube_and_painted_facets empty_modifier_volume_stamps_no_regions`. FACT pass/fail per test." → FACT.
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` :107-114 — RegionMapping responsibility. Delegate SUMMARY.
  - `docs/08_coordinate_system.md` — scaled integer units. Read directly.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — LOCATIONS dispatch.
- Verification:
  - `cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd` — all six tests GREEN.
  - `cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd` — must stay GREEN.
- Exit condition: All `benchy_4color_modifier_part_e2e_tdd` tests GREEN; regression suite green.

### Step 6: Regression sweep + clippy + check

- Task IDs:
  - `TASK-191`, `TASK-192a`
- Objective: Confirm no regressions and lint cleanliness.
- Precondition: Step 5 GREEN.
- Postcondition: All regression suites GREEN; clippy clean.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): any source file the lint pass demands (sticking to files-in-scope from earlier steps).
- Files explicitly out-of-bounds: macros, WIT, SDK, IR.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test threemf_transform_tdd && cargo test -p slicer-host --test gcode_emit_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd`. Return FACT pass/fail per file." → FACT.
  - Question: "Run `cargo clippy --workspace -- -D warnings`. FACT pass/fail with first warning if fail." → FACT.
  - Question: "Run `cargo check --workspace`. FACT pass/fail with first error if fail." → FACT.
- Context cost: `S`
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: All three FACTs GREEN.
- Exit condition: Clean workspace; no regressions.

### Step 7: Doc + deviation registration

- Task IDs:
  - `TASK-191`, `TASK-192a`
- Objective: Add the schema_version header annotation under IR 0 in `docs/02_ir_schemas.md`. Append TASK-191 and TASK-192a rows in `docs/07_implementation_status.md`. Register DEV-052 as `Closed — Packet 56b, 2026-MM-DD` in `docs/DEVIATION_LOG.md`. Add chronology entry in `docs/14_deviation_audit_history.md`.
- Precondition: Step 6 clean.
- Postcondition: Docs reflect packet outcome.
- Files allowed to read:
  - `docs/02_ir_schemas.md` — lines 62-244, 250, 506 (read narrow sections for the annotation precedent).
  - `docs/07_implementation_status.md` — delegate FACT for the append position.
  - `docs/DEVIATION_LOG.md` — delegate FACT for the next free DEV-### slot.
  - `docs/14_deviation_audit_history.md` — delegate FACT for the append position.
- Files allowed to edit (≤ 3 per dispatch; 4 files spread across two dispatches):
  - `docs/02_ir_schemas.md`
  - `docs/07_implementation_status.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
- Files explicitly out-of-bounds: all source.
- Expected sub-agent dispatches:
  - Question: "Add the schema_version header annotation under IR 0 `MeshIR` in `docs/02_ir_schemas.md` following the IR 2 precedent at line 250: `**Current schema_version: 1.1.0** (Bumped to 1.1.0 by packet 56b — populated \\`modifier_volumes\\` from \\`Metadata/model_settings.config\\`.)`. Return the resulting block as SNIPPETS, ≤ 5 lines." → SNIPPETS.
  - Question: "Append `[x] TASK-191` and `[x] TASK-192a` rows to `docs/07_implementation_status.md` immediately after the TASK-190 row registered by Packet 56, each naming packet `56b_threemf-modifier-part-ir-routing`. Return the resulting two lines verbatim. SNIPPETS." → SNIPPETS.
  - Question: "Confirm DEV-052 is the next free DEV-### slot in `docs/DEVIATION_LOG.md` (with DEV-050 and DEV-051 already closed by Packet 56). Return FACT yes/no + the existing DEV row count." → FACT.
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` line 250, 506 — patterns for the schema_version annotation.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'schema_version: 1\.1\.0.*packet 56b' docs/02_ir_schemas.md` → exit 0.
  - `rg -q '\[x\] TASK-191.*56b_threemf-modifier-part-ir-routing' docs/07_implementation_status.md` → exit 0.
  - `rg -q '\[x\] TASK-192a.*56b_threemf-modifier-part-ir-routing' docs/07_implementation_status.md` → exit 0.
  - `rg -c '^\| DEV-052.*Closed.*Packet 56b' docs/DEVIATION_LOG.md` → 1.
- Exit condition: All `rg` checks pass.

### Step 8: Packet acceptance ceremony

- Task IDs:
  - All packet TASK ids.
- Objective: Dispatch every pipe-suffixed AC command from `packet.spec.md` to a worker and record FACT pass/fail. If any criterion fails, return to the relevant step.
- Precondition: Steps 0-7 GREEN.
- Postcondition: All AC commands GREEN; `packet.spec.md` ready to flip to `status: implemented`.
- Files allowed to read: `packet.spec.md` (this packet).
- Files allowed to edit (≤ 3): `packet.spec.md` (status flip on success only).
- Files explicitly out-of-bounds: every source file.
- Expected sub-agent dispatches:
  - One dispatch per AC command, each returning FACT pass/fail.
- Context cost: `S`
- Authoritative docs: this packet's `packet.spec.md`.
- OrcaSlicer refs: none.
- Verification:
  - All AC commands return PASS.
- Exit condition: Status flippable to `implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| Step 0 | S | Pure dispatch (precondition + WIT gate). |
| Step 1 | S | New E2E test files; centroid math computed via FACT. |
| Step 2 | M | `resolve_object` branching + `ModifierVolume` construction + schema bump. Heaviest single-file edit. |
| Step 3 | S | Manifest gate (likely no-op). |
| Step 4 | S | Region-overlap TDD-RED. |
| Step 5 | M | Region-mapping overlap stamp + pipeline thread. |
| Step 6 | S | Regression + clippy + check sweep dispatch. |
| Step 7 | S | Doc + deviation registration. |
| Step 8 | S | Acceptance ceremony dispatches. |

Aggregate: **M** (2 M + 7 S).

## Packet Completion Gate

- All 9 steps complete.
- Every step exit condition met.
- Packet acceptance criteria GREEN (each verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-191 and TASK-192a via worker dispatch.
- DEV-052 registered in `docs/DEVIATION_LOG.md` and chronology in `docs/14_deviation_audit_history.md`.
- DEV-050, DEV-051 must NOT be touched by this packet (already closed by Packet 56).
- WIT-mirror gate confirmed clean (Step 0).
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md` (Step 8).
- Confirm packet-level verification commands are GREEN (Steps 6, 8).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson.
- This packet does NOT run `cargo test --workspace` at closure. The targeted regression suites in Step 6 cover the producer + `modifier_part` consumer surface end-to-end.
