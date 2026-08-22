# Pinch 'n Print — Config Keys Reference

**What this covers:** the catalog of config keys the host binary and core modules
recognise — their types, defaults, and which module or host struct registers
each.

**Who it's for:** anyone setting a config value, adding a config key, or tracking
where a default lives.

**Prerequisites:** `03_wit_and_manifest.md` for the manifest schema rules that
govern how a module declares a key.

This file is the canonical catalog of config keys recognised by the host
binary, core modules, and the resolved-config pipeline. For the manifest
**schema rules** (table format, valid types, validation expressions) see
`03_wit_and_manifest.md`. For namespaced override conventions
(`object_config:`, `paint_config:`) see also `02_ir_schemas.md`
§"Config Key Namespaces".

> **Generated, not hand-maintained.** The three tables below marked _(generated)_
> are produced by `cargo xtask gen-config-docs` from the authoritative sources —
> module `[config.schema]` manifests and `docs/config/host-keys.toml` (itself
> locked to the live code defaults — `FeedrateConfig::default()`,
> `ResolvedConfig::default()`, and named `DEFAULT_*` constants — by the
> `host_keys_doc_lock_tdd` slicer-runtime test (`crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`)). Do not edit them by hand;
> CI fails (`gen-config-docs --check`) if they drift. The hand-written sections
> further down add per-domain prose (units, macros, semantics).

Each entry below lists:

- **Key** — runtime string used in `ConfigView::get_*` and CLI flat-key form.
- **Type** — `bool` / `int` / `float` / `string` / `enum`. See
  `docs/03_wit_and_manifest.md` "Config Field Types Reference".
- **Default** — value when the user does not set the key.
- **Range / values** — clamp bounds or accepted enum strings.
- **Packet** — packet that introduced the key.
- **Module(s)** — primary consumer(s). `[host]` means consumed by a host
  built-in (e.g. `DefaultGCodeEmitter`).
- **Source-of-truth** — module-owned keys live in the module's manifest TOML
  (`modules/core-modules/<name>/<name>.toml`); host-registered keys live in the
  consumer struct under `crates/slicer-runtime/src/` (e.g.
  `feedrate.rs::FeedrateConfig`) and are mirrored into
  `docs/config/host-keys.toml`. There is no single `config_schema.rs` file.

---

## Module-owned config keys (generated)

Every `[config.schema]` key across `modules/core-modules/*/<name>.toml`. The
per-domain sections lower in this file add prose for these same keys; this table
is the authoritative catalog of their defaults and ranges.

<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->
| Key | Type | Default | Range | Module |
|---|---|---|---|---|
| `alternate_extra_wall` | bool | `false` | — | `arachne-perimeters` |
| `bridge_flow` | float | `1.0` | >= 0.0 | `arachne-perimeters` |
| `bridge_line_width` | float | `0.0` | [0.0, 2.0] | `arachne-perimeters` |
| `detect_overhang_wall` | bool | `true` | — | `arachne-perimeters` |
| `detect_thin_wall` | bool | `false` | — | `arachne-perimeters` |
| `extra_perimeters_on_overhangs` | bool | `false` | — | `arachne-perimeters` |
| `initial_layer_line_width` | float | `0.0` | [0.0, 2.0] | `arachne-perimeters` |
| `initial_layer_min_bead_width` | float | `3400` | >= 0.0 | `arachne-perimeters` |
| `inner_wall_line_width` | float_or_percent | `0` | [0.0, 2.0] | `arachne-perimeters` |
| `layer_height` | float | `0.2` | [0.01, 1.0] | `arachne-perimeters` |
| `line_width` | float | `0` | [0.0, 2.0] | `arachne-perimeters` |
| `max_bead_count` | int | `0` | >= 0.0 | `arachne-perimeters` |
| `min_bead_width` | float | `4000` | >= 0.0 | `arachne-perimeters` |
| `min_central_distance` | float | `0` | >= 0.0 | `arachne-perimeters` |
| `min_feature_size` | percent | `"25%"` | >= 0.0 | `arachne-perimeters` |
| `min_length_factor` | float | `0.5` | [0.0, 2.0] | `arachne-perimeters` |
| `min_width` | float | `4000` | >= 0.0 | `arachne-perimeters` |
| `min_width_top_surface` | float_or_percent | `"0.0"` | >= 0.0 | `arachne-perimeters` |
| `nozzle_diameter` | float | `0.4` | >= 0.01 | `arachne-perimeters` |
| `only_one_wall_first_layer` | bool | `false` | — | `arachne-perimeters` |
| `only_one_wall_top` | bool | `false` | — | `arachne-perimeters` |
| `outer_wall_line_width` | float_or_percent | `0` | [0.0, 2.0] | `arachne-perimeters` |
| `outer_wall_offset` | float | `0` | >= 0.0 | `arachne-perimeters` |
| `overhang_reverse` | bool | `false` | — | `arachne-perimeters` |
| `overhang_reverse_internal_only` | bool | `false` | — | `arachne-perimeters` |
| `overhang_reverse_threshold` | float_or_percent | `"0.0"` | [0.0, 10.0] | `arachne-perimeters` |
| `precise_outer_wall` | bool | `false` | — | `arachne-perimeters` |
| `seam_candidate_angle_threshold_deg` | float | `30.0` | [0.0, 180.0] | `arachne-perimeters` |
| `sparse_infill_density` | float | `20.0` | [0.0, 100.0] | `arachne-perimeters` |
| `spiral_vase` | bool | `false` | — | `arachne-perimeters` |
| `support_raft_layers` | int | `0` | [0.0, 20.0] | `arachne-perimeters` |
| `thick_bridges` | bool | `false` | — | `arachne-perimeters` |
| `wall_count` | int | `3` | >= 1.0 | `arachne-perimeters` |
| `wall_direction` | string | `"counter_clockwise"` | — | `arachne-perimeters` |
| `wall_distribution_count` | int | `1` | >= 1.0 | `arachne-perimeters` |
| `wall_maximum_deviation` | float | `0.005` | [0.0001, 1.0] | `arachne-perimeters` |
| `wall_maximum_resolution` | float | `0.05` | [0.001, 10.0] | `arachne-perimeters` |
| `wall_sequence` | string | `"InnerOuter"` | — | `arachne-perimeters` |
| `wall_transition_angle` | float | `10.0` | [0.0, 180.0] | `arachne-perimeters` |
| `wall_transition_filter_deviation` | float | `1000` | >= 0.0 | `arachne-perimeters` |
| `wall_transition_length` | percent | `"100%"` | >= 0.0 | `arachne-perimeters` |
| `alternate_extra_wall` | bool | `false` | — | `classic-perimeters` |
| `bridge_flow` | float | `1.0` | >= 0.0 | `classic-perimeters` |
| `bridge_line_width` | float_or_percent | `0.0` | [0.0, 2.0] | `classic-perimeters` |
| `detect_overhang_wall` | bool | `true` | — | `classic-perimeters` |
| `detect_thin_wall` | bool | `true` | — | `classic-perimeters` |
| `extra_perimeters` | int | `0` | [0.0, 10.0] | `classic-perimeters` |
| `extra_perimeters_on_overhangs` | bool | `false` | — | `classic-perimeters` |
| `filter_out_gap_fill` | float | `0.5` | [0.0, 5.0] | `classic-perimeters` |
| `gap_fill_medial_axis_on_painted` | bool | `false` | — | `classic-perimeters` |
| `gap_infill_speed` | float | `30.0` | [1.0, 300.0] | `classic-perimeters` |
| `infill_wall_overlap` | percent | `"15%"` | — | `classic-perimeters` |
| `initial_layer_line_width` | float_or_percent | `0.0` | [0.0, 2.0] | `classic-perimeters` |
| `inner_wall_line_width` | float_or_percent | `0` | [0.0, 2.0] | `classic-perimeters` |
| `inner_wall_speed` | float | `45.0` | [1.0, 300.0] | `classic-perimeters` |
| `layer_height` | float | `0.2` | [0.01, 2.0] | `classic-perimeters` |
| `line_width` | float | `0` | [0.0, 2.0] | `classic-perimeters` |
| `min_width_top_surface` | float_or_percent | `"0.0"` | >= 0.0 | `classic-perimeters` |
| `narrow_loop_length_threshold_mm` | float | `10.0` | [0.0, 1000.0] | `classic-perimeters` |
| `nozzle_diameter` | float | `0.4` | [0.1, 2.0] | `classic-perimeters` |
| `only_one_wall_first_layer` | bool | `false` | — | `classic-perimeters` |
| `only_one_wall_top` | bool | `false` | — | `classic-perimeters` |
| `outer_wall_line_width` | float_or_percent | `0` | [0.0, 2.0] | `classic-perimeters` |
| `outer_wall_speed` | float | `30.0` | [1.0, 300.0] | `classic-perimeters` |
| `overhang_reverse` | bool | `false` | — | `classic-perimeters` |
| `overhang_reverse_internal_only` | bool | `false` | — | `classic-perimeters` |
| `perimeter_arc_tolerance` | float | `0.0125` | [0.0, 1.0] | `classic-perimeters` |
| `precise_outer_wall` | bool | `false` | — | `classic-perimeters` |
| `seam_candidate_angle_threshold_deg` | float | `30.0` | [0.0, 180.0] | `classic-perimeters` |
| `slice_has_paint` | bool | `false` | — | `classic-perimeters` |
| `smaller_perimeter_line_width` | float | `0.25` | [0.05, 2.0] | `classic-perimeters` |
| `smaller_perimeter_threshold_mm` | float | `0.8` | [0.0, 10.0] | `classic-perimeters` |
| `sparse_infill_density` | float | `20.0` | [0.0, 100.0] | `classic-perimeters` |
| `spiral_vase` | bool | `false` | — | `classic-perimeters` |
| `support_raft_layers` | int | `0` | [0.0, 20.0] | `classic-perimeters` |
| `thick_bridges` | bool | `false` | — | `classic-perimeters` |
| `top_bottom_infill_wall_overlap` | percent | `"25%"` | — | `classic-perimeters` |
| `wall_count` | int | `3` | [1.0, 10.0] | `classic-perimeters` |
| `wall_sequence` | string | `"InnerOuter"` | — | `classic-perimeters` |
| `apply_to_all` | bool | `false` | — | `fuzzy-skin` |
| `point_distance` | float | `0.5` | [0.01, 5.0] | `fuzzy-skin` |
| `thickness` | float | `0.3` | [0.0, 2.0] | `fuzzy-skin` |
| `bridge_line_width` | float | `0.0` | [0.0, 2.0] | `gyroid-infill` |
| `infill_angle` | float | `45.0` | [0.0, 360.0] | `gyroid-infill` |
| `infill_density` | float | `0.2` | [0.0, 1.0] | `gyroid-infill` |
| `infill_speed` | float | `60.0` | [1.0, 300.0] | `gyroid-infill` |
| `initial_layer_line_width` | float | `0.0` | [0.0, 2.0] | `gyroid-infill` |
| `internal_solid_infill_line_width` | float | `0.0` | [0.0, 2.0] | `gyroid-infill` |
| `line_width` | float | `0` | [0.0, 2.0] | `gyroid-infill` |
| `sparse_infill_line_width` | float | `0.0` | [0.0, 2.0] | `gyroid-infill` |
| `top_surface_line_width` | float | `0.0` | [0.0, 2.0] | `gyroid-infill` |
| `infill_anchor` | float_or_percent | `"400%"` | [0.0, 1000.0] | `infill-linker` |
| `infill_anchor_max` | float_or_percent | `20.0` | [0.0, 1000.0] | `infill-linker` |
| `infill_overlap` | float | `0.45` | [0.0, 1.0] | `infill-linker` |
| `layer_height` | float | `0.2` | [0.01, 2.0] | `infill-linker` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `infill-linker` |
| `first_layer_height` | float | `0.3` | [0.01, 1.0] | `layer-planner-default` |
| `layer_height` | float | `0.2` | [0.01, 1.0] | `layer-planner-default` |
| `bridge_line_width` | float | `0.0` | [0.0, 2.0] | `lightning-infill` |
| `infill_density` | float | `0.2` | [0.0, 1.0] | `lightning-infill` |
| `infill_speed` | float | `60.0` | [1.0, 300.0] | `lightning-infill` |
| `initial_layer_line_width` | float | `0.0` | [0.0, 2.0] | `lightning-infill` |
| `line_width` | float | `0` | [0.0, 2.0] | `lightning-infill` |
| `sparse_infill_line_width` | float | `0.0` | [0.0, 2.0] | `lightning-infill` |
| `bed_temperature_initial_layer_single` | int | `60` | [0.0, 120.0] | `machine-gcode-emit` |
| `before_layer_change_gcode` | string | `""` | — | `machine-gcode-emit` |
| `change_extrusion_role_gcode` | string | `""` | — | `machine-gcode-emit` |
| `change_filament_gcode` | string | `""` | — | `machine-gcode-emit` |
| `filament_change_extrusion_role_gcode` | string | `""` | — | `machine-gcode-emit` |
| `filament_end_gcode` | string | `""` | — | `machine-gcode-emit` |
| `filament_start_gcode` | string | `""` | — | `machine-gcode-emit` |
| `layer_change_gcode` | string | `""` | — | `machine-gcode-emit` |
| `machine_end_gcode` | string | `"PRINT_END"` | — | `machine-gcode-emit` |
| `machine_start_gcode` | string | `"M190 S[bed_temperature_initial_layer_single]\nM…"` | — | `machine-gcode-emit` |
| `nozzle_diameter` | float | `0.4` | [0.1, 2.0] | `machine-gcode-emit` |
| `nozzle_temperature_initial_layer` | int | `215` | [0.0, 300.0] | `machine-gcode-emit` |
| `process_change_extrusion_role_gcode` | string | `""` | — | `machine-gcode-emit` |
| `time_lapse_gcode` | string | `""` | — | `machine-gcode-emit` |
| `bridge_speed` | float | `25.0` | — | `overhang-classifier-default` |
| `enable_overhang_speed` | bool | `true` | — | `overhang-classifier-default` |
| `inner_wall_speed` | float | `60.0` | — | `overhang-classifier-default` |
| `outer_wall_speed` | float | `60.0` | — | `overhang-classifier-default` |
| `overhang_1_4_speed` | float | `0.0` | — | `overhang-classifier-default` |
| `overhang_2_4_speed` | float | `0.0` | — | `overhang-classifier-default` |
| `overhang_3_4_speed` | float | `0.0` | — | `overhang-classifier-default` |
| `overhang_4_4_speed` | float | `0.0` | — | `overhang-classifier-default` |
| `slowdown_for_curled_perimeters` | bool | `false` | — | `overhang-classifier-default` |
| `thin_wall_speed` | float | `30.0` | — | `overhang-classifier-default` |
| `disable_fan_first_layers` | int | `1` | >= 0.0 | `part-cooling` |
| `enable_overhang_fan` | bool | `true` | — | `part-cooling` |
| `fan_speed_max` | int | `255` | [0.0, 255.0] | `part-cooling` |
| `fan_speed_min` | int | `51` | [0.0, 255.0] | `part-cooling` |
| `overhang_fan_speed` | int | `100` | [0.0, 100.0] | `part-cooling` |
| `slow_down_for_layer_cooling` | bool | `true` | — | `part-cooling` |
| `slow_down_layer_time` | float | `5.0` | >= 0.0 | `part-cooling` |
| `slow_down_min_speed` | float | `10.0` | >= 0.0 | `part-cooling` |
| `path_optimization_emit_layer_markers` | bool | `true` | — | `path-optimization-default` |
| `retract_length` | float | `0.8` | — | `path-optimization-default` |
| `retract_mode` | enum | `"gcode"` | — | `path-optimization-default` |
| `retract_speed` | float | `25.0` | — | `path-optimization-default` |
| `travel_z_hop` | float | `0.0` | — | `path-optimization-default` |
| `bridge_line_width` | float | `0.0` | [0.0, 2.0] | `rectilinear-infill` |
| `infill_angle` | float | `45.0` | [0.0, 360.0] | `rectilinear-infill` |
| `infill_density` | float | `0.2` | [0.0, 1.0] | `rectilinear-infill` |
| `infill_speed` | float | `60.0` | [1.0, 300.0] | `rectilinear-infill` |
| `initial_layer_line_width` | float | `0.0` | [0.0, 2.0] | `rectilinear-infill` |
| `internal_solid_infill_line_width` | float | `0.0` | [0.0, 2.0] | `rectilinear-infill` |
| `line_width` | float | `0` | [0.0, 2.0] | `rectilinear-infill` |
| `sparse_infill_line_width` | float | `0.0` | [0.0, 2.0] | `rectilinear-infill` |
| `top_surface_line_width` | float | `0.0` | [0.0, 2.0] | `rectilinear-infill` |
| `seam_mode` | enum | `"aligned"` | — | `seam-placer` |
| `seam_mode` | enum | `"aligned"` | — | `seam-planner-default` |
| `brim_width` | float | `8.0` | [0.0, 30.0] | `skirt-brim` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `skirt-brim` |
| `skirt_brim_enabled` | bool | `true` | — | `skirt-brim` |
| `skirt_distance` | float | `3.0` | [0.0, 20.0] | `skirt-brim` |
| `skirt_height` | int | `1` | [1.0, 10.0] | `skirt-brim` |
| `skirt_loops` | int | `6` | [0.0, 20.0] | `skirt-brim` |
| `ironing_enabled` | bool | `false` | — | `support-surface-ironing` |
| `ironing_flow_rate` | float | `100.0` | [1.0, 200.0] | `support-surface-ironing` |
| `ironing_spacing` | float | `0.1` | [0.01, 1.0] | `support-surface-ironing` |
| `ironing_speed` | float | `30.0` | [1.0, 300.0] | `support-surface-ironing` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `support-surface-ironing` |
| `ironing_enabled` | bool | `false` | — | `top-surface-ironing` |
| `ironing_flow` | float | `0.1` | [0.01, 1.0] | `top-surface-ironing` |
| `ironing_pattern` | enum | `"rectilinear"` | — | `top-surface-ironing` |
| `ironing_spacing_mm` | float | `0.1` | [0.01, 1.0] | `top-surface-ironing` |
| `ironing_speed` | float | `20.0` | [1.0, 300.0] | `top-surface-ironing` |
| `enable_support` | bool | `true` | — | `traditional-support` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `traditional-support` |
| `support_angle` | float | `60.0` | [0.0, 90.0] | `traditional-support` |
| `support_density` | float | `20.0` | [0.0, 100.0] | `traditional-support` |
| `support_filament` | int | `0` | >= 0.0 | `traditional-support` |
| `support_interface_filament` | int | `0` | >= 0.0 | `traditional-support` |
| `support_interface_spacing_mm` | float | `0.4` | [0.1, 2.0] | `traditional-support` |
| `support_speed` | float | `50.0` | [1.0, 300.0] | `traditional-support` |
| `enable_support` | bool | `true` | — | `traditional-support-planner` |
| `support_base_pattern` | string | `"rectilinear"` | — | `traditional-support-planner` |
| `support_interface_bottom_layers` | int | `-1` | [-1.0, 10.0] | `traditional-support-planner` |
| `support_interface_top_layers` | int | `2` | [0.0, 10.0] | `traditional-support-planner` |
| `support_layer_height_mm` | float | `0.0` | [0.05, 1.0] | `traditional-support-planner` |
| `support_overhang_angle` | float | `45.0` | [0.0, 90.0] | `traditional-support-planner` |
| `support_top_z_distance_mm` | float | `0.0` | [0.0, 5.0] | `traditional-support-planner` |
| `enable_support` | bool | `true` | — | `tree-support` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `tree-support` |
| `support_angle` | float | `60.0` | [0.0, 90.0] | `tree-support` |
| `support_density` | float | `20.0` | [0.0, 100.0] | `tree-support` |
| `support_layer_height_mm` | float | `0.0` | [0.05, 1.0] | `tree-support` |
| `support_speed` | float | `50.0` | [1.0, 300.0] | `tree-support` |
| `support_top_z_distance_mm` | float | `0.0` | [0.0, 5.0] | `tree-support` |
| `base_raft_layers` | int | `1` | [0.0, 20.0] | `tree-support-planner` |
| `enable_support` | bool | `true` | — | `tree-support-planner` |
| `interface_raft_layers` | int | `0` | [0.0, 20.0] | `tree-support-planner` |
| `raft_first_layer_density` | float | `0.4` | [0.0, 1.0] | `tree-support-planner` |
| `support_interface_bottom_layers` | int | `-1` | [-1.0, 10.0] | `tree-support-planner` |
| `support_interface_top_layers` | int | `2` | [0.0, 10.0] | `tree-support-planner` |
| `support_layer_height_mm` | float | `0.0` | [0.05, 1.0] | `tree-support-planner` |
| `support_on_build_plate_only` | bool | `false` | — | `tree-support-planner` |
| `support_raft_layers` | int | `0` | [0.0, 20.0] | `tree-support-planner` |
| `support_top_z_distance_mm` | float | `0.0` | [0.0, 5.0] | `tree-support-planner` |
| `tree_support_branch_angle` | float | `45.0` | [0.0, 75.0] | `tree-support-planner` |
| `tree_support_branch_diameter` | float | `5.0` | [0.5, 20.0] | `tree-support-planner` |
| `tree_support_branch_diameter_angle` | float | `5.0` | [0.0, 90.0] | `tree-support-planner` |
| `tree_support_branch_distance` | float | `1.0` | [0.1, 10.0] | `tree-support-planner` |
| `tree_support_interface_spacing_mm` | float | `0.4` | [0.1, 2.0] | `tree-support-planner` |
| `tree_support_wall_count` | int | `1` | [1.0, 10.0] | `tree-support-planner` |
| `bed_shape` | float-list | `—` | — | `wipe-tower` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `wipe-tower` |
| `retract_length` | float | `2.0` | [0.0, 20.0] | `wipe-tower` |
| `wipe_tower_enabled` | bool | `true` | — | `wipe-tower` |
| `wipe_tower_purge_volume` | float | `10.0` | [1.0, 50.0] | `wipe-tower` |
| `wipe_tower_width` | float | `60.0` | [1.0, 100.0] | `wipe-tower` |
| `wipe_tower_x` | float | `10.0` | [0.0, 300.0] | `wipe-tower` |
| `wipe_tower_y` | float | `10.0` | [0.0, 300.0] | `wipe-tower` |
<!-- END GENERATED: module-config-keys -->

**Note — `support_interface_bottom_layers`:** the key remains user-visible with
default `-1`, but bottom interface layers are not yet implemented; `support-planner`
emits one typed code-`1003` warning (via `SupportGeometryOutput::push_diagnostic`,
packet 118) before the layer loop when the value is not `-1`. See
`docs/specs/_OLD/support-modules-orca-port.md` (archived spec).

### `seam_mode` values

Accepted on both `seam-placer` and `seam-planner-default` (default `"aligned"` —
packet 180 changed the default from `"nearest"` in both manifests together, matching
OrcaSlicer's canonical `spAligned` default):

- `nearest` — per-layer nearest-candidate seam placement.
- `rear` — select the candidate with the highest Y coordinate (rear of the print bed).
- `random` — pseudo-random candidate selection based on layer index.
- `aligned` — cross-layer chained + spline-smoothed seam computed in the SeamPlanning prepass (`seam-planner-default`) and snapped to real wall candidates by `seam-placer` (see ADR-0046). Default mode since packet 180.
- `aligned_back` — same as `aligned` with a rear (max-Y) bias in candidate scoring.

## Host-registered config keys (generated)

Keys consumed by host built-ins, mirrored from their code source of truth
(`feedrate.rs::FeedrateConfig` for per-role speeds in mm/s;
`resolved_config.rs::ResolvedConfig` for shell-window / slicing-precision /
fill-role keys; named `DEFAULT_*` constants in `run.rs` / `pipeline.rs` for keys
read directly from the config source) via `docs/config/host-keys.toml`, which the
`host_keys_doc_lock_tdd` slicer-runtime test (`crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs`) holds equal to those defaults
(the speed check is exhaustive — adding a `FeedrateConfig` field fails the build
until it is documented). In CI this staleness check runs through the
`check-deviations --check` step (`.github/workflows/ci.yml`), which invokes the
same `gen-config-docs --check` code path. Per-role speeds feed
`DefaultGCodeEmitter::resolve_feedrate(role, paint_layer, …)`, which emits F-tokens
in mm/min (see `docs/08_coordinate_system.md` "F-Token Formatting Convention").

<!-- BEGIN GENERATED: host-speeds (cargo xtask gen-config-docs) -->
| Key | Type | Default | Range | Source |
|---|---|---|---|---|
| `bottom_surface_speed` | float | `100.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `bridge_speed` | float | `25.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `filament_ironing_speed` | float | `0.0` | >= 0 (0 = use ironing_speed) | `feedrate.rs::FeedrateConfig` |
| `gap_infill_speed` | float | `30.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `initial_layer_infill_speed` | float | `60.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `initial_layer_speed` | float | `30.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `initial_layer_travel_speed` | float | `120.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `inner_wall_speed` | float | `60.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `internal_bridge_speed` | float | `37.5` | > 0 | `feedrate.rs::FeedrateConfig` |
| `ironing_speed` | float | `20.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `outer_wall_speed` | float | `60.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `overhang_1_4_speed` | float | `0.0` | >= 0 (0 = no override (packet 57)) | `feedrate.rs::FeedrateConfig` |
| `overhang_2_4_speed` | float | `0.0` | >= 0 (0 = no override (packet 57)) | `feedrate.rs::FeedrateConfig` |
| `overhang_3_4_speed` | float | `0.0` | >= 0 (0 = no override (packet 57)) | `feedrate.rs::FeedrateConfig` |
| `overhang_4_4_speed` | float | `0.0` | >= 0 (0 = no override (packet 57)) | `feedrate.rs::FeedrateConfig` |
| `prime_tower_speed` | float | `90.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `skirt_speed` | float | `50.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `sparse_infill_speed` | float | `100.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `support_interface_speed` | float | `80.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `support_speed` | float | `80.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `thin_wall_speed` | float | `30.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `top_surface_speed` | float | `100.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `travel_speed` | float | `120.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `travel_speed_z` | float | `0.0` | >= 0 (0 = use travel_speed for Z) | `feedrate.rs::FeedrateConfig` |
| `wipe_speed` | float | `96.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `wipe_tower_speed` | float | `90.0` | > 0 | `feedrate.rs::FeedrateConfig` |
| `bottom_fill_holder` | string | `"rectilinear-infill"` | — (holder of claim:bottom-fill (packet 37)) | `resolved_config.rs::ResolvedConfig` |
| `bottom_shell_layers` | int | `3` | [1, 10] | `resolved_config.rs::ResolvedConfig` |
| `bridge_fill_holder` | string | `"rectilinear-infill"` | — (holder of claim:bridge-fill (packet 37)) | `resolved_config.rs::ResolvedConfig` |
| `flat_bridge_closing_join` | string | `"miter"` | — (flat-bridge enclosure closing join: miter (OrcaSlicer parity, default) | square | round (legacy, bit-identical, slow)) | `resolved_config.rs::ResolvedConfig` |
| `gcode_resolution` | float | `0.0125` | >= 0 (D-P tolerance for walls / brim) | `resolved_config.rs::ResolvedConfig` |
| `gcode_xy_decimals` | int | `3` | [1, 6] (X / Y / Z token formatting) | `resolved_config.rs::ResolvedConfig` |
| `infill_resolution` | float | `0.04` | >= 0 (D-P tolerance for infill / bridge / top / bottom) | `resolved_config.rs::ResolvedConfig` |
| `min_segment_length` | float | `0.05` | >= 0 (short-segment dropper) | `resolved_config.rs::ResolvedConfig` |
| `slice_closing_radius` | float | `0.049` | >= 0 (per-layer Clipper2 close) | `resolved_config.rs::ResolvedConfig` |
| `sparse_fill_holder` | string | `"rectilinear-infill"` | — (holder of claim:sparse-fill (packet 37)) | `resolved_config.rs::ResolvedConfig` |
| `support_resolution` | float | `0.0375` | >= 0 (D-P tolerance for support / interface) | `resolved_config.rs::ResolvedConfig` |
| `top_fill_holder` | string | `"rectilinear-infill"` | — (holder of claim:top-fill (packet 37)) | `resolved_config.rs::ResolvedConfig` |
| `top_shell_layers` | int | `3` | [1, 10] (deviates from OrcaSlicer's 4) | `resolved_config.rs::ResolvedConfig` |
| `thumbnail_path` | string | `""` | — (absent/empty = no THUMBNAIL_BLOCK; CLI --thumbnail overrides (packet 55)) | `pipeline.rs::DEFAULT_THUMBNAIL_PATH` |
| `use_relative_e_distances` | bool | `true` | — (false selects M82; serializer issues G92 E0 on mode change (packet 54)) | `run.rs::DEFAULT_USE_RELATIVE_E_DISTANCES` |
| `wall_generator` | string | `"classic"` | — (values classic or arachne; selects the perimeter-generator claim holder (com.core.classic-perimeters vs com.core.arachne-perimeters) at module-load time, before ResolvedConfig exists (packet 112 Step 10)) | `slicer-scheduler::execution_plan::DEFAULT_WALL_GENERATOR` |
<!-- END GENERATED: host-speeds -->

`filament_ironing_speed > 0.0` overrides `ironing_speed` for the `Ironing` role.
The four `overhang_*_4_speed` keys all-zero short-circuits the overhang
classifier for byte-identical pre-packet-57 output.

**Overhang speed key consumption (Packet 88):** the four `overhang_*_4_speed`
keys are still REGISTERED on `feedrate.rs::FeedrateConfig` (table above)
so host-side fallback resolution stays trivial, but the active CONSUMER
is the `overhang-classifier-default` FinalizationModule
(`modules/core-modules/overhang-classifier-default/`) — see ADR-0008.
The module reads each point's prepass-stamped `overhang_distance_mm`, gated
on `overhang_quartile.is_some()`, then interpolates the six `speed_sections`
from `OVERHANG_OVERLAP_LEVELS {90, 75, 50, 25, 13, 0}`, rounds and clamps
the result to the role reference speed, and emits one
`EntityMutation::SetPointSpeedFactors` per entity. The former whole-entity
speed-factor mutation is gone. The host's `overhang_classifier::classify_layers`
prepass stamps the point annotations; it does NOT read the speed keys. Treat
the source column above as "registration site"; treat ADR-0008 as the
authoritative pointer to the consumer.

<!-- VERIFY: this section previously documented a `union_paint_regions_at_harvest`
     bool key (default true) on a `paint-segmentation` scope, plus a
     `docs/02_ir_schemas.md` §"Harvest Strategy" cross-reference. Neither exists:
     the key has zero hits under `crates/`, there is no `paint-segmentation`
     module (paint segmentation is host-native in
     `crates/slicer-core/src/algos/paint_segmentation/`), and no "Harvest
     Strategy" section exists in `02_ir_schemas.md`. The `SemanticRegion` type it
     referenced was deleted with `PaintRegionIR` in packet 95. Removed as
     fabricated/retired. -->

## Deviations from OrcaSlicer (generated)

Generated keys whose numeric default differs from the matching key in
`docs/ORCA_CONFIG_REFERENCE.md` (the upstream snapshot). Everything else matches
upstream or has no upstream equivalent.

<!-- BEGIN GENERATED: orca-deviations (cargo xtask gen-config-docs) -->
| Key | Owner | Pinch 'n Print default | OrcaSlicer default |
|---|---|---|---|
| `brim_width` | `skirt-brim` | `8.0` | `0.0` |
| `filter_out_gap_fill` | `classic-perimeters` | `0.5` | `0.0` |
| `inner_wall_speed` | `classic-perimeters` | `45.0` | `60.0` |
| `ironing_speed` | `support-surface-ironing` | `30.0` | `20.0` |
| `nozzle_temperature_initial_layer` | `machine-gcode-emit` | `215` | `200.0` |
| `outer_wall_speed` | `classic-perimeters` | `30.0` | `60.0` |
| `skirt_distance` | `skirt-brim` | `3.0` | `2.0` |
| `skirt_loops` | `skirt-brim` | `6` | `1.0` |
| `support_angle` | `traditional-support` | `60.0` | `0.0` |
| `support_angle` | `tree-support` | `60.0` | `0.0` |
| `support_interface_top_layers` | `traditional-support-planner` | `2` | `3.0` |
| `support_interface_top_layers` | `tree-support-planner` | `2` | `3.0` |
| `support_speed` | `traditional-support` | `50.0` | `80.0` |
| `support_speed` | `tree-support` | `50.0` | `80.0` |
| `top_shell_layers` | `resolved_config.rs::ResolvedConfig` | `3` | `4.0` |
| `tree_support_branch_angle` | `tree-support-planner` | `45.0` | `40.0` |
| `tree_support_branch_distance` | `tree-support-planner` | `1.0` | `5.0` |
| `wipe_tower_x` | `wipe-tower` | `10.0` | `15.0` |
| `wipe_tower_y` | `wipe-tower` | `10.0` | `220.0` |
<!-- END GENERATED: orca-deviations -->

---

## Print speeds (packet 52, 57)

The per-role speed keys and their defaults are in the generated
**Host-registered config keys** table above (authoritative, mirrored from
`FeedrateConfig::default()`). This section previously hand-listed them and had
drifted 15 of 26 defaults away from the code.

---

## Cooling and fan (packet 53)

Keys consumed by the `part-cooling` finalization-stage module
(`modules/core-modules/part-cooling/`). Defaults and ranges are in the generated
**Module-owned config keys** table above (module `part-cooling`). Behaviour:
`enable_overhang_fan` modulates the fan on overhang quartiles 3–4;
`slow_down_for_layer_cooling` reduces speed toward `slow_down_min_speed` when a
layer's print time falls below `slow_down_layer_time`.

---

## Support (packet 31b + packet 28/30)

Keys split across the `tree-support-planner` and `tree-support` core modules.
Defaults and ranges are in the generated **Module-owned config keys** table
above (modules `tree-support-planner`, `tree-support`). Note `support_layer_height_mm
= 0.0` means "use the model layer height".

**Tree support family configuration (packet 221).** `support_family` is the
canonical per-region family selector: values starting with `tree*` or `hybrid*`
select the tree family, `normal*`/`classic*` the traditional family (absent
defaults to traditional). It drives which `tree-support-planner`/`tree-support`
pair serves each region (see `docs/04_host_scheduler.md` § "Planner-Renderer
Pairing").

**Traditional support base pattern (packet 222).** `support_base_pattern`
selects the traditional family's base fill pattern (e.g. `rectilinear`); it is
owned by `traditional-support-planner` and recorded in each plan entry's
`capabilities` as `traditional-base-pattern`.

---

## Extrusion mode (packet 54)

`use_relative_e_distances` (default `true` = M83) is in the generated
**Host-registered config keys** table above. `false` selects M82; the serializer
issues `G92 E0` on mode transition and layer reset.

---

## Retraction mode (packet 34)

`retract_mode` (enum, default `"gcode"`, values `"gcode"` / `"firmware"`;
`path-optimization-default`) is in the generated **Module-owned config keys**
table above.

- `"gcode"` → inline `G1 E-<length> F<speed>` retract / `G1 E<length> F<speed>` unretract.
- `"firmware"` → `G10` / `G11`. Length and speed remain in IR for diagnostics
  but are not serialized.

---

## G-code preamble (packet 55)

The user config keys here are `thumbnail_path` (default `""`) and `thumbnails`
(packet 173; raw-config string, never a module-manifest key). Both are in the
generated **Host-registered config keys** table above / the CONFIG_BLOCK:

- An absent/empty `thumbnail_path` emits no thumbnail block; the `--thumbnail
  <PATH>` CLI flag overrides it (CLI wins). `thumbnail_path` is invocation-time
  only and is stripped before the CONFIG_BLOCK is written.
- `thumbnails` (packet 173) is a case-insensitive comma-separated `WxH/EXT`
  list selecting the entries the block contains. Accepted extensions: `PNG`,
  `JPG`/`JPEG` (aliases of one format), `QOI`, `BTT_TFT`, and `COLPIC`; a
  malformed entry (e.g. `48x/PNG`, `48x48`, or `48x48/BMP`) is rejected with an
  error naming the offending entry verbatim, and the pipeline surfaces that
  error instead of emitting a thumbnail block. When the key is absent, exactly
  one PNG entry at the source PNG's own dimensions is emitted (source bytes
  passed through). `thumbnails` **remains in the CONFIG_BLOCK** (unlike
  `thumbnail_path`, which is stripped), and entry order in the block follows
  the order of specs in the string, default entry first when the key is absent.
  The `--thumbnail <PATH>` CLI flag supplies the single source PNG; all
  requested sizes/formats travel in this key and are rendered PNP-side
  (fork-facing deviation `D-173-THUMBNAIL-SINGLE-PNG`).

Each entry is framed per the packet-173 wire format — `; <tag> begin <W>x<H>
<len>` / `; <tag> end` with `; `-prefixed base64 lines wrapped at 78 columns —
replacing the packet-55 bare-base64 `THUMBNAIL_BLOCK_*` chunks (76 chars/line);
the outer `; THUMBNAIL_BLOCK_START` / `; THUMBNAIL_BLOCK_END` sentinels are
retained. See `docs/02_ir_schemas.md` for the full per-entry envelope format.

The G-code header also emits `; filament_diameter`, `; filament_density`, and
`; max_z_height` comment lines, but **these are not user config keys** — there is
no `config_source` key for them:

- `filament_diameter` / `filament_density` are emitter constants
  (`1.75 mm` / `1.24 g·cm⁻³`) on `DefaultGCodeEmitter`
  (`FeedrateConfig` in `crates/slicer-ir/src/feedrate.rs`), overridable only programmatically
  via `with_filament_config(...)`. Wiring them to config keys is a future
  enhancement, not a current capability.
- `max_z_height` in the header is the **computed** top-layer Z (with fallback
  floor `max_z_height_floor_mm = 256.0`), not a settable key.

See `docs/02_ir_schemas.md` "G-code envelope blocks" for the full envelope format.

---

## Custom G-code injection points
<!-- anchor: custom-gcode-injection-points -->

The `machine-gcode-emit` module registers eleven custom-G-code injection points:
`machine_start_gcode`, `before_layer_change_gcode`, `time_lapse_gcode`,
`layer_change_gcode`, `machine_end_gcode`, `filament_end_gcode`,
`change_filament_gcode`, `filament_start_gcode`,
`change_extrusion_role_gcode`, `filament_change_extrusion_role_gcode`, and
`process_change_extrusion_role_gcode`. Their defaults and types are in the
generated **Module-owned config keys** table above. Each value is a single-pass
template using square-bracket placeholders such as `[key]`.

At a layer boundary, the canonical emission order is the reserved tags
`;LAYER_CHANGE`, `;Z:`, and `;HEIGHT:`, followed by
`before_layer_change_gcode`, `time_lapse_gcode`, and
`layer_change_gcode`, in that order. `machine_start_gcode` is emitted before
the first layer, and `machine_end_gcode` is emitted after the last layer.

For a toolchange, the injection order is
`filament_end_gcode` -> `change_filament_gcode` -> `T<n>` ->
`filament_start_gcode`. Thus the tool-select command sits between
`change_filament_gcode` and `filament_start_gcode`; it does not precede all
three injection points.

For an extrusion-role change, the order is
`change_extrusion_role_gcode` -> `filament_change_extrusion_role_gcode` ->
`process_change_extrusion_role_gcode` -> `;TYPE:<label>`. All three role
injection points are emitted before the `;TYPE:` raw marker.

Placeholder availability is site-specific. The common layer context is
available as `[layer_num]`, `[layer_z]`, and `[max_layer_z]` at
`before_layer_change_gcode`, `time_lapse_gcode`, `layer_change_gcode`, and
`machine_end_gcode`. It is also available at each toolchange and role site as
specified below; it is not available at `machine_start_gcode`.

| Injection point | Available placeholders |
| --- | --- |
| `machine_start_gcode` | none |
| `before_layer_change_gcode` | `[layer_num]`, `[layer_z]`, `[max_layer_z]` |
| `time_lapse_gcode` | `[layer_num]`, `[layer_z]`, `[max_layer_z]` |
| `layer_change_gcode` | `[layer_num]`, `[layer_z]`, `[max_layer_z]` |
| `machine_end_gcode` | `[layer_num]`, `[layer_z]`, `[max_layer_z]` |
| `filament_end_gcode` | `[layer_num]`, `[layer_z]`, `[max_layer_z]`, `[filament_extruder_id]` (the old tool) |
| `change_filament_gcode` | `[layer_num]`, `[layer_z]`, `[max_layer_z]`, `[previous_extruder]`, `[next_extruder]`, `[toolchange_count]` |
| `filament_start_gcode` | `[layer_num]`, `[layer_z]`, `[max_layer_z]`, `[filament_extruder_id]` (the new tool) |
| `change_extrusion_role_gcode` | `[layer_num]`, `[layer_z]`, `[extrusion_role]`, `[last_extrusion_role]` |
| `filament_change_extrusion_role_gcode` | `[layer_num]`, `[layer_z]`, `[extrusion_role]`, `[last_extrusion_role]` |
| `process_change_extrusion_role_gcode` | `[layer_num]`, `[layer_z]`, `[extrusion_role]`, `[last_extrusion_role]` |

At the three role sites, `[layer_num]` is N+1, where N is the current layer
index. At every other site, including all toolchange sites, `[layer_num]` is N.
The `[filament_extruder_id]` at `filament_end_gcode` identifies the old tool;
at `filament_start_gcode` it identifies the new tool. The role sites do not
provide `[max_layer_z]`.

Toolchange sites may run **before the first layer begins** (canonical
`set_extruder` calls during start G-code observe no layer yet). In that case
`[layer_num]` follows the canonical pre-layer value of **-1**; implementations
must not assume a layer context exists during start-G-code toolchanges.

When a per-site variable is unavailable, its placeholder remains verbatim, the
run returns `Ok`, and exactly one warning names the config key and injection
site. The unresolved bracketed text is therefore emitted unchanged rather than
aborting the slice (packet 186's warn-and-pass policy — an unresolved
placeholder is not a slice error).

For **list-valued configuration** (real 3MF input supplies per-extruder
settings as vectors, e.g. `nozzle_diameter` arrives as `['0.4']`), the
placeholder engine renders the **first element** of the list, recursively
(packet 186). An **empty** list supplies no value: the key stays out of the
lookup, the placeholder remains verbatim, and it is included in the aggregated
warning — it is **not** replaced with an empty string (`M104 S[nozzle_temperature]`
must never collapse to `M104 S`).

### Not implemented

These canonical injection points are not reachable through the PnP pipeline:

- `file_start_gcode` — the canonical point is emitted above `; HEADER_BLOCK_START`, but `DefaultGCodeSerializer::serialize_gcode` writes the header block before post-pass commands; the nearest serializer point is after `; HEADER_BLOCK_END`.
- `wrapping_detection_gcode` — it is gated by canonical `enable_wrapping_detection`, which has no PnP configuration or emission path.
- `machine_pause_gcode` — it is tied to per-print-Z `CustomGCode::PausePrint`, which PnP does not model.
- `template_custom_gcode` — it is tied to per-print-Z `CustomGCode::Template`, which PnP does not model.
- `printing_by_object_gcode` — it requires a by-object print sequence, and PnP has no by-object path.

Per `docs/adr/0051-gcode-marker-contract-ownership.md` (amendment recorded as
`D-285-ADR-0051-AMENDED` in `docs/DEVIATION_LOG.md`), the same warn-and-pass
policy covers a malformed `;LAYER_CHANGE` marker — one not followed within two
commands by a `;Z:` marker — at the layer-scoped injection sites. The
`machine-gcode-emit` walk reuses the prior layer Z (or, for layer 1, layer 1's
own initial Z context) and surfaces the malformed stream with one
`ERR_MALFORMED_LAYER_MARKER` warning. This is the documented fallback, not a
coincidental behaviour.

---

## Slicing precision (packet 60)

The host precision keys (`gcode_resolution`, `infill_resolution`,
`support_resolution`, `min_segment_length`, `gcode_xy_decimals`,
`slice_closing_radius`) carried on `ResolvedConfig` are in the generated
**Host-registered config keys** table above; `perimeter_arc_tolerance` is
module-owned (`classic-perimeters`; the fake `arachne-perimeters` module was
deleted in P108) and appears in the generated **Module-owned config keys**
table. Defaults / all-zero short-circuit
to byte-identical pre-packet-60 output.

See `docs/02_ir_schemas.md` "Polyline simplification and precision" for the
per-role tolerance dispatch table.

---

## Multi-layer shell thickness (packet 35)

`top_shell_layers` / `bottom_shell_layers` are in the generated
**Host-registered config keys** table above (`top_shell_layers` deviates from
OrcaSlicer's `4` — see the generated **Deviations from OrcaSlicer** table). They
set the top/bottom-surface classification windows in `classify_region_surfaces`;
per-region override is automatic via `RegionMapIR.entries[*].config` once
`RegionMapping` runs.

---

## Fill-role claims (packet 37)

Four `ResolvedConfig` keys — `top_fill_holder`, `bottom_fill_holder`,
`bridge_fill_holder`, `sparse_fill_holder` (each default `"rectilinear-infill"`)
— select the holder module for the corresponding fill-role claim. They are in
the generated **Host-registered config keys** table above. Each accepts any
loaded module that declares `holds = ["<role>-fill"]`. See
`docs/04_host_scheduler.md` validation pass 2 for conflict-resolution rules.
Per-region overrides are supported via `RegionMapIR.entries[*].config`.

---

## Authored coloring (packet 226)

| Key | Type | Default | Range | Source |
|---|---|---|---|---|
| `fill_authored_coloring` | string-list | `[]` | — (fill-role claim IDs permitted to author per-path tool indices; ADR-0058) | `resolved_config.rs::ResolvedConfig` |

`fill_authored_coloring` lists fill-role claim IDs — the four `FILL_CLAIM_IDS`
(`crates/slicer-scheduler/src/validation.rs`): `claim:top-fill`,
`claim:bottom-fill`, `claim:bridge-fill`, `claim:sparse-fill`, written exactly
as they appear in a manifest's `[claims] holds` — whose holder module is allowed
to set `ExtrusionPath3D.tool_index` for that region. It is not in the generated
**Host-registered config keys** table above, because that table's lock test
(`host_keys_doc_lock_tdd`) ties every generated host row to a scalar
`ResolvedConfig` default; it is a `cli` key on `ResolvedConfig` like the
`*_fill_holder` keys, and per-region overrides are supported via
`RegionMapIR.entries[*].config` — the same path as `infill_density`.

This key is **one half of a two-sided grant**. The module must *also* disclose
the capability claim `claim:authored-coloring` in its manifest (see
`docs/03_wit_and_manifest.md`'s `claim:authored-coloring` row). Either side
missing denies the grant: an undisclosed module gains nothing from being listed
here, and a disclosing module gains nothing until an operator lists its fill-role
claim for the region.

The default is the empty list, which means **deny** — out of the box no module
can author per-path tooling until an operator opts a fill role in.

The failure mode is a **silent strip, never an error**: when the grant is absent
— or the authored index is out of range against `tool-count` — the host resets
`tool_index` to `None` at the marshal/commit boundary and resolves the region
tool exactly as before. Nothing is logged as a config error and no slice fails.

See `docs/adr/0058-authored-coloring-per-path-tool-carrier.md` for the full
contract.

---

## Override namespaces

Two structural namespaces are recognised at runtime (see
`docs/02_ir_schemas.md` IR 5 "Config Key Namespaces" and IR 3 "Config
Precedence Rules").

| Namespace | Packet | Override target |
|---|---|---|
| `object_config:<object_id>:<key>` | 35a | Per-object override for a single `ObjectId`. |
| `paint_config:<semantic>:<key>` | 51 | Per-paint-semantic override; applies during `PrePass::RegionMapping`. |

Precedence (lowest → highest):

```
global < object_config:<id>:<key> < paint_config:<semantic>:<key>
```

`PaintSemantic` serialisation for `<semantic>`: `material`, `fuzzy_skin`,
`support_enforcer`, `support_blocker`, or the inner string for
`PaintSemantic::Custom(s)` (verbatim, hyphen-allowed).

---

## Walls (packet 104)

Keys consumed by `classic-perimeters` to gate single-wall reduction on specific layer types (the fake `arachne-perimeters` module was deleted in P108; a real Arachne implementation landed under P110+P112, and packet 148 registers `precise_outer_wall`, `wall_sequence`, and `seam_candidate_angle_threshold_deg` on `arachne-perimeters` for parity with `classic-perimeters`). Packet 149 re-publishes `only_one_wall_top` on `arachne-perimeters` and adds `min_width_top_surface` to both perimeter manifests. Defaults and source-of-truth live in the respective module manifests under `modules/core-modules/<name>/<name>.toml`.

| Key | Type | Default | Range | Module(s) |
|---|---|---|---|---|
| `only_one_wall_top` | bool | `false` | — | `classic-perimeters`, `arachne-perimeters` |
| `only_one_wall_first_layer` | bool | `false` | — | `classic-perimeters` |
| `outer_wall_line_width` | float_or_percent | `0` (auto sentinel → `1.125 × nozzle_diameter`) | [0.0, 2.0] | `classic-perimeters`, `arachne-perimeters` |
| `inner_wall_line_width` | float_or_percent | `0` (auto sentinel → `1.125 × nozzle_diameter`) | [0.0, 2.0] | `classic-perimeters`, `arachne-perimeters` |
| `precise_outer_wall` | bool | `false` | — | `classic-perimeters`, `arachne-perimeters` |
| `detect_thin_wall` | bool | `true` | — | `classic-perimeters` |
| `filter_out_gap_fill` | float | `0.5` | [0.0, 5.0] | `classic-perimeters` |
| `seam_candidate_angle_threshold_deg` | float | `30.0` | [0.0, 180.0] | `classic-perimeters`, `arachne-perimeters` |
| `wall_sequence` | string | `"InnerOuter"` | `OuterInner`, `InnerOuter`, `InnerOuterInner` | `classic-perimeters`, `arachne-perimeters` |
| `min_width_top_surface` | float_or_percent | `"0.0"` (gate off; upstream default `300%`) | base: `inner_wall_line_width` | `classic-perimeters` |
| `min_width_top_surface` | float_or_percent | `"0.0"` (code fallback: filter disabled; upstream default `300%`) | base: `preferred_bead_width_outer` | `arachne-perimeters` |

**`only_one_wall_top`** — when `true`, the perimeter generator reduces walls on top solid surfaces. **Packet 185 (AC-10)** redefined the topmost branch: `top_shell_index() == Some(0)` **unconditionally** forces exactly one wall on the topmost top sub-area (canonical `process_classic`'s topmost `loop_number = 0`), with no `min_width_top_surface` gate. Non-topmost top sub-areas (`top_shell_index() == Some(N>0)`) apply the `min_width_top_surface` threshold module-locally (following `emit_only_one_wall_top_second_pass` in `arachne-perimeters`); the generic `split_top_surfaces` in `crates/slicer-core/src/top_surface_split.rs` keeps its current signature and behavior. On non-top layers (`top_shell_index() == None`) the key is a no-op. Classic's infill-overlap selection (AC-11) is: `top_bottom_infill_wall_overlap` is selected on layer 0 or on topmost regions (`top_shell_index() == Some(0)`); for all other regions `infill_wall_overlap` is selected.

**`only_one_wall_first_layer`** — when `true`, the perimeter generator emits a single outer wall on the first layer of the print (layer index 0).

**`outer_wall_line_width`** — extrusion width for the outermost wall loop. Overrides the module-level `line_width` for outer walls only; allows a narrower outer wall for surface detail without affecting inner walls.

**`inner_wall_line_width`** — extrusion width for all inner wall loops. Overrides `line_width` for inner walls only.

**Serializer header width defaults (packet 182).** When the G-code serializer
(`DefaultGCodeSerializer`) is built with no resolved width override, its
header-comment fallback reports `; outer_wall_line_width = 0.4` and
`; inner_wall_line_width = 0.4`, matching the pipeline's `legacy_line_width`
(`0.4` mm) fallback. These are **serializer defaults, not per-slice
resolved-width reporting** — the header stays config-blind and is not wired to
the resolved widths; it simply reports the value the pipeline falls back to.

**Packet 184 (wall-width type parity, classic half).** Both keys on `classic-perimeters` are retyped `float_or_percent` with `unit = "mm"`, `ratio_over` the nozzle diameter, `max = 2.0`, and `min` lowered from `0.1` to `0.0` — the lowered floor exists so the canonical auto sentinel is accepted, because scalar range checks run against the raw *unresolved* magnitude. Width resolution happens through the shared `resolve_role_width` resolver (packet 185, below); an explicit value `> 0.0` is used as-is (an explicit mm float, or a percent of nozzle diameter), and a resolved value of exactly `0` is OrcaSlicer's **auto sentinel** — canonical `Flow.cpp::new_from_config_width` routes `0` to `Flow.cpp::auto_extrusion_width`, which returns `1.125 × nozzle_diameter` for both `frExternalPerimeter` and `frPerimeter`.

The `nozzle_diameter` read is hoisted **above** both width reads and its own fallback is `legacy_line_width` (previously `inner_wall_line_width`), breaking a read cycle between the nozzle key and the widths it now feeds.

### Shared width resolver precedence (packet 185)

`resolve_role_width` (`crates/slicer-core/src/flow.rs`) is the single context-aware
resolver consumed by all five width-consuming core modules (`classic-perimeters`,
`arachne-perimeters`, `rectilinear-infill`, `gyroid-infill`, `lightning-infill`).
The normative precedence chain (locked):

1. **`bridge_line_width`** first — a positive configured `bridge_line_width` wins on a bridge context, even on the first layer;
2. else a positive **`initial_layer_line_width`** on the first layer;
3. else the **role width** (the canonical role-keyed key — see the role map below);
4. a role width of exactly **`0`** (the auto sentinel) falls back to **`line_width`**;
5. a zero `line_width` falls back to **auto width** = `1.125 × nozzle_diameter`.

Role-keyed mapping: `BottomSolidInfill` (and `InternalSolidInfill`) → `internal_solid_infill_line_width`; `SparseInfill` → `sparse_infill_line_width`; `TopSolidInfill` → `top_surface_line_width`; there is **no** bottom-specific width key. `BottomSolidInfill`'s canonical key resolution is therefore `internal_solid_infill_line_width` except for the first-layer/bridge overrides above. Resolution covers **geometric widths/spacing only** — canonical top/bottom/internal-solid flow-ratio controls are excluded (deferred as `DEV-102`). An explicit `0.4` remains exactly `0.4`; only absent/zero keys route through the auto sentinel. Defaults moved to the `0` auto sentinel (AC-5), including global `line_width`; the key rename `first_layer_line_width` → `initial_layer_line_width` (AC-4) keeps a scheduler-side legacy alias that rejects profiles specifying both names (AC-N1). Percent/`FloatOrPercent` values are transported through `ResolvedConfig.extensions` un-resolved (TASK-303).

Two residuals were tracked as one shared wall-width/percent-transport residual in `docs/DEVIATION_LOG.md`:

1. **Closed by packet 185 (AC-5 and closure-note items 3/5/6):** the `0.4` mm manifest/absent-key fallback is **gone** — Classic, Arachne, rectilinear, gyroid, and lightning absent width keys now use the canonical auto-`0` sentinel, resolving to `1.125 × nozzle_diameter` (0.675 mm at nozzle 0.6), identical to the explicit-zero case; the earlier `[FWD-1]` decision ("keep legacy 0.4 for absent keys") was superseded. An explicit `0.4` remains explicit. See the ADR-0043 amendment.
2. **Ingestion residual:** no live slice could carry a `Percent` / `FloatOrPercent` value end-to-end because the config **parser** discarded what `parse_percent_default` parsed (`parse_config_field_entry` invoked it as a bare validation statement). **Closed by packet 185 (TASK-303):** `parse_config_field_entry` now retains the parsed `ConfigValue` on `ConfigFieldEntry::parsed_default`, and scheduler config resolution threads it into `ResolvedConfig.extensions`, so a percent-typed flow key with a schema default reaches a live slice unchanged. `ResolvedConfig::to_config_map`'s `extensions` pass-through was never the barrier — it is a transparent channel that already carries any variant.

**`precise_outer_wall`** — when `true`, the perimeter generator compensates outer-wall width to hit the model boundary precisely. Gated on `wall_sequence == InnerOuter` because inner walls must be committed first for the compensation math to work.

**`detect_thin_wall`** — when `true`, the perimeter generator inserts `LoopType::ThinWall` extrusion paths in regions too narrow for a full wall pair. Disable to suppress thin-wall fill in favour of gap-fill only.

**`filter_out_gap_fill`** — minimum gap width (mm) below which `LoopType::GapFill` paths are suppressed. `0.0` means emit all gap-fill. Values larger than `line_width` suppress most gap-fill paths. Emitted as `ExtrusionRole::GapFill` in G-code.

**`wall_sequence`** — controls the print order of outer and inner walls per layer. Enum variants:
- `OuterInner` — outer wall prints first; better surface quality on slow machines.
- `InnerOuter` — inner walls print first; better dimensional accuracy (default).
- `InnerOuterInner` — inner walls first, outer wall next, remaining inner walls last; balances both goals by bracketing the outer wall.

**`min_width_top_surface`** — OrcaSlicer `min_width_top_surface` (`coFloatOrPercent`, canonical default `300%` of line width). Minimum wall width applied when narrowing walls on top solid surfaces. **Packet 150:** `arachne-perimeters`'s copy is retyped `float_or_percent`, resolved module-side via `ConfigView::get_abs_value`, closing G6/D-104h for this key. **Packet 152:** `arachne-perimeters` consumes it — the `only_one_wall_top` second WallToolPaths pass filters top sub-area walls by this width.

**Packet 184 (D-152 classic half, now closed).** `classic-perimeters` no longer registers this as a fixed mm float and no longer reads-and-discards it. Both perimeter modules now agree on `type = "float_or_percent"`, `default = "0.0"`, `unit = "%"`, `min = 0.0`, and `classic-perimeters` **gates its `only_one_wall_top` split on it**: at the split call site, top sub-areas whose minimum bounding-box extent falls below the resolved threshold are retained out of the top portion (they keep the full configured wall count), and the survivors are passed through `offset2_ex` (shrink by `-t`, expand by `+t + 0.85 × perimeter_width`, the `0.85` thin-lettering constant kept verbatim from canonical `PerimeterGenerator::split_top_surfaces`) with an empty-result fallback. At the `"0.0"` default the gate is **off**, so behaviour is unchanged for existing profiles.

**Nuance — the two modules resolve the same key against different bases.** `classic-perimeters` resolves it via `get_abs_value` against `inner_wall_line_width`, matching canonical's `ratio_over`. `arachne-perimeters` resolves it against `ArachneParams::preferred_bead_width_outer` (the canonical `bead_width_0` target, derived from `outer_wall_line_width` per ADR-0043). When outer and inner wall widths are equal — the default — the two agree; when they differ, the *same percent string* resolves to a different absolute threshold in each module. Reconciling the arachne base to the inner width is not packet 184's work.

The gate deliberately does **not** port canonical's additional `ext_perimeter_spacing/2` floor on the threshold (`[FWD-2]`), matching the already-landed arachne half.

---

## Overhangs (packet 149)

Keys registered on both `classic-perimeters` and `arachne-perimeters` mirroring OrcaSlicer's overhang-wall `PrintConfig.cpp` options. Defaults and source-of-truth live in the respective module manifests under `modules/core-modules/<name>/<name>.toml`.

| Key | Type | Default | Range | Module(s) |
|---|---|---|---|---|
| `detect_overhang_wall` | bool | `true` | — | `classic-perimeters`, `arachne-perimeters` |
| `overhang_reverse` | bool | `false` | — | `classic-perimeters`, `arachne-perimeters` |
| `overhang_reverse_internal_only` | bool | `false` | — | `classic-perimeters`, `arachne-perimeters` |
| `extra_perimeters_on_overhangs` | bool | `false` | — | `classic-perimeters`, `arachne-perimeters` (re-published on `arachne-perimeters`; `classic-perimeters` has carried this key since packet 108 / T-077) |

**`detect_overhang_wall`** — when `true`, the perimeter generator identifies wall segments that overhang the layer below for downstream overhang-aware handling (speed/fan classification, reversal). Registered by packet 149; the detection/classification consumer remains the existing `overhang-classifier-default` finalization module (see `docs/15_config_keys_reference.md` "Overhang speed key consumption" note below), not new code in the perimeter modules themselves.

**`overhang_reverse`** / **`overhang_reverse_internal_only`** — mirror OrcaSlicer's wall-winding-reversal options for overhang segments (alternating print direction to reduce sagging on printed-in-air perimeters; `_internal_only` restricts reversal to internal, non-outer-visible walls). Registered by packet 149 with OrcaSlicer's own defaults (`false`/`false`). **Gap, not yet closed:** no code path currently changes wall winding or direction based on these keys.

**`extra_perimeters_on_overhangs`** — already present on `classic-perimeters` since packet 108/T-077 (see the generated **Module-owned config keys** table above); packet 149 re-publishes it on `arachne-perimeters` for cross-generator parity. When `true`, adds extra perimeter loops specifically over overhang regions to improve their surface strength.

---

## Strength (packet 149)

Keys registered on `arachne-perimeters` for the alternating-extra-wall strength feature, plus two pre-existing gate keys re-registered here for the alternating-extra-wall gate condition.

| Key | Type | Default | Range | Module(s) |
|---|---|---|---|---|
| `alternate_extra_wall` | bool | `false` | — | `arachne-perimeters` |
| `spiral_vase` | bool | `false` | — | `arachne-perimeters` |
| `sparse_infill_density` | float | `20.0` | % | `arachne-perimeters` |

**`alternate_extra_wall`** — OrcaSlicer `alternate_extra_wall` (`coBool`, default `false`). When `true`, `arachne-perimeters` bumps `ArachneParams.max_bead_count` by `+2` on odd layers — this codebase's beading stack emits `max_bead_count / 2` walls, so a `+2` bump is the PnP-side equivalent of OrcaSlicer's `loop_number++` for this option. Gated on `!spiral_vase && sparse_infill_density > 0`, mirroring OrcaSlicer's own gate (alternating extra walls only make sense with solid infill present and outside spiral-vase mode).

**`spiral_vase`** — registered on `arachne-perimeters` solely to provide the gate condition for `alternate_extra_wall` above; no spiral-vase toolpath behavior is implemented by this packet.

**`sparse_infill_density`** — registered on `arachne-perimeters` solely to provide the gate condition for `alternate_extra_wall` above (`> 0` means solid infill exists); does not change infill density behavior on the perimeter module itself (see `gyroid-infill`/`rectilinear-infill`/`lightning-infill`'s own `infill_density` keys in the generated **Module-owned config keys** table for the actual infill-density consumers).

---

## Bridging (packet 149)

Keys registered on both `classic-perimeters` and `arachne-perimeters`, consumed by `slicer_core::flow::bridging_flow(bridge_flow_ratio, thick_bridges)` and applied to `is_bridge` vertices' `flow_factor`.

| Key | Type | Default | Range | Module(s) |
|---|---|---|---|---|
| `bridge_flow` | float | `1.0` | — | `classic-perimeters`, `arachne-perimeters` |
| `thick_bridges` | bool | `false` | — | `classic-perimeters`, `arachne-perimeters` |

**`bridge_flow`** — OrcaSlicer `bridge_flow_ratio` equivalent. Scales the per-vertex `flow_factor` applied at `is_bridge` vertices; `1.0` is a no-op.

**`thick_bridges`** — when `true`, `bridging_flow()` now computes OrcaSlicer's round cross-section flow factor `π·dmr²/(4·w·h)` (`dmr = nozzle_diameter·sqrt(bridge_flow_ratio)`), instead of the configured `bridge_flow` ratio directly. **Packet 150:** replaced the previous hardcoded `1.0` stub with this per-vertex `Flow::bridging_flow` parity formula, closing G5.

---

## Wall count, winding, and simplification tolerances (packet 151)

Six keys registered on `arachne-perimeters` closing Arachne parity gaps
G1/G2/G7/G8/G9 (packet 151).
Defaults and source-of-truth live in
`modules/core-modules/arachne-perimeters/arachne-perimeters.toml`.

| Key | Type | Default | Range / values | Description |
|---|---|---|---|---|
| `wall_count` | int | `3` | >= 1 | User-facing per-region perimeter count; consumed as `max_bead_count = 2 × wall_count` (Orca `WallToolPaths.cpp:525`). |
| `wall_direction` | string | `"counter_clockwise"` | `counter_clockwise`, `clockwise` | Contour (outer-surface) winding direction; holes are always wound opposite the contour. |
| `only_one_wall_first_layer` | bool | `false` | — | When `true`, forces a single wall (`max_bead_count = 2`) on layer 0 instead of `wall_count`. |
| `overhang_reverse_threshold` | float_or_percent | `"0.0"` | [0.0, 10.0] mm | Overhang-steepness threshold for `overhang_reverse`; `0` treats every overhang as steep and reverses. |
| `wall_maximum_resolution` | float | `0.05` | [0.001, 10.0] mm | Minimum wall line-segment length (mm) for Arachne wall simplification; replaces `meshfix_maximum_resolution` on the wall path. |
| `wall_maximum_deviation` | float | `0.005` | [0.0001, 1.0] mm | Allowed positional error (mm) for Arachne wall simplification; replaces `meshfix_maximum_deviation` on the wall path. |

**`wall_count`** — OrcaSlicer's user-facing per-region perimeter count. `arachne-perimeters` translates it to `max_bead_count = 2 × wall_count` (Orca `Arachne/WallToolPaths.cpp:525`) when `max_bead_count` is not explicitly set; an explicit `max_bead_count` still wins. Closes G1's sibling `wall_count` gap and the AC-1 `wall_count` acceptance criterion.

**`wall_direction`** — OrcaSlicer `wall_direction` (`coEnum`, `PrintConfig.cpp:2188-2198`, default `CounterClockwise`). Contour (`ExteriorSurface`) loops are forced CCW or CW per this key; hole loops are always wound opposite the contour (`PerimeterGenerator.cpp:527-545`). Closes G1.

**`only_one_wall_first_layer`** — OrcaSlicer `only_one_wall_first_layer` (`coBool`, `PrintConfig.cpp:1513-1517`). On layer 0 the perimeter generator forces `loop_number = 0` — a single outer wall — regardless of `wall_count` (`PerimeterGenerator.cpp:2137-2139`). Also registered on `classic-perimeters` (see the "Walls (packet 104)" table above). Closes G2.

**`overhang_reverse_threshold`** — OrcaSlicer `overhang_reverse_threshold` (`coFloatOrPercent`, `PerimeterGenerator.cpp:68-77`). Advisory companion to `overhang_reverse` / `overhang_reverse_internal_only` (see "Overhangs (packet 149)" above): when `0`, overhang detection treats every overhang as steep and reverses wall direction on odd layers. Closes G7.

**`wall_maximum_resolution`** / **`wall_maximum_deviation`** — OrcaSlicer `wall_maximum_resolution` / `wall_maximum_deviation` (`coFloat`, `PrintConfig.cpp:7242-7263`, upstream defaults `0.5` mm / `0.025` mm; PnP manifest defaults state the CODE fallbacks `0.05` mm / `0.005` mm per the reconcile guard — the code-vs-upstream default divergence is logged as `D-168-ARACHNE-SIMPLIFY-FALLBACKS-TIGHTER-THAN-CANONICAL`). These REPLACE `meshfix_maximum_resolution` / `meshfix_maximum_deviation` for the Arachne wall path (Orca `WallToolPaths.cpp:487-503,702-719`); they are wired directly (no `min()`/merge) into `ArachneParams.smallest_line_segment_squared` / `allowed_error_distance_squared` as mm² (squared). The third upstream tolerance `meshfix_maximum_extrusion_area_deviation` is a distinct parameter and intentionally NOT replaced here. Closes G9.

---

## Arachne beading strategy stack (packet 111)

Keys registered on `arachne-perimeters` for the `slicer_core::beading` `BeadingStrategy` stack (`crates/slicer-core/src/beading/`, T-210..T-216). Consumed by `BeadingStrategyFactory::create_stack` (`crates/slicer-core/src/beading/factory.rs`) — wiring into `arachne-perimeters::run_perimeters` itself is still P112's T-230. All slicer-unit defaults below assume a 0.4 mm nozzle diameter (1 unit = 100 nm; see `docs/08_coordinate_system.md`) — OrcaSlicer's `PrintConfig.cpp` registers 6 of the 13 as `coPercent` (percentage of nozzle diameter) rather than fixed lengths, so the absolute defaults here are derived (`percent × 0.4 mm`), not literal upstream constants. Two of the original 13 (`outer_wall_offset`, `max_bead_count`) have no upstream `PrintConfig.cpp` entry at all — internal Arachne C++ algorithm parameters in `libslic3r/Arachne/` exposed as config keys because this codebase's module boundary requires them to be configurable. Two more internal parameters (`optimal_width`, `preferred_bead_width_outer`) were ALSO exposed that way until D-160: they shadowed the user's wall widths and made arachne output invariant to `outer_wall_line_width`/`inner_wall_line_width`. They are RETIRED (ADR-0043); the module now derives them from the wall-width keys exactly as canonical `PerimeterGenerator` derives `bead_width_0`/`bead_width_x` from the wall flows. The remaining new key, `detect_thin_wall`, is a real `PrintConfig.cpp` `coBool` option (not a `coPercent`), gating whether `WideningBeadingStrategy` is wrapped into the stack at all.

| Key | Type | Default | Units | Module |
|---|---|---|---|---|
| `min_feature_size` | percent | `25%` | % of `nozzle_diameter` | `arachne-perimeters` |
| `min_bead_width` | float | `4000` | slicer units (0.4 mm) | `arachne-perimeters` |
| `wall_transition_filter_deviation` | float | `1000` | slicer units (0.1 mm) | `arachne-perimeters` |
| `wall_transition_length` | percent | `100%` | % of `nozzle_diameter` | `arachne-perimeters` |
| `wall_transition_angle` | float | `10.0` | degrees | `arachne-perimeters` |
| `wall_distribution_count` | int | `1` | count (bead-index radius) | `arachne-perimeters` |
| `min_length_factor` | float | `0.5` | dimensionless ratio | `arachne-perimeters` |
| `initial_layer_min_bead_width` | float | `3400` | slicer units (0.34 mm) | `arachne-perimeters` |
| `outer_wall_offset` | float | `0` | slicer units | `arachne-perimeters` |
| `max_bead_count` | int | `0` | >= 0 | `arachne-perimeters` |
| `detect_thin_wall` | bool | `false` | boolean | `arachne-perimeters` |
| `inner_wall_line_width` | float_or_percent | `0` (auto sentinel) | mm (canonical `bead_width_x` source) | `arachne-perimeters` |
| `outer_wall_line_width` | float_or_percent | `0` (auto sentinel) | mm (canonical `bead_width_0` source) | `arachne-perimeters` |

**`min_feature_size`** — OrcaSlicer `min_feature_size` (`PrintConfig.cpp` ~line 6836-6845, `coPercent` of nozzle diameter, upstream default `25%`). **Packet 150:** retyped `percent`, base `nozzle_diameter` (resolved module-side via `ConfigView::get_abs_value`), closing G6/D-104h. Below this thickness, a region is too narrow for the wrapped strategy's normal bead distribution. **Maps to `WideningBeadingStrategy`'s internal `min_input_width` field** (`crates/slicer-core/src/beading/widening.rs`) — confirmed via the OrcaSlicer tooltip ("Minimum thickness of thin features; thinner is not printed, thicker is widened to min wall width"), which matches `min_input_width`'s role as the sub-threshold-detection cutoff exactly.

**`min_bead_width`** — OrcaSlicer `min_bead_width` (`PrintConfig.cpp` ~line 6873-6879, `coPercent` of nozzle diameter, upstream default `100%`; corrected here from the packet's original `200`-unit suggestion). The fixed bead width `WideningBeadingStrategy` emits for regions below `min_feature_size`; maps to its internal `min_bead_width` field (name matches verbatim).

**`wall_transition_filter_deviation`** — OrcaSlicer `wall_transition_filter_deviation` (`PrintConfig.cpp` ~line 6799-6812, `coPercent` of nozzle diameter, upstream default `25%`; corrected here from the packet's original `200`-unit suggestion). Margin extending the extrusion-width range to reduce back-and-forth transitions between wall counts; maps to `DistributedBeadingStrategy`'s internal `transition_filter_dist` field (`crates/slicer-core/src/beading/distributed.rs`) — reserved there for a later decorator step, not yet read by `compute`.

**`wall_transition_length`** — OrcaSlicer `wall_transition_length` (`PrintConfig.cpp` ~line 6788-6797, `coPercent` of nozzle diameter, upstream default `100%`). **Packet 150:** retyped `percent`, base `nozzle_diameter` (resolved module-side via `ConfigView::get_abs_value`), closing G6/D-104h. Space allotted to split/join wall segments when transitioning between wall counts; maps to `DistributedBeadingStrategy`'s `default_transition_length` field. **Packet 143** wired it live: consumed by `get_transitioning_length` (`crates/slicer-core/src/beading/distributed.rs`) in the transition-end machinery (`filter_transition_mids`/`generate_all_transition_ends`).

**`wall_transition_angle`** — OrcaSlicer `wall_transition_angle` (`PrintConfig.cpp` ~line 6814-6825, `coFloat`, degrees, upstream default `10.0` — matches the packet's original suggestion exactly). Threshold wedge angle above which no wall-count transition occurs. **Packet 144** wired it live: threaded via `BeadingFactoryParams::wall_transition_angle` into `filter_central` (`crates/slicer-core/src/skeletal_trapezoidation/centrality.rs`) as the centrality-predicate threshold (replacing a temporary π-cap workaround; see the historical centrality-threshold correction).

**`wall_distribution_count`** — OrcaSlicer `wall_distribution_count` (`PrintConfig.cpp` ~line 6827-6834, `coInt`, dimensionless count, upstream default `1` — matches the packet's original suggestion exactly). Maps directly to `DistributedBeadingStrategy`'s internal `distribution_count` field — the Gaussian decay radius (in bead-count units) used by `compute`'s surplus/deficit redistribution.

**`min_length_factor`** — dimensionless ratio (default `0.5`), the multiplier consumed by the not-yet-ported `removeSmallLines` step (roadmap T-227: drops odd, non-closed lines shorter than `min_length_factor * min_width`). The OrcaSlicer `PrintConfig.cpp` key found under this exact name registers as a `coFloat` in mm rather than a ratio, which may be a distinct UI-facing option sharing the name rather than the internal Arachne algorithm parameter T-227 targets; the ratio semantics here follow the well-documented CuraEngine/Orca Arachne source (`WallToolPaths.cpp`) that T-227 cites. **Packet 146** wired it live: consumed by `remove_small_lines` (`crates/slicer-core/src/arachne/remove_small.rs`) as the non-top/bottom-layer divisor (`min_width * min_length_factor`).

**`initial_layer_min_bead_width`** — OrcaSlicer `initial_layer_min_bead_width` (`PrintConfig.cpp` ~line 6863-6871, `coPercent` of nozzle diameter, upstream default `85%`; corrected here from the packet's original `850`-unit suggestion, which mistook the percentage for a raw slicer-unit value). Minimum wall width for the first layer. Consumed via `BeadingFactoryParams::initial_layer_min_bead_width` → `WideningBeadingStrategy` (`crates/slicer-core/src/beading/`), read module-side in `arachne-perimeters` (`arachne_params_from_config`).

**`outer_wall_offset`** — not a user-facing OrcaSlicer `PrintConfig.cpp` option; it is an internal Arachne algorithm parameter (`coord_t`) threaded through `BeadingStrategyFactory`/`OuterWallInsetBeadingStrategy`. Maps to `OuterWallInsetBeadingStrategy`'s offset amount; `0` (matches the packet's original suggestion) disables the decorator's inward offset.

**`max_bead_count`** — not a user-facing OrcaSlicer `PrintConfig.cpp` option; upstream computes it internally as `2 * inset_count` (capped) in `Arachne/WallToolPaths.cpp`. This codebase exposes it directly as a config key consumed by `LimitedBeadingStrategy`'s cap threshold. The manifest default is `0`, meaning auto-derive `2 × wall_count`; an explicit non-zero `max_bead_count` overrides that.

**`inner_wall_line_width` / `outer_wall_line_width`** (on `arachne-perimeters`) — `float_or_percent` keys with default `0` (the auto sentinel, matching upstream `coFloatOrPercent` default `0`), mirroring the classic-perimeters keys. The module derives its two beading targets from them: `optimal_width` (struct field; canonical `bead_width_x` = `perimeter_flow.scaled_spacing()`, i.e. the INNER wall) from `inner_wall_line_width`, and `preferred_bead_width_outer` (struct field; canonical `bead_width_0` = `ext_perimeter_flow.scaled_spacing()`, i.e. the OUTER wall) from `outer_wall_line_width`, converting width → Flow spacing via `line_width_to_spacing` before feeding the strategy stack and converting back at emission (`VariableWidth.cpp::thick_polyline_to_multi_path`). The former config keys `optimal_width`/`preferred_bead_width_outer` — Arachne-internal knobs exposed as user config — are RETIRED per ADR-0043 because they shadowed the wall widths. The `ArachneParams` STRUCT fields keep the canonical names; only the config keys are gone.

**`detect_thin_wall`** — OrcaSlicer `detect_thin_wall` (`PrintConfig.cpp:6299-6305`, `coBool`, upstream default `false`, label "Detect thin wall", tooltip "Detect thin wall which can't contain two line width. And use single line to print."). Gates whether `WideningBeadingStrategy` is wrapped into the `BeadingStrategyFactory::create_stack` composition at all — maps to the internal Arachne `print_thin_walls` parameter passed into `BeadingStrategyFactory::makeStrategy`. `false` (the default, matching upstream exactly) means `WideningBeadingStrategy` is **absent from the stack entirely**, not merely a no-op — the same absent-vs-no-op convention already used for `OuterWallInsetBeadingStrategy`/`outer_wall_offset`.

---

## Maintenance Notes

- When adding a new config key:
  1. Choose host-registered **only if** the key is consumed by a host built-in.
     Otherwise the module manifest (`modules/core-modules/<name>/<name>.toml`
     `[config.schema]`) is the right home.
  2. For a **module-owned** key: add it to the manifest, then run
     `cargo xtask gen-config-docs` — the generated tables above update
     automatically. Do not hand-edit the generated blocks.
  3. For a **host-registered** key: add the default to the consumer struct, mirror
     it into `docs/config/host-keys.toml`, extend the lock test in
     `FeedrateConfig` in `crates/slicer-ir/src/feedrate.rs` (locked by `host_keys_doc_lock_tdd`), then run `cargo xtask gen-config-docs`.
  4. Cross-reference from the relevant packet's design doc.
- Removing a key requires a major IR / WIT bump (see
  `docs/02_ir_schemas.md` "IR Versioning Contract").
- This file is enumerated; the `docs/03_wit_and_manifest.md` "Config Field
  Types Reference" remains the source of truth for the meta-format
  (`type`, `min`, `max`, `unit`, `display`, `group`, `advanced`).
