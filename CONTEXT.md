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
