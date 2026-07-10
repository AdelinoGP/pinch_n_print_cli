---
status: implemented
packet: bridge-detector-orca-parity-fixes
task_ids:
  - TASK-168
supersedes: 36_bridge-detector-orca-parity
---

# 36-rev1_bridge-detector-orca-parity-fixes

## Goal

Replace packet 36's bbox-and-centroid heuristics with the project's honest PrePass mesh-adjacency analog of OrcaSlicer's `BridgeDetector`. Packet 36 shipped on green tests over heuristic implementations: the mesh-adjacency analysis seeded clusters from `FacetClass::TopSurface` (top-facing facets — bridges are down-facing overhangs), computed `anchor_width_mm` and `xy_footprint` from cluster bounding boxes rather than anchor-edge runs and facet projections, hardcoded `bridge_direction_deg` to 0° or 90° from bbox aspect ratio, used a centroid containment heuristic (not a polygon set difference) for `infill_areas \ bridge_areas` in `rectilinear-infill`, and validated the result with one tautological schema-version test and one fake sharp-anchor test that uses a rectangle. This packet rewrites those pieces, adds rotated-bridge fixtures so the bbox shortcut cannot regress, replaces the broken AC tests with tests that actually enforce their claims, and reopens the DEV-035 / DEV-036 / TASK-167 closure markers that were prematurely flipped.

This packet does **not** attempt full Orca parity for `BridgeDetector::detect_angle`. Per packet 12-rev1's documented divergence, `docs/04_host_scheduler.md` forbids new fine-layer slicing passes and `par_iter` per-layer execution forbids N±1 layer synchronization. Orca's `detect_angle` requires `lower_slices` (the prior layer's filled regions). The honest project policy is to derive `bridge_direction_deg` from the 3D anchor-edge orientation at PrePass over MeshIR, not from a per-layer 2D line-coverage sweep. The "Orca default" doc-comment attribution on `min_bridge_length_mm` and `anchor_width_mm` is removed; values are documented as project policy with explicit rationale.

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

## Architecture Constraints

- **No new fine-layer slicing pass** (inherited from 12-rev1).
- **No new per-layer state** crossing `par_iter` boundaries.
- **Mesh adjacency analysis happens at PrePass.** Per-layer state stays inside `execute_layer_slice`.
- **Polygon ops live in `slicer-core::polygon_ops`** (amendment to packet 36's design.md). `slicer-helpers` is a mesh-only crate (decimate / repair / STEP import / merge). The new helper `validate_polygon_simplicity` lives in `slicer-core::polygon_ops`, alongside `intersection`, `offset`, `difference`, `OffsetJoinType`.
- **`slice-region-data` WIT shape is unchanged.** This packet does not add fields; it only fixes the values that flow through them. No WIT bump, no schema bump beyond what packet 36 already declared (`SurfaceClassificationIR = 1.1.0`, `SliceIR = 1.2.0`).
- **Schema versions are constant-sourced.** Going forward the values come from `slicer_ir::CURRENT_*_SCHEMA_VERSION`; literal constructors that previously inlined `SemVer { major: 1, minor: 1, patch: 0 }` (or similar) for production paths must use the constant.
- **Closure markers must reflect implementation reality.** A deviation may not be marked `Closed` while its underlying defect is unresolved. The 36 → 36-rev1 sequence is the model for future remediation packets that need to flip closure markers.

## Data and Contract Notes

- IR or manifest contracts touched:
  - **No schema bumps in this packet.** Packet 36 already declared `SurfaceClassificationIR = 1.1.0` and `SliceIR = 1.2.0`. This packet only adds the `CURRENT_*_SCHEMA_VERSION` constants and rewires production constructors to use them; the on-wire shape is unchanged.
  - `MeshAnalysisConfig` field rename (`min_anchor_width_mm` → `anchor_width_mm`) is a **breaking API change** for any external consumer that constructed it by name. Within the workspace, the only constructors are `Default::default()` and possibly fixture sites — all updated as part of Step 3.
- WIT boundary considerations:
  - **No WIT changes.** Verify the existing accessor methods on `HostSliceRegionView` exist (the spec-review dispatch flagged a grep miss). If missing, add them — this is the only WIT-adjacent risk.
- Determinism or scheduler constraints:
  - Mesh adjacency analysis is pure over `(MeshIR, MeshAnalysisConfig)`; deterministic.
  - `bridge_direction_deg` tie-break: ties on anchor-edge length broken by first-encountered run in cluster facet order. Documented in `compute_bridge_direction_deg` as a code comment.
  - `bridge_orientation_deg` tie-break in `assemble_bridge_areas` is unchanged from packet 36 (longest valid bridge wins; first wins on ties).
  - Polygon set-difference operations from clipper2 are deterministic when the input is deterministic.

## Locked Assumptions and Invariants

- `MeshIR.objects[*].mesh.indices` is in triangle order (3 indices per facet); same assumption used by `mesh_analysis.rs` today.
- The mesh is "manifold enough" for half-edge analysis. Non-manifold meshes degrade gracefully: anchor-edge identification yields whatever runs the half-edge map can complete; bbox-fallback values are never substituted (a degraded answer is still an honest one).
- 100 nm/unit coordinate convention.
- `slicer_core::polygon_ops::offset` with `Miter` (or `Round`) join handles sharp anchor corners without producing self-intersecting contours; `validate_polygon_simplicity` is the explicit check (NEG-1).
- `BridgeRegion.facet_indices` is always non-empty for clusters that survive validity filtering.
- The `is_bridge` flag on `SlicedRegion` and the `bridge_areas` field are populated by separate code paths but should agree: `is_bridge == true` implies `!bridge_areas.is_empty()` after the new assembly. NEG-2 verifies the module's defensive behavior in the inconsistent state, but the inconsistent state should not arise in practice after this packet.

## Risks and Tradeoffs

- **Cluster-seed inversion.** Switching from `TopSurface` to down-facing facets is a one-line predicate change but it changes which clusters get analyzed. Existing test fixtures that worked under the inverted contract may need to be regenerated. Mitigation: rotated-bridge fixtures are designed from scratch in this packet; the axis-aligned fixture from packet 36 is rebuilt to the new contract.
- **Anchor-edge run vs perpendicular projection ambiguity.** "Shortest perpendicular run" requires picking an axis to project onto. In this packet, the axis is the bridge direction (the longest anchor-edge run's orientation). Mitigation: `compute_anchor_width_mm` is documented to take `bridge_direction_deg` as input; the dependency is explicit.
- **`xy_footprint` polygon-union performance.** Unioning N triangle polygons per cluster per object can be slow for high-poly meshes. For typical Benchy-scale bridges (≤ 100 facets per cluster), sub-millisecond. Mitigation: the existing `slicer_core::polygon_ops::union` is clipper2-backed; if performance regresses, profile and add a coarser-grained union batch.
- **`OffsetJoinType` availability.** Packet 36 used `Square`; design.md specified Mitter/Round. If `slicer_core::polygon_ops::OffsetJoinType` doesn't expose `Miter`, fall back to `Round` and document.
- **`is_bridge && bridge_areas.is_empty()` may be unreachable.** The new assembly should make this state unreachable. NEG-2 verifies the module's defensive behavior anyway, since the state can still be constructed by tests or future regressions.
- **Reopening DEV-035 / DEV-036 / TASK-167 will appear as "regression" in audit tooling** that treats Closed as terminal. Mitigation: explicit reopen-rationale text on each row; downstream audit tooling should be told that closure markers are not monotonic.
- **`slicer-helpers` boundary amendment** registers a new DEV-### but does not actually change behavior — it documents reality. Risk: a future packet could re-introduce the same misattribution. Mitigation: the new DEV-### explicitly names `docs/13` as the source of truth.
- **No new WIT or SDK changes** — but the spec review flagged a grep miss on `fn bridge_areas` / `fn bridge_orientation_deg` accessor impls in `wit_host.rs`. Mitigation: Step 4 (slice-time fixes) is preceded by an explicit dispatch confirming these impls exist; if missing, they are added in the same step.
