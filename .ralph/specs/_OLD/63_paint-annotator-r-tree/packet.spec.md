---
status: implemented
packet: 63_paint-annotator-r-tree
task_ids: []
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Depends on packet 62_paint-annotator-performance for BoundingBox2 type and SemanticRegion.aabb field. This packet adds the rstar R-tree crate and a per-(layer, semantic) spatial index at PaintRegionIR construction time.
---

# Packet Contract: 63_paint-annotator-r-tree

## Goal

Replace the linear O(N) `for region in paint_regions.get(...)` scan in `point_in_paint_region` with an O(log N) `rstar::RTree<BoundingBox2>` spatial index lookup per `(layer_index, semantic)` key, built once at `PaintRegionIR` construction and queried via `locate_in_envelope()`.

## Scope Boundaries

This packet adds the `rstar` crate to `slicer-core` dependencies, defines a `PaintRegionRTreeIndex` companion type built alongside `PaintRegionIR` at harvest time, and replaces the linear region iteration in `point_in_paint_region` with an R-tree candidate lookup that selects only the regions whose AABB contains the query point. It depends on the `BoundingBox2` type and `SemanticRegion.aabb` field introduced in packet 62. The index is not stored on the IR itself (avoiding an `rstar` dependency in `slicer-ir`); it is passed through the blackboard and annotation request as a companion `Arc`. The query path changes are limited to `paint_region.rs` and `slice_postprocess.rs`.

## Prerequisites and Blockers

- Depends on: `62_paint-annotator-performance` (provides `BoundingBox2` type and `SemanticRegion.aabb` field)
- Unblocks: nothing downstream
- Activation blockers: None

## Acceptance Criteria

- **AC-1. Given** a `PaintRegionIR` constructed from a benchy_4color run, **when** the R-tree index is built per `(layer_index, semantic)` key, **then** the index contains one envelope per `SemanticRegion` (matching its `aabb`). | `cargo test -p slicer-core paint_region`

- **AC-2. Given** a query point inside exactly one `SemanticRegion`'s AABB, **when** `point_in_paint_region` is called, **then** only regions whose AABB contains the point are tested for polygon containment — regions whose AABB does not contain the point are never passed to `semantic_region_contains_point`. | `cargo test -p slicer-core paint_region`

- **AC-3. Given** a query point outside all region AABBs, **when** `point_in_paint_region` is called, **then** no `semantic_region_contains_point` call is made and the result is `Ok(None)`. | `cargo test -p slicer-core paint_region`

- **AC-4. Given** two overlapping regions with different `paint_order` values, **when** an R-tree query returns both as candidates, **then** the `paint_order` precedence logic still determines the winner (same as the pre-R-tree path). | `cargo test -p slicer-host --test scenario_traces_tdd`

- **AC-5. Given** a `PaintRegionIR` built without a companion `PaintRegionRTreeIndex` (deserialized from disk, or index not built), **when** `point_in_paint_region` is called with `rtree_index: None`, **then** the function falls back to the existing linear-scan-with-AABB-pre-filter path from packet 62 — no panic on missing index. | `cargo test -p slicer-host --test core_module_ir_access_contract_tdd`

- **AC-6. Given** the end-to-end benchy_4color pipeline run, **when** the `--report` HTML is inspected, **then** the `Layer::SlicePostProcess` wall-clock time (which includes paint annotation) is further reduced relative to the packet-62 baseline (additional 20-50% reduction expected from O(N) → O(log N) region selection). | `cargo run --bin slicer-host --release -- run --model resources/benchy_4color.3mf --module-dir modules/core-modules --output /tmp/out.gcode --report /tmp/slicer-report.html`

## Negative Test Cases

- **AC-N1. Given** a `PaintRegionIR` with zero regions for a `(layer_index, semantic)` key, **when** `point_in_paint_region` is called, **then** the empty-tree path returns `Ok(None)` without any candidate iteration — no attempt to query an empty R-tree. | `cargo test -p slicer-core paint_region`

- **AC-N2. Given** `PaintRegionIR` serialization/deserialization tests, **when** round-tripped through serde, **then** `PaintRegionIR` is identical before and after — the companion `PaintRegionRTreeIndex` is NOT part of the IR and does not affect serialization. | `cargo test -p slicer-host --test core_module_ir_access_contract_tdd`

## Verification

- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo test -p slicer-core paint_region`
- `cargo test -p slicer-host --test scenario_traces_tdd`

## Authoritative Docs

- `docs/02_ir_schemas.md` §"PaintRegionIR" — confirm `LayerPaintMap` and `PaintRegionIR` derive macros. Delegate FACT.
- `docs/01_system_architecture.md` — data ownership model. Delegate SUMMARY (> 300 lines).

## Doc Impact Statement

1. `docs/02_ir_schemas.md` §"PaintRegionIR" — add note that a companion `PaintRegionRTreeIndex` is built at `harvest_paint_segmentation_ir` time (not stored on the IR itself) and threaded through the pipeline to accelerate region queries. | `rg -q 'PaintRegionRTreeIndex\|R-tree\|spatial index' docs/02_ir_schemas.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/generated_documentation/03_algorithmic_complexities.md` §"AABB Tree" — AABBTreeIndirect build/lookup complexity; parity anchor for O(log N) region queries

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
