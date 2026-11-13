# t148 — supports-on plane carve

Ticket: [Supports-on plane carve: align the planner's collision occupancy with the
print plane](../../issues/148-supports-on-plane-carve.md).

Take: 2026-11-13, ordinary arm, `target/release/pnp_cli.exe` built from this
branch, `RAYON_NUM_THREADS=12`, the standing matched-pair configs, 24 external
core modules. Both guest freshness gates exit 0 before the campaign.

## Verdict

**The supports-on output is no longer support-incomplete, and the zero-tolerance
exact-Z gate no longer drops whole bodies.** The planner now carves emitted roles
against the model occupancy at each layer's **print plane** (`layer.z`) — the
plane `validate_entry` queries at `SupportPlanEntry::anchor_z` — beside the
midpoint payload it already carved. On the four supports-on cells:

- `exact-Z occupancy` diagnostics fall from **101** (benchy) / **44,474** (base)
  to **0**;
- every run is `degraded: false`;
- the **33 dropped layers reappear** (32 base, 1 benchy) and **no support layer
  disappears**;
- support path rises **+294.871 mm** on both benchy cells and **+36,700.688 /
  +36,700.716 mm** on base classic / arachne;
- the four supports-off cells stay **byte-identical** to the t144 active set with
  zero diagnostics.

**The supports-on ratios of record are re-derived** and are now quotable as a
matched-settings claim (the previous ruling in
[t147](../../issues/147-supports-on-body-rejection-attribution.md) suspended
them): **2.705x / 2.829x / 3.356x / 3.248x** (benchy classic-on, benchy
arachne-on, base classic-on, base arachne-on), process CPU corroborating in every
cell. No causal wall-cost claim is taken from the cross-take delta — the window's
Orca arm moved too (see §5).

## 1. The change

The root cause was t147's: `layer_occupancy_payload` / `build_roles_batched`
(`modules/core-modules/tree-support-planner/src/lib.rs`) carved
`SupportAnalysisIR::model_occupancy`, which is sampled at the layer **midpoint**
(`layer_sample_plane_z`, canonical `compute_slice_z`), while `validate_entry`
(`crates/slicer-wasm-host/src/support_aggregation.rs`) queries exact-Z occupancy
at `anchor_z = layer.z`. The gate is zero-tolerance, so a sliver dropped the whole
body — and with it the only entry at that layer.

The fix publishes the validated plane and carves against it:

- `SupportAnalysisIR::print_plane_occupancy: Vec<SupportAnalysisPrintPlaneEntry>`
  (`crates/slicer-ir/src/slice_ir.rs`) — additive schema **2.1.0**, one
  object-wide cross-section per `(global layer, object)`. The host built-in
  `collect_print_plane_occupancy`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) batches one
  `slice_mesh_ex` pass per object over its print planes, quantized through
  `mm_to_units → units_to_mm` — the exact argument `validate_entry` passes to the
  query service, so the carve and the gate see the same plane. The WIT carrier is
  `print-plane-occupancy` on `support-analysis-view` (package
  `slicer:prepass-support-geometry@2.1.0`).
- `print_plane_occupancy_payload` + the emit loop of the tree planner chain the
  print-plane collision set into `collision_with_territory` and into
  `role_collision`, both inflated by `support_object_xy_distance` like the
  midpoint payload. `prefetch_occupancy_memo` batches the chunk's print-plane
  offsets beside the midpoint offsets, so no serial per-layer host call was added.
- The validators are untouched: `validate_entry` still queries the print plane
  with `SUPPORT_OVERLAP_TOLERANCE = 0`, and the carved result is what reaches it.

The midpoint payload is kept as the conservative extra the ticket allowed: output
is carved against both planes.

## 2. Correctness census

Support path is the t147 metric (`Support` + `Support interface` extrusion), so
the recovery is quoted on the same ruler that measured the loss.

| cell | diagnostics before → after | degraded | support path before → after (mm) | Δ (mm) | layers before → after | recovered | lost |
| --- | --- | --- | --- | ---: | --- | ---: | ---: |
| benchy classic-on | 101 → **0** | true → **false** | 82,644.645 → 82,939.516 | **+294.871** | 201 → 202 | 1 | 0 |
| benchy arachne-on | 101 → **0** | true → **false** | 82,644.645 → 82,939.516 | **+294.871** | 201 → 202 | 1 | 0 |
| base classic-on | 44,474 → **0** | true → **false** | 354,855.077 → 391,555.765 | **+36,700.688** | 400 → 432 | 32 | 0 |
| base arachne-on | 44,474 → **0** | true → **false** | 354,855.049 → 391,555.765 | **+36,700.716** | 400 → 432 | 32 | 0 |

Recovered planes:

- benchy: **L170** (z = 34.2), **293.608 mm** against t147's neighbour-interpolation
  estimate of 192.601 mm.
- base: **3.2, 3.8, 4.0, 5.6, 6.8, 7.0, 7.2, 7.4, 7.8, 8.0, 8.2, 8.4, 16.2, 20.0,
  26.6, 31.2, 31.4, 35.6, 38.6, 43.4, 43.8, 52.0, 53.4, 63.4, 66.0, 70.4, 70.6,
  72.6, 77.6, 79.2, 82.4, 83.0** — **36,723.3 mm** recovered against the t147
  estimate of 36,466.7 mm (+0.70 %). The recovered planes account for the whole
  path gain on base: the remaining 22.6 mm of the net delta is existing layers
  losing a sliver to the new print-plane carve.

The layer sets are exact, not just counts: `recovered_z` is the set difference of
the support-layer Z lists before and after, and `lost_z` is empty in every cell.
The arachne siblings recover the same planes as their classic siblings.

## 3. The candidate-coverage clause, read two ways

t147 recorded **20** candidates immediately above a dropped layer that Orca
supports at that plane (**19/32 base + 1/1 benchy**). Post-fix:

- **Plane presence: 20/20.** Every one of those planes now carries support
  (39.5–2,335.6 mm per plane on base, 293.6 mm on benchy). This is the reading
  the acceptance clause was written to express: the job no longer computes its
  wall ratio with the plane's support missing.
- **Polygon coverage (t147's > 5 % metric): 11/19 base, 0/1 benchy.** Measured,
  recorded in `census.json`, and **not** used as a pass gate, because the metric
  does not describe PnP tree support: the retained-layer control — candidates on
  layers the gate never dropped — reads the same 0.000–0.021 coverage. PnP's tree
  interface is placed under node footprints, not spread across the whole overhang
  polygon; Orca's traditional-style tree does spread. That is a pre-existing
  geometry-parity gap, outside this fairness fix, and the numbers are disclosed
  rather than dressed up.

The same clause's support-path half is met on the measured ruler: +36.7 m base,
+0.295 m benchy.

## 4. Supports-off identity

The four supports-off cells were re-sliced on this binary and are byte-identical
to the t144/t145 active set, with zero `exact-Z occupancy` diagnostics:

| cell | hash | matches active set |
| --- | --- | --- |
| benchy classic-off | `1ae0cfa8…` | yes |
| benchy arachne-off | `8faf02b1…` | yes |
| base classic-off | `c0b9014f…` | yes |
| base arachne-off | `17f63e93…` | yes |

The new active reference set is
[`reference.json`](reference.json): the four supports-off hashes carry forward,
the four supports-on hashes move, and t144/t145's supports-on set is recorded as
superseded (every supports-on ratio built on it is historical).

## 5. Re-derived ratios (ordinary arm, supports-on)

Matched pair, one window, 1 warmup + 3 measured runs per tool per cell, matched
keys asserted per run by the driver, load 9 % / 12 %.

| cell | Orca wall / CPU (s) | PnP wall / CPU (s) | ratio | t145 ratio (superseded set) |
| --- | --- | --- | ---: | ---: |
| benchy classic-on | 2.709 / 11.422 | 7.327 / 39.281 | **2.705x** | 2.683x |
| benchy arachne-on | 2.565 / 11.984 | 7.256 / 41.219 | **2.829x** | 2.580x |
| base classic-on | 21.598 / 95.047 | 72.487 / 588.656 | **3.356x** | 3.001x |
| base arachne-on | 22.341 / 103.875 | 72.570 / 592.828 | **3.248x** | 3.455x |

Every PnP support sample is one hash per cell across all four runs; CPU
corroborates the wall ordering in all four cells (PnP burns 3.4x–6.5x Orca's
CPU).

**Cost disclosure, not a claim.** The fix adds one batched print-plane mesh pass
per object and the print-plane offsets, and it prints the support the job always
demanded. PnP's process CPU rises visibly (base classic-on 492.7 → 588.7 s
median between t145's window and this one), while wall moved 70.26 → 72.49 s. The
cross-take wall delta is confounded: Orca's own medians moved in the same window
(base classic-on 23.414 → 21.598 s, base arachne-on 25.822 → 22.341 s), and
t145's base arachne-on PnP sample was deschedule-skewed (83.5/89.2/102.4 s at a
flat ~497 s CPU). No causal wall cost is claimed. There is no wall bar: this is a
fairness fix.

## 6. Disclosed gaps

- **No accelerated timing arm.** `cargo xtask dist --accelerated` still fails in
  `libmimalloc-sys` (`cxx` C1083, t142/t145 disclosure); a manual-cfg host is
  barred as a timing artifact by `docs/23_controlled_perimeter_builds.md`.
- **No Orca arm for the supports-off cells** in this take: they were not
  re-recorded, and their t145 ratios stand unchanged.
- **3 measured runs per cell**, one window; classic-cell output was stable this
  time (four identical hashes per cell, no DEV-093 jitter), disclosed because it
  is not guaranteed.
- **Coverage metric limitation** (§3): the polygonal coverage reading is not a
  valid PnP tree-interface criterion; the retained-layer control is recorded so
  the claim can be falsified.

## Evidence

- [`census.json`](census.json) — every number above: hashes, diagnostics,
  layers, per-plane paths, recovered planes, coverage rows, retained-layer
  control, wall medians.
- [`reference.json`](reference.json) — the active reference set.
- [`measure_support.py`](measure_support.py) — support-path/layer census.
- [`reduce_t148.py`](reduce_t148.py) — regenerates `census.json` from the raw
  (gitignored) tree; needs `shapely`, and imports t147's parser so both takes
  quote the same metric.
- [`verify.py`](verify.py) — **347/347 checks**, `--self-test` rejects **8/8**
  negative controls; stdlib-only, reads only `census.json`.
- [`REPRODUCE.md`](REPRODUCE.md) — exact commands.
