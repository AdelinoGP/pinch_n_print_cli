# 150 — Phase 5 width-limiting deletes slice area instead of returning it to BASE

Type: task
Status: open
Assignee: —
Blocked by: —
Map: ../map.md

## Question

Filed by ticket 98 (2026-09-10) from the parity audit of `run_phase5_width_limit`
against canonical `cut_segmented_layers` (`MultiMaterialSegmentation.cpp`).

**The defect.** `run_phase5_width_limit`
(`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) erodes each painted
region to a band of `mmu_segmented_region_max_width` measured in from the layer
outline, then writes back **only** regions with a non-empty `variant_chain`:

```rust
for region in &mut working[i].regions {
    if region.variant_chain.is_empty() { continue; }   // BASE skipped
    if let Some(polys) = layer_map.get(&region.variant_chain) {
        region.polygons = polys.clone();               // = difference_ex(polys, inner)
    }
}
```

The kernel `cut_segmented_layers`
(`crates/slicer-core/src/algos/paint_segmentation/width_limit.rs`) carries the
same skip (`if chain.is_empty() { continue; }`, commented "base region is left
unchanged"). BASE was computed earlier in the Phase 7 compose block as the
**residual** — `polys_by_color[&None]`, the unpainted Voronoi cells — so it does
not cover the painted interior. Nothing downstream re-derives it: the Phase 6
"full layer cross-section" is built as the union of all regions, which is now
holed.

Net effect: the painted area lying deeper than `max_width` inside the outline
belongs to **no region**. That contradicts this file's own stated invariant —
*"segmentation PARTITIONS area, it never removes it"* (the invariant backstop
immediately above the Phase 5 call site), which only guards the compose block,
not Phase 5.

**Canonical does not lose the area.** Canonical cuts *every* index including
`extruder_idx == 0` (the unpainted facet state), but its segmented regions are
**overrides on a parent `LayerRegion` that covers the whole slice**:
`apply_mm_segmentation` (`PrintObjectSlice.cpp`) splits the parent by the
per-extruder expolygons and whatever no extruder claims stays with the parent,
printing with the object's default filament. The port's regions are a partition,
not an override set, so the port's equivalent of "falls back to the parent" is
"BASE absorbs the remainder".

**Measured, 2026-09-10** (debug `pnp_cli`, `resources/cube_4color.3mf`,
`--module-dir modules/core-modules --no-default-module-paths`, default slice vs
`{"mmu_segmented_region_max_width": 2.0}`), extrusion length by `;TYPE:`:

| type | default | max_width=2.0 | delta |
|---|---:|---:|---:|
| Sparse infill | 838.12 | 283.47 | **−66%** |
| Internal solid infill | 217.92 | 98.81 | **−55%** |
| Inner wall | 1086.81 | 2266.58 | +109% |
| Outer wall | 1476.75 | 1980.40 | +34% |

Fill collapses (the interior is unowned) while walls more than double (every
surviving band gets its own perimeter loops). The object's volume did not
change. The two existing e2e gates
(`crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs`, AC-5/AC-6)
assert only that the toolpath *differs* from the default slice, so they pass on
a slice that has lost most of its infill.

**The question to resolve:** land the parity fix and the invariant that would
have caught it. Expected shape — after the kernel returns, BASE's polygons
become `layer_total_contours − union(painted after erosion)` (BASE *grows*,
matching canonical's "interior prints with the default filament"), guarded by an
area-conservation assertion over the whole region set rather than a per-region
one. Decide en route:

- whether the BASE re-derivation belongs in the kernel or in the driver's
  write-back (the kernel currently takes `input_expolygons_per_layer` as the
  union of all regions, so it already has the layer contour);
- what happens on a layer where BASE was dropped entirely (the compose block
  drops BASE when there are neither modifier annotations nor residual cells);
- whether the modifier-annotation branch (BASE = full layer contours, so BASE
  and painted regions **overlap** by construction) needs a different rule;
- whether the area-conservation invariant is asserted in the pipeline or only in
  tests.

Sized at claim time per the map's "Packets are for complex implementation only"
rule: this is a geometry change to a parity-critical pass with an invariant to
add, so it is a packet candidate, not obviously a direct in-session fix.

## Answer
