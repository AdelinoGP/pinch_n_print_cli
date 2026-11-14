# Skirt hull input: fold the brim area's contour, not the bead centreline

Type: task / fairness fix + measured output campaign
Label: wayfinder:task
Status: resolved (2026-11-14; measured fairness fix — the skirt hull folds the
  brim area's outer contour, the matched job moves 6/8 cells, the brim is provably
  untouched, and the supports-off adhesion oracle reaches **4/4** (4/8 overall).
  The skirt shifts outward by the canonical half flow spacing (0.178–0.180 mm) and
  the brim→skirt step closes from 4.962–4.965 mm to **5.140–5.144 mm** against
  t146's Orca 5.139 mm. See ## Answer.)
Assignee: 2026-11-13/14 OpenCode wayfinder session — the human said "Continue the perf vs orca map"; 151 was the only unblocked, unclaimed AFK frontier ticket
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Variable-width anchor: per-segment envelope or the direct
  contour?](146-variable-width-anchor-shape.md)'s 2026-11-13 answer — the last
  supports-off oracle miss is **not** a variable-width anchor error. It
  decomposes into ~0.10 mm of brim-extent term (inside its 12 % band, shared with
  the classic cell) plus a **0.1786 mm skirt-hull input defect**:
  `SkirtBrim::generate` folds the outermost brim bead's **centreline** into the
  skirt hull, while canonical `Print::_make_skirt` folds the brim **area's outer
  contour** (`m_objectBrimAreasByInstance[...]` `area.contour.points`, appended to
  `occupied_points` in `Print.cpp`). The two differ by exactly half a flow
  spacing; the measured brim-to-skirt bbox step is **5.139 mm** on Orca against
  **5.143 mm** predicted and **4.963 mm** on PnP. The Arachne variable-width
  increment is at most ~0.04 mm.

## Question

Make the skirt hull's brim input canonical, then prove the change is
`Skirt`-only and re-record the matched job.

1. **Replace the hull input.** In `SkirtBrim::generate`
   (`modules/core-modules/skirt-brim/src/lib.rs`) the `brim_points` vector is
   extended from `path.points` of the emitted brim entities. Fold the **outer
   contour of the brim area** instead — the `outer` side of the module's
   `clip_polygons(&outer, &inner, ClipOperation::Difference)` step, which is the
   ring canonical appends. Do **not** substitute an offset of the bead centreline
   by `spacing/2`: the area contour is the canonical input and is already in
   hand.
2. **Carry the points forward, do not recompute.** If the `outer` polygons are
   out of scope where `brim_points` is built, capture their contour points in the
   same loop that emits the brim. Any re-derived offset is a second
   approximation and is not this ticket.
3. **Leave everything else untouched.** The anchor reconstruction, the
   `brim_object_gap`, the flow-spacing quantisation, the support clip, the
   `skirt_height > 1` layer hulls, and the skirt ring distance formula
   (`skirt_distance + spacing/2 + i * spacing`, which already matches canonical's
   `skirt_distance - spacing/2` plus `+= spacing` per loop) all stay as they are.
4. **Disclose, do not fix, the ~0.10 mm brim-extent residual.** It is inside the
   12 % brim band and shared with the classic cell, where the anchor is exact.
   Chasing it is a separate decision.
5. **Scope.** Read as inside t140's exception, which names "the layer-0 object
   footprint and the `skirt_height` layer hulls"; this is the skirt hull's
   layer-0 input. No `SliceIR`/WIT change, no new clipper operation.

## Pre-declared gate

- **Identity, everything but the skirt.** For all eight cells, every motion line
  outside `Skirt` must be identical with `E` stripped, and `|dE|` outside
  `Skirt` must be zero. The skirt itself shifts outward by ~0.18 mm, so all eight
  hashes move; that movement is the change, not a regression.
- **Reference set.** A fresh 16-cell set, `Skirt`-only delta audit, byte-identical
  on the ordinary and accelerated **output** arms. Until it lands, t148's set
  (`evidence/t148-print-plane-carve/reference.json`) is the currency and every
  ratio of record keeps its disclosure.
- **Oracle, four supports-off cells.** Pre-declared target **4/4**. The estimate
  from the measured 0.1786 mm radial term and the 87.244 mm outer-loop perimeter
  is base classic-off **~0.4 %** and base arachne-off **~0.9 %** against the
  3.0 % band. A miss is a result and is disclosed; it never widens the band.
- **No wall claim.** Three paired runs per cell as attribution only; medians must
  be read against a measured same-arm null band, not against zero. No scoreboard
  re-derivation: the skirt-brim surface is t103's ~235 ms module-reported
  whole-print time.
- **Route.** No destination term changes. This is a fairness fix, not a route
  step.

## Evidence the take re-derives

- `evidence/t144-brim-anchor/reduced/summary.json` now carries `growth_mm`,
  `path_length_mm` and `loop_count` per arm and feature (t146 extended
  `reduce_t144.py`; `verify.py` exits 0 against the enriched set), so the
  brim-to-skirt step is re-derivable without the gitignored raw G-code tree.
- Canonical is the pinned `v2.5.0` = `40ecc66f78`; cite `Print::_make_skirt` and
  `make_brim` by file + function, never by line number.

## Answer

**Resolved 2026-11-14 — measured fairness fix, banked; oracle 3/4 → 4/4.**
`SkirtBrim::generate` now extends `brim_points` from the **brim area's outer
contour** (`area[i].contour`, the `outer − inner [− support]` result already in
hand) and no longer from the emitted beads' `path.points`; the old fold is
deleted. Nothing else moved: the t144 anchor reconstruction, the 0.1 mm gap, the
flow-spacing quantisation, the support clip, the `skirt_height > 1` layer hulls
and the ring formula are untouched, and the host binary is unchanged.

**Output: baseline 8/8 against t148's active set; candidate moves 6/8 and is
byte-identical across the ordinary and accelerated arms in 8/8.** The fresh
16-cell set is [`evidence/t151-skirt-hull-contour/reference.json`](../evidence/t151-skirt-hull-contour/reference.json).

**The ticket's "all eight hashes move" is falsified, and the mechanism is
disclosed:** on the two base supports-on cells the first-layer support footprint
clips the whole brim area, so `area` is empty, `brim_points` stays empty and the
skirt hull has no brim input to change. The delta audit proves that inertness
better than hash equality does — **zero moved sections and zero changed `E`
lines** in both.

**Mechanism, measured:** the skirt bbox grows by **0.178–0.180 mm** on every
moving cell (half a flow spacing = 0.1786 mm), the **brim bbox is identical in
all six**, and the brim→skirt step (`max_x(Skirt) − max_x(Brim)`) closes
**4.962–4.965 → 5.140–5.144 mm** against t146's Orca **5.139 mm** and **5.143 mm**
prediction.

**Delta audit:** `Skirt` is the only moved section in the six moving cells, and
outside `Skirt` every motion line is identical with `E` stripped in 8/8. The
literal "`|dE|` outside `Skirt` is zero" clause is **falsified**: 3–35 lines per
moving cell (≤ 0.007 % of up to 508,651) carry a re-rounded `E`, max
**0.00098 mm** — the running-accumulator effect t144 measured at the same
magnitude, not moved geometry.

**Oracle (four supports-off cells):** benchy classic-off **1.32 % → 0.05 %**,
benchy arachne-off **1.34 % → 0.04 %**, base classic-off **2.89 % → 0.37 %**
(estimate ~0.4 %), base arachne-off **3.45 % (MISS) → 0.94 %** (estimate ~0.9 %).
**4/4 pass**, base Arachne-off flips, and the brim swept-area rel-diffs are
unchanged in all four cells. The residual is now the ~0.10 mm brim-extent term
plus ≤ 0.04 mm of Arachne width, exactly t146's decomposition.

**A/B:** 3 interleaved pairs × 8 cells (48 runs) as attribution only. Every
median (−4.70 … +6.43 % wall) sits inside the **measured** same-arm null band
(**±12.86 % wall / ±6.98 % CPU**, 3 cells × 3 pairs), the pair spread reaches
+49.77 % on one base Arachne-on pair, and the window's absolute walls are about
twice the output campaign's — so **no wall claim** is taken. The surface is
t103's ~235 ms.

**Drive-by, disclosed as not caused:** `integrated_parity_skirt_brim_tdd` (red
since t142) omitted the two declared keys `brim_object_gap` and
`min_skirt_length`, so the module failed closed before geometry ran. The fixture
now authors both at their registry defaults and the test passes, comparing native
against wasm finalization on the changed module.

**Falsifier added:** `skirt_hull_folds_the_brim_area_contour_not_the_bead_centreline`
(`modules/core-modules/skirt-brim/tests/skirt_brim_tdd.rs`) pins `x_max = 22.5`
against the centreline's `22.3`; it fails on the pre-change library
(`outermost skirt x=22.3 should be 22.5`) and passes on the candidate.

**Scope:** module-only; read as inside t140's exception; no `SliceIR`/WIT change,
no new clipper operation. Freshness gates: `build-guests --check` exit 0,
`build-guests --accelerated --check` exit 0. No scoreboard re-derivation, as the
ticket's gate required. Evidence:
[`evidence/t151-skirt-hull-contour/`](../evidence/t151-skirt-hull-contour/FINDINGS.md)
(`verify.py` PASS, `--self-test` 16/16 mutations rejected; `audit_delta.py`
self-checked with its own negative controls; `reduce_t151.py` regenerates
`reduced/summary.json`).
