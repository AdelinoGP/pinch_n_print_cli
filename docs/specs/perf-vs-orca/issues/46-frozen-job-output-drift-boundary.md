# Frozen-job output drift boundary

Type: task
Status: open
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

Why does the current host+guest tree change the frozen supports-off Benchy
Arachne job's actual G-code moves and TYPE counts under identical model/config
and external-only flags: intended config/scheduler behavior, a correctness
defect, or a provenance problem? Which output/fairness boundary must future
optimization and adoption takes use?

Surfaced by [Clip-universe preparation hoist](44-clip-universe-hoist.md).
Its paired candidate is output-identical to the fresh baseline, but both differ
from the old reference. Exact-flag frozen control still reproduces that
reference. See [hoist findings](../evidence/t44-clip-universe-hoist/FINDINGS.md)
and the two reduced controls in `evidence/t44-clip-universe-hoist/evidence.json`.
The hoist experiment did not attribute this drift to any merge or fix it.

## Work — separate human take required

- Verify current inputs, ordinary guest freshness, and exact module isolation
  before diagnosing. Do not mix host/guest schema eras or assume a hash change
  means a geometry defect.
- Localize the first changed resolved config/IR/output boundary against the
  reproducible frozen control. Distinguish intended current-job behavior from
  lost work. The existing emitter's unclosed-loop/missing-edge warnings must
  remain visible; clean completion counters do not prove their absence.
- Return the intended-vs-defect decision and next scope to the human. A repair,
  reference replacement, acceptance retry or timing campaign needs its own
  authorization; do not alter either preserved reference to make a gate pass.

This question is about output/job provenance and fairness, not a new Orca
geometry-parity program. No investigation, fix, measurement or commit is
authorized merely by creating this ticket.
