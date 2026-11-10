# Staged fold: canonical skirt-brim re-take and cross-layer occupancy offset batching

Type: task / staged re-take + BUILD candidate
Label: wayfinder:task
Status: **Stage 1 resolved 2026-11-10** (the canonical skirt/brim re-take);
  Stage 2 is open, unclaimed and still gated by 137
Assignee: dev (session 39-40 — the human said "Continue the perf vs orca map"
  (no ticket named); 135 Stage 1 is the first unblocked, unclaimed frontier
  ticket in order — 136 and 135 Stage 2 are gated by 137, and 137 is the
  human's)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: [137](137-planner-scheduling-gate.md) — **Stage 2 only** (the
  four-cell gate form must be confirmed). Stage 1 is unblocked and may run
  first (2026-11-09 fold ruling).
Graduated from: [Planner effective parallelism](134-planner-effective-parallelism.md)'s
  2026-11-09 ruling: the per-layer occupancy offsets are the largest serial term left in
  the planner (**14,249 ms wall / 14,169 ms thread** on base classic-on, **0.99x**), while
  the clip core already runs at **4.81x**. The derived ceiling is **11.2 s** at that fan-out.
  **Fold ruling (2026-11-09, [Matched settings vs the 8 mm brim](130-brim-matched-settings-ruling.md)):**
  this ticket is also the brim/skirt re-take — the canonical port + matched
  adhesion/priming values + reference re-record + scoreboard re-derivation are
  its Stage 1 (staged with separate gates; a fused single candidate is rejected
  as the t115 failure mode).

## Stage 1 (runs unblocked): the t130 folded re-take

The 2026-11-09 ruling closes both matched-settings asymmetries here, staged
before the candidate so attribution survives:

1. **Canonical port.** Replace `SkirtBrim::generate_brim_entities` /
   `generate_skirt_entities` (`modules/core-modules/skirt-brim/src/lib.rs`)
   bounding-box rectangles with canonical geometry ported from the pinned
   `v2.5.0` (`40ecc66f78`): `make_brim` (`Brim.cpp`) and the skirt construction
   in `Print.cpp` (object-convex-hull anchored). The port carries the standard
   OrcaSlicer attribution header (`docs/ORCASLICER_ATTRIBUTION.md`), and pins
   `min_skirt_length` semantics.
2. **Matched keys, both arms, fail-closed.** Author `brim_width = 8`,
   `skirt_loops = 6`, `skirt_distance = 3.0`, `skirt_height = 1` in the four
   matched configs (`evidence/matched-pair/configs/`); amend
   `New-OrcaOverrideArgs` (`--brim-width=0` → `--brim-width=8`, plus
   `--skirt-loops=6 --skirt-distance=3 --skirt-height=1`) and assert all four
   resolved values per run on both sides (`Test-OrcaEvidence` and a PnP-side
   appendix assert). The t103 pin doctrine re-scopes from "2.4-era values" to
   "the matched job".
3. **Fresh 16-cell output campaign that records the NEW reference set** —
   t123's candidate-ordinary set is retired as the currency and every
   acceptance citing it re-points. Arms byte-identical within themselves.
4. **Delta audit (the port's correctness gate):** against the old set, moved
   lines are confined to skirt/brim entities and the config appendix — zero
   non-skirt/brim structural geometry lines move. **Parity oracle:** PnP's
   emitted skirt/brim loops geometrically match Orca's emitted skirt/brim loops
   for the same job in all 8 cells, within a declared tolerance (canonical
   citations by file + function at `40ecc66f78`).
5. **Old-vs-new paired A/B** (the standing paired form, wall + CPU, interleaved)
   recording the port's cost per cell — attribution only; the port is mandated,
   not a KEEP/DROP candidate.
6. **Closing gate: the 8-cells × 3-tools scoreboard re-derivation** (standing
   matched-pair protocol with the t103/t98 pins and the new asserts),
   superseding t124's ratios of record. Until then they stand as historical and
   every scoreboard keeps the disclosure.

Disclosures: `skirt_per_object` (Orca chain `1`, PnP global) is single-object
moot in this matrix; `min_skirt_length` is pinned by the port; the Orca source
vs installed-binary non-identity caveat is unchanged.

## Stage 2 question: the cross-layer occupancy batch

[Planner effective parallelism](134-planner-effective-parallelism.md) measured the
planner's remaining serial term: `inflate_model_occupancy_memoized` still issues one
singular `host::offset_polygons` per active layer for the emit check (**433 calls /
14,169 ms thread / 14,249 ms wall** on base classic-on; **186 / 585 ms / 598 ms** on
benchy classic-on). The calls are independent across layers, and the emit pass already
buffers four layers for the clip batch (`EMIT_BATCH_LAYERS`). Batch the per-layer
occupancy payloads across the buffered layers with the same `offset_polygons_batch`
service the role carve uses, then serve both memo sites from the batch result.

The measured ceiling is **11.2 s** at the planner clip core's **4.81x** fan-out
(**8.7 s** at t78's 2.6x clamp, **13.0 s** at 12x), i.e. **11.1x** the 1,010 ms floor
and **12.1 %** of base classic-on's 92.5 s required prepass cut. It is a tranche, not
the route: the planner's whole scheduling potential is below the 50.66 s share t134
priced as the route threshold.

## Why this is not t125

[Ticket 125](125-strong-planner-occupancy-offset-batching.md) batched the **per-origin
role-carve** offsets inside the bounded flush and failed the gate in six of eight cells,
with base classic-on gaining wall but regressing process CPU. This shape batches the
**per-layer emit-check** payloads (the 433 unique offsets t132 did not memoise) across
the layer chunk, and leaves the per-origin carve exactly where t132 left it. t125's
failure is the control: the candidate must report the per-call wall and CPU of both
halves and must not silently move the role-carve half into the batch.

## Pre-declared gates

**Stage 2** runs under the **t117 four-cell form**, because the four supports-off
cells are structurally inert: the whole `PrePass::SupportGeometry` stage is
**0.64–0.81x** the 0.80 % floor there (t134 measured), so no planner-only change can pass
the literal all-8-cell form. The final form is
[137](137-planner-scheduling-gate.md)'s to rule:

- Byte-identical G-code in all eight cells on both build arms (against the Stage-1
  reference set).
- Positive median wall with CPU corroborating at 0.80 % in all four supports-on cells.
- No median wall or CPU regression in the four supports-off cells.

**Stage 1** gates: the 16-cell campaign + delta audit + parity oracle (above), the
old-vs-new paired attribution, and the closing scoreboard re-derivation. The old-vs-new
delta is expected and disclosed, not gated KEEP/DROP.

## Acceptance

- Stage 1: canonical port with the attribution header; all four keys authored and
  asserted per run on both arms; a fresh 16-cell output campaign recording the new
  reference set (every hash named in this ticket's closure); the delta audit (moved
  lines confined to skirt/brim + appendix) and the skirt/brim parity oracle; the
  old-vs-new paired A/B recording the port's cost; the 8-cells × 3-tools scoreboard
  re-derivation superseding t124's ratios of record.
- Stage 2: candidate build paired against the Stage-1 baseline; four supports-on cells
  at or above the decided floor with CPU corroborating, four supports-off cells
  regression-free; a measured conversion of the occupancy thread-time to wall on the
  candidate arm, so the 4.81x proxy is replaced by the batch's own fan-out.

## Answer

Stage 1 resolved 2026-11-10. The canonical port lands, the matched job is authored
and asserted on both arms **including the adhesion flow model**, and the **new
reference set** replaces t123's. The strict skirt/brim parity oracle is **2/8**;
the residual is measured and attributed below rather than hidden.

**The port.** `SkirtBrim::generate` builds the brim as the canonical annulus:
`offset(outline, brim_object_gap)` to `offset(outline, gap + quantised_width)`,
minus the first-layer **support region**, filled with concentric lines half a
spacing inside the outer boundary and one spacing per pass, emitting both the
outer and the hole boundary of each shrunken band (`make_brim` /
`tryExPolygonOffset`, `Brim.cpp`). The skirt is the convex hull of the first
`skirt_height` layers **plus the brim footprint**, offset by
`skirt_distance + spacing/2 + i x spacing` with `jtRound` and 0.1 mm arc
tolerance (`Print::_make_skirt`, `Print.cpp`). The brim and skirt spacing is the
canonical flow spacing `line_width - first_layer_height x (1 - pi/4)`
(`Flow::rounded_rectangle_extrusion_spacing`), 0.35708 mm at the matched job.
Canonical at the pinned `v2.5.0` = `40ecc66f78`, OrcaSlicer attribution header
present. Module keys added: `brim_object_gap` (0.1), `min_skirt_length` (0;
**non-zero fails closed** — no filament cross-section model) and
`first_layer_height` (flow-spacing input).

**Three measured corrections to the t130 instruction** (all are matched keys, all
asserted per run from each arm's resolved appendix):

- **`brim_type = outer_only`.** The vendor chain resolves `auto_brim`, which
  recomputes the width from its adhesion heuristic and discards an explicit
  `--brim-width=8` (measured on benchy classic-off: a ~10 x 5 mm, 3-loop brim).
- **`initial_layer_line_width = 0.4`.** This is the adhesion flow model. Orca
  resolved 0.5 mm while PnP prints layer 0 at its 0.4 mm `line_width`, so the
  same 8 mm quantised to **7.31 mm vs 8.0 mm** and the skirt bands were
  **2.74 mm vs 2.40 mm**. Pinning Orca to 0.4 makes both spacings 0.35708 mm.
  The t82 fairness table's "line_width 0.4 on both sides" claim is **falsified
  by the resolved appendix** (Orca resolves 0.42 for walls, 0.5 for layer 0).
- **`tree_support_brim_width = 0`.** Orca resolves 3 mm and prints support brims
  under the same `; FEATURE: Brim` label; PnP's tree-support module emits none.

**Support clipping must be region-level, not bead-level.** Canonical subtracts
the support *slice contour*; the IR carries bead centerlines, so the port closes
the bead union at `2.5 x line_width` and fills it. The bead-exact variant left
17 brim loops where Orca prints none on both base supports-on cells, and clipping
ring *lines* instead of the *area* let the brim thread between support beads
(470 of 1,401 brim points within 0.1 mm of a bead).

**Output campaign (16 cells, two build arms).** The baseline arm (pre-port
module, identical host binary) reproduces t123's eight hashes exactly, so every
move is attributable to the port. The candidate records the new reference set,
identical on both arms: `7db0641b…`, `c02aa4e9…`, `a2f7a1fe…`, `2326455d…`,
`7ef84447…`, `1ddea2b7…`, `defae354…`, `4aa1847c…` (full hashes in
`evidence/t135-skirt-brim-retake/reduced/summary.json`). **Every subsequent
acceptance that cites t123's set must re-point to this one.**

**Delta audit.** Every motion line outside `Skirt`/`Brim` is identical after the
extrusion field is stripped, in all eight cells; the extrusion field differs by
at most **0.00098 mm** (declared tolerance 1e-3 mm, an artefact of the running
extrusion accumulator).

**Parity oracle (translation-invariant; bands declared before the first campaign
and not widened).** **2/8 pass.** Benchy supports-off passes outright: brim work
**+2.4 / +2.6 %**, brim area **+1.3 %**, skirt area **+2.3 %**, 22 brim lines on
PnP against Orca's fragmented 30. Base supports-off is **22 / 22 lines** and
+8.1 / +7.3 % brim work, with skirt area +3.2 / +3.1 % against a 3 % band — the
residual is the **anchor convention** (PnP anchors on its outer-wall path, Orca
on the slice contour; g-code carries the path, not the contour, so the comparison
carries a ~0.2 mm bias). Supports-on: on base both arms clip the entire brim away
(0 / 0 loops), but the skirt hull differs 12.5 % because `_make_skirt` folds the
support footprint in and PnP's tree supports are not Orca's `tree_strong`; on
benchy Orca's brim fragments into **39 runs** around its support pads against
PnP's 19, so its brim area is 3x larger. Radial extent is 8/8 within 0.92 mm.

**A/B (3 interleaved pairs, same host binary both arms).** Wall ratios
0.963–1.046, CPU 0.992–1.026 — the port's cost is inside the box's paired noise
floor; no cell wins on both axes in 3/3 pairs.

**Scoreboard re-derivation (8 cells x 3 tools x (1 warmup + 3 measured)).**
**2.445x–3.769x ordinary / 2.420x–3.745x accelerated; required cut
59.1 %–73.5 %.** Every PnP cell has one hash across both build arms equal to the
new reference set; PnP runs are `degraded = false` with zero fatal errors; every
Orca run asserts the seven matched keys from its resolved appendix. **t124's
2.36x–4.65x is superseded, not improved**: the adhesion job, the first-layer
width and the support brim all changed. One batch was aborted by a full disk mid
`base-classic-on`; the three remaining base cells were re-taken in a second
interleaved batch and both are appended in one CSV.

**Disclosures.** `auto_brim` overrides `brim_width`; `min_skirt_length > 0` fails
closed; support clipping is a declared closing radius; the skirt simplify is
collinear-only against canonical's 0.05 mm Douglas-Peucker; Orca's per-feature
wall widths (0.42 / 0.45 / 0.5) still differ from PnP's 0.4 and matching them is
a separate decision; source-vs-installed-binary caveat unchanged (t128); shared
6-physical / 12-logical-core box, scoreboard in two windows. Evidence:
[`evidence/t135-skirt-brim-retake/`](../evidence/t135-skirt-brim-retake/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 16/16 negative controls rejected).

**Stage 2 is untouched**: the cross-layer occupancy batch remains open,
unclaimed, and gated by [137](137-planner-scheduling-gate.md).
