# Requirements: 63_paint-annotator-r-tree

## Packet Metadata

- Grouped task IDs: none (no open TASK-###; builds on packet 62 infrastructure)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 62 reduces the multiplicative factor in `point_in_paint_region` by unioning per-facet regions, adding AABB pre-filters, caching lookups, and parallelizing contour iteration. However, the region selection loop within `point_in_paint_region` remains a **linear O(N) scan** over all `SemanticRegion` entries for a given `(layer_index, semantic)` key:

```rust
for region in paint_regions.get(layer_index, semantic) {
    if !semantic_region_contains_point(region, point, boundary_inclusion) { continue; }
    // ... winner logic
}
```

After union-at-harvest, a `(layer, semantic)` bucket might contain 2-20 regions (materials, modifiers, fuzzy skin zones). The AABB pre-filter from packet 62 skips polygon containment for regions whose bounding box misses the point, but the outer loop still iterates every region to check its AABB. This is O(N) in region count, where N is the number of distinct `(object_id, value)` combinations per semantic per layer.

OrcaSlicer's `AABBTreeIndirect` (`03_algorithmic_complexities.md`, line 331) provides O(log N) nearest-neighbor and intersection queries via a static balanced tree. This packet adds equivalent O(log N) region lookup by building an `rstar::RTree<BoundingBox2>` index per `(layer_index, semantic)` key at `PaintRegionIR` construction time, then replacing the linear scan with `rtree.locate_in_envelope(&point_aabb)` to obtain only the candidate regions that could contain the point.

## In Scope

- Add `rstar = "0.12"` (or latest compatible) to `crates/slicer-core/Cargo.toml` dependencies
- Add a per-`(layer_index, semantic)` `RTree<BoundingBox2>` (or a struct wrapping it with region index mapping) at `PaintRegionIR` construction time — either on `LayerPaintMap` alongside `semantic_regions` or as a parallel `HashMap<PaintSemantic, RTreeIndex>`
- Build the R-tree at `harvest_paint_segmentation_ir` time (already the single construction site, modified by packet 62)
- Replace the `for region in regions` linear scan in `point_in_paint_region` with an `rtree.locate_in_envelope()` call that returns candidate region indices, then test only those candidates
- Preserve `paint_order` precedence logic: candidates from the R-tree are filtered and sorted by `paint_order` as before
- Fall back to existing AABB-pre-filtered linear scan when the R-tree index is absent (deserialized IR, or `(layer, semantic)` key not found)
- Ensure the R-tree index is NOT serialized — it is reconstruction-only, like `SemanticRegion.aabb`
- Exclude the R-tree index from `PartialEq` for `LayerPaintMap` (if stored on that struct)
- Update `docs/02_ir_schemas.md` with a note about the reconstruction-only spatial index

## Out of Scope

- Replacing `slicer_core::union` with a hole-preserving variant — not needed for this packet
- Adding R-tree to any other query path (only `point_in_paint_region` in `paint_region.rs`)
- Guest WASM changes — the index is host-side only
- Changing the `paint_order` contract or precedence rules
- Changing `SemanticRegion.aabb` computation — that was done in packet 62
- Adding `rstar` to any crate other than `slicer-core`
- Adding an incremental update API for the R-tree — it is rebuilt from scratch at each harvest

## Authoritative Docs

- `docs/02_ir_schemas.md` §"PaintRegionIR" — delegate FACT: what derive macros are on `LayerPaintMap`? Must not include `PartialEq` on the R-tree wrapper or must implement custom `PartialEq` that ignores the index.
- `docs/01_system_architecture.md` — delegate SUMMARY (> 300 lines) of the data ownership model: is `PaintRegionIR` rebuilt per-pipeline, or can it be cached and reused across runs? R-tree rebuild cost is acceptable if harvest runs once per pipeline.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/generated_documentation/03_algorithmic_complexities.md` §"AABB Tree" — OrcaSlicer's `AABBTreeIndirect` build/lookup complexity and query API; parity anchor for O(log N) pattern
- `OrcaSlicerDocumented/generated_documentation/02_core_data_structures.md` §"AABBTree" — if present, the tree envelope type used by OrcaSlicer for comparison

## Acceptance Summary

- Positive cases: `AC-1` through `AC-6` from `packet.spec.md`
  - AC-1: R-tree built with one envelope per region
  - AC-2: only AABB-containing regions tested for polygon containment
  - AC-3: zero-region query returns `Ok(None)` without containment checks
  - AC-4: paint_order precedence preserved with R-tree candidates
  - AC-5: deserialized IR fallback (no index)
  - AC-6: additional wall-clock reduction from O(log N) lookup
- Negative cases: `AC-N1` through `AC-N2` from `packet.spec.md`
  - AC-N1: empty R-tree handled (no query attempted)
  - AC-N2: R-tree not serialized, not in PartialEq

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core paint_region` | AC-1 through AC-3, AC-N1: R-tree construction, candidate filtering, empty-tree | FACT pass/fail |
| `cargo test -p slicer-host --test scenario_traces_tdd` | AC-4: paint_order precedence with R-tree | FACT pass/fail |
| `cargo test -p slicer-host --test core_module_ir_access_contract_tdd` | AC-5: deserialized fallback; AC-N2: no serialized index | FACT pass/fail |
| `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.stl --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html` | AC-6: additional wall-clock reduction | FACT: annotator row time |
| `cargo check --workspace` | Compile gate | FACT pass/fail |
| `cargo clippy --workspace -- -D warnings` | Lint gate | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: after Step 1 (rstar dependency + R-tree type), `cargo check --workspace` must pass — the new crate is present but no code uses it yet.
- Cross-step invariant: after Step 2 (R-tree construction at harvest), the R-tree is built but the query path still uses the linear scan (unchanged). Verify the tree is populated correctly via a unit test that constructs PaintRegionIR and asserts on `rtree.size()`.
- Cross-step invariant: after Step 3 (replace linear scan with R-tree lookup), all existing query results are identical to the packet 62 baseline. The R-tree is a candidate selector, not a decision maker — it narrows the candidate set but does not change which region wins.
- Step ordering rationale: dependency addition (Step 1) must precede all consumers. Construction (Step 2) must precede query replacement (Step 3). The linear scan is kept as a fallback path for deserialized IR throughout.

## Context Discipline Notes

- Large files in the read-only path: `slice_ir.rs` (> 1000 lines) — range-read only the `SemanticRegion`, `LayerPaintMap`, `PaintRegionIR` struct defs. `paint_region.rs` (~130 lines post-packet-62) — read in full.
- Likely temptation reads: `rstar` crate documentation or source — delegate a SUMMARY of the `RTree::locate_in_envelope` API shape; do not browse crates.io or docs.rs directly. `Cargo.lock` — skip; `rstar` version resolution is handled by `cargo update`.
- Sub-agent return-format hints: all `cargo test` dispatches return FACT (pass) or SNIPPETS (fail: test name + assertion + ≤ 20 lines).
