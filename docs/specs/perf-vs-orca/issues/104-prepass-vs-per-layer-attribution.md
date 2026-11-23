# Prepass versus per-layer: what a cell's wall is made of at post-t102 HEAD

Type: task
Label: wayfinder:task
Status: resolved (2026-11-02; measurement-only — **the wall is the prepass** (68.5–89.5 % of every classic cell against 7.9–20.1 % per-layer); the park needs **77.8–93.9 % of the prepass removed**, the entire per-layer phase deleted never reaches it, and the two dominators together still leave 1.55x–3.34x. No candidate, no campaign, no production edit — see ## Answer)
Assignee: 2026-11-02 OpenCode wayfinder session — the human said "continue the
perf vs orca map" (no ticket named); the frontier held **no open child**, six
fog routes were offered, and the human selected **prepass-vs-per-layer
attribution**
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the 2026-11-02 pre-take orientation run (below), taken after
[ticket 103](103-orca-rebaseline-repaired-rig.md) made the per-cell arithmetic
the binding question

## Question

**Which phase owns each cell's wall at current HEAD, and what does the park's
57–82 % cut require of each phase?**

[ticket 103](103-orca-rebaseline-repaired-rig.md) re-baselined the Orca arm and
left Orca winning all eight cells at **3.52x–8.25x**, so every cell must lose
**57–82 %** of its wall to reach ~1.5x. Ten committed KEEPs have moved the
profile since the last whole-pipeline attributions (t73 base floor 2026-10-18,
t84 seam/overhang 2026-10-24, t88 supports-on increment 2026-10-25), and the
map's route choices have all been priced as *small* effects against a 0.80 %
floor. This ticket asks the complementary question: **is any phase large enough
to matter at all, and is the per-layer pipeline even where the wall is?**

The sharp sub-question is whether PnP's per-layer pipeline is already
Orca-competitive. If it is, then no per-layer optimization can close the gap and
the destination's arithmetic is decided by the prepass alone.

## Measured premise (orientation run, disclosed — not a campaign)

One quiet instrumented benchy classic-supports-off run at HEAD `e61c5655`
(`target/release/pnp_cli.exe` = t103's ordinary binary `4cb96942…`, output
`3672dc7e…` == the t87 reference):

| phase | ms | share of `slice_complete` 7,636 ms |
| --- | ---: | ---: |
| `PrePass::Slice` | 2,104 | 28 % |
| `PrePass::ShellClassification` | 1,999 | 26 % |
| other prepass (`OverhangAnnotation` 726, `SeamPlanning` 202, `MeshAnalysis` 166, `SupportGeometry` 39, `RegionMapping` 6, `PaintSegmentation` 6, `LayerPlanning` 1, `SupportAnalysis` 0) | 1,151 | 15 % |
| **prepass total** | **5,254** | **69 %** |
| per-layer (240 layers, all stages) | 1,485 | 19 % |
| postpass | 171 | 2 % |
| validation | 2 | 0.03 % |
| outside every phase (startup, mesh IO, teardown) | ≈724 | 9 % |

PnP's **entire per-layer pipeline is 1.49 s for 240 layers**, against Orca's
whole **2.03 s** slice for the same cell — i.e. the per-layer phase alone is
~0.73x Orca's entire job. That is the fact this ticket tests and prices; it is
one run, so nothing here is a wall claim.

## Scope

- **Cells:** the four **classic** cells (benchy/base × supports off/on). The
  Arachne generator changes the per-layer mix but not the two prepass
  dominators; the Arachne cells are left unmeasured and disclosed as such.
- **Walls:** ordinary-mode uninstrumented only, 1 warmup + 3 measured per cell,
  median claimed, `RAYON_NUM_THREADS=12`, quiet machine (load recorded before
  and after).
- **Split:** one instrumented run per cell (`--instrument-stderr`), phase and
  stage `elapsed_ms`; instrumented runs are **attribution only** and never
  denominated as wall claims (the map's standing rule).
- **Orca side:** taken from t103's committed medians, never re-measured here.

## Deliverable

1. A per-cell phase table (prepass / per-layer / postpass / validation /
   outside-phases) and the top prepass stages.
2. A **counterfactual arithmetic** table per cell: wall, Orca wall, park target
   (`1.5 × Orca`), required cut (s and %), and the resulting ratio if (a) the
   whole prepass vanished, (b) the whole per-layer phase vanished, (c) only the
   two prepass dominator stages vanished.
3. A classification of each dominator as **work-bound / representation-bound /
   engine-bound**, citing the map's committed measurements (t59 for the flat-
   bridge closing, t75 for ShellClassification phase-B, t70 for the clipper
   engine ratios) rather than re-deriving them.

## Stop rule

Measurement-only. No candidate, no campaign, no production edit. If the
arithmetic shows that no phase-level removal reaches the park in any cell, the
answer says so plainly and the destination question returns to the human — it
does **not** graduate another below-floor candidate to keep the frontier busy.

## Method

Scripts and reduced evidence under
`docs/specs/perf-vs-orca/evidence/t104-phase-split/`; `verify.py --self-test`
must exit 0 from a plain checkout.

## Answer (2026-11-02) — measurement-only; the destination is a prepass question

**The per-layer pipeline is no longer where the wall is, and no phase-level
removal short of nearly the whole prepass reaches the park.**

| cell | ratio of record | cut to park | prepass share | if prepass 0 | if per-layer 0 | if both dominators 0 | prepass fraction needed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | **3.78x** | 4.63 s (60.3 %) | **68.5 %** | 1.19x | 3.02x | 1.76x | **88.1 %** |
| benchy classic-on | **5.43x** | 8.97 s (72.4 %) | **77.0 %** | 1.25x | 4.66x | 3.02x | **93.9 %** |
| base classic-off | **4.93x** | 52.46 s (69.6 %) | **89.5 %** | 0.52x | 4.51x | 1.55x | **77.8 %** |
| base classic-on | **8.25x** | 131.63 s (81.8 %) | **89.4 %** | 0.87x | 7.60x | 3.34x | **91.5 %** |

Phase split (instrumented, attribution only; base classic-off / benchy
classic-off in brackets): prepass **68.5–89.5 %** [89.5 / 68.5], per-layer
**7.9–20.1 %** [8.7 / 20.1], postpass 0.9–3.5 %, outside phases 0.3–9.0 %. The
implied per-layer wall is **1.54 / 1.74 / 6.54 / 12.70 s**, below Orca's whole
slice (2.03 / 2.29 / 15.28 / 19.49 s) in every cell.

Dominators: `PrePass::Slice` **41.5 s = 51.2 %** of base classic-off (28.4 % /
18.1 % / 19.5 % of the other three) — the single largest measured term in the
map; `PrePass::ShellClassification` 17.4 % base-off; and on the supports-on
cells `PrePass::SupportGeometry` **40.1 %** (91.9 s) of base classic-on and
26.2 % of benchy classic-on, which with `SupportAnalysis` is ~50 % of that cell
— the `DEV-156` style-substitution site.

**Ledger corrections this take carries:**
- t73's 2026-10-18 base-off split (prepass 115.0 s / per-layer **94.3 s** of a
  214.5 s instrumented wall) is superseded: the per-layer phase is now **8.7 %**
  (7,016 ms of 80,924 ms). The change happened between t73 and t91; **no
  per-KEEP decomposition is claimed**. t91's 2026-10-27 base-on capture
  (prepass 149,723 ms of 168,319 ms slice = 89.0 %) re-measures at **89.4 %**,
  so today's split is the same instrument semantics, not a new one.
- Every remaining per-layer fog entry is bounded by that phase: e.g.
  `region_partition`'s 3x-per-layer re-run (10.04 s thread-accumulated, t60) sits
  inside a per-layer phase whose whole implied wall is 6.54 s (base-off) /
  12.70 s (base-on).
- `apply_opening`'s round-join anomaly is bounded by `PrePass::ShellClassification`
  (14.0 s base-off / 22.0 s base-on), not by the per-layer phase.

**Walls are not claimed.** All four uninstrumented sets ran under external load
(a sibling checkout's integration-test binary, then `clippy-driver` + two
browser processes); inflation against t103 is **1.80x / 1.32x / 1.69x / 1.51x**
(base-off / base-on / benchy-off / benchy-on) and load fell 95.6 % → 16.8 %
partway through the batch. Ratios of record and counterfactual denominators are
ticket 103's paired quiet medians. Shares are one instrumented pass per cell,
every pass clean and byte-identical to the t87 reference set (`3672dc7e…`,
`eb4817b4…`, `292df501…`, `22200671…`); a benchy repeat at a different load
moved no share by more than **5.92 pp**. Arachne unmeasured (the two
supports-off dominators are generator-independent by construction).
`verify.py --self-test` exits 0 checkout-only: **70/70 claims, 9/9 negative
controls detected**.

**Verdict: no candidate, no campaign, no production edit.** Reaching the park
requires removing **77.8–93.9 %** of a phase that is 69–90 % of the wall, and the
closest paired removal (base classic-off, both dominators deleted) is still
**1.55x**, 3 % short. Two shapes carry real arithmetic and both need a human
ruling before they can be ticketed: the slice closing's consumer (the ≥10 %
flat-bridge gate ticket 59 closed as representation-bound) and the support
engine's style substitution (port organic, or pin Orca to `tree_strong`). The
destination is therefore a **prepass-rewrite decision**, not another
ticket-sized candidate.

Evidence: [`evidence/t104-phase-split/`](../evidence/t104-phase-split/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 9/9 negative controls detected).

## Comments

- 2026-11-02: claimed before any work, per the map's convention; ticket created
  by the same session's charting pass (frontier was empty, route chosen by the
  human from six fog routes).
- 2026-11-02: measurement blocked for part of the session by a sibling
  checkout's integration-test binary (`D:\slicerProject\pinch_n_print_cli`,
  PID 26952, ~10 of 12 cores). The human ruled **"proceed with noise
  disclosed"**; the take accordingly rests its ratios on t103's paired medians
  and its phase shares on instrumented runs, and claims no wall.
