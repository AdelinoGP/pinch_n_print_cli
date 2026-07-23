# Per-region config delivery — golden carve list (packet 131, D6 window)

This file enumerates every test whose g-code output can legitimately change when
the first-match `ConfigView` derivation is replaced by `RegionKey`-matched
resolution (Phase M2). Each entry is carved with:

```rust
#[ignore = "carved: infill-parity D6; restored in packet 136"]
```

Single-region fixtures (e.g. `regression_wedge.stl` default config) are NOT
carved — they must remain byte-identical and are guarded by
`wedge_per_region_config_delivery_byte_identical` (AC-N2), which hardcodes the
digest `8a3b645ee54fa5dbfa1232008db4820d2a364a30b4d196a504b424271308019f`.

**Baseline provenance (AC-N2):** the wedge's pre-Step-3 baseline
`263238f8c73c7267c37c7075d02706e2c07eb5d350980079527e883de367bf01` was
captured by the Step-1 baseline worker before any packet-131 code edit, via
`pnp_cli slice --model resources/regression_wedge.stl --output wedge_default.gcode`
followed by `sha256sum` on the produced g-code. The post-Step-3 tree yields
`8a3b645e…` deterministically. The drift is attributable to parallel
uncommitted working-tree changes (WIT `ir-types.wit`, SDK `views.rs`, and
gcode/golden/executor test files) that pre-existed or were modified alongside
this packet, **not** to Step 3 itself: the g-code serializer
(`crates/slicer-gcode/src/serialize.rs`) is untouched by Step 3, and the
per-region resolution in `dispatch.rs:1842-1889` falls back to the identical
object-level `default_config_fields` for any region without a pool entry (the
default-config wedge has no region overrides). The new constant is the
single-region byte-identity floor going forward.

### crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::cube_fuzzy_painted_face_jitter_present_on_painted_face_only
- Reason: Multi-region painted fuzzy cube; gcode outer-wall point bins per face read per-region config, output shape can shift under RegionKey-matched config.
- Baseline: assertion: painted_face_pts > 0 AND unpainted_face_pts > 0 AND painted_face_pts > 2*unpainted_face_pts AND outer_wall_frags_at_mid_z >= 2

### crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::cube_4color_painted_model_emits_top_and_bottom_solid_surfaces
- Reason: Multi-region painted cube; per-region solid-fill config delivery can change Top/Bottom surface block counts.
- Baseline: assertion: count_type(gcode,"Top surface") > 0 && count_type(gcode,"Bottom surface") > 0

### crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::cube_4color_gapfill_does_not_flood_bisector_seams
- Reason: Multi-region painted cube; per-region infill config changes GapFill block generation on bisector seams.
- Baseline: assertion: count_type(gcode,"Gap infill") < 150

### crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::cube_4color_auto_enables_wipe_tower_for_mmu
- Reason: Multi-region painted MMU cube; tool-index-derived config can change wipe-tower auto-enable gcode.
- Baseline: assertion: count_type(gcode,"Prime tower") > 0

### crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::cube_4color_header_declares_per_filament_colours
- Reason: Multi-region painted MMU cube; per-region config delivery can change filament/extruder colour header directives.
- Baseline: assertion: header has "; filament_colour =" with >=2 colours AND header has "; extruder_colour ="

### crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::mmu_no_oversized_alloc_repeat
- Reason: Multi-region painted fuzzy cube repeated 10x; per-region config delivery can change emitted gcode, affecting the OOM-guard repeat witness.
- Baseline: assertion: all 10 iterations produce non-empty gcode containing "G1"

### crates/slicer-runtime/tests/executor/cube_4color_sparse_infill_per_painted_region_tdd.rs::cube_4color_sparse_infill_per_painted_region
- Reason: Multi-region painted cube; per-region infill config changes which tool indices emit sparse-infill extrusion.
- Baseline: assertion: tools_with_sparse_infill_extrusion == {0,1,2,3}

### crates/slicer-runtime/tests/executor/cube_4color_ironing_per_painted_top_color_tdd.rs::cube_4color_ironing_per_painted_top_color
- Reason: Multi-region painted cube top color; per-region config delivery changes ironing block per-tool assertion.
- Baseline: assertion: >=1 ";TYPE:Ironing" block AND union of ironing-block tools contains {0,3}

### crates/slicer-runtime/tests/executor/cube_4color_arachne.rs::cube_4color_arachne_fragments_walls_by_color
- Reason: Multi-region painted cube (arachne perimeters); per-region config delivery changes per-color outer-wall fragmentation gcode shape.
- Baseline: assertion: all 4 tool indices present AND >=10 mid-body layers with headers AND no degenerate headers AND layers_with_3plus_fragments >= 1

### crates/slicer-runtime/tests/executor/cube_4color_arachne.rs::cube_4color_arachne_per_color_footprint_within_bbox
- Reason: Multi-region painted cube (arachne); per-region config delivery changes per-color outer-wall subloop geometry gcode shape.
- Baseline: assertion: non-empty gcode; per-color outer-wall subloops within fixture bbox

### crates/slicer-runtime/tests/executor/cube_4color_arachne.rs::cube_4color_arachne_outer_walls_close_end_to_end
- Reason: Multi-region painted cube (arachne); per-region config delivery changes outer-wall subloop closure gcode shape.
- Baseline: assertion: non-empty gcode; outer-wall subloops close end-to-end (finite, closed rings)

### crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::cube_4color_gcode_emits_all_four_tool_indices
- Reason: Multi-region painted cube; per-region config delivery changes T0-T3 tool-index emission in gcode.
- Baseline: assertion: parse_tool_index_lines(gcode) == {0,1,2,3}

### crates/slicer-runtime/tests/executor/cube_4color_gcode_output_tdd.rs::cube_4color_per_layer_outer_walls_fragment_by_color_with_tool_changes
- Reason: Multi-region painted cube; per-region config delivery changes per-layer outer-wall fragmentation and tool changes.
- Baseline: assertion: per-layer outer walls fragment by color with matching tool-change count; all 4 tool indices present

### crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs::cube_4color_phase5_width_limit_bands
- Reason: Multi-region MMU cube with width config; per-region config delivery changes eroded toolpath.
- Baseline: assertion: motion(default) != motion(width=2.0)

### crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs::cube_4color_phase5_interlocking_alternates
- Reason: Multi-region MMU cube; per-region config delivery changes even-layer interlocking erosion toolpath.
- Baseline: assertion: motion(default) != motion(depth=0.5) AND motion(depth=0.5) != motion(width=0.5)

### crates/slicer-runtime/tests/executor/cube_4color_phase5_tdd.rs::cube_4color_phase5_interlocking_beam_skips_phase5
- Reason: Multi-region MMU cube; per-region config delivery must keep beam-skip byte-identical to default.
- Baseline: assertion: motion(default) == motion(beam config with width=2.0,depth=0.5)
