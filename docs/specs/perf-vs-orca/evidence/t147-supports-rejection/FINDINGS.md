# t147 — What the `exact-Z occupancy` body rejections drop

Route: [Supports-on output is support-incomplete: what the `exact-Z occupancy` body rejections drop?](../../issues/147-supports-on-body-rejection-attribution.md).
Graduated from: [Benchy fixture replacement](../../issues/145-benchy-fixture-rebaseline.md)'s
2026-11-13 take.

**Headline: the rejections are not evidence that the demands were redundant.
They are a plane mismatch between the planner's carve and the host's
validator, amplified by an all-or-nothing gate. The PnP supports-on output is
support-incomplete: 32 complete support layers are missing from the emitted
base G-code and 1 from benchy, including the layer directly under an overhang.
On base that is ~36.5 m of absent support path (10.28 % of the PnP support
path this take measures) and on benchy 0.19 m (0.23 %). The four supports-on
ratios of record are therefore not quotable as a matched-settings claim; they
stand only with this disclosure until the asymmetry is closed.**

## 1. Census

The probe (`probe.patch`) dumps every validated plan entry, every rejection's
overlap against the exact-Z occupancy, and the same entry against the
`SupportAnalysisIR::model_occupancy` payload the planner carved with.

| cell | diagnostics | rejected entries | rejected layers | unique demands | demands only rejected | retained entries |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-on | 101 | 1 | 1 (L170) | 101 | **0** | 201 |
| base classic-on | 44,474 | 32 | 32 | 2,892 | **0** | 400 |

The diagnostic count is a count of `(body, demand)` **pairs**, not of
unsupported overhangs. One rejected entry carries every demand its tree column
serves, so a column whose single layer is rejected emits thousands of
diagnostics at once. On base the 44,474 pairs collapse to 2,892 unique demands,
and **every one of them is also carried by a retained entry**; no demand is
uniquely dropped. The emitted-support check in §3 is what decides whether the
print is incomplete, not the diagnostic count.

The rejected entries are the *only* entry at their layer: the probe finds no
retained entry on any of the 33 rejected layers, and the G-code has no support
extrusion there. Base's 32 rejected layers are:

```
15, 18, 19, 27, 33, 34, 35, 36, 38, 39, 40, 41, 80, 99, 132, 155, 156, 177,
192, 216, 218, 259, 266, 316, 329, 351, 352, 362, 387, 395, 411, 414
```

That includes two four-layer runs, **33–36** and **38–41** (layer 37 prints
1,399.3 mm of support and is present), plus 24 isolated layers spread over the
rest of the print.

## 2. Geometry: the planner carves the midplane, the host validates the print plane

The two planes are deliberately different:

- `SupportAnalysisIR::model_occupancy` is filled from `SliceIR`, which is cut at
  `layer_sample_plane_z` (`crates/slicer-core/src/algos/prepass_slice.rs`):
  `layer.z - 0.5 * effective_layer_height`, the **layer midpoint**.
- `SupportPlanEntry::anchor_z` is `layer_plan.layers[i].z`, the **print plane**
  (the top of the layer), and `validate_entry`
  (`crates/slicer-wasm-host/src/support_aggregation.rs`) queries the exact-Z
  occupancy at that `anchor_z`.
- The tree planner's `layer_occupancy_payload` and `build_roles_batched`
  (`modules/core-modules/tree-support-planner/src/lib.rs`) carve every role
  against the midpoint payload (inflated by `support_object_xy_distance`), then
  the host re-checks the same role at the print plane with a **zero** overlap
  tolerance (`SUPPORT_OVERLAP_TOLERANCE`).

The measured consequence is uniform: **every rejected entry overlaps the print
plane and none overlaps the midpoint payload.**

| cell | entries | print-plane overlap | midpoint overlap |
| --- | ---: | --- | --- |
| benchy classic-on | 1 | 5.8342 mm² (2.276 % of a 256.33 mm² body) | 0.0000 mm² |
| base classic-on | 32 | 8.4e-6 – 1.4706 mm², sum 6.8920 mm² (median 0.0805 mm²; 0.000001 %–0.069 % of body) | **0.0000 mm² on all 32** |

The two ends of that spread are different physical stories:

- **Benchy L170 is a real collision.** The model cross-section gains 5.83 mm² between
  the midpoint (z = 34.1) and the print plane (z = 34.2); the body the planner
  emitted would print through it. The host rejection is correct *for the body as
  emitted*.
- **Base is a sliver family.** The overlap is 8.4e-6 to 1.47 mm² on bodies of
  24.6 to 3,308 mm²; the body is disjoint from the midpoint, so the planner's
  carve did its job on the plane it was given. The validator sees the sub-layer
  growth the midpoint cannot show.

In both cases the planner is emitting a body that overlaps the exact print
plane because it never sees that plane. The host gate is doing the planner's
job for it.

## 3. The emitted-support check

The probe's G-code is byte-identical to the t145 currency on both fixtures
(benchy `d9fc80ba…`, base `585c6a8e…`), so the probe did not move output. PnP
prints **zero** support at every rejected layer; Orca prints support at every
corresponding plane. Orca's support planes are independent of the 0.2 mm
object grid (`independent_support_layer_height = 1`), so coverage below uses
the nearest Orca support plane at or below the candidate plane within 0.35 mm.

**Immediate overhang candidates.** For each rejected layer L, the probe selects
the candidate demands whose overhang is at L+1: these are the overhangs whose
top interface would have printed at L. Coverage is the area of the candidate
footprint intersected by buffered support extrusions (half width 0.2 mm).

| cell | immediate candidates | PnP coverage at L | Orca coverage at L | Orca-supported (> 5 %) |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-on | 1 (candidate 68, L171, 22.403 mm²) | **0.0** | 0.3685 | **1 / 1** |
| base classic-on | 32 | **0.0 on all** | 0.0–0.996 | **19 / 32** |

Benchy candidate 68 is the clearest case: at z = 34.2 PnP has 0 % coverage and
Orca 36.85 %; at z = 34.0 PnP 9.6 % and Orca 63.8 %. PnP's nearest meaningful
support is at z = 33.8 (0.6 mm below the overhang); Orca's is at the 0.2 mm
support gap. A second benchy candidate, 76 (L183, 214.7 mm²), is Orca-supported
at 36.0–36.4 mm (14.7 %–32.7 %) and PnP-unsupported at all four planes; it is
carried by the same rejected column's demands but is not directly above the
rejected layer, so it is recorded as a second observation rather than folded
into the L170 causal claim.

On base, 19 of the 32 overhangs directly above a missing layer have Orca
support at that plane and none in PnP; the other 13 are unsupported by both
tools at that exact plane (Orca's variable support grid may place their support
elsewhere, so they are not counted as evidence either way).

**Missing path bound.** For each rejected layer the reduced set records the
support path at the layer and interpolates the missing path from the nearest
present layers below and above.

| cell | missing support path | share of PnP support path | solid-fill upper bound |
| --- | ---: | ---: | ---: |
| benchy classic-on | 192.601 mm | 0.233 % | — |
| base classic-on | 36,466.736 mm | 10.2765 % | 116,454 mm (summed entry area / 0.42 mm) |

The neighbor interpolation is the measured estimate (the layers around a gap
are the same column, so their paths bracket the missing one); the area bound is
reported only as an upper bound, because support is not a solid fill. The
support-path totals are measured by the same t147 parser on both arms (Support
+ Support interface extrusion); they are **not** the t145 `support path`
metric and must not be mixed with it.

## 4. Classification

**Class (b), a defect, on two independent layers.**

1. **Planner plane defect.** `build_roles_batched` carves against the midpoint
   payload, so a body can be disjoint at `z - 0.5 h` and overlap at `z`. The
   host's exact-Z rejection is the correct detection of that overlap; the
   planner should not have emitted the body. The benchy L170 entry is the
   unambiguously real case (5.83 mm² inside the model cross-section at the print
   plane); the base family is the sliver case where a 8.4e-6–1.47 mm² overlap
   condemns a 24.6–3,308 mm² body.
2. **All-or-nothing host gate.** `validate_entry` removes the complete entry if
   any role overlaps; `record_rejection` then emits one unmet record per
   `(body, demand)` pair. Canonical `draw_circles` instead uses
   `avoid_object_remove_extra_small_parts(circle, get_collision(...))` — a
   difference that keeps what survives. Here the difference would keep 250.5 of
   the benchy 256.3 mm² and ≥ 99.93 % of every base body. Dropping the whole
   layer turns a sliver into a full 0.2 mm gap and, because each rejected entry
   is the only entry at its layer, into a missing support layer.

It is **not** class (c): the demands are not redundant. No demand is uniquely
dropped, but the printed column loses its layer and, at the overhang, its top
interface. It is also **not** class (a) for the base family: a correct
rejection would mean the body genuinely intersects the model at the print
plane, and the planner is the component that put it there. The rejection is
correct as a safety net; the unmet demand is not expected behaviour.

## 5. Fairness price and the ruling

The t145 post-swap supports-on ratios of record are benchy classic-on
**2.683x**, benchy arachne-on **2.580x**, base classic-on **3.001x**, base
arachne-on **3.455x**. All four PnP runs are `degraded: true` with the
rejection counts above, and all four print less support than their own planner
demanded. The supports-off cells have zero rejections and are unaffected.

**Ruling: the four supports-on ratios are not quotable as a matched-settings
claim.** They remain historical numbers with this disclosure (32 base layers /
1 benchy layer, 36.47 m / 0.19 m of missing support path, 19/32 and 1/1
immediate overhangs Orca-supported but PnP-unsupported), and no **new**
supports-on ratio of record may be taken until the asymmetry is closed. The
standing policy is to close asymmetries before a ratio of record; a fairness
asymmetry ticket is graduated for the fix and the re-record.

The wall-clock direction is unambiguous even though the magnitude is not
measured here: PnP paid *less* work than the matched job demands, so the four
ratios understate PnP's true cost and the destination arithmetic (PnP already
slower in every cell) is not rescued by this asymmetry.

## 6. What would close it

The fix belongs on the planner side; moving the validator to the midpoint would
hide the benchy L170 collision and print support through the model.

- Carve every emitted role against the **print-plane** occupancy at
  `anchor_z`, not only `SupportAnalysisIR::model_occupancy`. The host already
  computes those exact cross-sections for `validate_entry`'s prefetch
  (`prefetch_cross_sections` over every entry's `anchor_z`), so the payload
  exists in the same take; the cheap design is to publish it to the support
  planners (a `print_plane_occupancy` map, or the existing exact-Z query host
  interface) and use it as the final carve set, with the midpoint union kept as
  a conservative extra.
- Keep the zero-tolerance host gate as the tripwire, but make its diagnostic
  proportional: record the overlap area and the clear area, and count unique
  demands rather than `(body, demand)` pairs, so a sliver is visible as a
  sliver.
- Re-run the four supports-on cells and re-record the reference set after the
  fix; the supports-off cells are unaffected and need no re-take.

## 7. Provenance and reproduction

- `probe.patch` is the temporary host probe (entry/overlap dump + candidate
  dump, enabled only by `PNP_T147_PROBE_DIR`). It was applied to
  `crates/slicer-wasm-host/src/support_aggregation.rs` and
  `crates/slicer-wasm-host/Cargo.toml` for the take and removed before landing;
  it is **not** a production change.
- The probe is output-neutral: its G-code hashes are the t145 currency on both
  fixtures (benchy `d9fc80ba…`, base `585c6a8e…`).
- `census.json` is the reduced set; `reduce_t147.py` regenerates it from the
  gitignored raw tree at `target/t147/`; `verify.py` is stdlib-only and checks
  every claim in this document from `census.json`.
- Orca g-code hashes: benchy `d62f4e3f…`, base `a5553430…` (t145 m3).
- The t145 Orca arm prints support on independent support planes
  (`independent_support_layer_height = 1`); the coverage comparison uses the
  nearest plane at or below the candidate within 0.35 mm and records the plane
  in `census.json`. That grid difference is a measurement caveat, not a claim
  in this ticket.
