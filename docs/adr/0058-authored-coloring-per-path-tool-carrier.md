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
