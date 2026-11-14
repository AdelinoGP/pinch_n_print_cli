# Seam candidate generation on the printed loops: authorize the per-layer relocation?

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-14; human ruling — **DECLINED**. The relocation is
  structurally impossible under ADR-0046's accepted decision: aligned seams are
  cross-layer, a per-layer module cannot see two layers, and canonical
  candidate scoring itself reads the previous layer's contours. Packet 168
  already discovered the constraint during grounding. S3 closes as "not a clean
  deletion and not relocatable"; the 1,248 ms stays in the fog with its
  blocker. See ## Answer.)
Assignee: 2026-11-14 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map"; 153 was the only unblocked, unclaimed frontier ticket
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

## Answer

**Resolved 2026-11-14 — DECLINED: the relocation is structurally impossible
under the current execution model, and this question was already decided as
such.** The human's instruction was to verify the prior attempt before trying
it ("Don't think that is possible, it was attempted before, verify it before
trying it"), and the record is decisive.

**The prior decision.** ADR-0046
([`docs/adr/0046-aligned-seam-in-seam-planning-prepass.md`](../../../adr/0046-aligned-seam-in-seam-planning-prepass.md),
status **accepted**) asks exactly this question — *where does the seam
machinery live?* — and evaluates the same two candidate homes this ticket
proposes. Its "Option 1" is the per-layer `seam-placer` module, and it is
rejected as **structurally impossible**:

> Per-layer modules are **re-instantiated per call and run in parallel across
> layers** (ADR-0045 records that no state survives between calls — the module
> is rebuilt per call, and packet 102 already ruled cross-call caching
> forbidden). A per-layer module can never see two layers, so it can never
> chain anything.

The only sanctioned cross-layer conduit is one-way and prepass-first (ADR-0020):
`SeamPlanIR` is produced by `PrePass::SeamPlanning` and delivered to the
per-layer `seam-placer` as a host-injected `resolved_seam`. There is no
layer → prepass channel.

**The prior attempt.** Packet 168
([`docs/spec_packets/_OLD/168-seam-aligned-modes.md`](../../../spec_packets/_OLD/168-seam-aligned-modes.md),
status implemented) discovered the constraint during grounding and recorded it
as an architecture constraint, not a preference:

> **Per-layer modules cannot chain across layers.** Guest instances are
> re-created per call ... and layers run under `par_iter` with per-layer arenas
> and a read-only blackboard. **All cross-layer machinery MUST live in the
> `PrePass::SeamPlanning` stage**; `seam-placer` only consumes.

**The stronger fact this take adds.** The relocation fails even without the
alignment chaining, because canonical **candidate generation and scoring are
themselves cross-layer**. `build_seam_candidates`
(`modules/core-modules/seam-planner-default/src/visibility.rs`) builds
candidates "for every contour vertex of every layer" and its per-layer terms
read the **previous** layer's contours: `prev_dist` is the signed distance to
the previous layer's contours (driving `overhang` and `unsupported_dist`) while
`curr_dist` is the distance to the current layer's, and `layer_angle` is the
global layer angle used by canonical `curling_influence`. A per-layer module
call sees one layer, so it cannot compute either the previous-layer distance or
the cross-layer angle. The builder is **dormant by design** and exercised only
by tests (module header, `modules/core-modules/seam-planner-default/src/lib.rs`)
because packet 178 moved production candidates onto per-region `SliceIR`
geometry; wiring it back to the printed loops would reintroduce the same
cross-layer requirement in the per-layer phase.

**Why the framing cannot work.** t153 proposes generating candidates on the
printed external-perimeter loops — which exist only after per-layer perimeter
generation — while keeping canonical's alignment. That is circular in this
pipeline: the prepass runs once, before any layer work, and consumes candidates
for all layers; the loops it would need do not exist until after it has run.
Making it work requires one of the three escapes ADR-0046 already rejected:

1. a per-layer cross-call accumulator — forbidden by ADR-0045 / packet 102;
2. a host-builtin native alignment pass — moves slicing policy out of the
   module system, bypassing the manifest/config surface and ADR-0020;
3. a pipeline restructure that materialises every layer's perimeters before
   seam planning — a two-phase layer pipeline, not a t81-style key exception.

**Consequence.** S3 closes as **not a clean deletion and not relocatable**. The
1,248 ms (`PrePass::SeamPlanning` 1,779 → 531 ms) is the price of the
cross-layer aligned seam in this architecture. It stays in the map's fog with
that blocker recorded. No parity exception is spent: the
`nearest` / `rear` / `random` modes could in principle generate candidates
per-layer (they do not chain), but the matched job resolves
`seam_position = aligned`, so that relocation buys the destination nothing and
is not route work.

**Re-entry condition.** Only if the destination is redrawn to include a
two-phase layer pipeline (all per-layer perimeters before seam planning) or a
host-orchestrated seam pass. Both are architecture work under ADR-0045/0046,
neither is on the t128 route, and ADR-0046's alternatives already rejected the
host path.

**Disclosures.** Fact-verification take only: no code change, no measurement,
no timing claim, and no reference-set movement. ADR-0046 is `accepted` and was
not superseded (checked across `docs/adr/`, `docs/DEVIATION_LOG.md`, and the
packet records); its amendments (packets 178/179/180) change the source
geometry, the visibility budget and the default mode, never the placement
decision.
