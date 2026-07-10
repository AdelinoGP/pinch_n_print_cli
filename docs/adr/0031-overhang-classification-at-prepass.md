# ADR-0031: Overhang Classification Moves to a `PrePass::OverhangAnnotation` Stage

## Status

Accepted.

## Context

[ADR-0008](./0008-overhang-as-finalization-module.md) placed overhang classification inside a `FinalizationModule` (`overhang-classifier-default`) because the algorithm walked `LayerCollectionView` wall geometry — and walls don't exist until perimeter generation completes, which forced the classification downstream to `PostPass::LayerFinalization`.

`docs/specs/overhang-pipeline-restructuring.md` documents three problems with that placement surfacing during perimeter-modules grilling: per-entity (not per-segment) granularity loses precision versus OrcaSlicer's `detect_steep_overhang`; classification and speed-factor application are coupled, so per-vertex overhang data is computed and discarded, leaving `Point3WithWidth.overhang_quartile` permanently dead; and no upstream Tier 2 consumer (perimeter generation, fuzzy-skin, infill) can read overhang data at all, blocking perimeter-roadmap tasks T-024 and T-077.

The changed algorithm — mesh cross-sections vs walls — is the resolution: OrcaSlicer's own algorithm (`PerimeterGenerator.cpp:159-199`) classifies against lower **slices** of the mesh, not walls. Walls are merely an inset-by-`line_width/2` proxy for the true cross-section. PrePass has full `MeshIR` and `LayerPlanIR` Z-sequence access — everything the corrected algorithm needs — so the algorithm no longer requires waiting for wall geometry to exist.

The changed use case is multiple Tier 2 consumers: perimeter generation, fuzzy-skin, and infill modules can each read per-layer quartile polygon partitions once classification runs in PrePass and lands on the Blackboard, instead of the result being locked inside one finalization module's local `HashMap`.

## Decision

Introduce a new `PrePass::OverhangAnnotation` stage that runs strictly after `PrePass::MeshAnalysis` and `PrePass::LayerPlanning` (needs `MeshAnalysis`'s facet-level classification to AABB-prefilter the cross-section work, and `LayerPlanning`'s global Z sequence to walk consecutive layers). It computes per-layer mesh cross-sections — reusing a shared `slicer-core/src/algos/mesh_cross_section.rs` helper extracted from `support_geometry.rs`'s existing plane-triangle intersection code, since `triangle_mesh_slicer.rs` is the actual source of the plane-triangle primitives being generalized, not `support_geometry.rs`'s callers — and partitions the resulting distance field into four quartile bands via polygon partition (`Vec<(quartile, Vec<ExPolygon>)>` per layer), with quartile thresholds derived from `line_width` config (`line_width * { 0.5, 1.0, 1.5, 2.0 }`) rather than OrcaSlicer's hardcoded constants. The result extends `SurfaceClassificationIR` (not a new parallel IR) with per-layer quartile polygon data, keeping per-object indexing in one place.

`overhang-classifier-default` is **kept**, not retired: it shrinks to a pure finalization-tier consumer that reads `Point3WithWidth.overhang_quartile` (now populated upstream) and applies `EntityMutation::SetSpeedFactor`.

This ADR **supersedes ADR-0008's "unnecessary scope" caveat** — the claim that a dedicated stage was unnecessary scope no longer holds once multiple Tier 2 consumers need the data — but ADR-0008's core decision that **speed-factor application belongs at finalization** remains valid and unchanged: `set-speed-factor` mutation still happens in the `FinalizationModule`, only classification moves upstream.

### Amendment (overhang-after-Slice inversion)

The original decision above had `PrePass::OverhangAnnotation` **compute its own per-layer mesh cross-sections** (via a shared `mesh_cross_section.rs` helper) and run **after `PrePass::MeshAnalysis` / `PrePass::LayerPlanning` but before `PrePass::Slice`**. That was a second full-mesh slicing pass in addition to `PrePass::Slice`'s, and it did not match OrcaSlicer, whose `PrintObject::detect_overhangs_for_lift` (`PrintObject.cpp:880-908`) derives overhang by diffing consecutive already-computed `lslices` — never re-slicing.

This clause is amended: `PrePass::OverhangAnnotation` now runs **strictly after `PrePass::Slice`** and derives each object's overhang bands from the committed `SliceIR` (its final per-layer region polygons), diffing consecutive layers (`diff(current, previous)`). The object meshes are therefore sliced **exactly once**, in `PrePass::Slice` (which now slices in world space so its polygons, its bridge classification, and the overhang bands derived from them all agree). The flat-bridge enclosure test in `PrePass::Slice` likewise computes its unsupported region as the same consecutive-slice diff instead of consuming `overhang_quartile_polygons`, so nothing between the old and new positions depends on overhang data. The rest of this ADR — the multi-consumer motivation, the `SurfaceClassificationIR` extension shape, the quartile-polygon output, and keeping `overhang-classifier-default` as a finalization consumer — stands unchanged; Tier 2 consumers still read the bands from the `Blackboard` during layer execution, which runs after all of PrePass regardless of the intra-PrePass move.

## Consequences

- **New stage, new WIT surface.** Unlike ADR-0008's zero-WIT-churn constraint, this decision does add a `PrePass::OverhangAnnotation` stage and new `SurfaceClassificationIR` fields plus `SliceRegionView` accessors (`overhang_areas()`, `overhang_quartile_polygons()`) — an explicit, accepted departure from ADR-0008's "no new stage" framing, justified by the multi-consumer use case ADR-0008 did not anticipate.
- **`Point3WithWidth.overhang_quartile` becomes live.** Perimeter generation (and other Tier 2 modules) populate it from the Phase 3 accessors; it is no longer a dead IR field.
- **`overhang-classifier-default` shrinks** to ~50 LOC, dropping `classify.rs` and `lines_distancer.rs` (the cross-layer wall-distance code is redundant once classification happens upstream); its manifest narrows to depend on per-vertex `overhang_quartile` instead of `LayerCollectionIR.path_geometry`.
- **Mesh cross-section code is shared**, not duplicated: `SupportGeometry` and the new `OverhangAnnotation` stage both call the promoted `mesh_cross_section.rs` helper, avoiding a second plane-triangle implementation.
- **Migration must be behaviour-transparent.** Regression coverage (packet-level, tracked in the roadmap's Phase 5) compares pre- and post-refactor gcode speed factors within tolerance so the move is invisible to end output.

## Alternatives considered

- **Amend ADR-0008 in place** instead of writing a new ADR. Rejected: ADR-0008's placement was correct *for the wall-geometry-based algorithm*; the algorithm itself has changed (mesh cross-sections, not walls), and the constraint that forced the original decision (walls don't exist until perimeter generation) no longer applies. A new ADR keeps the historical record of why the original placement was made, while `docs/specs/overhang-pipeline-restructuring.md` remains the roadmap superseding it.
- **Retire `overhang-classifier-default` entirely**, moving speed-factor application to the host. Rejected: `EntityMutation::SetSpeedFactor` is a finalization-tier API; ADR-0008's reasoning that this application step belongs in a `FinalizationModule` is unaffected by the classification-algorithm change.
- **Distance-field output shape** instead of quartile polygon partition. Rejected: polygon sets match the existing IR style elsewhere (e.g. `BridgeRegion`), and per-vertex quartile membership reduces to a cheap point-in-polygon test against 4 small polygon sets rather than sampling a field.

## Cross-references

- ADR-0008 (overhang annotation as a FinalizationModule) — speed-factor-application-at-finalization decision stands; only the "unnecessary scope" caveat is superseded.
- `docs/specs/overhang-pipeline-restructuring.md` — the roadmap this ADR unblocks (Phase 0); resolves roadmap open decision O-1.
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — T-024 and T-077 depend on this restructuring landing.
