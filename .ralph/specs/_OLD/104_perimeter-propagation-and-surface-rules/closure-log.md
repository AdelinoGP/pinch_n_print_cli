# Closure Log — Packet 104: Perimeter Propagation and Surface Rules

Closed: 2026-06-19

## Acceptance Gate Result

All ACs PASS:

| AC | Description | Result |
|----|-------------|--------|
| AC-1 | Per-vertex `is_bridge` propagation (outer wall, point-in-polygon, vertex-order-independent) | PASS |
| AC-2 | Inner-wall `WallBoundaryType::MaterialBoundary` via `build_wall_flags(is_outer: false)` | PASS |
| AC-2b | Inner-wall boundary type at `slicer-runtime` contract level (classic **and** arachne) | PASS |
| AC-3 | `overhang_areas()` and `surface_group()` view accessors + WIT `surface-group` record | PASS |
| AC-3-EMPTY | `overhang_areas()` returns empty slice (P106 not landed; regression bed) | PASS |
| AC-4 | `only_one_wall_top` across all three `top_shell_index` branches: `None` (full), `Some(0)` (blanket 1 wall), `Some(N>0)` (`split_top_surfaces` carve) | PASS |
| AC-5 | `only_one_wall_first_layer = true` reduces walls to 1 at `layer_index == 0` | PASS |
| AC-6 | `overhang_quartile: None` at all construction sites with sibling-roadmap doc-comment; `D-104-OVERHANG-QUARTILE-NONE` in DEVIATION_LOG.md | PASS |
| AC-N1 | Empty `bridge_areas` — no panic, all `is_bridge == false` | PASS |
| AC-N2 | `only_one_wall_top = true` on non-top layer (`top_shell_index == None`) — wall count unchanged | PASS |

Build gates:
- `cargo check --workspace --all-targets`: CLEAN
- `cargo clippy --workspace --all-targets -- -D warnings`: CLEAN (0 warnings)
- `cargo xtask build-guests --check`: no STALE

Additional test coverage added this session (all PASS):
- `slicer-core::split_top_surfaces_golden_tdd` (3 goldens, independent analytic oracle, ≤0.005 mm Hausdorff)
- `slicer-core::inner_wall_concave_reprojection_tdd` (2 — concave reprojection sampler)
- `slicer-wasm-host::contract::surface_group_resolution_tdd` (4 — runtime surface_group resolution)

## Post-Activation Scope Expansion

Three work items were added **after** packet activation (at reviewer direction) and are NOT deferrals — they are completed in this packet:

1. **`surface_group()` runtime threading (was `D-104-SURFACE-GROUP-NOT-THREADED`).** `SurfaceClassificationIR` is now threaded from the runtime blackboard through the layer-dispatch path so `surface_group()` resolves real data at runtime. Cascade (~12 LOC, 4 files, mirroring the existing `slice_ir` rail): `LayerStageInput.surface_classification` field (`slicer-wasm-host/src/traits.rs`) ← populated from `blackboard.surface_classification()` (`slicer-runtime/src/layer_executor.rs`) → projected at `binding.rs` run_stage → `dispatch_layer_call` → `push_slice_regions` → `sliced_region_to_data` (replaced the hardcoded `None` at `dispatch.rs:1300`). Locked by `surface_group_resolution_tdd` (4 cases: happy path → `Some(..)` with populated fields; id-not-found → `None`; object-not-in-classification → `None`; no `nonplanar_surface` → `None`). **Deviation retired.**

2. **Sub-top `only_one_wall_top` carve (was `D-104-ONLY-ONE-WALL-TOP-SUBTOP`).** Ported OrcaSlicer `split_top_surfaces` (`PerimeterGenerator.cpp:775`) into a new file `crates/slicer-core/src/top_surface_split.rs` (with attribution header). Adapted to reuse our pre-classified `region.top_solid_fill()` (PrePass::ShellClassification — already bridge/bottom-deduped) instead of OrcaSlicer's inline `upper_slices` derivation. `region ∩ top_solid_fill` → 1 wall; `region ∖ top_solid_fill` → full `wall_count`. Sliver filter: shoelace area threshold `MIN_AREA_UNITS_SQ = 300_000 units²` (≈ 3×10⁻³ mm²). Wired into both modules for `top_shell_index() == Some(N>0)`; `Some(0)` keeps the blanket gate and `None` stays a no-op. Locked by 3 goldens + `sub_top_layer_carve_case` / `sub_top_layer_noop_when_flag_disabled`. **Deviation retired.**

3. **Reprojection inner-wall paint sampler.** Replaced the index-based shortcut in `build_wall_flags` with geometric reprojection: each inset-ring vertex is projected to the nearest point on the original contour (via `slicer_core::geometry::closest_point_on_polygons`) and samples the annotation there. New signature carries `inset_ring_points: Option<&[Point2]>` + `original_polygons: Option<&[ExPolygon]>`. The `// TODO precise inner-wall paint sampler` comment is removed — work is done. Locked by `inner_wall_concave_reprojection_tdd` on a notched rectangle, asserting a specific concave-vertex inner-wall `tool_index` that the old index shortcut gets wrong. This also makes annotation sampling composable with the sub-top carve (carved sub-regions still sample paint from the original contour).

**T-025 (`flow_factor`) is NOT deferred** — it is completed by this packet: the field is plumbed with the documented `1.0` default. Flow-compensation is a separate roadmap area, not a P104 follow-on.

## Remaining Deviation (1)

- `D-104-OVERHANG-QUARTILE-NONE` — `Point3WithWidth.overhang_quartile` is left `None` at all construction sites (shared `expolygon_to_path3d` helper + arachne inline path). This is a genuine structural forward-dependency on the sibling roadmap `overhang-pipeline-restructuring` Phase 3 (O-T031), which introduces the quartile classification. Closing it inline would duplicate sibling-roadmap work against an unsettled IR shape. The field is advisory; the `overhang_*_4_speed` keys fall back to no-override when stamps are absent. No functional regression.

## Forward Dependency (not a deviation — pre-approved stub)

- **`overhang_areas()` returns an empty Vec** until packet `106_overhang-pipeline-prepass-foundation` (O-T010) lands the net-new `OverhangRegion.xy_footprint` field. P104 ships the accessor + WIT func + empty-stub populator by design; AC-3-EMPTY is the regression bed P106 flips to non-empty. Accessor signature will not change.

## Spec-Review Fix Iteration (Applied Before Close)

1. **Classic vertex-order normalization removed.** An unplanned classic-only wall-vertex min-(x,y) normalization in `emit_walls` had been added solely to satisfy a brittle index-based AC-1 test. Removed; the AC-1 test was rewritten to be vertex-order-independent (zips emitted path points with feature flags, classifies by x-coordinate). Restores classic's original seam/vertex ordering.
2. **`build_wall_flags` `is_outer` parameter made live.** The inner-wall empty-paint → `WallBoundaryType::Interior` downgrade now lives inside the helper (matching `design.md`'s locked invariant); the post-hoc module-side downgrade was removed from both modules.
3. **Arachne AC-2b contract coverage added** (previously classic-only).

## Test-Fixture Re-Baseline

- `inner_walls_get_no_paint_propagation` renamed to `inner_walls_get_paint_propagation` (assertion `Some(3)`) in both modules' `boundary_paint_tdd.rs`, reflecting the new T-021/T-022 inner-wall paint propagation behavior. This is the packet intent realizing itself, not a divergence.

## Golden / Oracle Fixture Inventory

`crates/slicer-core/tests/split_top_surfaces_golden_tdd.rs` — 3 fixtures, hand-derived analytic oracle (NOT snapshotted from the implementation), symmetric polygon Hausdorff ≤ 50 units (0.005 mm):

| Fixture | Region | top_solid_fill | Expected top_portion | Expected non_top_portion | Oracle derivation |
|---|---|---|---|---|---|
| full-top-coverage | 10 mm square `[0,0..100000,100000]` | identical square | whole square | empty | `A ∩ A = A`, `A ∖ A = ∅` |
| partial ~50% | 10 mm square | right half `[50000,0..100000,100000]` | right half | left half `[0,0..50000,100000]` | axis-aligned clip at `x = 50000` |
| L-shape | vert arm `[0,0..20000,80000]` ∪ horiz arm `[0,0..80000,20000]` | upper vert arm `[0,20000..20000,80000]` | upper vert arm | horiz arm `[0,0..80000,20000]` | `L ∩ upper_vert = upper_vert`, `L ∖ upper_vert = horiz_arm` |

## Forward-Looking Notes (Non-Blocking — recorded so they aren't rediscovered as surprises)

1. **Sub-top carve seam-candidate dedup at the carve boundary.** Each carve portion (`top_portion`, `non_top_portion`) emits its own seam candidates, so a seam can theoretically land on the shared carve boundary. This matches the carve semantics (each portion is independently optimized) and dedup is NOT tested. The seam-placer review (P108 T-080..T-083 territory) should check this case once it consumes the carved output. Not an issue today — only becomes relevant when a seam-placer consumes the split output.

2. **`MIN_AREA_UNITS_SQ = 300_000` is a hardcoded sliver threshold** (≈ 0.003 mm²; 1 mm² = 10⁸ units²) in `crates/slicer-core/src/top_surface_split.rs`. It matches the OrcaSlicer `PerimeterGenerator.cpp` reference at port time. Revisit (and consider promoting to a config key) if sliver behavior diverges in real prints or if printers with different nozzle geometries need a different threshold.

3. **Golden Hausdorff measure is vertex→edge only.** Sufficient for the three axis-aligned convex fixtures shipped (full / partial-50% / L-shape). If a future packet (e.g. Phase 6) adds curved-boundary or concave fixtures to the `split_top_surfaces` golden set, upgrade the metric to the bidirectional polyline-Hausdorff that P103 established for the `medial_axis` goldens.

## Repo-Hygiene Follow-Up (Resolved in amend)

`docs/adr/` previously contained two ADRs both prefixed `0013` (`0013-mmu-per-color-outer-wall-fragmentation.md` and `0013-producer-trait-for-host-builtin-seam.md`) — a pre-existing duplicate-slot collision. **Resolved in the same P104 commit via amend**: producer-trait ADR renumbered to `0024-producer-trait-for-host-builtin-seam.md`. Slot `0022` was reserved by packet 106 (`0022-overhang-classification-at-prepass.md`) and slot `0023` by packet 110 (`0023-arachne-port-strategy.md`); `0024` was the next free slot. `0013-mmu-per-color-outer-wall-fragmentation.md` keeps slot `0013` — it carries 25 cross-references in spec docs and `docs/specs/perimeter-modules-orca-parity-roadmap.md` vs. only 2 references to the producer-trait ADR (both in P104's own documentation, now updated).
