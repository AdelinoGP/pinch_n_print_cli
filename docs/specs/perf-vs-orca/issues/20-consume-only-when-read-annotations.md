# Consume-only-when-read annotations

Type: task
Status: open
Assignee: unassigned — routing claim 2026-10-08 resolved into the read-only
premise audit below; awaiting the human's route decision (the below-fold park in
[Gap budget per cell](12-gap-budget-per-cell.md) remains in force)
Blocked by: 12, 18, 19

## Question

Design and measure consume-only-when-read per-vertex annotations in
`expolygon_to_path3d_indexed` (`crates/slicer-core/src/perimeter_spatial.rs`) —
compute each per-vertex quantity (`overhang_quartile`,
`signed_distance_to_boundary`, `is_bridge`) only when a downstream consumer
actually reads it. This is lead 3 of the
[emit_walls premise falsified](08-emit-walls-premise-falsified.md) ranking,
which the mode-comparative addendum **raised in weight**: with acceleration
necessary-but-not-sufficient, avoiding work the consumer never reads compounds
with whatever the query path eventually costs.

Scoping comes first: [Accelerated residual query
attribution](18-accelerated-residual-query-attribution.md) determines what is
still worth avoiding once accelerated queries and fallback suppression are
accounted for — the candidate is that remainder, not the ordinary-mode 69.8%.

Constraints: annotations must be value-identical wherever they are read
(output preserved exactly); the change is guest-side in the classic perimeter
path (and `PerimeterSpatialContext::is_bridge`'s scan belongs in the same
look-before-you-read design if the split says so).

Acceptance: paired ordinary + accelerated A/B per the standing metric, then
keep/drop to the human. Timing chain position 5.

## Comments

### Routing claim and premise audit — 2026-10-08 (read-only; no take started)

The session took this ticket as the first open, unblocked child on resumption
(t19 closed 2026-10-08 with a KEEP). Per the map's convention the claim is a
routing claim only: the below-fold park in
[Gap budget per cell](12-gap-budget-per-cell.md) is **not** lifted (t19's lift
was explicitly candidate-scoped), so no implementation, experiment or timing
campaign was started. The claim resolved into the following premise audit at
HEAD `b56f429a` (t19's KEEP commit); every fact below is code/config-derived —
no slice was run.

**The premise survives and sharpens: on the matched job the only semantic
consumer of the two stamps returns before reading them, so the
`overhang_quartile` / `overhang_distance_mm` stamping is unread work there.**

1. **Consumer audit.** The only production site that makes a *decision* on
   `Point3WithWidth::overhang_quartile` / `overhang_distance_mm` is
   `OverhangClassifierDefault::run_finalization`
   (`modules/core-modules/overhang-classifier-default/src/lib.rs`): it reads
   `point.overhang_quartile.is_some()` and `point.overhang_distance_mm` to build
   per-point speed factors. Every other referencing site transports or
   null-fills: `seam-placer`'s `interpolate_point` copies whichever side of `t`
   it is on; `seam-planner-default` writes `None`; `fuzzy-skin` nulls;
   `wave-overhangs`, `part-cooling`, `path-optimization-default`,
   `machine-gcode-emit` and `crates/slicer-gcode` never read them; the
   `slicer-wasm-host` marshal sites are transports; tests read.
   `WallFeatureFlags::is_bridge` sits differently: classic's `build_ring_wall`
   drives `bridging_flow` through it into `flow_factor`, which
   `crates/slicer-gcode` consumes for `E`, so it cannot be dropped as unread
   while any wall vertex is a bridge vertex; whether the matched job carries one
   is a take-time measurement. Classic also lacks arachne's
   `!bridge_areas.is_empty()` empty-input guard around the scan (arachne guards,
   classic does not — a parity-shaped observation, not yet a candidate).

2. **The liveness gate.** `run_finalization` early-returns twice: on
   `enable_overhang_speed == false` (manifest default **true**) and then on all
   four `overhang_1_4_speed` … `overhang_4_4_speed` bands at 0 (manifest
   defaults **0.0**). None of the four matched configs
   (`evidence/matched-pair/configs/pnp-{classic,arachne}-supports-{off,on}.json`)
   authors any of those keys, so on every matched cell the module returns
   before reading a single stamp, while `classic-perimeters` /
   `arachne-perimeters` still run every query. (`slowdown_for_curled_perimeters`
   shares the same table and the same skip; `bridge_flow` / `thick_bridges` are
   untouched.)

3. **What that means for the candidate shape.** The surviving design is not a
   faster query but a **conditional stamp**: gate the two stamps on the
   consumer's own liveness predicate, value-identical when live. The queries it
   removes are the measured dominant cost: ticket 08 measured them at 69.8% of
   classic guest fuel (ordinary); ticket 18's accelerated stage-2 capture puts
   the two exact-scan families at ≈61.7% benchy / 57.9% base of the classic
   module (`inside_scan` 147.13 / 2,814.51 B + `quartile_scan` 141.48 /
   1,004.66 B of 467.50 / 6,598.34 B); ticket 25 puts Arachne's two queries at
   86.4% of that module. The wall-transfer caveat from
   [Gap budget](12-gap-budget-per-cell.md) still governs: this cannot close a
   6.28–26.24x gap, and the standing paired A/B decides keep/drop.

4. **Take-readiness notes.** Exactness requires gating on exactly the
   classifier's predicate (`enable_overhang_speed` && any band ≠ 0) so a live
   config keeps byte-identical stamps; the liveness keys would have to be
   visible to the perimeter modules' config views. Both generators stamp (the
   ticket's "classic path" wording is narrower than the sites — arachne holds
   the same two stamps in `run_perimeters`). The existing
   `PathAnnotationMode::NonPlanarNoQuartile` split already proves the per-mode
   stamp distinction. This audit is static — a take must re-verify dynamically
   (classifier skip observed, no other runtime reader).

**Route state.** The frontier is this ticket and [serial host
floor](27-serial-host-prepass-floor.md); both sit under standing park decisions
and the route awaits the human's selection. No candidate, experiment or timing
was started.
