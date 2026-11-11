# Overhang band partition deletion: derive the quartile from the distance the consumer already has

Type: task / measured candidate
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Overhang quartile bands: is the 4-band polygon partition read by anything?](141-overhang-quartile-band-deletion.md)'s
  2026-11-11 Phase-1: the quartile **value** has no production reader, the only
  read is a presence bit equal to the sign of the signed distance the consumer
  already computes, and the partition is **9.6–10.6 s** of the base
  classic-off wall (24–26 % of it, 37–40 % of the cell's required cut).

## Question

Delete `partition_into_bands` from `PrePass::OverhangAnnotation` and derive
`overhang_quartile` — presence, and the bucket value if anything ever reads it
— from the `signed_distance_to_boundary` the perimeter annotation already
computes, in the shape that keeps the matched job's G-code byte-identical.

Two forks from the Phase-1, both to be settled by measurement, not assumption:

1. **The layer key.** `annotate_overhangs` returns the previous-layer boundary
   only for layers that have overhang, which is why `overhang_distance_mm` is
   `None` on a layer with none. Shape A keeps the per-layer
   `difference_ex(current, previous)` key test and deletes only the partition
   (expected saving ≥ 85.4 % of the stage wall, t84's offset share). Shape B
   populates the boundary on every layer and deletes the whole stage; it is
   only admissible if `insert_extended_points_with_point_widths` provably
   inserts nothing on a `current ⊆ previous` layer and no other distance reader
   changes behaviour.
2. **The debug/IR surface.** `overhang_quartile_polygons` feeds
   `visual_debug_render`'s band silhouettes and the preparation-input keys. The
   preferred shape leaves the IR field empty on the production path and makes
   the renderer reconstruct bands from `prev_layer_boundary` + the resolved
   line width at render time; retiring the tap is the alternative. Neither may
   cost anything on the slice path.

Where the distance-derived quartile lands matters: the producer resolves the
band width as `outer_wall_line_width` falling back to `line_width` (0.4 in the
matched job), while the consumer knows the per-point extrusion `width`. The
threshold source must be the producer's width, so the context needs the scalar
(`line_width_mm` or the three thresholds), not the polygons.

Gate — the standing contract, unchanged:

- **Output:** byte-identical G-code in the 16 arm/cell cells, or a delta
  audited before timing, confined to `E` and to overhang-speed sections. The
  Phase-1 argument says the only possible disagreement is a point lying on the
  previous boundary; a non-zero delta must be shown to be exactly that set.
- **Timing:** median uninstrumented wall at the **0.80 %** paired floor with
  process CPU corroborating, in **all 8** cells (the shape touches every cell),
  with no cell's wall or CPU regressing.
- **Freshness:** `cargo xtask build-guests --check` exit 0 before attributing
  anything to a guest; `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo xtask check-literals` before committing.
- **Evidence:** a new `evidence/t142-…/` set with `verify.py` and negative
  controls; the reference set is t139's unless the output moves.

## Comments

_None yet._
