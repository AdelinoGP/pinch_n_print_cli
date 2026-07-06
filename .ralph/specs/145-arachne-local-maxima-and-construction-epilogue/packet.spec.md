---
status: draft
packet: 145-arachne-local-maxima-and-construction-epilogue
task_ids:
  - none
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 145-arachne-local-maxima-and-construction-epilogue

## Goal

Port `generateLocalMaximaSingleBeads` (N9 — hexagonal micro-loop for isolated thick spots with odd bead count) and the `constructFromPolygons` construction epilogue (N10 — `separatePointyQuadEndNodes`, `collapseSmallEdges`, incident-edge normalization), so local maxima that never join a domain chain get their center dot and degenerate zero-length edges / shared quad-start nodes are cleaned up by construction.

## Scope Boundaries

Add `generateLocalMaximaSingleBeads` as the final step of `generate_toolpaths` (N9), and add the three-pass construction epilogue to `from_polygons` in `graph.rs` (N10). Full in/out-of-scope lists live in `requirements.md`.

## Prerequisites and Blockers

- Depends on: `144-arachne-angle-fudge-and-noncentral-regions` (C — D's `generateLocalMaximaSingleBeads` reads the normalized centrality; C's `filterNoncentralRegions` must land first so local-maxima detection runs on the corrected central set).
- Unblocks: `146-arachne-postprocess-order-and-remove-small-simplify` (E — E's `removeSmallLines` interacts with the micro-loops D emits).
- Activation blockers: none.

## Acceptance Criteria

Acceptance Criteria are stated **once**, here. `requirements.md` references them by ID, never copies them.

- **AC-1. Given** a near-square region with an odd bead count whose center is a local maximum (`isLocalMaximum(true)`, not central), **when** `generate_toolpaths` runs, **then** the output contains a 6-segment hexagonal micro-loop (radius `width/8`, `is_odd = true`) at the local maximum — `generateLocalMaximaSingleBeads` (`SkeletalTrapezoidation.cpp:2383-2413`) emits the center dot so isolated thick spots don't vanish.
  | `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --nocapture 2>&1 | tee target/test-output-d-ac1.log`
- **AC-2. Given** a polygon whose Voronoi diagram produces degenerate zero-length edges (integer rounding) and pointy-corner cells sharing quad-start nodes, **when** `SkeletalTrapezoidationGraph::from_polygons` runs, **then** (1) no edge has zero length (`collapseSmallEdges` removed them), (2) each node's `incident_edge` is the first `prev`-less edge (incident-edge normalization), and (3) pointy-corner quad-start nodes are unique per quad (`separatePointyQuadEndNodes`).
  | `cargo test -p slicer-core --features host-algos --test arachne_construction_epilogue --nocapture 2>&1 | tee target/test-output-d-ac2.log`

## Negative Test Cases

- **AC-N1. Given** the construction epilogue is in place, **when** `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast` runs, **then** N1 red tests stay GREEN — the epilogue's `collapseSmallEdges` + incident-edge normalization don't regress A1's junction placement.
  | `cargo test -p slicer-core --features host-algos --test arachne_parity_red_junction_bands --no-fail-fast 2>&1 | tee target/test-output-d-neg1.log`

## Verification

Gate commands only — the 2–3 commands the preflight / closure gate runs. The full verification matrix lives in `requirements.md` §Verification Commands.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --features host-algos --test arachne_local_maxima_single_beads --test arachne_construction_epilogue --no-fail-fast 2>&1 | tee target/test-output-d-gate.log`

## Authoritative Docs

- `docs/02_ir_schemas.md` — §"Arachne extrusion-line geometry (Packet 112)" (lines ~1091-1150); `ExtrusionLine::is_odd` field shape (D's micro-loops are `is_odd = true`).
- `docs/08_coordinate_system.md` — §"Constant Conversion Table" (~30 lines); `width/8` radius conversion.
- `docs/DEVIATION_LOG.md` `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry — read full; D's epilogue extends 113c's `from_polygons`.
- `docs/specs/arachne-parity-N1-N13-plan.md` — read full; cross-packet policies.

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `docs/DEVIATION_LOG.md` — new entry `D-145-LOCAL-MAXIMA-EPILOGUE` documenting the N9+N10 fix, with an addendum on `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` noting D extends 113c's `from_polygons` with the canonical epilogue. Supersession pattern.
  - `rg -q 'D-145-LOCAL-MAXIMA-EPILOGUE' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2383-2413` — `generateLocalMaximaSingleBeads` (6-segment hexagonal micro-loop, radius `width/8`, `is_odd = true`, for odd-bead-count local maxima that are `isLocalMaximum(true)` and not central).
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:538-546` — `constructFromPolygons` epilogue: `separatePointyQuadEndNodes`, `collapseSmallEdges`, incident-edge normalization.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp` — `collapseSmallEdges` + `separatePointyQuadEndNodes` implementations (delegate for exact signatures).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.