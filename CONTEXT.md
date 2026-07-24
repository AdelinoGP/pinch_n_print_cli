# Context Glossary

A shared vocabulary for the pinch_n_print modular slicer. Definitions only —
no implementation details. See `docs/` for architecture and contracts.

## Terms

### Paint data
Per-facet annotations a user applies to a mesh: material/tool assignment, fuzzy
skin, support enforcer/blocker, and seam enforcer/blocker. Authored in the
**GUI**, not the CLI. Carried in a 3MF as `paint_*` triangle attributes.

### Region modifier (modifier volume)
A sub-volume overlaid on an object that overrides config for the region it
covers (e.g. a denser-infill box, a negative/cut part, a support
enforcer/blocker zone). Authored in the **GUI**, not the CLI. Carried in a 3MF
as extra objects referenced as components, classified by a sidecar.

### Solid (connected component)
A maximal set of triangles in a mesh that are joined to each other through
shared edges — one physically-disconnected piece of geometry. A single STL/OBJ
file often contains several solids fused into one triangle soup; a STEP file
already separates them.

### Split to objects
Fanning a mesh's distinct solids out into separate addressable objects, so each
can be transformed, configured, painted, or region-modified independently. A
user choice exposed in **both** the GUI and the CLI's convert/import path (which
splits by default). The inverse of merging components into one mesh.

### Paint-ready 3MF
A 3MF the backend CLI emits that carries **geometry only** (vertices +
triangles, one or more objects) and no paint data or region modifiers. It is the
hand-off artifact the GUI opens so a user can add paint and region modifiers
there. The CLI's job ends at producing clean geometry; authoring annotations is
the GUI's responsibility.

### Paint semantic
The typed meaning of a piece of **paint data**: `Material`, `FuzzySkin`,
`SupportEnforcer`, `SupportBlocker`, or `Custom`. Paint data is what the user
applies to a facet; its paint semantic is what that mark means. Overlapping
marks of different semantics resolve by deterministic precedence. A paint
semantic is either a **region-split semantic** or carried as a
**segment annotation**, depending on what consuming modules ask for.

### Region-split semantic
A **paint semantic** that a loaded module asks the host to treat as a
region-splitting axis. When such a semantic appears in **paint data** on a base
region, the slicer materializes one or more **painted variants** of that region,
each with its own resolved configuration. Semantics that are not region-split are
carried as **segment annotations** instead. The set of region-split semantics is
decided per slicer instance, not hard-coded.

### Variant chain
The ordered sequence of paint-semantic discriminators that distinguishes a
**painted variant** from its base region. Two regions of the same object and
same base region with different variant chains are distinct regions for module
dispatch and configuration purposes. An empty chain identifies the base region.
The order is canonical: built-in semantics follow OrcaSlicer precedence
(Material before FuzzySkin), community-defined semantics layer after.

### Painted variant
A region distinguished from its base by a non-empty **variant chain**. Each
painted variant carries its own resolved configuration — the base config plus
the layered overlays contributed by each semantic in the chain. Painted variants
of the same base region cover disjoint pieces of that base region's geometry.

### Segment annotation
Per-contour-segment paint metadata that does NOT drive region-splitting. Carries
continuous quantities, seam enforcer/blocker placement, and any per-point paint
data a consuming module reads point-by-point rather than by region. Distinct
from **paint data on a region-split semantic**, which materializes into a
**painted variant** instead.

### Blackboard
Host-owned shared state computed once before any layer is sliced and then
treated as read-only while layers are processed. Modules read from it during
layer execution but never write back to it there.

### Marshalling boundary
The single place where data crossing between the host's internal
representation (IR) and a module's WIT view is translated, in both directions,
and where guest-emitted output is re-attributed to the source **region** it
came from. It owns both the mechanical type translation and the origin-based
identity reconstruction, so the question "how does a value cross the
host/module seam" has one answer rather than several scattered across dispatch
and host-side code.

### Module tier
The coarse grouping of **stages** by pipeline phase — prepass, layer, postpass,
finalization. Each tier has exactly one WIT *world*, and a module belongs to
exactly one tier. This is what "world" means when someone says "which world does
this module target": the tier, not the stage.

### Stage contract
The set of exports a module must satisfy to be loadable. Today this is the whole
**module tier**'s world — a layer module must provide all ten layer exports, even
though it implements one **stage**, so a change to any one stage's signature
invalidates every module in the tier. The unit of *contract* is therefore the
tier, while the unit of *work* is the stage; the two are not the same thing, and
conflating them under the single word "world" is why a one-stage change bills the
whole tier.

### Stage interface
The unit that collapses that gap: one independently versioned WIT **package** per
**stage**, each holding a single interface, so a module declares only the stage
contract it actually implements and a change to one stage cannot invalidate the
others. The package — not the interface — is the unit, because a version attaches
to a package and an interface cannot carry one; interfaces sharing a package share
its version and so bump together. Distinct from **module tier** — a tier groups
stage interfaces rather than fusing them. Decided in ADR-0045; not yet
implemented, and named here because the distinction is the point.

### Per-region output origin
The explicit identity tag a guest attaches to perimeter and infill output pushes
so the **marshalling boundary** can route each push back to the source **region**.
Set via the WIT `set-current-origin` method (host `explicit_perimeter_origin`
field) or the SDK `begin_region` context method. Takes highest precedence in
the additive `effective_perimeter_origin` chain (explicit → perimeter-region
touch → slice-region touch), so a guest that calls `begin_region` at the top
of its `for region in regions` loop tags every wall loop, infill area, seam
candidate, reordered wall loop, sparse/solid/ironing path with the correct
region — restoring per-tool sparse-infill distribution on multi-region prints.
Distinct from the `touch_slice_region` / `touch_perimeter_region` fallback,
which is defence-in-depth for guests that forget `begin_region` and is the
only origin source for the support stage (deferred to a sequel packet).

### Global layer
One authoritative horizontal slicing plane spanning the whole build and shared
by every object. The canonical Z list against which all per-object layers are
aligned.

### Object-local layer
A layer counted relative to a single object, independent of where that object
sits in the global Z list. Each maps deterministically onto a **global layer**.

### Sync layer
A **global layer** where objects printing at different layer heights line up on
a common Z. Decided during planning, not recomputed while printing.

### Catch-up layer
A layer where an object that has fallen behind the global Z spans from its
previous local height up to the next **sync layer** in a single step.

### Active region
A single object's **region** at one **global layer**, carrying its fully
resolved configuration — no remaining fallbacks or overrides left to apply.

### Region override
A configuration or module-selection change applied at the scope of a **region**,
narrowing or replacing what the object-level config would otherwise specify.
(Distinct from **Region modifier** — see Flagged ambiguities.)

### Claim
An exclusive capability slot (e.g. generating infill) that exactly one module
holds for a given layer/object/region. Prevents two modules from contending for
the same job.

### Degraded success
A slice that finishes despite one or more non-fatal module failures. The result
is usable but flagged as degraded, and every failure is reported — never silent.

### Fatal failure
A module, contract, or integrity error that aborts the slice immediately. No
silent continuation past the failure.

### Shell depth
Depth, in layers, of a region within its owning object's top or bottom shell zone. `0` = exposed surface; `None` = outside any shell zone of that object. A property of a region of an object, computed per-object — not shared across objects on a layer.

### Infill
The sparse or solid extrusion paths filling the interior of a region's
wall-inset polygon. Produced by `Layer::Infill` modules (raw segments) and
connected by the `Layer::InfillPostProcess` linker (see Infill linker). Divided
into four roles — sparse, top-solid, bottom-solid, bridge — each carried by a
pre-partitioned polygon (`sparse_infill_area`, `top_solid_fill`,
`bottom_solid_fill`, `bridge_areas`) produced by the host at
`Layer::Perimeters` commit with precedence `bridge > bottom > top > sparse`.

### Fill holder
The module currently configured to produce extrusions for one of the four
infill roles on a region. Selected per-role via `top_fill_holder` /
`bottom_fill_holder` / `bridge_fill_holder` / `sparse_fill_holder` (default
`"rectilinear-infill"`). Distinct from a **declared claim**: a module may
declare a claim it never holds (out-prioritized by another holder), and may
contain emission code for a role it neither declares nor holds (dead until the
manifest adds the claim). The `should_emit(role)` gate keys on the *held* set,
not the *declared* set.

### Declared claim vs held claim
A **declared claim** is a capability slot a module's manifest says it *can*
hold. A **held claim** is what the host dispatcher resolves that module to
actually hold for a given `(layer, object, region)` triple at runtime. Code
branching on `should_emit(role)` is gated by the *held* set, so a module
emitting code for a role it doesn't declare is dead code, not a user option —
until the manifest adds the claim and the user configures the module as the
holder.

### Infill linker
The single `Layer::InfillPostProcess` module that connects raw infill segments
(emitted by all `Layer::Infill` modules) into continuous multi-point polylines,
uniformly across all regions and modules, applying the infill overlap offset
and re-clipping against the partitioned fill polygons. Linking is per
(region, role); endpoint connection *between* regions happens only inside a
wall-sharing group — never across perimeter walls. Required infrastructure —
without it, infill is raw disjoint segments with maximum travel. Distinct from
`Layer::PathOptimization`, which sorts already-linked whole entities but does
not connect endpoints. Diverges from OrcaSlicer, which links inside each fill
class. See ADR-0025 (+ 2026-07-01 amendment).

### Wall-sharing group
A base region together with the wall-less sibling regions that share its
perimeter walls: paint virtual-variants without their own perimeter entry, and
modifier sub-regions (see Modifier sub-region). Boundaries *within* the group
carry no walls, so infill may anchor or connect along them; boundaries between
groups are wall-backed and infill never crosses them. The only scope in which
the infill linker connects paths across region boundaries. See ADR-0025
amendment and ADR-0030.

### Modifier sub-region
A wall-less sub-region produced by intersecting a modifier volume's
cross-section with its base region's fill areas: it has its own region
identity and config (e.g. a different infill density) but shares the base
region's walls — no perimeters are generated at the modifier boundary.
Contrast with paint splits, which produce fully walled regions. The mechanism
behind local-stiffness infill modifiers. The sub-region carries an empty `variant_chain`; its identity is its modifier-namespace `region_id` (base × 1 000 003 + modifier_hash), with `wall_source_region_id` pointing back to the base region. See ADR-0030.

### Lightning tree
The branching structure that lightning infill extrudes: grown top-down across
all layers of an object (each layer's branches must land on the layer below),
then sampled per layer at fill time. Tree generation is a whole-object,
cross-layer computation (a PrePass concern), not a per-layer one; the
lightning module only samples the finished trees for its layer and emits raw
branch polylines. See ADR-0029.

### Infill overlap
The lateral inset applied to infill scan lines so they overlap the perimeter
walls, ensuring adhesion and avoiding gaps at the wall/infill boundary. In
PnP, applied by the infill linker module (not the infill module or the host
partition) as a Clipper2 offset on the wall-inset polygon before re-clipping
raw segments. OrcaSlicer applies it inside the fill class. PnP centralizes it
in the linker so infill modules emit geometry only.

### Overhang quartile
The discrete severity band (1 = least severe, 4 = most severe) classifying
how far a wall vertex sits from the nearest supported edge on the layer
below, measured in multiples of line width. Absent (`None`) means the vertex
is fully supported — quartile 0 is not a valid state.
_Avoid_: overhang band, overhang level

### Curled height
The estimated vertical distance (mm) a wall segment has lifted away from the
layer below during printing, computed once per segment by comparing the
current layer's outer wall to the previous layer's boundary and curvature.
`None` where no previous layer exists (layer 0) or no curling was detected.
_Avoid_: curl amount, warp height

### Artificial curl distance
A synthesized "distance from support" value derived from a nearby segment's
**curled height** and proximity, expressed in the same units as overhang
distance so it can be classified through the same **overhang quartile**
bands and slowed via the same overhang speed configuration. Lets
curl-avoidance reuse the overhang speed table on the layer above a curled
segment, instead of a separate curl-specific one.
_Avoid_: curl distance, synthetic distance

### Central/spine edge
An edge of the Arachne skeletal-trapezoidation graph classified as lying on
the medial axis of the input polygon — the locus a variable-width toolpath
walks along, as opposed to a **rib edge**, which runs perpendicular out to
the polygon boundary. Marked by the faithful `dR < dD * sin(transitioning_angle/2)`
predicate; the sequence of central edges within one topological domain is
what `connectJunctions` stitches into a single toolpath.

### Region order
The Arachne ordering pass over finalized extrusion lines, before they become
`WallLoop`s. It applies OrcaSlicer's odd-after-enclosing constraint so an
inner region follows its enclosing outer region. It uses spatial adjacency
constraints and a deterministic topological walk, with the selected wall
sequence governing direction. It establishes a proposed region order; the
perimeter module owns committing that proposal as the final `WallLoop` print
order.

### Region-order constraint set
The directed before/after relation between spatially adjacent Arachne
extrusion lines. A valid region-order constraint set is acyclic because it
contains only canonical adjacent-inset relations; cycle recovery is not part
of region ordering.

### Committed wall sequence
The final print order for walls governed by `wall_sequence`, established by
the perimeter module. Path optimization may reduce travel within this order,
but must not invert the selected wall sequence.

### Wall sequence
The configured three-state ordering policy for a region's walls: `InnerOuter`,
`OuterInner`, or `InnerOuterInner`. It is resolved by the perimeter module and
preserved across execution boundaries; it is not reducible to an outer-first
boolean.

### Role partition
The division of a region's fillable area into the four canonical
**extrusion-role** polygons — sparse infill, top solid fill, bottom solid
fill, and bridge — each of which is a distinct fill job with its own pattern,
density, and boundary. Canonical OrcaSlicer establishes it by bucketing
surfaces on `extrusion_role` in `group_fills` and keeps the buckets disjoint
by mutual clipping. A role partition is not a single area: the union of the
four polygons is a strictly weaker object, and substituting the union for the
partition is what lets one role's extrusion cross another's territory.

### Cross-region link
A join between two infill polylines that belong to sibling regions of the same
role, rather than to one region. It is a deliberate Pinch 'n Print improvement
over canonical, which links only within a single fill surface, and it is what
lets a region split — by paint, by **region modifier**, or by variant — stop
costing a travel move at every seam. It is constrained by the **role
partition**: siblings are joined per role, never across roles.

### SparsePointGrid
A sparse spatial-hash utility used by Arachne region ordering to find nearby
extrusion junctions without allocating a dense grid. Its cell size is the
search radius itself; queries return candidates from cells intersecting the
query area. The caller owns the precise eligibility predicate.

### Rib edge
An `EdgeType::EXTRA_VD` perpendicular-foot edge pair inserted after every
transferred spine edge during Arachne graph construction, connecting a
spine node to the polygon boundary. Ubiquitous — inserted at every
transferred edge, not just at reflex corners — and unconditionally
non-central; it delimits one side of a **quad**, but is never itself
walked by `connectJunctions`.

### Quad (Arachne)
The unit cell of the Arachne skeletal-trapezoidation graph: a
four-sided region bounded by two **central/spine edges** and two
**rib edges**, produced per-cell during graph construction from the
underlying Voronoi diagram. Centrality, bead-count assignment, and
propagation all operate over the quad/rib topology rather than raw
Voronoi cells.

### Edge junctions
The OrcaSlicer-faithful per-edge extrusion storage: one `Vec<ExtrusionJunction>`
per edge (matching OrcaSlicer's `LineJunctions` typedef), ordered peak-side
(high R) to boundary-side (low R), with `perimeter_index == junction_idx`.
`connectJunctions` walks each domain as a single chain and emits these vectors;
`stitch_extrusions` reconnects the fragments into closed perimeters.

### Domain-start
A central edge with no predecessor in its topological domain (the
`!prev`-equivalent condition) — the point from which `connectJunctions`'s
quad-chain walk begins for that domain, mirroring OrcaSlicer's
`unprocessed_quad_starts` bookkeeping.

### `getNextUnconnected`
The traversal step `connectJunctions` uses to advance from one central
edge to the next unprocessed edge in the same domain, via a
`next`-then-`twin`-hop continuation. Requires the graph's `next`/`prev`/
`twin` pointers to be topologically faithful to the domain structure
(not copied verbatim from the raw per-cell Voronoi DCEL) for the walk to
terminate correctly and cover every edge exactly once.

### `BeadingPropagation`
The recursive traversal that assigns bead counts to every **quad** in a
topological domain, starting from the widest edge and propagating inward
via the quad/rib topology. Each step consults the `BeadingStrategy` stack
to determine how many beads fit at the current edge's radius, then
records the count and continues to the next narrower edge. The result is
a per-edge bead count that varies smoothly along the domain, enabling
variable-width extrusion.

### `getBeading`
The `BeadingStrategy` method that computes the bead layout (count,
positions, widths) for a given edge radius. Called by `BeadingPropagation`
at each step; returns a `Beading` containing the ordered list of bead
widths and their offsets from the medial axis. The strategy stack
composes multiple strategies (e.g. widening, narrowing, middle-out)
via delegation.

### Wall line width (vs. bead width vs. flow spacing)

Three width-like quantities that are NOT interchangeable — conflating them
caused both `D-147-STITCH-TINY-POLY-UNITS`'s neighbourhood and the two D-160
bugs, so keep the domains straight:

- **Wall line width** — the user-facing extrusion width in mm
  (`outer_wall_line_width` / `inner_wall_line_width`). What the printed bead
  physically measures across. This is the number the user configures and the
  number G-code flow math must reproduce.
- **Flow spacing** — the centre-to-centre distance between adjacent
  extrusions: `spacing = width − layer_height·(1 − π/4)` (canonical
  `Flow::rounded_rectangle_extrusion_spacing`; PnP
  `slicer_core::flow::line_width_to_spacing`). Always strictly narrower than
  the width, because adjacent rounded-rectangle beads overlap at their
  semicircular flanks.
- **Bead width** — Arachne's per-junction target width inside the beading
  engine. **Arachne bead widths ARE flow-spacing values**, not extrusion
  widths: canonical feeds `WallToolPaths` `bead_width_0/x =
  ext_perimeter/perimeter_flow.scaled_spacing()`, and everything the
  `BeadingStrategy` stack computes and stores on junctions lives in that
  spacing domain. At emission, canonical converts BACK to an extrusion width
  (`VariableWidth.cpp::thick_polyline_to_multi_path`:
  `flow.with_width(unscale(w) + height·(1 − π/4))`); PnP does the same in
  `arachne-perimeters::build_walls` via `flow_to_width`.

Rules of thumb: config keys and `PerimeterIR` vertex widths are WIDTHS;
everything between `arachne_params_from_config`'s `line_width_to_spacing` and
`build_walls`' `flow_to_width` is SPACING. A width smuggled into the spacing
domain under-extrudes by `layer_height·(1 − π/4)` (~10.7% at 0.4/0.2) —
exactly D-160 Bug B. `classic-perimeters` never enters the spacing domain, so
it has no back-conversion; that asymmetry is intentional.

A non-positive flow spacing (width ≤ layer_height·(1 − π/4)) is a **config
error, not a value**: canonical throws `FlowErrorNegativeSpacing` and the
slice aborts; PnP returns an error that every caller must treat as
slice-fatal (D-162). There is no "no usable spacing" sentinel and no
fall-back-to-width — a surviving sentinel is how a width re-enters the
spacing domain.

### Transition end
The narrow end of a **quad** where the bead count decreases by one
relative to the wide end — the point where a variable-width extrusion
transitions from N beads to N-1 beads. Marked by the `BeadingStrategy`
when the edge radius can no longer accommodate the current bead count.
The transition's geometry (length, position along the quad) determines
the smoothness of the width change in the printed toolpath.

### `filterNoncentralRegions`
The post-processing step that discards **quad** regions whose central
edge is classified as non-central (i.e. `dR >= dD * sin(θ/2)`), so that
only truly medial-axis-aligned regions survive to bead-count assignment.
Prevents the variable-width walk from following edges that are too close
to the polygon boundary, which would produce degenerate or zero-width
extrusion.

### Local maximum
A **central edge** whose radius is greater than both its predecessor and
successor in the domain — a local widening of the polygon. The
`BeadingStrategy` stack uses local maxima as the starting points for
bead-count propagation (widest-first), ensuring the bead count is
assigned at the most generous cross-section and then reduced as the
domain narrows.

### `separateOutInnerContour`
The graph-construction step that isolates the inner (hole) contour's
Voronoi cells from the outer contour's, producing a separate
skeletal-trapezoidation graph for each topological contour. Without
this, the medial axis of a hole would be walked as if it were part of
the outer boundary, producing toolpaths that cross the void. Each
contour's graph is then processed independently through bead-count
assignment and `connectJunctions`.

### Fixed-inset wall model
The OrcaSlicer wall-placement model that the D-105 beading fix implements: the
beading strategy places capped walls at uniform `optimal_width` insets (one Flow
spacing) with the surplus region thickness carried as `left_over` (infill),
rather than distributing the cap's beads across the full polygon thickness. In
PnP this is realized by `LimitedBeadingStrategy::compute` recomputing the parent
at `optimal_thickness = max_bead_count * optimal_width`. The contrast is the
pre-D-105 buggy behavior of `DistributedBeadingStrategy`, which distributes
`max_bead_count` beads across the full region thickness, producing variable bead
widths in different insets. The PnP cap of `optimal_bead_count` at
`max_bead_count + 1` is the signal that triggers the fixed-inset model; the
under-cap boundary (`bead_count == max_bead_count && even`) is the boundary case
where a center sentinel marks where infill/skin should align.
_Avoid_: distributed wall model, thickness-distributed beading

### Discretize cases (Arachne)
The three-case analysis of OrcaSlicer's `SkeletalTrapezoidation::discretize`,
which determines which Voronoi edges get subdivided and how: (1) seg-seg or
secondary → `{start, end}` with no subdivision; (2) point-segment → parabolic
discretization; (3) point-point → a straight edge subdivided by
`discretization_step_size` with angular marking vertices. A faithful port must
distinguish all three. The PnP port's `discretize_edge` currently collapses
cases 1 and 3 into one `!is_curved` branch — a latent divergence tracked by
spec packet 154 (thin-strip collapse). The PnP `is_curved` flag (from boost
voronoi) corresponds to case 2; `!is_curved` covers both case 1 and case 3.

### split-middle threshold
A `BeadingStrategy` parameter (OrcaSlicer `wall_split_middle_threshold`, PnP `get_split_middle_threshold`) that sets the thickness at which the middle bead of an odd-bead-count region is split into two, distinguishing the "split" parity branch from the "add" branch in `DistributedBeadingStrategy::optimal_bead_count` and `RedistributeBeadingStrategy`'s transition-thickness math. In PnP it is a required trait method forwarded by all four decorators to the innermost `DistributedBeadingStrategy`. See also **intersection-distance gate** and **Fixed-inset wall model**; recorded as a parity residual in `docs/DEVIATION_LOG.md` D-155.

### intersection-distance gate
The `dist_greater` predicate in `ExtrusionLine::simplify` (`Arachne/utils/ExtrusionLine.cpp:163-175`) that rejects removing a junction when the intersection of the extended `(prev_prev, prev)` and `(curr, next)` lines lies farther than `smallest_line_segment_squared` from either `prev` or `curr`, even when the segment-length and height-2 tests would otherwise allow removal — guarding against artifact "spikes" on near-colinear polylines. Ported into PnP's `simplify_distance_gated` tier-3 special case (packet 155, G20). See also **Split-middle threshold**; the G20 RED test's parameters were corrected under `docs/DEVIATION_LOG.md` D-156.

### Self-captured baseline
A regression fixture recorded from PnP's own prior output, not from an
independent OrcaSlicer reference. Green means "unchanged from the snapshot,"
never "correct"; a **structural invariant** that never existed cannot be
proven by a self-captured baseline alone. See ADR-0042.

What is unavailable is an OrcaSlicer *binary* — nothing in this build
environment can be run to produce reference output for a given input, which is
why these fixtures exist at all (see `docs/DEVIATION_LOG.md` D-109/D-112). That
is narrower than "no OrcaSlicer reference exists". OrcaSlicer's C++ *source* is
readable, and a developer who has a checkout can adjudicate canonical behaviour
from it — several entries in `docs/DEVIATION_LOG.md` were settled that way. Any
such checkout is local and gitignored, never vendored, so it is not available to
every developer and must be cited by file and function name, never by line
number (see `CLAUDE.md` §"OrcaSlicer Citation Style"). Reading the source
settles what canonical *does*; it does not produce the numbers a golden needs.

### Structural invariant
A unit-independent assertion of an Arachne correctness property — e.g.
closure within tolerance, loop count/nesting, bead-count sequence,
transitions-present, no self-intersection, coverage ratio, or "no bead wider
than ~2× optimal width" — rather than an equality check against a captured
snapshot or an absolute-coordinate fixture. Invariant to PnP's 1-unit=100nm
divergence from OrcaSlicer (`docs/08_coordinate_system.md`), so it stays
meaningful even where absolute-coordinate comparison would be
flaky-by-construction. See ADR-0042.

### Coverage subject
A source geometry that supplies reproducible Arachne perimeter input and paired
Classic/Arachne output at aligned Z planes. Only coverage subjects contribute to
the observed coverage minimum; the source geometry and both generator
configurations must remain available for reproduction.

### Repeatability margin
The maximum same-subject/same-Z repeated-run delta, capped at `0.02`. It absorbs
measurement instability, not fixture spread or known regressions.

### LLM-visual oracle
The uncommitted OrcaSlicer reference gcode (`tmp/orcaSlicer_arachne_benchy.gcode`),
rendered alongside PnP's own output via `pnp_cli visual-debug` and compared by
Claude's multimodal vision. It **steers** investigation — flagging where two
renders differ — but never **adjudicates** whether a flagged difference is a
real defect; the mechanism must always be confirmed structurally (gcode/IR),
never concluded from the image alone. See ADR-0042.

### Benchy error class
A distinct defect family observed on the benchy reference print (e.g. D4's
inner-wall over-extrusion, D5's dropped bow geometry), used to scope a
**synthetic reproduction fixture** so the fixture reproduces a real,
previously-observed failure mode rather than an arbitrary shape. See
`docs/specs/arachne-parity-recovery.md`.

### Synthetic reproduction fixture
A minimal, hand-built input constructed to reproduce one **benchy error
class** in a fast unit test, backing a **structural invariant** assertion.
Manufactured deliberately rather than sampled from benchy directly, because
simple/arbitrary fixtures do not reliably trigger the error class they are
meant to guard against. See `docs/specs/arachne-parity-recovery.md`.

## Flagged ambiguities

### "region"
Ambiguous on its own — always qualify which sense is meant:
- **Region modifier** — a GUI-authored sub-volume that overrides config over the
  space of geometry it covers.
- **Active region** / **Region override** — a resolved configuration partition of
  one object at one layer, identified by `(object, base region, variant chain)`.
  Multiple **painted variants** of the same base region count as distinct active
  regions when their variant chains differ.
- **Base region** — the unpainted root partition for an object at a layer, before
  any painted variants are materialized. A base region is the `(object, region)`
  pair you'd identify in an OrcaSlicer print where no MMU paint exists.

### "paint applies to region X"
Ambiguous between two mechanisms — qualify which:
- **Region-split** — the paint causes a **painted variant** of the base region
  to materialize, with its own resolved configuration. Use for discrete paint
  data that should drive per-region config differentiation (e.g. Material extruder,
  FuzzySkin on/off).
- **Segment annotation** — the paint is carried per contour point of the base
  region's contour, with no region-split. Use for continuous quantities and
  per-segment metadata (e.g. seam placement, scalar coefficients).

### Visual-debug tap
A request-gated, read-only capture point at a scheduler stage's committed
output, used by `pnp_cli visual-debug` to produce per-stage/per-layer PNG
evidence. A tap never adds a module, WIT, or **Blackboard** API and never
changes slice geometry — it reads IR the pipeline already committed.

### Tap class
Which capture mechanism a **visual-debug tap** uses, determined by where its
source IR lives (ADR-0040). Three classes: **Blackboard-read** (source is a
whole-print **Blackboard slot** committed during prepass; captured post-prepass
with no per-layer work), **arena** (source is an `apply` commit into a per-layer
`LayerArena` slot; per-layer closure truncatable at the furthest tap), and
**PostPass-whole-print** (source is post-finalization `Vec<LayerCollectionIR>`
or emitted `GCodeIR`; requires running the full pipeline prefix). A request's
furthest selected tap fixes the class, and therefore the cost, of the run.

### Dependency closure (visual-debug)
The minimal set of pipeline work executed to reach a request's furthest selected
**tap**, then stop. For Blackboard-read taps it is prepass through the committing
stage; for arena taps, prepass plus the per-layer stage sequence truncated at the
tap over only selected layers; for PostPass taps it is the whole-print prefix
(all layers → finalization → postpass). Every executed-but-unrendered expansion
and its reason is recorded in the bundle manifest.

### Blackboard slot
One write-once field of the **Blackboard** holding a whole-print IR product
(`SurfaceClassificationIR`, `LayerPlanIR`, `SeamPlanIR`, `SupportGeometryIR`/
`SupportPlanIR`, `SliceIR`, `RegionMapIR`), committed during prepass and read via
its accessor. The committed slot accessor is the read boundary a Blackboard-read
**tap** captures from — an owned clone of the slot payload, never a live borrow.

### Overlay (visual-debug)
A toggleable class of machine events (`travel`, `seams`, `retractions`,
`z_hops`, `tool_changes`) rendered by `pnp_cli visual-debug` as its own
isolated image — faint gray base geometry plus only that class's glyphs —
never composited with other overlays.
_Avoid_: marker layer, annotation layer

### Glyph (visual-debug)
The fixed shape encoding one overlay event kind in a rendered image (seam =
circle, retraction = down-triangle, unretraction = up-triangle, z-hop =
diamond, tool change = square, travel = dotted polyline with endpoint marks).
Shape is the primary distinguisher — a glyph must be identifiable without
relying on color.

### Overlay event
The structured JSON mirror of one rendered glyph, recorded on the image's
manifest entry (`overlay_events`): position in mm plus the event's numeric
payload (retraction length, z-hop height, tool indices, travel polyline and
total length). The manifest mirror and the drawn glyphs come from one
collection pass, so image and data cannot disagree. The events are the
LLM-primary channel; the PNG is confirmation.

### Tool map
A geometry visualization colored by resolved tool index (`color_by: "tool"`)
instead of extrusion role. Colors come from a fixed high-contrast per-index
palette by default, or from the config's authored filament colors
(`tool_color_source: "filament"`) — never inferred.

### Region-key join
Matching `RegionMapIR.entries` (keyed by `RegionKey { global_layer_index,
object_id, region_id, variant_chain }`) to the `SliceIR` region carrying the same
four identifiers, so RegionMapping can be rendered as real region geometry tinted
by its `RegionPlan` dispatch/config instead of a fabricated diagram (ADR-0037
Amendment, packet 161).
