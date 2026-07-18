---
status: implemented
packet: 113c-arachne-faithful-graph-construction
task_ids: []
---

# 113c-arachne-faithful-graph-construction

## Goal

Replace `arachne-perimeters`'s graph-construction and junction-connection layer with a faithful
port of OrcaSlicer's real algorithm so outer walls actually close into loops. A `/diagnose`
session (2026-07-05) traced the visual artifacts reported against `resources/cube_4color.3mf`
(holes, gaps, a stray floating bar when sliced with `wall_generator=arachne`) to a confirmed,
systemic defect: 100% of outer-wall gcode segments fail to close (283/283 non-closed headers,
mean gap 18.7mm), reproduced in isolation via `run_arachne_pipeline` on a single simple polygon,
and reproduced even for a bare 10mm square (the existing `arachne_perimeters_simple_square` test's
own doc comment admits 26 fragmented lines for one square). Root cause:
`crates/slicer-core/src/skeletal_trapezoidation/graph.rs::from_polygons` copies `next`/`prev`/
`twin` verbatim from the raw `boostvoronoi` per-Voronoi-cell DCEL — topologically wrong for
spine-walking at every junction/branch vertex, which exists in every non-trivial polygon. Real
OrcaSlicer's `constructFromPolygons` builds the graph per-cell with a rib edge interleaved after
*every* transferred edge (not just at reflex corners, as this codebase's `rib.rs` from packet
113b assumes); that interleaving is what lets `getNextUnconnected()` traverse junctions of any
degree. This packet ports that real construction faithfully, reworks `connectJunctions`
accordingly, re-validates every downstream Arachne stage against the new graph shape, and
corrects two deviation-log entries whose prior "Closed" status was based on insufficient interim
fixes rather than the real defect.

## Problem Statement

Packet 113b's `build_quad_rib_topology` (`crates/slicer-core/src/skeletal_trapezoidation/rib.rs`)
only inserts rib edges at reflex/sharp polygon corners — an admitted "Step 1 minimal"
implementation that was never upgraded to the full algorithm. Real OrcaSlicer's
`constructFromPolygons`/`makeRib` inserts a rib edge pair after **every** transferred edge,
interleaved directly into the `next`/`prev` chain; that interleaving is what lets
`getNextUnconnected()` traverse junctions of any degree. Because this codebase's
`SkeletalTrapezoidationGraph::from_polygons` instead copies `next`/`prev`/`twin` verbatim from
the raw `boostvoronoi` per-cell DCEL (which encodes "walk around one Voronoi cell's own
boundary," not "continue along the medial-axis spine"), `generate_toolpaths.rs`'s domain-walk
breaks at every junction/branch vertex — present in every non-trivial polygon, including a
plain square's single central X-junction. A `/diagnose` session (2026-07-05) confirmed this is
not an edge case: 100% of outer-wall gcode segments for `resources/cube_4color.3mf` fail to
close (283/283, mean gap 18.7mm), reproduced in isolation via `run_arachne_pipeline` on a single
polygon, and reproduced even for a bare 10mm square (the existing
`arachne_perimeters_simple_square` test's own doc comment admits 26 fragmented lines). This
mis-diagnosis traces to packet 113b's OrcaSlicer-read-delegation protocol: its `makeRib`
dispatch asked for a SUMMARY of the callee's ~30-line body in isolation, never
`constructFromPolygons`'s caller loop — the single fact (called after every edge, not just at
corners) that would have caught this. `docs/DEVIATION_LOG.md`'s `D-112-MMU-TOPOLOGY` and
`D-113B-CONNECTJUNCTIONS` entries both show `Closed`, but neither closure touched graph
construction: `D-112-MMU-TOPOLOGY`'s 11th-pass closure was a test-harness realignment (gcode
header re-pairing to eliminate a sampling-aliasing artifact); `D-113B-CONNECTJUNCTIONS`'s
closure was the central-only domain-walk generalization now proven insufficient (breaks at
every rib). This packet replaces the graph-construction and junction-connection layer with a
faithful port of the real algorithm, re-validates every downstream Arachne stage against the
new graph shape, and corrects the deviation-log record.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this
  packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"),
  the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported,
  rebuild without `--check` before re-running the failing test. Note: this packet's change
  surface is entirely host-side (`slicer-core`); no WIT or module edits, so guest staleness is
  not expected. The freshness check is run as a precaution, same as packet 113b.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer
  constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary.
  Full porting checklist in `docs/08_coordinate_system.md`.

- **Degeneracy-handling contract stays in force.** `docs/adr/0023-arachne-port-strategy.md`'s
  degeneracy table (collinear input, T-junctions, duplicate vertices, near-collinear-within-
  `epsilon_offset` segments) is a `preprocess.rs`-level contract that Step 3's new construction
  must keep honoring — this packet changes how the graph is built ON TOP of already-preprocessed
  segments, not the preprocessing contract itself.

- **Single point of failure:** Step 3 (faithful graph construction) is the only structural
  dependency for Steps 4-10. If Step 3 produces incorrect topology, every downstream step fails.
  The implementer MUST run Step 3's own tests (AC-3, AC-N1, AC-N3) and confirm CLEAN before
  proceeding to Step 4.

- **No schema bump:** topology changes are internal to `skeletal_trapezoidation`/`arachne`, not
  `slicer-ir`. `ExtrusionLine`/`ExtrusionJunction` shapes from packet 112 are unchanged.

## Data and Contract Notes

- **`VCell` is additive**, mirroring `boostvoronoi::Cell` — no existing `HalfEdge`/`Vertex`
  fields change shape in Step 1.
- **`STHalfEdge`'s `next`/`prev`/`twin` semantics change meaning in Step 3**: they no longer
  mirror the raw `boostvoronoi` DCEL 1:1 (as the current doc comment claims) — they encode the
  freshly-constructed per-cell-chain-with-interleaved-ribs topology instead. Any code outside
  this packet's scope that assumed the old "1:1 mirror" semantics (there should be none, since
  `graph.rs` is the sole consumer of the raw `voronoi.rs` output) must be checked during Step 3.
- **No IR schema bump**: `ExtrusionLine`/`ExtrusionJunction` from packet 112 are unchanged by
  this packet — only their upstream construction path changes.
- **No WIT changes**: this packet's entire surface is `slicer-core` internals.
- **Determinism**: the new graph construction must remain deterministic (AC-N3, carried forward
  from packet 113b's AC-N4) — iterate cells/edges in stable index order, no `HashMap` iteration
  order dependence.

## Locked Assumptions and Invariants

- The per-cell chain + interleaved-rib construction is a faithful port of OrcaSlicer's
  `constructFromPolygons`/`transferEdge`/`makeRib` — verified via the relaxed-contract dispatches
  in Steps 2-3 (which MAY include code excerpts) plus this design's own pre-seeded mechanics
  (see §Verified Algorithm Mechanics), and via code review.
- `getNextUnconnected`'s dead-end-then-twin mechanism requires no per-hop centrality filtering —
  unlike the current (incorrect) `walk_domain_chain`, which filters every hop by
  `edge_junctions.contains_key` and therefore breaks at every rib.
- The 2-3 edge "quad" granularity in `connectJunctions` is real, not an implementation
  convenience — `getQuadMaxRedgeTo` operates within one such short quad per iteration, not
  across a whole domain.
- Fixture re-baselining across Steps 5-9 is accepted as self-captured regression-locking, per
  the established `D-112-SELFCAPTURED-BASELINES`/`D-109-SELF-CAPTURED-FIXTURES` precedent — no
  OrcaSlicer binary exists to validate output against. The algorithm-faithfulness criterion is
  asserted via direct OrcaSlicer code references and the invariant suite (AC-8), not output
  match.
- `remove_small_lines`'s primary preservation invariant (`is_closed && inset_idx == 0` never
  removed) must survive the topology change unchanged (AC-N2, carried forward from packet 113b).

## Risks and Tradeoffs

- **Single point of failure (Step 3):** if the per-cell + rib construction is wrong, every
  downstream step fails. Mitigation: AC-3 + AC-N1 + AC-N3 are the cheapest early failure
  checks (does a square close into one ring? does it have ribs? is it deterministic?) — if any
  fails, Step 3 is broken and Step 4 cannot proceed. Internal (non-gated) checkpoints inside
  Step 3 further de-risk this (see `implementation-plan.md` Step 3).
- **Cell-range-walk complexity underestimated:** if Step 2's spike finds that a raw
  incident-edge cycle walk does NOT suffice (i.e. `compute_point_cell_range`/
  `compute_segment_cell_range`'s extra logic is genuinely required), Step 3's scope grows.
  Mitigation: absorb this inside Step 3's internal checkpoints rather than adding a new packet
  step (per this packet's own grilling decision).
- **`insert_node`'s bug history:** three compounding DCEL bugs were found here under the OLD
  topology. Mitigation: Step 6 is a dedicated, gated step with its own regression test (≥2
  same-edge splits near a rib insertion), not folded into Step 5's lower-risk revalidation.
- **Fixture re-baselining locks in undetected port bugs:** with no OrcaSlicer oracle, a subtle
  faithfulness bug could get re-baselined as the new "expected" output. Mitigation: the
  invariant suite (AC-8) checks properties that hold regardless of the specific geometry
  (closure, quad length, junction-count delta) — these cannot be satisfied by a re-baselined
  bug the way a plain snapshot-diff could be.
- **`propagation.rs`/`centrality.rs` coupling to the old rib model:** `centrality.rs` directly
  references `EdgeType::EXTRA_VD`; if its exclusion logic assumed ribs were rare (corner-only),
  it may need adjustment now that ribs are ubiquitous. Mitigation: Step 5's re-validation is
  explicitly scoped to check this, not just re-run existing tests.
- **Deviation log supersession pattern is new to this project** (prior packets always closed
  in place). Mitigation: documented explicitly in `packet.spec.md`'s Doc Impact Statement and
  this design.md, so a future reader understands why `D-112-MMU-TOPOLOGY`/
  `D-113B-CONNECTJUNCTIONS` still show `Closed` with a pointer forward, rather than being
  edited to `Reopened`.
