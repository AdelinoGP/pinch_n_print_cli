# Task Map: 107_overhang-pipeline-consumers-and-refactor

Maps packet task IDs (O-T030..O-T053) to their source rows in the overhang-pipeline-restructuring roadmap and to the implementation-plan steps that deliver them.

Backlog source: `docs/specs/overhang-pipeline-restructuring.md` Phase 3 (O-T030..O-T032), Phase 4 (O-T040..O-T042), Phase 5 (O-T050..O-T053).

## Phase 3 — View accessors

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| O-T030 | Add `SliceRegionView::overhang_areas() -> &[ExPolygon]` (XY footprint of overhang facets) — derived from O-T010 (P106) | Phase 3 | Step 1 | done |
| O-T031 | Add `SliceRegionView::overhang_quartile_polygons() -> &[QuartileBand]` (per-layer quartile partition) — derived from O-T011 (P106) | Phase 3 | Step 1 | done |
| O-T032 | Mirror accessors on `PaintRegionLayerView` / `SurfaceClassificationView` if applicable; pick consistent naming with `bridge_areas()` | Phase 3 | Step 1 | done — evaluated — no mirror added (no named consumer; per design default) |

## Phase 4 — `overhang-classifier-default` refactor

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| O-T040 | Refactor `overhang-classifier-default` to read `Point3WithWidth.overhang_quartile` from `LayerCollectionView` entities; apply speed factors only | Phase 4 | Step 2 | done |
| O-T041 | Delete `classify.rs` and `lines_distancer.rs`; register retirement in `docs/DEVIATION_LOG.md` | Phase 4 | Step 2 | done |
| O-T042 | Update `overhang-classifier-default.toml` manifest: drop `LayerCollectionIR.path_geometry` reads; add narrow `overhang_quartile` read declaration | Phase 4 | Step 2 | done — reads = ["LayerCollectionIR", "LayerCollectionIR.overhang_quartile"] — base entry retained because scheduler DAG matching is exact-string (skirt-brim/part-cooling precedent) |

## Phase 5 — Verification

| Task ID | Roadmap Title | Roadmap Phase | Packet Step | Status |
| --- | --- | --- | --- | --- |
| O-T050 | End-to-end integration test: overhang ramp mesh → PrePass classification → perimeter generation → wall vertices carry expected quartiles → speed factors applied | Phase 5 | Step 3 | done — landed in documented-gap mode: perimeter side still writes None; full-propagation test ready but #[ignore]d pending T-024-WIRE-VIEW-CONSUMER |
| O-T051 | Regression coverage: pre-vs-post-refactor G-code speed factors within calibrated tolerance for benchy / standard fixtures | Phase 5 | Step 4 | done |
| O-T052 | Update `docs/01_system_architecture.md` Tier 1 + `docs/02_ir_schemas.md` to document `PrePass::OverhangAnnotation` and `SurfaceClassificationIR.overhang_quartile_polygons` | Phase 5 | Step 5 | done |
| O-T053 | Close perimeter-modules roadmap D-10 and D-12 deviations; mark T-024 / T-077 unblocked; register `D-104-OVERHANG-QUARTILE-NONE` closure in `docs/DEVIATION_LOG.md` | Phase 5 | Step 5 | done |

## Cross-Packet Contracts

- **FORWARD-DEP on P106** — `SurfaceClassificationIR.overhang_quartile_polygons` and `QuartileBand` do not exist until P106 ships (O-T011 / P106 Step 2). The `overhang_quartile_polygons()` view accessor returns empty data until P106 is `status: implemented`.
- **FORWARD-DEP on P104** — `SliceRegionView::overhang_areas()` and `surface_group()` do not exist until P104 ships (T-023 / P104 Step 1). AC-2 cannot run until P104 is `status: implemented`.
- **P108 consumer (T-077)** — `extra_perimeters_on_overhangs` becomes a real consumer once this packet's view accessors are live.
