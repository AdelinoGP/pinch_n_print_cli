# ADR-0036 — LimitedBeadingStrategy Must Be a Faithful, Algorithm-Level Port of OrcaSlicer's Reference

<!-- filename: 0036-limited-beading-strategy-faithful-port-audit -->

## Status

Accepted (2026-07-13). Authored as the closing record of the D-105 beading
faithful-port audit, extending ADR-0034's graph-construction faithfulness bar to
the beading-strategy layer.

## Context

The D-105 closure session (packet 150, the original 2026-07-10 closure) wired
`line_width_to_spacing` into bead placement. A subsequent session (2026-07-13)
attempted to close the G4 test
(`arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width`) by introducing
an architectural fix in
`crates/slicer-core/src/beading/limited.rs::LimitedBeadingStrategy::compute`
(recompute parent at `optimal_thickness = max_bead_count * optimal_width`, carry
surplus to `left_over`, symmetry mirror) and `::optimal_bead_count` (cap at
`max_bead_count + 1`). A faithful-port audit against the canonical
`F:\slicerProject\OrcaSlicerDocumented\src\libslic3r\Arachne\BeadingStrategy\LimitedBeadingStrategy.cpp`
verified the architectural fix as faithful to `LimitedBeadingStrategy.cpp:64-127`,
but surfaced:

- One introduced bug (the `0.01` scale error at `limited.rs:185`, now corrected
  to `0.01 * UNITS_PER_MM` per the convention in `beading/distributed.rs:199` and
  `arachne/generate_toolpaths.rs:230`).
- Two pre-existing divergences the session did NOT touch: the missing under-cap
  center sentinel (`LimitedBeadingStrategy.cpp:73-82`, now ported) and the wrong
  over-cap sentinel position using centerline instead of inner edge
  (`LimitedBeadingStrategy.cpp:118-131`, now corrected).
- A separate fabricated spine subdivision in
  `crates/slicer-core/src/skeletal_trapezoidation/graph.rs::from_polygons_with_beading`
  that was REVERTED per ADR-0034 (graph-construction faithfulness).

ADR-0034 already governs graph-construction faithfulness
(`SkeletalTrapezoidationGraph::from_polygons`, `makeRib`, `getNextUnconnected`).
This ADR extends the same faithfulness standard to the beading-strategy layer.

## Decision

`LimitedBeadingStrategy` must be a faithful, algorithm-level port of OrcaSlicer's
`LimitedBeadingStrategy.cpp` in ALL branches: the under-cap center sentinel
(lines 69-84), the over-cap recompute-at-optimal-thickness + `left_over` +
symmetry mirror (lines 95-110), the over-cap sentinel position (inner edge, not
centerline, lines 118-131), and `optimal_bead_count` three-branch cap at
`max_bead_count + 1` with the `0.01 * UNITS_PER_MM` transition-zone threshold
(lines 162-179). The PnP `f64` type is exact for the half-bead-width
sentinel-position formula; the C++ `coord_t` integer-division ±1nm hazard at
lines 116-117 does not apply.

## Rejected alternatives

- **Keep the pre-existing wrong sentinel positions and document as deviations.**
  Rejected: the file was open, the fixes are cheap, and the sentinel's whole
  purpose is to mark where infill aligns; getting it wrong by half a bead width
  defeats the purpose.
- **Defer the sentinel fixes to a follow-up packet.** Rejected: same-class
  corrections opened by the session, closing them in the same change avoids a
  stale intermediate state.
- **Keep the `0.01` scale error as "functionally near-equivalent for mm-scale
  tests".** Rejected: scale errors compound on fine geometry, and the sibling
  files already establish the `0.01 * UNITS_PER_MM` convention.

## Consequences

- Any future packet touching `crates/slicer-core/src/beading/limited.rs` must
  treat "faithful port of all `LimitedBeadingStrategy.cpp` branches" as the bar
  — not "passes the currently-tested fixtures." A partial port that only handles
  the over-cap branch (as the pre-existing port did) is a defect, not a
  documented scope limitation.
- The thin-strip medial-axis collapse root cause (D-105D, spec packet 154) is
  NOT closed by this ADR — it's a graph-construction/topology issue governed by
  ADR-0034. The beading-strategy layer is faithful; the topology layer is not.
- OrcaSlicer-read dispatches for the beading-strategy layer must follow the same
  delegation-protocol corollary as ADR-0034: request the caller's loop context
  if the callee's body alone could lose structural information.

## Future reviewers

- If the PnP `f64` type is ever changed to an integer `coord_t`-equivalent type,
  the half-bead-width sentinel-position formula must replicate the C++
  `+ width/2` integer-division hazard explicitly. The C++ comment at
  `LimitedBeadingStrategy.cpp:116-117` documents this.
- If a future performance concern makes the symmetric-mirror loop
  (`LimitedBeadingStrategy.cpp:109-110`) redundant for some parent strategy,
  that tradeoff must be recorded as a new ADR revising this one — not silently
  reintroduced as an unreviewed simplification.
