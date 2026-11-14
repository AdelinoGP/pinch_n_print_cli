# Skirt hull input: fold the brim area's contour, not the bead centreline

Type: task / fairness fix + measured output campaign
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
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
