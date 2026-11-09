# t133 — internal-bridge erosion on a candidate-bbox operand

Ticket: [Internal-bridge offsets on a candidate-bbox operand](../issues/133-internal-bridge-windowed-offsets.md)

**Verdict: KEEP, and the shape is exact.** Window the **negative**
`deep_infill_area` erosion only; leave the positive `deep_infill_clip_area`
whole-layer. All eight cells reproduce ticket 123's reference G-code byte for
byte, and the measured saving clears the 0.80 % floor in all eight cells with
CPU corroborating. The positive offset cannot be windowed without moving bridge
directions: the full-window variant changed eight benchy layers by up to
22.09 mm of fill re-routing (audited below), which is why the shipping shape is
the erosion-only window.

## 1. Candidate

`internal_bridge_offset_window` (in
`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) returns the union
bounding box of the layer's `internal_solid_fill` candidates, grown by
`(2 x 4.5 + 1) x qualify_spacing_mm`. The erosion operand is
`deep_infill_area ∩ window`; the erosion itself is unchanged
(`-4.5 x spacing`, miter, limit 2.0). Both consumers of the eroded set —
`qualified.retain` and the `internal_unsupported_area` filter inside
`internal_bridge_angles` — read it only inside a candidate.

The window is a locality bound, not an approximation. `offset` uses
`miter_limit = 2.0`, so the erosion of `deep_infill_area` at a point inside the
candidate bbox depends only on source points within `2 x 4.5 x spacing` of that
point. The window boundary is a further `spacing` away, so clipping removes no
source geometry that can reach a candidate, and the covered set inside the
candidate bbox is unchanged. The clip is `intersection`, which preserves the
operand set exactly.

## 2. Why the positive offset is not windowed (the rejected formulation)

The first candidate clipped **both** `deep_infill_area` offsets to the window
(the shape t131 screened). It moved G-code on all four classic cells sampled:

| cell | reference hash | full-window hash | audited delta |
| --- | --- | --- | --- |
| benchy classic-off | `90638bf3…` | `d4078b30…` | 8 Internal Bridge layers, 992 geometry lines, max shift 22.09 mm; 0 structural geometry lines |
| base classic-off | `b3b4ac5e…` | `f351cda8…` | 5 Internal Bridge geometry lines; 0 structural geometry lines |
| benchy classic-on | `711c6334…` | `1dc1e5d9…` | bytes 8,253,678 → 8,253,729 (moved) |
| base classic-on | `4e31a2db…` | `8275a124…` | bytes equal, content moved |

The delta is confined to the bridge role, but it is not a rounding artifact:
the bridge direction itself changes. `internal_bridge_angles` builds
`limiting_area = bridge area ∪ (sparse expansion area ∩ deep_infill_clip_area)`
and samples that whole-layer boundary set for the direction. Clipping the
positive operand removes the far-away part of the sparse expansion area, so the
anchor set changes even though every candidate is local. The erosion has no such
global consumer, which is why it can be windowed exactly.

## 3. Ceiling (t131's S5 screen, erosion half)

The probe timed both whole-layer offsets and their windowed reruns. Only the
negative column is available to this shape; ticket 131's margin was
6 x spacing, this candidate's is 10 x spacing, so these are upper bounds on the
removed erosion work:

| cell | layers | erosion ran | whole (ms) | windowed (ms) | speedup | operand points (whole → windowed) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 56 | 30 | 594.9 | 62.7 | 9.48x | 157,115 → 33,320 (4.72x) |
| benchy classic-on | 56 | 30 | 572.1 | 56.7 | 10.08x | 157,115 → 33,320 (4.72x) |
| base classic-off | 130 | 42 | 9,911.4 | 1,124.4 | 8.81x | 860,721 → 97,607 (8.82x) |
| base classic-on | 130 | 42 | 9,923.8 | 1,185.9 | 8.37x | 860,721 → 97,607 (8.82x) |

The probe did not include the `deep_infill_area ∩ window` cost; the campaign
measures the real wall.

## 4. Output (16-cell contract)

The output campaign ran **8 cells x 4 arms = 32 runs** (baseline/candidate x
ordinary/accelerated) before any timing. Every cell emits **one** G-code hash
across its four arms, and every hash is ticket 123's candidate-ordinary reference
set:

| cell | SHA-256 |
| --- | --- |
| benchy classic-off | `90638bf3ea9e592d573b223c3eb9af64473efe804578d2478adfbbc5f0e67008` |
| benchy classic-on | `711c6334e397e5ae6911b15bfc71d42add9e93822cddd0a29391db6ef531d556` |
| benchy arachne-off | `2ba5d84ccf3c8deea6f39f4977dd71221faaecde9987d1699b361a5366e86380` |
| benchy arachne-on | `a972b0f2c25a988e667ecf5a5b3d6c3f5ba87f5c8872733c2a4b4e5d51a9695e` |
| base classic-off | `b3b4ac5e885c7f637b98f79eca21901a41d85739627d4dcb40820663c15e90ef` |
| base classic-on | `4e31a2db368ff80f388512211ae824b9660342a465c87f093f6182023e34eb4e` |
| base arachne-off | `6b865d4a2cb892daa3172add3e3677e0dc35192bcedd0acbc463abca7064d715` |
| base arachne-on | `62d6eedc02e5db972928f830104aab95c3598f6df6407159cb5438adb35bef13` |

The delta audit is therefore an equality proof: **zero moved lines**, structural
or fill, in all eight cells. The host binary differs between the arms
(`f3d50217…` baseline ordinary, `a439cbdb…` candidate ordinary; `65d3b7ad…`
baseline accelerated, `2fc3dfbd…` candidate accelerated) and the module tree is
shared (`7bb99a7c…` / `fbbd7484…` planner hashes unchanged within each build
arm), which is what a host-only change must look like.

## 5. Paired gate

Three interleaved pairs per cell by default, `RAYON_NUM_THREADS=12`, ordinary
arms only. Two cells were extended after the first batch because their effect is
near the floor and the pair spread exceeds it (see §7): benchy arachne-on to six
pairs, base classic-on to twelve. All recorded pairs enter the verdict; no pair
is dropped.

| cell | pairs | median wall | Δ | floor multiple | Δ CPU |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3 | 4.83 → 4.64 s | +0.19 s (+3.88 %) | **4.85x** | +0.55 s |
| benchy classic-on | 3 | 8.84 → 8.71 s | +0.13 s (+1.45 %) | **1.81x** | +0.30 s |
| benchy arachne-off | 3 | 5.01 → 4.87 s | +0.14 s (+2.84 %) | **3.55x** | +0.75 s |
| benchy arachne-on | 6 | 8.92 → 8.79 s | +0.13 s (+1.49 %) | **1.86x** | +0.93 s |
| base classic-off | 3 | 42.51 → 39.82 s | +2.69 s (+6.34 %) | **7.92x** | +7.64 s |
| base classic-on | 12 | 102.45 → 101.09 s | +1.36 s (+1.33 %) | **1.66x** | +4.48 s |
| base arachne-off | 3 | 43.82 → 39.98 s | +3.84 s (+8.76 %) | **10.94x** | +10.75 s |
| base arachne-on | 3 | 106.95 → 102.18 s | +4.77 s (+4.46 %) | **5.58x** | +8.94 s |

**8/8 PASS**, process CPU corroborating in every cell. The narrowest cells are
the supports-on classics (1.66x and 1.81x the floor); the widest are the
supports-off arachne cells (10.94x and 5.58x). A strict majority of paired runs
favours the candidate in every cell; base classic-on is the thinnest at 8/12.

## 6. Route arithmetic

This is a banked keep, not a destination step:

- base classic-off gains **2.69 s = 9.7 %** of that cell's **27.7 s** required
  prepass cut.
- base classic-on gains **1.36 s = 1.5 %** of that cell's **92.5 s** required
  prepass cut.
- benchy classic-off and -on gain **0.19 s / 0.13 s**; the benchy prepass is
  already too small for a prepass-only win, so the percentage of a required cut
  is not the useful frame there.

**Ledger correction to t131.** The S5 probe converted the erosion's thread-time
to wall at the stage's measured parallelism (5.25x / 5.26x / 8.34x / 8.78x).
Measured against the classic campaign, the conversion is **2.80x / 3.96x /
3.27x / 6.42x** (benchy-off / -on, base-off / -on): the erosion was partly
serial, so t131's wall-equivalent understated benchy classic-off by 1.7x and
base classic-off by 2.3x, and was close on base classic-on. The measured savings
here are the campaign's, not the probe's.

The shape does not close any cell, and it leaves the positive offset whole-layer.
The full-window experiment (§2) shows why an exact angle-equivalent formulation
would have to reproduce the whole-layer `expansion_area ∩ deep_infill_clip_area`
anchor set; this take did not find one.

## 7. Disclosures

- **Not an idle box.** The pre-campaign idle sample was **21.2 %** total CPU
  (DWM 18.5 %, two Chrome processes 10.8 % each, Xbox service 10.8 %, Discord
  6.2 %); a later idle sample was **10.9 %**. The box has 6 physical / 12 logical
  cores. Every cell is internally paired and the pair order alternates, but the
  background load is part of the measurement's error bar.
- **Sequential sampling on two cells.** The first batch was three pairs per
  cell. benchy arachne-on was extended to six pairs and base classic-on to
  twelve (pairs 7–12 in a later window) because the effect is near the 0.80 %
  floor and the pair spread is larger than the effect; the pre-declaration was
  "twelve total pairs for this cell, verdict on the twelve-pair median". The
  twelve-pair result is +1.36 s (+1.33 %, 1.66x floor) with 7/12 pairs positive
  and CPU positive in 12/12. No pair was discarded.
- **Windows.** The output campaign and the A/B batch are separate windows; the
  two extended-cell batches are later windows again. No measured ratio spans
  windows: each cell's pairs are consecutive and internally paired.
- **No Orca arm.** This is a PnP-internal A/B; the scoreboard of record remains
  ticket 124's. Output identity means the candidate's G-code is the same set the
  scoreboard measured, so only the PnP wall column moves, by the deltas above.
- **Multi-region jobs.** The window is per region, so exactness holds for any
  number of objects/regions; the saving shrinks when a region's candidates span
  most of its deep-infill extent.
- **Freshness.** Both guest checks exit 0; the candidate changes no guest. The
  delta audit (hash equality) was taken before any timing, per t111's contract.

Evidence: [`evidence/t133-windowed-bridge-offsets/`](.) —
`verify.py` 357 checks, 17/17 negative controls rejected; `reduce_t133.py`
reproduces `reduced/summary.json`; `rejected-full-window/` preserves the
positive-offset experiment and its audits.
