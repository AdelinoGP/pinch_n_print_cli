# Modifier-Volume Infill Regions — Local Density Control (M1/M2/M3)

## Context

Companion to `docs/specs/infill-parity-rectilinear-gyroid-linker.md`. The recurring OrcaSlicer
use case: an infill-modifier volume inside a part raises infill density locally (stiffness
control). Reference behavior (verified against an OrcaSlicer gcode preview, 2026-07-01): one
set of perimeter walls around the object outline; fill area partitioned at the modifier
boundary; each sub-area filled at its own density/pattern; each pattern anchored and connected
along its own boundary — including the wall-less shared arc — by small boundary-walk
polylines; **no walls at the modifier boundary**.

As of 2026-07-01 this is non-functional end-to-end in PnP (evidence in ADR-0030 §Context:
modifier config stamped globally, no geometric split, first-match `ConfigView`). This spec is
the phase plan; the architecture decision is ADR-0030.

**Status: implemented.** M2 (per-region config delivery) landed with packet
`131_per-region-config-delivery`; M1 (geometric split) landed with packet
`132_modifier-region-split`; M3 completed inside packet `136_infill-parity-integration`.
See `docs/07_implementation_status.md` TASK-256/TASK-257/TASK-261. Delivered refinements
recorded below:

- **Sub-region identity** (M1 delivery): an unpainted parent produces a child with
  an empty `variant_chain`; a painted parent produces a child retaining the
  parent's chain and using the painted parent ID in the modifier-namespace
  payload. The shared `modifier_sub_region_id` helper carries a high-bit
  namespace marker over the `base_region_id * MODIFIER_VARIANT_REGION_ID_STRIDE +
  hash` payload, where the hash is derived from the object ID and sliced
  modifier footprint. The `wall_source_region_id` predicate masks the namespace
  bit before recovering the parent ID. See `docs/02_ir_schemas.md` IR 5
  §"Sub-region `region_id` namespace".
- **Prepass materialization** (M1 delivery): modifier meshes are sliced during
  `split_modifier_sub_regions_for_prepass`; the same shared splitter is reused
  by the Tier-2 footprint fallback, so both paths use identical geometry and
  priority semantics.
- **Overlap precedence** (M1 delivery): for overlapping non-support modifier volumes,
  priority is applied first and document order breaks ties — the first winning modifier
  owns the footprint; later modifiers intersect only the remaining base area.

Packet mapping: `131_per-region-config-delivery` (M2), `132_modifier-region-split` (M1),
M3 folds into `136_infill-parity-integration`.

## Authoritative references

- `docs/adr/0030-modifier-splits-fill-not-perimeters.md` — the decision (read first).
- `docs/adr/0028-…` §Amendment — `wall-source-region-id` on `perimeter-region-view`.
- `docs/adr/0025-…` §Amendment — wall-sharing groups; different-config siblings link
  per-region with no overlap inset on wall-less arcs.
- `crates/slicer-core/src/algos/region_mapping.rs:266-268,615-624` — global stamping (retired by M1).
- `crates/slicer-core/src/algos/prepass_slice.rs:286` — solid-mesh-only slicing (M1 adds modifier-mesh slicing).
- `crates/slicer-runtime/src/region_partition.rs` — partition site gaining the fill-polygon split (M1).
- `crates/slicer-wasm-host/src/dispatch.rs:1629-1645` — first-match ConfigView (retired by M2).
- `crates/slicer-model-io/src/loader.rs:547-622` + `sidecar.rs:15` — modifier ingestion (unchanged).

## Phase M2 — Per-region config delivery (packet 131; ordered before M1 so the fix is
independently testable on existing painted multi-region fixtures)

- Replace the first-match `ConfigView` derivation: each region dispatch iteration resolves the
  `RegionKey`-matched `ResolvedConfig` from `RegionMapIR`'s interned pool
  (`slice_ir.rs:1176-1185`).
- Guest surface: region views (`slice-region-view`, `perimeter-region-view`) gain a config
  accessor so per-region values (e.g. `infill_density`, `line_width`, speed keys) are readable
  inside the module's per-region loop. Additive WIT bump; guest rebuild ceremony.
- Behavior guard: single-region layers read exactly the config they read before (negative AC).
- NOTE: painted multi-region fixtures may legitimately change output (they currently read an
  arbitrary region's config). This opens the roadmap's golden carve window — survey + carve
  list starts here (see roadmap D6).

## Phase M1 — Modifier geometric region split (packet 132; host-only, no WIT change)

- Slice each `ModifierVolume` mesh at the layer Z (`slice_mesh_ex` on the modifier mesh);
  empty cross-section ⇒ no split on that layer (Z-interval scoping falls out).
- Intersect the modifier cross-section with the owning region's four partitioned fill polygons
  and split them into wall-less sub-regions: own `region_id` + config binding
  (`ModifierScope` extended beyond `AllFeatures`), `wall-source-region-id = base`,
  **no wall loops at the modifier boundary** (walls stay merged on the base region).
- Precedence composition: the split applies to the already-partitioned polygons, so
  `bridge > bottom > top > sparse` precedence is unchanged — pinned by test.
- Negative AC: objects without modifier volumes produce byte-identical `SliceIR`.

## Phase M3 — End-to-end integration (inside packet 136)

- Fixture: a cube with a centered modifier volume (sphere or cylinder), base density ~15%,
  modifier density ~40%, both regions `rectilinear-infill` (plus a gyroid variant if cheap).
- Asserts: one wall set only (no wall loops at the modifier boundary); two distinct line
  spacings in the emitted infill; every infill path inside its sub-region's polygon; linked
  polylines anchored along each sub-region's own boundary including the shared arc; no
  unfilled ring at the shared boundary (the no-inset-on-wall-less-arcs rule, ADR-0025
  amendment branch (b)).
- HTML-report visual check against the OrcaSlicer reference behavior.

## Risks

- **Region-count growth** on modifier-bearing layers; downstream per-region loops pay
  iteration cost (bounded by modifier count).
- **ConfigView fix churn**: painted fixtures change output at M2 — planned golden carve, see
  roadmap D6.
- **Split geometry robustness**: modifier meshes may be open/self-intersecting; reuse the
  slicing repair path used for solid meshes; degenerate intersections fall back to no-split
  (base config), never to a crash.
