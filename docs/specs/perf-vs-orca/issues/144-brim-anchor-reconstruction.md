# Brim anchor reconstruction: feed `make_brim` the slice contour from the wall path

Type: task / fairness fix + measured output campaign
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Brim anchor: feed the slice contour (or reconstruct it) into `skirt-brim`?](140-brim-contour-input.md)'s
  2026-11-12 human ruling: the gap is real printed geometry and gets its own
  **scoped t81-style exception** (the fourth recorded parity exception), the
  anchor is reconstructed **module-side** from the wall path plus half its
  width, and the work runs under the fairness ceremony, not a KEEP/DROP gate.
Licence: [Out of scope, fourth exception](../map.md) — scoped to the
  `com.core.skirt-brim` anchor inputs only (the layer-0 object footprint and the
  `skirt_height` layer hulls). It is not a licence for other postpass parity
  work, and it does not change the *Asymmetry* glossary definition.

## Question

`SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`) builds the
brim anchor from the `ExtrusionRole::OuterWall` **centerline**
(`entity.path.points`), converts each closed loop with
`geometry::closed_loop_to_ex_polygon`, and unions the loops per object into
`footprints`. Canonical `make_brim` anchors on the **slice contour**, which is
half an outer-wall width outside that centerline (PnP 0.20 mm, Orca 0.21 mm at
the matched first layer). The same representation gap applies to the skirt's
`skirt_height > 1` layer hulls, which currently take wall centerline points.

Reconstruct the contour from the wall path — offset the filled footprint
outward by `0.5 × the resolved outer-wall width` — before the existing
`make_brim` pipeline, and do the same for the multi-layer skirt hull inputs.
The measured cost of the gap, re-derived by
[t139](139-layer-plane-parity.md): the brim itself is −5.2 % (inside its 12 %
band) and the base supports-off skirt is an undershoot of −4.3 / −4.4 % swept
and −5.7 / −6.6 % outer loop against the 3 % band; the two benchy supports-off
cells pass; the four supports-on cells are support-engine-limited and stay
[out of scope](../map.md). The gap also makes the innermost brim bead centered
on the part outline instead of its inner edge touching it — a real overlap
defect on every cell, not only an oracle percentage.

## Design (settled by the t140 ruling)

1. **Anchor width.** Read the outer-wall entity's own point widths
   (`Point3WithWidth.width`); for the uniform form use the loop's **modal
   non-zero width** (ties broken by the first point) and record the choice,
   falling back to `self.line_width` only when every width is zero. Do not
   hard-code 0.4: the repo already encodes this half-width convention in
   `Point3WithWidth.overhang_distance_mm`'s `boundary_offset = 0.5 × width`.
   The uniform form is an approximation on Arachne, whose outer-wall width
   varies per point; the oracle measures the approximation rather than assuming
   it is exact.
2. **Footprint.** Offset the **filled** footprint (`outer_only` already drops
   holes) outward by `0.5 × width` with `OffsetJoinType::Round` and
   `BRIM_ARC_TOLERANCE_MM` — the same tolerance the existing brim offsets use,
   and the same pure-distance model the t138 oracle used. Keep the existing
   `brim_object_gap` (0.1 mm), the quantisation and `tryExPolygonOffset`
   untouched; only the anchor moves.
3. **Skirt hulls.** For `skirt_height > 1`, compute the non-brim hull of each
   layer above layer 0 and offset it outward by half the outer-wall width before
   it joins the final hull (convex hull commutes with the Minkowski sum, so the
   hull of the offset contours is the offset of the hull). Layer 0 is dominated
   by the brim footprint and needs no separate offset. This path is **not
   exercised by the matched job** (`skirt_height = 1`): unit-test it and
   disclose it as such.
4. **Leave the support clip alone.** `support_area`'s closing over support beads
   is the documented substitute for the support slice contour; the support
   engine is out of scope. Do not touch it, and keep the comment naming the gap.
5. **No contract change.** The finalization world exposes only
   `print-entity-view`; a direct contour would need a `layer-collection-view`
   addition, an IR schema bump and host plumbing. That shape is deliberately not
   taken. If a base Arachne cell still misses the 3 % band because its outer
   wall varies per point, this ticket's answer prices the next shape (a
   per-segment variable-width bead envelope, module-only, or the direct contour,
   a contract change) as its own decision; it does not revert the anchor.

## Pre-declared rules

- **Fairness fix, not a route step.** No wall claim and no KEEP/DROP gate. The
  skirt-brim surface is ~235 ms of the job (t103); the destination's required
  cut is unchanged. The oracle's ceiling from this fix alone is **4/8**.
- **Landing gate (t139 precedent).** Geometry correctness — the anchor
  reconstructs the contour on every layer and every object, the brim's inner
  boundary is outside the part outline, and the innermost bead no longer
  overlaps the footprint — plus a bounded output audit. The oracle result is
  the acceptance **measure**, not a pass/fail gate: a band miss is disclosed
  and graduates the next shape; the candidate is not reverted for it.
- **Output contract.** Baseline reproduces t139's reference set
  (`93e21fee…` / `b3f1eccc…` / `3510e876…` / `96cdac85…` / `ca14d055…` /
  `585c6a8e…` / `d8fdc1d9…` / `c9fee59a…`) exactly before the change; the
  candidate records a fresh 16-cell set (8 cells × ordinary/accelerated),
  identical within each arm. The set moves in all eight cells: the brim exists
  on every cell, and the supports-on brim is additionally clipped by support
  geometry.
- **Delta audit.** With the `E` field stripped, motion outside
  `;TYPE:Skirt` / `;TYPE:Brim` must be identical; inside them the expected
  movement is the 0.2 mm outward shift (the t138 decomposition's −27.65 mm
  anchor term on the base brim path is the cross-check, measured not assumed).
  State the measured `|dE|` bound; do not pre-declare a fabricated number.
- **No scoreboard re-derivation.** The 8-cells × 3-tools scoreboard is not
  re-taken; t139's ratios stand with the disclosure that the adhesion geometry
  moved again and no ratio of record moves. A 3-pair interleaved wall/CPU A/B is
  taken as attribution only, and the box's null band is measured rather than
  assumed.
- **Freshness.** Both `cargo xtask build-guests --check` and
  `cargo xtask build-guests --accelerated --check` (when the accelerated arm is
  used) exit 0 before and after; any guest artifact rebuilt for the campaign is
  disclosed.

## Acceptance

- The anchor is the reconstructed contour on every layer and every object; a
  unit test pins the half-width reconstruction (a known square loop with a known
  width reconstructs the expected larger contour, and the brim's inner boundary
  lands outside the part outline).
- A multi-layer skirt-hull unit test (`skirt_height > 1`) pins the contour input
  and is disclosed as not exercised by the matched job.
- The fresh 16-cell reference set is recorded in this ticket's closure, with the
  baseline reproducing t139's set first.
- The delta audit is recorded (feature-scoped movement, measured `|dE|`).
- The t135/t139 adhesion oracle is re-run on the four supports-off cells, with
  brim (12 %) and skirt (3 %) bands reported swept and outer-loop, and the
  supports-on cells reported as support-engine-limited.
- The 3-pair A/B and its null band are recorded; no wall claim is taken.
- The active reference set is named; t143 (or any later output-checking ticket)
  is pointed at it if it runs after this change.
- Both freshness gates exit 0; no unrelated production edit rides along.
- A recorded answer with the achieved oracle state, the achieved anchor geometry
  and the next shape (if a band still misses).

## Build forks and known reds (re-derive, do not quote)

- The per-layer boundary key and the visual-debug tap recorded by
  [t142](142-overhang-band-partition-deletion.md) are unrelated; do not touch
  them.
- `integrated_parity_skirt_brim_tdd`'s missing `brim_object_gap` red is
  pre-existing (confirmed at HEAD in the t142 take); confirm it again at this
  ticket's HEAD before using that test as a gate.
- `layer_slice_tdd`'s z = 0.1 layer under t139's midpoint plane is pre-existing;
  not caused by this change.
- The accelerated **timing** arm may still fail in the controlled driver
  (`libmimalloc-sys` verbatim path with the `RUSTC` wrapper + MSVC 14.51, t142);
  accelerated **output** parity can be checked with the ordinary host plus the
  accelerated guests, disclosed, if the timing arm is unavailable. The
  `--accelerated --check` freshness gate still applies.

## Route arithmetic and disposition

This is **not** a route step: no wall claim, ~235 ms of surface, oracle ceiling
4/8, and the 8/8 destination is unchanged. It is a fairness/correctness fix
under the fourth recorded exception. It enters the frontier after
[t143](143-union-phase-request-grouping.md) by number; the human may reorder.
