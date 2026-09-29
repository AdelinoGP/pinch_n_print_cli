# T27 candidate premise audit — 2026-09-28

This corrects the candidate interpretations in [probe iteration 1](FINDINGS-SUBSTAGE.md)
and [probe iteration 2](FINDINGS-SUBSTAGE-2.md). Those files remain the
historical probe record; their measured brackets are not changed.

## Phase-B sibling-region offset memo: drop for the matched job

`gate_internal_bridge_sites` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`)
iterates timelines keyed by `(object_id, region_id)` from
`build_region_timelines`. Its `deep_infill_area` is derived from that region's
depth window and from already-committed lower `internal_bridge_areas`; it is
not necessarily a common full-layer input across sibling regions. The
negative offset is already lazy (only after qualification survives). Even if
the inputs happened to match, sharing would need the actual input geometry
and spacing checked, not merely `(object, layer, spacing)`.

Both matched STL jobs contain one object; the layer planner's base region is
`region_id = 0`, and paint/modifier sub-regions arise after ShellClassification.
A no-code-change diagnostic run with
`RUST_LOG=slicer_runtime::slice_postprocess_prepass=debug` and the classic
supports-on matched config recorded skip events in
`target/t27-audit/{benchy,base}-on.stderr.log` (gitignored). A census of
`internal bridge skip` events found:

| Job | Plan layers | Skip events | Region IDs | Most skip events per object/layer | Reasons |
| --- | ---: | ---: | --- | ---: | --- |
| benchy-on | 240 | 226 | `0` only | 1 | 192 `internal_solid_fill_empty`, 4 `unsupported_empty`, 30 `qualified_empty_or_deep_sparse_clip` |
| base-on | 495 | 484 | `0` only | 1 | 371 `internal_solid_fill_empty`, 2 `unsupported_empty`, 111 `qualified_empty_or_deep_sparse_clip` |

The census covers logged skips, not a direct count of successful offset calls;
the single-object/base-region structure plus the timeline key rules out
cross-region reuse in these jobs. Iteration 2's measured offset work (base-on:
12.07 s/121 deep-clip calls; 9.34 s/36 erosion calls) remains real, but the
claimed *up-to-20 s* saving from a sibling memo is unsupported. Do not A/B
that memo on these fixtures; it has no eligible siblings. A new within-timeline
work-skip would need its own input/representation proof and hit-rate probe.

## Overhang footprint: exact intersection is a production consumer

`classify_object` (`crates/slicer-core/src/algos/mesh_analysis.rs`) supplies the
unioned overhang `xy_footprint`. Its local `region_needs_support` uses
`expolygon_bbox`, but `SliceRegionView::derive_needs_support`
(`crates/slicer-sdk/src/views.rs`) calls `intersection_ex(&self.polygons,
&overhang.xy_footprint)` and returns true on non-empty intersection. The
runtime `commit_support_analysis_builtin` calls this per region, and the
native, prepared and inbound WASM marshal paths also call it. This is an
**exact overlap predicate**, not a bbox predicate: disjoint triangles can have
overlapping bounding boxes. The regression test
`slice_region_view_derive_needs_support_overlapping_bboxes_but_disjoint_polygons_is_false`
(`crates/slicer-sdk/tests/layer_module_tdd.rs`) protects that distinction.
`visual_debug_render` (`crates/slicer-runtime/src/visual_debug_render.rs`)
draws the overhang footprint; `assemble_bridge_areas`
(`crates/slicer-core/src/algos/prepass_slice.rs`) intersects **bridge**
footprints, a distinct field.

Thus the iteration-1 sentence "no consumer needs the exact union" and the
suggested bbox replacement are unsafe. A representation-safe alternative
might preserve *exact facet coverage* without eagerly unioning it, but must
first prove eligibility and visual-debug/IR output behavior, including
degenerate and touching geometry, before any wall A/B. No candidate binary or
speed claim is produced by this audit.

## Flat-bridge representation analysis (next experiment)

`assemble_flat_bridge_areas` (`crates/slicer-core/src/algos/prepass_slice.rs`)
already returns when the bottom footprint, unsupported region,
`flat_unsupported`, or `flat_bridge` is empty. Its hot path computes
`support = difference(region.polygons, unsupported_region)`, then
`closed = closing_ex(support, 12 mm, closing_join)`, then
`refilled = difference(closed, support)`, and uses the area of each
`intersection(component, refilled)` to decide whether to emit the **full**
component. `unsupported_region` is computed outside that function once per
region from the current and preceding raw slices; the bottom footprint is
cached once per object. For the matched single-region jobs, neither
cross-region closing memoization nor reusing a sibling's unsupported input
can remove calls.

Replacing `support = region.polygons \\ unsupported` with
`intersection(current_raw, previous_raw)` is only a *geometric* identity when
`region.polygons == current_raw`, and even then Clipper's contour/order
representation can change. The closing and the 10%-of-component overlap
test amplify small contour changes into changed bridge flags. Bounding boxes
alone cannot decide enclosure (a free-edge band and a true gap can have the
same bbox). The existing `flat_bridge_enclosure_closing_avoids_round_arc_explosion`
(`crates/slicer-core/tests/algo_prepass_slice_tdd.rs`) and
`flat_bridge_span_over_gap_flagged_via_layer_diff`
(`crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`) tests cover positives;
an optimization needs both positive and free-edge negatives, plus a
production-layer IR/output comparison before a paired A/B. The previous
per-component bbox guard skipped none of the measured Benchy calls (see the
comment at `assemble_flat_bridge_areas`); no fresh hit rate on base has been
measured. **No representation-safe call reduction is established yet.**
