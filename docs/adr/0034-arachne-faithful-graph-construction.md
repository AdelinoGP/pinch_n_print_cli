# ADR-0034 — Arachne Graph Construction Must Be a Faithful Per-Cell + Interleaved-Rib Port

## Status

Accepted (2026-07-05). Authored alongside packet `113c-arachne-faithful-graph-construction`,
before its implementation begins — this decision is meant to exist as a guardrail from day one,
not be recorded retroactively once the packet closes.

## Context

Packet 113b (`.ralph/specs/113b-arachne-topology-faithfulness`) set out to build a "synthetic
quad/rib topology pass" faithful to OrcaSlicer's `makeRib`, gating four dependent Arachne passes
(centrality, bead count, transitions, `connectJunctions`). What actually shipped
(`crates/slicer-core/src/skeletal_trapezoidation/rib.rs::build_quad_rib_topology`) only inserts
rib edges at reflex/sharp polygon corners — an admitted "Step 1 minimal" implementation, per its
own doc comment, that was never upgraded. `crates/slicer-core/src/skeletal_trapezoidation/
graph.rs::SkeletalTrapezoidationGraph::from_polygons` copies `next`/`prev`/`twin` verbatim from
the raw `boostvoronoi` per-Voronoi-cell DCEL — which encodes "walk around one Voronoi cell's own
boundary," not "continue along the medial-axis spine." This is topologically wrong at every
junction/branch vertex, which exists in every non-trivial polygon (even a plain square's single
central X-junction).

A `/diagnose` session (2026-07-05), triggered by a user-reported visual bug in
`resources/cube_4color.3mf` sliced with `wall_generator=arachne` (holes, gaps, a stray floating
bar), traced this to a systemic, confirmed defect: 100% of outer-wall gcode segments fail to
close into loops (283/283 non-closed headers, mean gap 18.7mm), reproduced in isolation via
`run_arachne_pipeline` on a single simple polygon, and reproduced even for a bare 10mm square
(the existing `arachne_perimeters_simple_square` test's own doc comment already admitted 26
fragmented lines for one square, without anyone treating that as a red flag). `docs/
DEVIATION_LOG.md`'s `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS` entries both show
`Closed`, but neither closure touched graph construction: `D-112-MMU-TOPOLOGY`'s 11th-pass
"closure" was a test-harness realignment (gcode header re-pairing to eliminate a
sampling-aliasing artifact); `D-113B-CONNECTJUNCTIONS`'s closure was a central-only domain-walk
generalization now proven insufficient (it breaks at every rib once ribs are correctly
interleaved).

Root-causing this required reading OrcaSlicer's real C++ source directly (available locally at
`F:\slicerProject\OrcaSlicerDocumented\src\libslic3r\Arachne\`, the project's standard
attribution-header reference). That source revealed real OrcaSlicer's `constructFromPolygons`
inserts a rib edge pair after **every single transferred edge** (not just at reflex corners),
interleaved directly into the `next`/`prev` chain — this interleaving is what makes
`getNextUnconnected()` correctly traverse junctions of any degree. Packet 113b's own `makeRib`
dispatch, under this project's standard OrcaSlicer-read-delegation protocol (SUMMARY only,
≤ 200 words, no code, callee-body only), never surfaced this: a summary of `makeRib`'s ~30-line
body in isolation cannot convey that its caller (`constructFromPolygons`) invokes it after every
edge unconditionally, since that fact lives in the caller's loop, not the callee. Nothing short
of an explicit architectural decision — this ADR — stops a future packet from repeating the same
simplification under time pressure, since the delegation protocol's default framing (summarize
one function in isolation) will keep losing this exact class of fact unless a packet
specifically asks for caller-loop context.

## Decision

**Arachne's graph construction (`SkeletalTrapezoidationGraph::from_polygons`) and junction
connection (`generate_toolpaths.rs`'s `connectJunctions` equivalent) must be faithful,
algorithm-level ports of OrcaSlicer's real construction — a per-cell chain built via
`transferEdge`, with a rib edge pair inserted after every transferred edge via `makeRib`
(interleaved into the `next`/`prev` chain, not a post-hoc classification pass over an
already-built graph), and junction stitching via the real `getNextUnconnected`/
`getQuadMaxRedgeTo` quad-by-quad mechanism — not a simplified approximation, and not a pass that
only handles a subset of cases (e.g. "just reflex corners") with the intent to generalize later.**

This is a correctness requirement, not a style preference: the current codebase already
demonstrates that a "close enough for the tested fixtures" approximation at this specific layer
produces wall geometry that never closes into loops for any non-trivial input, including the
simplest possible one (a plain square). There is no partial version of this construction that
produces correct topology — spine-chain construction and rib interleaving are mutually
dependent, which is also why packet `113c-arachne-faithful-graph-construction`'s own
implementing step for this work is an explicitly-documented L-effort exception with no natural
split point (see that packet's `packet.spec.md` §Prerequisites and Blockers).

**Process corollary:** when delegating an OrcaSlicer-read dispatch for this specific algorithm
family (graph construction / DCEL topology), the dispatch prompt MUST ask about the **calling
loop's** structure and invocation frequency, not only summarize the target function's own body
in isolation. Packet `113c-arachne-faithful-graph-construction`'s `packet.spec.md`/`requirements.md`
encode a scoped exception to the standard SUMMARY-only, no-code delegation contract for exactly
this reason (Steps 2-3 there may request up to 30-line code excerpts of caller-side loop
structure).

## Rejected alternatives

- **Keep the reflex-corner-only rib pass and patch around its symptoms downstream** (e.g. a more
  aggressive `stitch_extrusions` proximity-bridge, or accepting "open junction graph, not closed
  rings" as documented, by-design behavior — which is literally what `D-112-MMU-TOPOLOGY`'s
  prior status claimed). Rejected: this is exactly the shape of fix `CLAUDE.md`'s "never game
  verification by weakening assertions... or skipping checks" rule forbids, and it does not
  produce OrcaSlicer parity — the actual stated goal of this branch's work. It also does not fix
  the user-visible symptom (open walls render as holes/gaps in a 3D preview) at its source.
- **Treat "self-captured fixtures, no OrcaSlicer oracle" as license to approximate the
  algorithm loosely.** Rejected: the self-captured-fixture precedent (`D-112-SELFCAPTURED-
  BASELINES`, `D-109-SELF-CAPTURED-FIXTURES`) is about the absence of a literal OrcaSlicer
  *binary* to diff numeric output against — it was never meant to license approximating the
  *algorithm* itself when the real source is directly readable, as it is here.
  `docs/adr/0023-arachne-port-strategy.md` already establishes the "faithful port, verified by
  code reference and review" standard for this exact subsystem; this ADR reaffirms and sharpens
  it specifically for graph construction after this packet demonstrated a concrete instance of
  it being under-applied.
- **Build the real OrcaSlicer C++ checkout to generate golden-oracle fixtures for this specific
  layer**, closing the "no oracle" gap entirely. Considered during packet
  `113c-arachne-faithful-graph-construction`'s grilling session and explicitly declined: a
  multi-hour CMake+vcpkg+MSVC infrastructure lift with no precedent in this project's prior
  arachne packets, disproportionate to what invariant-based testing (closed rings, quad-chain
  length, junction-count-delta bounds — properties that hold regardless of specific geometry)
  already achieves.

## Consequences

- Any future packet touching `crates/slicer-core/src/skeletal_trapezoidation/graph.rs`,
  `rib.rs`, or `crates/slicer-core/src/arachne/generate_toolpaths.rs` must treat "faithful,
  algorithm-level port" as the bar, not "passes the currently-tested fixtures." A partial
  implementation that only handles a subset of topological cases (e.g. "corners only", "simple
  convex shapes only") is a defect, not a documented scope limitation, unless a future ADR
  explicitly revises this one.
- OrcaSlicer-read dispatches for this algorithm family must request caller-loop context, not
  just callee-body summaries — see this ADR's Process corollary and packet
  `113c-arachne-faithful-graph-construction`'s scoped delegation-protocol exception.
- `docs/DEVIATION_LOG.md`'s `D-112-MMU-TOPOLOGY` and `D-113B-CONNECTJUNCTIONS` entries are
  superseded by `D-113C-FAITHFUL-GRAPH-CONSTRUCTION` (registered by packet
  `113c-arachne-faithful-graph-construction`) rather than edited in place — their own narrative
  remains a record of what was actually tried and why it fell short, useful for any future
  agent tempted to repeat a similar shortcut.
- `docs/adr/0023-arachne-port-strategy.md` remains the governing document for Voronoi-level
  degeneracy handling (collinear input, T-junctions, duplicate vertices, near-collinear
  segments); this ADR does not relax or restate that contract, only the topology layer built on
  top of it.

## Future reviewers

- If a future performance or complexity concern makes a full faithful port genuinely
  infeasible for some specific case, that tradeoff must be recorded as a **new, explicit ADR**
  revising this one — not silently reintroduced as an unreviewed simplification the way packet
  113b's reflex-corner-only pass was.
- If `test_voronoi.cpp` or any other upstream OrcaSlicer test file gains coverage for
  `SkeletalTrapezoidation`/`WallToolPaths`/`connectJunctions` in a future vendored update,
  re-evaluate whether literal test porting becomes possible — as of this ADR, zero such tests
  exist upstream (confirmed by direct search of `OrcaSlicerDocumented/tests/` during packet
  `113c-arachne-faithful-graph-construction`'s planning).
