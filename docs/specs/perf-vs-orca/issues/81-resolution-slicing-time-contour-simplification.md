# Slicing-time contour simplification (`resolution`) port

Type: task
Label: wayfinder:task
Status: resolved (2026-10-23; human-approved KEEP — the port is committed.
Parity: invariants + exact stage confinement in all 8 cells. Speed: classic
cells double digits, Arachne wall-neutral, CPU down in every cell — see ## Answer)
Assignee: 2026-10-22/23 OpenCode wayfinder session — the human said "continue the
perf vs orca map" with no ticket named, so the session took the only frontier
ticket (81/81) in order
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Resolution-matched fairness premise](80-resolution-fairness-premise.md)

## Question

Can PnP implement OrcaSlicer's `resolution` — the slicing-time contour
simplification that runs **before** walls and infill are generated — faithfully
enough to match canonical geometry, and how much slice wall does it recover?

## Why this is not an output-preserving optimization

`resolution` changes geometry by design. Canonical consumes it on the sliced
surfaces (`PerimeterGenerator.cpp`'s `surface_simplify_resolution` →
`union_ex(surface.expolygon.simplify_p(...))`, the later
`ex.douglas_peucker(...)`, and `Fill.cpp`'s `resolution` read), so implementing it
produces different G-code than today — exactly as canonical does. The proof gate
therefore **cannot** be byte-identity against the preserved t58 references. It has
to be canonical parity: the simplified contours must match what OrcaSlicer's
Douglas–Peucker at the same tolerance produces on the same input, with the
established parity evidence standard.

Until that parity condition is defined, this ticket cannot be kept, only
measured.

## Measured basis

[Resolution-matched fairness premise](80-resolution-fairness-premise.md) measured
the key's worth on Orca's side at the matched job: running Orca at its canonical
0.001 mm floor instead of the matched 0.01 mm costs it **+9.5 % to +65.2 % wall**
(median **+40.2 %**, CPU **+21 % to +92 %**) across all eight cells
([evidence](../evidence/t80-resolution-fairness/FINDINGS.md)). PnP implements no
slicing-time equivalent, so that work is entirely absent from PnP today.

PnP's existing tolerances are emission-time only
(`gcode_resolution` 0.0125 / `infill_resolution` 0.04 / `support_resolution`
0.0375 in `crates/slicer-gcode/src/serialize.rs`, fed from
`crates/slicer-ir/src/resolved_config.rs`) and cannot substitute: they run after
all geometry work. Note their defaults are **exactly** Orca's internal per-role
constants (`RESOLUTION = 0.0125`, `SPARSE_INFILL_RESOLUTION = 0.04`,
`SUPPORT_RESOLUTION = 0.0375` in `libslic3r.h`), so that half is faithful porting
of a different mechanism — do not "correct" those values while adding
`resolution`. The mesh slicer's `simplify_polygon_points`
(`crates/slicer-core/src/triangle_mesh_slicer.rs`) removes only exactly collinear
points and takes no tolerance.

**The PnP-side ceiling is unmeasured.** Orca's +40 % is Orca's sensitivity on
Orca's pipeline; PnP's geometry-heavy prepass and per-layer stages may respond
differently in either direction. This ticket's candidate must produce its own
paired A/B, not inherit a fraction of the Orca number.

## Work

1. **Parity definition first.** Derive from canonical what the simplified
   surfaces must be. The canonical call is `ExPolygon::simplify_p(tolerance)`
   (`ExPolygon.cpp`): it runs `MultiPoint::_douglas_peucker` on the contour and
   then on **each hole independently**, and it closes the ring before the pass
   (`p.points.push_back(p.points.front())` … `p.points.pop_back()`), so the
   simplification is per-ring and wrap-around-sensitive. `PerimeterGenerator`'s
   tolerance is `m_scaled_resolution`, initialised as
   `scaled<double>(print_config->resolution.value > EPSILON ? value : EPSILON)`;
   `scale_` divides by `SCALING_FACTOR` (`1e-6`, i.e. 10⁶ units per mm) while
   PnP's unit is 100 per mm, so the tolerance must be converted, not copied.
   Decide where in PnP's pipeline the equivalent belongs — a prepass stage
   (host-side, before module dispatch) or inside each perimeter/fill module — and
   state why. Also decide whether PnP's existing
   `simplify_polygon_points` (collinear-only) stays as a separate pre-pass.
2. **Key and default.** Add the `resolution` config key with canonical default
   `0.01` and canonical clamp (`DynamicPrintConfig::normalize_fdm` raises values
   below `0.001` to `0.001`).
   Re-derive how the existing appendix alias `("resolution", "gcode_resolution")`
   is affected: with a real key, the alias should stop being a disclosure lie.
   Coordinate with the feature-gap tracker's alias map (ticket 07) and its
   `03-asset-scoped-gap.md` adjudication, which currently treats the per-domain
   emission keys as the counterpart.
3. **Parity evidence.** Build the comparison on matched-job layers: canonical
   `simplify_p` at 0.01 mm versus the PnP implementation, per layer, on both
   fixtures. The output identity requirement is parity with canonical, not with
   the old references.
4. **Paired A/B.** Standing gate, ordinary + accelerated, on the pre-declared
   cells. Because this is a geometry change, the proof matrix must be rebuilt
   against canonical-parity output rather than the t58 references, and the
   scoreboard's output disclosure must show the new byte/`;TYPE:` counts.
5. **Ordering note.** At `resolution` 0.01 the contours handed to every
   downstream stage change, so this candidate interacts with all previously
   measured stage attributions (t73's base split, t75/t76/t78's keeps). Re-derive
   any attribution it touches rather than quoting the older shares.

## Limits to state up front

- The Orca-side measurement prices one side only; PnP's recovered wall is
  unmeasured until this ticket's own A/B.
- PnP's infill/support emission tolerances are **coarser** than Orca's 0.01 and
  run the opposite way; a `resolution` port does not net them.
- Orca's clamp means "no simplification" cannot be measured even canonically; the
  0.001 floor understates the key's full worth.
- A geometry-affecting keep in this map is a precedent change: every subsequent
  output-identity gate must name which reference regime it compares against.

## Answer

**Resolved 2026-10-23 — human-approved KEEP. The port is committed.**

### Parity condition (work item 1)

Derived from the canonical checkout, then implemented once in
`crates/slicer-sdk/src/contour_resolution.rs`:

- `ExPolygon::simplify_p` closes each ring before `MultiPoint::_douglas_peucker`
  (push first point, simplify, pop), runs the contour and **each hole
  independently**, then `simplify_polygons` re-unions the rings with NonZero.
- The DP is reproduced exactly: iterative stack, `<=` acceptance, strictly
  greater furthest-index tie-break, and the seeded degenerate segment at the
  ring's first vertex (which is what makes the first split land on the vertex
  furthest from the ring start).
- Units are converted, never copied: canonical 1e-6 mm (`SCALING_FACTOR`, 10 000
  units for 0.01 mm) versus PnP's 100 nm unit (100 units for 0.01 mm).
- The `normalize_fdm` floor is applied **in the resolver**
  (`crates/slicer-config/src/resolution.rs`), so the G-code appendix discloses
  the value the modules apply; "disabled" stays inexpressible, as canonical
  requires.
- The key rides the `ResolvedConfig::extensions` carrier, not a fixed typed
  field: `RegionMapIR` interns `ResolvedConfig`, so a typed field would change
  the Postcard layout and break the frozen 3.0.0 fixture (the constraint that
  reverted the packet-10 fixed-field addition). `ResolvedConfig::resolution()`
  reads it, with the canonical default when absent. The extension refactor
  landed after the campaign; both fixtures reproduce the measured G-code bytes
  exactly (`3672dc7e…` benchy classic-off, `292df501…` base classic-off), so the
  timing numbers describe the committed source.

Placement follows canonical's call sites rather than one host pass. PnP has no
`Surface` decomposition at the perimeter stage, so site 1 runs at region
granularity (contour + holes) at the same point in the wall pass:

| Canonical site | PnP seam |
|---|---|
| pre-wall `union_ex(surface.expolygon.simplify_p(...))` (classic + arachne) | perimeter module, before wall generation |
| gap `douglas_peucker` before `medial_axis` | classic `filtered_gaps` loop |
| classic innermost contour → infill boundary | before the infill-boundary offset |
| classic inner-part pass | **no PnP counterpart — recorded, not invented** |
| arachne `infill_contour` | `publish_infill_areas` |

The `0.2 ×` arc-fitting tolerance branch is unexpressible (PnP has no
`enable_arc_fitting`) and inert on the matched job (appendix-verified
`enable_arc_fitting = 0` in t80's captures).

### Parity evidence (work item 3)

The repo's Parity Evidence Standard is invariants, not goldens, and no Orca test
covers `simplify_p`/`_douglas_peucker`, so there was nothing to port. The
invariant suite pins canonical's actual guarantee (every removed vertex within
tolerance of the output segment that replaced it), subsequence/anchoring,
corner survival, monotonicity, no re-introduction, ring closure, first-split
seeding, hole topology + exact area, and the unit/floor conversion.

Disclosed caveat: canonical's `simplify_polygons` runs on **legacy Clipper 6.4.2**
(`StrictlySimple(true)` + NonZero), while PnP unions through `clipper2-rust`
(NonZero, no strictly-simple pass). Same set semantics, different
representation; byte-identical output is not claimable and is not claimed.

### Stage confinement

`evidence/t81-resolution-port/confinement.py` compares typed IR captures
exactly: **20/20 PASS** across all eight cells — `Layer::Slice` identical in all
eight, `Layer::Perimeters` differs in all eight, and the
`PrePass::SupportGeometry` silhouette is byte-identical in all four supports-on
cells (raster evidence, disclosed as such because that tap has no typed
capture). At 1024² the perimeter PNGs are visually indistinguishable between
arms, which is why the exact IR is the evidence and the image is only an
artifact.

### Paired A/B (work item 4)

Pre-declared: wall < 0 and CPU ≤ +3.0 % in all eight cells, 6 alternating pairs,
stop by rule at the first failing cell. **Gate met on the campaign of record:**

| cell | n | wall | CPU | bytes |
|---|---:|---:|---:|---:|
| benchy classic-off | 6 | **−14.31 %** | −23.12 % | −9.40 % |
| benchy classic-on | 6 | **−9.00 %** | −18.77 % | −4.69 % |
| benchy arachne-off | 6 | −0.07 % | −3.16 % | −1.51 % |
| benchy arachne-on | 6 | −1.87 % | −1.80 % | −0.74 % |
| base classic-off | 6 | **−22.24 %** | −26.87 % | −6.26 % |
| base classic-on | 6 | **−10.87 %** | −20.29 % | −3.20 % |
| base arachne-off | 12 | −1.07 % | −2.08 % | −1.58 % |
| base arachne-on | 6 | −0.88 % | −1.44 % | −0.78 % |

`base arachne-off` failed its first six pairs at **+2.30 %** and stopped the
campaign by rule; the human's explicit "run it again" re-measured it at
**−2.77 %** (combined n=12 median −1.07 %, spread −5.57 … +29.71). That cell is
**wall-neutral within measurement resolution** — the spread, not the sign, is
the finding, and one of its twelve samples is visibly disturbed. The other three
Arachne cells are likewise inside their own spread. Classic is the real speed
case: −9 to −22 % wall, −18 to −27 % CPU.

Every cell produced exactly one G-code SHA across all its runs (deterministic),
no sample was starved by the recipe's criterion (cpu/wall 7.57–8.30 against the
~6.5 clean 12-thread floor), and the baseline arm reproduces the standing t58
classic-off reference `34886ccb…` exactly.

### Reference regime (work item 4)

The byte-identity regime cannot survive a geometry change. Replaced by: invariants
+ stage confinement + determinism, with per-cell candidate SHAs recorded in
`evidence/t81-resolution-port/campaign/*.csv` as the new reference set. Any later
take comparing against the old t58 references for this job is comparing against a
superseded regime and must say so.

### Ordering note (work item 5)

Attributions that predate this port describe the unsimplified pipeline. t73's base
split, t75/t76/t78's keeps and the t72 scoreboard were all measured on
full-detail contours; do not quote them as current without re-deriving.

### Limits, as stated up front

- Orca's +9.5–65.2 % is Orca's sensitivity. PnP's own response was measured here
  and differs by generator: classic large, Arachne ~nil.
- PnP's emission-side tolerances (`gcode_resolution` 0.0125 / `infill_resolution`
  0.04 / `support_resolution` 0.0375) are Orca's own per-role constants and do
  not net against `resolution`; they run after all geometry work.
- Canonical's clamp means "no simplification" cannot be measured; the 0.001 mm
  floor is not the same experiment as "off".
- Geometry-affecting keep: every later output-identity gate must name its
  reference regime.

Evidence: [`evidence/t81-resolution-port/`](../evidence/t81-resolution-port/FINDINGS.md)
(`confinement.py` 20/20, `run_port_pair.ps1`, `campaign/`, `corrected_scoreboard.py`).
