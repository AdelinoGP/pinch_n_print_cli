# Slicing-time contour simplification (`resolution`) port

Type: task
Label: wayfinder:task
Status: open
Assignee: unclaimed
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
all geometry work. The mesh slicer's `simplify_polygon_points`
(`crates/slicer-core/src/triangle_mesh_slicer.rs`) removes only exactly collinear
points and takes no tolerance.

**The PnP-side ceiling is unmeasured.** Orca's +40 % is Orca's sensitivity on
Orca's pipeline; PnP's geometry-heavy prepass and per-layer stages may respond
differently in either direction. This ticket's candidate must produce its own
paired A/B, not inherit a fraction of the Orca number.

## Work

1. **Parity definition first.** Derive from canonical what the simplified
   surfaces must be: `simplify_p` semantics (Douglas–Peucker variant, tolerance
   in scaled units, per-polygon and per-hole, start-vertex sensitivity) and the
   exact stage at which it applies relative to
   `PrePass::Slice` / `PrePass::ShellClassification` / the perimeter modules.
   Decide where in PnP's pipeline the equivalent belongs — a prepass stage
   (host-side, before module dispatch) or inside each perimeter/fill module —
   and state why.
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

_(unresolved)_
