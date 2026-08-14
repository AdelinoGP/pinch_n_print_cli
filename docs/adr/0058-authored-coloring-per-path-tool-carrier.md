# 0058 Authored coloring: per-path tool carrier with a two-sided grant

Modules previously had no control over coloring — tool is host-resolved per
region (`resolve_region_tool_index`). The Dragon Curve community module needs to
vary tool per scan line by tiling, so we add a per-path `tool-index: option<u32>`
field to the `extrusion-path3d` WIT record, and gate its use behind a two-sided
grant: the module must disclose `claim:authored-coloring` in its manifest AND its
fill-role claim must be listed in the `fill_authored_coloring` setting. The host
strips any ungranted `Some(tool)` at the marshal boundary and colors per region
as before. When granted, the per-path tool overrides the region-resolved tool
(including a material-variant tool).

## Status
Accepted (Dragon Curve community module; spec:
`docs/specs/community-modules-dragon-curve-infill.md`).

## Considered Options
- **Field on `extrusion-path3d` (chosen).** Survives the infill linker, which
  clones and re-emits `ExtrusionPath3D` and produces new clipped/re-linked paths
  inside `chain_or_connect_infill`. Support/finalization stages also use the type
  but set `None`. Cost: a versioned `slicer:types/geometry` WIT bump ripples to
  all guests and the wasm host.
- **Per-path tool side-list on the infill-output-builder (rejected).** Narrower
  blast radius, but breaks at the linker's new-path production — the linker
  cannot carry a parallel list through `chain_or_connect_infill`.
- **Emit separate entities (rejected).** No new WIT shape, but no existing
  per-entity tool seam for infill output; less clean and does not survive linking
  any better.

## Consequences
- `extrusion-path3d` consumers in support/finalization now carry an unused `None`
  tool; behaviorally unchanged.
- The infill linker's `paths_compatible` must add tool equality and split/refuse
  chains across differing per-path tools — the same guard it already applies at
  region level (`compatible_regions` / `majority_owner`).
- Per-line tool changes raise the tool-change count and therefore wipe-tower
  purge volume. Known cost, not a correctness break.
- No OrcaSlicer precedent; recorded as a PnP deviation (`docs/DEVIATION_LOG.md`).

## Amendment — 2026-08-13 (packet 226)

The two-sided grant, the field-on-the-path decision, the linker tool-equality
guard, and the accepted purge-volume cost all stand. Two points of reasoning are
corrected.

### 1. The side-table rejection was argued on the wrong grounds

#### Retired clause (verbatim, Considered Options)

> - **Per-path tool side-list on the infill-output-builder (rejected).** Narrower
>   blast radius, but breaks at the linker's new-path production — the linker
>   cannot carry a parallel list through `chain_or_connect_infill`.

#### Replacement

That argument holds only for a *positional* side-list, which is not this
project's established pattern. The established pattern is a **keyed side table**:
ADR-0052 Decision 2 puts the per-point speed carrier in an `entity_id`-keyed side
table on `LayerCollectionIR`, and states explicitly that it is "Not a per-point
field on `Point3WithWidth`". A keyed side table survives cloning and re-emission
by construction, so the linker argument does not reject it.

The real reason a keyed side table does not work here is **identity timing**.
Keying requires a stable per-path identity, and `entity_id` is not assigned until
`assemble_ordered_entities`, which runs *after* the guest authors infill and
*after* the linker splices paths. A guest therefore has no key to write against
at authoring time. Minting one would mean adding an `id` field to
`ExtrusionPath3D` — the same workspace-wide WIT + IR sweep as adding
`tool_index` itself, for strictly less: an extra indirection and a second
structure to keep consistent through linking.

### 2. Field-residency check against ADR-0032

ADR-0032's governing principle, quoted forward in ADR-0052, is that a field earns
IR/WIT residency only when it is produced and consumed by *different* modules
across the guest boundary. `tool_index` qualifies: it is produced by an infill
guest and consumed by the host emitter and the infill linker.

Worth recording as the bar this clears: `ExtrusionPath3D` has never gained a
field since its introduction (2026-03-15, commit `84385a90`) — `tool_index` is
the first — while `Point3WithWidth` grew from 5 to 8 fields over the same period.
