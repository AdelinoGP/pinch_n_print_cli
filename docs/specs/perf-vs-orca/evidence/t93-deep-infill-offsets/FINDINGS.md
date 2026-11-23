# t93 findings — whole-layer deep-infill offsets

Ticket: [Whole-layer deep-infill offsets in internal-bridge qualification](../issues/93-whole-layer-deep-infill-offsets.md).
Take: 2026-10-28 → 2026-10-29, HEAD `7b42afed`.

Phase 1 screens four shapes named by the ticket against the map's **0.80 %**
floor on the base classic supports-off cell (0.585 s of the 73.17 s cell):

| shape | verdict | measured ceiling |
| --- | --- | --- |
| 1. lazy dilation (bbox precheck) | **measured-dead** | **0 %** — 0 of 130 calls skip |
| 2. windowed erosion | **above floor** | **1.38–1.43 %** of the cell |
| 3. cheaper erosion predicate | **rejected in Phase 1** | no representation-equivalence proof under Miter joins |
| 4. duplicate recompute reuse | **measured-dead** | **0.28 %** absolute ceiling, and no exact reuse exists |

Phase 2 therefore builds shape 2 only; shapes 1, 3 and 4 are recorded as
closed in the ticket. The candidate is host-only and changes no geometry
function: it confines the `-4.5 × spacing` erosion to the neighbourhood both
of its consumers can observe.

**Final ruling: human-approved DROP (2026-10-29).** The candidate passed the
16-cell proof and won both ordinary cells, but the pre-declared four-cell wall
clause was missed by the two accelerated cells (`+0.13 %`, `+0.62 %`). The
source is reverted; the candidate is preserved as `candidate.patch`.

## Phase 1 — screening

One cell, the ticket's declared screening cell: **ordinary base classic
supports-off**, matched config
`evidence/matched-pair/configs/pnp-classic-supports-off.json`, `tmp/base.stl`,
`RAYON_NUM_THREADS=12`, host-only probe (`PNP_T93_PROBE=1`) applied to
`crates/slicer-runtime/src/slice_postprocess_prepass.rs` and reverted before
this take closed. Two runs; run 2 measured the windowed counterfactual cold
(before the production erosion) to test order bias.

**Output identity holds in both runs:** G-code SHA
`292df50179c4033d6204a0353e038534b4d3a98381126998bb5722a41a571a80`,
21,023,739 bytes — the post-t81 ordinary reference for this cell. Every run is
clean (`degraded=false`, `fatal=0`, `non_fatal=0`).

| run | wall | process CPU | cpu/wall | `PrePass::ShellClassification` |
| --- | ---: | ---: | ---: | ---: |
| 1 (full erosion first) | 84.287 s | 750.625 s | 8.906 | 13,826 ms |
| 2 (windowed erosion first) | 80.164 s | 735.969 s | 9.181 | — |

Instrumented wall is **higher than the uninstrumented cell's 73.17 s**
(t87 scoreboard): the probe's per-call timers and `--profile` overhead are the
gap. Instrumented timings are attribution only and are never a wall claim; the
run's own cpu/wall ratio is used only to convert accumulated work to a wall
equivalent, which is the map's established method (t89 → t91 → t92).

### Accumulated terms (both runs)

Accumulated = sum over the layer axis (the qualification phase runs 130 calls
over 12 threads). Wall-equivalent = accumulated ÷ the run's own cpu/wall ratio.
Share = wall-equivalent ÷ 73.17 s.

| term | run 1 ms | run 2 ms | calls | run 2 wall-eq | run 2 share |
| --- | ---: | ---: | ---: | ---: | ---: |
| `offset(deep_infill_area, +1.5 × spacing)` (clip area) | 12,770.0 | 12,434.6 | 130 | 1,354.4 ms | 1.85 % |
| production erosion `offset(deep_infill_area, −4.5 × spacing)` | 10,274.6 | 10,215.5 | 42 | 1,112.7 ms | 1.52 % |
| — windowed operand `intersection(deep, window)` | 63.4 | 80.1 | 42 | 8.7 ms | 0.012 % |
| — windowed erosion `offset(operand, −4.5 × spacing)` | 899.9 | 891.3 | 42 | 97.1 ms | 0.133 % |
| **windowed-erosion saving** | **9,311.3** | **9,244.1** | — | **1,006.9 ms** | **1.376 %** |
| duplicate recompute calls (whole-call total) | 1,948.6 | 1,890.1 | 9 | 205.9 ms | 0.281 % |
| pre-declared floor | — | — | — | 585.4 ms | 0.80 % |

The two runs' windowed saving agrees within **3.70 %** (run 1 full-first, run 2
window-first), so the measurement is not an order artifact.

### Shape 1 — lazy dilation is measured-dead

The precheck is conservative and exact: a Clipper2 Miter offset with the
default limit 2.0 cannot leave the operand bbox grown by `2 × |delta|`, so a
surface more than `5 × spacing` from the deep-area bbox cannot reach the
`+1.5 × spacing` clip area. On this cell it **never fires**:

- **0 of 130** calls have every surface farther than `5 × spacing` from the
  deep bbox (`bbox_skip=1`).
- **130 of 130** calls have at least one surface whose expanded candidate
  intersects the deep clip area.

The deep sparse area sits directly beneath the solid-fill surfaces on every
bridge candidate layer, so the precondition this shape needs does not occur on
the matched job. Ceiling: **0 ms**.

### Shape 2 — windowed erosion clears the floor

The erosion is only queried inside the pre-retain qualified set: the `retain`
predicate intersects each qualified polygon against it, and
`internal_bridge_angles`' per-candidate filter intersects
`offset(candidate, spacing)` (reach `2 × spacing`) against it. A Miter offset
with limit 2.0 displaces no vertex farther than `2 × |delta| = 9 × spacing`, so
clipping the operand to the qualified bbox grown by `14 × spacing` leaves every
observable point unchanged: the cut is farther than the `9 + 2 = 11 × spacing`
reach, and the remaining `3 × spacing` is margin.

Measured directly by a probe-only second erosion on the windowed operand:

- 42 calls carry a non-empty pre-retain qualified set; 14 survive the retain.
- Production erosion **10,215.5 ms** accumulated; windowed operand + erosion
  **971.4 ms** — a **9,244.1 ms** saving.
- Wall-equivalent **1,006.9 ms = 1.376 %** of the cell, **1.72× the floor**.
- Two calls regress slightly under windowing (run 1: −1.7 ms and −19.0 ms;
  run 2: −12.5 ms on one call) because the rectangular cut adds collinear
  vertices. The net is dominated by the 39 calls that shrink; the candidate
  always windows and the paired campaign measures the net.

### Shape 3 — cheaper erosion predicate is rejected, not built

`polygon ∩ erode(deep, 4.5 × spacing) ≠ ∅` is not provably equal to
`dilate(polygon, 4.5 × spacing) ∩ deep ≠ ∅` under **Miter** joins: Miter
dilation and erosion are not adjoint (a Miter join is not a metric ball), so
the two predicates can disagree at corners. The ticket says to reject it in
Phase 1 without a representation-equivalence proof rather than build it.
Rejected; no probe time spent.

### Shape 4 — duplicate recompute is measured-dead twice over

- **9 of 130** calls are phase-B recomputes (121 precompute + 9 recompute);
  each recompute key appears exactly twice.
- **0 of 9** duplicate pairs carry an identical `deep_fp`: the recompute's
  `deep_infill_area` is `remove_filled_polygons_on_lower_layers(...)`, so the
  lower committed bridges change the operand. Reusing the first pass's offset
  results is therefore **not exact** — the ticket's parenthetical "the deep
  area does not depend on bridge commits" does not hold for the operands the
  `deep_fp` fingerprints capture.
- Even deleting every recompute to zero is only **205.9 ms wall-equivalent =
  0.281 %**, below the 0.585 s floor.

### Verdict

Sum of the removable ceilings: shape 2's 1,006.9 ms plus shape 4's 205.9 ms
(shape 1 zero, shape 3 rejected) = **1,212.8 ms = 1.66 %** of the cell, above
the floor. Phase 2 is authorised for shape 2 only.

## Phase 2 — candidate

The candidate is `eroded_area_in_qualified_neighbourhood`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`). It replaces the
whole-layer `offset(deep_infill_area, −4.5 × spacing)` with the same offset on
the operand clipped to the pre-retain qualified set's bbox grown by
`14 × spacing` (Clipper2 Miter reach `9 × spacing` + the angles filter's
`2 × spacing` candidate expansion + `3 × spacing` margin). No geometry
function changes; both consumers (`retain` and `internal_bridge_angles`'
per-candidate filter) observe the same set.

`windowed_erosion_preserves_every_observable_qualified_neighbourhood_verdict`
pins the contract on a fixture whose qualified set contains one polygon inside
the sparse area and one wholly disjoint from it, with the window covering a
small part of the area: the `retain` verdict and the angles-filter verdict
agree with the full erosion for both polygons, and the windowed operand is
strictly smaller. `cargo test -p slicer-runtime --lib
slice_postprocess_prepass` 5/5; both `calicat_internal_bridge_*` e2e tests
pass.

### Proof matrix — 16/16 cells

Baseline and candidate arms, both modes, all `{benchy, base} × {classic,
arachne} × {supports off, on}` cells:

- **16/16** cells are byte-identical between arms, one SHA per arm.
- **8/8** ordinary cells equal the post-t81 reference set
  (`t91-exact-z-memo/references.json`).
- **8/8** accelerated cells equal their ordinary counterparts byte for byte.
- **24 modules** per snapshot; the module trees are byte-identical between arms
  in both modes (`provenance.json`); the binaries are distinct per arm.
- Every run is clean (`degraded=false`, `fatal=0`, `non_fatal=0`), with
  non-vacuous `Outer wall` / `Sparse infill` (and `Support` / `Support
  interface` on supports-on) roles.

`qualification.json`: `pass=true`, 16 cells, zero problems.

### Paired campaign — four pre-declared base classic cells

2 warmups + 6 alternating pairs per cell, 12 threads, external
`GetProcessTimes` wall and process CPU, zero starvation exclusions (all
cpu/wall ratios within ±3 % of their arm's median).

| cell | baseline wall | candidate wall | paired wall | paired CPU | pairs | wall clause |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| ordinary base classic-off | 74.02 s | 71.59 s | **−3.31 %** (−2.43 s) | **−1.19 %** (−9.7 s) | 6/6 | pass |
| ordinary base classic-on | 170.34 s | 173.22 s | **−0.26 %** | **−0.48 %** (−7.3 s) | 6/6 | pass |
| accelerated base classic-off | 81.01 s | 81.58 s | **+0.13 %** | **−0.81 %** (−6.5 s) | 6/6 | **fail** |
| accelerated base classic-on | 165.90 s | 168.10 s | **+0.62 %** | **−0.45 %** (−3.2 s) | 6/6 | **fail** |

Every retained pair is byte-identical, each arm carries one SHA, and the two
arms are distinct binaries in all four cells.

**The pre-declared gate is NOT MET on the letter:** it requires paired-median
wall below baseline in all four cells, and the two accelerated cells measure
`+0.13 %` and `+0.62 %`. Both misses are far inside their own cells' repeat
ranges (accelerated-off baseline 76.9–85.4 s, candidate 78.0–83.1 s;
accelerated-on baseline 161.0–173.4 s, candidate 163.0–172.5 s) and the paired
CPU falls in both (`−0.81 %`, `−0.45 %`). The ordinary cells pass the wall
clause, and the ordinary base classic-off cell is the production default.

### The win is real work, and the wall transfer is larger than the work equivalent

On ordinary base classic-off the campaign's process-CPU fall is **9.7 s**,
which matches the Phase-1 probe's **9.2–9.3 s** accumulated erosion saving
(within 5 %). The wall fall is **2.43 s**, larger than the `CPU ÷ baseline
ratio` projection of ~1.0 s because the candidate also raises the arm's
cpu/wall ratio (**9.65 → 9.84**): less of the wall is spent stalled while the
rest of the slice runs. The candidate removes work; it does not merely spread
it. The same signature appears in the accelerated cells (CPU falls 6.5 / 3.2 s)
even though their walls are neutral.

### Disclosures

- **Campaign interruption.** The opencode server restarted during
  `ordinary-base-classic-on`; that cell had 7 of 14 rows and was re-run whole.
  The partial capture is preserved as
  `measure-interrupted-20261029/ordinary-base-classic-on/` and is not ledger
  input. `ordinary-base-classic-off` had completed before the restart and is
  used as measured; the other three cells were measured in the resumed round
  on 2026-10-29. Ratios are paired within each cell, not across rounds.
- **Load.** 4.6 % before the campaign, 2.0 % before the resumed round; no
  starvation exclusions. Pair 4 of ordinary-base-classic-on moved +6.08 % the
  wrong way on wall (CPU +0.33 %); pairs 4–6 of accelerated-base-classic-on
  are the source of that cell's +0.62 % paired median. These are disclosed, not
  averaged away.
- **Instrumented wall is never a wall claim.** Phase-1 walls were 84.3 / 80.2 s
  against the uninstrumented 73.17 s cell; only accumulated ÷ cpu/wall is used.
- **The arm medians and the paired median disagree in ordinary-base-classic-on**
  (arm medians +2.9 s, paired median −0.26 %). The pre-declared metric is the
  paired median; the discrepancy is the cell's own spread, and the cell is a
  wash within measurement resolution.
- **Both guest freshness gates exit 0** before and after the take; the candidate
  file is not in any guest's dependency closure.

### Ruling

**Human-approved DROP (2026-10-29); the candidate is reverted.** The candidate
removes measured work (CPU falls in all four cells, matching the probe) and
wins the production-default ordinary mode in both cells
(−3.31 % / −0.26 % wall), byte-identical everywhere. The strict four-cell wall
clause is missed by +0.13 / +0.62 pp in the accelerated cells, both inside
their own repeat ranges — the same class of sub-noise miss the human accepted
on t75 (+1.08 % CPU) and t91 (+0.08 pp CPU), but here the clause was
pre-declared for all four cells and the two misses are positive, so the human
ruled DROP. The re-appliable diff is `candidate.patch`; the tree carries no
production change and both freshness gates exit 0.

## Evidence

- `probe-rows.json` — the committed reduced observation (two runs, 130 call
  rows each, both offsets, bbox verdicts, windowed counterfactual, duplicate
  fingerprints).
- `probe.patch` — the env-gated Phase-1 probe, re-appliable to HEAD.
- `candidate.patch` — the dropped Phase-2 candidate, re-appliable to HEAD
  (the tree carries no production change).
- `extract_probe.py`, `verify.py` — reduce and re-derive the Phase-1 table;
  `verify.py --self-test` mutates the inputs and rejects every negative
  control.
- `build-snapshots.ps1`, `run-cell.ps1`, `run-all-proof.ps1`,
  `run-all-measure.ps1`, `extract.py`, `reduce.py`, `verify.py` — the Phase-2
  harness.
- `proof-rows.json`, `measure-rows.json`, `qualification.json`,
  `ab-reduction.json`, `summary.json`, `provenance.json` — the committed
  reduced observations every headline number is re-derived from.
- Raw captures live under
  `.local-artifacts/perimeter-reference-preparation/t93-deep-infill-offsets/`
  (gitignored, optional); nothing in the committed set reads them.
