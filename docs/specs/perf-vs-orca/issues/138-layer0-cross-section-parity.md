# Layer-0 cross-section parity: do the two arms print the same first layer?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Staged fold: canonical skirt-brim re-take](135-cross-layer-occupancy-batching.md)'s
  2026-11-10 Stage-1 oracle residual.

## Question

The t135 Stage-1 parity oracle cannot reach 8/8 because the two arms' **layer-0
geometry differs upstream of `com.core.skirt-brim`**. On the matched job the
measured differences are:

- **Object footprint.** On `base.stl` the two slicers emit materially different
  first-layer outer-wall loops. Measured on the same cell: PnP outer-wall loop
  length 9.728 mm enclosing 3.122 mm² against Orca's 5.566 mm / 0.914 mm²; the
  brim's innermost loop encloses 6.18 mm² on PnP against 4.59 mm² on Orca. The
  brim anchors on the slice contour, which the g-code does not carry, so any
  g-code-level brim comparison inherits a ~0.2 mm anchor bias — measured as
  +8.1 % / +7.3 % brim area on the two base supports-off cells.
- **Support hull.** `_make_skirt` folds the support footprint into the skirt
  hull. PnP's tree supports are not Orca's `tree_strong`, so the skirt's outer
  loop differs by 12.5 % on both base supports-on cells even where both arms
  agree the brim is entirely clipped away (0 / 0 loops).
- **Feature composition.** Orca fragments the benchy supports-on brim into 39
  runs around its support pads against PnP's 19, so its `; FEATURE: Brim`
  section is 3× the area; the object brim cannot be separated from support-area
  brims at g-code level.

The question: **which of these are slicing differences (Z convention, contour
offset, first-layer height, wall-width resolution), which are support-engine
differences, and which are cosmetic feature-splitting** — and can the layer-0
cross-section be brought to parity (or shown equal modulo a named transform)?

Deliverable: a Phase-1 attribution that classifies each measured difference,
states whether the adhesion oracle can ever reach 8/8 without a slicing change,
and — if a slicing change is required — prices it against the destination.
