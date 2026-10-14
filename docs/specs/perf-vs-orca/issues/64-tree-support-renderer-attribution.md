# Tree-support renderer self-fuel and scan-fill attribution

Type: task
Label: wayfinder:task
Status: open
Assignee: unassigned
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 61

## Question

What owns `com.core.tree-support`'s largely unmarked rendering work, what reaches
the slice's wall-critical path, and is there an exact reusable-input reduction
worth a separately qualified experiment?

The [Benchy current bottleneck refresh](61-benchy-current-bottleneck-refresh.md)
measures 43.432/43.431 billion renderer instructions on Classic/Arachne
supports-on, 96.85% module self. The renderer is 20.42% of Arachne supports-on
guest fuel; supports-off is the 0.109/0.107 billion negative control. Its
7.389/6.402 s module elapsed sums are **accumulated parallel worker time**, not
wall or CPU. Wall importance is not yet established.

This is the `Layer::Support` renderer, **not**
`com.core.tree-support-planner`; the existing planner carve, union and rejected
projection-cache investigations do not answer this question. Renderer fuel
must not be ranked against the planner's fuel as a proxy for total cost, because
the serial planner also reaches native services.

## Scope and decision evidence

1. After fresh output/freshness qualification, split `TreeSupport::run_support`
   (`modules/core-modules/tree-support/src/lib.rs`) into plan filtering,
   interface regularization, role union, paint/skeleton queries, host wall/fill
   offsets, scan filling and output construction. Non-overlapping guest fuel
   scopes plus export/host brackets must reconcile the residual.
2. `scan_fill_region` in the same file scans every contour/hole edge at each row
   and repeatedly converts coordinates. Measure row/edge/vertex counts and
   fuel before blaming this loop or proposing an active-edge/preconverted-input
   shape. Include high-fuel middle layers and body/interface roles.
3. Qualify actual real-input full-role replay in both modes against an
   independent baseline. Preserve holes, row positions and crossing ordering,
   integer/f32 rounding, wall/fill separation, widths/flows, paint vetoes,
   off-grid anchors, roles and output order. Do not skip regularization or
   simplify planned geometry merely to reduce work.
4. Establish wall relevance using same-run scheduling/brackets, with supports-off
   and both generators as controls. Return a ranked reduction recommendation
   or a documented no-useful-reduction result; no fuel-to-wall assumption.

## Acceptance and boundaries

Attribution only; no candidate, planner rewrite, WIT change or timing campaign
authorized by ticket creation. A surviving output-identical reduction graduates
to a separate experiment and the standing paired ordinary + accelerated
wall/CPU gate, with KEEP/DROP returned to the human. Any discovered geometry
parity defect follows the repo's separate correctness program, not a silent
performance baseline change.

Evidence: [refresh findings](../evidence/perf-refresh-2026-10-13/FINDINGS.md),
its full aggregate profiles, per-layer fuel counts and checkout-only verifier.
