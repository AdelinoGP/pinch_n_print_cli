# T27 probe iteration 3 — SupportAnalysis sweep interior split (at HEAD, 2026-10-08)

Ticket: [Serial host floor](../../issues/27-serial-host-prepass-floor.md).
Probe: `PERF-T27-PROBE3` (temporary; removed before any commit; preserved as
`probe3.patch`). Driver: `run_probe3.ps1` (pwsh 7 — Windows PowerShell 5.1's
Roslyn rejects the expression-bodied `T()` helper, measured 2026-10-08).

Vintage: HEAD `b56f429a` (t19 KEEP) + probe. Guests fresh
(`cargo xtask build-guests --check` exit 0) before the batch. Two
uninstrumented slices, matched supports-on configs, `RAYON_NUM_THREADS=12`,
process wall/CPU via `GetProcessTimes`. Probe lines are `PERF-T27-PROBE3` rows
on stderr, outside the JSONL contract. Raw captures in gitignored
`target/matched-pair/t27-probe3/`; extracted `.probe.txt` files are committed
with this note's take.

## Why this probe

The ticket was parked 2026-09-29 pending "a new output-safe, measurably useful
reduction with a real-layer IR oracle". The 2026-09-27/28 probes attributed
every other stage: ShellClassification phase B (two serial Miter offsets),
`PrePass::Slice` (`assemble_flat_bridge_areas` ~70% of fuel, parallel),
OverhangAnnotation (parallel), the tail (teardown). **The SupportAnalysis
serial sweep (24.68 s wall base-on) had never been decomposed.** This probe
splits its interior.

## Instrumented row split (calling-thread wall per row; sweep is serial)

| Row | benchy classic-on | base classic-on | class |
| --- | ---: | ---: | --- |
| `sa3_stage_wall` (whole `commit_support_analysis_builtin`) | 2.150 s | 38.198 s | wall |
| `sa3_model_occupancy_sweep` | 0.819 s | 24.237 s | wall (serial) |
| `  sa3_occupancy_block` (map insert + bounds + top tracking) | 2.4 ms | 12.0 ms | wall |
| `  sa3_layer_polygons_extend` | 0.7 ms | 3.6 ms | wall |
| `  sa3_view_construction` (`SliceRegionView::from_ir` per region) | 3.2 ms | 17.0 ms | wall |
| `  sa3_needs_support_total` | 0.816 s | 24.217 s | wall |
| `    sa3_needs_support_predicate` (the exact `intersection_ex`) | **0.812 s** | **24.194 s** | wall |
| `sa3_layer_union_build` | 0.058 s | 0.562 s | wall |
| `sa3_modifier_slice` | ~0 | ~0 | wall |
| `sa3_contact_par_wall` | 1.272 s | 13.388 s | wall (parallel region) |
| `sa3_territory` | ~0 | ~0 | wall |
| `sa3_termination_tail` | 23 µs | 20 µs | wall |

Counts: 240 regions benchy / 495 base; 576 / 845 polygons summed across
`region.polygons`; the predicate runs once per region and is ~3.4 ms/call
benchy, **~48.9 ms/call base**.

## Reading

1. **The sweep's cost is one predicate**: 99.8% of the sweep and **63.3% of the
   whole SupportAnalysis stage** on base-on is
   `SliceRegionView::derive_needs_support` — specifically its per-region
   `intersection_ex(&self.polygons, &overhang.xy_footprint)`
   (`crates/slicer-sdk/src/views.rs`). Everything else in the sweep is
   milliseconds.
2. **The `SliceRegionView::from_ir` deep-clone hypothesis is dead** (17 ms /
   495 calls base). The view is constructed to read `object_id` + `polygons`
   and nothing else here; the cost is the Clipper call, not the clone.
3. This is the same shape t25 measured for the guest side (the exact
   intersection/winding predicate over big polygon sets is the cost), now
   host-side and serial: the object-global `xy_footprint` is the union over
   **all 326,546 overhang facets** on base, and it is re-intersected against
   every region cross-section on every layer, 495 times serially.
4. **Candidate direction (not yet measured):** a per-footprint-polygon bbox
   index (superset filter — bboxes only ever pass more candidates, never drop
   a true overlap) computed once per stage call, then the exact intersection on
   surviving polygons only. Output-identical by set algebra; the wall prize is
   bounded by the 24.19 s predicate, i.e. up to ≈ 22 s of the 38.2 s stage wall
   on base-on if the filter prunes most footprint area. Probe iteration 3b
   measures the filter's candidate counts, points, and an exactness mismatch
   counter against the unchanged predicate.
5. The same predicate is duplicated on the dispatch path
   (`marshal/prepared.rs::prepare_regions`, `marshal/in_.rs`,
   `marshal/native.rs`) but ticket 05's arena cache covers that side
   (`LayerArena::ensure_prepared_regions`); the sweep side is uncached.

## Probe iteration 3b — the bbox-filter candidate is refuted; the serial predicate is the shape

`run_probe3b.ps1` (same driver + candidate-measuring brackets) ran both cells
cleanly (benchy 28.62 s / 140.3 cpu; base 574.6 s / 2378.2 cpu, both
`ok/degraded=false`, G-code SHA identical to probe-3: `76eb4e1d…` /
`f60e5e9b…`). Measured:

- **Filter pruning is weak.** On base, 519k precomputed footprint-polygon
  entries over 495 regions; the region-bbox filter keeps **233k (≈45 %)**.
  Candidate-only `intersection_ex` costs **21.16 s** against the real
  predicate's 24.19 s — only ~12 % cheaper despite half the polygons. The
  predicate's cost is carried by the large merged polygons, not raw count.
- **Exactness holds** (0 mismatches in 495 + 240 dual-computed regions) but the
  saving does not justify the surface: this candidate is **dropped**.
- **The real shape is the serialness.** The 24.2 s predicate is pure per-region
  work sitting inside a serial loop, in a stage whose *next* pass
  (`sa3_contact_par_wall`, 13.2–13.4 s wall) already saturates 12 threads
  (~12x worker sum over wall). Moving the predicate into the existing rayon
  pattern preserves every value (identical function, identical inputs,
  `par_iter().collect()` preserves order) and puts the work on the parallel
  axis. Probe iteration 3c dual-computes the parallel value against the serial
  truth on both cells and measures the parallel pass wall.

## Probe iteration 3c — the parallel candidate: 8.7x on the predicate, exact values

`run_probe3c.ps1` ran the dual-compute form: the production shape of the
candidate (a `par_iter().map()` pre-pass over `slices × regions`, same empty
filter, order-preserving `collect()`) plus the unchanged serial predicate, with
a per-region comparison counter. Both cells clean (`ok/degraded=false`, G-code
SHA identical to probes 3a/3b):

| row | benchy classic-on | base classic-on |
| --- | ---: | ---: |
| `sa3_par_predicate_wall` (parallel pre-pass, 12 threads) | **0.089 s** | **2.914 s** |
| `sa3_needs_support_predicate` (unchanged serial predicate) | 0.781 s | 25.274 s |
| `sa3_par_compared` (regions compared) | 240 / 240 | 495 / 495 |
| `sa3_par_mismatch` | **absent (0)** | **absent (0)** |

**Verdict: the candidate is real and exact on the real job.** The parallel
pre-pass is 8.7x faster than the serial predicate on both cells
(0.089/0.781 and 2.914/25.274), every value agrees with the serial run, and the
G-code is byte-identical. Wall prize ≈ 22.4 s on the base supports-on stage
(38.2 s → ~15.8 s stage wall in the instrumented probe; the stage is 55–63% of
the prepass on the base cells but only ~1–2% of those cells' wall, so the cell
prize is proportionally small — the standing paired A/B decides).

Candidate source shape (candidate commit pending human keep/drop):
`commit_support_analysis_builtin` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`)
gains an indexed parallel pre-pass
(`slices.iter().flat_map(regions).filter(!empty).collect()` →
`par_iter().map(derive_needs_support).collect()`), and the serial sweep consumes
`needs_support_by_region[i]` in the same order; no other line changes. Host-only
change: no guest artifact is touched, so the A/B arms differ by `pnp_cli.exe`
alone in each mode.

## Paired A/B — benchy classic supports-on (both modes, 2026-10-09)

Protocol: [`protocol-ab.json`](protocol-ab.json), driver
[`run_ab_par_predicate.ps1`](run_ab_par_predicate.ps1), reducer
[`verify-ab.py`](verify-ab.py). Arms differ only in `pnp_cli.exe`
(ordinary baseline `d0d6e08f…` / candidate `1ea961b7…`; accelerated baseline
`c46eff52…` / candidate `1ef4da21…`); module trees byte-identical (`diff -rq`
clean) in both modes, both mode freshness checks exit 0. Proof phases passed
first in both modes (all four arms `76eb4e1d…`). Six alternating AB/BA pairs
per mode, one excluded warmup per arm, 12 threads, external-only,
uninstrumented, no probe lines.

| mode | metric (candidate − baseline) | paired median | faster pairs | range |
| --- | --- | ---: | ---: | --- |
| ordinary | wall | **−1.105 s** | **6 / 6** | [−1.251, −0.618] |
| ordinary | process CPU | **−0.828 s** | 5 / 6 | [−1.797, +0.703] |
| accelerated | wall | **−0.943 s** | 4 / 6 | [−1.028, +0.410] |
| accelerated | process CPU | **+0.047 s** | 3 / 6 | [−0.500, +1.922] |

Starvation: no pair excluded in either batch (ordinary best cpu/wall 5.132 /
5.374; accelerated 4.657 / 4.815). Every measured output byte-identical to
`76eb4e1d…` (8,658,663 bytes, `status=ok`, `degraded=false`, 0 fatal /
0 non-fatal, 0 open-loop warnings). Raw rows:
`target/t27-ab/campaign/measure/{mode}/benchy/rows.csv` (gitignored).

**Reading.** Wall improves in both modes on the decisive direction (6/6
ordinary, 4/6 accelerated, medians −1.1 / −0.9 s). Process CPU agrees in
ordinary (−0.828 s, 5/6) but is **flat-to-marginally-higher in accelerated
(+0.047 s on a 118 s base, i.e. within the per-sample spread)**. That is the
expected physical signature of a pure *serialization* fix: the same instructions
now run on the parallel axis, so wall falls while total process CPU stays put;
the small positive median is parallelism/launch overhead, not extra work. The
standing strict clause ("lower candidate median wall **and** lower candidate
median CPU") is therefore **not met in accelerated** on this cell, even though
no sample contradicts the wall direction. The decisive cell is base
supports-on, where the moved predicate is 24.2 s (27x benchy's 0.8 s) — measured
below.

## Paired A/B — base classic supports-on (both modes, 2026-10-09)

Same protocol, driver, reducer, arm binaries and mode freshness checks as the
benchy batch; proof phases passed first in both modes (all four base arms
`f60e5e9b…`). Six alternating AB/BA pairs per mode, one excluded warmup per
arm, 12 threads, external-only, uninstrumented. The ordinary batch completed
13:25; the accelerated batch was killed host-side ~2 s into candidate-m1
(14:05) and resumed under Windows Task Scheduler for its remaining 11 runs,
completing 17:55.

| mode | metric (candidate − baseline) | paired median | faster pairs | range |
| --- | --- | ---: | ---: | --- |
| ordinary | wall | **−18.198 s** | **6 / 6** | [−29.285, −13.681] |
| ordinary | process CPU | **+12.789 s** | 0 / 6 | [+5.859, +20.766] |
| accelerated | wall | **−19.380 s** | 5 / 6 | [−41.152, +26.758] |
| accelerated | process CPU | **+9.047 s** | 2 / 6 | [−4.484, +66.391] |

Starvation: no pair excluded in either batch (ordinary best cpu/wall 4.510 /
4.676; accelerated 3.927 / 3.902). Every measured output byte-identical to
`f60e5e9b…` (43,912,792 bytes, `status=ok`, `degraded=false`, 0 fatal /
0 non-fatal, 0 open-loop warnings). Raw rows:
`target/t27-ab/campaign/measure/{ordinary,accelerated}/base/rows.csv`
(gitignored); combined four-batch reduction:
[`t27-ab-reduction.json`](t27-ab-reduction.json).

**Interruption caveat (accelerated pair 1).** baseline-m1 (479.5 s wall — the
fastest baseline sample) was measured pre-kill; candidate-m1 (506.3 s) is the
first measured run after the resume, immediately after a discarded baseline
warm-up slice. Pair 1 is the batch's only wall-regression pair, and its
candidate cpu/wall ratio (3.851) is the candidate arm's lowest — still above
the starvation cutoff (0.75 × 3.902 ≈ 2.93), so the protocol retains it; no
automatic resampling. As a sensitivity check only, excluding pair 1 leaves the
wall median at −20.4 s and the CPU median at +8.25 s. The ordinary batch ran
uninterrupted.

**Reading.** Wall improves in both modes (6/6 ordinary, 5/6 accelerated;
medians −18.2 / −19.4 s, ≈3.4% / 3.7% of the arm-median wall), but process CPU
rises in both (medians +12.8 / +9.0 s, ≈0.5% of the arm-median CPU) — the same
instructions on the parallel axis plus parallelism overhead. The wall win is
real and consistent; the standing strict clause ("lower candidate median wall
**and** lower candidate median CPU") is **not met on base in either mode**.
Four-batch reduction (`reduce-ab.py` → `t27-ab-reduction.json`):
ordinary/benchy meets the keep clause; ordinary/base, accelerated/benchy, and
accelerated/base do not; all four batches clean, no starvation exclusions, all
five reduction negative controls caught in every batch. Per
`protocol-ab.json`, that returns DROP/inconclusive to the human.
**Human decision 2026-10-09: KEEP** — the base wall win (≈3.4% / 3.7% of
arm-median wall at ≈0.5% CPU overhead) was accepted as a serialization fix;
committed as `perf(support-analysis): parallel eligibility pre-pass
(ticket 27)`.

## Probe runs output identity

| run | wall | cpu | bytes | sha256 (prefix) | status |
| --- | ---: | ---: | ---: | --- | --- |
| benchy-classic-on (3a) | 31.17 s | 139.78 s | 8,658,663 | `76eb4e1d…` | ok/degraded=false/0 fatal/0 non-fatal |
| base-classic-on (3a) | 566.41 s | 2374.52 s | 43,912,792 | `f60e5e9b…` | ok/degraded=false/0 fatal/0 non-fatal |
| benchy-classic-on (3b) | 28.62 s | 141.36 s | 8,658,663 | `76eb4e1d…` | ok/degraded=false/0 fatal/0 non-fatal |
| base-classic-on (3b) | 574.61 s | 2378.20 s | 43,912,792 | `f60e5e9b…` | ok/degraded=false/0 fatal/0 non-fatal |

(Instrumented runs; walls are not acceptance figures — the probe adds ~14–21
stderr lines and the `Scope` overhead.)
