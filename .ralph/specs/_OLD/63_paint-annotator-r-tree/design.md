# Design: 63_paint-annotator-r-tree

## Controlling Code Paths

- Primary code path: `harvest_paint_segmentation_ir` (dispatch.rs) already builds `PaintRegionIR` with unioned `SemanticRegion` entries (packet 62). This packet adds R-tree construction at the same site, storing the index alongside the region Vec. At query time, `point_in_paint_region` (paint_region.rs) replaces the linear `for region in regions` with an R-tree candidate lookup.
- Neighboring tests or fixtures: `paint_region.rs` unit tests (if any are added for R-tree behavior), `scenario_traces_tdd.rs` (precedence test), `core_module_ir_access_contract_tdd.rs` (deserialization fallback)
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations

## Architecture Constraints

- The R-tree index is reconstruction-only (like `SemanticRegion.aabb` from packet 62). It is NOT serialized, NOT part of `PartialEq`, and rebuilt from scratch at each `harvest_paint_segmentation_ir` call. Deserialized `PaintRegionIR` has no index; queries fall back to the packet-62 linear-scan-with-AABB-pre-filter path.
- `rstar` must be compatible with WASM (the `slicer-core` crate is used in WASM guests). Default `rstar` features should work; verify no `std`-only features are enabled that break `no_std` builds.
- The R-tree index stores `(BoundingBox2, usize)` pairs where `usize` is the index into `Vec<SemanticRegion>`. This avoids storing `SemanticRegion` references (which would have lifetime issues with `RTree`).

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: Define `PaintRegionRTreeIndex` in `slicer-core` (where `rstar` lives), wrapping `HashMap<u32, HashMap<PaintSemantic, RTree<(BoundingBox2, usize)>>>`. Build it at `harvest_paint_segmentation_ir` and return it alongside `PaintRegionIR`. Store it in the blackboard as a companion `Arc<PaintRegionRTreeIndex>` and thread it through to `SlicePostProcessPaintAnnotationRequest`. Change `point_in_paint_region` to accept `Option<&PaintRegionRTreeIndex>` as a separate parameter. At query time, if the index is `Some`, look up the `RTree` for `(layer_index, semantic)`, call `locate_in_envelope(&query_point_aabb)`, collect candidate indices, retrieve candidates from the `SemanticRegion` Vec, and run the existing precedence loop only on candidates. If `None` (deserialized IR, or index not built), fall back to the existing linear-scan-with-AABB-pre-filter path (packet 62 behavior). This approach keeps `slicer-ir` free of an `rstar` dependency — the index lives in `slicer-core` and is passed alongside `PaintRegionIR`, not stored on it.

- Exact functions, traits, manifests, tests, or fixtures expected to change:

  1. `crates/slicer-core/Cargo.toml` — add `rstar = "0.12"` dependency
  2. `crates/slicer-core/src/paint_region.rs` — define `PaintRegionRTreeIndex` newtype; change `point_in_paint_region` signature to accept `rtree_index: Option<&PaintRegionRTreeIndex>`; replace linear scan with R-tree lookup when index is `Some`; keep linear scan as fallback when `None`
  3. `crates/slicer-host/src/dispatch.rs` — at `harvest_paint_segmentation_ir`, after building `per_layer`, iterate and build one `RTree<(BoundingBox2, usize)>` per `(layer_index, semantic)` key, inserting `(region.aabb.unwrap_or_default(), region_index)` pairs; wrap in `PaintRegionRTreeIndex`; return alongside `PaintRegionIR`
  4. `crates/slicer-host/src/slice_postprocess.rs` — thread the `PaintRegionRTreeIndex` through to `point_in_paint_region` and `is_point_numerically_ambiguous` calls
  5. `docs/02_ir_schemas.md` — add note that a companion `PaintRegionRTreeIndex` is built at harvest and used by the query path (not part of the IR itself)

- Rejected alternatives:
  - **Store R-tree index on `PaintRegionIR`**: Would require `slicer-ir` to depend on `rstar` or use type erasure. Passing the index as a separate companion `Arc` keeps `slicer-ir` dependency-free and follows the existing pattern (`PaintRegionIR` is the pure data; indexes are compute artifacts).
  - **Use a flat `Vec` of `(layer, semantic, aabb, region_index)` and a single global R-tree**: The R-tree would mix layers/semantics, requiring post-filtering. Per-key trees are cleaner and match the query pattern (query is always scoped to one key).
  - **Use `kdtree` or `spade` instead of `rstar`**: `rstar` is the most widely used Rust spatial index, has good WASM compatibility, and provides `RTree::locate_in_envelope()` which matches our use case exactly.
  - **Build a custom balanced AABB tree**: Reinventing `AABBTreeIndirect` would duplicate OrcaSlicer's approach but at significant implementation cost. `rstar` provides the same O(log N) query pattern with an off-the-shelf implementation.

## Files in Scope (read + edit)

- `crates/slicer-core/Cargo.toml` — role: add rstar dependency; expected change: one line under `[dependencies]`
- `crates/slicer-core/src/paint_region.rs` — role: define `PaintRegionRTreeIndex` newtype + query helpers; expected change: newtype definition, `point_in_paint_region` signature change to accept `Option<&PaintRegionRTreeIndex>`, R-tree lookup + linear fallback
- `crates/slicer-host/src/dispatch.rs` — role: harvest PaintRegionIR; expected change: build `PaintRegionRTreeIndex` after `per_layer` construction, return alongside `PaintRegionIR`
- `crates/slicer-host/src/slice_postprocess.rs` — role: annotation loop; expected change: thread `PaintRegionRTreeIndex` through to `point_in_paint_region` and `is_point_numerically_ambiguous` calls
- `docs/02_ir_schemas.md` — role: IR documentation; expected change: note that a companion `PaintRegionRTreeIndex` is built at harvest time

## Read-Only Context

- `crates/slicer-core/src/paint_region.rs` — read in full (~130-160 lines post-packet-62) — purpose: understand the current `point_in_paint_region` signature and inner loop to replace with R-tree lookup
- `crates/slicer-host/src/dispatch.rs` — read `harvest_paint_segmentation_ir` body only (lines ~2003-2085, updated by packet 62) — purpose: insertion point for R-tree construction
- `crates/slicer-host/src/slice_postprocess.rs` — read `SlicePostProcessPaintAnnotationRequest` struct and `execute_slice_postprocess_paint_annotation` body (lines ~1-30 for request struct, ~286-492 for loop) — purpose: where to add the `rtree_index` field and thread it through

## Out-of-Bounds Files

- `OrcaSlicerDocumented/` — delegate parity checks; never load
- `target/`, `Cargo.lock` — never load
- `crates/slicer-host/src/slice_postprocess.rs` — the annotation loop is not changed by this packet
- `crates/slicer-host/tests/` (all test files) — except those explicitly listed in verification commands

## Expected Sub-Agent Dispatches

- "Summarize `rstar::RTree::locate_in_envelope` API shape from the rstar 0.12 docs; return FACT (≤ 5 lines describing the method signature and return type)" — purpose: confirm the query API before writing code
- "Run `cargo test -p slicer-core paint_region`; return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines)" — purpose: validate Step 3 R-tree query
- "Run `cargo test -p slicer-host --test scenario_traces_tdd`; return FACT or SNIPPETS" — purpose: validate Step 3 precedence
- "Run `cargo test -p slicer-host --test core_module_ir_access_contract_tdd`; return FACT or SNIPPETS" — purpose: validate Step 2 deserialization fallback
- "Run `cargo check --workspace`; return FACT pass/fail" — purpose: validate compilation with new dependency

## Data and Contract Notes

- IR contracts touched: `PaintRegionIR` is **unchanged** — no new fields, no schema bump. The `PaintRegionRTreeIndex` is a companion type in `slicer-core`, passed alongside `PaintRegionIR`, not stored on it.
- WIT boundary: unchanged.
- Function signature change: `point_in_paint_region` gains `rtree_index: Option<&PaintRegionRTreeIndex>` parameter. This is a breaking change for any external callers of this public function. Update all call sites: `slice_postprocess.rs` (annotation loop), `is_point_numerically_ambiguous`, and all test fixtures that construct call arguments.
- Determinism: `rstar::RTree` bulk-load is deterministic for the same input order. Input order is deterministic (same group sort from packet 62).
- Memory: One `RTree<(BoundingBox2, usize)>` per `(layer_index, semantic)` key in the `PaintRegionRTreeIndex`. For benchy_4color with ~200 layers and ~4 semantics, ~800 small trees. Each tree contains ~2-20 entries after union-at-harvest. Memory overhead is negligible (~KB).
- Blackboard: `harvest_paint_segmentation_ir` currently returns only `PaintRegionIR`. After this change, it returns a tuple `(PaintRegionIR, PaintRegionRTreeIndex)` or the dispatcher wraps the index alongside the IR. The blackboard `commit_paint_regions` may need to store both, or the index can be threaded directly into the annotation request.

## Locked Assumptions and Invariants

- The query point AABB for `locate_in_envelope` is a zero-area envelope: `AABB::from_corners(point, point)`. This is the standard rstar pattern for point-in-envelope queries.
- `rstar::RTree` uses `RTree::locate_in_envelope` which returns an iterator of `&T`. We store `(BoundingBox2, usize)` and use the `usize` to index into `Vec<SemanticRegion>`.
- The R-tree is rebuilt from scratch at each `harvest_paint_segmentation_ir` call — there is no incremental update. This matches the pipeline model where `PaintRegionIR` is constructed once per run.
- The linear-scan fallback path (when `rtree_index` is `None`) is identical to the packet-62 postcondition. This ensures correctness when running against deserialized IR or when the index is not yet built.
- `rstar` version 0.12 is expected to be WASM-compatible. If it pulls in std-only dependencies, the implementer should try `default-features = false` or pin an earlier compatible version.
- The `PaintRegionRTreeIndex` is `Arc`-wrapped (matching the `Arc<PaintRegionIR>` pattern) for sharing across threads in the parallel query path from packet 62.

## Risks and Tradeoffs

- **New dependency risk**: `rstar` is a new crate dependency. It has a moderate compile time cost. It is widely used and maintained, but any future breaking changes require a coordinated update.
- **R-tree build time at harvest**: Building an R-tree per `(layer, semantic)` key adds O(N log N) construction time at harvest. For ~200 layers × ~4 semantics × ~10 regions = ~8000 entries total, the build time is negligible (< 1 ms). The query-time savings far outweigh this.
- **WASM compatibility**: `slicer-core` is used by WASM guests. If `rstar` pulls in `std::collections` or `alloc` features incompatible with WASM, the implementer must use `default-features = false` or add a `wasm` feature flag that conditionally compiles the R-tree only for host targets.
- **Memory overhead**: One `RTree` per `(layer, semantic)` key. For benchy_4color (~200 layers, ~4 semantics), this is ~800 trees. Each tree is small (~10-20 entries). Total memory is ~few KB. For large models with many layers and many materials, the tree count scales linearly but each tree remains small after union-at-harvest.

## Context Cost Estimate

- Aggregate (sum across all steps): `M` (Step 1: S, Step 2: M, Step 3: M, Step 4: S)
- Largest single step: `M` (Step 3: R-tree query replacement — requires reading the full paint_region.rs query path and replacing the central iteration loop)
- Highest-risk dispatch: `cargo check --workspace` with new `rstar` dependency — if WASM compatibility fails, this dispatch reveals it immediately. Return format: FACT pass/fail.

## Open Questions

- [FWD] Is `rstar` 0.12 WASM-compatible without feature tweaks? The implementer should try `cargo check -p slicer-core --target wasm32-unknown-unknown` after adding the dependency. If it fails, add `default-features = false` or a conditional compilation gate.
- [FWD] Should the R-tree be stored at the `PaintRegionIR` level or on `LayerPaintMap`? The design proposes `PaintRegionIR` level for simpler serde-skip. The implementer may find that `LayerPaintMap`-level storage yields a cleaner API; either is acceptable as long as the index is not serialized.
- None activation-blocking.
