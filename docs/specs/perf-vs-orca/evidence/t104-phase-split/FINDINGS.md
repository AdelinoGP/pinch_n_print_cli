# t104 — Prepass versus per-layer: what a cell's wall is made of

Route: [ticket 104](../../../issues/104-prepass-vs-per-layer-attribution.md).
Measurement-only take at HEAD `e61c5655` (tree carries this take's evidence and
docs only), ordinary mode, `target/release/pnp_cli.exe` = ticket 103's ordinary
binary `4cb96942…`, `RAYON_NUM_THREADS=12`, matched-pair configs, 24 external
modules.

Four classic cells: {benchy, base} × {supports off, on}. The Arachne generator
is **not** measured here (disclosed below).

## Headline

**The prepass is 68.5–89.5 % of every classic cell's wall, the per-layer
pipeline is 7.9–20.1 %, and the park's 57–82 % cut maps onto removing
77.8–93.9 % of the prepass.** Deleting the *entire* per-layer phase never
reaches the park in any cell (3.02x / 4.51x / 4.66x / 7.60x would remain);
deleting the entire prepass would (0.52x / 0.87x / 1.19x / 1.25x). The two
biggest prepass stages together are still not enough (1.55x / 1.76x / 3.02x /
3.34x remain).

The map's in-map candidate class is therefore exhausted **by arithmetic, not by
taste**: every remaining named lead is a per-layer or a single-prepass-stage
shape, and the destination needs a prepass rewrite.

## Method, and the two regimes

- **Walls**: `run-walls.py`, uninstrumented, process creation-to-exit, 1 warmup
  + 2 measured per cell. In this take they were measured under **disclosed
  external load** (a sibling checkout's integration-test binary, then a
  `clippy-driver` and two browser processes), so they are **corroboration only**
  and enter the arithmetic as a load-inflation factor. Nothing in this document
  claims a wall.
- **Ratios and counterfactual denominators**: ticket 103's paired, quiet,
  today-vs-today medians
  ([`t103-orca-rebaseline/summary.json`](../t103-orca-rebaseline/summary.json)) —
  never re-measured here.
- **Shares**: one instrumented pass per cell (`--instrument-stderr`), reduced by
  `reduce.py`. A phase's implied wall is `share × ratio-of-record median`; the
  two regimes are never subtracted from one another.
- Every instrumented pass is clean (`degraded=false`, `non_fatal_error_count=0`)
  and byte-identical to the t87 post-t81 reference set: `3672dc7e…` /
  `eb4817b4…` / `292df501…` / `22200671…`.

### The machine, disclosed

Load was **95.6 % → 16.8 %** across the wall batch and moved again during the
instrumented passes (the same cells' instrumented walls sit +1.5 % (benchy-on),
+7.0 % (benchy-off), +9.7 % (base-off) and +43.6 % (base-on) above their t103
medians). A benchy classic-off repeat at a different load moved **no phase
share by more than 5.92 pp**, and base-classic-on's prepass share lands within
0.5 pp of ticket 91's independent 2026-10-27 capture (below) — so the shares are
the robust part of this take, which is why the arithmetic rests on them.

## Table 1 — phase split (instrumented, attribution only)

| cell | `slice_complete` | prepass | per-layer | postpass | outside phases | layers |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 8,096 ms | **5,545 (68.5 %)** | 1,626 (20.1 %) | 190 (2.3 %) | 731 (9.0 %) | 240 |
| benchy classic-on | 12,458 ms | **9,595 (77.0 %)** | 1,752 (14.1 %) | 435 (3.5 %) | 674 (5.4 %) | 240 |
| base classic-off | 80,924 ms | **72,433 (89.5 %)** | 7,016 (8.7 %) | 765 (0.9 %) | 708 (0.9 %) | 495 |
| base classic-on | 228,997 ms | **204,832 (89.4 %)** | 18,077 (7.9 %) | 5,405 (2.4 %) | 680 (0.3 %) | 495 |

"outside phases" is `slice_complete` minus the four phase brackets — startup,
mesh IO, teardown. Per-layer stage totals are thread-accumulated and are **not**
quoted as wall (base classic-off: 81,716 ms of thread-time over a 7,016 ms
phase wall, ~11.6x parallelism).

## Table 2 — prepass stages, in cell order

| stage | benchy off | benchy on | base off | base on |
| --- | ---: | ---: | ---: | ---: |
| `PrePass::Slice` | 2,301 (28.4 %) | 2,253 (18.1 %) | **41,472 (51.2 %)** | 44,558 (19.5 %) |
| `PrePass::ShellClassification` | 2,027 (25.0 %) | 1,922 (15.4 %) | 14,042 (17.4 %) | 21,969 (9.6 %) |
| `PrePass::OverhangAnnotation` | 768 (9.5 %) | 688 (5.5 %) | 9,951 (12.3 %) | 16,266 (7.1 %) |
| `PrePass::MeshAnalysis` | 171 (2.1 %) | 162 (1.3 %) | 4,924 (6.1 %) | 4,587 (2.0 %) |
| `PrePass::SeamPlanning` | 219 (2.7 %) | 198 (1.6 %) | 1,762 (2.2 %) | 3,224 (1.4 %) |
| `PrePass::SupportGeometry` | 43 (0.5 %) | **3,260 (26.2 %)** | 231 (0.3 %) | **91,902 (40.1 %)** |
| `PrePass::SupportAnalysis` | 0 | 1,096 (8.8 %) | 0 | 22,254 (9.7 %) |

The stages that implement the two dominators are `SLICE_PRODUCER`
(`host:slice`, `crates/slicer-runtime/src/builtins/prepass_slice_producer.rs`)
and `SHELL_CLASSIFICATION_PRODUCER` in the same file; the closing inside the
slice stage is `polygon_ops::closing_ex`
(`crates/slicer-runtime/src/profiling_report.rs` documents its benchy cost).

## Table 3 — the counterfactual arithmetic

`ratio_if_X_zero` = `(1 − share(X)) × ratio_of_record`. Park = **1.5x**.

| cell | ratio of record | cut to park | prepass share | if prepass 0 | if per-layer 0 | if both dominators 0 | prepass fraction needed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3.78x | 4.63 s (60.3 %) | 68.5 % | **1.19x** | 3.02x | 1.76x | 88.1 % |
| benchy classic-on | 5.43x | 8.97 s (72.4 %) | 77.0 % | **1.25x** | 4.66x | 3.02x | 93.9 % |
| base classic-off | 4.93x | 52.46 s (69.6 %) | 89.5 % | **0.52x** | 4.51x | 1.55x | 77.8 % |
| base classic-on | 8.25x | 131.63 s (81.8 %) | 89.4 % | **0.87x** | 7.60x | 3.34x | 91.5 % |

Implied walls (share × ratio-of-record median), for reading the phases against
Orca's whole slice (2.03 / 2.29 / 15.28 / 19.49 s): prepass **5.26 / 9.55 /
67.47 / 143.88 s**, per-layer **1.54 / 1.74 / 6.54 / 12.70 s**, postpass
0.18 / 0.43 / 0.71 / 3.80 s.

## Findings

1. **The per-layer pipeline is no longer where the wall is.** It is 7.9–20.1 %
   of every classic cell, and in all four the *whole* per-layer pipeline is
   implied to cost less than Orca's entire slice. Deleting 100 % of it leaves
   3.02x–7.60x — i.e. 2.0x–5.1x the 1.5x park threshold.
2. **The prepass is the only phase large enough to reach the park** — and only
   by removing **77.8–93.9 %** of it. This is the arithmetic that kills the
   in-map candidate class: no representation-safe reduction of one or two
   prepass stages can supply it.
3. **The closest any paired removal comes is base classic-off at 1.55x**
   (`PrePass::Slice` + `PrePass::ShellClassification` both removed entirely,
   51.2 % + 17.4 % of the cell) — **3 % short of the park**, and that is a
   counterfactual with the two stages deleted, not reduced.
4. **`PrePass::Slice` is now the single largest measured term in the map**:
   41,472 ms = **51.2 %** of the base classic-off cell, and 28.4 % / 18.1 % /
   19.5 % of the other three. Ticket 59 closed its representation-safe
   reduction (the closing is consumed by a per-component ≥10 % flat-bridge
   gate, and local closings flip that gate on 38.5 % of benchy and 68.2 % of
   base call layers), so this term is closed *for this ticket's purposes*: it is
   the thing the destination needs and the map has no safe shape for it.
5. **On the supports-on cells the dominant prepass stage is support geometry,
   not slicing.** `PrePass::SupportGeometry` is **40.1 %** of base classic-on
   (91,902 ms) and **26.2 %** of benchy classic-on; with
   `PrePass::SupportAnalysis` the support engine is ~50 % of the base
   classic-on cell. That is the site of the style substitution `DEV-156` names
   (PnP runs the classic engine's **Strong** where canonical resolves
   **organic**), so the supports-on fairness question is now positioned as the
   largest single lever on the two worst cells.
6. **The per-layer phase collapsed since the map last described it.** Ticket 73
   (2026-10-18) recorded base classic-off instrumented wall **214.5 s** with
   prepass 115.0 s and per-layer **94.3 s** (44 %); ticket 91 (2026-10-27)
   recorded base classic-on prepass **149,723 ms of 168,319 ms slice** (89.0 %,
   the share this take re-measures at 89.4 %). So the 13x fall in per-layer wall
   happened between those two vintages — the window in which the
   consume-only-when-read and `resolution` keeps landed. **No per-KEEP
   decomposition is claimed**; the point is that the map's per-layer leads were
   harvested by those keeps, and today's residual is small enough that
   per-layer work cannot move the destination.
7. **`region_partition`'s 3x-per-layer dedup lead is bounded by a 6.5–12.7 s
   phase** on the base cells (implied wall 6.54 s of 75.38 s and 12.70 s of
   160.86 s). Whatever fraction of it is removable, the phase cannot supply the
   park. The same bound applies to every other per-layer fog entry.

## What this means for the map

The destination ("beat Orca in all 8 cells") is now an **explicit prepass
problem**: remove 78–94 % of a phase that is 69–90 % of the wall, in a code path
whose largest term (the slice closing) has a closed representation-safety
verdict. That is not a ticket-sized change; it is a program-level decision, and
this ticket's stop rule leaves it with the human rather than graduating another
below-floor candidate.

Two shapes with real arithmetic survive and are named for the map's fog:

- **`PrePass::Slice` (51.2 % of base classic-off)** — needs a change to *what
  the closing feeds* (the ≥10 % flat-bridge gate), which is canonical-parity
  work and outside this map absent a t81-style exception.
- **`PrePass::SupportGeometry` (40.1 % of base classic-on)** — needs the
  engine-style question settled first (port organic, or pin Orca to
  `tree_strong`), which the map already carries as a human decision.

## Disclosures

- Walls are **not claimed**: all four uninstrumented sets ran under external
  load, with inflation **1.80x / 1.32x / 1.69x / 1.51x** (base-off / base-on /
  benchy-off / benchy-on) against ticket 103's medians. The load fell from
  95.6 % to 16.8 % partway through the batch, which is visible in the retained
  rows (`base-classic-on` 237.5 s then 187.2 s).
- Instrumented walls are attribution only and are never denominated as wall
  claims: base classic-on's instrumented pass ran at **+43.6 %** over its t103
  median. Its *share* is corroborated independently by ticket 91's capture
  (89.0 % vs 89.4 %) and by base classic-off's same-day 89.5 %.
- The Arachne generator is unmeasured here. The two supports-off dominators are
  generator-independent by construction (`PrePass::Slice` and
  `ShellClassification` run before wall generation); the Arachne cells' phase
  splits are unknown.
- The benchy classic-off repeat is a share-stability check, not a second
  measurement: max phase-share delta **5.92 pp** between a 8.21 s and a 13.23 s
  pass.
- No candidate, no campaign, no production edit; `verify.py --self-test` exits 0
  from a plain checkout with **9/9** negative controls detected.

## Reproduction

`REPRODUCE.md` in this directory. Committed reduced set: `phase-split.json`,
`counterfactuals.json`, `walls.csv`, `load.txt`. Raw event streams and G-code
live in `target/t104-instrumented/` (gitignored).
