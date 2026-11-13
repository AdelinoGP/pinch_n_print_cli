# Variable-width anchor: per-segment envelope or the direct contour?

Type: grilling
Label: wayfinder:grilling
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Brim anchor reconstruction](144-brim-anchor-reconstruction.md)'s
  2026-11-12 oracle re-run: after the uniform modal-width reconstruction, the
  base Arachne supports-off cell still misses the 3 % skirt outer-loop band at
  **+3.45 %** (base classic-off now passes at +2.89 %; both benchy supports-off
  cells pass at +1.32 % / +1.34 %). The design pre-declared this case: a base
  Arachne miss prices the next shape rather than reverting the anchor.

## Question

Base Arachne's outer wall varies per point, so the t144 **uniform** form (one
modal width per object) cannot reconstruct the contour exactly. The measured
residual is the last supports-off oracle miss. Which shape takes it:

1. **Per-segment variable-width bead envelope (module-only).** Offset each
   outer-wall segment outward by its own half width and union the segment
   envelopes into the anchor footprint. It stays inside the t140 scoped
   exception (the `com.core.skirt-brim` anchor inputs), needs no contract
   change, and its cost is bounded by the same ~235 ms postpass surface. Open
   question: does the envelope produce a *fair* contour (round joins at
   segment ends, self-intersection cleanup) or a visible scallop?
2. **Direct slice contour (contract change).** Add the layer-0 slice contour
   (and the `skirt_height` layer hull contours) to the finalization world's
   reads, i.e. a `layer-collection-view` addition plus an IR schema bump and
   host plumbing. Exact by construction, but a contract change for a ~3 %
   adhesion band on one cell.
3. **Accept 3/8 and stop.** The adhesion oracle is a measure, not a revert
   gate; the route's destination arithmetic is untouched by any of these.

The answer must also price the chosen shape against the destination: the
skirt-brim surface is ~235 ms of the job, so none of these is a route step and
none carries a wall claim.

## Data the answer starts from

- Oracle (candidate arm, supports-off): benchy classic-off brim +1.79 % /
  skirt outer +1.32 %; benchy arachne-off +1.73 % / +1.34 %; base classic-off
  **+1.14 % / +2.89 %** (passes); base arachne-off **+1.86 % / +3.45 %**
  (misses by 0.45 points).
- Baseline arm for the same cells: brim +5.17 % / +6.73 % and skirt outer
  +5.65 % / +6.62 % on the two base cells — the uniform reconstruction closed
  most of a ~4-point gap.
- The supports-on cells are support-engine-limited and out of scope
  (t138/t139); their adhesion output is byte-identical before/after t144
  because the support footprint clips the entire brim area.
- The active matched-job reference set is t144's candidate set (see
  [t144](144-brim-anchor-reconstruction.md)'s answer); any later shape re-records
  it under the fairness ceremony.
