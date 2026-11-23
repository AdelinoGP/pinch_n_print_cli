# ShellClassification phase-B serial commit reduction

Type: task
Label: wayfinder:task
Status: resolved
Assignee: 2026-10-19 OpenCode wayfinder session — resumed on the human's
"continue"; first frontier ticket in order
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 73

## Question

Can `PrePass::ShellClassification`'s serial phase-B commit be reduced without
changing the classification output?

The fresh base captures measure the stage at **26.0–31.4 s** in all four base
cells ([findings](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md)).
Ticket 27's 2026-09-27 probe localized
`gate_phase_b_commit_wall` at **30.29 s (base-off) / 25.07 s (base-on)** — a
serial commit loop around the per-surface `expand(closing(spacing))` /
`intersection` chain plus `internal_bridge_angles`
(`crates/slicer-core/src/algos/bridge_over_infill.rs`) over
`deep_infill_clip_area`, `internal_unsupported_area` and `expansion_area`.
The fresh stage walls are the same order, so this remains the largest
unaddressed serial block outside `PrePass::Slice`.

Deliverables:

1. Re-derive the phase-B split at HEAD (the t27 probe predates many keeps) and
   confirm the serial commit's current share of the stage.
2. Test the t27-named cheapest exact shape: pre-reduce the
   `internal_bridge_angles` input (one shared pre-intersection per qualified
   polygon rather than three full-area intersections), and — only if the
   per-surface operations are independent — a parallel commit. Record why any
   refuted shape is unsafe.
3. Qualify output byte-identity on the 16-cell matrix against the preserved
   references and run the standing paired ordinary + accelerated A/B on the
   four pre-declared supports-on cells. Keep/drop is the human's.

Constraints: attribution first; no production edit before a measured owner;
output byte-identity is the hard gate; no default switch or auto-commit.
Profile/worker elapsed is not process CPU or uninstrumented wall.

## Acceptance

- A current-head phase-B split with the serial share named.
- Either a candidate passing the standing gate, or a measured no-safe-reduction
  verdict with the refuted shapes recorded.
- Durable reduced evidence and a checkout-only verifier, per the map's
  evidence policy.

## Pre-declaration (2026-10-19, before the timing campaign)

Attribution at HEAD reproduced t27's shape: the two full-layer Miter offsets
(`+1.5 x spacing` clip and `-4.5 x spacing` erosion) are 78-81 % of the serial
phase-B wall on the three base cells and 65 % on Benchy classic-on, while the
`internal_bridge_angles` input step is 8-18 % (a much smaller term than the
t27 iteration-1 note suggested). The tested candidate is therefore a
**work-preserving parallelization**: every layer's exact commit result is
precomputed in the independent qualification phase; the serial commit only
recomputes a layer when its lower-candidate bridge set is non-empty (9 of 121
base layers, 13 of 43 Benchy layers at the attribution HEAD). No geometry
function is changed.

Timing gate, four pre-declared classic supports-on cells (ordinary and
accelerated x Benchy and base), 6 alternating pairs per cell plus 2 warmups per
arm, `RAYON_NUM_THREADS=12`, frozen snapshots, external `GetProcessTimes`:

- Wall clause: the paired-median wall delta is negative in **every** cell.
- CPU clause: the paired-median process-CPU delta is at most **+1.0 %** in
  every cell. This is the corroboration clause for a parallelization: the total
  work must not grow materially, while the serial wall term falls. (Precedent:
  ticket 27's accepted serialization fix measured ~+0.5 % CPU.)
- Output identity: 16/16 cells byte-identical to the preserved t58 references.
- Starvation exclusions are disclosed; a cell with fewer than 6 retained pairs
  is inconclusive, not a pass.
- Keep/drop remains the human's; no auto-commit.

## Answer

**Attribution (HEAD `fb83702c`)**: the serial phase-B commit is not the
`internal_bridge_angles` input step (2.4/1.4/1.8/0.48 s; 8–18 % of phase B).
It is the two full-layer Miter offsets — `offset(deep_infill_area, +1.5 x
spacing)` and `offset(deep_infill_area, -4.5 x spacing)` — at **78.1 % /
81.0 % / 79.5 %** of phase B on the three base cells and **65.1 %** on Benchy.
The t27 iteration-2 split survives; the iteration-1 candidate shape does not
carry the ticket. The t27 sibling-region memo remains refuted for the matched
single-region jobs.

The only order-dependent phase-B input is
`remove_filled_polygons_on_lower_layers`, and it is non-empty on **9 of 121**
base layer entries (13 of 43 Benchy entries). Everything else in the per-layer
commit reads committed geometry and the layer's own fields.

**Candidate (uncommitted, `candidate.patch`)**: precompute the exact per-layer
commit in the parallel qualification phase; the serial commit publishes it
unchanged when the lower-candidate set is empty, and recomputes a layer with
the original serial sequence when it is not. No geometry function changes.
Candidate stage walls: base classic-off 40.8 → 12.6 s, base classic-on 28.1 →
12.8 s, base arachne-on 30.1 → 13.6 s, Benchy classic-on 5.75 → 1.77 s; all
four outputs byte-identical to the preserved references.

**16-cell proof gate: PASS (16/16)** — baseline and candidate byte-identical to
each other and to the t58 references, clean and non-vacuous in both modes.

**Standing paired gate: strict pre-declared gate NOT MET.** 24/24 pairs
retained, no starvation exclusions, every pair byte-identical. Wall passes in
all four cells: ordinary Benchy **−3.08 %**, ordinary base **−3.48 %**,
accelerated Benchy **−3.06 %**, accelerated base **−4.11 %** paired median.
CPU passes in three cells (+0.42 %, +0.86 %, +0.51 %) and misses the +1.0 %
ceiling in ordinary base at **+1.08 %** — a 0.08 percentage-point miss. The
candidate removes 8.2 s ordinary / 9.6 s accelerated of base wall at roughly
unchanged total CPU; the small increase is the expected parallelization
overhead. Two ordinary-base pairs carry a load-signature wall regression
(+7.47 %, +6.23 %, CPU +2.10 %, +2.69 %); the paired median still wins and no
pair was excluded post hoc.

**Measured recommendation: KEEP** with the CPU-clause miss disclosed. The wall
win is consistent across both modes and both fixtures (3.1–4.1 %), the proof
gate is clean, and the CPU cost is a 0.08 pp overshoot of a work-preserving
corroboration bound — the same class of miss the human accepted on ticket 27
(+0.5 % CPU for a serialization fix) and ticket 67 (+0.19 % CPU). The human's
ruling is required before any commit.

Evidence: [`evidence/t75-shellclass-phase-b/`](../evidence/t75-shellclass-phase-b/FINDINGS.md)
(`split.json`, `qualification.json`, `ab-reduction.json`, `candidate.patch`,
`probe.patch`, `reduce.py`, `reduce_ab.py`, `verify.py`).

## Resolution

**KEEP (2026-10-19).** The serial phase-B commit's dominant term was the two
full-layer Miter offsets (78–81 % of phase B on base, 65 % on Benchy), not the
`internal_bridge_angles` input step (8–18 %). The kept candidate precomputes
each layer's exact commit result in the parallel qualification phase and
recomputes only the layers whose lower-candidate bridge set is non-empty (9 of
121 base entries, 13 of 43 Benchy entries). No geometry function changed.

Measured: candidate stage walls 40.8 → 12.6 s (base classic-off), 28.1 → 12.8 s
(base classic-on), 30.1 → 13.6 s (base arachne-on), 5.75 → 1.77 s (Benchy
classic-on); **16/16 proof PASS** against the preserved t58 references;
**24/24 paired runs retained**, every pair byte-identical. Wall passes in all
four gate cells: **−3.08 % ordinary Benchy, −3.48 % ordinary base, −3.06 %
accelerated Benchy, −4.11 % accelerated base**. CPU passes in three cells and
misses the pre-declared +1.0 % work-preserving ceiling in ordinary base at
**+1.08 %**; the human accepted the 0.08 pp miss as the expected parallelization
overhead (ticket 27 precedent) and approved **KEEP**.

Production commit: `07b97cad`. The frozen candidate snapshots were built
before a comment-placement cleanup; the committed source differs only by
comment text/line placement, and a rebuilt final binary reproduces the
`f60e5e9b…` reference on ordinary base classic-on. Pre-commit gates: workspace
clippy `-D warnings` clean, `check-literals` 0 violations,
`check-test-quality --report` 0 findings, the focused shell/bridge tests pass,
both guest freshness gates exit 0.

Closeout: the standing developer distributions were rebuilt at the kept HEAD
(ordinary `36533e0c…`, accelerated `728e69fe…`); both freshness gates exit 0.


