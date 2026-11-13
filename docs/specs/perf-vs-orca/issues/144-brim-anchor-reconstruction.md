# Brim anchor reconstruction: feed `make_brim` the slice contour from the wall path

Type: task / fairness fix + measured output campaign
Label: wayfinder:task
Status: resolved (2026-11-12; measured fairness fix — the anchor is the
  reconstructed contour, the matched job moves 6/8 cells, and the adhesion
  oracle reaches **3/4 supports-off** (**3/8** overall). Base classic-off flips
  from a −5.65 % skirt-outer undershoot to **+2.89 %** and passes; base
  Arachne-off remains **+3.45 %** (0.45 points outside its 3 % band) and
  graduates the next-shape decision [t146](146-variable-width-anchor-shape.md).
  The two base supports-on cells are byte-identical because their support
  footprint clips the whole brim area. See ## Answer)
Assignee: 2026-11-12 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 144 is the first frontier ticket in
  order (145 is the other, also unblocked)
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

## Answer

**Adopted the uniform half-width reconstruction, and the measurement corrected
one pre-declared assumption: the anchor moves 6/8 cells, not 8/8.** The two
base supports-on cells are byte-identical because the first-layer support
footprint clips the entire brim annulus — no brim path is emitted, so no brim
point reaches the skirt hull either. The support clip is the out-of-scope
support engine (t138/t139) and is untouched. The anchor is still correct on
those cells; it is occluded, not missing. The oracle's supports-on half remains
support-engine-limited, as pre-declared.

**1. What landed.** `SkirtBrim::generate` (`modules/core-modules/skirt-brim/src/lib.rs`)
now reconstructs the slice contour `make_brim` anchors on: the filled
outer-wall footprint (`outer_only` semantics preserved) is offset outward by
`0.5 x the resolved outer-wall width` with `OffsetJoinType::Round` and
`BRIM_ARC_TOLERANCE_MM` (0.0125 mm) before the existing gap, quantisation and
`tryExPolygonOffset` steps. The width is the **per-object modal non-zero**
`Point3WithWidth.width` (ties broken by the first point; every-zero falls back
to `line_width`), recorded rather than hard-coded — the entity's own 0.4 mm on
classic and its dominant width on Arachne. For `skirt_height > 1`, each higher
layer's convex hull is offset outward by half that layer's modal outer-wall
width before it joins the final hull (convex hull commutes with the Minkowski
sum); the matched job authors `skirt_height = 1`, so that path is unit-tested
and **disclosed as not exercised**. The 0.1 mm `brim_object_gap`, the flow
spacing, the quantisation, the concentric fill and the support clip are
unchanged. No `SliceIR`/WIT change; the fourth recorded parity exception covers
exactly these anchor inputs.

**2. Unit coverage.** `brim_anchor_is_the_contour_not_the_wall_centerline`
pins a 0.8 mm point width on a 1.0 mm square loop: the innermost brim line must
sit at `contour 1.4 + gap 0.1 + spacing/2`, and the bead's inner edge must stay
outside the part outline (a `line_width` read lands 0.2 mm too far in and
fails). `skirt_hull_reconstructs_the_contour_on_higher_layers` pins the
`skirt_height = 2` hull offset. The quantisation test now asserts the outer brim
boundary against the reconstructed 1.2 mm half extent. `cargo test -p
skirt-brim` is **23/23 green** across its three test targets
(`skirt_brim_tdd` 18, `finalization_live_tdd` 4, `slicer_module_binding_tdd` 1).

**3. Output campaign (24 runs: 8 cells x baseline, candidate ordinary,
candidate accelerated).** Both ordinary arms share one byte-identical host
binary (`5e902c04…`) and differ only in `skirt-brim` (`92e0d3ed…` baseline /
`74ada739…` candidate ordinary / `95f0cf25…` candidate accelerated). The
baseline arm reproduces the active pre-t144 set in **8/8** cells (t139's four
base hashes and t143's four replacement-fixture benchy hashes). The candidate
records the fresh set, **byte-identical across the ordinary and accelerated arms
in 8/8**, and moves from the baseline in **6/8**:

| cell | candidate | vs baseline |
| --- | --- | --- |
| benchy classic-off | `1ae0cfa8…` | moved |
| benchy classic-on | `d9fc80ba…` | moved |
| benchy arachne-off | `8faf02b1…` | moved |
| benchy arachne-on | `cd7d4a2d…` | moved |
| base classic-off | `c0b9014f…` | moved |
| base classic-on | `585c6a8e…` | **inert** (support clip) |
| base arachne-off | `17f63e93…` | moved |
| base arachne-on | `c9fee59a…` | **inert** (support clip) |

Full hashes: `docs/specs/perf-vs-orca/evidence/t144-brim-anchor/reference.json`,
now the **active reference set** for later output-checking tickets.

**4. Delta audit.** `audit_delta.py` over the 8 pairs: **pass 8/8**. Every
motion line outside `;TYPE:Skirt` / `;TYPE:Brim` is byte-identical with the `E`
field stripped, in all eight cells. The largest `|dE|` is **0.00098 mm**
(tolerance 0.001); changed-E lines are 110 / 23 / 76 / 44 on benchy and
170 / **0** / 219 / **0** on base — the two zeros are the inert supports-on
cells. The adhesion-section movement is the expected outward shift, not a
reflow: the sanity pair on base classic-off moved the brim bbox by −0.192 /
+0.200 mm in x and −0.204 / +0.197 mm in y on a 0.4 mm outer wall.

**5. A/B (attribution only; no wall claim).** Three interleaved pairs per cell
over all eight ordinary cells, plus a six-cell same-arm null control. Median
wall Δ (candidate − baseline): benchy **+1.84 / −2.86 / −0.55 / +0.43 %**, base
**+1.32 / +1.63 / +1.18 / −5.58 %**; median CPU Δ **+1.20 / −0.72 / −0.08 /
+0.04 %** and **0.00 / +0.70 / +0.27 / −0.03 %**. Every median sits inside the
measured null band (worst wall **±24.82 %**, worst CPU **±4.52 %**), so the A/B
reads as no wall claim — the box's own paired noise, not an assumed constant,
bounds the instrument. The skirt-brim surface is ~235 ms of the job.

**6. Adhesion oracle: 3/4 supports-off, 3/8 overall.** Fresh Orca arm (`-Runs 1`,
same matched configs, same vendor pin; the prior Orca tree was absent from
`target/`). Supports-off, candidate vs Orca:

| cell | brim swept (12 %) | skirt swept | skirt outer (3 %) | verdict |
| --- | ---: | ---: | ---: | --- |
| benchy classic-off | +1.79 % | +0.68 % | +1.32 % | PASS |
| benchy arachne-off | +1.73 % | +0.68 % | +1.34 % | PASS |
| base classic-off | +1.14 % | +2.19 % | +2.89 % | **PASS** (was −5.65 %) |
| base arachne-off | +1.86 % | +2.62 % | **+3.45 %** | MISS (3 %) |

Baseline for the same cells and Orca reference: benchy classic-off
+2.90 / +1.54 / +2.85 (pass), benchy arachne-off +2.96 / +1.60 / +2.86 (pass),
base classic-off +5.17 / +4.28 / **+5.65 (fail)**, base arachne-off +6.73 /
+4.41 / **+6.62 (fail)**. The uniform reconstruction closes most of a ~4-point
base gap and flips base classic-off; base Arachne-off lands 0.45 points outside
the 3 % outer-loop band, exactly the case the design pre-declared. **The next
shape is a decision, not a revert:** graduated
[t146](146-variable-width-anchor-shape.md) — per-segment variable-width bead
envelope (module-only, inside the same exception), the direct slice contour
(contract change), or accept 3/8. The oracle is a measure; the stop rule is
t139's geometry-plus-bounded-audit form.

**7. Gates and disclosures.** `cargo clippy --workspace --all-targets -- -D
warnings` exit 0; `cargo xtask check-literals` 0 violations;
`check-test-quality --report` 0 findings. `build-guests --check` and
`build-guests --accelerated --check` both exit 0 before and after; the ordinary
take rebuilt `skirt-brim-guest`, the accelerated take rebuilt `skirt-brim-guest`
and `tree-support-planner-guest`. The accelerated **host** binary is not
buildable on this box (the known `libmimalloc-sys` verbatim-path / MSVC 14.51
failure, t142), so the accelerated **output** arm is the ordinary host plus the
accelerated **component** tree, disclosed. Staging the un-componentised
`target/guests-accelerated/wasm32-unknown-unknown/release/*.wasm` intermediates
produces a load failure; the componentised
`target/guests-accelerated/artifacts/modules/core-modules/**` is the correct
tree (recorded in REPRODUCE). The pre-existing
`integrated_parity_skirt_brim_tdd::integrated_parity_skirt_brim` red was
confirmed at this HEAD: it fails on the missing `brim_object_gap` config key
before any geometry, as the ticket recorded. `layer_slice_tdd`'s z = 0.1 layer
is untouched.

**8. Disposition.** Fairness fix under the fourth recorded exception — **not** a
route step: no wall claim, ~235 ms of surface, oracle ceiling 4/8, destination
arithmetic unchanged. The 8-cells x 3-tools scoreboard is **not** re-derived;
t139's **2.279x–4.187x ordinary / 2.311x–4.179x accelerated, required cut
56.1 %–76.1 %** stand with the adhesion-geometry disclosure. Evidence:
[`evidence/t144-brim-anchor/`](../evidence/t144-brim-anchor/FINDINGS.md)
(`verify.py` re-derives every claim; `--self-test` 13/13 negative controls).
