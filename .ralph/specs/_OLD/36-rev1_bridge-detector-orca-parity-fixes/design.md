# Design: bridge-detector-orca-parity-fixes

## Architectural Divergence Statement (cited rationale, not new policy)

This packet does **not** port OrcaSlicer's `BridgeDetector::detect_angle`. Per packet 12-rev1's documented divergence (`design.md:18-20`):

> No new fine-layer-height slicing pass. `docs/04_host_scheduler.md` reserves prepass slicing for coarser support layers; this packet must not add a new slicing pass.
> No change to per-layer parallel execution. `docs/04_host_scheduler.md §Per-Layer Execution` runs layers via `par_iter`; no synchronization between layer N and N±1 may be introduced.
> Use `PrePass::MeshAnalysis` output as the classification source.

Orca's `detect_angle` requires `lower_slices: const ExPolygons&` (the prior layer's filled regions, set in the constructor) to compute `_anchor_regions = intersection_ex(grown_bridge, union(lower_slices))`. Without per-layer N-1 access, that data is unavailable. The honest project-policy analog is to derive bridge orientation from the **3D anchor-edge orientation** at PrePass over `MeshIR`, where adjacency information is fully available without per-layer synchronization.

The "Orca default" comment attribution on `min_bridge_length_mm` and `anchor_width_mm` is removed because OrcaSlicer has no fixed defaults for these quantities — `min_bridge_length` is not a config key in `PrintConfig.cpp`, and anchor handling uses runtime `spacing` derived from extrusion width and dynamic `_anchor_regions`. Only `expansion_margin_mm = 1.0` corresponds to Orca's `BRIDGE_INFILL_MARGIN`. The other two values become explicit project policy.

## Controlling Code Paths

- Primary code path:
  - `crates/slicer-host/src/mesh_analysis.rs` — rewrite `compute_bridge_metrics`, `compute_anchor_width_mm`, `compute_xy_footprint`, `compute_bridge_direction_deg`. Rename and consolidate `MeshAnalysisConfig` fields. Update doc comments.
  - `crates/slicer-host/src/layer_slice.rs::assemble_bridge_areas` — switch `OffsetJoinType::Square` → `Miter`; add `expansion_margin_mm` sanity guard.
  - `modules/core-modules/rectilinear-infill/src/lib.rs` — replace `partition_expoly_by_bridges`; fix `is_bridge && bridge_areas.is_empty()` branch.
  - `crates/slicer-ir/src/slice_ir.rs` — add `CURRENT_*_SCHEMA_VERSION` constants; rewire defaults.
  - `crates/slicer-core/src/polygon_ops.rs` — add `validate_polygon_simplicity`.
  - `crates/slicer-host/src/wit_host.rs` (lines `2900-3010` only) — verify/add accessor methods on the resource trait impl.
- Neighboring tests or fixtures:
  - `crates/slicer-host/tests/bridge_detector_tdd.rs` — major rewrite + rotated-bridge fixtures; clean stale TDD scaffolding comments.
  - `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs` — three new tests (AC-8, AC-9, NEG-2).
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` — substring tighten (AC-11).
  - `crates/slicer-ir/tests/ir_tests.rs` — replace tautology test (AC-10).
- Doc updates:
  - `docs/02_ir_schemas.md` — schema banners + new field listings + stale-comment removal.
  - `docs/13_slicer_helpers_crate.md` — remove polygon-utility claim; cross-reference `slicer-core::polygon_ops`.
  - `docs/DEVIATION_LOG.md` — flip DEV-035, DEV-036; register one new DEV-### for the slicer-helpers boundary amendment.
  - `docs/07_implementation_status.md` — reopen TASK-167; add TASK-168.
  - `.ralph/specs/36_bridge-detector-orca-parity/packet.spec.md` — frontmatter `status:` flip.
- OrcaSlicer comparison surface:
  - `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` / `.cpp` — for the divergence-rationale paragraph only.

## Architecture Constraints

- **No new fine-layer slicing pass** (inherited from 12-rev1).
- **No new per-layer state** crossing `par_iter` boundaries.
- **Mesh adjacency analysis happens at PrePass.** Per-layer state stays inside `execute_layer_slice`.
- **Polygon ops live in `slicer-core::polygon_ops`** (amendment to packet 36's design.md). `slicer-helpers` is a mesh-only crate (decimate / repair / STEP import / merge). The new helper `validate_polygon_simplicity` lives in `slicer-core::polygon_ops`, alongside `intersection`, `offset`, `difference`, `OffsetJoinType`.
- **`slice-region-data` WIT shape is unchanged.** This packet does not add fields; it only fixes the values that flow through them. No WIT bump, no schema bump beyond what packet 36 already declared (`SurfaceClassificationIR = 1.1.0`, `SliceIR = 1.2.0`).
- **Schema versions are constant-sourced.** Going forward the values come from `slicer_ir::CURRENT_*_SCHEMA_VERSION`; literal constructors that previously inlined `SemVer { major: 1, minor: 1, patch: 0 }` (or similar) for production paths must use the constant.
- **Closure markers must reflect implementation reality.** A deviation may not be marked `Closed` while its underlying defect is unresolved. The 36 → 36-rev1 sequence is the model for future remediation packets that need to flip closure markers.

## Code Change Surface

- Selected approach:
  - **Cluster seed: down-facing facets.** Replace the `FacetClass::TopSurface` BFS seed in `mesh_analysis.rs` with a seed over `FacetClass`-classified down-facing/overhang facets. The existing `FacetClass` taxonomy already distinguishes top vs bottom vs side via the `overhang_threshold_deg` test; reuse that logic. (Whether this is a separate `FacetClass::Bridge` variant or a predicate over the existing variants is an implementation detail for Step 3.)
  - **Anchor-width from edge run.** The existing `build_half_edge_map` already identifies edges shared with non-bridge neighbors. Walk the perimeter of each cluster, group anchor edges into contiguous runs, project each run onto the axis perpendicular to the cluster's bridge axis, take the **shortest** run length as `anchor_width_mm`. Remove `#[allow(dead_code)]` on the anchor-edge structures.
  - **`xy_footprint` from facet projection.** For each facet in the cluster, compute the XY-projected triangle as a 3-point `Polygon`. Union all triangles via `slicer_core::polygon_ops::union` (one cluster → one or more `ExPolygon`s, depending on contour shape). Result populates `xy_footprint`.
  - **`bridge_direction_deg` from longest anchor edge.** Of all contiguous anchor-edge runs identified above, take the orientation of the longest run as the bridge direction. (Ties broken by first-encountered run for determinism.) This is the 3D analog of `detect_angle`'s "best coverage angle" without needing per-layer line-coverage scoring.
  - **`MeshAnalysisConfig` rename + consolidation.** Rename `min_anchor_width_mm` → `anchor_width_mm`. Add `overhang_threshold_deg: f32` field (currently a separate function parameter on `execute_mesh_analysis_with`). `Default::default()` keeps the existing values: `anchor_width_mm = 0.5`, `min_bridge_length_mm = 10.0`, `expansion_margin_mm = 1.0`, `overhang_threshold_deg = 45.0`. Doc comments rewritten as project policy (no "Orca default" claim).
  - **`OffsetJoinType::Miter`** in `assemble_bridge_areas`. Per packet 36 design.md:124 ("Clipper-style `MitterLimit`/`RoundJoin` semantics with a small mitter limit"). Miter is the closer match to clipper2's `MitterLimit` than `Square`. If `Miter` is not exposed by `slicer_core::polygon_ops`, use `Round`; flag in the code comment which alternative was chosen and why.
  - **Set difference in `rectilinear-infill`.** Replace `partition_expoly_by_bridges` body with: `bridge_parts = intersection(&[expoly], bridge_areas)`; `non_bridge_parts = difference(&[expoly], bridge_areas)`. Both via `slicer_core::polygon_ops`. Remove the inline "geometry ops not yet available" comment. Delete the now-unused `polygon_centroid` / `point_in_expoly_union` / `point_in_polygon` private helpers if they have no other call site.
  - **Branch fix for `is_bridge && bridge_areas.is_empty()`**: skip the bridge emission entirely (treat the same as `!is_bridge && bridge_areas.is_empty()`). Document the intent in a one-line code comment that points at the inconsistency: bridges should never reach the module with `is_bridge = true` and `bridge_areas` empty after the new `assemble_bridge_areas` runs.
  - **`CURRENT_*_SCHEMA_VERSION` constants** in `slicer_ir::slice_ir`. Replace literal `SemVer { major: …, … }` constructors at production-path sites (not test fixtures) with the constant.
  - **`validate_polygon_simplicity`** in `slicer-core::polygon_ops`: wraps the existing clipper2 validity check. Returns `Ok(())` for simple polygons; returns `Err(PolygonSimplicityError { contour_indices: Vec<usize> })` listing the failing contour indices when invalid.
  - **Test rewrites**: see `packet.spec.md` Acceptance Criteria for AC-by-AC content. Notable structural changes:
    - Build a shared `make_rotated_bridge_mesh(width_mm, length_mm, rotation_deg)` test helper at the top of `bridge_detector_tdd.rs` and reuse for AC-2 through AC-6.
    - Build a shared `make_vshape_sharp_anchor_footprint(interior_angle_deg)` test helper for NEG-1.
    - Restructure AC-7 fixture so `infill_areas` strictly contains `xy_footprint` by ≥ 2 mm in all directions (e.g., `xy_footprint = [0,0]–[20,5]`, `infill_areas = [-3,-3]–[23,8]`).
    - Strengthen `valid_bridge_passes_min_length_filter` to also assert `anchor_width_mm` matches the perpendicular run length within 0.1 mm.
- Rejected alternatives considered:
  - **Add a separate `BridgeMetricsConfig` struct** — rejected: the rename/consolidation is the spec-mandated shape; introducing a third config struct multiplies surface area unnecessarily.
  - **Port Orca's `detect_angle` with a synthetic prior-layer assumption** — rejected: would silently lie about anchoring and would still violate the "no inter-layer state at slice time" constraint.
  - **Move polygon ops from `slicer-core` to `slicer-helpers`** — rejected per user decision; the spec amendment is the smaller-blast-radius answer.
  - **Add a feature flag to retain the old centroid heuristic** — rejected: keeping the broken implementation as a fallback poisons future debugging.
  - **Replace the rotated-fixture tests with a property-based proptest** — rejected for this packet: the explicit fixture asserts are easier to debug under a remediation review; proptest can be a follow-up packet if value warrants.

## Files in Scope (read + edit)

Primary edit targets (≤ 3 per step; aggregate across packet ≤ 9 files of code + 4 docs — split into atomic steps):

- Step "Reopen closure": `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md`, `.ralph/specs/36_bridge-detector-orca-parity/packet.spec.md`.
- Step "Constants + helper": `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-core/src/polygon_ops.rs`.
- Step "Mesh adjacency rewrite": `crates/slicer-host/src/mesh_analysis.rs` (single file, large rewrite).
- Step "Slice-time fixes": `crates/slicer-host/src/layer_slice.rs`.
- Step "Module fixes": `modules/core-modules/rectilinear-infill/src/lib.rs`.
- Step "Test rewrites — host": `crates/slicer-host/tests/bridge_detector_tdd.rs`, `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`.
- Step "Test rewrites — module + IR": `modules/core-modules/rectilinear-infill/tests/bridge_infill_emission_tdd.rs`, `crates/slicer-ir/tests/ir_tests.rs`.
- Step "Doc updates": `docs/02_ir_schemas.md`, `docs/13_slicer_helpers_crate.md`, plus the new DEV-### row in `docs/DEVIATION_LOG.md`.

No new test files are created; all rewrites land in the existing test files.

## Read-Only Context

- `OrcaSlicerDocumented/src/libslic3r/BridgeDetector.hpp` / `.cpp` — delegate FACT/SUMMARY only; never load.
- `crates/slicer-helpers/src/lib.rs` — public API only via symbol search; **for confirmation that polygon ops are NOT here** (so the spec amendment is justified).
- `crates/slicer-core/src/polygon_ops.rs` — public API; identify existing `intersection`, `offset`, `difference`, `OffsetJoinType` signatures, and the clipper2 validity primitive used by the new `validate_polygon_simplicity`.
- `docs/02_ir_schemas.md` — `BridgeRegion` and `SlicedRegion` sections; additive-minor rule.
- `docs/03_wit_and_manifest.md` — § "WIT/Type Changes Checklist" — used to verify the host bindgen accessor impls flagged by the spec review.
- `docs/04_host_scheduler.md` — delegate SUMMARY of "PrePass Execution" + "Per-Layer Execution" (cited in the divergence-rationale paragraph).
- `docs/08_coordinate_system.md` — read directly.
- `docs/13_slicer_helpers_crate.md` — read directly; updated by this packet.
- `docs/DEVIATION_LOG.md` — read directly; updated by this packet.
- `crates/slicer-host/src/wit_host.rs` lines `2900-3010` — read range only; verify accessor impls.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` — delegate only.
- `target/`, `Cargo.lock`, generated code — never load.
- `crates/slicer-host/src/dispatch.rs` — > 2000 lines; out of scope.
- `crates/slicer-host/src/prepass.rs` — out of scope (we extend `execute_mesh_analysis` directly).
- All other core modules (`gyroid-infill`, `lightning-infill`, etc.) — out of scope unless the WASM rebuild step fails for one specifically.
- `wit/deps/ir-types.wit` — out of scope; no WIT changes in this packet (the field shapes that packet 36 added are kept).
- `crates/slicer-sdk/src/views.rs` — out of scope; the SDK accessors that packet 36 added are kept.
- `crates/slicer-macros/src/lib.rs` — out of scope; no macro changes.

## Expected Sub-Agent Dispatches

- "Confirm `slicer_core::polygon_ops` exposes `intersection`, `offset` (with `OffsetJoinType::Miter` or `Round`), `difference`. Return FACT with file:line and signatures." — purpose: validate Step 2 (helper) and Step 5 (module fix).
- "In `crates/slicer-host/src/wit_host.rs` lines 2900–3010, do `fn bridge_areas` and `fn bridge_orientation_deg` accessor impls exist on the `HostSliceRegionView` (or equivalent) trait? Return FACT yes/no with file:line." — purpose: validate the spec-review flag on bindgen accessors.
- "In `crates/slicer-host/src/mesh_analysis.rs`, identify the existing `FacetClass` enum variants and the function that classifies each facet. Return FACT (variant list + classifier function name + file:line)." — purpose: validate Step 3 (cluster seed selection).
- "Run `cargo test -p slicer-host --test bridge_detector_tdd`; return FACT (PASS/FAIL per test)." — purpose: validate Steps 3, 4, 6.
- "Run `cargo test -p rectilinear-infill --test bridge_infill_emission_tdd`; return FACT pass/fail per test." — purpose: validate Step 5 + Step 7.
- "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_exact_bridge_infill_marker`; return FACT pass/fail." — purpose: validate Step 7.
- "Run `cargo test -p slicer-ir bridge_detector_schema_versions_are_constant_sourced`; return FACT pass/fail." — purpose: validate Step 2.
- "Run `./modules/core-modules/build-core-modules.sh`; return FACT pass/fail with the failing module name on failure." — purpose: validate Step 5 (post-edit WASM rebuild).
- "Run `cargo test --workspace`; return FACT pass/fail with failing test list (max 20 lines)." — purpose: final acceptance.
- "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail." — purpose: final acceptance.
- "In `docs/DEVIATION_LOG.md`, find the row for DEV-035 and DEV-036 and confirm `Status` column reads `Open` after Step 1; return FACT one-line quote each." — purpose: validate closure-reversal.
- "In `docs/07_implementation_status.md`, confirm TASK-167 row is `[ ]` after Step 1 and TASK-168 row exists and is `[ ]`; return FACT one-line quote each." — purpose: validate closure-reversal.

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

## Context Cost Estimate

- Aggregate: `M` (8 atomic steps, each S or M).
- Largest single step: `M` (Step 3: mesh adjacency rewrite — single file, large internal change).
- Highest-risk dispatch: WASM rebuild (`./modules/core-modules/build-core-modules.sh`) — pass FACT-only return capturing the failing module name on failure.

## Open Questions

- None. The architectural divergence is settled (12-rev1 set the precedent, this packet cites it). The slicer-helpers vs slicer-core boundary is settled (spec amendment per user decision). The Orca-defaults attribution is settled (drop and document as project policy). The closure-reversal mechanism is settled (flip in DEVIATION_LOG.md and docs/07; reopen note points at TASK-168).
