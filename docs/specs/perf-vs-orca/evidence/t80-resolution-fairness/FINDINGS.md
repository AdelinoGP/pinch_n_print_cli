# t80 — Resolution fairness premise for the matched job

Ticket: [80-resolution-fairness-premise](../../issues/80-resolution-fairness-premise.md)

Measurement-only. No production edit, no PnP build, no candidate. Two
OrcaSlicer arms on the matched job, differing only in `resolution`, priced with
the ticket-11 rig's measurement math and validated from Orca's own G-code config
appendix.

## What the key is, canonically

`PrintConfig.cpp` registers `resolution` as `coFloat`, default **0.01 mm**:

> The G-code path is generated after simplifying the contour of models to avoid
> too many points and G-code lines. Smaller values mean higher resolution and
> more time required to slice.

The vendor process profile the matched job inherits
(`process/0.20mm Standard @BBL X1C.json`) does **not** set the key, so the
matched Orca arm has always run 0.01 mm.

The clamp is a canonical detail that bounds any experiment:
`DynamicPrintConfig::normalize_fdm` / `normalize_fdm_1` clamp any `resolution` below
**0.001 mm** up to 0.001, so
"simplification disabled" is **not expressible** in OrcaSlicer. The finest arm
this take can run is therefore 0.001 mm.

### It is a slicing-time pass, not an emission tolerance

Canonical consumption sites that precede wall and infill generation:

| site | what it simplifies |
| --- | --- |
| `PerimeterGenerator`'s `m_scaled_resolution` → `surface.expolygon.simplify_p(...)` / `ex.simplify_p(m_scaled_resolution, &pp)` | the sliced surfaces each island's perimeter loops are generated from, and the infill contours (`ExPolygon::simplify_p` → `MultiPoint::_douglas_peucker`) |
| `PerimeterGenerator`'s second `ex.douglas_peucker(surface_simplify_resolution)` | infill contours |
| `Fill.cpp`'s `resolution` read | the fill path |
| `Layer::simplify_support_path` / `LayerRegion`'s path simplify | emitted support and region paths |

`PerimeterGenerator`'s member is initialised as
`scaled<double>(print_config->resolution.value > EPSILON ? ... : EPSILON)`, and
`scale_` divides by `SCALING_FACTOR` (`1e-6`), i.e. Orca's internal unit is
10⁶ per mm. PnP's unit is 100 per mm, so a port must convert the tolerance.

The first two rows are the ones that change how much geometry every downstream
stage must process. The emit-side rows are a separate, smaller effect.

## What PnP does instead

PnP has **no** `resolution` implementation (`docs/ORCA_CONFIG_REFERENCE.md` marks
it ❌). Its nearest keys are emission-time per-role Douglas–Peucker tolerances:

| PnP key | default | applied at | canonical origin |
| --- | ---: | --- | --- |
| `gcode_resolution` | 0.0125 mm | `slicer-gcode` emission, perimeter/gap/brim/raft roles | Orca's internal `RESOLUTION` constant |
| `infill_resolution` | 0.04 mm | emission, infill roles | Orca's internal `SPARSE_INFILL_RESOLUTION` constant |
| `support_resolution` | 0.0375 mm | emission, support roles | Orca's internal `SUPPORT_RESOLUTION` constant |

Those three defaults are **exactly** Orca's own per-role constants
(`libslic3r.h`: `RESOLUTION = 0.0125`, `SPARSE_INFILL_RESOLUTION = 0.04`,
`SUPPORT_RESOLUTION = 0.0375`), so PnP's emission split is a faithful port of a
*different* mechanism, not a mis-valued copy of the config key. The mesh slicer's
`simplify_polygon_points` (`crates/slicer-core/src/triangle_mesh_slicer.rs`)
removes only exactly collinear points and takes no tolerance.

So the two sides are not the same mechanism at a different value: the config key
`resolution` (0.01) is what Orca applies to the **surfaces it generates geometry
from**; PnP has no key there at all, and simplifies only **paths it is about to
write**, at Orca's constants.

### What the trackers believed

`docs/specs/orca-feature-gap/issues/03-asset-scoped-gap.md` records
`min_segment_length`, `infill_resolution`, `support_resolution`,
`perimeter_arc_tolerance` as "Pinch splits path simplification per-domain; Orca
has one global `resolution` (already mapped above)" — i.e. the split keys were
treated as the counterpart. That adjudication covers the **emission** half only.

`crates/slicer-gcode/src/serialize.rs`'s appendix alias `("resolution",
"gcode_resolution")` is likewise disclosure-only, and
[ticket 26](../../issues/26-config-appendix-resolved-settings.md) states it does
not change `ResolvedConfig::to_config_map`. Neither record prices the
slicing-time work.

## Measurement

Four campaigns (two fixtures × two generator families), 3 alternating pairs per
cell plus one warmup per arm, `resolution` verified from each run's G-code
appendix. Deltas are `(floor − default) / default`, so a positive delta is what
Orca gains from its slicing-time simplification.

| cell | default wall | floor wall | wall Δ | CPU Δ | bytes Δ |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 5.72 s | 8.95 s | **+58.4 %** | +59.2 % | +66.6 % |
| benchy classic-on | 6.25 s | 10.43 s | **+65.2 %** | +70.7 % | +46.0 % |
| benchy arachne-off | 5.97 s | 7.88 s | **+32.1 %** | +21.1 % | +9.0 % |
| benchy arachne-on | 7.05 s | 10.17 s | **+47.3 %** | +44.1 % | +20.0 % |
| base classic-off | 32.79 s | 44.81 s | **+37.5 %** | +68.6 % | +90.8 % |
| base classic-on | 43.02 s | 60.42 s | **+42.8 %** | +92.0 % | +67.7 % |
| base arachne-off | 33.90 s | 37.13 s | **+9.5 %** | +46.3 % | +14.7 % |
| base arachne-on | 43.13 s | 55.90 s | **+30.9 %** | +64.7 % | +26.5 % |

Median wall delta across the eight cells: **+40.2 %**. The effect is positive in
every cell and in every one of the 24 measured pairs' CPU deltas.

**Stability.** Seven of eight cells have tight wall ranges (span ≤ 9 %) and
consistent pair signs. `base/arachne-off` is the exception: its floor-arm wall
samples span 32.50–41.81 s (one pair even reverses sign at −5.2 %) while its
CPU is stable at 71.2–75.2 s, **+46.3 %** against a stable 49.5–49.9 s default.
Read that cell's wall median as noise-limited and its CPU as the stable signal —
the mechanism is still present, its wall transfer on that one cell is not
resolved by this campaign.

**Cross-check that the arms reproduce the matched job.** The `default` arm's
benchy classic-off G-code is 3,890,403 bytes, against the 3,890,416 bytes the
ticket-28 census recorded for the historical matched Orca capture (−13 bytes,
0.0003 %); Orca's own output varies run to run (base classic-on default samples
span 37.41–37.47 MB), so output identity is not a gate here and was not used as
one.

## Verdict

**The study has not been resolution-matched, and the asymmetry is large.**
Against the pre-declared rule (≥ 3 % in any cell), this take graduates two
follow-ups and records a disclosure:

1. **Disclosure (map + fairness contract).** `resolution` joins the matched
   job's "uncontrolled by design (disclosed, not matched)" list, with the
   measured price: the matched Orca profile simplifies contours at 0.01 mm
   before generating walls and infill, and running it at the canonical 0.001 mm
   floor instead costs Orca **+9.5 % to +65.2 % wall** (median +40.2 %, CPU
   +21 % to +92 %). PnP does no slicing-time simplification at all, so every
   ratio this map has published compares PnP at full contour detail against an
   Orca that does materially less geometric work.
2. **Implementation candidate.** A faithful `resolution` port is graduated as
   [81-resolution-slicing-time-contour-simplification](../../issues/81-resolution-slicing-time-contour-simplification.md).
   It is **canonical parity work with a performance consequence**, not an
   output-preserving optimization: implementing it changes geometry exactly as
   canonical does, so its proof gate is Orca-equivalent simplification, not
   byte-identity to the t58 references. The measurement above prices Orca's side
   only; PnP's recovered wall is unmeasured and needs that candidate's own A/B.

Arithmetic caution for the map's ratios (not a measurement): if the current
Orca wall were raised by a cell's measured factor, that cell's ratio would fall
by the same factor — e.g. +58.4 % on benchy classic-off would turn a 5.14x ratio
into 5.14 / 1.584 ≈ 3.25x. Those t72-era ratios are stale by five keeps, so this
is a correction factor to apply to whatever today's ratio is, not a new
scoreboard.

**Tracker correction.** `docs/specs/orca-feature-gap/issues/03-asset-scoped-gap.md`
records `min_segment_length`, `infill_resolution`, `support_resolution`,
`perimeter_arc_tolerance` as the per-domain counterpart of Orca's `resolution`.
That holds for **emission** only. The slicing-time half
(`PerimeterGenerator`'s `surface_simplify_resolution` → `simplify_p`,
`Fill.cpp`) has no PnP counterpart, which is the half that changes how much
geometry work happens. The feature-gap adjudication needs re-deriving; this take
does not edit that tracker.

## Limits

- This prices **Orca's** sensitivity to its own key. It does not measure what a
  faithful PnP implementation would recover; PnP's pipeline differs and the
  ceiling needs its own candidate.
- PnP's emission path simplification runs at Orca's own per-role constants, which
  are **coarser than the config key's 0.01** for infill (0.04) and support
  (0.0375) and slightly coarser for perimeters (0.0125). At emission PnP
  therefore removes *more* points than Orca does at 0.01 — the opposite sign to
  the missing slicing-time pass. The two are not netted out here, so this bounds
  one side of the asymmetry rather than settling net work parity.
- Orca's `resolution` also drives emit-time path simplification when arc fitting
  is off (`Layer`, `LayerRegion` use `scaled(print_config.resolution.value)`;
  with arc fitting on they switch to `SCALED_SUPPORT_RESOLUTION` /
  `SCALED_SPARSE_INFILL_RESOLUTION` and the surface tolerance becomes `0.2 ×`).
  The measured delta therefore mixes the slicing-time and emit-time uses of the
  key; this take does not split them.
- The floor arm (0.001 mm) is not "no simplification": the canonical clamp
  prevents that. The measured delta is therefore a **lower bound** on what full
  contour detail costs Orca.
- Output geometry differs between arms by design (this key changes geometry).
  Output identity is not a validity gate; the appendix value is.
