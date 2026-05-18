# Implementation Plan: bridge-detector-orca-parity-fixes

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-168 (TASK-167 is reopened in Step 1 as the audit-trail anchor).
- TDD-driven: Step 7 rewrites tests to FAIL against the broken state from packet 36; Steps 3–6 turn them green.
- Each step honors the context-discipline preamble. Stop reading at 60% context; hand off at 85%.

## Steps

### Step 0: FACT-confirm slicer-core polygon API and host bindgen accessors

- Task IDs:
  - `TASK-168`
- Objective: read-only discovery — confirm exact signatures of `slicer_core::polygon_ops::{intersection, offset, difference, union}`, the available `OffsetJoinType` variants, the existing clipper2 validity primitive (for `validate_polygon_simplicity`), the `FacetClass` variants in `mesh_analysis.rs`, and the presence of `fn bridge_areas` / `fn bridge_orientation_deg` accessor impls on the WIT resource trait in `wit_host.rs`.
- Precondition: Step 0 not yet run.
- Postcondition: five FACTs recorded.
- Files allowed to read: none directly (delegate dispatches only).
- Files allowed to edit (≤ 3): none.
- Expected sub-agent dispatches:
  - "Return signatures and file:line of `intersection`, `offset`, `difference`, `union` in `crates/slicer-core/src/polygon_ops.rs`. Also list every `OffsetJoinType` variant. FACT only."
  - "Identify the clipper2 polygon-validity primitive available in `crates/slicer-core/src/polygon_ops.rs` (or its imports). FACT one-line."
  - "List `FacetClass` enum variants in `crates/slicer-host/src/mesh_analysis.rs` and the function that classifies each facet. FACT only."
  - "In `crates/slicer-host/src/wit_host.rs` lines 2900–3010, do `fn bridge_areas` and `fn bridge_orientation_deg` accessor methods exist on the `HostSliceRegionView` resource trait? Return FACT yes/no with file:line."
  - "In `crates/slicer-host/tests/wit_boundary_tdd.rs`, list every literal `SliceRegionData { ... }` constructor (line numbers only). FACT — needed to verify nothing breaks when we touch the host record."
- Context cost: `S`.
- Authoritative docs: `docs/13_slicer_helpers_crate.md`.
- OrcaSlicer refs: none.
- Verification: the five FACTs.
- Exit condition: API surface known; `OffsetJoinType::Miter` availability known; `FacetClass` variants known; bindgen accessor presence known.

### Step 1: Reopen DEV-035 / DEV-036 / TASK-167; supersede packet 36; register slicer-helpers boundary deviation

- Task IDs:
  - `TASK-168`
- Objective: flip closure markers and register the spec amendment. No code changes; only doc/spec edits.
  - In `docs/DEVIATION_LOG.md`: flip DEV-035 and DEV-036 from `Closed` to `Open`. Append to each rationale: "Closure rationale was incorrect — algorithms were heuristic stubs (bbox/centroid). Reopened by packet 36-rev1 / TASK-168." Register one new `DEV-NNN` (next free ID) for "polygon ops live in `slicer-core::polygon_ops`, not `slicer-helpers`; packet 36 design.md was incorrect about the boundary; backed by clipper2-rust per existing DEV-015."
  - In `docs/07_implementation_status.md`: flip TASK-167 row from `[x]` to `[ ]` with reopen note "Reopened 2026-05-05 by TASK-168 (packet 36-rev1) — packet 36 closure markers were premature; algorithms were heuristic stubs." Add new `TASK-168` row pointing at this packet.
  - In `.ralph/specs/36_bridge-detector-orca-parity/packet.spec.md`: flip frontmatter `status: implemented` → `status: superseded`. Add a one-line frontmatter `superseded_by: 36-rev1_bridge-detector-orca-parity-fixes` and a "Superseded" callout near the top of the body.
- Precondition: Step 0 complete.
- Postcondition: DEV-035 and DEV-036 read `Open`; TASK-167 reads `[ ]`; TASK-168 row exists; packet 36 reads `superseded`; new DEV-### row exists for the slicer-helpers boundary amendment.
- Files allowed to read:
  - `docs/DEVIATION_LOG.md` — read range covering DEV-035, DEV-036, and the highest existing DEV-### only.
  - `docs/07_implementation_status.md` — read range covering TASK-167 and the highest existing TASK-### only.
  - `.ralph/specs/36_bridge-detector-orca-parity/packet.spec.md` — read frontmatter + the first body paragraph only.
- Files allowed to edit (≤ 3 per pass; 4 total — split across two passes if needed):
  - `docs/DEVIATION_LOG.md`
  - `docs/07_implementation_status.md`
  - `.ralph/specs/36_bridge-detector-orca-parity/packet.spec.md`
- Expected sub-agent dispatches:
  - "Find the highest DEV-### currently in `docs/DEVIATION_LOG.md`. FACT one number."
  - "After the edits in Step 1 land, confirm DEV-035 and DEV-036 `Status` columns read `Open` and the new DEV-### row exists. FACT one-line quote per row."
  - "After the edits in Step 1 land, confirm TASK-167 reads `[ ]` and TASK-168 row exists. FACT one-line quote per row."
- Context cost: `S`.
- Authoritative docs: `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`.
- OrcaSlicer refs: none.
- Verification: the post-edit FACT dispatches above.
- Exit condition: closure markers reflect the actual implementation state.

### Step 2: Add `CURRENT_*_SCHEMA_VERSION` constants and `validate_polygon_simplicity`

- Task IDs:
  - `TASK-168`
- Objective:
  - In `crates/slicer-ir/src/slice_ir.rs`: expose `pub const CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION: SemVer = SemVer { major: 1, minor: 1, patch: 0 };` and `pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer { major: 1, minor: 2, patch: 0 };`. Update production literal constructors of `SurfaceClassificationIR` and `SliceIR` to populate `schema_version` from these constants. (Test-fixture constructors may stay literal; the AC-10 test will verify the constants are the source.)
  - In `crates/slicer-core/src/polygon_ops.rs`: add `pub fn validate_polygon_simplicity(poly: &ExPolygon) -> Result<(), PolygonSimplicityError>` and `#[derive(Debug)] pub struct PolygonSimplicityError { pub contour_indices: Vec<usize> }`. Wraps the clipper2 validity primitive identified in Step 0. Returns `Ok(())` for simple polygons; `Err(...)` listing failing contour indices when invalid.
- Precondition: Step 1 complete; Step 0 FACTs identified the clipper2 validity primitive.
- Postcondition: workspace builds; `crates/slicer-ir` exposes the two constants; `slicer_core::polygon_ops::validate_polygon_simplicity` is callable.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` — full read OK (file is small).
  - `crates/slicer-core/src/polygon_ops.rs` — public API only via symbol search; range-read around the validity primitive identified in Step 0.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-core/src/polygon_ops.rs`
  - any production-path source file that constructs `SurfaceClassificationIR { schema_version: SemVer { … }, … }` or `SliceIR { schema_version: SemVer { … }, … }` and needs to switch to the constant (delegate "find every literal `SemVer { major: 1,` in production code (not `tests/`)" first; mechanical edits).
- Expected sub-agent dispatches:
  - "Find every literal `SemVer { major: 1,` constructor across `crates/` outside `tests/` directories. Return LOCATIONS."
  - "Run `cargo build --workspace`. Return FACT pass/fail."
  - "Run `cargo test -p slicer-core polygon_ops::validate_polygon_simplicity`. Return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs: `docs/02_ir_schemas.md` (additive-minor rule, unchanged in this packet).
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace`
  - `cargo test -p slicer-ir bridge_detector_schema_versions_are_constant_sourced` (will still fail; the test rewrite lands in Step 7 — but the constants must be present for the test to compile against).
- Exit condition: build green; constants exposed; helper callable.

### Step 3: Mesh adjacency rewrite + `MeshAnalysisConfig` rename

- Task IDs:
  - `TASK-168`
- Objective:
  - In `crates/slicer-host/src/mesh_analysis.rs`:
    - Rename `MeshAnalysisConfig.min_anchor_width_mm` → `anchor_width_mm`.
    - Add `overhang_threshold_deg: f32` field to `MeshAnalysisConfig`; remove the separate parameter from `execute_mesh_analysis_with`'s signature (callers pass it via the struct). Update all in-workspace call sites.
    - Rewrite `compute_bridge_metrics` cluster seed: use down-facing / overhang-classified facets (per the `FacetClass` variant identified in Step 0), not `FacetClass::TopSurface`. BFS neighbor predicate also uses the new variant.
    - Rewrite `compute_xy_footprint`: union of facet XY projections via `slicer_core::polygon_ops::union`. One `ExPolygon` per disconnected contour group.
    - Rewrite `compute_anchor_width_mm`: shortest perpendicular run length of contiguous anchor edges from the half-edge map. Remove `#[allow(dead_code)]` on the anchor-edge structures.
    - Rewrite `compute_bridge_direction_deg`: orientation of the longest anchor-edge run.
    - Rewrite all four function doc comments to remove "Orca default" attribution; document as project policy with one-line rationale referencing 12-rev1's architectural divergence.
    - Add a one-line `// 12-rev1 architectural divergence: see docs/04_host_scheduler.md PrePass + Per-Layer Execution sections` comment near `compute_bridge_direction_deg`.
- Precondition: Step 2 complete.
- Postcondition: mesh-analysis-level tests in `bridge_detector_tdd.rs` PASS for all rotated-bridge cases (AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, NEG-3) — but only after Step 7 rewrites the tests; until then, the new tests do not exist yet, so this step's exit condition is workspace builds clean and all *existing* mesh-analysis tests still pass.
- Files allowed to read:
  - `crates/slicer-host/src/mesh_analysis.rs` — full file (~700 lines after packet 36; one-time read at start of step).
  - `crates/slicer-core/src/polygon_ops.rs` — public API only.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/mesh_analysis.rs`
  - `crates/slicer-host/src/lib.rs` (re-export `MeshAnalysisConfig` if public — only if the rename breaks an external re-export).
  - any in-workspace caller of `execute_mesh_analysis_with` whose signature changes (delegate "find every call to `execute_mesh_analysis_with` and `MeshAnalysisConfig::default()`" first).
- Expected sub-agent dispatches:
  - "Find every call to `execute_mesh_analysis_with` across `crates/`. Return LOCATIONS."
  - "Find every constructor of `MeshAnalysisConfig` (literal or `Default::default()`). Return LOCATIONS."
  - "Run `cargo build --workspace`. Return FACT pass/fail."
  - "Run `cargo test -p slicer-host --test bridge_detector_tdd` (the existing packet 36 tests must still compile; some may now fail because the algorithm changed — that is expected and fixed in Step 7). Return FACT pass/fail per test."
- Context cost: `M`.
- Authoritative docs: `docs/04_host_scheduler.md` (cited rationale, no new content).
- OrcaSlicer refs: none new.
- Verification:
  - `cargo build --workspace`.
  - `cargo test -p slicer-host --test bridge_detector_tdd` (informational; expect some test failures pending Step 7).
- Exit condition: workspace builds; `MeshAnalysisConfig` renamed; cluster seed flipped; all four functions rewritten with correct algorithms; all in-workspace callers updated.

### Step 4: Slice-time fixes — `OffsetJoinType::Miter` + sanity guard

- Task IDs:
  - `TASK-168`
- Objective:
  - In `crates/slicer-host/src/layer_slice.rs::assemble_bridge_areas`:
    - Replace `OffsetJoinType::Square` with `OffsetJoinType::Miter` (or `OffsetJoinType::Round` if `Miter` is not available — Step 0 FACT decides). Add a one-line code comment explaining the choice.
    - Add defensive guard at the top of the per-bridge loop: `if !br.expansion_margin_mm.is_finite() || br.expansion_margin_mm < 0.0 { continue; }`.
- Precondition: Step 3 complete.
- Postcondition: existing AC-4 / AC-7 / NEG-1 / NEG-3 tests still produce some output (PASS or FAIL — the rewrite happens in Step 7; this step only changes the code).
- Files allowed to read:
  - `crates/slicer-host/src/layer_slice.rs` — range `240-320` (the `assemble_bridge_areas` body).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/layer_slice.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`. Return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `cargo build --workspace`.
- Exit condition: build green; offset join changed; sanity guard present.

### Step 5: `rectilinear-infill` fixes — real set difference + branch fix + WASM rebuild

- Task IDs:
  - `TASK-168`
- Objective:
  - In `modules/core-modules/rectilinear-infill/src/lib.rs`:
    - Replace `partition_expoly_by_bridges` body with: `bridge_parts = intersection(&[expoly.clone()], bridge_areas)`; `non_bridge_parts = difference(&[expoly.clone()], bridge_areas)`. Both via `slicer_core::polygon_ops`. Remove the inline "geometry ops not yet available" comment.
    - Delete now-unused private helpers (`polygon_centroid`, `point_in_expoly_union`, `point_in_polygon`) if they have no other call site.
    - Fix the `is_bridge && bridge_areas.is_empty()` branch: do NOT emit any `BridgeInfill` paths in this state. Treat as `bridge_areas` empty across the board. Add a one-line code comment: `// is_bridge implies non-empty bridge_areas after assemble_bridge_areas; defensive guard for the inconsistent state.`.
  - Re-run `./modules/core-modules/build-core-modules.sh` after edits.
- Precondition: Step 4 complete.
- Postcondition: WASM build succeeds; module-level tests still compile (rewrite in Step 7).
- Files allowed to read:
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — full file.
  - `crates/slicer-core/src/polygon_ops.rs` — public API only.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/rectilinear-infill/src/lib.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`. Return FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`. Return FACT pass/fail with the failing module name on failure."
- Context cost: `M`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: build script + cargo build.
- Exit condition: build green; WASM rebuild green; centroid heuristic gone; branch fix in place.

### Step 6: Verify host bindgen accessors (or add)

- Task IDs:
  - `TASK-168`
- Objective:
  - Based on Step 0 FACT: if `fn bridge_areas` and `fn bridge_orientation_deg` accessor impls exist on the WIT resource trait in `crates/slicer-host/src/wit_host.rs:2900-3010`, this step is a one-line confirmation. If missing, add them: `fn bridge_areas(&mut self, self_: Resource<SliceRegionData>) -> Vec<ExPolygon> { ... }` and `fn bridge_orientation_deg(&mut self, self_: Resource<SliceRegionData>) -> f32 { ... }`, sourcing from `SliceRegionData.bridge_areas` and `SliceRegionData.bridge_orientation_deg` respectively.
- Precondition: Step 5 complete.
- Postcondition: bindgen-required trait impls present; `cargo build --workspace` green.
- Files allowed to read:
  - `crates/slicer-host/src/wit_host.rs` — range `2900-3010` only.
  - `wit/deps/ir-types.wit` — the `slice-region-view` resource block (already known: lines `56-79` per spec-review).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/wit_host.rs`
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`. Return FACT pass/fail." (if accessors were missing, this is the falsifying check.)
- Context cost: `S`.
- Authoritative docs: `docs/03_wit_and_manifest.md` § "WIT/Type Changes Checklist".
- OrcaSlicer refs: none.
- Verification: `cargo build --workspace`.
- Exit condition: build green; both accessors present.

### Step 7: Rewrite tests at all three levels (TDD against the broken state)

- Task IDs:
  - `TASK-168`
- Objective:
  - In `crates/slicer-host/tests/bridge_detector_tdd.rs`:
    - Add a shared `make_rotated_bridge_mesh(width_mm: f32, length_mm: f32, rotation_deg: f32, with_top_surfaces: bool) -> (MeshIR, ...)` helper at the top.
    - Add a shared `make_topfacing_only_mesh()` helper for NEG-3.
    - Add a shared `make_vshape_sharp_anchor_footprint(interior_angle_deg: f32) -> Vec<ExPolygon>` helper for NEG-1.
    - Replace `bridge_detector_schema_versions_are_correct` (this test stays in `crates/slicer-ir/tests/ir_tests.rs` — keep it deleted from the host tests; do not duplicate).
    - Replace `sharp_anchor_offset_does_not_self_intersect` with `vshape_sharp_anchor_pipeline_produces_simple_polygons` (NEG-1).
    - Restructure `slice_assembles_expanded_bridge_polygons` → rename to `expansion_margin_grows_polygon_observably` (AC-7); fixture: `xy_footprint = [0,0]–[20,5]`, `infill_areas = [-3,-3]–[23,8]`; assert each output bbox grows by ≥ 1.5 mm in both X and Y.
    - Strengthen `valid_bridge_passes_min_length_filter` to also assert `anchor_width_mm` matches the perpendicular run length within 0.1 mm (AC-1's anchor part).
    - Add `bridge_cluster_seeded_from_downfacing_facets_only` (AC-1 cluster part).
    - Add `anchor_width_from_anchor_edge_run_not_bbox` (AC-2).
    - Add `xy_footprint_is_facet_projection_not_aabb` (AC-3).
    - Add `bridge_direction_follows_anchor_edge_orientation` (AC-4).
    - Add `rotated_short_bridge_fails_min_length_filter` (AC-5).
    - Rename existing `narrow_anchor_fails_anchor_width_filter` to `rotated_narrow_anchor_fails_anchor_width_filter` (AC-6); change fixture to 2 mm × 40 mm needle rotated 45°; add positive-detection precondition (`bridge_regions.len() >= 1` before the validity assertion).
    - Add `topsurface_only_mesh_produces_no_bridge_regions` (NEG-3).
    - Delete `non_bridge_region_has_empty_bridge_areas` and `invalid_bridge_excluded_from_slice_areas` if they are now redundant with the new AC-1 / NEG-3 / AC-5 trio (or keep them — they were OK in the spec review). Decide based on overlap.
    - Clean stale TDD scaffolding comments throughout (`unreachable_unchecked`, `todo!`, "RED state", "stub", "first_valid binding above is dead").
  - In `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs`:
    - Add `straddling_expoly_partitioned_via_set_difference` (AC-8).
    - Add `bridge_paths_use_bridge_orientation_not_sparse_alternation` (AC-9).
    - Add `empty_bridge_areas_emits_no_bridge_infill_even_when_is_bridge_true` (NEG-2).
    - Keep `bridge_areas_emit_bridge_infill_at_oriented_angle` (the one strong test from packet 36).
  - In `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`:
    - Rename `benchy_gcode_contains_bridge_infill_evidence` → `benchy_gcode_contains_exact_bridge_infill_marker`. Change `gcode.lines().any(|l| l.contains(";TYPE:Bridge"))` → `gcode.lines().any(|l| l.trim() == ";TYPE:Bridge infill")`.
  - In `crates/slicer-ir/tests/ir_tests.rs`:
    - Replace `bridge_detector_schema_versions_are_correct` with `bridge_detector_schema_versions_are_constant_sourced`. Body asserts: (a) `slicer_ir::CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION == SemVer { 1, 1, 0 }`, (b) `slicer_ir::CURRENT_SLICE_IR_SCHEMA_VERSION == SemVer { 1, 2, 0 }`, (c) a freshly-default-constructed `SurfaceClassificationIR` (or a production constructor) has `schema_version == CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION`, (d) same for `SliceIR`. The (c)/(d) assertions ensure the constants are the source, not duplicated literals.
- Precondition: Step 6 complete.
- Postcondition: every AC test in `packet.spec.md` PASSES.
- Files allowed to read:
  - `crates/slicer-host/tests/bridge_detector_tdd.rs` — full file.
  - `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` — full file.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — range covering the bridge test only.
  - `crates/slicer-ir/tests/ir_tests.rs` — range covering the schema-version test only.
- Files allowed to edit (≤ 3 per pass; multiple passes):
  - Pass 1: `crates/slicer-host/tests/bridge_detector_tdd.rs` (large rewrite).
  - Pass 2: `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs`, `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`, `crates/slicer-ir/tests/ir_tests.rs` (3 files).
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test bridge_detector_tdd`. Return FACT pass/fail per test."
  - "Run `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd`. Return FACT pass/fail per test."
  - "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_exact_bridge_infill_marker`. Return FACT pass/fail."
  - "Run `cargo test -p slicer-ir bridge_detector_schema_versions_are_constant_sourced`. Return FACT pass/fail."
- Context cost: `M`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: every targeted cargo test above PASSES.
- Exit condition: every AC and NEG test in `packet.spec.md` PASSES.

### Step 8: Doc updates and acceptance ceremony

- Task IDs:
  - `TASK-168`
- Objective:
  - In `docs/02_ir_schemas.md`:
    - Bump `SliceIR` schema_version banner from `1.1.0` to `1.2.0` and attribute to packet 36.
    - Add `SurfaceClassificationIR` schema_version banner `1.1.0` (currently undocumented), attributed to packet 36.
    - Update the `BridgeRegion` section to list `anchor_width_mm`, `bridge_length_mm`, `expansion_margin_mm`, `is_valid`, `xy_footprint`.
    - Update the `SlicedRegion` section to list `bridge_areas` and `bridge_orientation_deg`.
    - Remove the stale "currently initialized empty in `crates/slicer-host/src/mesh_analysis.rs:213`; production runs always see `false` until packet 36 populates" comment near `is_bridge` (lines ~519-525). Replace with a one-line "populated by packet 36 / 36-rev1".
  - In `docs/13_slicer_helpers_crate.md`:
    - Remove the polygon/geometry utility claim from the description.
    - Add a one-line cross-reference: "Polygon ops (intersection, offset, difference, union, validate_polygon_simplicity) live in `slicer-core::polygon_ops`. See DEV-### registered by packet 36-rev1."
  - Run the full acceptance gate.
- Precondition: Step 7 complete.
- Postcondition: every AC verification command in `packet.spec.md` PASSES; docs updated; the new DEV-### in `DEVIATION_LOG.md` (registered in Step 1) cross-references `docs/13`.
- Files allowed to read: none directly (dispatch only).
- Files allowed to edit (≤ 3):
  - `docs/02_ir_schemas.md`
  - `docs/13_slicer_helpers_crate.md`
- Expected sub-agent dispatches:
  - "In `docs/02_ir_schemas.md`, confirm `SliceIR` banner reads `1.2.0`, `SurfaceClassificationIR` banner reads `1.1.0`, `BridgeRegion` section lists the 5 new fields, `SlicedRegion` section lists the 2 new fields, and the stale comment is removed. FACT one-line per item."
  - "In `docs/13_slicer_helpers_crate.md`, confirm the polygon-utility claim is removed and the cross-reference to `slicer-core::polygon_ops` is present. FACT one-line."
  - "Run `cargo test --workspace`. Return FACT pass/fail with failing test list (max 20 lines on fail)."
  - "Run `cargo clippy --workspace -- -D warnings`. Return FACT pass/fail."
  - "Run `./modules/core-modules/build-core-modules.sh`. Return FACT pass/fail."
- Context cost: `S`.
- Authoritative docs: `docs/02_ir_schemas.md`, `docs/13_slicer_helpers_crate.md`.
- OrcaSlicer refs: none.
- Verification: every AC command from `packet.spec.md`.
- Exit condition: every AC PASSES; docs updated; workspace tests + clippy + WASM rebuild green; packet ready to move to `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Five FACT dispatches; zero edits. |
| Step 1 | S | Doc-only edits; closure flips + supersede marker. |
| Step 2 | S | IR constants + helper; mostly mechanical. |
| Step 3 | M | Mesh-adjacency rewrite — single file, large internal change. |
| Step 4 | S | Two-line code change in `layer_slice.rs`. |
| Step 5 | M | `partition_expoly_by_bridges` rewrite + branch fix + WASM rebuild. |
| Step 6 | S | Verify or add two accessor methods. |
| Step 7 | M | Test rewrites at three levels; multiple files; rotated fixtures. |
| Step 8 | S | Doc updates + final acceptance gate. |

Aggregate: `M`. No single step is `L`. If Step 3 trends to `L` during implementation (because the half-edge anchor-edge walk is more involved than expected), split into "Step 3a: cluster seed + xy_footprint" and "Step 3b: anchor_width + bridge_direction" before continuing.

## Packet Completion Gate

- All steps complete.
- Every AC verification command in `packet.spec.md` PASSES (11 positive + 3 negative = 14 commands).
- `cargo test --workspace` PASSES.
- `cargo clippy --workspace -- -D warnings` PASSES.
- `./modules/core-modules/build-core-modules.sh` PASSES.
- `docs/07_implementation_status.md` carries TASK-168 (closed) and TASK-167 (closed-again, with reopen-then-close audit trail).
- `docs/02_ir_schemas.md` documents the schema bumps + new fields.
- `docs/13_slicer_helpers_crate.md` no longer claims polygon utilities.
- `docs/DEVIATION_LOG.md` carries: DEV-035 + DEV-036 closed-with-correct-rationale (i.e., closed by **this** packet, citing the rewritten algorithms); the new DEV-### registered in Step 1 for the `slicer-helpers` boundary amendment.
- `.ralph/specs/36_bridge-detector-orca-parity/packet.spec.md` carries `status: superseded`.
- `packet.spec.md` for this packet ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, and `./modules/core-modules/build-core-modules.sh` PASS.
- Confirm DEV-035 and DEV-036 closure rationale on the **second** close cites the algorithmic rewrites delivered by this packet (not the bbox/centroid stubs).
- Record any remaining packet-local risk:
  - Slice-time `_anchor_regions` refinement à la Orca's `detect_angle` is not delivered (intentional; out of scope).
  - `bridge_speed` / `bridge_flow_ratio` thermal/cooling overrides not wired (out of scope).
  - Multi-cluster `bridge_orientation_deg` algorithm unchanged (out of scope).
- Confirm implementer's peak context usage stayed under 70%. If not, log the breach and identify which step ran hot.
