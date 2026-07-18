---
status: implemented
packet: 96
task_ids: [TASK-246, TASK-246-BISECTOR]
---

# 96_paint-segmentation-phase5-width-limit

## Goal

Implement OrcaSlicer's `cut_segmented_layers` per `docs/specs/orca-paint-segmentation-parity.md` §3 Phase 5 so the `mmu_segmented_region_max_width` and `mmu_segmented_region_interlocking_depth` config keys take geometric effect, with the OrcaSlicer-parity semantic that `mmu_segmented_region_interlocking_beam == true` SKIPS Phase 5 entirely at the driver level (verified against `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2452`). Per layer, per variant chain, erode the variant's polygons by `difference_ex(variant_polygons, offset(input_expolygons, -depth_for_layer_mm, ...))` where `depth_for_layer = (layer_idx % 2 == 0 && interlocking_depth_units != 0) ? interlocking_depth_units : region_width_units` (OrcaSlicer parity: even-layer depth is STANDALONE `interlocking_depth`, NOT additive with `region_width`). The inward-offset primitive is the existing `pub fn offset(polygons: &[ExPolygon], delta_mm: f32, join, arc_tolerance) -> Vec<ExPolygon>` at `crates/slicer-core/src/polygon_ops.rs:195` invoked with a NEGATIVE delta; no `offset_expolygons_inward` helper exists. Wire the pass into the driver `pub fn execute_paint_segmentation` at `crates/slicer-core/src/algos/paint_segmentation/mod.rs:393`, AFTER the inlined variant-composition block ends at `mod.rs:802` (the `working[i].regions = new_regions;` write under the `if !new_regions.is_empty()` guard at line 801) and BEFORE the final return at `mod.rs:999`, guarded by `if !interlocking_beam`. Read config keys via the P1a interner helper `RegionMapIR::config_for` (defined at `crates/slicer-ir/src/slice_ir.rs:1230`). Add three `[config.schema.*]` TOML entries to `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` (the existing core-module governing painted-mesh ingest; see `design.md` §"Schema Landing Site" for the rationale and the P97/P5a coordination note). Extend the cube_4color suite with the SHAPE-DEPENDENT tests the roadmap describes. Additionally, implement the bisector-edge ownership mechanism (TASK-246-BISECTOR) needed to drive AC-22b GREEN (a deferred P95 test re-claimed by P96 per deviation `D-95-AC22-BISECTOR-DEDUP`). Make sure default-config slicing is byte-identical to the post-P95 baseline (Phase 5 short-circuits at the driver via `!beam` AND in the kernel when both keys are 0).

## Problem Statement

P95 ships Phases 1, 2, 3, 4, 6, 7 of the paint-segmentation pipeline. Phase 5 — `cut_segmented_layers` — is deferred because it's the only stage whose impact is purely geometric refinement of an already-correct variant assignment. With Phase 5 missing:

- The `mmu_segmented_region_max_width` config key has no geometric effect. A user who configures `0.4` (the OrcaSlicer default unit width) sees no change in the produced regions — the assignment is sharp-edged, no width limiting.
- The `mmu_segmented_region_interlocking_depth` config key has no effect. Without interlocking beams between layers, painted regions stack vertically with no inter-layer reinforcement, which is a print-quality regression for multi-color models.

Additionally, P95 closed with one acceptance test (`cube_4color_per_layer_outer_wall_count_matches_unpainted_baseline_within_one`) deferred under deviation `D-95-AC22-BISECTOR-DEDUP`: every Voronoi edge between two differently-colored cells is traced as an outer wall by BOTH adjacent cells, doubling the per-layer outer-wall count for N-color slices. P96 OWNS the fix.

The OrcaSlicer-parity goal stated in the v2 audit and the roadmap explicitly includes Phase 5. This packet closes the gap by porting `cut_segmented_layers` per spec §3 Phase 5 AND implementing the bisector-edge ownership mechanism:

- Per layer, per variant chain, erode the variant's polygons by `difference_ex(variant_polygons, offset(input_expolygons, -depth_for_layer_mm, OffsetJoinType::Miter, OFFSET_ARC_TOLERANCE_MM))`. The inward-offset primitive is the existing `pub fn offset(polygons: &[ExPolygon], delta_mm: f32, join, arc_tolerance) -> Vec<ExPolygon>` at `crates/slicer-core/src/polygon_ops.rs:195` invoked with a NEGATIVE `delta_mm`; there is NO `offset_inward` / `offset_expolygons_inward` helper. Per-layer depth selection (OrcaSlicer parity, verified against `MultiMaterialSegmentation.cpp:1294`): `depth_for_layer = (layer_idx % 2 == 0 && interlocking_depth_units != 0) ? interlocking_depth_units : region_width_units`. The even-layer branch uses `interlocking_depth` STANDALONE — NOT additive with `region_width`. Conversion: `depth_mm = depth_units / 10_000.0` (1 unit = 100 nm = 1e-4 mm; see `docs/08_coordinate_system.md`).
- When `interlocking_beam = true`: the driver SKIPS the entire `cut_segmented_layers` call (OrcaSlicer parity verified against `MultiMaterialSegmentation.cpp:2452`). The original P96 draft assumed `beam = true` means "constant-depth alternation"; this was incorrect.
- When both `region_width = 0` and `interlocking_depth = 0`: kernel short-circuits internally (no-op).

The Phase 5 pass plugs into the driver `pub fn execute_paint_segmentation` at `crates/slicer-core/src/algos/paint_segmentation/mod.rs:393`, INSERTED after the inlined variant-composition block ends at `mod.rs:802` and before the final `Ok(Arc::new(working))` return at `mod.rs:999`. Reads config via `RegionMapIR::config_for(&region_key)` (P1a; helper defined at `slice_ir.rs:1230`). Default config (all three keys at declared defaults) preserves byte-identical behavior via the driver-level `!beam` guard.

The bisector-edge ownership mechanism (TASK-246-BISECTOR): extend `SlicedRegion` with a `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` field; populate it in `execute_paint_segmentation` between the variant block end and the Phase 5 call; consume it in `modules/core-modules/classic-perimeters/src/lib.rs` outer-wall emission (`run_perimeters` at `lib.rs:85`, polygons consumed at `lib.rs:94`). See `design.md` §"Bisector-Edge Ownership (AC-22b) Code Change Surface" for the full surface.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. The `SlicedRegion` IR field addition (TASK-246-BISECTOR) GUARANTEES guest staleness; rebuild is mandatory.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- **Short-circuit invariant (kernel)**: when both `mmu_segmented_region_max_width = 0` and `mmu_segmented_region_interlocking_depth = 0` (in i64 units), the kernel returns `Ok(())` immediately. Default-config slices produce byte-identical g-code via this path (AC-8).
- **Beam-flag invariant (driver-level skip, OrcaSlicer parity)**: `mmu_segmented_region_interlocking_beam = true` causes the DRIVER to skip the entire `cut_segmented_layers` call. The kernel itself does NOT take a `beam` parameter. This matches `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2452` (the call is gated on `!segmentation_interlocking_beam`). AC-7 and AC-N3 both verify this driver-level skip. The original P96 draft assumed `beam = true` produced constant-depth bands — this was incorrect.
- **Per-layer depth invariant (OrcaSlicer parity)**: `depth_for_layer = (layer_idx % 2 == 0 && interlocking_depth != 0) ? interlocking_depth : region_width`. Per `MultiMaterialSegmentation.cpp:1294`, the even-layer branch is STANDALONE `interlocking_depth`, NOT additive with `region_width`. The original P96 draft showed an additive sketch (`interlocking_depth_units + region_width_units`) — this was incorrect.
- **Negative-value invariant**: any of the depth/width keys with negative value triggers `PaintSegmentationError::InvalidPhase5Config { key, value }` at runtime (AC-N1). The config-schema declares both as `minimum = 0.0`; the runtime guards against schema validator bypass.
- **Empty-output invariant**: width larger than the smallest variant footprint correctly produces empty per-variant polygons; entries persist in `SliceIR` (D15 compatible).
- **Bisector-edge-mask additivity invariant**: the new `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` field on `SlicedRegion` defaults to `None`. For unpainted slices, the field is ALWAYS `None`. AC-10 regression (11/11 + 10/10 GREEN) confirms unpainted-shape and existing-cube-paint paths are unaffected when no bisector edges exist. Outer-wall emission in `classic-perimeters` only consults the mask when `Some(_)`; `None` behaves exactly as the pre-AC-22b code.

## Data and Contract Notes

- IR contracts touched: `SlicedRegion` gains additive field `bisector_edge_skip_mask: Option<Vec<Vec<bool>>>` (default `None`). This is the only IR-shape change. Existing serialized `SliceIR` remains forward-compatible via `#[serde(default)]`. Doc Impact: `docs/02_ir_schemas.md` §"SlicedRegion" updated on closure.
- WIT boundary considerations: the `SlicedRegion` field addition propagates through guest bindgen. ALL guest WASMs are stale after this change; `cargo xtask build-guests --check` MUST be run after Step 4b (IR change) and any guest rebuild needed before Step 4c can be tested against a real `classic-perimeters` module. Schema entries in `mesh-segmentation.toml` ALSO require a rebuild of that guest. Both invalidations are covered by Step 8.
- Determinism or scheduler constraints: Phase 5's even/odd layer alternation is deterministic by layer index (same input → same output). Bisector-edge ownership is deterministic by lowest-color-id rule (no ambiguity when two cells differ in color).

## Locked Assumptions and Invariants

- **Default config → no-op**: with all three keys at declared defaults (`max_width = 0.0`, `interlocking_depth = 0.0`, `interlocking_beam = false`), Phase 5 short-circuits in the kernel (both depth/width = 0). AC-8 byte-identicality contract.
- **Driver-level beam guard (OrcaSlicer parity)**: `interlocking_beam = true` causes the DRIVER to skip the entire `cut_segmented_layers` call. The kernel does not consult the beam flag. AC-7 and AC-N3 both verify this. Per `MultiMaterialSegmentation.cpp:2452`.
- **Standalone even-layer depth (OrcaSlicer parity)**: `depth_for_layer = interlocking_depth` STANDALONE on even layers (not additive with `region_width`). Per `MultiMaterialSegmentation.cpp:1294`.
- **Negative values rejected**: schema-level (`minimum = 0.0`) + runtime defense (kernel returns `InvalidPhase5Config` error).
- **Bisector-mask additivity**: `SlicedRegion.bisector_edge_skip_mask` defaults to `None`. Unpainted slices and pre-AC-22b code paths never construct a `Some(_)` mask. AC-10 regression confirms unpainted-shape and existing-cube-paint paths unchanged.
- **Lowest-color-id ownership rule**: bisector edges between two differently-colored cells are owned by the cell with the LOWER color-id (canonical, deterministic). Documented in `bisector_ownership.rs` module-level comment on closure.

## Risks and Tradeoffs

- **Risk: Phase 5 interacts badly with downstream perimeter generation** (the variant polygons it produces have eroded inner boundaries; perimeter generator might produce duplicate / overlapping perimeters). Mitigation: AC-9's visual report check; if visual confirms banding without perimeter artifacts, packet ships.
- **Risk: integration test for "alternating bands across adjacent layers" requires careful Z-layer pair selection**. Mitigation: pick layers far from top/bottom (avoid edge effects); document the chosen layer indices in the closure log.
- **Risk: bisector-edge ownership rule mis-resolves the lower-color-id when `variant_chain` is multi-valued** (a chain like `[(extruder, 1), (extruder, 2)]` has more than one "color"). Mitigation: the resolution rule explicitly takes `min(color_id)` over the chain; documented in `bisector_ownership.rs` module-level docs.
- **Risk: AC-22b (bisector dedup) over-deletes edges** (e.g. a region's edge that does NOT touch a differently-colored neighbor gets masked). Mitigation: tagging stage only sets the mask when the geometric `EdgeKey` bucket contains ≥ 2 distinct-color entries; boundary edges with bucket size 1 keep `false` and stay emitted. The chosen algorithm is described in §"Bisector-Edge Ownership (AC-22b) Code Change Surface" → "Algorithm" with the worked 3-color-corner case.
- **Risk: geometric `EdgeKey` mis-buckets two near-coincident but not equal edges** (e.g. one came from compose_variants and the other from Phase 5 erosion). Mitigation: tagging stage runs BEFORE Phase 5 (Step 4b precedes Step 4a's call site is inside `!beam`, but Phase 5 only mutates polygons under `!beam`; tagging needs the pre-Phase-5 polygons to see true Voronoi-bisector geometry). Both edges originate from the same compose_variants Voronoi step and share exact `Point2<i64>` endpoints — no rounding drift.
- **Risk: schema landing in `mesh-segmentation.toml` becomes wrong if P5a deletes that module**. Mitigation: explicit coordination note in §"Schema Landing Site"; P5a takes responsibility for migration.
- **Tradeoff: `0.0` defaults vs. OrcaSlicer's non-zero defaults**. `0.0` is conservative and preserves byte-identical regression; users opt in via config. Register deviation `D-96-DEFAULT-ZERO` on closure (see Doc Impact Statement).
- **Tradeoff: corrected `beam = true` semantics deviate from the original P96 draft** (which assumed constant-depth bands). The new semantic matches OrcaSlicer's actual behavior verified at `MultiMaterialSegmentation.cpp:2452`. Register deviation `D-96-BEAM-FLAG-SKIPS` on closure recording the semantic correction.
