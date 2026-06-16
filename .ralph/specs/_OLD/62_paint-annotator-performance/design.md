# Design: 62_paint-annotator-performance

## Controlling Code Paths

- Primary code path: guest paint-segmentation module emits `PaintRegionEntry` per facet → `HostExecutionContext::push_paint_region` collects into `ctx.paint_region_entries` → `harvest_paint_segmentation_ir` converts to `PaintRegionIR` → blackboard → `execute_slice_postprocess_paint_annotation` queries via `point_in_paint_region`
- Neighboring tests or fixtures: `paint_segmentation_executor_tdd.rs` (harvest path, 3 paint_order + 0 polygon-count assertions), `macro_paint_region_roundtrip_tdd.rs` (9 paint_order + 1 polygon.len() assertions), `scenario_traces_tdd.rs` (2 precedence assertions), `paint_region_annotator_tdd.rs` (2 precedence assertions), `slice_postprocess_paint_annotation_tdd.rs` (per-point annotation values), plus 6 read-only verification test files
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load). Do not restate the delegation rules here.

## Architecture Constraints

- This packet edits `crates/slicer-ir/src/slice_ir.rs` — adding a field to `SemanticRegion`. Existing serialized IR does not contain `aabb`; the `#[serde(skip_deserializing, default)]` annotation ensures backward compatibility. New IR written after this change will skip `aabb` in serialization, so on-disk format is unchanged.
- `BoundingBox2` uses `Point2 { x: i64, y: i64 }` in 100 nm units — same coordinate system as the rest of the pipeline. No scale conversion needed for AABB comparisons.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The `slicer_core::union` helper at `polygon_ops.rs:93` uses `flat_map` + `union_64` — no `PolyTree`. Output polygons have zero holes regardless of input hole topology. This is safe for the current guest path (guest emits `ExPolygonView` with `holes: vec![]` — triangles only), but must be documented at the call site in `harvest_paint_segmentation_ir` as a known limitation.

- `paint_order` values change from densely-incrementing per-facet indices to `min(paint_order)` per group. Test assertions on specific `paint_order` values in `paint_segmentation_executor_tdd.rs`, `macro_paint_region_roundtrip_tdd.rs`, `scenario_traces_tdd.rs`, and `paint_region_annotator_tdd.rs` must be updated. The precedence contract (higher `paint_order` wins) is preserved.

- Guest WASM is **not** affected by this packet. The change surface is entirely host-side: IR types, harvest logic, and query helpers. No guest source, WIT, or SDK is edited. No WASM rebuild is required.

## Code Change Surface

- Selected approach: Union at harvest (not in guest, not in query path), add AABB at IR construction (not lazily computed), cache the regions slice at the outermost loop (not per-point), parallelize at polygon granularity (not per-point), early-break on sorted descending order.

- Exact functions, traits, manifests, tests, or fixtures expected to change:

  1. `crates/slicer-ir/src/slice_ir.rs` — add `BoundingBox2` struct with `contains_point()` method; add `aabb: Option<BoundingBox2>` to `SemanticRegion` with `#[serde(skip_deserializing, default)]`
  2. `crates/slicer-host/src/dispatch.rs` — rewrite `harvest_paint_segmentation_ir` body: two-pass build with `HashMap` grouping, `slicer_core::union` per group, sort per semantic Vec, compute `BoundingBox2` per region
  3. `crates/slicer-core/src/paint_region.rs` — add AABB pre-filter in `semantic_region_contains_point`; add early-break in `point_in_paint_region`
  4. `crates/slicer-host/src/slice_postprocess.rs` — cache `get()` call per semantic pair; add `rayon::par_iter()` on `region.polygons` with thread-local accumulator merge
  5. `docs/02_ir_schemas.md` — add `BoundingBox2` type documentation and `SemanticRegion.aabb` field note
  6. Test fixtures in `paint_segmentation_executor_tdd.rs`, `macro_paint_region_roundtrip_tdd.rs`, `scenario_traces_tdd.rs`, `paint_region_annotator_tdd.rs` — update `paint_order` values and polygon count assertions

- Rejected alternatives:
  - **Union in the guest module**: Would require WIT changes and guest rebuild; moves complexity to WASM where Clipper is not available; breaks the separation of concerns (guest produces raw geometry, host performs spatial ops).
  - **Lazy AABB computation in query path**: Would compute AABB repeatedly on every query; a single pre-computation at construction time is cheaper and follows the OrcaSlicer `AABBTreeIndirect` pre-built model.
  - **Per-contour-point parallelism**: Too fine-grained; would require synchronizing on every point's result. Polygon-level parallelism groups results naturally.
  - **Schema version bump for `SemanticRegion`**: Unnecessary — `#[serde(skip_deserializing)]` means `aabb` is never serialized, so the on-disk format does not change.

## Files in Scope (read + edit)

- `crates/slicer-ir/src/slice_ir.rs` — role: defines `SemanticRegion`, `BoundingBox3`; expected change: add `BoundingBox2` struct and `SemanticRegion.aabb` field
- `crates/slicer-host/src/dispatch.rs` — role: harvests `PaintRegionIR` from guest entries; expected change: rewrite `harvest_paint_segmentation_ir` with union + grouping + AABB computation + per-semantic sort
- `crates/slicer-core/src/paint_region.rs` — role: point-in-region query helpers; expected change: AABB pre-filter in `semantic_region_contains_point`, early-break in `point_in_paint_region`
- `crates/slicer-host/src/slice_postprocess.rs` — role: annotation loop consumer; expected change: cache `get()`, `par_iter()` with thread-local accumulators
- `docs/02_ir_schemas.md` — role: IR schema documentation; expected change: `BoundingBox2` type entry, `SemanticRegion.aabb` field documentation
- `crates/slicer-host/tests/paint_segmentation_executor_tdd.rs` — role: harvest-path test; expected change: update 3 `paint_order` assertions to post-union values
- `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — role: macro roundtrip test; expected change: update 9 `paint_order` + 1 `polygons.len()` assertions
- `crates/slicer-host/tests/scenario_traces_tdd.rs` — role: precedence/conflict test; expected change: update 2 precedence assertions
- `modules/core-modules/paint-region-annotator/tests/paint_region_annotator_tdd.rs` — role: annotator test; expected change: update 2 precedence assertions

## Read-Only Context

- `crates/slicer-host/src/paint_segmentation.rs` — read lines 180-199 only — purpose: confirm existing `union()` call pattern and `compare_semantic_regions` sort ordering used by the host executor path
- `crates/slicer-core/src/polygon_ops.rs` — read lines 93-95 only — purpose: confirm `pub fn union(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon>` signature and flat_map hole-loss comment at line 62
- `crates/slicer-host/src/wit_host.rs` — read lines 4384-4427 only — purpose: confirm `push_paint_region` validation rules (non-empty polygons, layer_index >= 0, etc.) that guarantee harvest input shape
- `crates/slicer-host/tests/paint_annotation_integration_tdd.rs` — read `ambiguous_triangle_paint_regions()` helper only — purpose: confirm test fixture shape after union (warning/error paths)
- `crates/slicer-host/tests/paint_region_transport_widening_tdd.rs` — delegate a FACT: does any test assert polygon hole counts on harvest-produced regions? (guest path produces no holes; only host executor path does)

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — delegate parity checks; never load
- `target/`, `Cargo.lock` — never load
- `crates/slicer-host/src/paint_segmentation.rs` (beyond lines 180-199) — the host executor path is out of scope
- `crates/slicer-host/src/layer_executor.rs` — paint annotation is dispatched inline within `slice_postprocess.rs`; the layer executor is a pass-through
- `modules/core-modules/paint-segmentation/` — guest source is unchanged
- `wit/` — WIT contracts are unchanged
- `crates/slicer-sdk/`, `crates/slicer-macros/`, `crates/slicer-schema/` — no SDK or macro changes

## Expected Sub-Agent Dispatches

- "Run `cargo test -p slicer-host --test paint_segmentation_executor_tdd`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)" — purpose: validate Step 2 union-at-harvest
- "Run `cargo test -p slicer-core paint_region`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)" — purpose: validate Steps 1+3 AABB pre-filter and early-break
- "Run `cargo test -p slicer-host --test slice_postprocess_paint_annotation_tdd`; return FACT (pass) or SNIPPETS" — purpose: validate Steps 2+4 caching and parallelization
- "Run `cargo test -p slicer-host --test scenario_traces_tdd`; return FACT (pass) or SNIPPETS" — purpose: validate Step 2 paint_order precedence preserved
- "Run `cargo test -p slicer-host --test macro_paint_region_roundtrip_tdd`; return FACT (pass) or SNIPPETS" — purpose: validate Step 2 polygon count assertions updated
- "Run `cargo test -p slicer-host --test paint_region_annotator_tdd`; return FACT (pass) or SNIPPETS" — purpose: validate Step 2 annotator precedence assertions updated
- "Run `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.stl --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html 2>&1`; return FACT (annotator row wall-clock time in seconds + 504 warning count)" — purpose: validate AC-9 and AC-10 end-to-end

## Data and Contract Notes

- IR contracts touched: `PaintRegionIR` output shape unchanged (still `HashMap<u32, LayerPaintMap>` with `HashMap<PaintSemantic, Vec<SemanticRegion>>`). `SemanticRegion` gains one optional field (`aabb`), serde-skipped — no schema version bump.
- WIT boundary: unchanged. Guest still emits `paint-region-entry` records; host still receives them into `paint_region_entries`. Only the conversion in `harvest_paint_segmentation_ir` changes.
- Determinism: group sorting by `(paint_order, object_id, value_key)` within each semantic Vec ensures byte-deterministic output across runs. The per-layer `HashMap` iteration order is already non-deterministic at the `per_layer` level but deterministic within each `LayerPaintMap.semantic_regions` Vec.
- Scheduler: no change to stage order, DAG edges, or claim semantics.

## Locked Assumptions and Invariants

- `slicer_core::union` discards holes — all guest-produced paint region entries carry `holes: vec![]` (triangles only). If a future guest version emits entries with holes, the harvest path must switch to a hole-preserving union variant.
- `paint_order` remains a `u64` running along the WIT entry insertion order. After grouping, `min(paint_order)` per group preserves precedence ordering between groups of different values.
- The AABB pre-filter is an optional optimization — setting `aabb = None` at construction time (or skipping the computation) must not change correctness, only performance.
- `rayon` is already a `slicer-host` dependency — no new Cargo.toml entry.
- Group key `(layer_index, object_id, semantic, value)` is correct because: (a) regions with the same value are query-equivalent and safe to merge; (b) `object_id` preserves per-object boundaries; (c) `paint_order` conflict logic only triggers between regions of different values, never within a same-value group.

## Risks and Tradeoffs

- **Test assertion churn**: 16 `paint_order` and 1 `polygons.len()` assertion across 4 test files must be updated. Each is a behavioral check, not noise — updating them validates the new contract.
- **Union performance at harvest time**: computing `union()` for hundreds of tiny triangles adds harvest-time cost. However, the harvest runs once per pipeline, while the annotation query runs per contour point. The upfront cost is amortized within the first few hundred queries.
- **Hole loss**: `slicer_core::union` flattens holes. Mitigated by the fact that guest output has no holes. If this assumption changes, the downstream effects are: (a) hole-containing paint regions would lose their holes silently; (b) annotation results would treat previously-excluded hole interiors as included. The call-site documentation serves as the canary.
- **par_iter() thread safety**: `PaintRegionIR` is `Arc`-wrapped and read-only. `warnings` and `degraded` require thread-local collection + merge. No shared mutable state beyond these two accumulators.
- **Early-break correctness**: requires regions to be sorted descending by `paint_order`. If the sort is omitted or incorrect, the early-break could skip a region with higher `paint_order`. The sort validation lives in the harvest path and is tested by the same assertions that check `paint_order` values.

## Context Cost Estimate

- Aggregate (sum across all steps): `M` (Step 1: S, Step 2: M, Step 3: M, Step 4: M)
- Largest single step: `M` (Step 2: union-at-harvest — requires reading dispatch.rs context, writing ~50 lines of grouping + union logic, updating 4 test files)
- Highest-risk dispatch: the end-to-end `slicer-host --release -- run --report` on benchy_4color.stl — may produce > 100 lines of HTML; the implementer should filter for the annotator table row only. Return format: FACT (two numbers: annotator_time_seconds, code504_count).

## Open Questions

- [FWD] Should the AABB be computed from all polygons (contour + holes) or only contours? For guest-produced triangles (no holes), this is irrelevant. The design computes from contour points only. If holes are later added to guest output, the AABB should expand to include hole boundaries.
- [FWD] What is the exact post-union `paint_order` value range for benchy_4color? The implementer will determine this empirically from test output and update assertions accordingly. The contract is that `paint_order` is `min(idx)` per group and precedence is preserved.
- None activation-blocking.
