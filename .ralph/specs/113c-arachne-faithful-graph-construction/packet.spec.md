---
status: active
packet: 113c-arachne-faithful-graph-construction
task_ids: []
backlog_source: docs/DEVIATION_LOG.md D-112-MMU-TOPOLOGY (line 50), D-113B-CONNECTJUNCTIONS (line 56); /diagnose session 2026-07-05 on resources/cube_4color.3mf visual artifacts
context_cost_estimate: L
---

# Packet Contract: 113c-arachne-faithful-graph-construction

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

## Scope Boundaries

This packet owns: exposing per-cell Voronoi metadata (Step 1); a research spike resolving open
questions about cell-range walking before the big rewrite (Step 2); the L-effort faithful
per-cell graph construction with interleaved rib insertion (Step 3, single point of failure for
the packet); the faithful `connectJunctions` domain-walk rework (Step 4); re-validation of
`centrality.rs`/`bead_count.rs` (Step 5); a dedicated re-audit of `propagation.rs::insert_node`
given its prior bug history (Step 6); re-validation of `stitch.rs`/`simplify.rs`/
`remove_small.rs` (Step 7); a faithfulness invariant suite plus selective `test_voronoi.cpp`
triage (Step 8); fixture re-baselining, deviation-log correction, and glossary updates (Step 9);
and end-to-end verification against `resources/cube_4color.3mf` (Step 10, closes the original
user-facing bug).

Out of scope: building the real OrcaSlicer C++ checkout to generate oracle golden fixtures
(considered and explicitly declined — see `design.md` Rejected Alternatives); splitting this
packet into 113c/113d (considered and declined — every step cascades sequentially from Step 3,
unlike 113a/113b's independent-items split); classic-perimeters (M1, frozen); spiral-vase and
non-planar (orthogonal sibling roadmaps).

## Prerequisites and Blockers

- Depends on: packet 113b (`status: implemented`) for the existing (now superseded) Arachne
  pipeline source, fixtures, and host-service bridge; the `/diagnose` session's root-cause
  finding (this packet's own provenance, not a separate numbered packet).
- Activation: no other packet currently holds `status: active` in `.ralph/specs/` (verified
  2026-07-05) — this packet activates immediately with `status: active`, no blocker.
- **L-step exception:** Step 3 (faithful per-cell graph construction) is genuinely L-effort with
  no natural split point — spine-chain construction and rib interleaving are mutually
  dependent, so building spine-only first and "adding ribs later" reproduces exactly the failure
  mode this packet fixes. This mirrors packet 113b's own L-exception for its `makeRib` step
  (`.ralph/specs/113b-arachne-topology-faithfulness/packet.spec.md` §Prerequisites and
  Blockers) — same category of exception, same justification, re-confirmed during this
  packet's grilling session. If subsequent design work surfaces a natural split point, the
  packet SHOULD be split before Step 3 begins.

## Acceptance Criteria

- **AC-1.** `crates/slicer-core/src/voronoi.rs` exposes per-cell metadata (`contains_point`,
  `contains_segment`, `contains_segment_startpoint`, `contains_segment_endpoint`,
  `source_index`, `source_category`, `get_incident_edge`, `is_degenerate`) mirroring
  `boostvoronoi::Cell`, via a new `VCell` type and `HalfEdgeGraph::cells`. | `cargo test -p slicer-core --features host-algos --test voronoi -- voronoi_cells_square_metadata 2>&1 | tee target/test-output-voronoi-cells.log`
- **AC-2.** A written cell-range-walk research note exists in `design.md` (§Step 2 Spike
  Findings) answering: (a) whether a raw `incident_edge → next → …` cycle walk suffices for
  cell-range determination or extra logic is needed; (b) whether the `source_index()`
  shared-vertex dedup ambiguity (documented in this packet's `design.md`) breaks provenance
  resolution and how the side-table design tolerates it. | `rg -q 'Step 2 Spike Findings' .ralph/specs/113c-arachne-faithful-graph-construction/design.md`
- **AC-3.** `SkeletalTrapezoidationGraph::from_polygons` builds the real per-cell chain +
  interleaved-rib topology (faithful `transferEdge`/`makeRib` port): a rib edge pair is inserted
  after every transferred edge (not just at reflex corners), adjacent cells' chains are
  cross-twinned, and a plain 10mm square's central-edge domain closes into exactly one ring. |
  `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- square_domain_closes_into_one_ring 2>&1 | tee target/test-output-graph-construction.log`
- **AC-4.** `generate_toolpaths.rs` replaces the central-only `walk_domain_chain` gate with a
  faithful `connectJunctions`/`getNextUnconnected`/`getQuadMaxRedgeTo` quad-by-quad stitch. A
  simple closed polygon's outer wall (`inset_idx == 0`) has `is_closed == true`. |
  `cargo test -p slicer-core --features host-algos --test generate_toolpaths -- outer_wall_closes_for_simple_polygon 2>&1 | tee target/test-output-connectjunctions.log`
- **AC-5.** `centrality.rs`'s `dR < dD·sin(angle/2)` predicate and `bead_count.rs`'s per-NODE
  assignment are re-validated (and adjusted if needed) against the new graph shape; their
  existing fixtures are re-recorded and green. |
  `cargo test -p slicer-core --features host-algos --test centrality --test bead_count 2>&1 | tee target/test-output-centrality-beadcount.log`
- **AC-6.** `propagation.rs::insert_node`'s DCEL rewiring is re-audited against the new
  interleaved-rib chain shape (given its 3-compounding-bug history under the old topology per
  `D-112-MMU-TOPOLOGY`'s 6th pass), with a dedicated regression test covering at least 2
  same-edge splits near a rib insertion. |
  `cargo test -p slicer-core --features host-algos --test propagation -- same_edge_splits_near_rib_insertion 2>&1 | tee target/test-output-insert-node.log`
- **AC-7.** `stitch.rs`/`simplify.rs`/`remove_small.rs` are re-validated: `stitch_extrusions`'s
  proximity-bridge path is unreached on the square and tapered-wedge fixtures (rings now arrive
  pre-closed from AC-4); Visvalingam-Whyatt and length-based removal behave sanely on the
  now-longer closed lines; primary preservation invariant (`is_closed && inset_idx == 0` never
  removed) still holds. |
  `cargo test -p slicer-core --features host-algos --test stitch --test simplify --test remove_small 2>&1 | tee target/test-output-downstream.log`
- **AC-8.** A faithfulness invariant suite asserts properties documented directly in the
  OrcaSlicer C++ source: quad chains span 2-3 edges; `getNextUnconnected` terminates within a
  bounded number of steps (no infinite loop); `|from_junctions.len() - to_junctions.len()| <= 1`
  at every junction stitch. A triage note (file:line provenance) identifies which
  `test_voronoi.cpp` degenerate-input cases (if any) were ported as `voronoi.rs`/`preprocess.rs`
  fixtures. |
  `cargo test -p slicer-core --features host-algos --test arachne_invariants 2>&1 | tee target/test-output-invariants.log`
- **AC-9.** `cube_4color_arachne_per_color_footprint_within_bbox`
  (`crates/slicer-runtime/tests/executor/cube_4color_arachne.rs`) is strengthened in place: its
  structural assertions (finite coordinates, 4 distinct color fragments) are kept, and its
  weakened bbox-with-tolerance check is replaced with a hard `is_closed == true` / near-zero-gap
  closure assertion. All self-captured Arachne fixtures affected by the topology change are
  re-recorded (at minimum: `tapered_wedge`, `narrow_strip_widening`, `max_bead_count_cap`,
  `complex_multi_feature`, `cube_4color_arachne`). |
  `cargo test -p slicer-runtime --test executor -- cube_4color_arachne 2>&1 | tee target/test-output-cube4color-strengthened.log`
- **AC-10.** Re-slicing `resources/cube_4color.3mf` with `wall_generator=arachne` and parsing
  the resulting gcode's `;TYPE:Outer wall` headers per layer shows 0% closure failures (down
  from the pre-packet 100%/283 documented in this packet's provenance). |
  `cargo test -p slicer-runtime --test executor -- cube_4color_arachne_outer_walls_close_end_to_end 2>&1 | tee target/test-output-e2e-closure.log`

## Negative Test Cases

- **AC-N1.** A plain square input produces multiple rib edges (not zero) — replaces packet
  113b's `quad_rib_topology_square_has_no_ribs`, which encoded an incorrect expectation (real
  OrcaSlicer's `makeRib` runs unconditionally after every transferred edge, not just at reflex
  corners). |
  `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- square_produces_multiple_ribs 2>&1 | tee target/test-output-rib-square-neg.log`
- **AC-N2.** `remove_small_lines` does NOT remove any `ExtrusionLine` where `is_closed == true
  && inset_idx == 0`, regardless of length — invariant preserved across the topology change
  (carried forward from packet 113b's AC-N3). |
  `cargo test -p slicer-core --features host-algos --test remove_small -- remove_small_lines_all_primary_invariant 2>&1 | tee target/test-output-remove-neg.log`
- **AC-N3.** The new per-cell graph construction pass is deterministic: two runs on the same
  input produce identical rib edges, chain structure, and cell assignments (carried forward from
  packet 113b's AC-N4, re-targeted at the new construction). |
  `cargo test -p slicer-core --features host-algos --test skeletal_trapezoidation -- graph_construction_is_deterministic 2>&1 | tee target/test-output-deterministic-neg.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check` (CLEAN — this packet edits `slicer-core/src/{voronoi.rs,
  skeletal_trapezoidation/*.rs, arachne/*.rs}`, which feed the host-side pipeline; no WIT
  changes)
- `cargo xtask test --workspace --summary` (final closure gate — per CLAUDE.md §"Test
  Discipline" workspace-test exception)

## Authoritative Docs

- `docs/02_ir_schemas.md` — no schema bump needed; topology changes are internal to
  `skeletal_trapezoidation`/`arachne`, not `slicer-ir`
- `docs/08_coordinate_system.md` — range-read §"Constant Conversion Table" only (unit
  conversion, same hazard packet 113b flagged)
- `docs/adr/0023-arachne-port-strategy.md` — read full (degeneracy-handling contract this
  packet's Step 3 must keep honoring: T-junctions, duplicate vertices, near-collinear segments)
- `docs/adr/0034-arachne-faithful-graph-construction.md` (NEW, authored alongside this packet)
  — read full; the architectural decision this packet implements
- `docs/DEVIATION_LOG.md` — read `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS` entries in
  full (both currently show `Closed`; this packet corrects that record — see Doc Impact
  Statement)

For each doc, the implementer should range-read or delegate. Do not load any doc > 300 lines in
full except `docs/adr/0023-arachne-port-strategy.md` and `docs/adr/0034-...md` (both short).

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` — register new `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` entry recording
  the real root cause and fix; explicitly state it supersedes and closes-for-real both
  `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS`. Append a one-line addendum to each of
  those two existing entries ("Superseded — see `D-113C-FAITHFUL-GRAPH-CONSTRUCTION`; this
  entry's prior 'Closed' status was based on an insufficient interim fix") — per grilling
  decision, this preserves their historical narrative rather than editing it in place. |
  `rg -q 'D-113C-FAITHFUL-GRAPH-CONSTRUCTION' docs/DEVIATION_LOG.md && rg -q 'Superseded.*D-113C-FAITHFUL-GRAPH-CONSTRUCTION' docs/DEVIATION_LOG.md`
- `CONTEXT.md` — add glossary entries (once Step 3/4's Rust shapes are settled, per grilling
  decision — not upfront) for: central/spine edge, rib edge, quad (Arachne), junction fan,
  domain-start, `getNextUnconnected`. Format: `### Term` + short prose, matching the file's
  existing "definitions only" convention. |
  `rg -q '### Rib edge' CONTEXT.md`
- `docs/01_system_architecture.md` — update §"Perimeter Modules — OrcaSlicer Parity Roadmap" to
  record the faithful graph-construction closure, superseding the prior M2-faithful marker's
  incomplete claim. | `rg -q '113c' docs/01_system_architecture.md`
- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — add a section noting 113c as the
  correction to 113b's incomplete quad/rib topology pass. | `rg -q '113c' docs/specs/perimeter-modules-orca-parity-roadmap.md`
- `docs/adr/0034-arachne-faithful-graph-construction.md` — NEW file, authored alongside this
  packet (see Deliverable note in this packet's own provenance); not deferred to an
  implementation step.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into
the implementer's own context. Default dispatch contract: return `LOCATIONS` (file:line + 1-line
context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns
are capped at 30 lines.

**Scoped exception for Steps 2 and 3 only** (per this packet's grilling session — packet 113b's
own `makeRib` dispatch asked for a SUMMARY of the callee body in isolation and never surfaced
`constructFromPolygons`'s caller loop, the single fact — called after every edge, not just at
corners — that would have caught the reflex-corner-only mistake): for Steps 2 and 3, sub-agent
dispatches MAY return up to 30-line code excerpts of caller-side loop structure (
`constructFromPolygons`'s per-cell iteration, `transferEdge`/`makeRib` call sites and arguments,
`getNextUnconnected`'s body) in addition to prose, and the dispatch prompt MUST explicitly ask
"what does the calling loop look like and how often is this invoked per polygon," not only
"summarize this function." Steps 4 and 8 keep the default SUMMARY-only contract, since Step 3
will have already surfaced the caller structure by then.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:431-560` —
  `constructFromPolygons()`: the per-cell iteration loop and its `makeRib` call sites. Step 2/3
  dispatch — request the caller loop structure, not just callee summaries.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:157-257` —
  `transferEdge()`: per-Voronoi-edge chain construction, including the "twin already exists"
  mirrored-construction branch.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:452-482` —
  `makeRib()`: rib-pair insertion and cursor reassignment.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:183-193` —
  `getNextUnconnected()`: dead-end-then-twin continuation.
- `OrcaSlicerDocumented/src/libslic3r/Geometry/VoronoiUtils.cpp` (`compute_segment_cell_range`,
  `compute_point_cell_range`) — cell-range determination, needed for Step 2's spike and Step 3's
  design.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidation.cpp:2260-2368` —
  `connectJunctions()`: the quad-by-quad stitch, `unprocessed_quad_starts`, `getQuadMaxRedgeTo`.
- `OrcaSlicerDocumented/src/libslic3r/Arachne/SkeletalTrapezoidationGraph.cpp:310-431` —
  `insertRib()`/`insertNode()`: needed for Step 6's `insert_node` re-audit.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by
`spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or
reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. Step 3 is
the one documented L-step exception (see §Prerequisites and Blockers); every other step must
stay S/M or be split.
