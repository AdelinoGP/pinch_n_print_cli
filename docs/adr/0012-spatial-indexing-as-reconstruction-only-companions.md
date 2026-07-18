# ADR-0012 — Spatial Indexing as Reconstruction-Only Arc Companions

## Status

Superseded (Packet 95). The `PaintRegionIR` / `SemanticRegion` /
`PaintRegionRTreeIndex` / `point_in_paint_region` machinery this ADR describes
was deleted wholesale in Packet 95 (per-variant polygons are now inlined into
`SliceIR.regions[*]` via `SlicedRegion.variant_chain`; there is no rtree
companion). Retained as a historical record of the reconstruction-only
spatial-index pattern. (Originally: Accepted, Packet 63 / TASK-200a-rtree.)

## Context

Packet 62 introduced an axis-aligned bounding-box (`BoundingBox2`) pre-filter on `SemanticRegion` to short-circuit polygon containment checks in `point_in_paint_region`. The pre-filter eliminated per-facet fragmentation but `point_in_paint_region` still walked every region in a `Vec<SemanticRegion>` for each query — `O(N)` per point, dominated by `pip` setup cost.

Packet 63 needed an `O(log N)` spatial index to drop the per-query cost. Four shapes were considered:

1. **Store the index directly on `PaintRegionIR`.** Forces `slicer-ir` to depend on `rstar` (or similar) and to either skip-serialize the index (breaking round-trip equality) or serialize it (multiplying IR size). Couples the IR contract to a specific spatial-index crate.
2. **A single global tree across all `(layer, semantic)` pairs.** Naive: query path would need to filter by `(layer, semantic)` anyway, so the tree is no better than a tagged Vec.
3. **A custom in-house AABB tree.** Reinventing rstar's bulk-load + R-tree fan-out; no test coverage; portability risk.
4. **Companion type wrapped in `Arc`, built at harvest, never stored on IR.** Matches the access pattern (one query per `(layer, semantic)`), keeps `slicer-ir` free of spatial-index deps, and falls back cleanly to linear scan when the index is absent (deserialised IR, unit tests, etc.).

Option 4 was selected. The decision is a reusable architectural pattern, not a one-off — packet 63 is the first instance and future packets will follow the same shape.

## Decision

**Spatial indexes that accelerate queries on an IR are built at harvest time as `Arc<T>` companions, NOT stored on the IR.** Concretely:

- Index types live in `slicer-core` (or whichever crate owns the algorithm), not in `slicer-ir`.
- Index instances are constructed in the same `harvest_*_ir` function that produces the IR they accelerate (e.g. `harvest_paint_segmentation_ir` builds both `PaintRegionIR` and `PaintRegionRTreeIndex`).
- Index instances are threaded through the `Blackboard` and per-stage request structs as a separate `Arc<IndexType>`.
- IR fields that the index references (e.g. `SemanticRegion.aabb`) carry `#[serde(skip_deserializing, default)]` and are pure reconstruction artefacts: present after harvest, `None` after deserialisation.
- Query helpers (e.g. `point_in_paint_region`) accept `index: Option<&IndexType>` and fall back to linear scan when absent.
- Determinism is preserved: index construction is keyed on the same deterministically-sorted input the IR carries, so rebuilds across runs produce byte-identical query results.

`rstar` is the canonical choice for 2D spatial indexes. Other crates may be used when justified, but the wrapper pattern (Arc companion + serde-skip on derived fields + linear fallback) is mandatory.

## Consequences

- **`slicer-ir` stays dependency-light.** No spatial-index crate appears in its `Cargo.toml`; IR consumers that don't need the index pay nothing.
- **Round-trip equality is straightforward.** `PartialEq` on the IR ignores the skipped fields by construction; serialised fixtures load cleanly even after the index type's API changes.
- **Fallback is real, not theoretical.** Every query path tests the `index: None` branch as part of its standard test suite (deserialised-from-disk fixtures hit it).
- **WASM compatibility must be verified per dependency.** `rstar` was confirmed WASM-compatible with `default-features = false`; future indexing crates must pass the same check before adoption.
- **One-off optimisation patterns are discouraged.** Future packets that need similar indexes (e.g. for `PerimeterIR` walls, `SliceIR` polygons) should reuse this pattern rather than inventing new ones.

## Rejected alternatives

- **Storing the index on the IR.** See context — couples `slicer-ir` to the index crate and breaks round-trip semantics. Rejected.
- **A single global spatial tree.** No real query-path benefit over per-`(layer, semantic)` indexes; rejected.
- **A hand-rolled AABB tree.** Reinvents `rstar`'s bulk-load with no test coverage; rejected unless a future packet identifies a profile-driven need.

## Future reviewers

- Do not migrate `PaintRegionRTreeIndex` back onto `PaintRegionIR` for "locality"; the serde-skip + fallback contract is what keeps deserialised IR loadable.
- New indexes follow the same shape: `Arc<IndexType>` companion, built at harvest, threaded separately, linear fallback.
- If a future query pattern genuinely needs cross-stage shared spatial structure, write a new ADR before deviating.
