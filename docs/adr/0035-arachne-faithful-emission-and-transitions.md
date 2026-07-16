# ADR-0035 — Arachne Emission, Transitions, and Post-Process Must Be Faithful Algorithm-Level Ports of OrcaSlicer's C++ Reference

<!-- filename: 0035-arachne-faithful-emission-and-transitions -->

## Status

Accepted (2026-07-08). Authored alongside packet `147-arachne-cross-cutting-closure`,
the closure packet for the Arachne parity N1-N13 chain (packets 141-147).

## Context

Packets 141-146 collectively deliver canonical OrcaSlicer parity for the Arachne
pipeline's full emission + transitions + post-process surface. The chain addresses
N1-N13 — the 13 numbered findings from the canonical parity audit at
`target/arachne_parity_audit_20260706_020657.md`, committed as red tests at
`b2ea52b7`. Each packet fixed its slice:

> **Correction (2026-07-15, audit reconciliation):** the per-packet file paths
> below were rewritten to the *actual* code loci. The original text named seven
> files that never existed (`beading_propagation.rs`, `generate_junctions.rs`,
> `connect_junctions.rs`, `transition_ends.rs`, `filter_noncentral_regions.rs`,
> `local_maxima.rs`, `post_process.rs`); the work was consolidated into
> `arachne/generate_toolpaths.rs`, `arachne/pipeline.rs`, and the
> `skeletal_trapezoidation/*.rs` modules instead. The Decision below is unchanged.

- **P141 (A1)** — `BeadingPropagation` + canonical `generateJunctions` (N1+N7).
  `populate_beading_propagation` in
  `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`;
  `generate_junctions` in `crates/slicer-core/src/arachne/generate_toolpaths.rs`.
- **P142 (A2)** — `perimeter_index` + canonical `connectJunctions` emission +
  canonical `is_odd` (N2+N4). Junction emission / domain-chain walk in
  `crates/slicer-core/src/arachne/generate_toolpaths.rs` (there is no standalone
  `connect_junctions` file or function — the walk is inline in that module).
- **P143 (B)** — canonical transition ends + `BeadingStrategy` trait extension
  (N3+N8). `generate_all_transition_ends` / `apply_transitions` in
  `crates/slicer-core/src/skeletal_trapezoidation/propagation.rs`.
- **P144 (C)** — `filterNoncentralRegions` + centrality coupling resolution
  (N5+N6). `filter_noncentral_regions` / `dissolve_noncentral_gap` in
  `crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`.
- **P145 (D)** — local maxima micro-loops + construction epilogue (N9+N10).
  `generate_local_maxima_single_beads` in
  `crates/slicer-core/src/arachne/generate_toolpaths.rs`.
- **P146 (E)** — canonical post-process order + per-line `min_width` +
  distance-gated `simplify` (N11+N12+N13). `remove_small_lines` in
  `crates/slicer-core/src/arachne/remove_small.rs`, `simplify` in
  `crates/slicer-core/src/arachne/simplify.rs`; post-process order in
  `crates/slicer-core/src/arachne/pipeline.rs`.

Packet 147 (F) closed the 7 deferred parity-audit findings (the cross-cutting
closure: `is_closed` pre-stitch, `has_bead` sub-run split,
`filter_noncentral_regions` 4 deviations, `connectJunctions` merge, `is_odd`
predicate, transition interpolation, `collapseSmallEdges` Pattern B), improved
the `cube_4color` e2e closure rate from 0% to 49.33% (455/898 outer-wall
closures still fail, mean gap 54.7 mm) — which does not meet AC-1's 0-failure
bar, so the gate `cube_4color_arachne_outer_walls_close_end_to_end` remains
`#[ignore]`d (there is no `MAX_FAILURES` numeric regression guard; the sole
mechanism is the `#[ignore]` attribute), and re-baselined the cross-crate
`perimeter_parity` fixtures.

The chain supersedes the PNP "ADAPTATION" divergence for the Arachne surface —
the project no longer maintains a separate, simplified algorithm and instead
ports OrcaSlicer's reference algorithms directly. This is consistent with
ADR 0023 (arachne-port-strategy) and ADR 0034 (graph construction must be a
faithful per-cell port).

## Decision

**`generateJunctions`, `connectJunctions`, `generateAllTransitionEnds`,
`applyTransitions`, `generateExtraRibs`, `filterNoncentralRegions`,
`collapseSmallEdges`, `dissolveNoncentralGap`, `removeSmallLines`, and
`simplifyToolpaths` must all be faithful, algorithm-level ports of OrcaSlicer's
real C++ implementations — a partial implementation that only handles a subset
of cases (e.g. "corners only", "just the cases we've tested") is a defect, not
a documented scope limitation, unless a future ADR explicitly revises this one.**

This extends ADR 0034's bar for graph construction to the post-graph-construction
emission/transitions/post-process surface. The same reasoning applies: the N1-N13
audit demonstrated that PNP's prior "ADAPTATION" approximations produced
systematically wrong output (open walls, missing transitions, incorrect bead
counts) that could not be patched downstream. A partial port that handles only
the geometry seen in the current fixture set is indistinguishable from the old
approximation — it will break on the next non-trivial input, just as the
reflex-corner-only rib pass did in ADR 0034's narrative.

**Process corollary:** when delegating an OrcaSlicer-read dispatch for any of
these functions, the dispatch prompt MUST ask about the **calling loop's**
structure and invocation frequency, not only summarize the target function's
own body in isolation. This mirrors ADR 0034's Process corollary and applies
to the same underlying failure mode: a callee-body-only summary of e.g.
`generateJunctions` cannot surface that its caller invokes it once per
`ExtrusionJunction` in a specific traversal order, which is the fact that
determines correctness.

## Rejected alternatives

- **Maintain PNP's simplified "ADAPTATION" approximations and document the
  gaps as known limitations.** Rejected: this is what the project did before
  the N1-N13 audit, and it produced wall geometry that never closes into loops
  for any non-trivial input. Documenting gaps does not fix the user-visible
  symptom (holes, missing walls, incorrect bead counts). The parity audit
  proved that the gap between "close enough for the tested fixtures" and
  "correct" is not a narrow edge case but the entire non-trivial input space.
- **Build the real OrcaSlicer C++ checkout to generate golden-oracle numeric
  fixtures for this surface.** Considered and declined for the same reasons as
  ADR 0034: a multi-hour CMake+vcpkg+MSVC infrastructure lift with no
  precedent in this project's prior arachne packets, disproportionate to what
  invariant-based testing (closed rings, bead-count deltas, transition-length
  bounds — properties that hold regardless of specific geometry) already
  achieves. The N1-N13 red tests in `crates/slicer-core/tests/arachne_parity_red_*`
  serve as the real parity oracles instead.
- **Port each function incrementally, shipping partial implementations with
  "TODO: generalize" markers.** Rejected: this is exactly the pattern that
  produced the reflex-corner-only rib pass in packet 113b (see ADR 0034's
  narrative). The N1-N13 chain proved that these functions are mutually
  dependent — a partial `connectJunctions` breaks when `generateJunctions`
  changes, and vice versa. Incremental partial ports create a permanently
  broken intermediate state that no single packet can fix without touching
  every function at once.

## Consequences

- Any future packet touching `crates/slicer-core/src/arachne/` (any file in
  the emission, transitions, or post-process surface) must treat "faithful,
  algorithm-level port" as the bar, not "passes the currently-tested fixtures."
  A partial implementation that only handles a subset of cases is a defect,
  unless a future ADR explicitly revises this one.
- OrcaSlicer-read dispatches for these functions must request caller-loop
  context, not just callee-body summaries — see this ADR's Process corollary,
  which mirrors ADR 0034's Process corollary for the same underlying failure
  mode.
- Self-captured fixtures guard self-regression, not OrcaSlicer ground truth.
  The N1-N13 red tests in `crates/slicer-core/tests/arachne_parity_red_*` are
  the real parity oracles — they encode specific, audited deviations from
  OrcaSlicer's output that were confirmed by direct C++ reference during the
  chain's implementation. A future packet that changes these tests without
  re-auditing against OrcaSlicer's source is introducing a regression.
- `docs/DEVIATION_LOG.md`'s `D-ARACHNE-ADAPTATION` entry (the PNP "ADAPTATION"
  divergence) is superseded by this ADR and the N1-N13 chain's deviations
  registered during packets 141-147. The old entry remains as a record of what
  was tried and why it fell short, useful for any future agent tempted to
  reintroduce a similar simplification.
- The `connectJunctions` storage layout and `stitch_extrusions` post-process are
  now faithful ports of OrcaSlicer's `LineJunctions` + `PolylineStitcher::stitch`
  (including the `canReverse` parity gate and the `3 * max_stitch_distance`
  tiny-polygon non-closure rule); see packet 153 for the implementation.

## Future reviewers

- If a future performance or complexity concern makes a full faithful port
  genuinely infeasible for some specific function in this surface, that
  tradeoff must be recorded as a **new, explicit ADR** revising this one —
  not silently reintroduced as an unreviewed simplification the way packet
  113b's reflex-corner-only pass was (see ADR 0034's narrative).
- If OrcaSlicer's upstream `WallToolPaths.cpp` or `Line.cpp` gains new
  emission or post-process functions in a future vendored update, re-evaluate
  whether this ADR's scope needs extension. As of this writing, the functions
  listed in the Decision section cover the full post-graph-construction surface
  that OrcaSlicer's Arachne pipeline exposes.
- The `cube_4color` e2e closure gate (`cube_4color_arachne_outer_walls_close_end_to_end`)
  was `#[ignore]`d at 49.33% closure — a closure oracle the pipeline did not pass, not a
  green regression guard (there is no `MAX_FAILURES` threshold mechanism). This clause
  required that a future packet raising the percentage must re-audit against OrcaSlicer's
  C++ source and un-ignore **only at 0 failures** — the percentage alone does not measure
  algorithmic faithfulness.

  **DISCHARGED 2026-07-16** (Arachne Parity Recovery, Track C; `D-147-CHAIN-CLOSURE` closed).
  The gate is at **0/699 (0.00%), mean gap 0.0000mm across all 125 layers** and is
  **un-ignored** — it is now a green regression guard. Both conditions were met, and the
  record is worth keeping because **the second one is what had teeth**:
  - *0 failures:* reached without any production change made for closure's sake — the
    49.33% figure was stale, and D5 (`5d0e1bcf`) + D4 (`1dfac847`) had already dissolved
    the residual upstream in the beading pipeline. Verified non-vacuous: the gate's body is
    byte-identical to the commit that recorded 455/898, it guards its own non-emptiness, and
    the 898→699 sub-loop drop was measured to be topology cleanup, not geometry loss
    (arachne-vs-classic outer-wall length ratio 0.9963, no region dropped).
  - *Re-audit:* the percentage was **already 0 when the audit ran**, so a
    percentage-only reading would have shipped a live defect. The audit found one:
    `D-147-STITCH-TINY-POLY-UNITS` — a spurious `/ UNITS_PER_MM` in
    `stitch.rs::finalize_chain` defeated canonical's `3 * max_stitch_distance` tiny-polygon
    rule in production. Because that defect *inflates* closure, the gate was re-measured
    after the fix (still 0/699, identical — the rule never fires on cube_4color's
    much-longer loops, so the gate never rested on it). `connectJunctions` and
    `pipeline.rs`'s post-process order both re-audited faithful.

  **This clause's reasoning is retained as precedent, not history:** a closure percentage —
  including 100% — is not evidence of faithfulness, and can be *manufactured* by an
  unfaithful rule. Any future gate un-ignored in this surface should pair the metric with a
  source-level re-audit, and should re-measure the metric *after* any faithfulness fix the
  audit produces.
