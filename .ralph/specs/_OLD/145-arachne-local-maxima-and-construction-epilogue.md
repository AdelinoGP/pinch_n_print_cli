---
status: implemented
packet: 145-arachne-local-maxima-and-construction-epilogue
task_ids:
  - none
---

# 145-arachne-local-maxima-and-construction-epilogue

## Goal

Port `generateLocalMaximaSingleBeads` (N9 — hexagonal micro-loop for isolated thick spots with odd bead count) and the `constructFromPolygons` construction epilogue (N10 — `separatePointyQuadEndNodes`, `collapseSmallEdges`; incident-edge normalization is a documented no-op since PNP's `STVertex` has no `incident_edge` field), so local maxima that never join a domain chain get their center dot and degenerate zero-length edges / shared quad-start nodes are cleaned up by construction.

## Problem Statement

Two canonical passes are absent (N9 + N10). **N9 (`generateLocalMaximaSingleBeads`
absent):** `SkeletalTrapezoidation.cpp:2383-2413` is the final step of
`generateSegments`: for nodes with odd `beading.bead_widths.size()`,
`isLocalMaximum(true)`, and not central, it emits a 6-segment hexagonal
micro-loop (radius `width/8`, `is_odd = true`) so isolated thick spots get their
center dot. Without it, local maxima that never join a domain chain simply
vanish (pinholes at e.g. the center of near-square regions with odd bead
counts). `grep local_maxima` in PNP finds no hits — the pass is entirely
missing. **N10 (`constructFromPolygons` epilogue missing):** PNP's
`SkeletalTrapezoidationGraph::from_polygons` (`graph.rs:306-371`) ends after
per-edge radius bounds; none of the canonical epilogue passes
(`SkeletalTrapezoidation.cpp:538-546`) exists: (1) `separatePointyQuadEndNodes`
duplicates shared boundary start-nodes so each quad traversal has a unique
start; (2) `graph.collapseSmallEdges()` removes degenerate zero-length edges
produced by integer rounding; (3) incident-edge normalization (each node's
`incident_edge` reset to the first `prev`-less edge) — **this pass is a
documented no-op in PNP** because `STVertex` has no `incident_edge` field
(confirmed by OrcaSlicer ground-truth as a fan-walk optimization, not
correctness; PNP's all-edges scans produce the same results for all 6
canonical read sites). Consequences in PNP: zero-length spine fragments
survive into centrality/junction math (degenerate `edge_length` guards paper
over them: `centrality.rs:167`, `propagation.rs:1042-1044`), and pointy-corner
cells share quad-start nodes, which the `connectJunctions` walk then has to
survive by its defensive "already claimed" break (`generate_toolpaths.rs:699-705`)
instead of by construction. This packet ports both passes — N9 as the final
step of `generate_toolpaths`, N10 as the epilogue of `from_polygons`.

This packet extends `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`'s `from_polygons` with
the canonical epilogue; 113c's per-cell graph construction (Steps 1-3) remains
canonical and untouched. D's epilogue is additive (two real passes + one
documented no-op appended after 113c's existing per-edge radius bounds).

## Architecture Constraints

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- Packet-specific constraint: **D's micro-loops are `is_odd = true` closed `ExtrusionLine`s** (canonical N4 semantics — the centerline bead of an odd-bead-count region). This is consistent with A2's `is_odd` fix (A2 owns the per-segment `is_odd` rule for the main walls; D's micro-loops are a separate emission with `is_odd = true` by construction, not a per-segment computation).
- Packet-specific constraint: **D's epilogue is additive** — three passes appended after 113c's existing per-edge radius bounds. 113c's `from_polygons` Steps 1-3 remain canonical and untouched. D does not re-derive 113c's per-cell construction.
- Packet-specific constraint: **`collapseSmallEdges`'s zero-length ε is a small constant in slicer units.** The implementer confirms the exact value via a delegated SUMMARY of `SkeletalTrapezoidationGraph.cpp`'s `collapseSmallEdges`.
- Packet-specific constraint: **WASM staleness does NOT apply** — D's change surface is `slicer-core`-internal; no path feeds the guest WASM build. The `wasm-staleness` snippet is intentionally omitted.
- Packet-specific constraint: **`incident_edge` is NOT ported — the normalization pass is a documented no-op.** OrcaSlicer ground-truth confirmed `STHalfEdgeNode::incident_edge` is a raw pointer used as the entry point for the fan-walk `edge = edge->twin->next` around a node, read by 6 stages (`isLocalMaximum`, `isCentral`, `isMultiIntersection`, `updateBeadCount`, `getOrCreateBeading`, `getNearestBeading`). PNP replaces ALL of these with all-edges scans (`edges.iter().filter(|e| e.start_vertex == v_idx)`) that visit the same edge set — correctness is preserved, the cost is O(E) per call instead of O(degree(v)). The N10 epilogue's incident-edge normalization (`SkeletalTrapezoidation.cpp:545-546`: "reset each node's `incident_edge` to the first `prev`-less edge") becomes a no-op in PNP because there is no `incident_edge` field to normalize. `separatePointyQuadEndNodes` and `collapseSmallEdges` are still ported (they mutate `prev`/`next`/`twin`/`from`/`to`, which PNP does have); only the incident-edge SET lines in those functions are skipped. See the preflight investigation in `docs/DEVIATION_LOG.md` `D-144a-CENTRALITY-COUPLING-RESOLVED` for the full use-site inventory.

## Data and Contract Notes

- IR or manifest contracts touched: **none**. D's surface is `slicer-core`-internal; no WIT/IR change. D's micro-loops are `ExtrusionLine` with `is_odd = true` (existing field shape).
- WIT boundary considerations: **none**. No WIT/IR schema change.
- Determinism: D's changes preserve determinism (graph walks are index-ordered; the micro-loop emission is a deterministic per-node predicate; `collapseSmallEdges`'s endpoint merge is deterministic under ties via index-ascending).

## Locked Assumptions and Invariants

- D's micro-loops are `is_odd = true` closed `ExtrusionLine`s (canonical N4 semantics). Consistent with A2's `is_odd` fix.
- D's epilogue is additive — three passes appended after 113c's existing per-edge radius bounds. 113c's `from_polygons` Steps 1-3 remain canonical.
- `collapseSmallEdges`'s zero-length ε is a small constant in slicer units (the implementer confirms via delegated SUMMARY).
- D keeps N1, N2, N3, N4 red tests GREEN (gated).
- D's `isLocalMaximum` predicate: a node is a local maximum if all its neighbors have `distance_to_boundary <=` its own.
- Fixture re-baseline uses the self-capture pattern; never read the JSONs directly.

## Risks and Tradeoffs

- **`collapseSmallEdges`'s endpoint merge could ripple into A1's junction fans.** Merging endpoints changes edge topology; A1's `generate_junctions` walks edges. Risk is contained by the N1 red tests (AC-N1 stays green) + the `generate_toolpaths` regression suite.
- **`separatePointyQuadEndNodes`'s node duplication changes the graph's vertex count.** Downstream stages (centrality, bead_count, propagation) must handle the duplicated nodes. Risk is contained by the regression suite (centrality/bead_count/propagation fixtures re-baselined).
- **`generateLocalMaximaSingleBeads`'s micro-loops interact with E's `removeSmallLines`.** D's micro-loops are `is_odd = true` closed lines; E's `removeSmallLines` only removes `is_odd && !is_closed` lines, so closed micro-loops survive. But E's post-process order change (N11) could affect them. D's commit message must flag this for E.
