# Requirements: bridge-detector-orca-parity-fixes

## Packet Metadata

- Grouped task IDs:
  - `TASK-168` (NEW — to be added to `docs/07_implementation_status.md`)
  - `TASK-167` (REOPENED — original packet 36 task; reopen note points at TASK-168)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`
- Supersedes: `36_bridge-detector-orca-parity`
- Reopens deviations: `DEV-035`, `DEV-036`

## Problem Statement

Packet 36 (`bridge-detector-orca-parity`) was closed on 2026-05-05 with all of its acceptance-criteria tests passing, the WASM rebuild succeeding, and the workspace gates green. A subsequent critical spec review revealed that the green CI was masking heuristic implementations standing in for the contracted analysis:

1. **Wrong cluster seed.** `compute_bridge_metrics` in `crates/slicer-host/src/mesh_analysis.rs` clusters facets whose `FacetClass == TopSurface` and walks neighbors only when the neighbor is also `TopSurface`. Bridges are unsupported overhangs (down-facing). Top surfaces are by definition not bridges. The cluster set is wrong from the start; the test fixtures happen to produce flagged bridges only because the geometry is hand-built to satisfy this inverted contract.

2. **Anchor-edge logic is dead code.** The `build_half_edge_map` function constructs a real half-edge graph and identifies anchor edges (edges shared with a non-bridge neighbor). The structure carrying that information is annotated `#[allow(dead_code)]`. The actual `anchor_width_mm` returned to callers is the AABB side perpendicular to `bridge_direction_deg`. For axis-aligned fixtures this happens to equal the true anchor-edge run; for any rotated bridge it does not.

3. **`xy_footprint` is the AABB.** `compute_xy_footprint` returns a four-point rectangle bounding the cluster, not the union of facet XY projections. A 5 mm × 20 mm bridge rotated 30° produces a footprint of ≈ 240 mm² instead of the true 100 mm². Slice-time `assemble_bridge_areas` then expands and intersects this oversized polygon, painting BridgeInfill far outside the real bridge.

4. **`bridge_direction_deg` is hardcoded to 0° or 90°.** `compute_bridge_direction_deg` returns `if dx >= dy { 0.0 } else { 90.0 }` based on the AABB aspect ratio. OrcaSlicer's `BridgeDetector::detect_angle` sweeps candidate angles and scores line coverage. None of that is here. For rotated bridges the printed bridge filament runs at 0° or 90° instead of along the unsupported gap.

5. **Centroid heuristic substituted for set difference.** `partition_expoly_by_bridges` in `modules/core-modules/rectilinear-infill/src/lib.rs:298-331` admits in an inline comment: *"Proper polygon clipping would be more precise but requires slicer-helpers geometry ops not yet available."* It assigns the entire infill expoly to **either** `bridge_parts` **or** `non_bridge_parts` based on centroid containment. Real partial coverage is impossible; bridge-area boundaries are paved over. Branch bug: when `is_bridge && bridge_areas.is_empty()`, the whole expoly is forced into bridge_parts but `has_bridge` was computed from `bridge_areas` only, so paths get `role = BridgeInfill` at the layer-alternating sparse angle — a contradictory state.

6. **AC tests do not enforce AC text.** Per the test-quality audit:
   - `bridge_detector_schema_versions_are_correct` constructs a `SemVer { 1, 1, 0 }` literal and asserts it equals `(1, 1, 0)`. The test would pass with any schema version everywhere else in the codebase. Pure tautology.
   - `sharp_anchor_offset_does_not_self_intersect` uses a 1 × 20 mm rectangle (not a V-shape), calls `slicer_core::polygon_ops::offset` directly (not the bridge pipeline), and never checks self-intersection. The AC-neg-2 contract is unenforced.
   - `slice_assembles_expanded_bridge_polygons` sets `infill_areas = xy_footprint`, which forces the +1.5 mm Minkowski expansion to be re-clipped to the original footprint. The AC-4 promise that the expansion is observable is structurally untestable in this fixture.
   - `valid_bridge_passes_min_length_filter` does not assert `anchor_width_mm` matches the perpendicular run, even though AC-1 requires it.
   - `benchy_gcode_contains_bridge_infill_evidence` substring is `";TYPE:Bridge"` not `";TYPE:Bridge infill"` as the AC text and assertion message claim.

7. **"Orca default" attribution is fictional.** `mesh_analysis.rs:27-48` claims `min_bridge_length_mm = 10.0` and `min_anchor_width_mm = 0.5` are Orca defaults. OrcaSlicer's `BridgeDetector` has no such fixed defaults: there is no `min_bridge_length` config (`PrintConfig.cpp` grep returns zero), and anchor handling uses dynamic `_anchor_regions` (intersection with lower slices) and `spacing` derived from extrusion width. Only `expansion_margin_mm = 1.0` matches Orca's `BRIDGE_INFILL_MARGIN`.

8. **`MeshAnalysisConfig` field names deviate.** Spec/design require `anchor_width_mm` and `overhang_threshold_deg` as struct fields. Implementation has `min_anchor_width_mm` (renamed) and keeps `overhang_threshold_deg` as a separate function parameter.

9. **`slicer-helpers` mandate violated and stale.** `design.md`, `requirements.md`, and `implementation-plan.md` for packet 36 mandate `slicer-helpers` for polygon ops. `slicer-helpers/src/` exposes only `decimate`, `repair`, `import_step`, `merge_step_meshes` — no polygon offset, no Minkowski, no intersect, no difference. The actual ops live in `slicer_core::polygon_ops`. `docs/13_slicer_helpers_crate.md` describes the crate as "polygon/geometry utilities" — also stale. The spec was wrong; the spec needs amending.

10. **`OffsetJoinType::Square` deviates from `design.md:124`** which specifies "Clipper-style `MitterLimit`/`RoundJoin` semantics with a small mitter limit."

11. **`docs/02_ir_schemas.md` was not updated**: SliceIR banner still reads `1.1.0`, SurfaceClassificationIR schema bump is undocumented, the new fields on `BridgeRegion` and `SlicedRegion` are absent, and the stale "until packet 36 populates" comment near `is_bridge` (lines 519-525) was not removed.

12. **DEV-035 and DEV-036 were closed on incorrect rationale.** DEV-035 closure cites `assemble_bridge_areas` (which is real) but the consumer in `rectilinear-infill` does not use real polygon clipping. DEV-036 closure cites `execute_mesh_analysis_with` populating `bridge_regions` (which is true) but the populated values are bbox heuristics, not real adjacency analysis. Both deviations need to be reopened.

13. **TASK-167 closure misattributed.** `packet.spec.md` declares `task_ids: [TASK-166]`, but TASK-166 is the config-resolution prerequisite closed by packet 35a; the actual packet 36 task is TASK-167. This is a packet-quality defect that propagates wrong audit trails.

14. **`task-map.md` missing.** Required by the spec-packet-generator template.

This packet rewrites the algorithmic pieces, replaces the broken tests, adds rotated-bridge fixtures, fixes documentation, and reopens the closure markers.

## Architectural Constraint (cited from packet 12-rev1)

Packet 12-rev1 documented the architectural reason this project cannot mirror OrcaSlicer's bridge handling literally:

> `docs/04_host_scheduler.md` reserves prepass slicing for coarser support layers; this packet must not add a new slicing pass. `docs/04_host_scheduler.md §Per-Layer Execution` runs layers via `par_iter`; no synchronization between layer N and N±1 may be introduced. Use `PrePass::MeshAnalysis` output as the classification source.

Orca's `BridgeDetector::detect_angle` requires `lower_slices: const ExPolygons&` to compute `_anchor_regions = intersection_ex(grown_bridge, union(lower_slices))`. Without per-layer access to the prior layer's filled regions, `_anchor_regions` cannot be built. Therefore this packet does **not** attempt to port `detect_angle`. The honest project-policy analog is to derive the bridge orientation from the 3D anchor-edge orientation at PrePass over MeshIR, where adjacency information is available without per-layer synchronization.

## In Scope

- **Mesh adjacency rewrite** in `crates/slicer-host/src/mesh_analysis.rs`:
  - Cluster seed: down-facing facets (use the existing overhang classification used by `FacetClass`).
  - `compute_anchor_width_mm`: shortest perpendicular run length of contiguous anchor edges from the half-edge map (replace the bbox approximation; remove `#[allow(dead_code)]` on the anchor-edge structures).
  - `compute_xy_footprint`: union of facet XY projections (clipper2 union of triangle 2D footprints; one `ExPolygon` per cluster, possibly multi-contour). Replace the AABB four-point rectangle.
  - `compute_bridge_direction_deg`: orientation of the longest anchor-edge run (3D analog of `detect_angle`). Replace the `if dx >= dy` bbox-aspect heuristic.
- **`MeshAnalysisConfig` rename**:
  - `min_anchor_width_mm` → `anchor_width_mm` (matches packet 36 spec contract).
  - Consolidate `overhang_threshold_deg` into the struct (currently a separate function parameter on `execute_mesh_analysis_with`).
  - Drop "Orca default" doc-comment attribution; document as project policy with rationale referencing packet 12-rev1's architectural divergence.
- **Slice-time fixes** in `crates/slicer-host/src/layer_slice.rs::assemble_bridge_areas`:
  - `OffsetJoinType::Square` → `OffsetJoinType::Miter` (per packet 36 design.md:124).
  - Defensive guard: skip bridge contribution when `expansion_margin_mm < 0.0` or `expansion_margin_mm.is_nan()`.
- **`rectilinear-infill` fixes** in `modules/core-modules/rectilinear-infill/src/lib.rs`:
  - Replace `partition_expoly_by_bridges` centroid heuristic with `slicer_core::polygon_ops::{intersection, difference}` for real set-difference. Remove the inline "geometry ops not yet available" comment.
  - Fix the `is_bridge && bridge_areas.is_empty()` branch: do NOT emit `BridgeInfill` paths in this state. The current code emits `role = BridgeInfill` at the layer-alternating sparse angle.
- **`crates/slicer-ir/src/slice_ir.rs`**:
  - `pub const CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION: SemVer = SemVer { major: 1, minor: 1, patch: 0 };`
  - `pub const CURRENT_SLICE_IR_SCHEMA_VERSION: SemVer = SemVer { major: 1, minor: 2, patch: 0 };`
  - Update `Default` impls (or all production literal constructors) to populate `schema_version` from these constants.
- **`crates/slicer-core/src/polygon_ops.rs`**:
  - Add `pub fn validate_polygon_simplicity(poly: &ExPolygon) -> Result<(), PolygonSimplicityError>`. Wraps the existing clipper2 validity check; returns the failing contour index list when invalid. Used by NEG-1.
- **Spec amendment**: `docs/13_slicer_helpers_crate.md` updated to remove the polygon-utility claim and cross-reference `slicer-core::polygon_ops`. Register one new DEV-### in `docs/DEVIATION_LOG.md` documenting that polygon ops live in `slicer-core::polygon_ops` (rationale: packet 36 design.md was incorrect about the boundary; backed by clipper2-rust per existing DEV-015).
- **Closure reversal**:
  - `docs/DEVIATION_LOG.md`: DEV-035 and DEV-036 flipped from `Closed` back to `Open` with the rationale "Closure rationale was incorrect — algorithms were heuristic stubs (bbox/centroid). Reopened by packet 36-rev1 / TASK-168."
  - `docs/07_implementation_status.md`: TASK-167 row flipped `[x]` → `[ ]` with "Reopened by TASK-168 (packet 36-rev1)" note. Add new TASK-168 row.
- **`docs/02_ir_schemas.md`** updated:
  - `SliceIR` schema_version banner bumped to `1.2.0` (was `1.1.0`).
  - `SurfaceClassificationIR` schema_version banner bumped to `1.1.0` (currently undocumented).
  - `BridgeRegion` section lists the five new fields: `anchor_width_mm`, `bridge_length_mm`, `expansion_margin_mm`, `is_valid`, `xy_footprint`.
  - `SlicedRegion` section lists the two new fields: `bridge_areas`, `bridge_orientation_deg`.
  - Stale comment near `is_bridge` (lines 519-525) removed.
- **Test rewrites and additions** — see Acceptance Criteria in `packet.spec.md` for AC-by-AC content; below are the touched test files:
  - `crates/slicer-host/tests/bridge_detector_tdd.rs` — major rewrite + rotated fixtures.
  - `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` — three new tests.
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — substring tighten.
  - `crates/slicer-ir/tests/ir_tests.rs` — replace tautology test.
- **WASM rebuild**: re-run `./modules/core-modules/build-core-modules.sh` after `rectilinear-infill` changes.
- **Verify host bindgen impls**: confirm `fn bridge_areas(&mut self, ...) -> Vec<ExPolygon>` and `fn bridge_orientation_deg(&mut self, ...) -> f32` accessor methods on the WIT resource trait in `crates/slicer-host/src/wit_host.rs` exist; if missing, add. The spec-review dispatch could not surface them via grep.
- **Supersede marker**: flip `.ralph/specs/36_bridge-detector-orca-parity/packet.spec.md` `status: implemented` → `status: superseded`, with a one-line pointer to this packet (cross-packet mutation rule allows status-only edits).

## Out of Scope

- Slice-time `_anchor_regions` refinement à la Orca's `detect_angle` — would require a new scheduler primitive granting controlled N-1 layer access; separate packet if pursued.
- `bridge_speed` / `bridge_flow_ratio` thermal/cooling overrides.
- Bridge-direction overrides via paint regions.
- Variable-density bridge fill.
- Moving polygon ops from `slicer-core` to `slicer-helpers`.
- `gyroid-infill` / `lightning-infill` bridge support.
- Multi-cluster `bridge_orientation_deg` algorithm changes (longest-valid-bridge-wins tie-break stays).
- Adding `validate_polygon_simplicity` for `Polygon` (only `ExPolygon` is added; sufficient for NEG-1).
- Fixing the determinism micro-issue noted in spec review (`best_bridge_length = 0.0_f32` sentinel + strict `>` — only matters for hypothetical zero-length valid bridges, which the new adjacency analysis prevents anyway).

## Authoritative Docs

- `docs/02_ir_schemas.md` — `BridgeRegion` and `SlicedRegion` sections; additive-minor rule. Read directly; updated by this packet.
- `docs/03_wit_and_manifest.md` — § "WIT/Type Changes Checklist". Read directly; used to verify host bindgen impls.
- `docs/04_host_scheduler.md` — § PrePass Execution + § Per-Layer Execution; cited as the architectural divergence reason. Delegate SUMMARY.
- `docs/08_coordinate_system.md` — 100 nm/unit conversions. Read directly.
- `docs/13_slicer_helpers_crate.md` — Updated by this packet.
- `docs/DEVIATION_LOG.md` — DEV-035, DEV-036 reopened; one new DEV-### registered.
- `docs/07_implementation_status.md` — TASK-167 reopened; TASK-168 added.

## OrcaSlicer Reference Obligations

- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` / `.cpp` — for the divergence-rationale paragraph in `design.md`. Delegate FACT/SUMMARY only. **Do NOT port `detect_angle`.**
- `OrcaSlicerDocumented/src/libslic3r/Surface.hpp` — `stBottomBridge` enum semantics. FACT only.

All OrcaSlicer reads MUST be delegated.

## Acceptance Summary

- Positive cases (11): cluster seed, anchor-width from edge run, xy_footprint from facet projection, direction from anchor-edge orientation, rotated min-length filter, rotated anchor-width filter, expansion margin observable, real set difference, bridge orientation precedence, schema versions constant-sourced, Benchy E2E exact marker.
- Negative cases (3): V-shape sharp-anchor pipeline produces simple polygons, empty `bridge_areas` inhibits BridgeInfill emission, top-only mesh produces no bridge regions.
- Measurable outcomes:
  - `cargo test --workspace` PASS.
  - `./modules/core-modules/build-core-modules.sh` PASS.
  - `cargo clippy --workspace -- -D warnings` PASS.
  - DEV-035 and DEV-036 marked `Open` again with rationale.
  - TASK-167 marked open in `docs/07_implementation_status.md`; TASK-168 added.
  - Packet 36's `packet.spec.md` carries `status: superseded`.
- Cross-packet impact: any future packet that consumes `BridgeRegion.anchor_width_mm`, `xy_footprint`, or `bridge_direction_deg` semantics inherits the corrected values.

## Verification Commands

- `cargo test -p slicer-host --test bridge_detector_tdd -- --nocapture`
- `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd -- --nocapture`
- `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_exact_bridge_infill_marker -- --nocapture`
- `cargo test -p slicer-ir bridge_detector_schema_versions_are_constant_sourced -- --nocapture`
- `cargo build --workspace`
- `./modules/core-modules/build-core-modules.sh`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`

## Step Completion Expectations

For each step in `implementation-plan.md`:

- Precondition stated explicitly.
- Postcondition observable.
- Falsifying check: a single command that fails until the step is done.
- Files allowed to read with line ranges where > 300 lines.
- Files allowed to edit ≤ 3.
- Expected sub-agent dispatches.
- Step context cost: S or M (no L). If a step trends to L, it MUST be split.

## Context Discipline Notes

- Large files in the read-only path:
  - `crates/slicer-host/src/wit_host.rs` (> 3000 lines) — read only the resource-trait `impl` region (lines `2900-3010`) to verify accessors. Out of scope to read in full.
  - `crates/slicer-host/src/mesh_analysis.rs` (~700 lines after packet 36) — full read in Step 3 only; otherwise range-read.
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.cpp` — delegate FACT/SUMMARY only.
- OrcaSlicer trees the implementer must NOT load directly: all of `OrcaSlicerDocumented/`.
- Likely temptation reads:
  - `crates/slicer-host/src/dispatch.rs` — out of scope; do not open.
  - Other infill modules (`gyroid-infill`, `lightning-infill`) — out of scope unless a WASM rebuild step fails for one specifically.
- Sub-agent return formats:
  - cargo runs → FACT pass/fail per test.
  - Build script → FACT pass/fail with the failing module name (≤ 5 lines).
  - OrcaSlicer FACT delegations → one-line FACT each.
  - Per-doc verification (banner bump, deviation row, task row) → FACT one-line quote each.
