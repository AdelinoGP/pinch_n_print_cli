# Seam candidate generation on the printed loops: authorize the per-layer relocation?

Type: grilling
Label: wayfinder:grilling
Status: open (unclaimed — human ruling)
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none ([152](152-seam-inset-offset-redundancy.md) resolved 2026-11-14)
Graduated from: [Seam inset offset: is the prepass offset redundant with the per-layer loop projection?](152-seam-inset-offset-redundancy.md)
  — the clean deletion is falsified: dropping the prepass inset offset and
  emitting the raw chosen point moves **456 of 496** base-classic-off layers
  (+1,177 / −1,149 points, 142 layers with a changed total `E`), and the two
  unit tests that pin the inset contract fail. The offset is load-bearing, and
  its measured term is **1,248 ms** (`PrePass::SeamPlanning` 1,779 → 531 ms,
  70.15 % of the stage). See
  [`evidence/t152-seam-inset-offset/FINDINGS.md`](../evidence/t152-seam-inset-offset/FINDINGS.md)
  (`verify.py` PASS, 4/4 mutations rejected).

## Question

Should the map authorize a **per-layer relocation** of seam candidate
generation to the printed loops, under a scoped t81-style exception?

The canonical replacement for t129's S3 is not a prepass deletion. Canonical
`SeamPlacer::collect_points` builds candidates from the **printed
external-perimeter loop points**, and `place_seam` projects onto that loop.
Those loops do not exist in PnP's prepass; they are produced by the per-layer
perimeter modules. PnP's prepass generates candidates on the **region
boundary**, and the per-layer `seam-placer` only projects the chosen point.
Matching canonical therefore means moving candidate generation and scoring into
`Layer::PerimetersPostProcess` (or a neighbouring per-layer stage), where the
loops exist.

What that would buy and cost, as measured today:

- The prepass offset term is **1,248 ms** on base classic-off (70.15 % of
  `PrePass::SeamPlanning`), **5.9x** the 0.80 % floor on the 26.254 s single-run
  cell and **~10.5 %** of the cell's current required cut. On base classic-on
  the t129 share was 1.6 % of the cut; the benchy shares are smaller.
- The relocation's net is **not** that number. It is the offset term minus the
  moved candidate-generation/scoring work — the current prepass residual is
  **531 ms** — minus any per-layer loop-collection cost. The per-layer phase
  already owns the projection, so the projection itself is not new work, but
  the scoring and candidate construction would be.
- The relocation **changes computed geometry in the per-layer phase** (seam
  positions), which the map's out-of-scope section says needs its own t81-style
  exception. The t111/t128 prepass exception does not reach it.
- It is a parity change, not just a speed shape: PnP would generate candidates
  on the actual loops, which is closer to canonical, but the scoring model
  (overhang / visibility / alignment) is not a full canonical port, so no
  parity claim is made without a separate audit.

The ruling requested: **authorize, decline, or re-scope.** If authorized, the
next ticket is a Phase-1 relocation scope (AFK) that prototypes candidate
generation on the per-layer loops and measures the net wall against the paired
gate, with no production landing before the fresh 16-cell reference. If
declined, S3 closes as "not a clean deletion" and the 1,248 ms stays in the
fog with its price recorded.

## Why this is HITL

The map's standing rule is that a change altering computed geometry in the
per-layer phase needs its own t81-style exception from the human. The screen has
already answered the technical question (the offset is load-bearing); what
remains is a scope decision, not another measurement.
