# Perimeter Modules — OrcaSlicer Parity Roadmap

**Status:** Active — drafted from audit of `classic-perimeters` and `arachne-perimeters` against `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` and `OrcaSlicerDocumented/src/libslic3r/Arachne/`.
**Scope:** Bring both perimeter modules to full OrcaSlicer feature parity, within this project's split-module architecture.
**Sequencing:** Two milestones (M1, M2). M1 ships Classic at parity and deletes the iterative-inset fake-Arachne module (`variable-width-perimeters` never ships — see D-110-DROP-VARIABLE-WIDTH). M2 creates real Arachne (Voronoi + skeletal trapezoidation + BeadingStrategy stack) fresh under `arachne-perimeters` via P110+P112.
**Task granularity.** Each `T-NNN` is a single discrete unit of work. Packets will be assembled from contiguous tasks later — not in this document.

---

## Related plans

- [`docs/specs/infill-fill-partition-plan.md`](./infill-fill-partition-plan.md) — host-side fill-polygon partition at `Layer::Perimeters` commit. **Must land before this roadmap's Phase 1** (T-013 specifically) to avoid `SlicedRegion` schema-bump collision.
- [`docs/specs/overhang-pipeline-restructuring.md`](./overhang-pipeline-restructuring.md) — moves overhang classification to PrePass via mesh cross-sections; adds `OverhangRegion.xy_footprint` (was D-12 here) and per-layer quartile polygons; refactors `overhang-classifier-default` to read-from-IR. **Precondition for T-024 (per-vertex overhang_quartile propagation) and T-077 (`extra_perimeters_on_overhangs`).** Authored by [ADR-0031](../adr/0031-overhang-classification-at-prepass.md) (landed at slot 0031 — slot 0022 had been taken by the per-region-origin ADR before P106 shipped).
- [ADR-0008](../adr/0008-overhang-as-finalization-module.md) — overhang annotation as a FinalizationModule. Partially superseded by [ADR-0031](../adr/0031-overhang-classification-at-prepass.md) (classification moves to PrePass; speed-factor application stays at finalization).
- [ADR-0011](../adr/0011-perimeter-module-owns-wall-sequencing.md) — perimeter module owns wall-sequence reordering.
- [ADR-0013](../adr/0013-mmu-per-color-outer-wall-fragmentation.md) — MMU multi-color perimeters fragment per-color with bisector ownership (supersedes `D-96-AC22-EXTERNAL-CONTOUR`). Drives the T-P96-A0 through T-P96-F task block below.
- [`docs/specs/paint-pipeline-orca-parity-roadmap.md`](./paint-pipeline-orca-parity-roadmap.md) — **Inherited obligation:** P96 closed AC-22b via a non-parity `SlicedRegion.external_contour` simplification (D-96-AC22-EXTERNAL-CONTOUR). This roadmap supersedes that mechanism with per-color outer-wall fragmentation + deterministic per-edge bisector ownership. See "Inherited from P96 — AC-22b reshape obligation" section below.
- [`docs/specs/paint-pipeline-orca-parity-roadmap.md`](./paint-pipeline-orca-parity-roadmap.md) — **Inherited obligation (P98):** P98 decoded `paint_seam` sub-facet strokes that now reach `SlicedRegion.variant_chain` (`("seam_enforcer"/"seam_blocker", …)`) but have no live consumer (D-98-SEAM-NO-CONSUMER) — `seam-placer` scores seams geometrically. This roadmap wires painted seam_enforcer/seam_blocker into seam-candidate generation. See "Inherited from P98 — paint_seam stroke consumption obligation" section below.
- **Out-of-scope sibling roadmap (referenced from closed decision):**
  - Spiral vase + non-planar wall pipeline (per D-3): LayerPlanning surface-group synthesis + `non-planar-walls` PerimetersPostProcess module + helical Z modulation.

## Architectural framing (read first)

This codebase splits `process_classic()`'s responsibilities across several modules. Before reading any task, internalise the split:

| OrcaSlicer responsibility | Owner in this codebase |
|---|---|
| Wall-loop geometry, hole/contour nesting, thin-walls, gap-fill, spiral vase | **Perimeter module** (classic / arachne) |
| Per-vertex paint/material/bridge/overhang/fuzzy flags on `WallLoop.feature_flags` | **Perimeter module** (propagation only — data computed upstream) |
| Seam candidate scoring (corner-based) | **Perimeter module** (producer) |
| Seam candidate selection + wall rotation | `seam-placer` at `Layer::PerimetersPostProcess` |
| Fuzzy-skin XY perturbation | `fuzzy-skin` at `Layer::PerimetersPostProcess` |
| Overhang speed-quartile dispatch | `overhang-classifier-default` at `PostPass::LayerFinalization` |
| Bridge detection (mesh-level) | `PrePass::MeshAnalysis` → `SurfaceClassificationIR` |
| Tool-change G-code from `tool_index` | path-optimization → GCodeEmit (packet 50b) |
| Top-surface / bottom-surface classification | `PrePass::MeshAnalysis` → `top_shell_index` / `bottom_shell_index` |
| NN ordering, retract/Z-hop | `path-optimization-default` |
| Top-surface ironing | `top-surface-ironing` at `Layer::Infill` |

Tasks that look like "implement overhang detection" therefore become "propagate the upstream overhang classification onto per-vertex flags". The perimeter module is much narrower than `process_classic` suggests.

---

## Milestone summary

### M1 — Classic parity + delete iterative-inset fake-Arachne — ✅ COMPLETE (P102–P109, verified by P109, 2026-07-02; parity fixtures are self-captured regression baselines, not OrcaSlicer-derived — accepted limitation, see D-109-SELF-CAPTURED-FIXTURES in `docs/DEVIATION_LOG.md`)
Outcomes:
- `classic-perimeters` reaches feature parity with OrcaSlicer `process_classic()`.
- The fake-Arachne `arachne-perimeters` module (iterative-inset, not real Arachne) is **deleted** under P108 (dir+tests+manifest removed; workspace member removed; doc refs removed). `variable-width-perimeters` never ships (D-110-DROP-VARIABLE-WIDTH). Rename tasks T-090/T-091/T-092 are cancelled and re-scoped to deletion.
- Cross-cutting fixes (shared util crate, IR widening, builder Result propagation, per-layer config).
- Reference-fixture parity harness up and running.

Phases:
- Phase 0 — Truth in advertising
- Phase 1 — Cross-cutting foundations
- Phase 2 — Upstream-data propagation
- Phase 3 — Surface-driven wall-count rules
- Phase 4 — `slicer-core` polygon-op primitives
- Phase 5 — Classic spacing model
- Phase 6 — Thin-walls + gap-fill
- Phase 7 — Classic special modes
- Phase 8 — Seam-candidate quality
- Phase 9 — Verification

### M2 — Real Arachne — DONE (P110–P112 + P113a/b follow-up; P112 implementation complete 2026-07-03, P113b topology-faithfulness pass complete 2026-07-04)
Outcomes:
- New `arachne-perimeters` module with Voronoi + skeletal trapezoidation + 5-strategy beading stack, wired end-to-end via a WIT host-service bridge (`generate-arachne-walls`, D-112-HOSTSVC-BRIDGE).
- Per-junction width assignment from real bead-count propagation, sourced from `BeadingStrategy::compute()` (D-112-TOOLPATH-WIDTH, closed).
- Parity-harness coverage for variable-width cases — 4 Arachne fixtures + `cube_4color_arachne` MMU structural fixture, all self-captured regression baselines (D-112-SELFCAPTURED-BASELINES; no OrcaSlicer oracle in-repo).
- `wall_generator` config key (`classic` default) resolves the `perimeter-generator` claim between `classic-perimeters` and `arachne-perimeters` (D-112-WALL-GENERATOR-SELECT, closed).

Phases:
- Phase 10 — Foundations (Voronoi + SkeletalTrapezoidation) — DONE (P110)
- Phase 11 — BeadingStrategy stack — DONE (P111)
- Phase 12 — Extrusion generation — DONE (P112)
- Phase 13 — Wire-up + verification — DONE (P112; T-234 closure ceremony run at packet close — GREEN)

### P113a/b — M2 topology-faithfulness follow-up
A small follow-up pass (P113a rib/quad-cell topology + P113b faithful `connectJunctions`) tightened M2's structural parity with OrcaSlicer's Arachne graph:

- **P113a (Step 1)** — synthetic rib/quad-cell topology pass
  (`slicer-core/src/skeletal_trapezoidation/rib.rs`) classifies sharp-corner
  Voronoi edges as `EXTRA_VD` ribs and groups them into `QuadCell`s, giving
  later centrality/propagation/toolpath passes the same rib/spine distinction
  upstream uses.

- **P113b (Steps 2–5)** — faithful predicate + bead-count propagation + toolpath
  emission:
  * `filter_central` uses `dR < dD * sin(transitioning_angle/2)` on spine
    edges with `EXTRA_VD` ribs unconditionally non-central;
  * `bead_count` moves to `STVertex` and is assigned per node;
  * propagation re-ports `generateTransitionMids`/`applyTransitions` helpers;
  * `generate_toolpaths` emits per-edge per-bead `ExtrusionLine`s from
    `BeadingStrategy::compute()`-sourced junction fans, with `is_closed=false`
    and `BTreeMap` bucket ordering, leaving ring closure to `stitch_extrusions`.

Self-captured regression fixtures (`centrality_*`, `bead_count_*`,
`propagation_*`, `toolpaths_tapered_wedge.json`) were re-baselined where the
new topology changed edge counts. See `D-113B-CONNECTJUNCTIONS` in
`docs/DEVIATION_LOG.md`.

### P113c — correction to 113b's incomplete quad/rib topology pass

P113b's quad/rib topology and faithful `connectJunctions` stitch (above)
were real improvements, but they were built on top of a graph whose
`next`/`prev`/`twin` pointers were still copied verbatim from the raw
per-cell `boostvoronoi` DCEL — topologically wrong for spine-walking at
junctions. This was confirmed as a systemic defect, not an edge case:
100% of outer-wall gcode segments failed to close (283/283, mean gap
18.7mm) on `resources/cube_4color.3mf`, reproduced even on a bare 10mm
square. `rib.rs`'s `build_quad_rib_topology` compounded the defect by
only inserting ribs at reflex corners, instead of after every transferred
edge as OrcaSlicer's real `constructFromPolygons`/`transferEdge`/`makeRib`
does.

P113c replaces graph construction itself with a faithful per-cell port of
that real algorithm (Steps 1-3), reworks `connectJunctions` in
`generate_toolpaths.rs` into a faithful quad-by-quad stitch over the new
topology (Step 4, also fixing a latent `emit_chain_lines` max-beads bug
that dropped inner-wall beads), re-validates centrality/bead-count/stitch/
simplify/remove-small against the new ubiquitous-rib topology (Steps 5, 7),
and finds + fixes two further DCEL-adjacent bugs in `propagation.rs`
beyond either prior packet's scope (Steps 6, 8b) via a new faithfulness
invariant suite (Step 8). This is what actually closes M2's structural
graph-construction parity claim — P113b's own "faithful connectJunctions"
framing above should be read as superseded by P113c for the
graph-construction layer specifically (the quad/rib classification and
centrality/bead-count/propagation adaptations it introduced remain valid
and are reused, not redone). See `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` in
`docs/DEVIATION_LOG.md`, which supersedes and closes-for-real both
`D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS`.

---

## Open decision points (must resolve before tasks marked `[blocked: D-N]`)

| ID | Decision | Default if unanswered |
|---|---|---|
| D-1 | ~~Wall-sequence ownership — perimeter module or `path-optimization-default`?~~ **CLOSED** by [ADR-0011](../adr/0011-perimeter-module-owns-wall-sequencing.md): perimeter module owns it. | |
| D-2 | ~~Gap-fill location — in `classic-perimeters` or a new `gap-fill` module?~~ **CLOSED:** in-module, emitted into `PerimeterRegion.walls` as `WallLoop { loop_type: GapFill, role: GapFill }`. No new IR field. Follows the existing `ThinWall` convention (T-062). | |
| D-3 | ~~Spiral-vase location — in `classic-perimeters` or a new `spiral-vase` finalization module?~~ **CLOSED:** spiral vase is a special-case configuration of the existing non-planar pipeline (SurfaceGroup + `LoopType::NonPlanarShell` + per-vertex Z within the layer Z envelope). It decomposes into a LayerPlanning extension (synthesise the surface group), the perimeter module's generic non-planar emission (D-11), and a `non-planar-walls` PerimetersPostProcess module for the helical Z modulation. **None of the spiral-vase-specific code is a perimeter-module concern.** Tracked as a sibling roadmap. | |
| D-11 | Non-planar wall emission scope — does this roadmap include emitting `LoopType::NonPlanarShell` walls when `region.nonplanar_surface.is_some()`? | Yes — include. Scope is "propagate upstream non-planar classification onto wall emission", same shape as T-020/T-021. Perimeter module reads `nonplanar_surface` and `surface_group.shell_count` and emits `LoopType::NonPlanarShell` walls with the right count. Per-vertex Z modulation is downstream (`non-planar-walls` module, separate workstream). |
| D-4 | Surface-classification view exposure for `extra_perimeters_on_overhangs` — extend `SliceRegionView` or add new `SurfaceClassificationView`? (Original quartile-derivation use case removed — see D-10.) | Extend `SliceRegionView` |
| D-10 | ~~Overhang-quartile per-vertex derivation owner~~ **CLOSED via sibling roadmap, consumer stack landed P107:** [`overhang-pipeline-restructuring.md`](./overhang-pipeline-restructuring.md) moves classification to PrePass via mesh cross-sections (more accurate than the current per-entity wall-distance algorithm), populates `Point3WithWidth.overhang_quartile` via perimeter-module propagation, and shrinks `overhang-classifier-default` to a speed-factor-only consumer. P107 delivered `SliceRegionView::overhang_quartile_polygons()`, the WIT `quartile-band` accessor, the host populator, and the refactored 78-LOC `overhang-classifier-default` (wall-distance `classify.rs`/`lines_distancer.rs` deleted per ADR-0031); this closed and resolved T-024's preconditions. Perimeter-side per-vertex propagation remains open, tracked as T-024-WIRE-VIEW-CONSUMER (see D-104-OVERHANG-QUARTILE-NONE). ADR-0008's "unnecessary scope" caveat re-examined under the new algorithm and use case. | |
| D-12 | ~~`OverhangRegion.xy_footprint` is missing~~ **CLOSED via sibling roadmap, resolved P107:** folded into [`overhang-pipeline-restructuring.md`](./overhang-pipeline-restructuring.md) Phase 1 (O-T010). Same workstream as overhang classification — single coherent PrePass-side overhang plumbing. P107's populator fills `overhang_areas` from the resulting `SurfaceClassificationIR.overhang_quartile_polygons`, keyed by `global_layer_index`. | |
| D-5 | ~~`extra_perimeters` plumbing — paint semantic, `RegionMapIR` overlay, or `SliceRegionView` accessor?~~ **CLOSED:** `RegionMapIR` overlay → `ConfigView`. `extra_perimeters` is a normal config key; per-region overrides flow through the existing RegionMapping pipeline; perimeter module reads via `_config.get("extra_perimeters")`. No view accessor, no paint semantic. Analysis-driven extras (`extra_perimeters_on_overhangs`) are a separate concern covered by T-077. | |
| D-6 | ~~`PerimeterRegion.walls` IR shape — flat list (current) or hole/contour tree (`parent_loop_index`)?~~ **CLOSED** by [ADR-0011](../adr/0011-perimeter-module-owns-wall-sequencing.md): flat list, final-print-order. Wall tree is in-module scaffolding only. | |
| D-7 | ~~Voronoi crate strategy — vendor `boost::polygon` port, adopt existing Rust crate, or write from scratch?~~ **CLOSED:** Adopt [`boostvoronoi`](https://docs.rs/boostvoronoi/) — pure-Rust port of `boost::polygon::voronoi`, matches OrcaSlicer's algorithm choice. Confirmed pre-grill. See [ADR-0023](../adr/0023-arachne-port-strategy.md) for the full rationale, BSL-1.0 license note, and degeneracy-handling strategy table. |
| D-8 | ~~`ExtrusionRole::GapFill` vs reuse `SparseInfill` + `is_thin_wall` flag?~~ **CLOSED:** add new `ExtrusionRole::GapFill` and `LoopType::GapFill` variants. Both enums marked `#[non_exhaustive]` if not already. Downstream consumers (`priority_for_role`, GCodeEmit, `part-cooling` fan dispatch, etc.) gain one match arm each. | |
| D-9 | ~~0-width-sentinel contract for `LimitedBeadingStrategy` — coordinate with all three infill modules, or post-process out of Arachne output before downstream sees it?~~ **CLOSED:** strip from external output. The infill-fill-partition plan now conveys the boundary information via `perimeter.infill_areas` polygon shape + host-side partition, so 0-width sentinels' cross-module-marker role is obviated. `LimitedBeadingStrategy`'s internal sentinel-insertion stays faithful for bead-count math; a strip-pass drops zero-width beads before `WallLoop` assembly. Documented as deliberate deviation in `docs/DEVIATION_LOG.md`. | |
| D-13 | ~~Bisector tie-break rule — match OrcaSlicer source byte-for-byte (cite line numbers), or default to lower color-ID owns when Orca's rule is opaque/non-deterministic? Authored by [ADR-0013](../adr/0013-mmu-per-color-outer-wall-fragmentation.md). Closure depends on T-P96-A0 investigation.~~ **CLOSED (P105, ADR-0013 Model A):** moot — the T-P96-A0 investigation ([orca-mmu-perimeter-investigation.md](./orca-mmu-perimeter-investigation.md)) found OrcaSlicer is partition-based with BOTH sides tracing the shared bisector; no tie-break rule exists or is needed. ADR-0013 was rewritten to Model A accordingly (D-105-MMU-MODEL-PIVOT). | (superseded draft answer: default to lower color-ID) |
| D-14 | ~~Carrier mechanism for bisector skip data — resurrect per-region `bisector_edge_skip_mask: Vec<bool>` (per-edge of `SlicedRegion.polygons.contour.points`), or expose via a new accessor that computes on-demand? Authored by [ADR-0013](../adr/0013-mmu-per-color-outer-wall-fragmentation.md).~~ **CLOSED (P105):** moot — Model A needs no skip data; the drafted mask, its host populator, WIT/SDK accessors, and `edge_offset_for_polygon` were fully removed (D-105-BISECTOR-MASK-DROPPED). | (superseded draft answer: resurrect the mask) |
| D-15 | ~~Arachne MMU dedup approach — per-edge wall mask (same as classic) or per-color boundary preprocessing (different mechanism, dedup before SkeletalTrapezoidation sees the cells)? Authored by [ADR-0013](../adr/0013-mmu-per-color-outer-wall-fragmentation.md). Closure depends on T-P96-A0 investigation citing OrcaSlicer's Arachne MMU path.~~ **CLOSED (P105):** per-color boundary preprocessing — consistent with Model A (each color's partition cells feed Arachne independently; per-edge wall masks are inapplicable since medial-axis walls don't map 1:1 onto cell edges, P96 worker proof). M2 implementation tracked as T-P96-E. | Per-color boundary preprocessing. |

---

## Inherited from P96 — AC-22b reshape obligation

P96 closed `cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one` via `SlicedRegion.external_contour` — a host-side `union_ex` of sibling painted regions that perimeter modules trace once per painted object, eliminating per-color outer-wall fragments. This is OrcaSlicer-divergent. OrcaSlicer's MMU emits per-color outer-wall fragments with tool changes at color transitions ("fragmentation" is the parity-correct behavior, not a defect).

**Authority:** [ADR-0013](../adr/0013-mmu-per-color-outer-wall-fragmentation.md) — supersedes `D-96-AC22-EXTERNAL-CONTOUR`. Three decisions to close before implementation begins: **D-13** (tie-break rule), **D-14** (carrier mechanism), **D-15** (Arachne approach). All three are gated on T-P96-A0 below.

Tasks fold into existing M1 phases (cross-references in `Phase`):

| ID | Title | Phase | Files | Acceptance |
|---|---|---|---|---|
| T-P96-A0 | OrcaSlicer-source investigation: audit MMU per-color outer-wall emission path. Cite `MultiMaterialSegmentation.cpp` and `PerimeterGenerator.cpp` per-color branches (line numbers). Document Orca's bisector tie-break rule (D-13) and Arachne MMU input-contour preprocessing path (D-15). Produces a one-pager under `docs/specs/orca-mmu-perimeter-investigation.md` that the implementation tasks cite. | Phase 0 | `docs/specs/orca-mmu-perimeter-investigation.md` (new) | One-pager committed. Tie-break rule named with file+line citation OR explicitly stated as "no deterministic rule found, defaulting to lower color-ID per ADR-0013". Arachne path described with file+line citation. D-13 and D-15 entries in this roadmap updated to **CLOSED** with the investigation's findings. |
| T-P96-A | Reshape AC-22b assertion from union-baseline to per-color fragmentation | Phase 9 | `crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs` | Test renamed to `cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes`. Assertions per painted layer: (1) count of distinct outer-wall extrusion sequences ≈ N distinct colors present; (2) union of all outer-wall extrusions exactly covers the layer's external perimeter (no gaps, no double-traces, ε-tolerance); (3) each fragment preceded by `T<N>` matching its color region's `ToolIndex`; (4) color transitions occur at cell-boundary corners within geometric tolerance. RED at write time. |
| T-P96-B | Revert `external_contour` consumption in classic-perimeters and arachne-perimeters | Phase 1 / Phase 2 | `modules/core-modules/classic-perimeters/src/lib.rs:111`, `modules/core-modules/arachne-perimeters/src/lib.rs:136` | Both modules trace outer walls per-cell again for painted SlicedRegions. `SlicedRegion.external_contour` IR field remains in place (harmless plumbing) but is unused; T-P96-D deletes it after green. Test (T-P96-A) stays RED with a different failure mode — bisectors traced twice. |
| ~~T-P96-C0~~ | **DROPPED** — Model A (ADR-0013 rewrite, P105) established that adjacent per-color outer walls are parallel and ~one line-width apart (each color offsets half-width inward from its own side of the bisector) — they never coincide, so there is nothing to deduplicate. No `bisector_edge_skip_mask` field, host populator, WIT accessor, or view accessor is introduced. See D-105-BISECTOR-MASK-DROPPED. | — | — |
| ~~T-P96-C1~~ | **DROPPED** — No `bisector_edge_skip_mask` exists in Model A (see T-P96-C0 drop reason above and D-105-BISECTOR-MASK-DROPPED). Classic-perimeters already traces each per-color region independently from its full contour, which is the correct Model A behavior (ADR-0013). No skip-edge logic is needed or introduced. | — | — |
| ~~T-P96-C2~~ | **DROPPED** — `variable-width-perimeters` is deleted (not renamed) under P108 per D-110-DROP-VARIABLE-WIDTH. The fake-Arachne module never ships and never consumes the mask. T-P96-A coverage for real Arachne is handled by T-P96-E in M2. | — | — |
| T-P96-C3 | Parity verification: golden-file check of full `cube_4color` G-code output against a recorded OrcaSlicer reference (tolerances per parity-harness pattern in T-100). | Phase 9 | `crates/slicer-runtime/tests/fixtures/perimeter_parity/cube_4color_orca.gcode` (recorded), `crates/slicer-runtime/tests/integration/perimeter_parity.rs` (harness extension) | Per-color fragment counts, tool-change positions, and wall-coverage union match Orca within tolerance. Investigation citation from T-P96-A0 referenced in test comment. |
| T-P96-D | Delete unused `external_contour` IR field after T-P96-A through T-P96-C3 land GREEN | Phase 1 | `crates/slicer-ir/src/slice_ir.rs:1282`, WIT, host populator, ~5 files | Field removed; `cargo check --workspace --all-targets` clean. SliceIR schema version bump. Cleanup task — strictly after C3. |
| T-P96-E | **DONE (2026-07-03)** — implemented as a validated pass-through per ADR-0013 current doctrine + OrcaSlicer canonical source verification; no tie-break mechanism needed, see closure-log.md for citations. Direct verification against ADR-0013 (`docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md`, current doctrine: "no skip mask, no per-edge ownership, no tie-break rule") and OrcaSlicer's canonical C++ source (`PerimeterGenerator.cpp:2600-2653`, `WallToolPaths.hpp`) confirmed real Arachne performs NO per-color contraction — per-color boundary isolation happens entirely upstream (layer/region segmentation), and Arachne is color-blind. Shipped `preprocess_per_color_inputs(painted_cells: &[(ToolIndex, Vec<ExPolygon>)]) -> Vec<(ToolIndex, Vec<ExPolygon>)>` is a validated pass-through: each color's cell boundary passes through unmodified; the function validates the non-overlap invariant and logs a warning (does not silently fix) if violated. ~~Arachne MMU dedup at boundary level (NOT per-edge wall mask). Preprocessing of per-color input contour before SkeletalTrapezoidation: each color's input cell has bisector edges with neighboring different-color cells contracted/removed per the tie-break rule. The result is per-color preprocessed input cells that Arachne ingests normally.~~ | Phase 10–12 (M2) | `modules/core-modules/arachne-perimeters/` (M2 real-Arachne module), `crates/slicer-core/src/arachne/preprocess.rs` | Per OrcaSlicer Arachne MMU citation from T-P96-A0. Cube_4color parity test (T-P96-C3) passes for Arachne. |
| T-P96-F | Re-baseline cube_4color SHA + add deviation entry | Phase 9 | `.ralph/specs/<packet>/closure-log.md`, `docs/DEVIATION_LOG.md` | Capture `P<packet>_CUBE_4COLOR_PARITY_SHA`. Add `D-<packet>-AC22-PARITY-RESHAPE` superseding D-96-AC22-EXTERNAL-CONTOUR. Cross-reference ADR-0013. |

Ordering (updated for Model A, P105):
1. **T-P96-A0** first — produces the investigation that closes D-13, D-14, D-15 and grounds the implementation. Independent of any other roadmap task.
2. **T-P96-A** lands the test RED.
3. **T-P96-B** reverts `external_contour` consumption (test goes GREEN immediately — classic-perimeters already traces per-color independently; Model A needs no bisector mask).
4. ~~**T-P96-C0**~~ / ~~**T-P96-C1**~~ / ~~**T-P96-C2**~~ — ALL DROPPED per D-105-BISECTOR-MASK-DROPPED and ADR-0013 rewrite. No skip mask is introduced; each color traces its full contour independently.
5. **T-P96-C3** parity verification against recorded OrcaSlicer output.
6. **T-P96-D** cleanup (delete `external_contour`).
7. **T-P96-E** in M2 alongside real Arachne.
8. **T-P96-F** at packet close — deviation supersession.

---

## Inherited from P98 — paint_seam stroke consumption obligation

P98 (loader paint-channel symmetry) made the 3MF loader decode `paint_seam` sub-facet strokes for all four channels. Those strokes now flow through `host:paint_segmentation` into `SlicedRegion.variant_chain` as `("seam_enforcer", _)` / `("seam_blocker", _)` entries — but **no live module reads them** (registered `D-98-SEAM-NO-CONSUMER`). `seam-placer` selects seams from geometric `SeamCandidate` scores computed by the perimeter generators, not from paint annotations. P98 makes seam paint *available*; this roadmap must wire the *consumer*.

This obligation folds into Phase 8 (Seam-candidate quality):

| ID | Title | Phase | Files | Acceptance |
|---|---|---|---|---|
| T-P98-SEAM | Consume painted seam_enforcer/seam_blocker in seam-candidate generation | Phase 8 | `crates/slicer-core/src/perimeter_utils.rs` (`generate_seam_candidates`), `modules/core-modules/seam-placer/src/lib.rs` | Painted `seam_enforcer` regions bias seam-candidate selection toward enclosed perimeter vertices; painted `seam_blocker` regions exclude enclosed vertices from candidacy. TDD on a fixture carrying both channels (e.g. `resources/cube_cilindrical_modifier.3mf`, which carries `seam_enforcer` strokes): a vertex inside a seam_enforcer region is chosen as the seam over a sharper-corner candidate outside it; a vertex inside a seam_blocker region is never chosen. Supersede `D-98-SEAM-NO-CONSUMER` with `D-<packet>-SEAM-CONSUMED` at close. |

Note: T-082/T-083 already audit seam-placer's candidate-list contract and the seam-planner interaction; T-P98-SEAM is the concrete paint→candidate wiring those audits feed into. Until it lands, painted seams are decoded and carried but have no effect on seam placement (production impact: `paint_seam` in 3MFs is silently inert).

---

## Inherited from P107 — overhang pipeline consumer follow-ups

Packet 107 (overhang-pipeline-consumers-and-refactor) delivered the consumer stack for `overhang-pipeline-restructuring.md` Phase 3: `SliceRegionView::overhang_quartile_polygons() -> &[QuartileBand]` (SDK), the WIT `quartile-band` record + accessor, the host populator (`overhang_areas` + `overhang_quartile_polygons`, keyed by `global_layer_index`), and a refactored `overhang-classifier-default` (78 LOC, reads per-vertex `Point3WithWidth.overhang_quartile`, deletes the wall-distance `classify.rs`/`lines_distancer.rs` per ADR-0031). This closes D-10 and D-12; T-024 and T-077 are unblocked (their preconditions are satisfied). Two gaps remained, registered as follow-up tasks — both now closed:

| ID | Title | Phase | Files | Acceptance |
|---|---|---|---|---|
| T-024-WIRE-VIEW-CONSUMER | **CLOSED 2026-07-03.** Perimeter generation still writes `overhang_quartile = None` (P104 placeholder, `crates/slicer-core/src/perimeter_utils.rs:311`) — wire `classic-perimeters`/`arachne-perimeters` to read `SliceRegionView::overhang_quartile_polygons()` and write per-vertex `Some(quartile)` via point-in-polygon test, completing T-024. Landed for `classic-perimeters` only — `arachne-perimeters` doesn't exist (deleted P108; real Arachne is `status: draft`, packets 110-112, not started). | Phase 2 | `modules/core-modules/classic-perimeters/src/lib.rs`, `crates/slicer-core/src/perimeter_utils.rs` | Overhang-ramp fixture: vertices in flagged quartile band carry expected quartile value. The production-coupled tripwire test `overhang_pipeline_e2e_tdd.rs` went green; `overhang_pipeline_full_propagation` (un-`#[ignore]`d) now passes for real, and the old `overhang_pipeline_partial_state_quartile_none` gap test was retired per its own embedded instructions. |
| T-077-GUEST-ADAPTER-FIELDS | **CLOSED 2026-07-03.** The guest-side macro adapter (`crates/slicer-macros` `__slicer_adapt_slice_regions`) does not map `overhang_areas`/`overhang_quartile_polygons`/`surface_group` into guest `SliceRegionView`s — only host-side/native consumers can see them today. Extend the adapter so real WASM Layer modules (e.g. `classic-perimeters` for T-077) can read these fields. | Phase 2 | `crates/slicer-macros/src/lib.rs` | Guest-side TDD: a WASM Layer module reading `overhang_areas()`/`overhang_quartile_polygons()` via the macro-generated adapter observes the same values as the native/host path. Verified transitively via `overhang_pipeline_full_propagation` and the guest-WASM rebuild (`cargo xtask build-guests`). |

## Inherited from P105 — spacing-model follow-up

| ID | Title | Phase | Files | Acceptance |
|---|---|---|---|---|
| T-052-WIRE-FLOW | `slicer_core::flow::line_width_to_spacing` (OrcaSlicer `Flow::new_from_width_height` spacing) is implemented + unit-tested but has no production caller — both perimeter modules compute the wall gap as the inline width-average `(outer+inner)/2` (D-105-FLOW-NOT-WIRED, Accepted/Open). Wire `flow` into `classic-perimeters` (and real Arachne when P110+ lands) and reshape P105 AC-1 from the width-average value to true `ext_perimeter_spacing2` parity. The original deferral target ("P106's `extra_perimeters` consumer") was invalidated by the 104–112 packet renumber; the consumer shipped in P108 without the wiring. | M2 (or the first spacing-touching packet) | `crates/slicer-core/src/flow.rs`, `modules/core-modules/classic-perimeters/src/lib.rs`, `crates/slicer-runtime/tests/integration/outer_inner_width_and_spacing_tdd` | AC-1 reshaped: the outer↔first-inner gap equals OrcaSlicer `ext_perimeter_spacing2` computed via `flow::line_width_to_spacing`, not the width-average 0.45 mm; `flow.rs` gains a production caller; D-105-FLOW-NOT-WIRED closes. |

---

# M1 — Classic parity + truthful rename

## Phase 0 — Truth in advertising

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-001 | Rewrite `classic-perimeters/src/lib.rs` doc-comment to match actual scope | `modules/core-modules/classic-perimeters/src/lib.rs` | Doc-comment removes "Per OrcaSlicer process_classic()" claim until parity is real; lists feature deltas with target task IDs. |
| T-002 | Rewrite `arachne-perimeters/src/lib.rs` doc-comment to state "iterative-inset width approximation, BeadingStrategy stack not implemented" | `modules/core-modules/arachne-perimeters/src/lib.rs` | Doc-comment is honest about algorithm. |
| T-003 | Register every audit-finding gap in `docs/DEVIATION_LOG.md` with target-task IDs | `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md` | One entry per gap, linked to a T-NNN. |
| T-004 | Add ADR `0009-perimeter-module-scope.md` defining the responsibility boundary between perimeter modules and downstream consumers | `docs/adr/0009-perimeter-module-scope.md` | Documents the table from "Architectural framing" above as binding. |
| T-005 | Declare symmetric `incompatible-with` between classic and arachne manifests | `modules/core-modules/{classic,arachne}-perimeters/*.toml` | Each manifest references the other in `incompatible-with`. |

## Phase 1 — Cross-cutting foundations

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-010 | Create `slicer-perimeter-utils` (new sub-module under `slicer-core` or standalone crate — pick at task time) | `crates/slicer-core/src/perimeter_utils.rs` or new crate | Public API surface: `build_outer_wall_flags`, `has_adjacent_material_change`, `find_adjacent_tool`, `extract_tool_index`, `default_feature_flags`, `expolygon_to_path3d`, `BASE_SPEED`. |
| T-011 | Migrate `classic-perimeters` to consume `slicer-perimeter-utils`; delete the duplicated definitions | `modules/core-modules/classic-perimeters/src/lib.rs` | Module no longer defines these symbols locally; tests still green. |
| T-012 | Migrate `arachne-perimeters` to consume `slicer-perimeter-utils`; delete the duplicated definitions | `modules/core-modules/arachne-perimeters/src/lib.rs` | Same as T-011. ≥160 LOC removed across both modules. |
| T-013 | Widen `WallBoundaryType::MaterialBoundary` to `Vec<MaterialBoundarySegment { point_range, near_tool, far_tool }>` | `crates/slicer-ir/src/slice_ir.rs`, schema version bump | New struct compiles and serialises; old data round-trips through a migration adapter. |
| T-014 | Update `build_outer_wall_flags` to emit the full transition list (not just first adjacent tool) | `crates/slicer-core/src/perimeter_utils.rs` (or wherever T-010 placed it) | 3-tool triangle TDD passes; all transitions captured. |
| T-015 | Plumb `LayerOverrides` into both modules' `run_perimeters` via the unused `_config` parameter | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs`, `crates/slicer-sdk/src/traits.rs` | `line_width`, `wall_count`, speeds re-resolved per-layer; new TDD asserts layer-0 vs layer-5 differs when overridden. |
| T-016 | Replace every `let _ = output.<fn>(…)` with `?` propagation in both modules | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | No remaining swallowed `Result`. |
| T-017 | Document `PerimeterOutputBuilder` failure modes (capacity, contract violation) in `docs/05_module_sdk.md` and add a negative-path TDD | `docs/05_module_sdk.md`, `modules/core-modules/classic-perimeters/tests/*` | Failure-mode contract documented; TDD passes. |
| T-018 | Reconcile manifest vs code defaults for `wall_count`, `outer_wall_speed`, `inner_wall_speed` | `modules/core-modules/{classic,arachne}-perimeters/*.toml`, `src/lib.rs` | Single source of truth (manifest); code fallback matches manifest. |
| T-019 | Read `_paint: &PaintRegionLayerView` in both modules (currently unused); document why if intentionally unread | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | Either consumed or explicitly documented as intentionally unused with rationale in the doc-comment. |

## Phase 2 — Upstream-data propagation into per-vertex flags

**Theme.** Data already exists upstream — bridge_areas, top/bottom shell index, overhang regions. The perimeter module currently hardcodes the corresponding `WallFeatureFlags` fields to defaults. These tasks read what's already there.

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-020 | Per-vertex `is_bridge` from `region.bridge_areas()` containment | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs`, `crates/slicer-core/src/perimeter_utils.rs` | For each wall vertex, point-in-polygon test against `bridge_areas`. Bridge-fixture TDD asserts only covered vertices flagged. |
| T-021 | Per-vertex `tool_index` propagated to **inner** walls (not just outer) when material boundary exists | `crates/slicer-core/src/perimeter_utils.rs` (shared `build_wall_flags`) | Inner-wall TDD: 2-tool fixture → inner walls carry `MaterialBoundary` where adjacent. |
| T-022 | Drop hardcoded `WallBoundaryType::Interior` for inner walls; compute boundary_type via same logic as outer | `crates/slicer-core/src/perimeter_utils.rs` | Same TDD as T-021. |
| T-023 | `[blocked: D-4]` Expose `OverhangRegion` lookup on per-layer-per-region view — scoped to `extra_perimeters_on_overhangs` (T-074-new) only, not quartile derivation | `crates/slicer-sdk/src/views.rs`, `crates/slicer-sdk/src/traits.rs` | View accessor returns per-vertex-resolvable overhang regions for the current layer/object. |
| T-024 | **CLOSED 2026-07-03 (T-024-WIRE-VIEW-CONSUMER).** `classic-perimeters` reads `region.overhang_quartile_polygons()` and `expolygon_to_path3d` (`crates/slicer-core/src/perimeter_utils.rs`) propagates per-vertex `Point3WithWidth.overhang_quartile` via `slicer_ir::point_in_polygon_winding`, MAX-quartile-on-multi-match, mirroring T-020's `is_bridge` pattern. Threaded through all 5 `emit_walls` call sites; `emit_nonplanar_shells` intentionally excluded (D-3). Verified end-to-end via `overhang_pipeline_full_propagation` (`crates/slicer-runtime/tests/integration/overhang_pipeline_e2e_tdd.rs`). Only `classic-perimeters` exists as a live target — `arachne-perimeters`/`variable-width-perimeters` was deleted under P108; real Arachne (packets 110-112) is `status: draft`, not started. | `modules/core-modules/classic-perimeters/src/lib.rs`, `crates/slicer-core/src/perimeter_utils.rs` | Overhang-ramp fixture: vertices in flagged quartile band carry expected quartile value; vertices outside overhang regions carry `None`. |
| T-025 | Per-vertex `flow_factor` plumbing (read from config / per-region overrides if applicable) | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | If no per-region flow compensation exists yet, document the field as "currently always 1.0; will be set when flow-compensation lands". Don't silently hardcode. |

## Phase 3 — Surface-driven wall-count rules

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-030 | Register `only_one_wall_top` config key in `docs/15_config_keys_reference.md` | `docs/15_config_keys_reference.md`, both `.toml` manifests | Key documented; manifest schema entries added. |
| T-031 | Read `region.top_shell_index() == Some(0)` and `only_one_wall_top == true`; force `wall_count = 1` for that region | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | Top-flagged TDD: `only_one_wall_top=true` → 1 wall; `false` → full count. |
| T-032 | Register `only_one_wall_first_layer` config key in `docs/15_config_keys_reference.md` | `docs/15_config_keys_reference.md`, both `.toml` manifests | Documented + manifested. |
| T-033 | Read `_layer_index == 0` and `only_one_wall_first_layer == true`; force `wall_count = 1` | `modules/core-modules/{classic,arachne}-perimeters/src/lib.rs` | First-layer TDD passes. |

## Phase 4 — `slicer-core` polygon-op primitives

**Theme.** These primitives are dual-use (Classic Phase 5-6 and Arachne Phase 10 pre-processing). Done now to unblock Classic.

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-040 | Port `offset2_ex(polys, -d, +d)` and `opening_ex(polys, d)` to `slicer-core` | `crates/slicer-core/src/polygon_ops.rs` | Output matches OrcaSlicer golden fixture for canonical polygons. |
| T-041 | Port `ExPolygon::medial_axis(min_width, max_width, &out)` to `slicer-core` | `crates/slicer-core/src/medial_axis.rs` | Wedge-fixture golden test matches OrcaSlicer within tolerance. |
| T-042 | Add `ThickPolyline` and `Point2WithWidth` IR types; `variable_width()` converter to `Vec<Point3WithWidth>` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-core/src/medial_axis.rs` | Round-trip TDD: ThickPolyline → variable-width path → ThickPolyline preserves widths. |
| T-043 | Port hole/contour containment + tree-builder (`PerimeterGeneratorLoop` analogue) to `slicer-core` | `crates/slicer-core/src/polygon_tree.rs` | Tree structure matches OrcaSlicer golden fixture for nested-hole polygon. |
| T-044 | Port `keep_largest_contour_only` helper (used by spiral-vase) | `crates/slicer-core/src/polygon_ops.rs` | Multi-polygon input → single-polygon output (largest by area). |
| T-045 | Promote `ray_to_polygons`, `nearest_point_on_polygons`, `point_to_segment_nearest` from `arachne-perimeters` to `slicer-core` | `crates/slicer-core/src/geometry.rs`, `modules/core-modules/arachne-perimeters/src/lib.rs` | Module no longer defines these; tests still green. |

## Phase 5 — Classic spacing model

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-050 | Port minimal `Flow::new_from_width_height(width, layer_height, nozzle_diameter)` math (width→spacing conversion) to `slicer-core` | `crates/slicer-core/src/flow.rs` | Unit tests against OrcaSlicer reference table. |
| T-051 | Replace single `line_width` field in `classic-perimeters` with `outer_wall_line_width` + `inner_wall_line_width` (+ `smaller_perimeter_line_width` reserved) | `modules/core-modules/classic-perimeters/src/lib.rs`, `.toml` | Outer wall renders at outer width, inner at inner. Manifest keys registered in `docs/15_config_keys_reference.md`. |
| T-052 | Implement `ext_perimeter_spacing2` (outer↔first-inner) vs `perimeter_spacing` (inner↔inner) arithmetic from `PerimeterGenerator.cpp:1501-1506, 1644` | `modules/core-modules/classic-perimeters/src/lib.rs` | Golden fixture asserts spacing-between-loops at expected values. |
| T-053 | Register and implement `precise_outer_wall` mode (gated on `wall_sequence == InnerOuter`) | `modules/core-modules/classic-perimeters/{src/lib.rs,classic-perimeters.toml}`, `docs/15_config_keys_reference.md` | Mode active only under correct wall-sequence gate; outer-wall spacing arithmetic adjusts per Orca. |
| T-054 | Register `wall_sequence` enum (`OuterInner` / `InnerOuter` / `InnerOuterInner`) in perimeter manifests; deregister from `path-optimization-default` per [ADR-0011](../adr/0011-perimeter-module-owns-wall-sequencing.md) | `docs/15_config_keys_reference.md`, both perimeter `.toml` manifests, `modules/core-modules/path-optimization-default/path-optimization-default.toml` | Key registered on perimeter modules only; `path-optimization-default` no longer declares it; startup validation rejects unknown reads. |
| T-054b | Implement `OuterInner` and `InnerOuter` modes in `slicer-perimeter-utils::wall_sequence_reorder` | `crates/slicer-core/src/perimeter_utils/wall_sequence.rs`, `modules/core-modules/{classic,variable-width}-perimeters/src/lib.rs` | OuterInner reverses entity order; InnerOuter is canonical. TDD: each mode produces expected sequence on a 3-wall fixture. |
| T-054c | Implement `InnerOuterInner` sandwich mode (per-outer-contour grouping using in-module wall tree) | `crates/slicer-core/src/perimeter_utils/wall_sequence.rs` | Multi-island fixture: each island's loops interleave correctly; cross-island loops are not interleaved. TDD assertions match Orca's `process_classic()` lines 1801–1913. |

## Phase 6 — Thin-walls + gap-fill

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-060 | Register `detect_thin_wall` config key | `docs/15_config_keys_reference.md`, `classic-perimeters.toml` | Documented + manifested. |
| T-061 | Implement thin-wall detection cascade (`offset2_ex` + `opening_ex` + `medial_axis`) from `PerimeterGenerator.cpp:1596-1609` | `modules/core-modules/classic-perimeters/src/lib.rs` | Thin-protrusion fixture produces ThinWall geometry. |
| T-062 | Emit ThinWall geometry as `WallLoop { loop_type: ThinWall, role: ThinWall, is_thin_wall: true }` with width profile from `ThickPolyline` | `modules/core-modules/classic-perimeters/src/lib.rs` | ThinWall loop visible in `PerimeterOutputBuilder`; widths variable. |
| T-062b | Add `LoopType::GapFill` and `ExtrusionRole::GapFill` variants; ensure both enums are `#[non_exhaustive]`; add match arms in `priority_for_role`, GCodeEmit, `part-cooling`, any other role-switching consumer | `crates/slicer-ir/src/slice_ir.rs`, `modules/core-modules/{part-cooling,machine-gcode-emit}/src/lib.rs`, host GCodeEmit | Enums compile; downstream consumers handle new variants without warnings. |
| T-063 | Implement gap collection per-inset: `diff_ex(offset(last, -0.5d), offset(offsets, 0.5d+safety))` from `PerimeterGenerator.cpp:1665-1670` | `modules/core-modules/classic-perimeters/src/lib.rs` | Notched-square fixture: gaps detected between perimeter and infill region. |
| T-064 | Run `medial_axis` over collected gaps; filter by `filter_out_gap_fill` length threshold; emit as `WallLoop { loop_type: GapFill, role: GapFill, path: variable-width from ThickPolyline }` inside `PerimeterRegion.walls` | `modules/core-modules/classic-perimeters/src/lib.rs` | Gap-fill `WallLoop`s present in `walls`; widths variable; integrated with downstream extrusion entity assembly. |
| T-065 | Register `gap_infill_speed` and `filter_out_gap_fill` config keys | `docs/15_config_keys_reference.md`, `classic-perimeters.toml` (or new `gap-fill.toml`) | Documented + manifested. |

## Phase 7 — Classic special modes

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-070 | Register `extra_perimeters` config key; ensure it's in `ResolvedConfig` and flows through `RegionMapIR` → `ConfigView` per D-5 | `docs/15_config_keys_reference.md`, `classic-perimeters.toml`, `crates/slicer-ir/src/slice_ir.rs` (`ResolvedConfig` if missing) | Key registered; per-region override via existing `RegionMapIR` mechanism produces correct `ConfigView` reading. |
| T-071 | Honour `extra_perimeters` config bonus: `loop_number = wall_count + _config.get("extra_perimeters") - 1` (Orca line 1569) | `modules/core-modules/classic-perimeters/src/lib.rs` | Region-overridden TDD: extra=2 → 2 extra loops above base wall_count. |
| T-072 | Register `smaller_perimeter_line_width`, `smaller_perimeter_threshold_mm`, `narrow_loop_length_threshold_mm` config keys | `docs/15_config_keys_reference.md`, `classic-perimeters.toml` | Documented + manifested. |
| T-073 | Implement narrow-island handling: islands < threshold use `smaller_ext_perimeter_flow` (Orca lines 1611-1628) | `modules/core-modules/classic-perimeters/src/lib.rs` | Long-narrow-strip TDD: narrow island uses smaller width. |
| ~~T-074~~ | **OUT OF SCOPE** per D-3 closure: spiral-vase-specific code is not a perimeter-module concern. Tracked in a sibling roadmap (`docs/specs/spiral-vase-and-non-planar-pipeline.md`, to be authored separately). | — | — |
| ~~T-075~~ | **OUT OF SCOPE** per D-3 closure: `spiral_vase` config key belongs to LayerPlanning's manifest (it drives surface-group synthesis there), not perimeter. Tracked in sibling roadmap. | — | — |
| T-074b | Per D-11: detect non-planar regions via `region.nonplanar_surface.is_some()`; branch wall generation to emit `LoopType::NonPlanarShell` walls instead of `Outer`/`Inner` | `modules/core-modules/{classic,variable-width}-perimeters/src/lib.rs` | Non-planar fixture: walls in flagged regions carry `LoopType::NonPlanarShell`; planar walls unaffected. |
| T-074c | Read `SurfaceGroup.shell_count` from the Blackboard; override `wall_count` accordingly for non-planar regions | (requires Blackboard / `SurfaceClassificationView` read — coordinate with D-4 view extension) | TDD: non-planar region with `shell_count=3` produces 3 walls regardless of config `wall_count`. |
| T-074d | Skip thin-wall, gap-fill, and `infill_areas` emission for non-planar regions (the surface-group sweep is the only geometry produced) | `modules/core-modules/{classic,variable-width}-perimeters/src/lib.rs` | TDD: non-planar region produces no ThinWall, no GapFill, no infill_areas. Documented in `docs/01_system_architecture.md` non-planar section. |
| ~~T-076~~ | **SUPERSEDED** by T-054b + T-054c (moved to Phase 5 because reordering is tightly coupled with the spacing model that produces the wall tree). | — | — |
| T-077 | **UNBLOCKED (P107) — precondition met.** `[blocked: D-4, precondition: overhang-pipeline-restructuring Phase 3]` — the Phase 3 precondition is satisfied: P107 populates `SliceRegionView::overhang_areas()` (added by sibling roadmap O-T030) from `SurfaceClassificationIR.overhang_quartile_polygons` via the host populator. Register `extra_perimeters_on_overhangs` config key; implement extra-perimeter generation in regions covered by `overhang_areas()`. Note: the guest-side macro adapter does not yet map `overhang_areas`/`overhang_quartile_polygons`/`surface_group` into guest `SliceRegionView`s (host/native consumers only today) — see follow-up **T-077-GUEST-ADAPTER-FIELDS** below, needed before this task's real-WASM module can consume the accessor. | `docs/15_config_keys_reference.md`, `classic-perimeters.toml`, `modules/core-modules/classic-perimeters/src/lib.rs` | Overhang-ramp fixture: when enabled, overhang region carries N+1 walls vs N elsewhere. |

## Phase 8 — Seam-candidate quality

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-080 | Replace every-vertex-candidate heuristic with sharp-corner threshold (config key `seam_candidate_angle_threshold_deg`, default ≈30°) | `crates/slicer-core/src/perimeter_utils.rs` (the shared `generate_seam_candidates`) | Square-fixture TDD: 4 candidates (one per corner), not N=hundreds. |
| T-081 | Register `seam_candidate_angle_threshold_deg` config key | `docs/15_config_keys_reference.md`, both `.toml` manifests | Documented + manifested. |
| T-082 | Audit `seam-placer/src/lib.rs` for any dependency on dense candidate lists; document in roadmap if downstream contract requires changes | `modules/core-modules/seam-placer/src/lib.rs` (read-only) | Either confirms no change needed, or files a task in this roadmap to update seam-placer in tandem. |
| T-083 | Confirm/document interaction with `seam-planner-default`: does its `PrePass::SeamPlanning` output feed perimeter-side candidate generation? | `modules/core-modules/seam-planner-default/src/lib.rs` (read), `docs/01_system_architecture.md` (update if needed) | Documented decision: either perimeter consumes seam-planner output, or the two are independent. |

## ~~Variable-width-perimeters rename~~ → DELETE fake-Arachne under P108 (D-110-DROP-VARIABLE-WIDTH)

Tasks T-090/T-091/T-092 are **cancelled** (rename never happens). Re-scoped to deletion, owned by **P108**:

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-090 | ~~Rename `arachne-perimeters` directory~~  **→ DELETE** `arachne-perimeters` directory, crate, workspace member, and `.wasm` artifact | `modules/core-modules/arachne-perimeters/` (entire dir removed); `Cargo.toml` workspace member entry removed; `modules/core-modules/` manifest dir | `cargo build --workspace` green; `cargo xtask build-guests` no longer discovers `arachne-perimeters`. |
| T-091 | ~~Update manifest~~ **→ Remove** all `arachne-perimeters` manifest and module-dir references from `docs/`, `.ralph/specs/`, config-key reference | `modules/core-modules/arachne-perimeters/` (deleted above); `docs/**/*.md`, `.ralph/specs/**/*.md` | No remaining reference to the deleted module id `com.core.arachne-perimeters` outside of deletion-noting entries. |
| T-092 | Remove all doc and spec stale forward-refs to `variable-width-perimeters` | `docs/**/*.md`, `.ralph/specs/**/*.md` | `rg variable-width-perimeters docs/ .ralph/specs/` returns zero hits outside of D-110-DROP-VARIABLE-WIDTH and deletion-noting entries. |

## Phase 9 — Verification

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-100 | Build reference-fixture parity harness under `crates/slicer-runtime/tests/integration/perimeter_parity.rs` | new test file | Harness loads a `(mesh, config, expected-`PerimeterIR`)` triple and runs the perimeter module. |
| T-101 | Record OrcaSlicer reference outputs for 6 M1 fixtures: solid square, holed square, multi-tool triangle, overhang ramp, bridge fixture, spiral-vase cone | `crates/slicer-runtime/tests/fixtures/perimeter_parity/` | Reference files committed; tolerances calibrated. |
| T-102 | TDD sweep for edge cases called out in audit: 3-tool polygon, inner-wall material boundary, 0/2-vertex polygon, hole-with-thin-wall, gap-fill-in-overhang, top-flagged region, first-layer override | `modules/core-modules/classic-perimeters/tests/` | ≥20 new TDDs green. (`variable-width-perimeters/tests/` dropped — module deleted under P108 per D-110-DROP-VARIABLE-WIDTH.) |
| T-103 | Walk every M1 deviation entry from T-003; close each with implementing task ID, or document residual deviation | `docs/DEVIATION_LOG.md` | All M1 deviations closed or justified. |
| T-104 | Update `docs/07_implementation_status.md` to mark Classic parity complete | `docs/07_implementation_status.md` | Status entry added. |
| T-105 | Run `cargo test --workspace` once at M1 close (per CLAUDE.md test-discipline closure ceremony rule) | n/a (test run) | Green. |

---

# M2 — Real Arachne

## Phase 10 — Foundations

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-200 | **DONE (2026-07-03).** ADR `0023-arachne-port-strategy.md`: document Voronoi crate selection (D-7), pure-Rust constraints, degeneracy handling expectations | `docs/adr/0023-arachne-port-strategy.md` | ADR merged; D-7 closed. |
| T-201 | **DONE (2026-07-03).** Vendor / depend on chosen Voronoi crate; wrap in `slicer-core::voronoi` with Orca-shaped API surface | `crates/slicer-core/src/voronoi.rs`, `Cargo.toml` | API surface: `voronoi_from_segments(Vec<Segment>) -> HalfEdgeGraph`. Collinear/T-junction stress fixtures pass. |
| T-202 | **DONE (2026-07-03).** Port `SkeletalTrapezoidationGraph` (half-edge graph storing R-values per edge) | `crates/slicer-core/src/skeletal_trapezoidation/graph.rs` | Graph reproduces Orca's graph for square + wedge golden fixtures. |
| T-203 | **DONE (2026-07-03).** Discretize parabolic VD edges to line segments | `crates/slicer-core/src/skeletal_trapezoidation/discretize.rs` | Output matches OrcaSlicer discretized graph within tolerance. |
| T-204 | **DONE (2026-07-03).** Port the 9-stage pre-processing pipeline from `WallToolPaths.cpp:590-604` (triple-offset, simplify, fixSelfIntersections, removeSmallAreas, etc.) | `crates/slicer-core/src/arachne/preprocess.rs` | Output matches Orca's pre-processed-outline fixture. Hazard ("destroys features < epsilon_offset ~11.5 µm") documented in doc-comment. |
| T-205 | **DONE (2026-07-03).** Create new `modules/core-modules/arachne-perimeters/` skeleton with manifest + empty `LayerModule` impl | `modules/core-modules/arachne-perimeters/` | Module loads under `com.core.arachne-perimeters`; `incompatible-with` declares `com.core.classic-perimeters` only (`variable-width-perimeters` is deleted by P108, per D-110-DROP-VARIABLE-WIDTH). |

## Phase 11 — BeadingStrategy stack

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-210 | **DONE (2026-07-03).** Define `BeadingStrategy` trait in `slicer-core::beading` (`compute`, `optimal_bead_count`, `get_transition_thickness`, etc.) | `crates/slicer-core/src/beading/mod.rs` | Trait covers all 5 strategies' surface. |
| T-211 | **DONE (2026-07-03).** Port `DistributedBeadingStrategy` (Gaussian-weighted width distribution) | `crates/slicer-core/src/beading/distributed.rs` | Reference Beading output matches Orca for 10 thickness inputs. |
| T-212 | **DONE (2026-07-03).** Port `RedistributeBeadingStrategy` (preserve outer-wall width consistency) | `crates/slicer-core/src/beading/redistribute.rs` | Reference Beadings match Orca on outer-consistent fixture. |
| T-213 | **DONE (2026-07-03).** Port `WideningBeadingStrategy` (thin-feature single-wall regime) | `crates/slicer-core/src/beading/widening.rs` | Thin-wedge fixture: features < min_input_width handled correctly. |
| T-214 | **DONE (2026-07-03).** Port `OuterWallInsetBeadingStrategy` (outer-wall toolpath offset, decorator) | `crates/slicer-core/src/beading/outer_wall_inset.rs` | Outer-wall-only offset; inner walls untouched. |
| T-215 | **DONE (2026-07-03).** Port `LimitedBeadingStrategy` (max-bead-count cap; 0-width sentinel insertion). Sentinels stay internal — see T-215b for strip-pass. | `crates/slicer-core/src/beading/limited.rs` | Internal sentinels inserted at correct positions on cap-boundary fixture; bead-count math correct end-to-end. |
| T-215b | **DONE (2026-07-03).** Implement strip-pass: drop zero-width beads from BeadingStrategy output before `WallLoop` assembly per D-9. Register the deviation in `docs/DEVIATION_LOG.md` with rationale. | `crates/slicer-core/src/beading/limited.rs` (or assembly boundary), `docs/DEVIATION_LOG.md` | External `WallLoop`s carry no zero-width entries; deviation logged. |
| T-216 | **DONE (2026-07-03).** Port `BeadingStrategyFactory` stack composition (Distributed → Redistribute → Widening → OuterWallInset → Limited) | `crates/slicer-core/src/beading/factory.rs` | Stack composition order asserted in test; mismatch fails. |
| ~~T-217~~ | **SUPERSEDED** by D-9 closure + T-215b. No coordination needed with infill modules; sentinels are stripped before external output. | — | — |
| T-218 | **DONE (2026-07-03).** Register all 11 Arachne `m_params.*` config keys in `docs/15_config_keys_reference.md` (`min_feature_size`, `min_bead_width`, `wall_transition_filter_deviation`, `wall_transition_length`, `wall_transition_angle`, `wall_distribution_count`, `min_length_factor`, `initial_layer_min_bead_width`, `outer_wall_offset`, `max_bead_count`, `optimal_width`) | `docs/15_config_keys_reference.md`, `arachne-perimeters.toml` | All keys documented + manifested. |

## Phase 12 — Extrusion generation

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-220 | **DONE (2026-07-03, packet 112).** Port centrality filtering (`filterCentral`, `filterNoncentralRegions`) — landed as a documented from-first-principles adaptation, not a literal port (see D-112-CENTRALITY-ADAPT in `docs/DEVIATION_LOG.md`) | `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs` | Central-edge marks match a self-captured regression baseline for 3 reference fixtures (no OrcaSlicer oracle in-repo — see D-112-SELFCAPTURED-BASELINES). |
| T-221 | **DONE (2026-07-03, packet 112).** Bead-count assignment on central edges (`optimal_bead_count(R)` per edge) | `crates/slicer-core/src/skeletal_trapezoidation/bead_count.rs` | Per-edge bead counts match a self-captured golden fixture. |
| T-222 | **DONE (2026-07-03, packet 112).** Port bead-count upward + downward propagation (`propagateBeadingsUpward`, `propagateBeadingsDownward`) — marks `TransitionMiddle` / `TransitionEnd`, folded into the propagation passes themselves as a documented adaptation (D-112-PROPAGATION-ADAPT) | `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs` | Transition placement matches a self-captured regression baseline for 3 reference fixtures. |
| T-223 | **DONE (2026-07-03, packet 112).** Port `generateToolpaths()` — emits `Vec<VariableWidthLines>` (sorted by inset_idx); reworked in Step 9D to source per-junction widths/offsets from `BeadingStrategy::compute()` rather than a geometric approximation (see D-112-TOOLPATH-WIDTH, closed) | `crates/slicer-core/src/arachne/generate_toolpaths.rs` | Per-junction width topology matches a self-captured regression baseline on the tapered-wedge fixture. |
| T-224 | **DONE (2026-07-03, packet 112).** Define `ExtrusionLine` + `ExtrusionJunction` IR types; schema bumped 4.6.0 → 4.7.0 | `crates/slicer-ir/src/slice_ir.rs` | Types compile; existing `Point3WithWidth` round-trips via converter. |
| T-225 | **DONE (2026-07-03, packet 112).** Port `stitch_extrusions` (join open polylines within `preferred_bead_width_outer - 1nm`; `BeadingFactoryParams` has no `bead_width_x` field) | `crates/slicer-core/src/arachne/stitch.rs` | Stitch-fixture output matches a self-captured baseline; primary perimeters preserved. |
| T-226 | **DONE (2026-07-03, packet 112).** Port `simplifyToolPaths` as Douglas-Peucker simplification per ExtrusionLine (a packet-specified deviation from Orca's Visvalingam-like algorithm — see D-112-SIMPLIFY-DP) | `crates/slicer-core/src/arachne/simplify.rs` | Output vertex counts match a self-captured baseline within tolerance. |
| T-227 | **DONE (2026-07-03, packet 112).** Port `removeSmallLines` (drop odd, non-closed lines shorter than `min_length_factor * min_width`) | `crates/slicer-core/src/arachne/remove_small.rs` | Primary perimeters never removed; transition lines correctly dropped. |

## Phase 13 — Wire-up + verification

| ID | Title | Files | Acceptance |
|---|---|---|---|
| T-230 | **DONE (2026-07-03, packet 112).** Wire `slicer-core::arachne` + `slicer-core::beading` + `slicer-core::skeletal_trapezoidation` into `arachne-perimeters` module's `run_perimeters` — via a new WIT host-service bridge `generate-arachne-walls` (the WASM guest cannot call `host-algos`-gated slicer-core directly; see D-112-HOSTSVC-BRIDGE, a documented architecture correction from this task's original in-guest design) | `modules/core-modules/arachne-perimeters/src/lib.rs`, `crates/slicer-sdk/src/host.rs`, `crates/slicer-core/src/arachne/pipeline.rs` | Module produces WallLoops with per-junction width; pre-processing + SKT + beading + extrusion-gen runs end-to-end on golden fixture (AC-9, `arachne_perimeters_simple_square_produces_walls`). |
| T-231 | **DONE (2026-07-03, packet 112).** Extend parity harness (T-100) with 4 Arachne fixtures (tapered wedge, narrow strip with widening, max-bead-count cap, complex multi-feature polygon) plus a `cube_4color_arachne` MMU structural-fragmentation fixture (D-112-MMU-TOPOLOGY) | `crates/slicer-runtime/tests/fixtures/perimeter_parity/`, `crates/slicer-runtime/tests/executor/cube_4color_arachne.rs` | Fixtures pass within calibrated tolerances against self-captured regression baselines (D-112-SELFCAPTURED-BASELINES). |
| T-232 | **DONE (2026-07-03, packet 112).** Walk every M2 deviation entry from T-003 update; close or justify — 9 new `D-112-*` entries registered in `docs/DEVIATION_LOG.md` (3 closed, 6 justified-residual with follow-on notes); D-7/D-9/D-15 closure notes below confirmed current | `docs/DEVIATION_LOG.md`, `docs/07_implementation_status.md` | All Arachne deviations closed or justified. |
| T-233 | **DONE (2026-07-03, packet 112).** Update `docs/01_system_architecture.md` Tier-2 box to name the real pipeline (Voronoi + SkeletalTrapezoidation + BeadingStrategy) and the `wall_generator` config selector; no "iterative-inset approximation" caveat existed to remove (P108 already left it clean) | `docs/01_system_architecture.md` | Doc reflects reality. |
| T-234 | **DONE (2026-07-03, packet 112).** Final `cargo xtask test --workspace` closure-ceremony run at packet-close acceptance ceremony per `packet.spec.md` AC-13 — VERDICT: PASS, 0 failed workspace-wide (127 test binaries); `cargo test -p slicer-core --features host-algos` 310 passed/0 failed; `cargo xtask build-guests --check` CLEAN; `cargo clippy --workspace --all-targets -- -D warnings` clean | n/a | Green — see the packet 112 closure log. |

---

## Appendix A — Task count snapshot

- M1 — Phase 0: 5 | Phase 1: 10 | Phase 2: 6 | Phase 3: 4 | Phase 4: 6 | Phase 5: 5 | Phase 6: 6 | Phase 7: 7 | Phase 8: 4 | Rename: 3 | Phase 9: 6 — **62 tasks**
- M2 — Phase 10: 6 | Phase 11: 9 | Phase 12: 8 | Phase 13: 5 — **28 tasks**
- **Total: 90 tasks**

Packets will bundle 3-6 contiguous tasks (per Phase or sub-phase boundary) when sized later.

## Appendix B — Task dependencies between phases

- Phase 1 (T-013 `MaterialBoundary` widening) → Phase 2 (T-021/T-022 inner-wall paint)
- Phase 1 (T-010 shared utils) → all subsequent phases that touch wall flags
- Phase 4 (T-041 medial_axis) → Phase 6 (T-061 thin-wall) and Phase 6 (T-064 gap-fill)
- Phase 4 (T-040 offset2_ex) → Phase 6 (T-061)
- Phase 4 (T-043 polygon tree) → Phase 7 (T-076 wall-sequence reorder, if D-1 lands in perimeter)
- Phase 4 (T-044 keep_largest_contour_only) → Phase 7 (T-074 spiral vase)
- Phase 9 (T-100 parity harness) → Phase 13 (T-231 Arachne fixture extension)
- All Phase 10 (Voronoi + SKT foundations) → Phase 11 (BeadingStrategy) → Phase 12 (extrusion generation) → Phase 13 (wire-up)

## Appendix C — Tasks by module/file (navigation)

**`modules/core-modules/classic-perimeters/`**
T-001, T-005, T-011, T-015, T-016, T-018, T-019, T-020, T-024, T-025, T-031, T-033, T-051, T-052, T-053, T-061, T-062, T-063, T-064, T-071, T-073, T-074, T-076, T-102

**`modules/core-modules/arachne-perimeters/` (current fake; deleted by T-090 under P108 — D-110-DROP-VARIABLE-WIDTH)**
T-002, T-005, T-012, T-015, T-016, T-018, T-019, T-020, T-024, T-025, T-031, T-033, T-090, T-091, T-092

**`modules/core-modules/arachne-perimeters/` (new in M2)**
T-205, T-218, T-230, T-231, T-233

**`crates/slicer-ir/`**
T-013, T-042, T-224

**`crates/slicer-sdk/`**
T-015, T-017, T-023, T-070

**`crates/slicer-core/`**
T-010, T-014, T-040, T-041, T-042, T-043, T-044, T-045, T-050, T-080, T-201, T-202, T-203, T-204, T-210–T-217, T-220–T-227

**`docs/`**
T-003, T-004, T-017, T-023, T-030, T-032, T-053, T-054, T-060, T-065, T-072, T-075, T-081, T-083, T-103, T-104, T-200, T-217, T-218, T-232, T-233

## Appendix D — Module ownership of OrcaSlicer `process_classic` config keys

For reviewers checking which module honours which key.

| Orca config key | Owner (this codebase) | M1 task |
|---|---|---|
| `wall_loops` | `classic-perimeters` `wall_count` | (existing) |
| `outer_wall_line_width`, `inner_wall_line_width`, `smaller_perimeter_line_width` | `classic-perimeters` | T-051 |
| `outer_wall_speed`, `inner_wall_speed` | `classic-perimeters` | (existing) |
| `wall_sequence`, `precise_outer_wall`, `wall_direction` | `classic-perimeters` + (per D-1) | T-053, T-054, T-076 |
| `detect_thin_wall` | `classic-perimeters` | T-060 |
| `gap_infill_speed`, `filter_out_gap_fill` | per D-2 | T-065 |
| `only_one_wall_top`, `only_one_wall_first_layer` | `classic-perimeters` | T-030, T-032 |
| `extra_perimeters_on_overhangs` | `classic-perimeters` + (per D-5) | T-070, T-071 |
| `overhang_reverse`, `overhang_reverse_internal_only`, `overhang_reverse_threshold` | per D-1 (likely path-optimization) | (deferred) |
| `spiral_vase` | per D-3 | T-074, T-075 |
| `bridge_angle`, `counterbore_hole_bridging` | (likely `PrePass::MeshAnalysis` extension — outside this roadmap) | (not in scope) |
| `fuzzy_skin*` | `fuzzy-skin` (existing) | (out of scope; we just set the flag) |
