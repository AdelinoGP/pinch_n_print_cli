# t144 — Brim anchor reconstruction: the contour is reconstructed, 6/8 cells move, and the adhesion oracle reaches 3/8

Route: [Brim anchor reconstruction](../../issues/144-brim-anchor-reconstruction.md).
Predecessor: [Layer-plane parity](../../issues/139-layer-plane-parity.md).
Licence: the fourth scoped parity exception ([t140](../../issues/140-brim-contour-input.md) ruling).

**Headline: `SkirtBrim::generate` now anchors on a reconstructed slice contour
instead of the outer-wall centerline.** The anchor is the filled wall footprint
grown by half the resolved outer-wall width (the per-object modal non-zero
`Point3WithWidth.width`, falling back to `line_width`), and the same
reconstruction is applied to each `skirt_height > 1` layer hull. The change is
module-only (`com.core.skirt-brim`), touches no contract, and leaves the support
clip untouched. The baseline arm reproduces the pre-t144 reference set in
**8/8** cells; the candidate records a fresh set that is byte-identical on the
ordinary and accelerated arms in **8/8**, and moves in **6/8** — the two base
supports-on cells are byte-identical because their support footprint clips the
whole brim area (measured, disclosed). The delta audit passes **8/8**: no motion
outside `Skirt`/`Brim` moves with `E` stripped, and the largest **|dE|** is
**0.00098 mm**. The adhesion oracle reaches **3/4 supports-off** cells
(**3/8** overall; supports-on stays support-engine-limited): base classic-off
moves the skirt-outer magnitude from **5.65 %** to **+2.89 %** and passes, while
base Arachne-off remains **+3.45 %**, 0.45 points outside its 3 % band — the
case the ticket pre-declared as the next shape's entry, graduated as
[t146](../../issues/146-variable-width-anchor-shape.md).

**Correction (2026-11-13, [t146](../../issues/146-variable-width-anchor-shape.md)).**
The oracle's relative difference is `abs(pnp - orca) / orca`
(`oracle_skirt_brim.py::rel`) and carries no direction. **Every supports-off cell
is an undershoot** — PnP prints a smaller brim and skirt than Orca — before and
after this change: base classic-off is 586.47 -> 603.64 mm² against a fixed
621.60 mm². So "−5.65 % → +2.89 %" is a magnitude history, never a direction
flip; the same correction applies to the ticket's Answer §6 and to its table row
below.

## 1. What changed

`SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`) previously
built `footprints` from `entity.path.points` — the **centerline** of the
`ExtrusionRole::OuterWall` loop — and passed that ring straight to the
`make_brim` pipeline. Canonical `make_brim` anchors on the **slice contour**,
half an outer-wall width outside that centerline. The reconstruction:

1. Collects every outer-wall point width per object and takes the **modal
   non-zero** width (ties broken by the first point; every-zero falls back to
   the module's `line_width`). `Point3WithWidth.overhang_distance_mm`'s
   `boundary_offset = 0.5 x width` is the repo's existing half-width
   convention.
2. Offsets the **filled** footprint (`outer_only` semantics preserved) outward
   by `0.5 x width` with `OffsetJoinType::Round` and
   `BRIM_ARC_TOLERANCE_MM` (0.0125 mm), then feeds that contour to the existing
   gap/quantisation/`tryExPolygonOffset` steps unchanged.
3. For `skirt_height > 1`, computes each higher layer's convex hull from its
   printed points, offsets it outward by half that layer's modal outer-wall
   width, and folds the offset hull's vertices into the final skirt hull.
   Convex hull commutes with the Minkowski sum, so this is the hull of the
   reconstructed contours. Layer 0 needs no separate offset: the brim footprint
   dominates it and is already folded in. The matched job authors
   `skirt_height = 1`, so **this path is unit-tested and disclosed as not
   exercised**.

`geometry::modal_non_zero_width` is the new helper. The support clip, the
0.1 mm `brim_object_gap`, the flow-spacing quantisation and the support-contour
comment are untouched. No `SliceIR`/WIT change.

Unit coverage (`modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`, 18/18
green):
- `brim_anchor_is_the_contour_not_the_wall_centerline` — a 1.0 mm square loop
  whose points carry 0.8 mm width (wider than the 0.4 mm config line width)
  must put the innermost brim line at `contour (1.4 mm) + gap (0.1) +
  spacing/2`, and the bead's inner edge must stay outside the part outline. A
  module that read `line_width` lands 0.2 mm too far in and fails.
- `skirt_hull_reconstructs_the_contour_on_higher_layers` — `skirt_height = 2`
  with a wider layer 1 pins the half-width offset of the higher hull.
- `brim_quantises_on_the_canonical_flow_spacing` now asserts the outer brim
  boundary against the reconstructed contour (1.2 mm half extent), not the
  1.0 mm centerline.

## 2. Output campaign: 24 runs, 8 cells x 3 arms

One run per (cell, arm); the two ordinary arms share one byte-identical host
binary (`exe_sha256 5e902c04…`) and differ only in the `skirt-brim` guest
(`92e0d3ed…` baseline vs `74ada739…` candidate ordinary vs `95f0cf25…`
candidate accelerated). The accelerated arm is the ordinary host plus the
accelerated **component** tree; the accelerated host binary is not buildable on
this box (the known `libmimalloc-sys` verbatim-path failure with MSVC 14.51,
t142), disclosed rather than hidden.

| cell | baseline | candidate ordinary | candidate accelerated |
| --- | --- | --- | --- |
| benchy classic-off | `a89e449b…` ✅ ref | `1ae0cfa8…` | `1ae0cfa8…` |
| benchy classic-on | `dceb7a48…` ✅ ref | `d9fc80ba…` | `d9fc80ba…` |
| benchy arachne-off | `3286449f…` ✅ ref | `8faf02b1…` | `8faf02b1…` |
| benchy arachne-on | `be786718…` ✅ ref | `cd7d4a2d…` | `cd7d4a2d…` |
| base classic-off | `ca14d055…` ✅ ref | `c0b9014f…` | `c0b9014f…` |
| **base classic-on** | `585c6a8e…` ✅ ref | `585c6a8e…` (inert) | `585c6a8e…` (inert) |
| base arachne-off | `d8fdc1d9…` ✅ ref | `17f63e93…` | `17f63e93…` |
| **base arachne-on** | `c9fee59a…` ✅ ref | `c9fee59a…` (inert) | `c9fee59a…` (inert) |

The two inert cells are the measured limit of the ticket's pre-declared "the set
moves in all eight cells": base's first-layer support footprint covers the whole
brim annulus, so no brim path is emitted and no brim point reaches the skirt
hull. The support clip is the out-of-scope support engine (t138/t139), and the
same cells' delta audit records **zero** changed `E` lines. The anchor is still
correct — it is occluded, not missing. Full hashes:
[`reference.json`](reference.json).

## 3. Delta audit

`audit_delta.py` over the 8 baseline/candidate pairs: **pass 8/8**. Every
motion line outside `;TYPE:Skirt` / `;TYPE:Brim` is identical with the `E` field
stripped, in every cell. Inside the adhesion sections the rings move outward by
the half width; the `E` accumulator re-rounds later moves by at most
**0.00098 mm** (base classic-off; tolerance 0.001). Changed `E` lines per cell:
benchy 110 / 23 / 76 / 44, base 170 / 0 / 219 / 0 — the two zeros are the inert
supports-on cells. Raw audit: `target/t144/audit.json` (gitignored).

## 4. Paired A/B (attribution only)

A 3-pair interleaved baseline-vs-candidate campaign over all eight ordinary
cells, plus a six-cell same-arm null control that measures the instrument's own
paired noise. The ticket takes **no wall claim**: the skirt-brim surface is
~235 ms of the job (t103) and the route arithmetic is unchanged.

| cell | median wall Δ (candidate − baseline) | median CPU Δ | null band, wall / CPU |
| --- | ---: | ---: | --- |
| benchy classic-off | +1.84 % | +1.20 % | ±8.90 / ±3.34 |
| benchy classic-on | −2.86 % | −0.72 % | ±0.87 / ±4.52 |
| benchy arachne-off | −0.55 % | −0.08 % | ±24.82 / ±2.50 |
| benchy arachne-on | +0.43 % | +0.04 % | (not in null set) |
| base classic-off | +1.32 % | 0.00 % | ±9.51 / ±0.49 |
| base classic-on | +1.63 % | +0.70 % | ±7.77 / ±1.08 |
| base arachne-off | +1.18 % | +0.27 % | ±4.52 / ±0.62 |
| base arachne-on | −5.58 % | −0.03 % | (not in null set) |

Three pairs per cell, baseline/candidate order alternating by pair parity. Every
median is inside the same-arm null control's measured band (worst wall
**±24.82 %**, worst CPU **±4.52 %**), so the A/B reads as **no wall claim** —
the six-cell null band, not an assumed constant, bounds the instrument. The
null band is wide on the short cells (benchy arachne-off, 3–5 s runs) and the
box was shared; no effect is claimed from any sign. Raw rows:
`target/t144/ab/rows.csv`, `target/t144/null/rows.csv` (gitignored).

## 5. Adhesion oracle: 3/4 supports-off, 3/8 overall

The t135/t139 oracle against a fresh Orca arm (`-Runs 1`, same matched configs;
the previous Orca reference tree was not in `target/`). Supports-off, candidate
vs Orca, brim (12 %) swept and skirt (3 %) outer-loop bands, swept also
reported:

| cell | brim swept | skirt swept | skirt outer | verdict |
| --- | ---: | ---: | ---: | --- |
| benchy classic-off | +1.79 % | +0.68 % | +1.32 % | PASS |
| benchy arachne-off | +1.73 % | +0.68 % | +1.34 % | PASS |
| base classic-off | +1.14 % | +2.19 % | +2.89 % | PASS |
| base arachne-off | +1.86 % | +2.62 % | **+3.45 %** | MISS (3 %) |

Baseline arm for the same cells, same Orca reference: benchy classic-off
+2.90 / +1.54 / +2.85 (pass), benchy arachne-off +2.96 / +1.60 / +2.86 (pass),
base classic-off +5.17 / +4.28 / **+5.65 (fail)**, base arachne-off +6.73 /
+4.41 / **+6.62 (fail)**. The uniform reconstruction moves the two base
supports-off cells by ~3.5–4 points of skirt outer-loop area and flips
base classic-off; base Arachne-off lands 0.45 points outside the band. The four
supports-on cells are the support engine and are out of scope (t138/t139); their
adhesion output is byte-identical in this take.

**The next shape is a decision, not a revert:** [t146](../../issues/146-variable-width-anchor-shape.md)
prices the per-segment variable-width bead envelope (module-only, inside the
same exception) against the direct slice contour (contract change) against
accepting 3/8. The stop rule is t139's: geometry correctness plus a bounded
audit; the oracle is a measure.

## 6. Freshness and gates

- `cargo test -p skirt-brim` — **23/23** across the module's three targets
  (`skirt_brim_tdd` 18, `finalization_live_tdd` 4,
  `slicer_module_binding_tdd` 1).
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- `cargo xtask check-literals` — 0 violations; `cargo xtask check-test-quality
  --report` — 0 findings.
- `cargo xtask build-guests --check` — exit 0 before and after; the only guest
  rebuilt is `skirt-brim-guest` (ordinary), disclosed.
- `cargo xtask build-guests --accelerated --check` — exit 0 before and after;
  `skirt-brim-guest` and `tree-support-planner-guest` rebuilt.
- Pre-existing red confirmed at HEAD, not caused:
  `integrated_parity_skirt_brim_tdd::integrated_parity_skirt_brim` fails on the
  missing `brim_object_gap` config key before any geometry runs (the ticket's
  recorded known red). `layer_slice_tdd`'s z = 0.1 layer is untouched.

## 7. Disposition

Fairness fix under the fourth recorded exception — not a route step: no wall
claim, ~235 ms of surface, destination arithmetic unchanged (the last scoreboard
remains t139's **2.279x–4.187x ordinary / 2.311x–4.179x accelerated, required
cut 56.1 %–76.1 %**), and no scoreboard re-derivation was taken. The new active
reference set is `reference.json` here; later output-checking tickets compare
against it.
