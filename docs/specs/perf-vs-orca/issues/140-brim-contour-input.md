# Brim anchor: feed the slice contour (or reconstruct it) into `skirt-brim`?

Type: grilling
Label: wayfinder:grilling
Status: open (unclaimed; human's decision)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Layer-plane parity: sample at the canonical midpoint](139-layer-plane-parity.md)'s
  2026-11-11 oracle re-run. With the slice plane at the canonical midpoint, the
  base supports-off cells still fail the 3 % skirt band — now as an **undershoot**
  (swept −4.3 % / −4.4 %, outer loop −5.7 % / −6.6 %; the brim itself moved from
  +8.1 % to −5.2 % and passes its 12 % band).

## Question

`com.core.skirt-brim` anchors `make_brim` on the emitted **outer-wall path**,
which sits half an outer-wall width (0.20 mm PnP / 0.21 mm Orca) inside the
slice contour that canonical `make_brim` anchors on. At the old top-plane
sampling that inward anchor partly cancelled the top-plane contour overshoot;
with the canonical plane it is the whole residual, and it now makes PnP's brim
and skirt slightly **smaller** than Orca's instead of larger.

Should the module consume the slice contour directly (a `SliceIR` contour, or a
reconstruction by an outward half-wall offset of the outer-wall path), and is
that licensed by the t130 close-asymmetries ruling — or does it need a t81-style
per-key exception?

Considerations on file:

- The representation gap: layer IR carries beads, not contours; the t135 Stage-1
  oracle measured the anchor term at −27.65 mm of the +88.04 mm contour term,
  with a 5.0 % non-circular residual (t138 §2).
- The residual is small and inside the brim band; only the skirt's 3 % band
  fails, on two cells. A t130 close-asymmetry reading would license it; t138's
  answer said it is "canonical-fidelity work, not a matched-settings asymmetry"
  and "stays out of scope unless the human grants a t81-style exception". The
  two rulings need reconciling.
- If licensed: the reconstruction must be exact enough not to reintroduce a
  plane-dependent residual; a direct contour would need a SliceIR field (a
  schema addition) or a module-side offset whose accuracy is measurable against
  the t139 oracle in one campaign.
