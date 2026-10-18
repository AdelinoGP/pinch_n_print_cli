# ShellClassification phase-B serial commit reduction

Type: task
Label: wayfinder:task
Status: open
Assignee: unclaimed — graduated by the 2026-10-18 base refresh; ticket creation
is not execution approval
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 73

## Question

Can `PrePass::ShellClassification`'s serial phase-B commit be reduced without
changing the classification output?

The fresh base captures measure the stage at **26.0–31.4 s** in all four base
cells ([findings](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md)).
Ticket 27's 2026-09-27 probe localized
`gate_phase_b_commit_wall` at **30.29 s (base-off) / 25.07 s (base-on)** — a
serial commit loop around the per-surface `expand(closing(spacing))` /
`intersection` chain plus `internal_bridge_angles`
(`crates/slicer-core/src/algos/bridge_over_infill.rs`) over
`deep_infill_clip_area`, `internal_unsupported_area` and `expansion_area`.
The fresh stage walls are the same order, so this remains the largest
unaddressed serial block outside `PrePass::Slice`.

Deliverables:

1. Re-derive the phase-B split at HEAD (the t27 probe predates many keeps) and
   confirm the serial commit's current share of the stage.
2. Test the t27-named cheapest exact shape: pre-reduce the
   `internal_bridge_angles` input (one shared pre-intersection per qualified
   polygon rather than three full-area intersections), and — only if the
   per-surface operations are independent — a parallel commit. Record why any
   refuted shape is unsafe.
3. Qualify output byte-identity on the 16-cell matrix against the preserved
   references and run the standing paired ordinary + accelerated A/B on the
   four pre-declared supports-on cells. Keep/drop is the human's.

Constraints: attribution first; no production edit before a measured owner;
output byte-identity is the hard gate; no default switch or auto-commit.
Profile/worker elapsed is not process CPU or uninstrumented wall.

## Acceptance

- A current-head phase-B split with the serial share named.
- Either a candidate passing the standing gate, or a measured no-safe-reduction
  verdict with the refuted shapes recorded.
- Durable reduced evidence and a checkout-only verifier, per the map's
  evidence policy.
