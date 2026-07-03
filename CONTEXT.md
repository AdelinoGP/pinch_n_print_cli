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
behind local-stiffness infill modifiers. See ADR-0030.

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
