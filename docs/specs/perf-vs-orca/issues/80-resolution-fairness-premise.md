# Resolution-matched fairness premise for the matched job

Type: task
Label: wayfinder:task
Status: resolved (2026-10-22; measurement-only — the job was **not**
resolution-matched: Orca's matched `resolution = 0.01` is worth
**+9.5 % to +65.2 %** of its own wall, median **+40.2 %**; disclosure recorded
and one implementation candidate graduated — see ## Answer)
Assignee: 2026-10-22 OpenCode wayfinder session — the human challenged the
fairness contract on resume ("has this study been running on the same
resolution as Orca does? The resolution defaults for PNP are higher"), which
outranks the open route question; the frontier was empty (79/79 resolved)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none

## Question

The matched job's fairness contract enumerates what is matched
(nozzle, layer heights, wall count, sparse infill, wall generator, supports) and
what is "uncontrolled by design (disclosed, not matched)" — but it never pins
OrcaSlicer's `resolution`, and PnP implements no slicing-time equivalent. Does
that asymmetry (a) break the contract as disclosed, and (b) carry enough wall to
matter against a 3.3x–18.0x gap?

## Premise (measured this session, before any timing)

**Canonical.** `PrintConfig.cpp`'s `resolution` entry is `coFloat`, default
**0.01 mm**, tooltip: "The G-code path is generated after simplifying the
contour of models to avoid too many points and G-code lines. Smaller values mean
higher resolution and more time required to slice." The BBL vendor process
profile used by the rig (`process/0.20mm Standard @BBL X1C.json`) does **not**
set the key, so the matched Orca arm inherits 0.01.

**It precedes the geometry work, not just emission.** Canonical consumption
sites (all on the sliced surfaces, before wall/infill generation):
`PerimeterGenerator.cpp`'s `surface_simplify_resolution` feeding
`union_ex(surface.expolygon.simplify_p(...))` and the later
`ex.douglas_peucker(surface_simplify_resolution)`; `Fill.cpp`'s `resolution`
read in the fill path. Emit-side sites exist too
(`Layer::simplify_support_path`, `LayerRegion`'s path simplify), but the
slicing-time surface simplification is the one that changes how much geometry
every downstream stage must process.

**PnP has no counterpart.** `docs/ORCA_CONFIG_REFERENCE.md` marks `resolution`
as not implemented (❌). PnP's nearest keys are emission-time per-role
Douglas–Peucker tolerances in `crates/slicer-gcode/src/serialize.rs`
(`gcode_resolution` 0.0125, `infill_resolution` 0.04, `support_resolution`
0.0375, from `crates/slicer-ir/src/resolved_config.rs` and
`docs/config/host-keys.toml`) — a different stage, coarser than 0.01 on
infill/support. The mesh slicer's `simplify_polygon_points`
(`crates/slicer-core/src/triangle_mesh_slicer.rs`) removes only exactly
collinear points; it takes no tolerance.

**The only prior adjudication was disclosure.** The appendix alias
`("resolution", "gcode_resolution")` in `crates/slicer-gcode/src/serialize.rs`
documents "the adjudicated PnP counterpart" and records the default divergence
(0.0125 vs 0.01), and
[config appendix resolved settings](26-config-appendix-resolved-settings.md)
states it is "appendix-only" and "does not change `ResolvedConfig::to_config_map`".
No ticket, map note or scoreboard disclosure ever treated the *work* asymmetry.

**Direction.** The unmatched key hands **Orca** the advantage: Orca simplifies
the contours it generates walls and infill from, PnP does not. PnP's coarser
infill/support emission tolerance runs the other way, so the net is not
obvious and must be measured, not asserted.

## Work

1. Price Orca's side in Orca's own currency: run the matched Orca profile with
   `resolution = 0.01` (as run) against the canonical floor `resolution = 0.001`
   (the finest expressible arm — see "Canonical correction" below), alternating
   pairs, uninstrumented, on the classic cells the map has been landing keeps
   on. Wall = process creation-to-exit; CPU = `GetProcessTimes` kernel+user, the
   ticket-11 rig's exact measurement math. Verify each arm's effective value from
   the `; resolution = ` line in the G-code appendix — a profile override that
   silently did not apply would otherwise read as "no effect".
2. Disclose output stats per arm (bytes, `; FEATURE:` counts). This key changes
   geometry by design, so output identity is **not** expected and must not be
   used as a validity gate; the validity gate is the appendix value.
3. Pre-declared decision rule (fixed before the campaign):
   - paired median delta **< 3 % in every cell** → the asymmetry is real but
     immaterial against a ≥ 3.3x gap: record the disclosure in the map and the
     scoreboard's uncontrolled list, no route change, no implementation ticket.
   - **≥ 3 % in any cell** → graduate the disclosure fix, and an implementation
     ticket for a faithful `resolution` port if the delta is large enough to
     move a cell's ratio materially. A port is canonical parity work (it changes
     geometry exactly as canonical does), not an output-preserving optimization.
4. State the limit plainly: this prices Orca's side only. PnP's coarser
   infill/support emission tolerance is the opposite sign and is not netted out,
   so the measurement bounds the asymmetry rather than settling the true
   work-parity gap.

## Canonical correction during the take

`resolution = 0` cannot be measured as "disabled": `DynamicPrintConfig::normalize_fdm`
clamps any value below **0.001 mm** up to 0.001, so OrcaSlicer always
simplifies and the finest expressible arm is the 0.001 floor (10x finer than the
matched 0.01). The campaign therefore pairs `default` (inherits 0.01) against
`floor` (explicit 0.001), which bounds the simplification's worth from below
rather than removing it. A smoke run of the `0` arm measured the appendix back as
`0.001`, which is how the clamp was found.

## Answer

**Resolved 2026-10-22: the study has not been resolution-matched, and the
asymmetry is large.** The matched Orca profile inherits `resolution = 0.01 mm`
from the vendor process profile and applies it as a **slicing-time contour
simplification** before walls and infill are generated; PnP implements no
equivalent (its `gcode_resolution` / `infill_resolution` / `support_resolution`
are emission-time only). Running the same profile at Orca's canonical 0.001 mm
floor (the clamp makes "off" inexpressible) costs Orca **+9.5 % to +65.2 % wall**
across the eight cells — median **+40.2 %**, CPU **+21 % to +92 %** — with
+9 % to +91 % more G-code, in 3 alternating pairs per cell with every run's
appendix-verified `resolution`.

Per cell (wall / CPU): benchy classic-off **+58.4 % / +59.2 %**, classic-on
**+65.2 % / +70.7 %**, arachne-off **+32.1 % / +21.1 %**, arachne-on
**+47.3 % / +44.1 %**; base classic-off **+37.5 % / +68.6 %**, classic-on
**+42.8 % / +92.0 %**, arachne-off **+9.5 % / +46.3 %**, arachne-on
**+30.9 % / +64.7 %**. `base/arachne-off`'s wall is noise-limited (floor samples
32.50–41.81 s, one pair reversing sign) while its CPU is stable; its wall
transfer is the one figure this campaign does not settle.

Consequences: (1) the disclosure is recorded on the map's fairness contract —
every ratio this map has published compares PnP at full contour detail against
an Orca doing materially less geometric work; (2) a faithful port is graduated as
[81-resolution-slicing-time-contour-simplification](81-resolution-slicing-time-contour-simplification.md),
explicitly as **canonical parity work**, not an output-preserving optimization;
(3) `docs/specs/orca-feature-gap/issues/03-asset-scoped-gap.md`'s
"per-domain split keys are the counterpart of `resolution`" adjudication covers
the emission half only and needs re-deriving there.

No production edit, no PnP build, no candidate, no scoreboard revision. Evidence:
[`evidence/t80-resolution-fairness/`](../evidence/t80-resolution-fairness/FINDINGS.md)
(`verify.py` exits 0 checkout-only and rejects 5/5 negative controls).
