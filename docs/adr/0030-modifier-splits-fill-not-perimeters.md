# ADR-0030 — Modifier Volumes Split the Partitioned Fill Polygons into Wall-Less Sub-Regions; Perimeters Stay Merged

## Status

Proposed (lands with packets `131_per-region-config-delivery` and `132_modifier-region-split`;
follows the 2026-07-01 infill-parity grilling).

## Context

Users routinely place infill-modifier volumes inside a part to raise infill density locally
(stiffness control). The OrcaSlicer reference behavior: **one** set of perimeter walls around
the object outline, the fill area **partitioned** at the modifier boundary, each sub-area
filled at its own density/pattern, each pattern anchored and connected along its own boundary —
including the wall-less shared arc — with no walls generated at the modifier boundary.

As of 2026-07-01, this use case is non-functional end-to-end in PnP, verified in code:

- The loader parses modifier volumes into `ObjectMesh.modifier_volumes`
  (`crates/slicer-model-io/src/loader.rs:547-622`, `sidecar.rs:15`) — ingestion works.
- But `stamp_modifier_config_deltas` applies modifier config **globally per object** — the
  only in-use `ModifierScope` variant is `AllFeatures`, with an explicit "no bbox/polygon
  overlap check" comment (`crates/slicer-core/src/algos/region_mapping.rs:266-268,615-624`).
- No geometric split exists: `prepass_slice.rs:286` slices only the solid mesh; modifier
  meshes are never intersected with the cross-section.
- Even with a split, per-region config could not reach a module: the dispatch builds ONE
  global `ConfigView` from the FIRST `RegionKey` matching the layer index
  (`crates/slicer-wasm-host/src/dispatch.rs:1633-1637`) — which is also a latent
  wrong-config bug for painted multi-region layers.

## Decision

1. **Modifier volumes split fill areas, not perimeters.** At region partition, the host slices
   each modifier-volume mesh at the layer Z, intersects the modifier cross-section with the
   owning region's partitioned fill polygons (`sparse_infill_area`, `top_solid_fill`,
   `bottom_solid_fill`, `bridge_areas`), and splits them into **wall-less sub-regions**:
   sub-regions carry their own `region_id` and config binding but share the base region's
   walls. No wall loops are generated at the modifier boundary. This is the deliberate
   contrast with paint/MMU splits, which DO produce per-region perimeters.

2. **Wall sharing is first-class.** Each modifier sub-region's `wall-source-region-id` (the
   `perimeter-region-view` field added by ADR-0028 §Amendment) points at the base region.
   This places modifier sub-regions in the base's **wall-sharing group**, which is exactly the
   scope in which the infill-linker may treat boundaries as wall-less (ADR-0025 §Amendment:
   different-config siblings link per-region along their own boundary with no overlap inset on
   wall-less arcs).

3. **Per-region config delivery is the companion change.** The first-match `ConfigView` is
   replaced with per-region config access for guest modules (region views gain a config
   accessor; additive WIT bump), so an infill module reads the modifier's density for the
   sub-region and the base density elsewhere. `ModifierScope` is extended beyond `AllFeatures`
   to carry the geometric scope.

## Consequences

**Positive**:
- The recurring local-stiffness use case works end-to-end, matching the OrcaSlicer reference
  behavior (one wall set, partitioned fill).
- The wall-sharing-group machinery is shared with paint virtual-variants — one linker code
  path covers both producers of wall-less siblings.
- Fixing the first-match `ConfigView` also fixes the latent arbitrary-config bug for painted
  multi-region layers.

**Negative**:
- Region counts grow on modifier-bearing layers; every per-region loop downstream pays the
  iteration cost (bounded by modifier count).
- The `ConfigView` fix can change output for existing painted fixtures (they currently read
  whichever region's config a BTreeMap yields first) — golden re-blessing is scheduled in the
  roadmap's integration packet.
- Solid-shell partition precedence (`bridge > bottom > top > sparse`) now composes with the
  modifier split; the split applies to the already-partitioned polygons so precedence is
  unchanged, but tests must pin the composition.

**Trade-offs we explicitly accept**:
- Modifier boundaries produce no walls, so a density transition is visible on top surfaces
  only through the solid-shell layers above it (same as OrcaSlicer). Users who want a hard
  boundary use a separate object, not a modifier.
- Z-interval scoping (modifier active only across its own Z range) falls out of slicing the
  modifier mesh per layer — an empty cross-section means no split on that layer.

## Future-Reviewer Notes

- **Do not generate perimeters at modifier boundaries "for consistency with paint splits".**
  The wall-less boundary is the point: it matches OrcaSlicer and is what makes the linker's
  own-boundary anchoring produce the reference picture.
- **Do not deliver per-region config by stamping deltas onto the global object config.** That
  is the retired mechanism; per-region delivery goes through the region-view config accessor.

## References

- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` §Amendment 2026-07-01 — wall-sharing
  groups + the two linking branches.
- `docs/adr/0028-infill-postprocess-contract-prior-ir-and-partitioned-polygons.md` §Amendment
  2026-07-01 — `wall-source-region-id` field.
- `docs/specs/modifier-region-infill.md` — the phase plan (M1/M2/M3).
- `crates/slicer-core/src/algos/region_mapping.rs:266-268,615-624` — global stamping (retired).
- `crates/slicer-wasm-host/src/dispatch.rs:1633-1637` — first-match ConfigView (retired).
- `crates/slicer-model-io/src/loader.rs:547-622` — modifier-volume ingestion (kept).
- `crates/slicer-runtime/src/region_partition.rs` — partition site gaining the split.
