# Overhang Pipeline Restructuring — PrePass Classification

**Status:** Active — drafted as a sibling to `perimeter-modules-orca-parity-roadmap.md`, but addresses a separable concern. Precondition for that roadmap's T-024 and T-077.
**Scope:** Move per-XY overhang classification (quartile derivation) from `PostPass::LayerFinalization` to `PrePass`, separate classification from speed-factor application, and unlock per-vertex overhang data for any Tier 2 consumer.
**Affects:** `slicer-core` (`mesh_analysis.rs` + new `overhang_annotation.rs`), `slicer-ir` (new or extended IR), `slicer-sdk` (view accessors), `overhang-classifier-default` (refactor to read-from-IR + apply-speed-factor only).

---

## Why this exists

The current `overhang-classifier-default` at `PostPass::LayerFinalization` (per [ADR-0008](../adr/0008-overhang-as-finalization-module.md)) classifies per-entity worst-case quartile by walking consecutive `LayerCollectionView`s and measuring signed distance from each wall vertex to the previous layer's wall geometry. That placement was forced by the algorithm's use of wall geometry — walls don't exist until perimeter generation completes, and Tier 2 parallel-per-layer execution prevents cross-layer access at that stage.

Three issues with the current design surfaced during perimeter-modules grilling:

1. **Per-entity granularity loses precision.** OrcaSlicer's `detect_steep_overhang` (`OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:159-199`) classifies per-segment, not per-entity. The current local-only `HashMap<(u32, u64), u8>` assigns one quartile to a whole `WallLoop`.
2. **Classification couples to speed-factor application.** Per-vertex overhang information is computed and immediately discarded; no downstream consumer can read it. `Point3WithWidth.overhang_quartile` (the IR field designed to carry it) is dead.
3. **No upstream consumer can use overhang data.** `Layer::Perimeters` would benefit from per-vertex quartile annotation (perimeter-modules roadmap T-024, T-077). `fuzzy-skin` could perturb differently in overhang zones. Infill modules could specialise. None can today.

The architecturally-correct version uses **mesh cross-sections** instead of wall geometry. The "supported region below" is fundamentally a 2D cross-section of the mesh, not a wall path. Walls are inset by `line_width/2` from the cross-section — a less accurate proxy. OrcaSlicer's algorithm actually uses lower **slices**, not walls (`PerimeterGenerator.cpp:159-199`). PrePass has full mesh access and `LayerPlanIR`'s global Z sequence — everything the corrected algorithm needs.

This roadmap moves classification to PrePass, separates it from speed-factor application, and unlocks per-vertex overhang IR for any Tier 2 consumer.

---

## Related plans

- [`docs/specs/perimeter-modules-orca-parity-roadmap.md`](./perimeter-modules-orca-parity-roadmap.md) — perimeter parity work that depends on this restructuring (T-024, T-077).
- [ADR-0008](../adr/0008-overhang-as-finalization-module.md) — overhang annotation as a FinalizationModule. **To be superseded** in part by a new ADR (see O-1 / T-001 below). The finalization-module placement was correct **for the wall-geometry-based algorithm**; with the mesh-cross-section algorithm, the constraints differ.
- [`docs/specs/infill-fill-partition-plan.md`](./infill-fill-partition-plan.md) — precedent for host-side post-commit polygon operations. Not a dependency, but informs the IR-mutation pattern.

---

## Open decision points

| ID | Decision | Default if unanswered |
|---|---|---|
| O-1 | ADR shape — write a new `0022-overhang-classification-at-prepass.md` that supersedes ADR-0008's "unnecessary scope" caveat, or amend ADR-0008 in place? | New ADR-0022. ADR-0008 stays accurate for "speed-factor application is a finalization concern"; the superseded part is just the "unnecessary scope" of a dedicated stage. |
| O-2 | New `OverhangAnnotationIR` vs extension of `SurfaceClassificationIR`? | Extension of `SurfaceClassificationIR`. Overhang classification is a sub-aspect of surface classification; a parallel IR would duplicate the per-object indexing. |
| O-3 | Mesh cross-section infrastructure — reuse `PrePass::SupportGeometry`'s plane-triangle intersection helpers, or implement independently? | Reuse via promotion to `slicer-core/src/algos/mesh_cross_section.rs` (extract from `support_geometry.rs`). Two callers means it earns its keep as a shared primitive. |
| O-4 | Quartile thresholds — OrcaSlicer's hardcoded constants (`detect_steep_overhang` uses `0.5 * extrusion_width` per band), or derive from `line_width` config? | Derive from config (`line_width * { 0.5, 1.0, 1.5, 2.0 }` for quartile band boundaries). Matches Orca's intent without baking in nozzle assumptions. |
| O-5 | Stage ordering — after `PrePass::MeshAnalysis` only, after `PrePass::LayerPlanning` only, or strictly after both? | Strictly after both. Needs MeshAnalysis for facet-level overhang classification (to AABB-prefilter the cross-section work) and LayerPlanning for the global Z sequence. |
| O-6 | Fate of existing `overhang-classifier-default` — retire the module entirely (functionality moves to host) or keep it as a finalization-tier consumer that reads quartiles from IR and applies speed factors? | Keep it. Speed-factor application is a finalization-tier concern (`EntityMutation::SetSpeedFactor` is a finalization API). The module shrinks to ~50 LOC — pure consumer. ADR-0008's core decision stands for that part. |
| O-7 | Output shape per layer — `Vec<(quartile, Vec<ExPolygon>)>` (4 polygon sets per layer) or distance field (e.g. signed-distance polygon)? | Polygon partition (4 sets). Matches existing IR style (polygons, not fields); per-vertex membership is a cheap point-in-polygon test. |
| O-8 | Fold `OverhangRegion.xy_footprint` (perimeter-roadmap D-12) into this roadmap? | Yes — same workstream (PrePass-side overhang plumbing). Closes the asymmetry with `BridgeRegion.xy_footprint` at the same time. |

---

## Phases & tasks

### Phase 0 — Decisions + ADR

| ID | Title | Files | Acceptance |
|---|---|---|---|
| O-T001 | Author `docs/adr/0022-overhang-classification-at-prepass.md` superseding ADR-0008's "unnecessary scope" clause (per O-1) | `docs/adr/0022-overhang-classification-at-prepass.md` | ADR documents: changed algorithm (mesh cross-sections vs walls), changed use case (multiple Tier 2 consumers), what stays from ADR-0008 (speed-factor application at finalization). |
| O-T002 | Resolve O-2 through O-8 (grill or decide at packet-generation time); update this roadmap | this file | All `[blocked: O-N]` tags removed. |

### Phase 1 — IR additions

| ID | Title | Files | Acceptance |
|---|---|---|---|
| O-T010 | `[blocked: O-8]` Add `xy_footprint: Vec<ExPolygon>` to `OverhangRegion`; populate in `MeshAnalysis` using the same pattern as `BridgeRegion` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-core/src/algos/mesh_analysis.rs:200-212` (alongside existing `OverhangRegion` construction), `compute_xy_footprint` extension | New field present; `MeshAnalysis` populates it; golden-file test on overhang fixture. |
| O-T011 | `[blocked: O-2]` Add per-layer overhang quartile polygons to `SurfaceClassificationIR` (extension) — `pub overhang_quartile_polygons: HashMap<u32, Vec<QuartileBand>>` where `QuartileBand { quartile: u8, polygons: Vec<ExPolygon> }` | `crates/slicer-ir/src/slice_ir.rs` (schema bump) | Type compiles; serializes; empty default works. |
| O-T012 | Promote mesh cross-section helpers from `PrePass::SupportGeometry` to a shared `slicer-core/src/algos/mesh_cross_section.rs` per O-3 | `crates/slicer-core/src/algos/mesh_cross_section.rs` (new), `crates/slicer-core/src/algos/support_geometry.rs` (consumer) | Two callers (SupportGeometry + new OverhangAnnotation) share one implementation; SupportGeometry tests still green. |

### Phase 2 — `PrePass::OverhangAnnotation` stage

| ID | Title | Files | Acceptance |
|---|---|---|---|
| O-T020 | `[blocked: O-5]` Declare `PrePass::OverhangAnnotation` stage in the stage order (after `MeshAnalysis` + `LayerPlanning`); host scheduling | `crates/slicer-runtime/src/execution_plan.rs`, `crates/slicer-scheduler/src/lib.rs`, `docs/01_system_architecture.md` Tier 1 diagram | Stage runs in correct order; manifest schema accepts it. |
| O-T021 | Implement classifier algorithm in `slicer-core/src/algos/overhang_annotation.rs`: for each consecutive `(layer_n, layer_{n-1})`, compute cross-sections via O-T012, derive distance field, partition into 4 quartile bands per O-7 | `crates/slicer-core/src/algos/overhang_annotation.rs` (new) | Reference fixture (overhang ramp at known angle) produces expected quartile partition; golden file. |
| O-T022 | `[blocked: O-4]` Wire quartile thresholds to config (`line_width` derivation) per O-4 | (same file) | Threshold values match config; TDD covers two `line_width` settings. |
| O-T023 | Host stage runner: invoke `overhang_annotation` after MeshAnalysis + LayerPlanning commit; write to Blackboard `SurfaceClassificationIR` extension field | `crates/slicer-runtime/src/prepass.rs` (or analogous), Blackboard commit path | End-to-end PrePass run produces overhang quartile polygons on Blackboard for a benchy-style fixture. |

### Phase 3 — View accessors

| ID | Title | Files | Acceptance |
|---|---|---|---|
| O-T030 | Add `SliceRegionView::overhang_areas() -> &[ExPolygon]` (XY footprint of all overhang facets covering this region on this layer) — derived from O-T010 | `crates/slicer-sdk/src/views.rs`, `crates/slicer-schema/wit/deps/ir-types.wit`, `crates/slicer-wasm-host/src/host.rs` | Accessor returns pre-filtered polygon list; WIT type identity verified per CLAUDE.md checklist. |
| O-T031 | Add `SliceRegionView::overhang_quartile_polygons() -> &[QuartileBand]` (per-layer quartile partition) — derived from O-T011 | (same files) | Accessor returns per-layer quartile bands for this region's polygon area. |
| O-T032 | Mirror the accessors on `PaintRegionLayerView` / `SurfaceClassificationView` if applicable; pick consistent naming with `bridge_areas()` | (same files) | Naming consistent; downstream readers unblocked. |

### Phase 4 — `overhang-classifier-default` refactor

| ID | Title | Files | Acceptance |
|---|---|---|---|
| O-T040 | `[blocked: O-6]` Refactor `overhang-classifier-default` to read `Point3WithWidth.overhang_quartile` from `LayerCollectionView` entities (populated upstream by perimeter modules consuming Phase 3 accessors) and apply speed factors only | `modules/core-modules/overhang-classifier-default/src/lib.rs`, delete `classify.rs` + `lines_distancer.rs` | Module shrinks to ~50 LOC; speed-factor mutation behaviour identical for golden fixtures (regression check). |
| O-T041 | Delete the now-redundant cross-layer wall-distance classification code (`classify.rs`, `lines_distancer.rs`); reference them in DEVIATION_LOG as retired in favour of upstream classification | `modules/core-modules/overhang-classifier-default/src/{classify,lines_distancer}.rs` removed, `docs/DEVIATION_LOG.md` | Files gone; deviation logged. |
| O-T042 | Update `overhang-classifier-default.toml` manifest: drop the dependency on `LayerCollectionIR.path_geometry` reads; add dependency on per-vertex `overhang_quartile` | manifest | Manifest reflects narrower IR access. |

### Phase 5 — Verification

| ID | Title | Files | Acceptance |
|---|---|---|---|
| O-T050 | Reference fixture for end-to-end overhang quartile propagation: overhang ramp mesh → PrePass classification → perimeter generation → wall vertices carry expected quartiles → finalization applies expected speed factors | `crates/slicer-runtime/tests/fixtures/overhang/`, `crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs` (new) | Layer-by-layer assertion against a recorded reference output. |
| O-T051 | Regression coverage: ensure existing benchy/test-fixture speed factors match (within tolerance) the pre-refactor behaviour, so the migration is transparent to gcode output | (same test file) | Diff between pre- and post-refactor gcode is within calibrated tolerance for the regression fixtures. |
| O-T052 | Update `docs/01_system_architecture.md` Tier 1 block to document `PrePass::OverhangAnnotation`; update `docs/02_ir_schemas.md` with the new field on `SurfaceClassificationIR` | `docs/01_system_architecture.md`, `docs/02_ir_schemas.md` | Docs match implementation. |
| O-T053 | Close perimeter-modules roadmap D-10 and D-12 deviations; mark T-024 / T-077 unblocked | `docs/specs/perimeter-modules-orca-parity-roadmap.md`, `docs/DEVIATION_LOG.md` | Cross-references aligned; perimeter roadmap can start consuming. |

---

## Task count snapshot

- Phase 0: 2 | Phase 1: 3 | Phase 2: 4 | Phase 3: 3 | Phase 4: 3 | Phase 5: 4 — **19 tasks**.

Packetisable into ~5 packets. Independent of perimeter-modules roadmap; can start immediately.

---

## Cross-reference summary

This roadmap is a precondition for these tasks in `perimeter-modules-orca-parity-roadmap.md`:

| Perimeter-roadmap task | What it gains |
|---|---|
| T-024 | Can now read overhang quartile polygons via Phase 3 accessors and populate `Point3WithWidth.overhang_quartile` per-vertex (was: deliberately left None). |
| T-077 (`extra_perimeters_on_overhangs`) | Can read `SliceRegionView::overhang_areas()` (O-T030) for region membership; can optionally use quartile thresholds (O-T031) for finer-grained extra-perimeter rules. |

Conversely, this roadmap depends on no part of the perimeter-modules roadmap. It can ship independently and start delivering value immediately (overhang-classifier-default behaviour improves the moment Phase 4 lands).
