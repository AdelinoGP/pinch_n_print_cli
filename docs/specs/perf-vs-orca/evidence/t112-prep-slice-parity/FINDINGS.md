# t112 — `PrePass::Slice` parity Phase 1: canonical closing vs the artefact-bound gate

Ticket: [PrePass::Slice parity Phase 1](../../issues/112-prep-slice-parity-phase1.md)
(wayfinder map [Slice performance vs OrcaSlicer](../../map.md)).
Take: 2026-11-03, first frontier child at HEAD `2dade816` (`2dade8160a9a6eb64b91a3b10b8e74d7bcf2add6`).
Scope: source-level parity and consumer analysis plus attribution probes only —
**no production edit, no candidate, no timing campaign, no reference set**.

## Verdict

**Canonical OrcaSlicer has no enclosure closing at all.** Its bridge is *every*
bottom area the layer below does not cover — `PrintObject::detect_surfaces_type`
(`PrintObject.cpp`) computes `opening_ex(diff_ex(slice_surfaces, safety_offset(lower_slices)),
external_perimeter_width / 10)` and tags the result `stBottomBridge`; there is no
"enclosed gap" test, no 12 mm radius and no per-component fraction gate. The only
closing in canonical's bridge path is `merge_bridges` (`LayerRegion.cpp`), a
`closing_ex` at `0.55 × 0.65 × 1.05 × solid_infill_spacing` (≈ 0.169 mm at a
0.45 mm spacing) on the **union of the bridge polygons and their wave-expanded
anchors**, whose purpose is to fill small unassigned holes after shell
regularisation.

PnP's `closing_ex(support, 12 mm, Miter)` + `≥ 10 % refill gate`
(`assemble_flat_bridge_areas`, `crates/slicer-core/src/algos/prepass_slice.rs`) is
therefore a **PnP-local discriminator with no canonical counterpart and no
`DEV` row**: different operand (`slices − lower`, not bridge + anchors), different
radius (**12 mm vs ≈ 0.15–0.17 mm, ~70–80×**), and a different consumer (a boolean
discriminator, not a hole filler). No shape can preserve the gate's exact verdicts
*and* claim canonical parity: t59 already showed the verdict field is decided by
clipper path-list artefacts, and canonical never evaluates that field. The parity
question is therefore to **replace** the gate, not to compute it more cheaply.

Recommended shape **P1**: in `assemble_flat_bridge_areas`, delete the closing and
the fraction gate and emit canonical's rule over PnP's flat family —
`opening_ex(flat_bridge, 0.1 × external_perimeter_width, Miter, 3.0)`, keeping
every surviving component. Measured ceiling and delta:

- **Ceiling, all eight cells clear the 0.80 % floor by 19.1x–62.5x.** The net
  ceiling (closing thread-time, capped by the `PrePass::Slice` stage wall, minus
  the measured canonical-opening cost) is **1,893 ms on the benchy cells** and
  **37,682 ms on the base cells**; as a share of the cell that is **15.3–24.7 %**
  and **23.4–50.0 %**. The closing is **86.8 %** (benchy) / **70.8 %** (base) of
  the stage's own accumulated thread-time, so this is not a vacuous
  "delete the whole stage" bound: uniform-concurrency attribution puts the
  expected saving at **1,720 ms / 26,855 ms**.
- **Delta, bounded and measured on both fixtures.** P1 changes the committed
  flat-family output on **182 of 239** benchy layers and **437 of 469** base
  layers (the first layer is cleared downstream by
  `gate_bridge_areas_by_unsupported_span`, so it contributes no delta). Net area
  **+8.86 mm² (+2.0 %)** on benchy and **+115.90 mm² (+35.9 %)** on base against
  the gate's emitted flat area, because the opening deletes **11.8 % / 14.0 %**
  of the flat-unsupported area as sub-0.08 mm slivers while the gate's rejected
  large components come back. 22 benchy layers grow and 171 shrink; 257 base
  layers grow and 207 shrink.
- **P2's classification-level delta is not boundable here**: canonical's
  unsupported-bottom set is **1,307 mm² / 4,655 mm²** on the two fixtures
  (**2.58x / 9.12x** the all-flat area), and canonical follows classification
  with wave expansion into the shell/sparse/top zones that PnP does not have.
  Recommend deferring P2 to the next prepass take, not shipping it now.

**P2** (canonical rule over the *whole* unsupported set, retiring the
facet-derived `assemble_bridge_areas`) is the fuller parity shape but is **not
boundable in this campaign**: PnP's fixed 1.0 mm `expansion_margin_mm` offset is
not canonical's wave expansion, so a G-code bound would be fiction. Porting
`propagate_waves_ex` is its own take (§3).

**P3** (keep the gate, swap the 12 mm radius for canonical's `merge_bridges`
radius) is **rejected on correctness**: it is not canonical behaviour applied to
this operand, it would collapse the gate to "micro-gaps only", and t59 measured
the gate sitting on numerical knife-edges (`min |fraction − 0.1|` = 4e-5 benchy /
2.4e-4 base). A radius swap is a silent output rewrite dressed as parity.

## 1. The closing result's consumers at HEAD

Re-derived at `2dade816`. t59's map holds; three consumers it did not name are
added (`arachne-perimeters`, `infill-linker`, `rectilinear-infill`), and packet
234a's internal-bridge qualification also reads `region.bridge_areas`.

| consumer | symbol | what it needs |
| --- | --- | --- |
| the gate itself | `assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) | the exact per-component **≥ 10 %** refill boolean; `closed`/`refilled` are then dropped |
| false-site gate | `gate_bridge_areas_by_unsupported_span` (`crates/slicer-core/src/algos/prepass_slice.rs`) called from `crates/slicer-runtime/src/slice_postprocess_prepass.rs` | exact `bridge_areas`; subtracts the committed lower layer's `infill_areas`, clears when the lower layer is absent |
| orientation | `update_external_bridge_orientation` (`crates/slicer-core/src/algos/prepass_slice.rs`), same caller | exact gated geometry + raw lower contours |
| internal-bridge angles | `gate_internal_bridge_sites` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) | `region.bridge_areas` as the `boundary_plines` operand |
| partition | `sync_perimeter_infill_areas_into_slice` (`crates/slicer-runtime/src/region_partition.rs`) | `bridge_final = bridge_areas ∩ perimeter.infill_areas`; `bridge > bottom > top > sparse` precedence |
| paint splits | `paint_segmentation::split_regions` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) | intersects and re-saves `bridge_areas` across modifier splits |
| perimeter flags | `BridgeSpatialIndex` (`crates/slicer-core/src/perimeter_spatial.rs`) | per-vertex `is_bridge` containment |
| guests | `wave-overhangs`, `classic-perimeters`, `arachne-perimeters`, `gyroid-infill`, `rectilinear-infill`, `infill-linker` | `SliceRegionView::bridge_areas()` / `internal_bridge_areas()` |
| visual debug | `pnp-cli/src/visual_debug.rs` | serialises `bridge_areas` verbatim into the SliceIR capture |

So the change surface of P1 is exactly "which flat components are bridges". Every
consumer above takes the emitted polygons, not the closing geometry.

## 2. Canonical vs PnP, by file + function

**Canonical, in pipeline order.**

1. `make_expolygons` (`TriangleMeshSlicer.cpp`) applies `slice_closing_radius`
   while slicing. PnP ports this as `apply_slice_closing_radius`
   (`crates/slicer-core/src/triangle_mesh_slicer.rs`, 0.049 mm) and calls it
   before `assemble_flat_bridge_areas`. **Already parity.**
2. `PrintObject::detect_surfaces_type` classifies the bottom: `bottom =
   opening_ex(diff_ex(layerm->slices.surfaces, lower_layer->lslices,
   ApplySafetyOffset::Yes), offset)` with `offset =
   layerm->flow(frExternalPerimeter).scaled_width() / 10`. The surface type is
   `stBottomBridge` unless `bottom_is_fully_supported`, which needs
   `support_top_z_distance == 0` (default **0.2 mm**), so in all eight matched
   cells every unsupported bottom is a bridge. No lower layer ⇒ `stBottom`.
3. `LayerRegion::process_external_surfaces` extracts the `stBottomBridge`
   surfaces and calls `expand_bridges_detect_orientations`
   (`closing_radius = 0.55f × 0.65f × 1.05f × flow(frSolidInfill).scaled_spacing()`),
   which runs `expand_expolygons` → `wave_seeds` / `propagate_waves_ex` into the
   shell, sparse and top expansion zones, groups the bridges, detects each
   bridge direction from its wave anchors (`detect_bridge_directions`), then
   `merge_bridges` → `closing_ex(bridge ∪ anchors, closing_radius)` and trims
   the expansion zones by the result.

**PnP at HEAD.**

1. `apply_slice_closing_radius` — parity with (1).
2. `classify_region_surfaces` + `assemble_bridge_areas`
   (`crates/slicer-core/src/algos/prepass_slice.rs`) build `bridge_areas` from
   **mesh facet classes** (`SurfaceClassificationIR.bridge_regions`), offset by a
   fixed `expansion_margin_mm` (1.0 mm). Classification is facet-based, not
   cross-section-based; the expansion is a constant offset, not wave propagation,
   and no fill zone is trimmed.
3. `assemble_flat_bridge_areas` adds flat bottoms: `bottom_surface_footprint ∩
   (slices − lower)` clipped to the region, kept only where
   `closing_ex(support = slices − lower, 12 mm)` refills ≥ 10 % of a component.
4. `gate_bridge_areas_by_unsupported_span` subtracts the lower layer again.

**Differences that matter to this ticket.**

- **D1 — classification rule.** Canonical: any unsupported bottom is a bridge.
  PnP: unsupported **and** (facet-classified **or** enclosed by support on
  opposite sides within 24 mm). The enclosure test has no canonical counterpart.
- **D2 — the closing's role.** Canonical's closing fills unassigned holes in a
  bridge **surface**; PnP's closing decides **membership**. Changing only the
  radius cannot turn one into the other.
- **D3 — expansion.** Canonical grows bridges into the surrounding shells by
  wave propagation and trims those zones; PnP offsets by 1.0 mm and trims nothing.
  Adjacent divergence, out of P1's scope.
- **D4 — direction.** Canonical derives the angle from wave anchors; PnP's
  `update_external_bridge_orientation` derives it from the gated area against the
  raw lower contours (packet 235). Same idea (`detect_bridging_direction`), different
  anchor set. Out of P1's scope.
- **D5 — first layer.** Both demote to a non-bridge bottom. Parity.

## 3. Shapes, ceilings and deltas

### P1 — canonical classification over the flat family (recommended)

Replace the closing and the fraction gate in `assemble_flat_bridge_areas` with
canonical's classifier:

```
bridge = opening_ex(
    flat_bridge,                    // bottom footprint ∩ (slices − lower) ∩ infill
    0.1 * external_perimeter_width,
    Miter, 3.0,
)
```

and emit every component. The canonical safety offset (`diff_ex(..., Yes)`
inflates the lower set by `SCALED_EPSILON` = 1e-4 mm = 1 PnP unit) must be added to
the diff so boundary-coincident material is not labelled bridge; that is the only
new geometry input. `external_perimeter_width` resolves exactly as
`support_analysis_producer::resolve_contact_params` does it: `outer_wall_line_width`
when present, else `line_width`.

- Cost removed: the 12 mm closing on `support`, the `closed − support`
  difference, and the per-component `component ∩ refilled` loop.
- Cost added: one `offset` (safety), one `difference`, one `opening_ex` per layer
  with a non-empty flat family.
- Delta: only flat-unsupported components that currently fail the gate become
  bridges, minus what the opening shaves off. No other family moves.

### P2 — canonical classification over the whole unsupported set

Also delete `assemble_bridge_areas`' facet path and derive `bridge_areas` from
`diff(slices, lower)` alone, matching `detect_surfaces_type`. This is the full
parity shape, and its **ceiling is at least P1's** — it removes the facet offsets
and the `bottom_surface_footprint` work on top of the closing — but it is **not**
a drop-in: canonical follows it with wave expansion into shells/sparse/top and
trims those zones, and PnP's 1.0 mm `expansion_margin_mm` offset is not that.
Without the expansion port the G-code delta cannot be bounded, so P2 fails this
ticket's stop rule on the delta, not on price. Record it as fog for the next
prepass take.

### P3 — radius swap (rejected)

Keep the gate, feed `merge_bridges`' `closing_radius` into `closing_ex`. This is
neither the canonical operand nor the canonical consumer, and it turns the gate
into a near-empty micro-gap filter. Rejected on correctness; no timing.

## 4. Measured ceilings

### Method

Two fresh measurements at HEAD `2dade816`, both on this Windows dev host with
`RAYON_NUM_THREADS=12`:

1. **Production attribution.** `pnp_cli slice --profile --instrument-stderr` on
   `tmp/3dbenchy.stl` and `tmp/base.stl` with the matched classic supports-off
   config (`evidence/matched-pair/configs/pnp-classic-supports-off.json`) and the
   in-tree module dir. The capture gives the `PrePass::Slice` `stage_complete`
   wall (S), the `host:slice` native row's accumulated thread-time (T), and the
   `polygon_ops::closing_ex` scope row inside it (C). `closing_ex` has exactly one
   caller in that stage — `assemble_flat_bridge_areas` — because
   `apply_slice_closing_radius` goes through `polygon_ops::offset`, not
   `closing_ex`.
2. **Probe.** `t59`'s reconstruction harness (`probe.patch`) plus this take's
   canonical arm, run per fixture: it re-derives the production inputs, asserts
   reconstruction fidelity against `execute_prepass_slice_single_layer_with_cache`
   on four sampled layers, and prices the canonical opening (O) on every layer.

**Ceiling** = `min(C, S) − O`. C and O are accumulated thread-time, so the result
is an upper bound on the wall saving (parallelism ≥ 1); subtracting O in full is
conservative because the opening can hide in freed parallel slack. Cell walls are
the PnP ordinary medians of record (t103 supports-off, t106 supports-on).

### The table

| cell | cell wall | Slice wall S | Slice thread T | closing C | closing share | expected | net ceiling | % of cell | × floor |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 7,680 | 1,983 | 20,259 | 17,576 | 86.8 % | 1,720 | **1,893** | 24.65 % | **30.8x** |
| benchy arachne-off | 7,690 | 1,983 | 20,259 | 17,576 | 86.8 % | 1,720 | **1,893** | 24.62 % | **30.8x** |
| benchy classic-on | 12,400 | 1,983 | 20,259 | 17,576 | 86.8 % | 1,720 | **1,893** | 15.27 % | **19.1x** |
| benchy arachne-on | 12,220 | 1,983 | 20,259 | 17,576 | 86.8 % | 1,720 | **1,893** | 15.49 % | **19.4x** |
| base classic-off | 75,380 | 37,929 | 410,505 | 290,646 | 70.8 % | 26,855 | **37,682** | 49.99 % | **62.5x** |
| base arachne-off | 78,890 | 37,929 | 410,505 | 290,646 | 70.8 % | 26,855 | **37,682** | 47.77 % | **59.7x** |
| base classic-on | 160,860 | 37,929 | 410,505 | 290,646 | 70.8 % | 26,855 | **37,682** | 23.43 % | **29.3x** |
| base arachne-on | 161,140 | 37,929 | 410,505 | 290,646 | 70.8 % | 26,855 | **37,682** | 23.39 % | **29.2x** |

All times in ms. "expected" is `closing share × S`, the attribution estimate
under uniform concurrency; the gate uses the larger, conservative "net ceiling".
The opening's own cost is **89.9 ms** (benchy) and **246.7 ms** (base) — the
closing is **146x / 853x** more expensive.

**Every cell clears the 0.80 % floor.** The narrowest is benchy classic-on at
19.1x; the widest is base classic-off at 62.5x. The shape is therefore not a
screening candidate — it is large enough that the campaign's risk is the output
delta and downstream parity, not the wall.

**Why the arachne cells carry the classic measurement.** `PrePass::Slice` runs
before shell classification and perimeter generation; `wall_generator` is read
only at module dispatch (`crates/slicer-runtime/src/run.rs`), never by
`prepass_slice.rs` or by the per-layer prepass, and the stage's only config
inputs are `slice_closing_radius` and `flat_bridge_closing_join`, both
generator-independent. So the stage cost, the closing cost and the opening cost
are identical across generator in the same fixture; only the cell wall differs.

**Probe vs production cost.** The probe's serial closing totals are lower than
the production `closing_ex` rows (**13.10 s vs 17.58 s** on benchy, **210.36 s vs
290.65 s** on base) because the production stage runs the closing on rayon
workers, where per-activation wall includes memory-bandwidth contention. Probe
totals also move between runs (benchy's closing moved 12.18 → 13.10 s across two
of this take's own runs, +7.6 %), so they are attribution only. The ceiling uses
the production number, which is the one the removed work actually costs.

## 5. Output-delta bound

The probe enumerates every layer with a non-empty flat family on both fixtures and
records, per layer, the gate's emitted component count and area, the all-flat
area, P1's opened area, and P2's classification-level area.

| fixture | closing calls | flat components | gate emitted | P1 components | gate area | all-flat area | P1 area | P2 area |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy | 240 | 3,268 | 1,298 | 315 | 438.41 | 506.85 | **447.27** | 1,307.30 |
| base | 470 | 11,693 | 5,899 | 2,454 | 323.03 | 510.19 | **438.92** | 4,654.62 |

Areas in mm², committed layers only (layer 0 is cleared downstream by
`gate_bridge_areas_by_unsupported_span`, so it contributes to the call and cost
counts but not to the areas). `gate area` is the area of the components the
≥ 10 % gate emits today; `P1 area` is canonical's opening applied to the whole
flat family.

**Mechanism of the delta, in both directions.**

- **The gate rejects real area, not only slivers.** It emits 438.41 / 323.03 mm²
  of the 506.85 / 510.19 mm² flat family — **13.5 % / 36.7 %** of the area is
  rejected. Those are the free-edge bands and cantilever lips canonical calls
  bridges, and P1 brings them back (+68.4 / +187.2 mm²).
- **Canonical's opening removes the thin tail.** `opening_ex` at 0.04 mm takes
  the family from 3,268 / 11,693 components to 315 / 2,454, deleting
  **59.58 / 71.27 mm² (11.8 % / 14.0 % of the family)**. The mean component area
  goes from 0.155 / 0.044 mm² to 1.42 / 0.179 mm², i.e. the population was
  dominated by sub-0.08 mm-wide features that canonical does not print as
  bridges. Net change: **+8.86 mm² / +115.90 mm²**.
- **Layer counts.** 182 / 239 benchy layers and 437 / 469 base layers change by
  more than 0.01 mm²; 22 benchy and 257 base layers grow, 171 and 207 shrink.
  No layer changes by a mechanism outside `assemble_flat_bridge_areas`.
- **The first layer pays a closing the next stage throws away.** Layer 0 has no
  lower cross-section, so `unsupported` is the whole slice, `support` is the
  crescent `apply_slice_closing_radius` leaves outside the raw polygons, and the
  12 mm closing runs on it; `gate_bridge_areas_by_unsupported_span` then clears
  the result because the layer below is absent. On benchy that call is
  **595.8 ms of the probe's 13,101.0 ms closing total (4.5 %)**; on base the same
  call is **1.1 ms** (39 support paths). P1 removes it.

**What the delta reaches.** `gate_bridge_areas_by_unsupported_span` re-subtracts
the lower layer (a no-op for the added components, which are already
unsupported), `update_external_bridge_orientation` re-derives the angle from the
new gated set, and the partition hands the new polygons to
`wave-overhangs` / `classic-perimeters` / `arachne-perimeters` / `gyroid-infill` /
`rectilinear-infill` / `infill-linker`. So the G-code delta is "flat-unsupported
area that the gate used to reject becomes bridge fill, and thin artefacts stop
being bridge fill". It can change per-vertex `is_bridge` flags, fill roles and
wave directions, but not the wall paths (`slices`/`perimeter.infill_areas` feed
those) and not the material volume: the same cross-section is printed with
different roles and directions.

**P2's delta is not boundable.** Canonical's unsupported-bottom set is
**1,307.30 mm² (2.58x** the all-flat area) on benchy and **4,654.62 mm²
(9.12x)** on base. PnP's facet family covers an unmeasured part of that set, and
canonical's wave expansion into the shell/sparse/top zones has no PnP counterpart
(`expansion_margin_mm` is a flat 1.0 mm offset). Any G-code bound for P2 would be
fiction until the expansion port exists.

## 6. Recommendation and the candidate's contract

### The ruling this take recommends

**Build P1.** It is the only shape that (a) is canonical behaviour on the
artefact field it touches, (b) clears the 0.80 % floor in all eight cells with
margin (19.1x–62.5x on the ceiling), and (c) has a measured, mechanism-explained
delta on both heavy fixtures. It does **not** need a new prepass stage, IR field,
or WIT surface: it deletes work inside `assemble_flat_bridge_areas`.

### The candidate's exact shape

1. In `assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`):
   - delete `closing_ex`, `refilled`, the per-component `intersection(component,
     refilled)` fraction test, `FLAT_BRIDGE_ENCLOSURE_RADIUS_MM`,
     `FLAT_BRIDGE_ENCLOSURE_MIN_FRACTION`, `MAX_FLAT_BRIDGE_SPAN_MM` and the
     `closing_ex` import;
   - compute `bridged = opening_ex(&flat_bridge, 0.1 * external_perimeter_width,
     OffsetJoinType::Miter, 3.0)` and extend `region.bridge_areas` with all of it,
     setting `is_bridge = true` when non-empty.
2. Add canonical's `ApplySafetyOffset::Yes`: inflate the **lower** slice set by
   `SCALED_EPSILON` = 1e-4 mm = **1 PnP unit** before the difference, i.e.
   `unsupported = difference(current_slices, offset(prev_slices, 1e-4, Miter, 0.0))`.
   At HEAD `unsupported` is a bare `difference`. Without this the new rule labels
   boundary-coincident material as bridge — the one canonical detail P1 cannot
   skip.
3. Resolve `external_perimeter_width` exactly as
   `resolve_contact_params` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`)
   does: the `outer_wall_line_width` extension when present, else `line_width`.
   Do not hardcode 0.4 mm.
4. `flat_bridge_closing_join` becomes unused. Remove the key from the reference
   doc and the manifests in the same change, or the config surface will claim a
   knob that does nothing.

### Tests that must move with it

- `crates/slicer-core/tests/algo_prepass_slice_tdd.rs`: the enclosure unit tests
  (`free_edge_stays_out`, the gap-span positive, the drilled-layer cases) encode
  the gate. They become canonical-rule tests: a free-edge band becomes a bridge,
  a sub-0.08 mm sliver is opened away, a gap span is kept.
- `crates/slicer-core/tests/bridge_false_site_gating_tdd.rs` and
  `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`: the
  `non_bridge_region_has_empty_bridge_areas` fixture must be re-derived under the
  new rule.
- The post-t81 reference set: every cell whose G-code carries flat-unsupported
  area moves. Re-record under t111's 16-cell contract; do not hand-edit.
- No `DEV` row exists for the enclosure gate today (it was never filed). P1
  removes it, so none is owed. The **facet-derived external bridge
  classification** and the fixed `expansion_margin_mm` offset remain unrecorded
  divergences from `detect_surfaces_type` + wave expansion; file a row when P2 is
  scoped.

### Campaign order (t111's contract, unchanged)

1. Fresh **16-cell** reference set, baseline reproducing the post-t81 set.
2. Candidate-vs-old-reference G-code delta audited **before** timing: expect the
   flat family's area to move by the probe's signed totals (+8.86 mm² benchy,
   +115.90 mm² base) plus the downstream `gate_bridge_areas_by_unsupported_span`
   re-clip; any movement outside the flat family is a red flag, not noise.
3. Paired wall/CPU gate, both arms byte-identical within themselves, strict wall
   improvement ≥ 0.80 % in **all eight** cells.

`P2` is the next prepass take's question (wave expansion + facet-path retirement),
not this campaign's. `P3` is rejected; do not re-open it without a canonical
citation for applying `merge_bridges`' radius to the support set.

## 7. Disclosures

- **Probe config.** The probe uses `ResolvedConfig::default()` (0.2 mm layers, 0.4
  mm `line_width`), which reproduces the cells' 240 / 495 layer counts (t104).
  It does not replay the matched config's manifest beyond that, so its areas are
  the flat family's on the matched layer schedule, not a cell-exact G-code
  account.
- **Walls of record predate the merge.** The cell walls in the ceiling table are
  t103 (supports-off) and t106/pinned (supports-on) medians, measured before the
  `a5e9a403` merge. [Ticket 115](../../issues/115-post-merge-reconciliation.md)
  owns re-baselining them; the ceiling multiples move with those walls, not with
  this take's measurements.
- **Instrumented walls are not performance claims.** `--profile` meters fuel and
  the native sink records marks; the `PrePass::Slice` stage wall (1,983 / 37,929
  ms) comes from that instrumented run and is used only as a cap on the closing's
  contribution. Cell performance is never quoted from it.
- **Arachne not profiled.** The arachne columns reuse the classic-cell stage and
  closing costs, justified by the source-level generator-independence argument
  above. If a future campaign shows a generator effect on `PrePass::Slice`, this
  table must be re-run on all eight cells.
- **The probe's closing is faster than production's.** 12.18 s vs 17.58 s
  (benchy) and 209.6 s vs 290.6 s (base): the probe is serial, the stage is not.
  The ceiling uses the production figure, so the probe's speed cannot inflate it.
- **No load sampling.** The two profile runs and the two probe runs were taken
  back-to-back on the dev host, but no CPU-load trace was recorded for this take.
  Everything reported is accumulated thread-time, an area, or a count — no wall
  claim rests on host quietness.
- **No production edit.** The probe is a test target in `probe.patch`; it was
  applied to measure and reverted afterwards. `git status` on `crates/` and
  `modules/` is clean in the delivered tree.

## Evidence index

- `probe.patch` — the t59 probe harness plus this take's canonical arm, exactly as
  run (test file + `Cargo.toml` target). Apply to re-derive `target/t112/`.
- `reduce_t112.py` — raw `target/t112/` → `reduced/`.
- `reduced/summary-benchy.json`, `reduced/summary-base.json` — probe aggregates and
  per-layer rows.
- `reduced/profile-benchy.json`, `reduced/profile-base.json` — `PrePass::Slice`
  stage wall, native thread-time, and the `polygon_ops::closing_ex` row.
- `reduced/ceilings.json` — the eight-cell ceiling arithmetic.
- `verify.py` — exits 0 against `reduced/` only.
