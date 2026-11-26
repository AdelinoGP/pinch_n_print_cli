# 35 — Author packet P28 — Strength / Advanced (Strength) — infill modules

Type: task
Status: resolved
Assignee: Adelino Penedo (agent session)
Blocked by: 06, 105, 107
Map: ../map.md

## Question

Author the spec packet for **P28 — Strength / Advanced (Strength) — infill modules** — 3 keys, Tier B new logic, owner infill modules. Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P28 — Strength / Advanced (Strength) — infill modules):

`align_infill_direction_to_model`, `detect_narrow_internal_solid_infill`, `minimum_sparse_infill_area`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Resolved as two folds and one direct implementation — no new packet** (user ruling,
2026-09-03). P28's three keys sit at three different seams, and ticket 04's
`infill modules` owner column is wrong for two of them.

| Key | Where it actually lands | Disposition |
| --- | --- | --- |
| `align_infill_direction_to_model` | `slicer-model-io` + config resolution + the fill modules' angle resolver | folded into packet **262a** |
| `detect_narrow_internal_solid_infill` | the module holding `claim:top-fill`, over the `InternalSolidInfill` role | folded into packet **262b** |
| `minimum_sparse_infill_area` | host `PrePass::ShellClassification` | **implemented in this session** |

## Why folds, not a packet

Both folded keys are *operators on decisions another packet is already building*,
and neither can be wired without that packet's code:

- **`align_infill_direction_to_model`** adds the object's Z-rotation to the fill
  angle **after** canonical `calculate_infill_rotation_angle` has applied the
  direction key and the rotate template (canonical `Fill.cpp::group_fills`). Packet
  262a is the packet that builds that resolver. Wiring the offset anywhere else
  would mean writing the resolver twice, and the ordering is load-bearing: fold it
  in before the template and a `"0,90"` template on a 30°-rotated object yields
  0°/90° instead of 30°/120°. 262a's new AC-14 fails exactly that mistake.
- **`detect_narrow_internal_solid_infill`** overrides the internal-solid *pattern*
  for narrow sub-areas, and packet 262b is the packet that turns
  `internal_solid_infill_pattern` into a `claim:top-fill` holder mapping.

## The three findings that made the fold the right call

1. **The object's rotation does not survive loading.** `resolve_object`
   (`crates/slicer-model-io/src/loader.rs`) composes each 3MF build item's
   transform, bakes it into the mesh vertices, and then the `ObjectMesh`
   constructor writes `transform: identity_transform()`. `ObjectMesh.transform` is
   therefore the identity for **every** loaded object, and canonical's
   `atan2(m(1,0), m(0,0))` of `object->trafo()` has no counterpart to read. So the
   key is not "declare and wire" at all — it needs a model-io/IR change first.
   262a now owns that as its Step 7.
2. **This port had no internal-solid fill *domain*.** `SlicedRegion::internal_solid_fill`
   is a *marker* (`top_solid_fill − top_solid_seed`, used by `arachne-perimeters`
   to find the exposed top and by internal-bridge detection to find what is not
   sparse); nothing fills it. The `InternalSolidInfill` role comes from a
   **per-region** `top_shell_index` / `bottom_shell_index` ≥ 1 — see
   `solid_fill_role` (`modules/core-modules/rectilinear-infill/src/lib.rs`). Two
   consequences: canonical's per-polygon pattern override cannot be a second claim
   holder (the claim seam is per region), so the narrow split has to live inside
   the holder module; and a polygon reclassified from sparse to solid has no
   dedicated vector to land in. *(Superseded in part, 2026-09-04: the domain now
   exists — see "The internal-solid domain" above. The claim-seam consequence
   stands; the "no vector" consequence was fixed by the five-way partition.)*
3. **No concentric filler exists yet.** `InfillPattern::Concentric` is in the IR
   enum, but the only concentric generator in the tree is the wall path;
   `arachne_parity.rs` records the absence as
   `D-104f-CONCENTRIC-INFILL-NO-ARACHNE`. Packet 264 ships a `concentric-infill`
   module, so 262b's fold puts the loop generator in `slicer-sdk`
   (`narrow_solid::concentric_loops`) and requires 264 to consume it rather than
   write a second copy.

## What landed in the tree — `minimum_sparse_infill_area`

Canonical, read not assumed: `LayerRegion::process_external_surfaces`
(`LayerRegion.cpp`) guards on `!spiral_mode && sparse_infill_density > 0`, computes
`min_area = scale_(scale_(minimum_sparse_infill_area))`, erases every internal
(sparse) expolygon with `area() <= min_area` from the sparse expansion zone, and
unions them into the internal-solid zone.

- **`crates/slicer-ir/src/resolved_config.rs`** — new cli-bound field
  `minimum_sparse_infill_area: f32 = 15.0` (canonical's `coFloat` default).
- **`crates/slicer-core/src/polygon_ops.rs`** — `expolygon_area`, promoted to a
  public helper (the shoelace/holes computation previously existed only as a
  private fn in `bridge_over_infill`).
- **`crates/slicer-runtime/src/slice_postprocess_prepass.rs`** —
  `convert_small_sparse_islands`, a new pass in the host built-in
  `PrePass::ShellClassification`. It runs after the two shell passes (so the solid
  roles are final) and before the bridge gates (so an island that just turned solid
  counts as solid support for the layer above, matching canonical's ordering of
  `process_external_surfaces` before bridge detection).

### The internal-solid domain (2026-09-04, user ruling)

The islands first landed in `bottom_solid_fill` with a `bottom_shell_index =
Some(1)` stamp (see the divergence history below). The user then ruled the
precedent sufficient to give internal solid infill its own classification
domain, and that was implemented the same day:

- **`region_partition::sync_perimeter_infill_areas_into_slice`** partitions by
  the five-way precedence `bridge > bottom > top > internal > sparse`. The
  PrePass shell-band marker content of `internal_solid_fill` (a subset of
  `top_solid_fill`) carves to empty; converted islands survive as the bucket's
  only polygons — canonical's `stInternalSolid` fill zone. The two
  no-perimeter skip arms carve the marker out too, so it never double-fills
  against `top_solid_fill`.
- **The `claim:top-fill` holders** (`rectilinear-infill`, `gyroid-infill`) emit
  `region.internal_solid_fill()` as `ExtrusionRole::InternalSolidInfill`,
  gated on `should_emit(ExtrusionRole::TopSolidInfill)` (the claim that owns
  solid fill; `should_emit` itself maps `InternalSolidInfill` to
  always-allowed, so the gate must name the owning claim).
- **`perimeter-region-view` gains `internal-solid-fill`** (WIT, host resource,
  SDK view + macro adapter, drift-test member list) so the infill linker clips
  `InternalSolidInfill` paths against the partitioned bucket; its
  `RoleBoundaries::for_role` unions it into the internal-solid boundary.
- **`wave-overhangs`** unions the bucket into its solid support geometry.
- **The `bottom_shell_index = Some(1)` stamp is retired.** The role now comes
  from the bucket itself — canonical's per-surface role decision. This removes
  the IR lie (a depth index that was not a shell depth) and the recorded
  `Some(0)` divergence: a converted island in a mixed region now emits at the
  internal-solid width/speed, not the exposed bottom's.

### Recorded divergences

- **The sparse zone is measured before the wall inset.** Canonical runs this after
  `make_perimeters`, so its `fill_surfaces` are already inset. This port classifies
  shells in a prepass that runs before perimeters exist, so the zone is
  `infill_areas − (top ∪ bottom ∪ bridge)`. Every island measures *larger* than
  canonical would measure it, so the conversion is strictly **conservative** — a
  subset of what canonical converts, never a superset. The alternative seam
  (`region_partition::sync_perimeter_infill_areas_into_slice`, where the
  post-inset sparse zone does exist) would have got the area right and the
  *ordering* wrong: it runs at `Layer::Perimeters`, after the prepass bridge
  detection that canonical performs downstream of this conversion.
- **`spiral_mode` has no counterpart**, so canonical's guard reduces to the density
  test. `spiral_mode` is still an unimplemented queue key.
- **Historical (retired 2026-09-04, see "The internal-solid domain" above):** the
  first landing rode `bottom_solid_fill` with a `bottom_shell_index = Some(1)`
  stamp, which made a converted island in a mixed region inherit
  `bottom_shell_index == Some(0)` and emit as `BottomSolidInfill` at the exposed
  bottom's width/speed. Both are gone — islands land in the dedicated
  `internal_solid_fill` bucket.

### Verification

`cargo test -p slicer-runtime --test executor minimum_sparse_infill_area` — 4
passed, 4 new. Each compares two runs differing only in the key under test:

- `minimum_sparse_infill_area_converts_island_at_or_below_threshold` — a 9 mm²
  island converts at threshold 15 and stays sparse at threshold 5; the converted
  case is stamped `bottom_shell_index == Some(1)`
- `minimum_sparse_infill_area_leaves_large_islands_sparse` — 100 mm² never converts
- `minimum_sparse_infill_area_is_disabled_at_zero_and_for_hollow_regions` — both
  halves of the guard
- `minimum_sparse_infill_area_converts_only_the_small_island_of_a_mixed_layer` —
  per-expolygon, not per-layer: the 9 mm² island moves and the 100 mm² one does not

No-regression: `cargo xtask test --summary -p slicer-runtime` — 8 binaries,
582 passed, **1 failed**, and that failure is **pre-existing at HEAD**:
`modifier_support_territory_top_shell_layers_keep_full_walls_and_internal_bridge_role`
fails with `wipe-tower corner (63.000, 232.972) lies outside bed polygon`.
Reproduced identically on the stashed baseline through `cargo xtask test`, so the
guest-freshness gate ran against the baseline tree. Not caused by this ticket; see
the map's **Not yet specified** entry filed for it.

## What the folds obligate

Packet **262a** gains AC-11 … AC-14, a new Step 7, and a widened change surface
(`ObjectMesh.model_rotation_z_rad`, two `ResolvedConfig` fields,
`crates/slicer-runtime/src/run.rs`). Packet **262b** gains AC-13 … AC-16, AC-N5,
`DIV-8`/`DIV-9`, a new Step 7b, and a carve-out to edit `rectilinear-infill` after
262a lands. Neither adds a WIT interface or an IR schema-version bump.

`detect_narrow_internal_solid_infill` ships at canonical's `true` default, so it
**does** change output on geometry that has narrow internal-solid areas. 262b's
AC-N5 bounds that: no narrow area, and exposed depth-0 surfaces, stay byte-identical.

