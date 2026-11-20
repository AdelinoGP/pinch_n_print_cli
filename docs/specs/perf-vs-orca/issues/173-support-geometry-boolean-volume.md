# Support-path boolean volume: who issues the 186 s of `clip_polygons`, and can it shed 92–96 % of `SupportGeometry`?

Type: task / attribution + shape pricing (instrumented probe; no candidate in
this take unless a shape is measured on the standing gate)
Label: wayfinder:task
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t171 post-t169 re-price](171-post-t169-prepass-reprice.md)
([`evidence/t171-post-t169-reprice/FINDINGS.md`](../evidence/t171-post-t169-reprice/FINDINGS.md)),
which re-derived the post-t169 split and found that the two base supports-on
cells are the only cells whose cut is **boolean**-shaped: `clip_polygons` is
**60.8 % / 60.9 %** of their host clipper thread-time while `offset` is
**34.2 % / 34.1 %**, and the whole `PrePass::SupportGeometry` stage would have to
shed **96.0 % / 92.4 %** of itself to reach 1.0x.

## Question

**What issues the support path's boolean volume, what does each site cost, and
does any reduction shape clear `base classic-on`'s 52.8 % / `base arachne-on`'s
50.8 % cut — or does the supports-on cluster also need a re-aim?**

Concretely:

1. **Census the `clip_polygons` volume by call site.** On `base classic-on` the
   `--profile` channel attributes **186.6 s of 343.4 s** host clipper thread-time
   to `host:native` (`polygon_ops::clip_polygons`, 169,586 calls), the bucket the
   tool documents as "native `polygon_ops` activity that no host built-in bracket
   claimed". Name the sites: which module, which stage, which function, how many
   calls, what operand size, and what thread-time each carries.
2. **Decompose `PrePass::SupportGeometry`** (54.9 % of both base supports-on
   cells; 27.6 / 27.6 s of instrumented wall) into guest self fuel, host clip,
   host offset and any per-role sub-phases, so the stage's 92–96 % requirement is
   priced against named terms rather than a bracket.
3. **Price the surviving shapes.** Candidate families already partly screened:
   operand size (point counts pushed per call), call count (batch/fan-out), and
   deletions (work with no production reader). State each shape's ceiling in ms
   and as a fraction of the cell's cut, with the 0.80 % floor arithmetic shown.
4. **Decide the cluster's next move:** a named shape, or the explicit finding
   that no boolean-volume shape reaches 52.8 % / 50.8 % and the supports-on base
   pair also needs a re-aim.

## What is already known

- **t171's census (2026-11-20, the post-t169 tree):** host clipper thread-time per
  cell is clip **53.1–60.9 %** / offset **34.1–39.4 %** on the supports-on cells
  and offset **42.9–51.2 %** / clip **29.5–34.4 %** on the supports-off cells.
  Owner split on `base classic-on`: `host:native` 236.3 s (clip 186.6, offset
  49.7), `host:shell_classification` 53.9 s, `host:support_analysis` 35.7 s
  (offset 32.6), `host:slice` 12.7 s. `host:shell_classification` is 100 % marked
  clipper work; the support stages are not.
- **The stage arithmetic is tight, not roomy.** `SupportGeometry` is 54.9 % of
  `base classic-on` and 54.9 % of `base arachne-on`; zeroing it leaves **0.95x /
  0.92x**, and the cut needs **96.0 % / 92.4 %** of the stage. `SupportAnalysis`
  adds only 8.4 % (zeroing it leaves 1.94x / 1.86x).
- **Guest fuel says the same cluster:** `tree-support` is **65.7 % / 68.0 %** of
  `base classic-on` / `base arachne-on` fuel and `tree-support-planner` another
  **10.4 % / 10.7 %**; the supports-off cells are `classic-perimeters` 65.0 % /
  `arachne-perimeters` 56.0 % instead.
- **t170's bound is the other direction:** even at zero cost for every host and
  guest `offset` / `offset2_ex` second, `base classic-on` reaches only **23.3 %**
  of its 52.7 % cut. The supports-on base pair is therefore *not* an offset
  problem, which is what makes a boolean census the next question.
- **Prior screens, so they are not re-run blind:**
  [t107](107-support-geometry-producer-decomposition.md) priced the *host
  producer* at 9 ms (0.0068x the floor) — that is not this volume;
  [t125](125-strong-planner-occupancy-offset-batching.md) DROPped per-origin
  occupancy batching (2/8 cells);
  [t134](134-planner-effective-parallelism.md) / [t136](136-planner-clip-batch-fanout.md)
  split the planner clip core (carve **11.56x**, union **3.66x** at 4.4 requests
  per call) and landed union-phase grouping at depth 12;
  [t135](135-cross-layer-occupancy-batching.md) Stage 2 landed the cross-layer
  occupancy batch (3.52x fan-out);
  [t160](160-tree-support-prepass-batch-probe.md) measured the renderer fold as
  **slower** in all four supports-on cells, so placement is not the lever and the
  renderer's cost is the work itself.
- **t131's family census** is the closest existing decomposition: the
  `+0.35 mm` occupancy family is 864 calls / 433 distinct payloads on
  `base classic-on`, the `−0.6 / −0.8 mm` collision family **1,350 calls over
  1,350 distinct payloads** (no memo hits). Neither is tagged by call site.

## Scope

1. One instrumented window, `--profile` per cell on the four supports-on cells
   (and one supports-off control), plus one targeted probe (t131-style seam
   timing, env-gated, reverted, patch preserved) that tags `clip_polygons` calls
   with their issuing site.
2. The site census: module, stage, function, calls, operand point counts, thread
   time; top sites ranked by measured thread time, with the unattributed share
   named.
3. The `SupportGeometry` decomposition and the shape prices, each expressed in ms
   and as a fraction of the cell's cut, with the floor and the 8-cell gate
   arithmetic shown.
4. A short list of shapes that survive with their measured carrier share, or the
   explicit re-aim finding.

## Acceptance

- `evidence/t173-support-boolean-volume/` with the census rows, the stage
  decomposition, the shape prices, `FINDINGS.md`, and a checkout-only
  `verify.py --self-test` that re-derives every quoted number from the committed
  reduced set.
- Map: destination-status block, Decisions-so-far line, and an updated
  **Not yet specified** (the surviving shapes with their prices, or the re-aim).
- No production edit unless a shape is measured on the standing gate; a
  per-layer or geometry-moving change would need a t81-style ruling first, per
  the map's standing precedent.

## Answer

_(pending)_
