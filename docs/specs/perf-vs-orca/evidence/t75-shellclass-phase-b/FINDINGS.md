# Ticket 75 — ShellClassification phase-B serial commit

Take: 2026-10-19, first frontier ticket in order after ticket 74's closeout.
Attribution first at post-t74 HEAD; candidate measured with the standing gate.
Keep/drop is the human's.

## Question

Can `PrePass::ShellClassification`'s serial phase-B commit be reduced without
changing the classification output? The t73 base refresh measured the stage at
26.0–31.4 s on every base cell, and ticket 27's 2026-09-27 probe localized
25–30 s of it in `gate_phase_b_commit_wall`.

## Attribution at HEAD

Four fresh ordinary instrumented captures (12 rayon threads, the map's matched
configs, `--instrument-stderr`, temporary `PNP_T75_PROBE` patch) at HEAD
`fb83702c` (slicer sources = the t74 keep `091e6bc9`). Every output is
byte-identical to its preserved t58 reference. The probe brackets the gate
interior; instrumented wall is attribution only and is never a keep/drop
number.

| cell | stage | gate | phase A (parallel) | phase B (serial) | clip `+1.5s` | erosion `-4.5s` | angles | gather |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic supports-off | 40,825 | 30,689 | 3,247 | 27,429 | 12,361 | 9,060 | 2,398 | 3,123 |
| base classic supports-on | 28,083 | 20,510 | 2,134 | 18,363 | 8,383 | 6,484 | 1,352 | 1,816 |
| base arachne supports-on | 30,138 | 23,914 | 2,179 | 21,724 | 9,788 | 7,488 | 1,817 | 2,225 |
| benchy classic supports-on | 5,751 | 3,662 | 1,021 | 2,638 | 872 | 846 | 478 | 303 |

All values in milliseconds. Read plainly:

- **The two full-layer Miter offsets are 78.1 % / 81.0 % / 79.5 % of the
  serial phase-B wall on the three base cells and 65.1 % on Benchy** — ticket
  27's iteration-2 sub-split survives at HEAD (its 90 % figure was a little
  high; `gather_areas_w_depth` and the angles step account for most of the
  rest).
- The `internal_bridge_angles` input step is **2.4 s / 1.4 s / 1.8 s / 0.48 s**
  (8–18 % of phase B), not the 90 %-class term the iteration-1 note had
  suggested. Its per-candidate work is real but too small to carry the ticket.
- The input census is identical across the three base cells because this stage
  runs before the perimeter generator: 121 qualified layer entries, 7,861
  deep-infill polygons / 794,883 vertices, 204 clip polygons / 310,332
  vertices, 90 erosion polygons / 55,971 vertices, 10 angle calls.
- **Only 9 of 121 base entries (and 13 of 43 Benchy entries) carry a non-empty
  lower-candidate bridge set.** That is the only order-dependent input:
  `remove_filled_polygons_on_lower_layers` subtracts earlier layers'
  `internal_bridge_areas`. The 9 base layers changed 181 polygons / 813
  vertices of the 7,861 / 794,883 deep-area census; everything else is
  independent of the serial commit's own writes.

The t27 iteration-2 sibling-region memo remains refuted for this job: the
matched prints have one region per object, so there are no sibling regions to
share a per-layer offset between. The surviving reduction is not a memo — it is
to move the independent per-layer work into the already-parallel qualification
phase and keep only the genuinely order-dependent layers serial.

## Candidate (uncommitted)

`candidate.patch` (one runtime file):

1. Factor the exact per-layer commit into
   `compute_internal_bridge_layer` (clip offset, per-surface qualification,
   erosion filter, presort, expansion area, angles) and
   `build_bridge_depth_layers` (the depth-window clone).
2. After the qualification pass, precompute that exact result for **every**
   qualified layer in parallel. The precompute reads only committed geometry
   and the layer's own fields.
3. In the serial commit, build the lower-candidate window and check whether it
   contains any non-empty bridge polygon. If it does not,
   `remove_filled_polygons_on_lower_layers` is a no-op and the precomputed
   result is exact — publish it. If it does, recompute that one layer with the
   original serial sequence (gather → remove → commit).

Exactness argument: for a layer with no non-empty lower candidates the removed
set is empty, and `remove_filled_polygons_on_lower_layers` returns
`deep_infill_area.to_vec()`; the precomputed calls therefore use identical
inputs, identical join types, and identical argument order as the old serial
path. For the affected layers the old path runs unchanged. No geometry function
is modified; the only change is which worker performs the independent work.

Candidate probe on the same four cells (instrumented, attribution only):

| cell | baseline stage | candidate stage | reduction | output |
| --- | ---: | ---: | ---: | --- |
| base classic supports-off | 40,825 | 12,636 | −28,189 ms (−69.0 %) | `20e7204c…` ≡ reference |
| base classic supports-on | 28,083 | 12,780 | −15,303 ms (−54.5 %) | `f60e5e9b…` ≡ reference |
| base arachne supports-on | 30,138 | 13,605 | −16,533 ms (−54.9 %) | `a1bc8110…` ≡ reference |
| benchy classic supports-on | 5,751 | 1,770 | −3,981 ms (−69.2 %) | `76eb4e1d…` ≡ reference |

The candidate's process CPU on those runs is −3.2 % / −0.2 % / +0.4 % / −0.2 %
against the baseline runs (instrumented; disclosure only). The stage-wall
reduction is the internal attribution signal; the standing paired gate below is
the acceptance measurement.

## Qualification

**16/16 PASS.** Baseline and candidate snapshots were built from the same
commit (`fb83702c`) and working tree (only the candidate runtime file differs;
the 24 external modules are byte-identical between the snapshots), every cell's
baseline and candidate outputs are byte-identical to each other **and to the
preserved t58 reference** for that cell, and every run is clean
(`status=ok`, `degraded=false`, 0 fatal / 0 non-fatal) and non-vacuous. The
matrix covers both modes × both fixtures × both generators × both support
states. See `qualification.json`.

## Standing paired gate

Four pre-declared cells, 6 alternating baseline/candidate pairs each, frozen
snapshots, external `GetProcessTimes`, `RAYON_NUM_THREADS=12`. **24/24 pairs
retained** (zero starvation exclusions), both arms clean, every pair's output
byte-identical (`f60e5e9b…` base / `76eb4e1d…` benchy).

| cell | baseline wall → candidate (s) | paired wall Δ | paired CPU Δ | pairs | clauses |
| --- | ---: | ---: | ---: | ---: | --- |
| ordinary benchy classic-on | 18.675 → 18.070 | −3.08 % | +0.42 % | 6/6 | pass / pass |
| ordinary base classic-on | 350.329 → 342.163 | −3.48 % | **+1.08 %** | 6/6 | pass / **fail** |
| accelerated benchy classic-on | 17.857 → 17.829 | −3.06 % | +0.86 % | 6/6 | pass / pass |
| accelerated base classic-on | 298.094 → 288.493 | −4.11 % | +0.51 % | 6/6 | pass / pass |

**Strict pre-declared verdict: GATE NOT MET.** The wall clause passes in every
cell (−3.06 % to −4.11 % paired median), but the ordinary-base process-CPU
median is **+1.08 %** against the pre-declared **+1.0 %** ceiling — a 0.08
percentage-point miss. Read plainly: the candidate removes 8.2 s ordinary and
9.6 s accelerated of base wall and spends roughly the same total CPU; the small
CPU increase is the expected parallelization/contention overhead, not added
work. Two of the six ordinary-base pairs carry a candidate wall regression
(+7.47 %, +6.23 %) coincident with CPU jumps (+2.10 %, +2.69 %), which is the
external-load signature the map's recipe warns about; the paired median still
wins. No pair was excluded, and no post-hoc exclusion rule was applied.

Campaign provenance and recovery (disclosed per the t57 discipline): the first
campaign (2026-10-19 16:01–16:59) completed ordinary Benchy and 4/6 pairs of
ordinary base before an opencode harness eviction killed the driver. The
partial ordinary-base cell is preserved as
`measure-interrupted-20261019/ordinary-base-classic-on` and is **not** part of
this result. A detached replacement campaign (2026-10-19 20:05–22:43) re-ran
ordinary base whole and both accelerated cells; ordinary Benchy is the
completed first-campaign cell. All cells use the same frozen snapshots. No
gate-failing cell was retried.

Source: `ab-reduction.json`; driver logs `target/t75/measure-driver.log` and
`target/t75/measure-driver2.log`.

## Evidence and verification

- Raw captures (gitignored): `target/t75/t75-probe/` (baseline attribution),
  `target/t75/t75-candidate/` (candidate attribution), and the qualification /
  measurement trees under
  `.local-artifacts/perimeter-reference-preparation/t75-shellclassification-phase-b/`.
- Reduced, in-repo set: `split.json`; reducer `reduce.py`; checkout-only
  verifier `verify.py` (exit 0, four negative controls rejected).
- `probe.patch`: temporary attribution probe (reverted before the candidate
  measurement; inert without `PNP_T75_PROBE`).
- `candidate.patch`: the production candidate.
- `qualification.json`: 16-cell output-identity gate.
- `ab-reduction.json`: paired gate reduction.

Snapshot provenance: ordinary baseline `pnp_cli.exe` `b74d5e36…`, accelerated
baseline `b1758a76…`; ordinary candidate `2b991c91…`, accelerated candidate
`fff541de…`. The 24 module files are byte-identical between the baseline and
candidate snapshots in each mode (no guest source changed).

## Caveats

- Instrumented and profiled runs are attribution only. The instrumented stage
  walls inflate; the acceptance numbers are the uninstrumented process-wall
  measurements in `ab-reduction.json`.
- The base candidate probe ran in a materially faster machine state than the
  baseline probe (e.g. base classic supports-off 174 s vs 306 s process wall),
  so no wall or CPU claim is made from those runs; the stage-wall split and
  byte-identity are the attribution.
- The pre-declaration permits at most +1.0 % paired-median process CPU for this
  work-preserving parallelization (ticket 27 precedent: ~+0.5 % CPU accepted
  for a serialization fix). Any cell above that is a gate miss, not a silent
  pass. Ordinary base measured +1.08 % and is reported as a miss.
- **Snapshot provenance:** the frozen candidate snapshots were built before a
  final comment-placement cleanup (a doc-comment block moved from the new
  helper to `gate_internal_bridge_sites`). The source difference is comment
  text and line placement only; no code, signature, or call order changed.
  Rebuilding the committed source with `cargo xtask dist --edition developer`
  and slicing ordinary base classic-on produced the reference output
  `f60e5e9b…` / 43,912,792 B. The 16-cell proof and standing gate were
  measured on the snapshots; the committed source is behaviorally identical
  (the host binary hash changes only through panic-location line metadata).

## Disposition

**Human-approved KEEP (2026-10-19); source commit `07b97cad`.** The strict
pre-declared gate is reported as **NOT MET** on the ordinary-base CPU clause
(+1.08 % vs the +1.0 % ceiling); the human accepted the 0.08 pp miss on the
ticket-27 serialization precedent. The remaining phase-B serial work is the
depth gather, the angles step and the 9/121 affected-layer recompute; the
stage's Pass 1 `apply_opening` join-parity lead remains in the map's fog.

## Closeout

Standing developer distributions rebuilt at the kept HEAD after the source and
docs commits: ordinary `36533e0c…`, accelerated `728e69fe…`; both guest
freshness gates exit 0. The take-era snapshot binaries (`2b991c91…` /
`fff541de…`) differ only through the disclosed comment-placement line metadata.
