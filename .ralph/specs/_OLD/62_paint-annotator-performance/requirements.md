# Requirements: 62_paint-annotator-performance

## Packet Metadata

- Grouped task IDs:
  - `TASK-130c` (completed — paint-region transport ExPolygon widening)
  - `TASK-181` (completed — paint-semantic-aware RegionMap)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`com.host.slice-postprocess-paint-annotator` consumes ~188 s wall-clock (2 255 853 ms summed across 12 threads) on a benchy-class print — roughly 120× the prepass pipeline. The root cause is structural:

1. `harvest_paint_segmentation_ir` at `dispatch.rs:2003-2085` copies guest-emitted `PaintRegionEntry` entries verbatim, producing one `SemanticRegion` per painted facet. For a benchy_4color object, this is hundreds of single-triangle `SemanticRegion` entries per `(layer, semantic)`.

2. `point_in_paint_region` at `paint_region.rs:24-55` iterates every region linearly with no spatial pre-filter. On a miss, `is_point_numerically_ambiguous` at `slice_postprocess.rs:510-540` does a second full O(regions × polygons × edges) scan. The comment at `slice_postprocess.rs:527` explicitly blames "un-unioned per-facet projected triangles."

3. `paint_regions.get(layer_index, semantic)` is called 3+ times per contour point (inside `point_in_paint_region`, inside `is_point_numerically_ambiguous`, and in the fallback branch) — redundant `BTreeMap` lookups for the same key.

4. The contour-point annotation loop at `slice_postprocess.rs:357-411` is fully serial despite `rayon` being an existing dependency and `PaintRegionIR` being `Arc`-wrapped and thread-safe.

5. `point_in_paint_region` iterates all regions even after finding a definitive winner — no early break on descending `paint_order`.

OrcaSlicer's MMU pipeline (`pseudocode_multimaterial_segmentation.md` Phase 1) unions per-facet projections into `ExPolygons` before any query — a documented porting gap (DEV-025, `docs/DEVIATION_LOG.md:33`). This packet closes that gap and adds the caching, spatial pre-filter, and parallelization that OrcaSlicer achieves via `EdgeGrid` and `tbb::parallel_for`.

## In Scope

- Union per-facet `PaintRegionEntry` polygons at `harvest_paint_segmentation_ir`, grouped by `(layer_index, object_id, semantic, value)`, using `slicer_core::union` (same helper already used at `paint_segmentation.rs:190` for modifier volumes)
- Add `BoundingBox2 { min: Point2, max: Point2 }` to `crates/slicer-ir/src/slice_ir.rs`, with `contains_point(&self, point: Point2) -> bool`
- Add `#[serde(skip_deserializing, default)] pub aabb: Option<BoundingBox2>` field to `SemanticRegion`
- Compute `aabb` at construction time in `harvest_paint_segmentation_ir` (min/max over all polygon contour points in the group)
- Add AABB pre-filter in `semantic_region_contains_point` at `paint_region.rs:57-66` — skip polygon iteration if `aabb` is `Some` and does not contain the point
- Cache `paint_regions.get(layer_index, semantic)` once per `(layer_index, semantic)` pair in `execute_slice_postprocess_paint_annotation`; pass the `&[SemanticRegion]` slice directly through `point_in_paint_region` and `is_point_numerically_ambiguous`
- Parallelize `region.polygons` iteration in `execute_slice_postprocess_paint_annotation` with `rayon::par_iter()`, collecting thread-local `warnings` and `degraded` flags, merging after `collect()`
- Add early-break in `point_in_paint_region`: sort `SemanticRegion` Vec descending by `paint_order` within each semantic bucket at harvest; break after first winner found
- Sort groups within each `Vec<SemanticRegion>` per semantic by `(paint_order, object_id, value_key)` for byte-deterministic output across runs
- Preserve `paint_order` precedence contract: `min(paint_order)` per group for same-value-merged regions; higher `paint_order` still wins
- Update `docs/02_ir_schemas.md` with `BoundingBox2` type and `SemanticRegion.aabb` field documentation
- Update test assertions in `paint_segmentation_executor_tdd.rs`, `macro_paint_region_roundtrip_tdd.rs`, `scenario_traces_tdd.rs`, `paint_region_annotator_tdd.rs` to reflect post-union `paint_order` values and polygon counts
- Document `slicer_core::union` hole-loss limitation at the call site in `harvest_paint_segmentation_ir`

## Out of Scope

- R-tree spatial index (rstar crate) — deferred to packet `63_paint-annotator-r-tree`
- Host executor path (`execute_paint_segmentation` in `paint_segmentation.rs`) — its per-facet fragmentation is a separate work item
- AABB computation on `ExPolygon` itself — `BoundingBox2` is computed once at region construction, not per-polygon
- Fixing `slicer_core::union` to preserve holes via `PolyTree` — documented limitation, acceptable for current guest output (triangles without holes)
- WIT interface changes — the guest protocol is unchanged; only the host-side conversion changes
- Guest WASM rebuilds — the change surface is host-only (IR types, harvest logic, query helpers)
- Changing the `code:504` ambiguity warning policy, message text, or `EPSILON_UNITS = 1` constant
- Adding a dedicated `cargo bench` target for paint annotation — the packet uses existing benchmarks (`pipeline`, `polygon_ops`) and `--report` HTML timing as proxy measurements; a dedicated bench target is recommended but not required for acceptance
- Modifying any module under `modules/core-modules/`

## Authoritative Docs

- `docs/02_ir_schemas.md` — lines 469-488 (PaintRegionIR, LayerPaintMap, SemanticRegion fields). Delegate fact-check for existing field names; implementer range-reads only this section.
- `docs/01_system_architecture.md` — dispatch and harvest lifecycle. Delegate SUMMARY (> 300 lines).
- `docs/04_host_scheduler.md` — lines 80-160 (PrePass stage order, PaintSegmentation → PaintRegionIR flow). Range-read only.
- `docs/08_coordinate_system.md` — range-read the unit-system section for `Point2` 100 nm convention; only needed to confirm `BoundingBox2` comparisons use native units with no scale conversion.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/generated_documentation/pseudocode_multimaterial_segmentation.md` — Phase 1 `union_ex` of region slices; parity anchor confirming OrcaSlicer unions paint polygons before query
- `OrcaSlicerDocumented/generated_documentation/03_algorithmic_complexities.md` §"AABB Tree" — O(log N) spatial query pattern; parity anchor for AABB pre-filter direction (full tree deferred to packet 63)
- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md` §"Paint-on triangle selection" — confirms OrcaSlicer's paint regions carry ExPolygons, not per-facet triangles

## Acceptance Summary

- Positive cases: `AC-1` through `AC-10` from `packet.spec.md`
  - AC-1 through AC-3: union-at-harvest structural change (fewer regions, unioned polygons)
  - AC-4: AABB pre-filter pass-through correctness
  - AC-5: deserialized IR backward compatibility (aabb = None)
  - AC-6: parallelization scaling
  - AC-7: early-break correctness
  - AC-8: paint_order precedence preserved
  - AC-9: code:504 warning count reduction
  - AC-10: end-to-end wall-clock reduction ≥ 90%
- Negative cases: `AC-N1` through `AC-N3` from `packet.spec.md`
  - AC-N1: DeterministicConflict not hidden by same-value grouping
  - AC-N2: empty polygons handled without panic
  - AC-N3: output determinism across runs
- Cross-packet impact: unblocks `63_paint-annotator-r-tree` (reuses `BoundingBox2` type and `SemanticRegion.aabb` field)

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-host --test paint_segmentation_executor_tdd` | AC-1: union-at-harvest produces fewer, unioned regions; AC-N3: output determinism | FACT pass/fail |
| `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd` | AC-2: get() caching; AC-6: par_iter correctness | FACT pass/fail |
| `cargo test -p slicer-core paint_region` | AC-3: AABB rejection; AC-4: AABB pass-through; AC-7: early-break; AC-N2: empty polygons | FACT pass/fail |
| `cargo test -p slicer-host --test core_module_ir_access_contract_tdd` | AC-5: deserialized aabb=None backward compat | FACT pass/fail |
| `cargo test -p slicer-host --test scenario_traces_tdd` | AC-8: paint_order precedence; AC-N1: DeterministicConflict | FACT pass/fail |
| `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd` | AC-1: polygon count assertions after union | FACT pass/fail |
| `cargo test -p slicer-host --test paint_region_annotator_tdd` | AC-8: paint_order precedence in annotator | FACT pass/fail |
| `cargo test -p slicer-host --test paint_region_transport_widening_tdd` | Hole fidelity after union changes | FACT pass/fail |
| `cargo test -p slicer-host --test paint_annotation_integration_tdd` | Warning/error paths unchanged | FACT pass/fail |
| `cargo test -p slicer-host --test region_mapping_paint_semantic_tdd` | Empty-polygon fixtures stable | FACT pass/fail |
| `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.stl --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html` | AC-9: 504 count; AC-10: wall-clock | FACT: annotator row time + 504 count |
| `cargo check --workspace` | Compile gate | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | Lint gate | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: after Step 1 (BoundingBox2 + aabb field), all existing tests must pass unchanged — the new field defaults to `None` and no code yet computes or reads it.
- Cross-step invariant: after Step 2 (union-at-harvest), `paint_segmentation_executor_tdd` and `macro_paint_region_roundtrip_tdd` assertions must be updated to reflect new polygon counts and `paint_order` values.
- Cross-step invariant: after Step 3 (AABB pre-filter + cache + early-break), `point_in_paint_region` results are identical to the pre-change path for any input — the AABB is a pre-filter, not a different containment algorithm.
- Step ordering rationale: IR type addition (Step 1) must precede all consumers because `SemanticRegion` derives `PartialEq` — adding a field changes how all test fixtures compare. Union-at-harvest (Step 2) must precede query-path changes (Steps 3-4) because the query optimizations are benchmarked against the already-unioned IR, not the fragmented one.
- Cross-step shared scratch: `BoundingBox2` and `SemanticRegion.aabb` added in Step 1 are consumed by Steps 2 (compute) and 3 (read).

## Context Discipline Notes

- Large files in the read-only path: `dispatch.rs` (~2200 lines) — range-read `harvest_paint_segmentation_ir` at lines 2003-2085 only. `slice_postprocess.rs` (~900 lines) — range-read the annotation loop at lines 286-492. `paint_region.rs` (~130 lines) — can be read in full.
- Likely temptation reads: `paint_segmentation.rs` host executor path — skip; it is out of scope. `wit_host.rs` — skip; the guest protocol is unchanged. Full `Cargo.toml` of slicer-core — skip; no new deps added in this packet.
- Sub-agent return-format hints: all `cargo test` dispatches return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines). The end-to-end `slicer-host --report` run returns FACT (annotator row time + 504 count as two numbers).
