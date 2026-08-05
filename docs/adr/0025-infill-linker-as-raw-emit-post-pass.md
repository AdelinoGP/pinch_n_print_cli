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
   module rewrite to raw emit), closing the lightning transitional gap inside this effort (packet
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

### Anchor-length rule now ported

The anchor-length rule is now implemented in the linker. For each candidate arc,
the whole-arc-vs-stub branch takes the entire contour run and merges the two
infill lines when the arc is below `anchor_length_max`; otherwise it emits an
`anchor_length`-long stub from each end and leaves the lines separate. The
`contour_stub` walk lerps its final partial segment, so a stub ends at the exact
requested arc budget rather than snapping to a ring vertex. Canonical consumes
candidate arcs shortest-first. Before this port, PnP's `nearest_pair_candidate`
picked each endpoint's best partner with `min_by` on distance, then the re-solve
loop's `candidates.sort_by` keyed on `endpoint_order(first)` with distance only
as a tiebreak; that pre-fix behavior was lexicographic-by-endpoint rather than
shortest-first. The implementation now replaces that loop with a distance-first
single pass over sorted candidates with a consumed-endpoint guard, so shorter
arcs claim their endpoints first. The deviation log records the pre-fix behavior
and the fix precisely.

The two module keys, `infill_anchor` and `infill_anchor_max`, are declared as
float-or-percent values and resolved against extrusion-flow spacing. Solid and
bridge buckets force both values to unlimited, matching canonical's solid/bridge
branch rather than allowing a restrictive sparse-region setting to block those
connections.

Two parity residuals, one closed transport finding, and one accepted behaviour
move from this packet are recorded in `docs/DEVIATION_LOG.md`:

- `DEV-110` — canonical's `ContourIntersectionPoint` neighbour bookkeeping
  (`could_take_prev`, `could_take_next`, `trim_prev`, and `trim_next`) is not
  ported; PnP clamps the stub to the next boundary position and consequently
  consumes both endpoints where canonical can leave one unconsumed.
- The percent-form transport finding — `parse_percent_default` parses `"400%"`
  into `ConfigValue::FloatOrPercent`, retained on `ConfigFieldEntry.parsed_default`
  and injected via the scheduler's schema-default threading (Packet 185 /
  TASK-303) into `ResolvedConfig.extensions` and hence the runtime
  `ConfigView`, so `get_abs_value` can take its percent arm against the
  extrusion-flow spacing base; `AnchorParams::from_config` fallbacks apply when
  `get_abs_value` returns `None` — i.e. when the corresponding key is absent or
  non-numeric.
- `DEV-112` — the percent formula matches canonical, but PnP supplies the
  module's generic `line_width` rather than canonical's per-role `frInfill`
  flow width as the base input.
- Declaring `line_width` un-deadens the reads that feed spacing and related
  geometry for non-default user values; the default `0.4` slices are unchanged.
  This is an accepted disclosed behaviour move, not a parity gap.

### Regression coverage

- `modules/core-modules/infill-linker/tests/anchor_length_tdd.rs` — whole-arc
  merging, over-limit stubs, exact lerped partial length, candidate ordering,
  percent resolution, zero-anchor dispatch, boundary clamping, and solid-bucket
  forcing.
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

## Amendment 2026-08-05 — determinism is part of the contract (packet 133)

The linker contract is deterministic by construction: candidate endpoints are
ordered by boundary arc position and segment index rather than by `HashMap`
iteration, and identical inputs must produce identical `InfillIR`. This is a
hard requirement for the parity/self-capture suites — a linker whose output
depended on hash-map ordering would fail the byte-comparison backstop of the
cross-region and anchor-length fixtures. `contour_connector`'s ring walks
(`BoundaryRing::directed_distance`) are themselves order-derived, so endpoint
ordering is the root of determinism: same input, same arcs in the same order,
same connectors.

## Amendment 2026-08-05 — lightning raw-emission conversion complete (packet 140)

The ADR's transitional statements about lightning-infill self-linking are now
stale. Packet `140_lightning-module-rewrite` completed the raw-emission
conversion: lightning no longer self-links and is no longer an exception to the
raw-emit + linker contract. Each stale statement is quoted, retired, and
replaced with current behavior.

### §Consequences — the lightning-is-inconsistent bullet

> **Lightning-infill (out of parity scope but exists) is inconsistent** until
> it too switches to raw emit. Until then, lightning self-links while
> rectilinear/gyroid emit raw — the linker handles both (it links whatever raw
> segments it receives; already-linked paths from lightning pass through
> unchanged unless the linker re-clips them). A DEVIATION_LOG entry notes this
> transitional state.

**Retired.** Packet 140 deleted the self-linking stub (`build_branches` and the
inline grid-sampling machinery) and rewrote `lightning-infill` as a thin
per-layer sampler: it reads the layer's tree segments from the
`PaintRegionLayerView::lightning_tree_segments_for` accessor and emits them as
raw `SparseInfill` polylines with the config-derived `speed_factor` — the module
adds no geometry of its own (AC-1). The lightning raw-emit deviation label was
retired (AC-4).

### §Amendment 2026-07-01, point 2 — the "until that packet lands" conditional

> Until that packet lands, the transitional note in §Consequences stands — but
> note the pass-through premise is weaker than written: paths carry no module
> identity (`ExtrusionPath3D` has no origin field), so the linker cannot
> reliably distinguish lightning's self-linked output from raw waves; the real
> fix is the raw-emit conversion, not pass-through detection.

**Retired.** The packet landed, and the prediction it made is what happened: the
real fix was the raw-emit conversion, not pass-through detection. The module
identity observation is moot — the linker no longer has any self-linked output
to distinguish, because every infill module emits raw. The pass-through
premise's weakness is historical, not live.

### §Future-Reviewer Notes — the transitional-inconsistency bullet

> **Lightning-infill's self-linking is a transitional inconsistency**, not a
> permanent design. It is tracked for a follow-up packet to switch to raw emit.
> Do not treat it as evidence that Architecture B is the real design.

**Retired.** There is no self-linking left to be transitional about: lightning
flows through Architecture A like every other module, and the linker is the only
place linking happens (AC-3 `lightning_pipeline_linked` — the sparse bucket
contains linked multi-point polylines derived from tree segments, mean
points-per-path > 2). The "do not treat as evidence for Architecture B" warning
is now moot rather than merely advisory.

Current behavior in full: `lightning-infill` samples and emits raw; the
`Layer::InfillPostProcess` linker clips and connects its output exactly as it
does for rectilinear and gyroid. The raw-emit contract has no remaining
module-side exceptions. The WIT `run-infill` signature now carries the paint
view (`slicer:world-layer@2.3.0`), which is how the module receives the tree
segments.

## Amendment 2026-08-05 — anchor-length contract (packet 192)

Packet `192-infill-linker-anchor-length` ported canonical `Fill::connect_infill`'s
per-arc anchor decision into the linker. The 2026-07-24 amendment's
"Anchor-length rule now ported" section records the implementation; this
amendment states the resolved contract precisely, as the packet specifies it.

### Rule 1 — `dont_connect`: chain-only fork

When the resolved `anchor_length_max` is below 0.05 mm (canonical
`FillParams::dont_connect()`'s `anchor_length_max < 0.05f`), the linker uses
chain-only behavior and emits no connector: `chain_or_connect_infill` skips
`connect_infill` entirely and runs only the chaining half, matching canonical's
fork to `chain_polylines`. The test asserts the paths are still chained
(reordered/reversed for travel), so the fork is provably chain-shaped rather
than a "threshold 0, so nothing links by accident" no-op (AC-N1).

### Rule 2 — zero anchor

For an over-limit arc (arc length ≥ `anchor_length_max`), a zero resolved
`anchor_length` produces neither stubs nor a merge: both input paths remain
separate and neither gains any contour points — canonical's stub branch is
`else if (anchor_length > SCALED_EPSILON)`, so zero disables single-side
anchoring entirely (AC-N2). A positive `anchor_length` enables the two contour
stubs: `contour_stub` walks the ring in each direction with opposite
`RingDirection`s, lerping the final partial segment so each stub measures
exactly the anchor budget rather than snapping to a ring vertex (AC-5, AC-6).

### Rule 3 — config resolution

`infill_anchor` defaults to 400% (percent form) and `infill_anchor_max` to
20.0 mm (absolute form), matching canonical
`ConfigOptionFloatOrPercent(400, true)` / `ConfigOptionFloatOrPercent(20, false)`.
Percent values resolve through `ConfigView::get_abs_value` against the
extrusion-flow spacing `line_width_to_spacing(line_width, layer_height)` —
canonical `Flow::rounded_rectangle_extrusion_spacing`, and deliberately never
the line spacing `line_width / density` (AC-9 asserts varying `infill_density`
leaves `anchor_length` unmoved). Absent or non-numeric keys fall back to
4.0 × the flow spacing for `anchor_length` and 20.0 mm for `anchor_length_max`,
and the resolved anchor length is clamped to `anchor_length_max` (canonical
`group_fills`' `std::min(anchor_length, anchor_length_max)`). Solid and bridge
buckets force both values to unlimited (1000.0 mm), matching canonical's
`surface.is_solid() || is_bridge` branch (AC-8).

### Rule 4 — trailing single-endpoint anchoring pass intentionally not ported

Canonical's trailing single-endpoint anchoring pass over
`graph.map_infill_end_point_to_boundary` — anchoring an unconsumed endpoint by
the shorter of its prev/next arcs, rejecting `l > anchor_length_max` — is
intentionally **not** ported. It is a second pass over a different data
structure and is recorded as follow-on work rather than widened into the
packet. An endpoint that lost its arc to a shorter candidate therefore receives
no single-side anchoring in PnP today.

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
- OrcaSlicer `Fill::connect_infill` / `Fill::chain_or_connect_infill` — per-fill linking, the reference being diverged from.
- `docs/DEVIATION_LOG.md` — the lightning-infill transitional inconsistency,
  the two containment holes recorded in the 2026-07-24 amendment (closed),
  the anchor-length rule, DEV-110 (neighbour bookkeeping),
  the percent transport finding, DEV-112 (percent-base width input), and
  the accepted `line_width` behaviour move.
