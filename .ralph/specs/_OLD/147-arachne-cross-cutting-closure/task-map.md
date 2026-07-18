# Task Map: 147-arachne-cross-cutting-closure

This packet has `task_ids: none` (the cross-cutting closure packet for the
N1–N13 chain). The task map documents the crosswalk for the chain closure +
the 7 deferred parity-audit findings.

## Parity-Audit Findings Crosswalk

| Finding | ID | Status | Files | Step |
| --- | --- | --- | --- | --- |
| `is_closed` pre-stitch | D-147-PARITY-AUDIT-FINDINGS #1 | Deferred (reverted this session — hexagon regression) | `generate_toolpaths.rs`, `stitch.rs` | Step 1 |
| `has_bead` sub-run split | D-147-PARITY-AUDIT-FINDINGS #2 | Deferred (PRIME open-ring blocker) | `generate_toolpaths.rs` | Step 1 |
| `filter_noncentral_regions` 4 deviations | D-147-PARITY-AUDIT-FINDINGS #3 | Deferred | `centrality.rs` | Step 2 |
| `connectJunctions` merge divergence | D-147-PARITY-AUDIT-FINDINGS #4 | Deferred | `generate_toolpaths.rs` | Step 3 |
| `connectJunctions` is_odd predicate | D-147-PARITY-AUDIT-FINDINGS #5 | Deferred | `generate_toolpaths.rs` | Step 3 |
| `generateJunctions` transition interpolation | D-147-PARITY-AUDIT-FINDINGS #6 | Deferred | `generate_toolpaths.rs` or `pipeline.rs` | Step 4 |
| `collapseSmallEdges` Pattern B | D-147-PARITY-AUDIT-FINDINGS #7 | Deferred | `graph.rs` | Step 4 |

## `docs/07` Crosswalk

| `docs/07` row | Packet | Status | This packet's relationship |
| --- | --- | --- | --- |
| M2 — P110..P113c (Real Arachne foundations + graph construction) | 110..113c | implemented | F closes the chain that extends 113c's faithful graph construction with canonical emission + transitions + post-process (A1–E). |
| M2 — P141/P142/P143/P144/P145/P146 (no TASK-###) | A1/A2/B/C/D/E | draft → implemented | F depends on ALL of A1–E strictly; F cannot close until they are `status: implemented`. |
| M2 — Real Arachne N1–N13 parity | (this chain) | — | F fixes the 7 deferred parity-audit findings (the cross-cutting closure) + records the chain closure in `docs/07_implementation_status.md` (M2 Real Arachne N1–N13 parity complete). |

## `docs/DEVIATION_LOG.md` Crosswalk

| Entry | Status | This packet's action |
| --- | --- | --- |
| `D-141-JUNCTION-BANDS` | Closed (A1) | F adds a one-line addendum noting the chain is closed. No in-place edits. |
| `D-142-CONNECTJUNCTIONS-EMISSION` | Closed (A2) | F adds a one-line addendum noting the chain is closed. |
| `D-143-TRANSITION-ENDS` | Closed (B) | F adds a one-line addendum noting the chain is closed. |
| `D-144-ANGLE-FUDGE-NONCENTRAL` | Closed (C) | F adds a one-line addendum noting the chain is closed. |
| `D-145-LOCAL-MAXIMA-EPILOGUE` | Closed (D) | F adds a one-line addendum noting the chain is closed. |
| `D-146-POSTPROCESS-ORDER` | Closed (E) | F adds a one-line addendum noting the chain is closed. |
| `D-147-CHAIN-CLOSURE` (NEW) | — | F creates this entry documenting the chain closure (all N1–N13 fixes in place, e2e closure gate green, `cargo xtask test --workspace --summary` PASS). |
| `D-147-PARITY-AUDIT-FINDINGS` | Open (deferred to packet 147) | F updates this entry to Closed — all 7 findings fixed. |
| `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` | Closed (113c) | Untouched by F (113c's chain is its own; F's ADR 0035 references it as the predecessor). |

## OrcaSlicer Refs by Step

F owns 7 parity-audit finding fixes, each with OrcaSlicer ground-truth refs (all delegated):

| Step | OrcaSlicer Refs (delegated) |
| --- | --- |
| Step 1 (has_bead + is_closed) | `SkeletalTrapezoidation.cpp:2198-2234`, `:2273-2366`, `WallToolPaths.cpp:790-803`, `PolylineStitcher.hpp` |
| Step 2 (filter_noncentral_regions) | `SkeletalTrapezoidation.cpp:811-866` |
| Step 3 (connectJunctions merge + is_odd) | `SkeletalTrapezoidation.cpp:2302-2327`, `:2344-2354` |
| Step 4 (collapseSmallEdges + transition interp) | `SkeletalTrapezoidationGraph.cpp:310-431`, `SkeletalTrapezoidation.cpp:2091-2127` |
| Step 5 (closure artifacts) | None (ADR 0035 references the chain's parity surface but introduces no new refs) |

## ADR Crosswalk

| ADR | Status | This packet's action |
| --- | --- | --- |
| `docs/adr/0034-arachne-faithful-graph-construction.md` | Existing (113c) | F reads it (short); ADR 0035 follows it as the next free number. |
| `docs/adr/0035-arachne-faithful-emission-and-transitions.md` (NEW) | — | F authors this ADR recording the chain's architectural decision: canonical `generateJunctions`/`connectJunctions` emission (A1/A2), transition ends + `generateExtraRibs` (B), `filterNoncentralRegions` + configured angle (C), local maxima + construction epilogue (D), canonical post-process order (E), superseding the PNP "ADAPTATION" divergence documented in 113c's ADR 0034. |