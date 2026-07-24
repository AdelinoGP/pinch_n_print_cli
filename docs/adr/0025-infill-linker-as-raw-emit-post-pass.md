# ADR-0025 — PnP Infill Modules Emit Raw Segments; a Dedicated `Layer::InfillPostProcess` Linker Connects Them

## Status

Accepted. Landed with the infill-parity effort: rectilinear-infill + gyroid-infill
parity rewrite + the `infill-linker` module. See the 2026-07-24 amendment for the
two containment defects found in the first implementation and closed since.

## Context

OrcaSlicer's infill pipeline links disjoint scan-line segments into continuous
multi-point polylines **inside each fill class**, in `_fill_surface_single` →
`connect_infill` (FillBase.cpp:1497-2201) and `chain_or_connect_infill`
(FillBase.cpp:2201-2300). Every fill pattern (rectilinear, gyroid, grid,
triangles, …) carries its own linking pass. Cross-region / cross-pattern
travel is handled later, at the G-code entity-ordering layer
(`fill_surface_extrusion` → `ExtrusionEntityCollection` sorting), which reorders
whole already-linked entities but does not break or re-connect paths.

The initial PnP infill-parity plan proposed mirroring this: each infill module
calls a shared `connect_infill` from `slicer-core::infill_ops` before pushing to
`InfillOutputBuilder`. A gap analysis surfaced that the existing
`Layer::InfillPostProcess` stage (`crates/slicer-scheduler/src/execution_plan.rs:33`,
`crates/slicer-wasm-host/src/dispatch.rs:435-454`) receives `PerimeterRegionView`
(which lacks the partitioned fill polygons) and a **fresh empty**
`InfillOutputBuilder` — and that
`LayerStageCommit::InfillPostProcess` (`crates/slicer-runtime/src/layer_executor.rs:1151-1156`)
**discards** the prior `InfillIR` and replaces it wholesale with whatever the
post-process module emits. A post-process linker therefore cannot, under the
current contract, read what `Layer::Infill` emitted.

A grilling session (2026-07-01) weighed two architectures:

- **Architecture B (in-fill self-link + additive cross-module pass):** modules
  self-link matching OrcaSlicer; a separate post-pass does additive
  cross-region optimization only. Self-sufficient modules; linker optional.
  Best matches "full OrcaSlicer parity."
- **Architecture A (raw emit, post-pass links all):** modules emit raw unlinked
  segments; the `Layer::InfillPostProcess` linker is the *only* place linking
  happens, globally across all regions and modules. Couples all infill output
  to the linker being present; diverges from OrcaSlicer's per-fill linking.
  Best matches "modules shallow and algorithm-focused."

The project owner chose **A** to maximize module shallowness and centralize
linking. This ADR records that choice and its trade-offs.

## Decision

PnP infill is split into two tiers with strict responsibility boundaries:

1. **`Layer::Infill` modules emit raw, unlinked segments.** A rectilinear
   module emits raw 2-point scan-line segments; a gyroid module emits raw wave
   polylines. Neither module calls `connect_infill`, applies the infill overlap
   offset, filters short segments, or chains paths. The module's job is:
   rotate polygon → scan-line / wave geometry → emit raw segments tagged with
   role + speed factor. No post-geometry.

2. **A single `Layer::InfillPostProcess` module (the "infill-linker") is the
   only place infill path connection happens.** It reads the prior `InfillIR`
   (the raw segments emitted by all `Layer::Infill` modules), applies the infill
   overlap offset, re-clips against the partitioned fill polygons, removes
   short segments (< 0.8 × spacing), runs `connect_infill` +
   `chain_or_connect_infill` globally across all regions and modules, and emits
   linked multi-point polylines. It is **required infrastructure** in the
   default dispatch graph — without it, infill is raw disjoint segments with
   maximum travel.

3. **The infill overlap offset is a linker concern, not a module or host
   concern.** The module emits segments over the unoffset wall-inset polygon
   (what `crates/slicer-runtime/src/region_partition.rs` already produces). The
   linker applies the overlap (`INFILL_OVERLAP_OVER_SPACING = 0.45 × spacing`)
   as a Clipper2 offset on the wall-inset polygon, re-clips the raw segments to
   the offset boundary, then connects them. This centralizes the one physical
   invariant (perimeter overlap) in one place.

4. **Linking algorithms (`connect_infill`, `chain_or_connect_infill`,
   `BoundaryInfillGraph`) live inside the infill-linker module, NOT in
   `slicer-core`.** Linking is the linker's sole responsibility. `slicer-core`
   gains only `clip_polylines` — a generic Clipper2 polyline-vs-polygon
   operation in `polygon_ops.rs`, useful beyond infill. This reverses the
   initial proposal to put `connect_infill` in `slicer-core::infill_ops`; the
   multi-language module promise (a C++/Zig TPMS module should not depend on a
   Rust linking helper) and the "modules shallow" goal both push the algorithm
   into the linker.

5. **Pipeline:**
   ```
   Layer::Infill (modules emit RAW segments over wall-inset polygon)
     → Layer::InfillPostProcess (infill-linker: offset + re-clip + connect)
     → Layer::Support
     → Layer::PathOptimization (entity-level sort of the linked polylines — unchanged)
   ```
   The two optimization stages operate at different levels: the linker connects
   path endpoints (path-level); `Layer::PathOptimization` reorders whole entities
   (entity-level). No conflict.

This diverges from OrcaSlicer, which links inside each fill class. The
divergence is deliberate: PnP centralizes linking to keep infill modules
shallow (geometry only) and to enable globally-optimal cross-region connection
that no single `run_infill` module (which sees only its own regions) can do.

## Consequences

**Positive**:
- Infill modules are maximally shallow: rectilinear is rotate → scan-line →
  emit; gyroid is rotate → waves → rotate-back → emit. No linking, no overlap,
  no short-filter, no chaining. Each module is ~150-250 lines of geometry.
- One linking algorithm, one place. `connect_infill` is not duplicated across
  rectilinear, gyroid, lightning, and future infill modules.
- Globally-optimal cross-region connection is possible: the linker sees all
  regions' raw segments and can connect endpoints across region/module
  boundaries via perimeter walks on the offset boundary.
- Swapping linking strategies (closest, monotonic, anchor-based) is a
  one-module change, not a per-module change.
- OrcaSlicer porting of the *geometry* (scan-line engine, gyroid wave math) is
  unaffected — the ported math lives in the module and is correct in isolation.

**Negative**:
- **The linker module is required infrastructure.** The default dispatch graph
  must include it, or every print ships with raw disjoint infill segments and
  maximum travel. `ResolvedConfig` must add the infill-linker to the default
  stage list. A user who removes it gets degraded-but-not-failed output.
- **Per-fill output is not valid infill until the linker runs.** A `run_infill`
  module's `ExtrusionPath3D` output is raw segments, not the connected polylines
  OrcaSlicer produces. Tests that assert on connected polylines must target the
  linker's output, not the infill module's. Existing infill tests that assert
  on path shape need surveying (some assert on raw segment count/length, which
  still pass; some assert on connected polylines, which now see raw segments).
- **WIT schema bump is load-bearing.** `run_infill_postprocess` must take the
  prior `InfillIR` as input (not an empty builder), and `perimeter-region-view`
  must carry the four partitioned fill polygons. Both are required for the
  linker to read prior output and re-clip against the right boundary. Every
  guest rebuilds (`cargo xtask build-guests`). See ADR-0028.
- **Lightning-infill (out of parity scope but exists) is inconsistent** until
  it too switches to raw emit. Until then, lightning self-links while
  rectilinear/gyroid emit raw — the linker handles both (it links whatever raw
  segments it receives; already-linked paths from lightning pass through
  unchanged unless the linker re-clips them). A DEVIATION_LOG entry notes this
  transitional state.
- **The linker must re-clip already-clipped segments.** The modules emit raw
  segments over the wall-inset polygon (unoffset). The linker applies the
  overlap offset and re-clips. The re-clip is not redundant — the segments were
  never clipped to the *offset* boundary, only to the *wall-inset* boundary.
  The re-clip is the linker applying the overlap for the first time.

**Trade-offs we explicitly accept**:
- Per-fill output is not "correct infill" in isolation. This is the cost of
  centralizing linking. The benefit (one algorithm, globally optimal, shallow
  modules) is worth it. A future packet could add a per-module "link my own
  output" escape hatch for modules that want to be self-sufficient, but that
  reintroduces duplication and is rejected for v1.
- The WIT schema bump (ADR-0028) is a real cost: every guest rebuilds, every
  exhaustive match on `PerimeterRegionView` gains fields. This is the standard
  pattern (ADR-0002, ADR-0009, ADR-0010 all paid it) and is not a reason to
  avoid the contract change.
- `Layer::PathOptimization` and the infill-linker both reduce travel, at
  different levels. A future reviewer might wonder why both exist. They do
  because the linker connects path endpoints (geometric), while
  PathOptimization sorts whole entities (combinatorial). Removing either
  degrades print time.

## Amendment 2026-07-01 — cross-region connection scoped to wall-sharing groups; lightning parity in-roadmap (grilling session)

Two claims in this ADR were sharpened by the 2026-07-01 grilling against the codebase:

1. **"Globally-optimal cross-region connection" is scoped to wall-sharing groups.** Code
   evidence showed extruded cross-region connection is physically invalid in the general case:
   perimeter walls are generated along every normal shared region boundary (each paint-variant
   region gets its own full wall loops, `crates/slicer-core/src/algos/prepass_slice.rs:244` +
   the paint-segmentation region rebuild), tool identity is resolved per-entity only after
   `Layer::InfillPostProcess` (`crates/slicer-runtime/src/layer_executor.rs:590-775`), and
   per-region config is invisible at the stage
   (`crates/slicer-wasm-host/src/dispatch.rs:1629-1645`). Cross-region connection is therefore
   restricted to **wall-sharing groups** — regions with no walls between them (paint
   virtual-variants sharing base walls, `region_partition.rs:35-44`, and modifier sub-regions
   per ADR-0030) — under the predicate: same object-id, same tool-index, same role, same
   wall-sharing group, path-compatible (equal `speed_factor`, endpoint widths within epsilon).
   Two linking branches:
   - **Same-config wall-less siblings:** union the group's role polygons, build one
     `ExPolygonWithOffset`, run `connect_infill` over the union boundary. Bucket ownership of
     a merged polyline: the region containing the majority of its length; tie → lower
     region-id.
   - **Different-config wall-less siblings** (the modifier-infill case — different densities/
     patterns): link per-region along the region's OWN boundary including the wall-less shared
     arc, applying **no overlap inset along wall-less arcs** (a uniform inset would leave a
     `2 × 0.45 × spacing` unfilled ring at the shared boundary).
   Connection between regions separated by walls remains invalid; revisit only with an IR
   change. Travel between such regions stays `Layer::PathOptimization`'s job. The two
   supporting view fields (`tool-index`, `wall-source-region-id`) are recorded in ADR-0028
   §Amendment.

2. **Lightning-infill is no longer a transitional exception.** The roadmap now includes full
   OrcaSlicer lightning parity (ADR-0029: `PrePass::LightningTreeGen` + `LightningTreeIR` +
   module rewrite to raw emit), closing DEV-081 inside this effort (packet
   `140_lightning-module-rewrite`). Until that packet lands, the transitional note in
   §Consequences stands — but note the pass-through premise is weaker than written: paths
   carry no module identity (`ExtrusionPath3D` has no origin field), so the linker cannot
   reliably distinguish lightning's self-linked output from raw waves; the real fix is the
   raw-emit conversion, not pass-through detection.

## Amendment 2026-07-24 — containment is part of the contract: per-role re-clip, and connectors route along the contour

The first `infill-linker` implementation satisfied §2's *wording* while violating
its *intent* in two independent places. Both are containment holes — geometry
escaping the polygon it was supposed to stay inside — and both are now closed.
This amendment records them so the contract cannot be re-read the loose way.

**Canonical does enforce per-role containment, three times over.** It is not an
emergent property of the fill patterns. In `libslic3r/Fill/`: `group_fills`
buckets surfaces into `SurfaceFill`s keyed on a `SurfaceFillParams` that includes
`extrusion_role`, so each role gets its own expolygon set; a mutual-clipping pass
then subtracts every other bucket from each bucket via
`diff_ex(polys, all_polygons, ApplySafetyOffset::Yes)`; and individual fills clip
their own output again, e.g. `FillGyroid::_fill_surface_single`'s
`intersection_pl(polylines, expolygon)`. PnP centralises linking (that is this
ADR's whole point), so all three guards collapse into the linker — which makes
the linker the *only* thing standing between a raw wave and a sparse stroke laid
across a top-solid island.

### Hole 1 — "the partitioned fill polygons" is plural, and the union is not a substitute

§2 says the linker "re-clips against the partitioned fill polygons". The
implementation's `region_boundary` (`modules/core-modules/infill-linker/src/orchestrate.rs`)
returned `PerimeterRegionView::infill_areas()` — the **union of all four role
polygons**, i.e. the wall inset. Re-clipping every role against the union is not
a weaker version of per-role clipping; it is the absence of it. A `SparseInfill`
stroke clipped to the union is free to run straight across the region's
`top_solid_fill` or `bridge_areas`.

This was invisible for `rectilinear-infill`, which is incidentally safe:
`scan_expolygon` pairs scan-line crossings against the per-role expolygon's own
edges, so its raw output never leaves the role polygon in the first place. It was
live for the wave-shaped and tree-shaped fills, whose raw emit deliberately
overshoots — gyroid's raw waves on the regression fixture start at x ≈ −8.6 mm on
a polygon spanning 0–5 mm.

**Contract, restated:** the linker resolves a boundary **per (region, role)**, from
that role's own host-partitioned polygon —
`sparse_infill_area` / `top_solid_fill` / `bottom_solid_fill` / `bridge_areas`.
`InternalSolidInfill` (the deep-shell relabel that `solid_role` applies at shell
index ≥ 1) maps to the union of the two solid-shell polygons. `infill_areas` is
used only (a) for views the host never partitioned, and (b) for roles that have no
dedicated partition. See `RoleBoundaries::for_role`.

Two consequences worth stating explicitly, because they are easy to get backwards:

- **Cross-region joining survives.** The wall-sharing-group union of the
  2026-07-01 amendment is an intentional PnP improvement over canonical and is
  preserved — but the union is now taken **per role across sibling regions**
  (`link_union_group`), not across roles within a region.
- **A known-empty role boundary is not an unknown one.** `for_role` returns
  `Option`: `None` means no boundary could be resolved and the paths pass through
  untouched (the historical behaviour); `Some(empty)` means the host partitioned
  the region and gave this role no area, so the role's paths have nowhere legal to
  go and clip away. Collapsing those two into "empty ⇒ pass through" would have
  turned this fix into a *new* leak for roles with an empty polygon.

### Hole 2 — a linking connector is extruded geometry, so it needs containment too

`connect_infill` joined two polylines with `first.points.extend(second.points)` —
a bare chord between the two endpoints, gated only on the arc distance between
their boundary projections. Nothing tested whether the chord stayed inside the
region. On any concave boundary it does not: two endpoints either side of a reflex
corner are a short arc apart and an arbitrarily long way apart *through the
notch*. Re-clipping the segments and then connecting them with an unclipped chord
leaves the containment guarantee exactly as broken as before.

Canonical never emits a bare chord. `Fill::connect_infill`
(`src/libslic3r/Fill/FillBase.cpp`) re-parametrises the boundary with the infill
endpoints spliced in as real ring vertices (`create_boundary_infill_graph`), then
routes each connector **along the contour**: `take_ccw_full` / `take_cw_full` copy
the run of ring vertices between two T-joints verbatim — no simplification, no arc
fitting, no collinearity merge. Canonical needs no containment test on the result
because the connector **is** exact boundary geometry. Containment is structural.

**Contract, restated:** a connector emitted by the linker must lie on the
boundary it was routed along. `contour_connector`
(`modules/core-modules/infill-linker/src/graph.rs`) materialises every ring vertex
strictly between the two joined endpoints, interpolating `z` and `width` across
the walk; `BoundaryRing::directed_distance` supplies the missing piece the old
distance helper discarded — *which way round* the ring the shorter walk goes.

Also from canonical, and now explicit in the code: **connectors never cross
rings.** `prev_on_contour` / `next_on_contour` are wired within one ring, and
`take` / `take_limited` always receive a single ring's point array; there is no
outer-contour-to-hole bridging connector. Endpoints that do not resolve to the
same ring are therefore left **unconnected** — the polylines stay separate rather
than being joined by a chord across the interior or across the gap between two
disjoint islands.

### Not yet ported from canonical

The anchor-length rule is still outstanding. Canonical decides per candidate arc:
if the arc between two adjacent T-joints is shorter than `anchor_length_max` it
takes the whole arc and merges the two lines into one polyline; otherwise it takes
only an `anchor_length`-long stub off each end (`take_limited`, which lerps the
final partial segment so the stub is exactly the requested length) and leaves the
lines separate. Canonical also sorts candidate arcs shortest-first and consumes
them greedily. PnP currently has a single distance gate (10 × spacing) with no
stub mode. This is a *quality* gap, not a containment gap — the connectors it
emits are contour geometry either way.

### Regression coverage

- `connect_tdd.rs` — `connector_routes_through_the_reflex_corner_instead_of_chording_the_notch`,
  `connector_walks_a_hole_ring_rather_than_cutting_across_it`,
  `endpoints_on_different_rings_are_never_joined`.
- `crates/slicer-runtime/tests/integration/infill_partitioned_input_tdd.rs` —
  `ac7*` now drive the **module + linker pair**. They previously called
  `run_infill` alone, which under this ADR cannot satisfy a containment assertion:
  §Consequences already says a fill module's output is raw segments and "tests
  that assert on connected polylines must target the linker's output". Note also
  that `lightning-infill` renders exclusively from the
  `PrePass::LightningTreeGen` product (ADR-0029), so its arm of those tests needs
  a `LightningTreeIR` in the paint view or it emits nothing and asserts vacuously.

## Future-Reviewer Notes

- **Do not re-suggest putting `connect_infill` in `slicer-core::infill_ops`.**
  This was the first instinct during the grilling and was rejected at the user's
  choice: linking is the linker module's job, full stop. `slicer-core` stays
  generic geometry only (`clip_polylines`). If a future module wants to
  self-link, it duplicates or depends on the linker module — accepted.
- **Do not re-suggest Architecture B (in-fill self-link).** The "full OrcaSlicer
  parity" goal was weighed against the "modules shallow" goal and the latter
  won. B is not wrong; it is a different value choice. PnP chose A.
- **Do not move the overlap offset back into the module or the host.** The
  overlap is a linker concern so modules emit pure geometry and the host
  partition stays the wall-inset (no overlap applied). Moving it back
  re-couples modules to a physical invariant.
- **Lightning-infill's self-linking is a transitional inconsistency**, not a
  permanent design. It is tracked for a follow-up packet to switch to raw emit.
  Do not treat it as evidence that Architecture B is the real design.

## References

- `docs/adr/0026-infill-linking-algorithms-in-linker-module.md` — algorithm home.
- `docs/adr/0027-gyroid-multi-role-fill-holder.md` — gyroid solid-shell scope.
- `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md` — contract change.
- `crates/slicer-scheduler/src/execution_plan.rs:19-41` — `STAGE_ORDER` (includes `Layer::InfillPostProcess`).
- `crates/slicer-wasm-host/src/dispatch.rs:435-454` — current `run_infill_postprocess` dispatch (empty builder).
- `crates/slicer-runtime/src/layer_executor.rs:1151-1156` — `InfillPostProcess` replace-commit.
- `crates/slicer-runtime/src/region_partition.rs` — wall-inset partition (no overlap applied).
- `crates/slicer-sdk/src/traits.rs:374-393` — `run_infill_postprocess` trait hook.
- `crates/slicer-schema/wit/deps/world-layer/world-layer.wit:25` — WIT signature.
- OrcaSlicer `src/libslic3r/Fill/FillBase.cpp:1497-2300` — `connect_infill` / `chain_or_connect_infill` (per-fill linking, the reference being diverged from).
- `docs/DEVIATION_LOG.md` — DEV-081 (lightning-infill transitional inconsistency).