# Asset — Scoped gap list after triage (ticket 03)

Derived from ticket 01's inventory by removing 42 keys ruled out of scope and 24 keys that are already implemented under a Pinch name. **415 keys remain** — this is the set the packet queue must cover (ticket 105 re-adjudicated `resolution` from the rename pool into this gap set: the host's emit-time per-role `gcode_resolution` does not implement canonical's generation-time global simplification).

## Ruled out of scope (42)

| key | class |
|---|---|
| `allow_multicolor_oneplate` | Bambu-proprietary hardware |
| `bbl_calib_mark_logo` | Bambu-proprietary hardware |
| `bbl_use_printhost` | print-host / preset management |
| `bed_custom_model` | plater / GUI state |
| `bed_custom_texture` | plater / GUI state |
| `best_object_pos` | plater / GUI state |
| `default_filament_colour` | filament metadata (non-physical) |
| `default_filament_profile` | print-host / preset management |
| `default_print_profile` | print-host / preset management |
| `enable_wrapping_detection` | Bambu-proprietary hardware |
| `filament_adhesiveness_category` | filament metadata (non-physical) |
| `filament_colour_type` | filament metadata (non-physical) |
| `filament_ids` | filament metadata (non-physical) |
| `filament_multi_colour` | filament metadata (non-physical) |
| `filament_notes` | filament metadata (non-physical) |
| `filament_printable` | filament metadata (non-physical) |
| `filament_settings_id` | filament metadata (non-physical) |
| `filament_vendor` | filament metadata (non-physical) |
| `head_wrap_detect_zone` | Bambu-proprietary hardware |
| `host_type` | print-host / preset management |
| `notes` | plater / GUI state |
| `nozzle_flush_dataset` | Bambu-proprietary hardware |
| `pellet_flow_coefficient` | pellet extruder hardware |
| `pellet_modded_printer` | pellet extruder hardware |
| `preferred_orientation` | plater / GUI state |
| `print_host` | print-host / preset management |
| `print_host_webui` | print-host / preset management |
| `print_order` | plater / GUI state |
| `printer_agent` | print-host / preset management |
| `printer_notes` | print-host / preset management |
| `printer_settings_id` | print-host / preset management |
| `printhost_apikey` | print-host / preset management |
| `printhost_authorization_type` | print-host / preset management |
| `printhost_cafile` | print-host / preset management |
| `printhost_password` | print-host / preset management |
| `printhost_port` | print-host / preset management |
| `printhost_ssl_ignore_revoke` | print-host / preset management |
| `printhost_user` | print-host / preset management |
| `scan_first_layer` | Bambu-proprietary hardware |
| `upward_compatible_machine` | print-host / preset management |
| `wrapping_detection_layers` | Bambu-proprietary hardware |
| `wrapping_exclude_area` | Bambu-proprietary hardware |

## Already implemented under a Pinch name (24)

These were counted as gaps by exact-name matching in ticket 01. Each is a **false gap**.

| OrcaSlicer key | Pinch 'n Print key | owner | fidelity |
|---|---|---|---|
| `fuzzy_skin_thickness` | `thickness` | `fuzzy-skin` | exact — **renamed in ticket 103** |
| `fuzzy_skin_point_distance` | `point_distance` | `fuzzy-skin` | exact — **renamed in ticket 103** |
| `close_fan_the_first_x_layers` | `disable_fan_first_layers` | `part-cooling` | exact — **renamed in ticket 99** |
| `fan_max_speed` | `fan_speed_max` | `part-cooling` | exact (word order) — **renamed in ticket 99**; scale deviation: Orca 0–100 % vs Pinch raw 0–255 → reclassified as gap work (P01) |
| `fan_min_speed` | `fan_speed_min` | `part-cooling` | exact (word order) — **renamed in ticket 99**; same scale deviation + declared-but-never-read → reclassified as gap work (P01) |
| `enable_overhang_bridge_fan` | `enable_overhang_fan` | `part-cooling` | exact — **renamed in ticket 99** |
| `retraction_length` | `retract_length` | `path-optimization-default` | exact — **renamed in ticket 101**; default 0.8 matches Orca |
| `retraction_speed` | `retract_speed` | `path-optimization-default` | exact — **renamed in ticket 101**; default aligned 25.0 → **30.0** (user ruling in-ticket) |
| `z_hop` | `travel_z_hop` | `path-optimization-default` | exact — **renamed in ticket 101**; default aligned 0.0 → **0.4** and range `[0, 5]` adopted (user ruling in-ticket) |
| `seam_position` | `seam_mode` | `seam-placer`, `seam-planner-default` | exact — **renamed in ticket 102**; default `aligned` already matches Orca (`spAligned`) |
| `wall_loops` | `wall_count` | `classic-perimeters` | exact — **renamed in ticket 102**; default aligned 3 → **2** to match Orca (user ruling in-ticket; host `ResolvedConfig` was already 2) |
| `printable_area` | `bed_shape` | `wipe-tower` | exact (Orca renamed Slic3r's `bed_shape`) — **renamed in ticket 100** |
| `prime_volume` | `wipe_tower_purge_volume` | `wipe-tower` | exact — **renamed in ticket 100**; default deviation surfaced: Pinch `10.0` vs Orca `45.0` (see 100) |
| `enable_prime_tower` | `wipe_tower_enabled` | `wipe-tower` | exact — **renamed in ticket 100**; the rename also makes Orca 3MF `enable_prime_tower` a *declared* key, so it now reaches the typed extractor instead of `extensions` |
| `prime_tower_width` | `wipe_tower_width` | `wipe-tower` | exact — **renamed in ticket 100**; defaults match Orca (60.0), no deviation |
| `support_top_z_distance` | `support_top_z_distance_mm` | `support-planner`, `tree-support` | exact (unit suffix) — **renamed in ticket 104** |
| `small_perimeter_threshold` | `smaller_perimeter_threshold_mm` | `classic-perimeters` | exact (unit suffix) — **renamed in ticket 102**; default aligned 0.8 → **0.0** to match Orca's "0 = no threshold effect" (user ruling in-ticket) |
| `initial_layer_print_height` | `first_layer_height` | `layer-planner-default` | exact — **renamed in ticket 104**; manifest default aligned 0.3 → **0.2** to match Orca (user ruling in-ticket; host `ResolvedConfig` was already 0.2) |
| `infill_direction` | `infill_angle` | `gyroid-infill`, `rectilinear-infill` | exact — **renamed in ticket 105** |
| `raft_layers` | `support_raft_layers` + `base_raft_layers` + `interface_raft_layers` | `support-planner` | **split** — one Orca key became three |
| `ironing_type` | `ironing_enabled` | `top-surface-ironing` | **narrowed** — Orca enum → Pinch bool |
| `support_ironing` | `ironing_enabled` | `support-surface-ironing` | **narrowed** — Orca enum → Pinch bool |
| `support_ironing_flow` | `ironing_flow_rate` | `support-surface-ironing` | exact — **renamed in ticket 106**; manifest default aligned 100.0 → **0.10** to match Orca's 10% coPercent (user ruling in-ticket: `flow_factor` is a raw multiplier, canonical is percent-of-flow; range mirrored to [0.01, 1.0]) |
| `support_ironing_spacing` | `ironing_spacing` | `support-surface-ironing` | exact — **renamed in ticket 106** |

## Judged Pinch-specific — no OrcaSlicer counterpart (37)

The remainder of ticket 01's 62-key rename pool. These do **not** remove anything from the gap list; they are recorded so the alias map (ticket 07) knows they need no Orca entry.

| Pinch key | note |
|---|---|
| `top_fill_holder`, `bottom_fill_holder`, `sparse_fill_holder`, `bridge_fill_holder` | module-claim routing (packet 37); an architecture concept Orca has no analogue for |
| `slice_has_paint`, `gap_fill_medial_axis_on_painted` | painted-region handling |
| `path_optimization_emit_layer_markers` | debug/diagnostic output |
| `filament_change_extrusion_role_gcode`, `process_change_extrusion_role_gcode` | Pinch-only; Orca has only `change_extrusion_role_gcode` (separately declared, and live) |
| `flat_bridge_closing_join` | flat-bridge enclosure join style; Orca has no key |
| `min_segment_length`, `infill_resolution`, `support_resolution`, `perimeter_arc_tolerance` | Pinch splits path simplification per-domain; Orca's global `resolution` is **re-adjudicated in ticket 105** as a separate gap (generation-time global simplification is a missing decision point; the host's emit-time per-role keys do not implement it) |
| `narrow_loop_length_threshold_mm`, `smaller_perimeter_line_width` | Orca's small-perimeter handling is speed-only (`small_perimeter_speed`), not width |
| `thin_wall_speed`, `bottom_surface_speed` | finer per-role speed split than Orca exposes |
| `bridge_line_width` | Orca derives bridge width from `bridge_flow` rather than a width key |
| `seam_candidate_angle_threshold_deg` | internal seam scoring parameter |
| `extra_perimeters` | legacy Slic3r name; Orca kept only `extra_perimeters_on_overhangs` (live) |
| `retract_mode` (`gcode`\|`firmware`) | no matching key in the reference snapshot |
| `support_density`, `support_layer_height_mm` | Orca expresses these as pattern spacing / a bool toggle, not equivalents |
| `tree_support_interface_spacing_mm` | tree-scoped variant of Orca's global `support_interface_spacing` |
| `skirt_brim_enabled` | composite enable; Orca gates skirt and brim separately |
| `prime_tower_speed`, `wipe_tower_speed` | Orca's nearest is `wipe_tower_max_purge_speed` (different semantic) |
| `thumbnail_path` | Orca uses `thumbnails` / `thumbnails_format` (outside the FFF sections) |
| `gcode_xy_decimals` | coordinate formatting; no reference counterpart |
| `bed_temperature_initial_layer_single` | Pinch collapses Orca's per-plate-type bed-temp family into one key |
| `apply_to_all` | fuzzy-skin scope flag; overlaps Orca's `fuzzy_skin` enum rather than renaming a key |
| `infill_density`, `infill_speed`, `infill_overlap` | **not renames — duplicates.** `sparse_infill_density`, `sparse_infill_speed` and `infill_wall_overlap` are *also* declared live under their Orca names |

## In-scope gap by section (415)

| count | Section / subsection |
|---:|---|
| 32 | Filament / Notes |
| 28 | Multimaterial / Prime tower |
| 20 | Extruder / Nozzle / Retraction |
| 19 | Quality / Walls and surfaces |
| 19 | Cooling / Notes |
| 17 | Quality / Seam |
| 17 | Filament / Bed temperature |
| 13 | Extruder / Nozzle / Extruder geometry / mapping |
| 12 | Support / Support |
| 11 | Speed / Acceleration |
| 10 | Strength / Infill pattern-specific |
| 10 | Multimaterial / Multimaterial advanced |
| 9 | Support / Tree supports |
| 10 | Quality / Precision |
| 9 | Quality / Bridging |
| 9 | Printer / Machine / Motion limits |
| 9 | Others / Special mode |
| 9 | Multimaterial / Flush options |
| 8 | Speed / Jerk (XY) |
| 7 | Strength / Infill |
| 7 | Strength / Advanced (Strength) |
| 7 | Printer / Machine / Print volume |
| 7 | Others / G-code output |
| 7 | Others / Fuzzy Skin |
| 7 | Extruder / Nozzle / Nozzle |
| 7 | Extruder / Nozzle / MMU Hardware |
| 6 | Strength / Top/bottom shells |
| 6 | Others / Brim |
| 6 | Multimaterial / Filament for Features |
| 6 | Extruder / Nozzle / Pressure advance |
| 5 | Support / Advanced (Support) |
| 5 | Quality / Layer height |
| 5 | Printer / Machine / Printer identity |
| 5 | Others / Skirt |
| 5 | Filament / Temperature (Nozzle) |
| 4 | Support / Interface |
| 4 | Printer / Machine / Timing |
| 4 | Printer / Machine / Power / recovery |
| 4 | Printer / Machine / Bed mesh |
| 4 | Multimaterial / Ooze prevention |
| 3 | Support / Support filament |
| 3 | Speed / Advanced (Speed) |
| 3 | Quality / Overhangs |
| 3 | Quality / Ironing |
| 3 | Printer / Machine / Resonance |
| 2 | Support / Support ironing |
| 2 | Support / Raft |
| 2 | Speed / Other layers speed |
| 1 | Speed / Initial layer speed |
| 1 | Quality / Wall generator — Arachne |
| 1 | Quality / Line width |
| 1 | Others / Post-processing Scripts |
| 1 | Calibration / Flow / Pressure advance calibration |

## In-scope keys, by section

### Calibration / Flow / Pressure advance calibration

- `calib_flowrate_topinfill_special_order`

### Cooling / Notes

- `activate_air_filtration`
- `activate_chamber_temp_control`
- `additional_cooling_fan_speed`
- `auxiliary_fan`
- `complete_print_exhaust_fan_speed`
- `dont_slow_down_outer_wall`
- `during_print_exhaust_fan_speed`
- `fan_cooling_layer_time`
- `fan_kickstart`
- `fan_speedup_overhangs`
- `fan_speedup_time`
- `full_fan_speed_layer`
- `internal_bridge_fan_speed`
- `ironing_fan_speed`
- `max_layer_height`
- `min_layer_height`
- `overhang_fan_threshold`
- `reduce_fan_stop_start_freq`
- `support_material_interface_fan_speed`

### Extruder / Nozzle / Extruder geometry / mapping

- `extruder_ams_count`
- `extruder_colour`
- `extruder_offset`
- `extruder_type`
- `extruder_variant_list`
- `filament_extruder_variant`
- `filament_self_index`
- `master_extruder_id`
- `physical_extruder_map`
- `print_extruder_id`
- `print_extruder_variant`
- `printer_extruder_id`
- `printer_extruder_variant`

### Extruder / Nozzle / MMU Hardware

- `cooling_tube_length`
- `cooling_tube_retraction`
- `extra_loading_move`
- `grab_length`
- `high_current_on_filament_swap`
- `parking_pos_retraction`
- `start_end_points`

### Extruder / Nozzle / Nozzle

- `default_nozzle_volume_type`
- `nozzle_height`
- `nozzle_hrc`
- `nozzle_type`
- `nozzle_volume`
- `nozzle_volume_type`
- `required_nozzle_HRC`

### Extruder / Nozzle / Pressure advance

- `adaptive_pressure_advance`
- `adaptive_pressure_advance_bridges`
- `adaptive_pressure_advance_model`
- `adaptive_pressure_advance_overhangs`
- `enable_pressure_advance`
- `pressure_advance`

### Extruder / Nozzle / Retraction

- `deretraction_speed`
- `long_retractions_when_cut`
- `long_retractions_when_ec`
- `retract_before_wipe`
- `retract_length_toolchange`
- `retract_lift_above`
- `retract_lift_below`
- `retract_lift_enforce`
- `retract_restart_extra`
- `retract_restart_extra_toolchange`
- `retract_when_changing_layer`
- `retraction_distances_when_cut`
- `retraction_distances_when_ec`
- `retraction_minimum_travel`
- `travel_slope`
- `use_firmware_retraction`
- `wipe`
- `wipe_distance`
- `z_hop_types`
- `z_offset`

### Filament / Bed temperature

- `bed_temperature_formula`
- `cool_plate_temp`
- `cool_plate_temp_initial_layer`
- `curr_bed_type`
- `default_bed_type`
- `eng_plate_temp`
- `eng_plate_temp_initial_layer`
- `hot_plate_temp`
- `hot_plate_temp_initial_layer`
- `supertack_plate_temp`
- `supertack_plate_temp_initial_layer`
- `support_chamber_temp_control`
- `support_multi_bed_types`
- `textured_cool_plate_temp`
- `textured_cool_plate_temp_initial_layer`
- `textured_plate_temp`
- `textured_plate_temp_initial_layer`

### Filament / Notes

- `filament_adaptive_volumetric_speed`
- `filament_change_length`
- `filament_cooling_final_speed`
- `filament_cooling_initial_speed`
- `filament_cooling_moves`
- `filament_cost`
- `filament_density`
- `filament_diameter`
- `filament_flow_ratio`
- `filament_ironing_flow`
- `filament_ironing_inset`
- `filament_ironing_spacing`
- `filament_is_support`
- `filament_loading_speed`
- `filament_loading_speed_start`
- `filament_max_volumetric_speed`
- `filament_minimal_purge_on_wipe_tower`
- `filament_multitool_ramming`
- `filament_multitool_ramming_flow`
- `filament_multitool_ramming_volume`
- `filament_ramming_parameters`
- `filament_shrink`
- `filament_shrinkage_compensation_z`
- `filament_soluble`
- `filament_stamping_distance`
- `filament_stamping_loading_speed`
- `filament_toolchange_delay`
- `filament_type`
- `filament_unloading_speed`
- `filament_unloading_speed_start`
- `temperature_vitrification`
- `volumetric_speed_coefficients`

### Filament / Temperature (Nozzle)

- `chamber_temperature`
- `idle_temperature`
- `nozzle_temperature`
- `nozzle_temperature_range_high`
- `nozzle_temperature_range_low`

### Multimaterial / Filament for Features

- `filament_map`
- `filament_map_mode`
- `solid_infill_filament`
- `sparse_infill_filament`
- `wall_filament`
- `wipe_tower_filament`

### Multimaterial / Flush options

- `filament_flush_temp`
- `filament_flush_volumetric_speed`
- `flush_into_infill`
- `flush_into_objects`
- `flush_into_support`
- `flush_multiplier`
- `flush_volumes_matrix`
- `flush_volumes_vector`
- `wiping_volumes_extruders`

### Multimaterial / Multimaterial advanced

- `interface_shells`
- `interlocking_beam`
- `interlocking_beam_layer_count`
- `interlocking_beam_width`
- `interlocking_boundary_avoidance`
- `interlocking_depth`
- `interlocking_orientation`
- `mmu_segmented_region_interlocking_depth`
- `mmu_segmented_region_max_width`
- `support_object_skip_flush`

### Multimaterial / Ooze prevention

- `ooze_prevention`
- `preheat_steps`
- `preheat_time`
- `standby_temperature_delta`

### Multimaterial / Prime tower

- `enable_filament_ramming`
- `enable_tower_interface_cooldown_during_tower`
- `enable_tower_interface_features`
- `filament_tower_interface_pre_extrusion_dist`
- `filament_tower_interface_pre_extrusion_length`
- `filament_tower_interface_print_temp`
- `filament_tower_interface_purge_volume`
- `filament_tower_ironing_area`
- `manual_filament_change`
- `prime_tower_brim_width`
- `prime_tower_enable_framework`
- `prime_tower_flat_ironing`
- `prime_tower_infill_gap`
- `prime_tower_skip_points`
- `purge_in_prime_tower`
- `single_extruder_multi_material`
- `single_extruder_multi_material_priming`
- `wipe_tower_bridging`
- `wipe_tower_cone_angle`
- `wipe_tower_extra_flow`
- `wipe_tower_extra_rib_length`
- `wipe_tower_extra_spacing`
- `wipe_tower_fillet_wall`
- `wipe_tower_max_purge_speed`
- `wipe_tower_no_sparse_layers`
- `wipe_tower_rib_width`
- `wipe_tower_rotation_angle`
- `wipe_tower_wall_type`

### Others / Brim

- `brim_ears`
- `brim_ears_detection_length`
- `brim_ears_max_angle`
- `brim_object_gap`
- `brim_type`
- `brim_use_efc_outline`

### Others / Fuzzy Skin

- `fuzzy_skin`
- `fuzzy_skin_first_layer`
- `fuzzy_skin_mode`
- `fuzzy_skin_noise_type`
- `fuzzy_skin_octaves`
- `fuzzy_skin_persistence`
- `fuzzy_skin_scale`

### Others / G-code output

- `exclude_object`
- `filename_format`
- `gcode_add_line_number`
- `gcode_comments`
- `gcode_flavor`
- `gcode_label_objects`
- `reduce_infill_retraction`

### Others / Post-processing Scripts

- `post_process`

### Others / Skirt

- `draft_shield`
- `min_skirt_length`
- `single_loop_draft_shield`
- `skirt_start_angle`
- `skirt_type`

### Others / Special mode

- `enable_timelapse`
- `print_sequence`
- `slicing_mode`
- `spiral_finishing_flow_ratio`
- `spiral_mode`
- `spiral_mode_max_xy_smoothing`
- `spiral_mode_smooth`
- `spiral_starting_flow_ratio`
- `timelapse_type`

### Printer / Machine / Bed mesh

- `adaptive_bed_mesh_margin`
- `bed_mesh_max`
- `bed_mesh_min`
- `bed_mesh_probe_distance`

### Printer / Machine / Motion limits

- `machine_max_acceleration_extruding`
- `machine_max_acceleration_retracting`
- `machine_max_acceleration_travel`
- `machine_max_acceleration_x/y/z/e`
- `machine_max_jerk_x/y/z/e`
- `machine_max_junction_deviation`
- `machine_max_speed_x/y/z/e`
- `machine_min_extruding_rate`
- `machine_min_travel_rate`

### Printer / Machine / Power / recovery

- `disable_m73`
- `emit_machine_limits_to_gcode`
- `enable_power_loss_recovery`
- `silent_mode`

### Printer / Machine / Print volume

- `bed_exclude_area`
- `extruder_clearance_height_to_lid`
- `extruder_clearance_height_to_rod`
- `extruder_clearance_radius`
- `extruder_printable_area`
- `extruder_printable_height`
- `printable_height`

### Printer / Machine / Printer identity

- `allow_mix_temp`
- `printer_model`
- `printer_structure`
- `printer_technology`
- `printer_variant`

### Printer / Machine / Resonance

- `max_resonance_avoidance_speed`
- `min_resonance_avoidance_speed`
- `resonance_avoidance`

### Printer / Machine / Timing

- `machine_load_filament_time`
- `machine_tool_change_time`
- `machine_unload_filament_time`
- `time_cost`

### Quality / Bridging

- `bridge_angle`
- `bridge_density`
- `counterbore_hole_bridging`
- `dont_filter_internal_bridges`
- `enable_extra_bridge_layer`
- `internal_bridge_angle`
- `internal_bridge_density`
- `internal_bridge_flow`
- `thick_internal_bridges`

### Quality / Ironing

- `ironing_angle`
- `ironing_angle_fixed`
- `ironing_inset`

### Quality / Layer height

- `first_layer_print_sequence`
- `first_layer_sequence_choice`
- `other_layers_print_sequence`
- `other_layers_print_sequence_nums`
- `other_layers_sequence_choice`

### Quality / Line width

- `support_line_width`

### Quality / Overhangs

- `make_overhang_printable`
- `make_overhang_printable_angle`
- `make_overhang_printable_hole_size`

### Quality / Precision

- `elefant_foot_compensation`
- `elefant_foot_compensation_layers`
- `enable_arc_fitting`
- `hole_to_polyhole`
- `hole_to_polyhole_threshold`
- `hole_to_polyhole_twisted`
- `precise_z_height`
- `resolution`
- `xy_contour_compensation`
- `xy_hole_compensation`

### Quality / Seam

- `has_scarf_joint_seam`
- `role_based_wipe_speed`
- `scarf_angle_threshold`
- `scarf_joint_flow_ratio`
- `scarf_joint_speed`
- `scarf_overhang_threshold`
- `seam_gap`
- `seam_slope_conditional`
- `seam_slope_entire_loop`
- `seam_slope_inner_walls`
- `seam_slope_min_length`
- `seam_slope_start_height`
- `seam_slope_steps`
- `seam_slope_type`
- `staggered_inner_seams`
- `wipe_before_external_loop`
- `wipe_on_loops`

### Quality / Wall generator — Arachne

- `min_feature_size`

### Quality / Walls and surfaces

- `bottom_solid_infill_flow_ratio`
- `extruder`
- `first_layer_flow_ratio`
- `gap_fill_flow_ratio`
- `inner_wall_flow_ratio`
- `internal_solid_infill_flow_ratio`
- `is_infill_first`
- `max_travel_detour_distance`
- `outer_wall_flow_ratio`
- `overhang_flow_ratio`
- `print_flow_ratio`
- `reduce_crossing_wall`
- `set_other_flow_ratios`
- `small_area_infill_flow_compensation`
- `small_area_infill_flow_compensation_model`
- `sparse_infill_flow_ratio`
- `support_flow_ratio`
- `support_interface_flow_ratio`
- `top_solid_infill_flow_ratio`

### Speed / Acceleration

- `accel_to_decel_enable`
- `accel_to_decel_factor`
- `bridge_acceleration`
- `default_acceleration`
- `initial_layer_acceleration`
- `inner_wall_acceleration`
- `internal_solid_infill_acceleration`
- `outer_wall_acceleration`
- `sparse_infill_acceleration`
- `top_surface_acceleration`
- `travel_acceleration`

### Speed / Advanced (Speed)

- `extrusion_rate_smoothing_external_perimeter_only`
- `max_volumetric_extrusion_rate_slope`
- `max_volumetric_extrusion_rate_slope_segment_length`

### Speed / Initial layer speed

- `slow_down_layers`

### Speed / Jerk (XY)

- `default_jerk`
- `default_junction_deviation`
- `infill_jerk`
- `initial_layer_jerk`
- `inner_wall_jerk`
- `outer_wall_jerk`
- `top_surface_jerk`
- `travel_jerk`

### Speed / Other layers speed

- `internal_solid_infill_speed`
- `small_perimeter_speed`

### Strength / Advanced (Strength)

- `align_infill_direction_to_model`
- `detect_narrow_internal_solid_infill`
- `ensure_vertical_shell_thickness`
- `extra_solid_infills`
- `infill_combination`
- `infill_combination_max_layer_height`
- `minimum_sparse_infill_area`

### Strength / Infill pattern-specific

- `infill_lock_depth`
- `infill_overhang_angle`
- `lateral_lattice_angle_1`
- `lateral_lattice_angle_2`
- `skeleton_infill_density`
- `skeleton_infill_line_width`
- `skin_infill_density`
- `skin_infill_depth`
- `skin_infill_line_width`
- `symmetric_infill_y_axis`

### Strength / Infill

- `fill_multiline`
- `gap_fill_target`
- `internal_solid_infill_pattern`
- `solid_infill_direction`
- `solid_infill_rotate_template`
- `sparse_infill_pattern`
- `sparse_infill_rotate_template`

### Strength / Top/bottom shells

- `bottom_shell_thickness`
- `bottom_surface_density`
- `bottom_surface_pattern`
- `top_shell_thickness`
- `top_surface_density`
- `top_surface_pattern`

### Support / Advanced (Support)

- `bridge_no_support`
- `independent_support_layer_height`
- `max_bridge_length`
- `support_base_pattern`
- `support_base_pattern_spacing`

### Support / Interface

- `support_bottom_interface_spacing`
- `support_interface_loop_pattern`
- `support_interface_pattern`
- `support_interface_spacing`

### Support / Raft

- `raft_contact_distance`
- `raft_expansion`

### Support / Support filament

- `support_filament`
- `support_interface_filament`
- `support_interface_not_for_body`

### Support / Support ironing

- `support_air_filtration`
- `support_ironing_pattern`

### Support / Support

- `enforce_support_layers`
- `raft_first_layer_expansion`
- `support_bottom_z_distance`
- `support_critical_regions_only`
- `support_expansion`
- `support_object_first_layer_gap`
- `support_object_xy_distance`
- `support_remove_small_overhang`
- `support_style`
- `support_threshold_angle`
- `support_threshold_overlap`
- `support_type`

### Support / Tree supports

- `tree_support_angle_slow`
- `tree_support_auto_brim`
- `tree_support_branch_angle_organic`
- `tree_support_branch_diameter_organic`
- `tree_support_branch_distance_organic`
- `tree_support_brim_width`
- `tree_support_tip_diameter`
- `tree_support_top_rate`
- `tree_support_with_infill`

---

## Update after 07 (standardise-to-Orca ruling)

Ticket 07 resolved how the rename layer is treated — **standardise, don't
document**. Consequences for this asset:

- **The 25-table's three shape-change rows are reclassified.** `ironing_type`
  (narrowed) and `support_ironing` (narrowed) are genuine gaps — both modules
  declare the same shared `ironing_enabled` bool, so Orca's enum modes are
  unexpressible and the two Orca features can't be toggled independently.
  Moved into the queue: P14 +`ironing_type` (Tier B), P15 +`support_ironing`
  (Tier A). `raft_layers` (split) is **not** a gap — Orca derives its
  base/interface split internally from one count (`Slicing.cpp:194-196`);
  Pinch's three keys are a strict superset. Documented divergence, no rename.
- **One rename 03's table missed:** `ironing_spacing_mm` →
  `ironing_spacing` (top-surface-ironing) — the "four spellings, one concept"
  line from 07's original question. **Renamed in ticket 106.**
- **Mechanical rename scope = 26 keys**: the 22 exact/word-order/unit-suffix
  rows of the 25-table + the 3 duplicate collapses (the `infill_*` duplicates
  row above) + `ironing_spacing_mm`. Executed by workstream tickets 99–107;
  each ticket updates its own rows here with the new names. The 34
  Pinch-specific keys and the raft split stay untouched. The ❌ column
  retirement is out of scope (07 ruling).
