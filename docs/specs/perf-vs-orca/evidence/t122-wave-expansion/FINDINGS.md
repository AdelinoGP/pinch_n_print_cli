# t122 — canonical bridge wave expansion: Phase-1 screen

Ticket: [PrePass::Slice parity Phase-1: wave expansion and the facet-path
retirement](../../issues/122-prep-slice-wave-expansion-facet-retirement.md)
(wayfinder map [Slice performance vs OrcaSlicer](../../map.md)).
Take: 2026-11-06, first takeable frontier child at HEAD `4bc021a4`.
Scope: source-level parity and consumer analysis plus a real-layer probe —
**no production edit, no candidate, no G-code campaign, no reference set**.

## Verdict

**STOP on the combined expansion shape. BUILD the classification half.**

The ticket bundles two changes that the source does not let us ship together:

1. **Retire the facet path and classify canonical's whole unsupported bottom
   set.** This is a prepass-local change, its delta is bounded on both
   fixtures, and its cost screen clears the 0.80 % floor in all eight cells at
   **12.5x–24.5x** (benchy) and **89.7x–213.9x** (base). This is a candidate.
2. **Port `expand_expolygons` / `wave_seeds` / `propagate_waves_ex` /
   `merge_bridges` and trim the three zones.** The expansion zones do not exist
   at `PrePass::Slice` — the sparse zone is created at `Layer::Perimeters`
   commit — so this is a per-layer partition change, not a prepass change. Its
   measured cost on benchy is **102.6 % of the classification half's entire
   saving** (1,151.9 ms added vs 1,122.9 ms removed), so the combined screen is
   a wash at best and **fails all four benchy floors**; the all-8-cell gate
   therefore fails. It is not a candidate.

The headline correction to the ticket's premise: **the expansion half is
mis-staged as a `PrePass::Slice` change.** The classification half is the
contained, large win; the expansion half belongs at the `Layer::Perimeters`
commit and needs its own stage contract. The classification-only candidate is
graduated as [ticket 123](../../issues/123-facet-bridge-path-retirement.md).

## 1. Canonical to PnP, by file + function

Canonical, in pipeline order:

| canonical step | symbol (file) | what it does |
| --- | --- | --- |
| classify | `PrintObject::detect_surfaces_type` (`PrintObject.cpp`) | `bottom = opening_ex(diff_ex(slices, safety_offset(lower_layer)), 0.1 × external_perimeter_width)`; every surviving unsupported bottom is `stBottomBridge` unless the layer below is absent (then `stBottom`). |
| extract zones | `LayerRegion::process_external_surfaces` (`LayerRegion.cpp`) | `stInternalSolid` → **shells**, `stInternal` → **sparse**, `stTop` → **top**. Bottom (`stBottom`) is not an expansion zone. |
| expand | `expand_expolygons` (`LayerRegion.cpp`) → `wave_seeds` / `propagate_waves_ex` (`Algorithm/RegionExpansion.cpp`) | For each zone, intersect the bridge boundary (expanded by `tiny_expansion`) with the zone, then propagate waves in `RegionExpansionParameters::build(full_expansion, 0.1, 5)` steps, clipped to the zone. |
| direction | `detect_bridge_directions` (`LayerRegion.cpp`) | Derives each bridge angle from the wave anchors, not from the lower contours. |
| merge | `merge_bridges` (`LayerRegion.cpp`) | Groups bridges whose anchors intersect, unions bridge + expansions, applies `closing_ex` at `0.55 × 0.65 × 1.05 × solid_infill_spacing`, emits `stBottomBridge`. |
| trim | `expand_bridges_detect_orientations` (`LayerRegion.cpp`) | `zone.expolygons = diff_ex(zone.expolygons, expanded_bridges)` for every zone the bridge expanded into. |

PnP at HEAD has the two halves in different places:

| PnP step | symbol (crate path) | what it has |
| --- | --- | --- |
| facet classification | `assemble_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) | Mesh-facet `BridgeRegion` footprints intersected with `region.infill_areas`, offset by `expansion_margin_mm` (1.0 mm), re-intersected. Appends to `bridge_areas`; also writes an orientation that the downstream gate overwrites. |
| flat classification | `assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`) | t116's canonical opening over the **flat** family only (`bottom_surface_footprint ∩ unsupported`), `0.1 × external_perimeter_width`, every surviving component kept. |
| gate | `gate_bridge_areas_by_unsupported_span` (`crates/slicer-core/src/algos/prepass_slice.rs`, called from `crates/slicer-runtime/src/slice_postprocess_prepass.rs`) | Subtracts the raw lower layer and clears when the object has no lower layer. |
| direction | `update_external_bridge_orientation` (`crates/slicer-core/src/algos/prepass_slice.rs`) | Re-derives the angle from the **gated** areas and the raw lower contours. This overwrites the facet path's angle. |
| shell zones | `commit_shell_classification_builtin` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) | Populates `top_solid_fill`, `bottom_solid_fill`, `internal_solid_fill`. |
| sparse zone + partition | `sync_perimeter_infill_areas_into_slice` (`crates/slicer-runtime/src/region_partition.rs`) | Runs at `Layer::Perimeters` commit; sets `perimeter.infill_areas`, then `bridge > bottom > top > sparse`. `SlicedRegion::sparse_infill_area` is documented as **empty until `Layer::Perimeters` commits**. |

**The stage mismatch.** Canonical's expansion runs after perimeters and fill
surfaces exist. In PnP:

- `PrePass::Slice` has `polygons`, `infill_areas` (= `polygons` at that
  stage), `bridge_areas`, and `is_bridge`. It has **no** shell/sparse/top
  zones.
- `PrePass::ShellClassification` runs after `PrePass::Slice` and creates the
  top/bottom/internal-solid zones.
- `sparse_infill_area` does not exist until the `Layer::Perimeters` commit
  partition, and `perimeter.infill_areas` — the wall-inset boundary the
  expansion must clip against — exists only there too.

So a faithful port cannot run inside `PrePass::Slice` without duplicating zone
derivation (a second source of truth for the fill partition). It must run at
or before the `Layer::Perimeters` partition. That changes the campaign's
stage, acceptance contract, and per-layer transport budget. The classification
half remains a `PrePass::Slice` change and is the only half that fits this
ticket's framing.

## 2. What the current prepass pays

The probe measured the production path per layer on both fixtures:

- `classify_region_surfaces` scans every bridge-facet of each region at the
  layer, transforms three vertices per facet, and point-in-polygon tests them.
  It returns a `bool` that `assemble_bridge_areas` never reads and that is
  re-derivable as `!bridge_areas.is_empty()`.
- `assemble_bridge_areas` produces the facet candidates; the downstream gate
  then subtracts the lower layer. The probe's facet polygon sum before the
  gate is **14,232.05 mm²** (benchy) and **419,106.22 mm²** (base), while the
  gated union actually emitted is **706.73 / 4,473.93 mm²**. At most **5.0 % /
  1.1 %** of the facet polygon area survives the gate; **≥ 95.0 % / 98.9 %** is
  discarded (the exact split between facet and flat is not separable in the
  union; the bound is conservative).
- `assemble_flat_bridge_areas` produces **447.16 / 438.76 mm²**, matching
  t116's **447.27 / 438.92 mm²** within 0.03 %.
- `update_external_bridge_orientation` overwrites the facet path's angle, so
  the facet orientation work is already dead.

This is the case for retiring the facet path: on base it is a very expensive
way to produce a set that is **1.04x** the canonical set (the current path costs
**27x** the canonical classifier). On benchy it produces only **54 %** of the
canonical set (`706.73 / 1,306.72`) while costing **3.3x** the canonical
classifier, because the facet+flat split misses unsupported sloped/free-edge
area that canonical classifies.

## 3. Probe method

`probe.patch` adds a temporary ignored test
(`t122_wave_expansion_probe_tdd`) and a `[[test]]` target. It loads the
fixture, places it on the default bed exactly as `pnp_cli` does, plans the
production 0.2 mm layer tops, runs `execute_mesh_analysis`, batches the raw
slices and bottom-surface footprints, and reconstructs each layer exactly as
`execute_prepass_slice_single_layer_impl` does. On three sampled layers per
fixture it asserts the reconstruction against
`execute_prepass_slice_single_layer_with_cache` (exact within 1e-6 relative).

Canonical arithmetic for the matched job (0.2 mm layers, 0.4 mm width, two
walls):

```text
spacing            = 0.4 - 0.2 × (1 - π/4)        = 0.35708 mm
shell_width        = 0.5 × 0.4 + 0.35708
                     + 0.35708 × (2 - 1)          = 0.91416 mm
expansion_solid    = shell_width × √2              = 1.2928 mm
expansion_sparse   = spacing                       = 0.35708 mm
closing_radius     = 0.55 × 0.65 × 1.05 × spacing  = 0.1340 mm
```

For each layer the probe records:

- the current facet + flat path's areas and serial cost;
- canonical's classification operand `diff(raw_cur, safety_offset(raw_prev))`
  and `opening_ex(...)` cost;
- an **upper bound** on the three zones' role movement: the region area within
  `expansion_solid` of the canonical bridge (the solid/top reach dominates the
  sparse reach, so this one band bounds all three zones), computed against a
  wall-inset zone proxy `offset(polygons, -shell_width)`;
- a cost proxy for `wave_seeds` + `propagate_waves_ex` (the canonical step
  arithmetic, each step a round offset clipped to the zone proxy), the zone
  trim, and `merge_bridges`' closing.

Every proxy choice is one-sided in the direction that inflates the measured
area and cost: canonical clips each wave to one fill zone and starts it from
open seed paths; the probe clips to the whole wall-inset region and offsets
closed polygons. A failure of the **area** bound is therefore decisive. The
cost proxy is a conservative screen, not a proof — canonical's per-zone clipping
may be cheaper than the proxy — so the negative benchy net is a strong stop
signal (together with the stage mismatch), not a theorem about the real
implementation; a positive net would still need the campaign.

**Cross-take reproduction.** The probe's canonical area is **1,306.72 /
4,653.28 mm²** against t112's **1,307.30 / 4,654.62** (0.04 % / 0.03 %); its
flat family is **447.16 / 438.76 mm²** against t116's **447.27 / 438.92**
(0.02 % / 0.04 %). The production fidelity check passed on all six sampled
layers.

## 4. Area delta (both fixtures)

Committed layers only (layer 0 is skipped, as canonical demotes it to
`stBottom`). Areas in mm².

| fixture | facet polygon sum | flat family | current gated union | canonical bridge | added | removed | moved solid-zone bound | moved sparse-zone bound | closing measured | closing Steiner bound |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy | 14,232.05 | 447.16 | 706.73 | **1,306.72** | **618.44** | **18.45** | **300.64** | 88.35 | −1.32 | 2,225.80 |
| base | 419,106.22 | 438.76 | 4,473.93 | **4,653.28** | **564.17** | **384.82** | **513.85** | 89.11 | −9.86 | 7,503.81 |

- **Classification delta.** `added + removed` is **636.90 mm²** on benchy
  (**48.7 %** of the canonical bridge area) and **948.98 mm²** on base
  (**20.4 %**). The benchy bridge set grows **1.85x**; the base set is
  **1.04x**. The added area is canonical's unsupported-bottom area that the
  facet/flat split missed; the removed area is facet/offset material that is
  not actually unsupported.
- **Expansion reach.** The solid/top reach band is the total bound on all
  three zones because `expansion_solid` (1.2928 mm) contains
  `expansion_sparse` (0.35708 mm). It is **300.64 / 513.85 mm²**, of which the
  sparse-reachable subset is **88.35 / 89.11 mm²**. The whole-region proxy
  (before the wall inset) is much looser — **9,997.82 / 29,807.91 mm²** — and
  is disclosed only as an outer bound.
- **Closing.** `merge_bridges`' closing shrinks the union slightly on both
  fixtures (**−1.32 / −9.86 mm²**). The `perimeter × closing_radius` Steiner
  bound is **2,225.80 / 7,503.81 mm²**; it is a formal bound, not a prediction.
- **Combined role-movement bound.** `added + removed + moved_solid_zone
  + max(0, closing)` = **937.53 mm²** benchy / **1,462.83 mm²** base — **71.7 %
  / 31.4 %** of the canonical bridge area.

## 5. Cost and ceiling

All timings are **serial accumulated thread-time** in the probe. The prepass
and the per-layer stages run layers on rayon, so these are not wall claims; the
screen charges the added work at its full serial cost and credits the removed
work at its full thread-time. That is the conservative direction on both
sides. The candidate must still pass the paired wall gate.

| fixture | classify | facet | flat | current classification | canonical classification | expand solid zone | expand sparse zone | trim zone | closing zone | expansion only | classification-only net | combined net (tight) | combined net (whole-region) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy | 751.7 | 580.4 | 268.7 | **1,600.8** | **477.9** | 538.5 | 211.3 | 36.5 | 365.8 | **1,151.9** | **+1,122.9** | **−29.0** | −2,811.3 |
| base | 18,275.8 | 74,474.4 | 5,712.3 | **98,462.4** | **3,610.1** | 1,595.3 | 1,407.9 | 232.0 | 1,741.4 | **4,976.6** | **+94,852.4** | **+89,875.8** | +42,504.5 |

The decisive ratio is the expansion's cost against the classification saving:

- **benchy: 1,151.9 / 1,122.9 = 102.6 %.** The expansion consumes the entire
  classification win; the combined shape is a wash at best.
- **base: 4,976.6 / 94,852.4 = 5.2 %.** The classification win absorbs it with
  two orders of magnitude to spare.

## 6. Eight-cell ceiling

Floors are 0.80 % of the **t121 post-merge ordinary** walls (the current
scoreboard of record; the reducer re-derives each floor from the wall).
Multiples are the probe's net thread-time divided by the floor.

| cell | wall | floor | classification-only | combined (tight) |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | 5.73 s | 45.84 ms | **24.50x** | **−0.63x** |
| benchy classic-on | 10.64 s | 85.12 ms | **13.19x** | **−0.34x** |
| benchy arachne-off | 6.01 s | 48.08 ms | **23.36x** | **−0.60x** |
| benchy arachne-on | 11.26 s | 90.08 ms | **12.47x** | **−0.32x** |
| base classic-off | 56.08 s | 448.64 ms | **211.42x** | **200.33x** |
| base classic-on | 132.24 s | 1,057.92 ms | **89.66x** | **84.96x** |
| base arachne-off | 55.43 s | 443.44 ms | **213.90x** | **202.68x** |
| base arachne-on | 124.36 s | 994.88 ms | **95.34x** | **90.34x** |

**All eight clear only for the classification-only shape.** The combined
expansion shape is negative in all four benchy cells; `ceilings.json`
`all_eight_clear` is `false`.

The classic/arachne cells share a fixture's net because `PrePass::Slice` runs
before shell classification and perimeter generation, and its inputs are
generator-independent (t112's source argument; the probe's zone proxy is
likewise built from the region, not the generator). Only the cell wall differs.
The expansion half is a per-layer stage and its real cost may vary with the
perimeter generator; the probe does not claim otherwise, and the base margin
absorbs any plausible variation while the benchy sign does not.

## 7. Consumer map and what the delta can reach

`bridge_areas` consumers at HEAD (t112's map, re-checked):

| consumer | symbol | what it takes |
| --- | --- | --- |
| direction | `update_external_bridge_orientation` (`crates/slicer-core/src/algos/prepass_slice.rs`) | gated `bridge_areas` + raw lower contours |
| internal-bridge angles | `gate_internal_bridge_sites` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) | `bridge_areas` as `boundary_plines`; authors `internal_bridge_areas` |
| partition | `sync_perimeter_infill_areas_into_slice` (`crates/slicer-runtime/src/region_partition.rs`) | `bridge_final = bridge_areas ∩ perimeter.infill_areas`; `bridge > bottom > top > sparse` |
| paint splits | `paint_segmentation::split_regions` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) | intersects and re-saves `bridge_areas` |
| perimeter flags | `BridgeSpatialIndex` (`crates/slicer-core/src/perimeter_spatial.rs`) | per-vertex `is_bridge` containment |
| guests | `wave-overhangs`, `classic-perimeters`, `arachne-perimeters`, `gyroid-infill`, `rectilinear-infill`, `infill-linker` | `SliceRegionView::bridge_areas()` / `internal_bridge_areas()` |
| visual debug | `pnp-cli/src/visual_debug.rs` | serialises `bridge_areas` verbatim |

- **The expansion trims exactly three roles:** `top_solid_fill` (canonical
  `stTop`), `internal_solid_fill` (canonical `stInternalSolid`), and
  `sparse_infill_area` (canonical `stInternal`). It does not target
  `bottom_solid_fill` (canonical `stBottom`). The precedence partition already
  gives bridge priority over bottom where they overlap, but the expansion adds
  no bottom-specific movement.
- **The only consumer outside the four fill roles is the per-vertex
  `is_bridge` flag** on perimeters and supports. It is derived from
  `bridge_areas`, so its movement is confined to the boundary of the moved
  area. t116 measured this as `E`-token-only changes on unchanged `G1` moves.
- **Direction is a separate decision.** Canonical's `detect_bridge_directions`
  derives the angle from wave anchors; PnP's
  `update_external_bridge_orientation` derives it from the gated areas and the
  raw lower contours. If the candidate keeps PnP's path, the area bound above
  holds. If it ports canonical's, the angle can rotate lines **inside** the
  bridge polygons — same area, a full re-thread of those lines. That is a
  consumer-visible rewrite, bounded by the bridge area but not priced by an
  area delta, and it must be audited separately. This take does not price it.
- **No other consumer can move.** The expansion only changes which polygons
  carry `bridge`, which polygons carry the three trimmed roles, and the
  bridge-derived per-vertex flag and direction. Skirt, brim, raft, travel,
  support geometry, wall paths, and material volume are not inputs to this
  change.

## 8. Recommendation

**Do not build the combined shape; build the classification-only retirement.**

The classification-only candidate (graduated as
[ticket 123](../../issues/123-facet-bridge-path-retirement.md)) is:

```text
unsupported     = difference(raw_cur, offset(raw_prev, 1e-4, Square))
bridged         = opening_ex(intersection(unsupported, region.infill_areas),
                             0.1 × external_perimeter_width, Miter, 3.0)
region.bridge_areas = bridged
region.is_bridge    = !bridged.is_empty()
```

It deletes `classify_region_surfaces`, `assemble_bridge_areas`,
`assemble_flat_bridge_areas`, the now-redundant
`gate_bridge_areas_by_unsupported_span`, and the prepass consumption of
`expansion_margin_mm`. It keeps `update_external_bridge_orientation` and the
internal-bridge gate. It files a `DEV` row for the still-unported wave
expansion (the bridge anchors are not grown) — re-derive the next id from
`docs/DEVIATION_LOG.md` at the point of use.

Why this is not the forbidden "facade over the 1.0 mm offset": the offset is
**deleted**, not relabelled. The emitted set is canonical's classification
result; the missing expansion is recorded as a divergence, not disguised as
the expansion.

Why the expansion is deferred:

1. its cost on benchy is 102.6 % of the classification saving — no margin for
   the 0.80 % floor;
2. its zones do not exist at `PrePass::Slice`; it is a `Layer::Perimeters`
   partition change with its own per-layer transport budget (t121's lesson);
3. the direction port is a separate consumer-visible rewrite that this area
   screen does not cover.

Re-enter the expansion only with (a) a per-zone implementation at the
Perimeters commit, (b) a measured benchy expansion cost below roughly
1.1 s thread-time (the classification saving it must not consume), and (c) an
explicit decision on `detect_bridge_directions` versus PnP's existing
direction path.

## 9. Disclosures

- **Probe config.** The probe uses `ResolvedConfig::default()` (0.2 mm layers,
  0.4 mm line width, two walls) and the production layer-top schedule. It does
  not replay the full matched manifest, so the areas are the stage's on the
  matched layer schedule, not a cell-exact G-code account. The production
  fidelity check covers the reconstruction, not the manifest.
- **Thread-time, not wall.** The probe runs serially; production runs layers on
  rayon. The screen is not a wall claim. The candidate must pass the standing
  paired wall/CPU gate at 0.80 % in all eight cells.
- **The zone proxy is an upper bound, not a measurement of canonical's output.**
  It uses `offset(polygons, -shell_width)` as the union of the three zones and
  offsets closed polygons. Canonical uses three disjoint zones, per-boundary
  clipping, and open seed paths. Both proxy choices increase area and cost.
- **The whole-region proxy is much looser.** It is reported for transparency
  (benchy combined net −2,811.3 ms) but is not the ceiling of record; the
  wall-inset zone proxy is the tight bound this take uses.
- **Closing.** The cost is priced on the union of the canonical bridge and the
  solid reach band; canonical closes per bridge group. The measured area
  change is negative on both fixtures; the Steiner bound is formal and loose.
- **The t121 walls are ledger facts.** They are the ordinary-arm post-merge
  medians from `evidence/t121-per-layer-dispatch/SCOREBOARD.md`; the reducer
  recomputes every floor from them. The accelerated arm's walls differ by
  <2 % and are not used.
- **Run-to-run variance.** Two runs of the fixed probe gave benchy combined
  nets of **−73.6 ms** and **−29.0 ms**; the sign is stable, the magnitude is
  within ~50 ms. The base net moved by <1 % across runs. No CPU-load trace was
  recorded; the probe ran on the shared dev box with browser load. No claim
  rests on host quietness.
- **No production edit.** `probe.patch` adds only a temporary test target and
  test file; it was applied to measure and is not part of the delivered tree.
  `git status` on `crates/` and `modules/` is clean apart from the probe files.
- **The 0.80 % floor is a screen, not the destination.** Even the
  classification-only win does not close the destination's 64 %–80 % cut; it is
  a route step.

## Evidence index

- `probe.patch` — the temporary test target and probe, exactly as run.
- `reduce_t122.py` — `target/t122/{benchy,base}.csv` → `reduced/`.
- `reduced/summary-benchy.json`, `reduced/summary-base.json` — per-layer rows
  and aggregates.
- `reduced/ceilings.json` — the eight-cell floor multiples.
- `verify.py` — exits 0 against `reduced/` only; `--self-test` rejects **14/14**
  negative controls.
- Raw `target/t122/{benchy,base}.csv` — gitignored, produced by the probe.
