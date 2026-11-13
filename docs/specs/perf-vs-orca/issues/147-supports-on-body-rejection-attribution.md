# Supports-on output is support-incomplete: what do the `exact-Z occupancy` body rejections drop?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Benchy fixture replacement](145-benchy-fixture-rebaseline.md)'s
  2026-11-13 take. Every PnP supports-on run in the matched job reports
  `degraded: true` with code-1200 non-fatal diagnostics from
  `PrePass::SupportGeometry` / `com.core.tree-support-planner`, all of the form
  `support demand '<id>' unmet for body '<tree-body-…>': body rejected: exact-Z
  occupancy`. The counts are stable and fixture-independent: **101** per benchy
  supports-on cell and **44,474** per base supports-on cell, identical on the
  lost benchy export (t135, t142) and the replacement (t143, t144, t145), and
  zero on the four supports-off cells. The map's supports-on ratios therefore
  compare a PnP job that prints **less support than its own planner demanded**;
  the standing policy is to close asymmetries before a ratio of record, and the
  four supports-on cells carry four of the eight destination ratios.

## Question

Does the planner's exact-Z occupancy test reject support bodies it should keep,
or is the rejection correct and the unmet demand expected? Classify each family
of rejections and say whether the supports-on PnP output is support-incomplete
in a way that invalidates the wall ratio (less support printed = less work paid
than the matched job demands), and what would close it.

Decide and record, in order:

1. **Census the rejections.** On one benchy supports-on cell and one base
   supports-on cell, count rejected bodies per layer, their layer span, the
   demand ids they would have served, and the `exact-Z occupancy` payload that
   caused the rejection (the colliding exact-Z query key and the body whose
   occupancy it hit). State whether the failure is a contiguous band (DEV-174's
   signature) or scattered.
2. **Check the emitted geometry against the demand set.** For a sample of
   rejected demands, locate the overhang region they were meant to support in the
   PnP G-code: is there support there from another body, or is the region
   unsupported? Do the same on the Orca arm's output for the same layer. This is
   the check that decides "incomplete" vs "the demand was redundant".
3. **Classify.** (a) correct rejection — the body genuinely collides with the
   model at that exact Z; (b) a defect — the test, the body geometry, or the
   occupancy record is wrong; (c) a modelling choice — bodies are built
   optimistically and demands are expected to go unmet. Cite the code path
   (`plan_for_object`, the exact-Z occupancy query, and the body-rejection site
   in `modules/core-modules/tree-support-planner/src/lib.rs`).
4. **Price the fairness effect.** If support is missing, the supports-on wall is
   understated on the PnP side. Bound the missing support (path mm, coverage mm²)
   and say whether the four supports-on ratios of record remain quotable with a
   disclosure, or need an exception/ticket.

## Pre-declared gate

This is an attribution, not a candidate: no performance gate and no production
edit. The correctness bar is that the classification is falsifiable from the
evidence:

- the rejection census and the emitted-support check are re-derivable from an
  in-repo reduced set, with raw captures cited only as the fuller source;
- the sample of demands examined is named (ids, layers) so the "incomplete" or
  "redundant" verdict cannot be a single-example anecdote;
- every supports-on ratio quoted in the answer carries the degradation count and
  the verdict beside it.

If the answer is (b) and a fix is exact by construction, graduate a candidate
ticket; do not land a fix in this take.

## Acceptance

- A rejection census (per cell, per layer span, per demand family) and the
  emitted-support check, as an in-repo evidence directory with a `verify.py`.
- A written classification with the code paths named by symbol.
- A ruling on the four supports-on ratios of record: quotable with disclosure,
  or a new fairness asymmetry ticket.
- The map's Next-status and the supports-on disclosure updated.
