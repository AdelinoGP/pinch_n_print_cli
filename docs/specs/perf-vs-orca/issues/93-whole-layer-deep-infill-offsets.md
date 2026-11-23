# Whole-layer deep-infill offsets in internal-bridge qualification

Type: task
Label: wayfinder:task
Status: resolved (2026-10-29; measured **GATE NOT MET** on the strict four-cell
wall clause — **human-approved DROP**, candidate reverted. Phase 1 closed three
of four shapes; windowed erosion cleared the floor and won the production
ordinary mode, but the two accelerated cells were wall-neutral (+0.13 / +0.62
pp). See ## Answer)
Assignee: 2026-10-28 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named); the frontier re-derivation found ticket 93
as the only open child
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Per-surface bridge boolean locality](92-per-surface-bridge-boolean-locality.md)

## Question

Can the two whole-layer offsets on the unioned deep sparse-infill area — the
`+1.5 × spacing` clip area and the `−4.5 × spacing` erosion filter — be made
cheaper or skipped without changing the internal-bridge qualification verdict,
and does the resulting wall win clear the map's **0.80 %** floor on the four
base classic cells?

**Screen first.** The total bracket is measured, but which fraction is
removable is not. Phase 1 sizes the removable part; Phase 2 builds the
candidate only if it clears the floor.

## Measured premise

The [t92 Phase-1 screen](../evidence/t92-per-surface-bridge/FINDINGS.md)
instrumented ordinary base classic supports-off and found that
`compute_internal_bridge_layer`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) spends almost
nothing on its per-surface shapes and almost everything on two whole-layer
offsets of `deep_infill_area` (the unioned sparse infill in the bridge-depth
window, from `gather_areas_w_depth`):

| term | run 1 | run 2 | calls | run 2 wall-eq | run 2 share |
|---|---:|---:|---:|---:|---:|
| `offset(deep_infill_area, +1.5 × spacing)` (clip area) | 12,462.9 ms | 11,375.2 ms | 130 | 1,233.2 ms | **1.69 %** |
| `offset(deep_infill_area, −4.5 × spacing)` (erosion filter) | 10,042.1 ms | 9,276.2 ms | 14 | 1,005.6 ms | **1.37 %** |
| **both** | **22,505.1 ms** | **20,651.4 ms** | — | **2,238.8 ms** | **3.06 %** |

Accumulated over the layer axis; wall-equivalent = accumulated ÷ the run's own
cpu/wall (t92's established conversion). The current base classic supports-off
cell wall is **73.17 s** ([t87 scoreboard](../evidence/scoreboard-refresh-20261025/SCOREBOARD.md)),
so the floor is **0.585 s**. The pair is **3.8–4.3x the floor** and **~9x t92's
own (measured-dead) ceiling**.

Operand facts measured by t92:

- `deep_infill_clip_area` is only **1–7 polygons** per call (mean 1.75) but the
  Miter offset costs a median **81.6 ms** (max 222.4 ms) — complex ring
  geometry, not polygon count.
- The erosion runs on the **14 of 130** calls whose qualified set is non-empty
  (18 qualified polygons total) and still costs **44 %** of the pair, up to
  **428.3 ms** on one layer.
- `region.internal_solid_fill` is tiny (196 surfaces over 130 calls, max 7), so
  the per-surface consumer set (`offset(surface, spacing)` + intersections) is
  **0.25 s accumulated** — the offsets, not their consumers, are the cost.

Both offsets are **whole-layer operands**: `deep_infill_clip_area` is the
per-surface clip operand *and* the `internal_bridge_angles` operand
(`expansion_area` intersection, per-candidate `area` filter), and the erosion
is the `retain` predicate's operand. Restricting the *consumer* to a bbox (t92's
Phase 2 step 1) cannot remove the offset build — which is why this is a
separate ticket, not a reopened t92.

Known shapes to price, in order of expected value:

1. **Lazy dilation.** Build `offset(deep_infill_area, 1.5 × spacing)` only when
   a conservative bbox precheck says some surface can reach the deep area
   (surface bbox grown by `spacing + 1.5 × spacing` plus a Miter-extension
   margin, vs the deep-area bbox). Exact by construction; the question is how
   many of the 130 calls skip it. t92 saw 130 calls and 14 qualified sets, but
   did **not** count how many calls had any surface passing the clip test.
2. **Windowed erosion.** Only the 14 qualified layers pay the erosion, and the
   qualified set is 18 polygons. Clip the deep-area operand to the qualified
   set's bbox grown by `4.5 × spacing` plus a Miter safety margin, erode, then
   clip the result back to the window. Miter offsets are local, so the result
   inside the window is unchanged; the win depends on how much of the layer the
   window covers. t84's overhang window (retains 97.84 % of points → 0.19 %
   ceiling, rejected) is the precedent to beat: here the window is driven by a
   tiny qualified set, not by a band layer.
3. **Cheaper erosion predicate.** `polygon ∩ erode(deep, 4.5 × spacing) ≠ ∅`
   is not provably equal to `dilate(polygon, 4.5 × spacing) ∩ deep ≠ ∅` under
   Miter joins. Price it only if it can be stated as a representation-equivalent
   test with a proof or an exhaustive fixture check; otherwise reject it in
   Phase 1 rather than building it.
4. **Duplicate recompute.** The phase-B path recomputes a layer's whole
   `compute_internal_bridge_layer` when lower candidates exist. Count how often
   the same `(layer, deep_infill_area)` is offset twice; if it is common, the
   recompute path may reuse the first pass's offset results (exact: the deep
   area does not depend on bridge commits).

## Phase 1 — screening (mandatory)

1. Instrument an ordinary base classic supports-off slice with a
   reverted-before-close probe that records, per call: the deep-area polygon,
   hole and ring-vertex counts, both offset spans, whether any surface passed
   the clip test, the conservative bbox-precheck verdict for shapes 1, 2 and 4,
   and the number of duplicate recomputes.
2. Convert each shape's removable accumulated time to wall with the run's
   measured cpu/wall ratio and compare the **sum** against **0.585 s**.
3. If the sum's conservative ceiling is below the floor, close as
   **measured-dead** with the probe reverted; no candidate.
4. If above, proceed to Phase 2 with only the shapes that cleared the floor
   individually or in a stated combination.

## Phase 2 — candidate (conditional)

1. Implement the measured shapes; no geometry function changes.
2. Emitted G-code must stay byte-identical to the post-t81 reference set in all
   sixteen cells, both modes.
3. An equivalence test on a fixture with a non-empty qualified set where at
   least one surface is disjoint from the deep area, and one where the
   qualified set is far from most of the deep area (the windowing case), each
   comparing the gated/lazy path against the current per-layer reference.

## Pre-declared floor

**0.80 % of the cell wall** (0.585 s on the current base classic supports-off
cell). Phase 1 measures the removable ceiling; Phase 2's paired campaign
measures the actual win.

## Proof gate

- Phase 1 probe reverted; both guest freshness gates exit 0.
- Phase 2 only: 16-cell output-identity matrix against the post-t81 reference
  set, both modes; the equivalence tests above.
- Phase 2 only: standing paired ordinary + accelerated gate on the four base
  classic cells, 6 alternating pairs per cell, 12 threads, zero starvation
  exclusions, every pair byte-identical, process CPU ≤ baseline + 3.0 %.
- Both guest freshness gates exit 0 (host-only candidate).

## Deliverables

1. The Phase 1 screening table with per-shape removable ceilings against the
   floor.
2. If above the floor, the candidate plus its equivalence tests.
3. The paired campaign results and a measured recommendation to the human. No
   auto-commit.

## Answer

**Measured GATE NOT MET on the strict four-cell wall clause — human-approved
DROP (2026-10-29, HEAD `7b42afed`).** The candidate is reverted (the tree
carries no production change); its re-appliable diff is
[`candidate.patch`](../evidence/t93-deep-infill-offsets/candidate.patch), and
the probe is
[`probe.patch`](../evidence/t93-deep-infill-offsets/probe.patch). Both guest
freshness gates exit 0 before and after the take.

### Phase 1 — three of four shapes closed

One ordinary base classic supports-off slice, two probe runs, outputs
byte-identical to the post-t81 reference `292df501…` and clean. Accumulated
over the layer axis; wall-equivalent = accumulated ÷ the run's own cpu/wall.

| shape | verdict | measured |
| --- | --- | --- |
| lazy dilation (`+1.5 × spacing` clip area) | **dead** | **0 of 130** calls skippable; **130 of 130** have a surface reaching the deep area |
| windowed erosion (`−4.5 × spacing` filter) | **above floor** | **9,244.1 / 9,311.3 ms** accumulated saving = **1,006.9 / 1,045.6 ms wall-eq = 1.38 / 1.43 %** of the cell vs the **585.4 ms (0.80 %)** floor |
| cheaper erosion predicate | **rejected** | no representation-equivalence proof under Miter joins (not adjoint) |
| duplicate recompute reuse | **dead** | **9** recomputes, **0** with an identical deep-area fingerprint; deleting all 9 reaches only **205.9 ms = 0.28 %** |

The erosion runs on **42** of 130 calls; the windowed operand costs **80.1 ms
clip + 891.3 ms offset** against **10,215.5 ms** for the full erosion (run 2).
The two runs agree within **3.70 %**; run 2 measured the windowed path first, so
the saving is not an order artifact.

### Phase 2 — candidate and proof

The candidate confines the erosion to the pre-retain qualified bbox grown by
`14 × spacing` (Miter reach `9 × spacing` + the angles filter's `2 × spacing`
candidate expansion + margin). Both consumers — the `retain` predicate and
`internal_bridge_angles`' per-candidate filter — observe the same set. No
geometry function changed; host-only.

**Proof: 16/16 cells byte-identical between arms, 8/8 ordinary cells equal to
the post-t81 reference set, 8/8 accelerated ≡ ordinary, 24 identical modules
per mode, distinct binaries per arm, zero unclean runs.** The unit test
`windowed_erosion_preserves_every_observable_qualified_neighbourhood_verdict`
pins both consumer verdicts on a fixture with an in-area and a disjoint
qualified polygon; `cargo test -p slicer-runtime --lib
slice_postprocess_prepass` is 5/5 and both `calicat_internal_bridge_*` e2e
tests pass.

### Campaign — ordinary wins, accelerated neutral

2 warmups + 6 alternating pairs per cell, 12 threads, zero starvation
exclusions, every retained pair byte-identical, one SHA per arm.

| cell | paired wall | paired CPU | wall clause |
| --- | ---: | ---: | --- |
| ordinary base classic-off | **−3.31 %** (−2.43 s) | −1.19 % (−9.7 s) | pass |
| ordinary base classic-on | **−0.26 %** | −0.48 % (−7.3 s) | pass |
| accelerated base classic-off | **+0.13 %** | −0.81 % (−6.5 s) | **fail** |
| accelerated base classic-on | **+0.62 %** | −0.45 % (−3.2 s) | **fail** |

The CPU fall matches the probe's 9.2–9.3 s accumulated saving on the screening
cell; the wall transfer is larger than the work equivalent because the
candidate raises the arm's cpu/wall ratio **9.65 → 9.84** (less stall, not
merely less CPU). The strict clause was missed by **+0.13 / +0.62 pp** in the
two accelerated cells, both inside their own repeat ranges — but it was
pre-declared as wall below baseline in **all four** cells, so the letter is NOT
MET. The human ruled **DROP**; no production change lands.

### Disclosures

- The opencode server restarted during `ordinary-base-classic-on`; that cell
  had 7/14 rows and was **re-run whole**. The partial is preserved as
  `measure-interrupted-20261029/` and is not ledger input.
  `ordinary-base-classic-off` had completed before the restart; the other three
  cells were measured in the resumed round on 2026-10-29. Ratios are paired
  within each cell, never across rounds.
- Load was 4.6 % before the campaign and 2.0 % before the resumed round; zero
  starvation exclusions. Pair 4 of ordinary classic-on moved +6.08 % the wrong
  way; pairs 4–6 of accelerated classic-on drive that cell's +0.62 %.
- Instrumented walls (84.3 / 80.2 s) are never a wall claim against the
  uninstrumented 73.17 s cell. The arm medians and the paired median disagree
  in ordinary classic-on (arm medians +2.9 s, paired median −0.26 %); the
  pre-declared metric is the paired median and the cell is a wash within its
  own spread.

Evidence:
[`evidence/t93-deep-infill-offsets/`](../evidence/t93-deep-infill-offsets/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, **18/18** negative controls
rejected).
