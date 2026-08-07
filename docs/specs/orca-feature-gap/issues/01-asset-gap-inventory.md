# Asset — Verified FFF gap inventory (ticket 01)

Generated 2026-08-07 from `docs/ORCA_CONFIG_REFERENCE.md` (sections **Quality** through **Printer / Machine**; SLA excluded) diffed against the live key registries. Reproduce with the commands in the ticket's Answer.

Columns: `col` = the hand-maintained ✅/❌ "In Codebase" column. `live` = exact key name found in a module `[config.schema]` manifest, `docs/config/host-keys.toml`, or a `get_*`/`ConfigKey::from` string literal. **`live=no` does not prove absence** — see the rename caveat in the ticket.

## Per-section summary

| absent | live | Section / subsection |
|---:|---:|---|
| 41 | 2 | Filament / Notes |
| 31 | 2 | Multimaterial / Prime tower |
| 25 | 0 | Printer / Machine / Printer identity |
| 23 | 4 | Cooling / Notes |
| 23 | 0 | Extruder / Nozzle / Retraction |
| 19 | 5 | Quality / Walls and surfaces |
| 18 | 1 | Quality / Seam |
| 17 | 0 | Filament / Bed temperature |
| 13 | 4 | Support / Support |
| 13 | 0 | Others / Special mode |
| 13 | 0 | Extruder / Nozzle / Extruder geometry / mapping |
| 12 | 0 | Printer / Machine / Print volume |
| 11 | 0 | Speed / Acceleration |
| 10 | 2 | Quality / Precision |
| 10 | 1 | Strength / Infill pattern-specific |
| 10 | 0 | Multimaterial / Multimaterial advanced |
| 10 | 0 | Multimaterial / Flush options |
| 9 | 5 | Support / Tree supports |
| 9 | 2 | Quality / Bridging |
| 9 | 0 | Printer / Machine / Motion limits |
| 9 | 0 | Others / Fuzzy Skin |
| 8 | 6 | Strength / Infill |
| 8 | 0 | Speed / Jerk (XY) |
| 7 | 1 | Others / G-code output |
| 7 | 1 | Extruder / Nozzle / Nozzle |
| 7 | 0 | Strength / Advanced (Strength) |
| 7 | 0 | Printer / Machine / Power / recovery |
| 7 | 0 | Extruder / Nozzle / MMU Hardware |
| 6 | 2 | Strength / Top/bottom shells |
| 6 | 1 | Quality / Layer height |
| 6 | 1 | Others / Brim |
| 6 | 0 | Multimaterial / Filament for Features |
| 6 | 0 | Extruder / Nozzle / Pressure advance |
| 5 | 4 | Others / Skirt |
| 5 | 1 | Filament / Temperature (Nozzle) |
| 5 | 0 | Support / Support ironing |
| 5 | 0 | Support / Advanced (Support) |
| 4 | 4 | Quality / Ironing |
| 4 | 2 | Support / Interface |
| 4 | 0 | Printer / Machine / Timing |
| 4 | 0 | Printer / Machine / Bed mesh |
| 4 | 0 | Multimaterial / Ooze prevention |
| 3 | 7 | Speed / Other layers speed |
| 3 | 5 | Quality / Overhangs |
| 3 | 0 | Support / Support filament |
| 3 | 0 | Support / Raft |
| 3 | 0 | Speed / Advanced (Speed) |
| 3 | 0 | Printer / Machine / Resonance |
| 1 | 7 | Quality / Wall generator — Arachne |
| 1 | 7 | Quality / Line width |
| 1 | 3 | Speed / Initial layer speed |
| 1 | 1 | Strength / Walls |
| 1 | 0 | Others / Post-processing Scripts |
| 1 | 0 | Others / Notes |
| 1 | 0 | Calibration / Flow / Pressure advance calibration |
| 0 | 8 | Speed / Overhang speed |
| 0 | 2 | Speed / Travel speed |
| 0 | 1 | Quality / Wall generator — Shared |
| 0 | 1 | Quality / Wall generator — Classic |

## Per-key inventory


### Calibration / Flow / Pressure advance calibration

| key | col | live |
|---|---|---|
| `calib_flowrate_topinfill_special_order` | ❌ | no |

### Cooling / Notes

| key | col | live |
|---|---|---|
| `activate_air_filtration` | ❌ | no |
| `activate_chamber_temp_control` | ❌ | no |
| `additional_cooling_fan_speed` | ❌ | no |
| `auxiliary_fan` | ❌ | no |
| `close_fan_the_first_x_layers` | ❌ | no |
| `complete_print_exhaust_fan_speed` | ❌ | no |
| `dont_slow_down_outer_wall` | ❌ | no |
| `during_print_exhaust_fan_speed` | ❌ | no |
| `enable_overhang_bridge_fan` | ❌ | no |
| `fan_cooling_layer_time` | ❌ | no |
| `fan_kickstart` | ❌ | no |
| `fan_max_speed` | ❌ | no |
| `fan_min_speed` | ❌ | no |
| `fan_speedup_overhangs` | ❌ | no |
| `fan_speedup_time` | ❌ | no |
| `full_fan_speed_layer` | ❌ | no |
| `internal_bridge_fan_speed` | ❌ | no |
| `ironing_fan_speed` | ❌ | no |
| `max_layer_height` | ❌ | no |
| `min_layer_height` | ❌ | no |
| `overhang_fan_speed` | ✅ | yes |
| `overhang_fan_threshold` | ❌ | no |
| `reduce_fan_stop_start_freq` | ❌ | no |
| `slow_down_for_layer_cooling` | ✅ | yes |
| `slow_down_layer_time` | ✅ | yes |
| `slow_down_min_speed` | ✅ | yes |
| `support_material_interface_fan_speed` | ❌ | no |

### Extruder / Nozzle / Extruder geometry / mapping

| key | col | live |
|---|---|---|
| `extruder_ams_count` | ❌ | no |
| `extruder_colour` | ❌ | no |
| `extruder_offset` | ❌ | no |
| `extruder_type` | ❌ | no |
| `extruder_variant_list` | ❌ | no |
| `filament_extruder_variant` | ❌ | no |
| `filament_self_index` | ❌ | no |
| `master_extruder_id` | ❌ | no |
| `physical_extruder_map` | ❌ | no |
| `print_extruder_id` | ❌ | no |
| `print_extruder_variant` | ❌ | no |
| `printer_extruder_id` | ❌ | no |
| `printer_extruder_variant` | ❌ | no |

### Extruder / Nozzle / MMU Hardware

| key | col | live |
|---|---|---|
| `cooling_tube_length` | ❌ | no |
| `cooling_tube_retraction` | ❌ | no |
| `extra_loading_move` | ❌ | no |
| `grab_length` | ❌ | no |
| `high_current_on_filament_swap` | ❌ | no |
| `parking_pos_retraction` | ❌ | no |
| `start_end_points` | ❌ | no |

### Extruder / Nozzle / Nozzle

| key | col | live |
|---|---|---|
| `default_nozzle_volume_type` | ❌ | no |
| `nozzle_diameter` | ❌ | yes |
| `nozzle_height` | ❌ | no |
| `nozzle_hrc` | ❌ | no |
| `nozzle_type` | ❌ | no |
| `nozzle_volume` | ❌ | no |
| `nozzle_volume_type` | ❌ | no |
| `required_nozzle_HRC` | ❌ | no |

### Extruder / Nozzle / Pressure advance

| key | col | live |
|---|---|---|
| `adaptive_pressure_advance` | ❌ | no |
| `adaptive_pressure_advance_bridges` | ❌ | no |
| `adaptive_pressure_advance_model` | ❌ | no |
| `adaptive_pressure_advance_overhangs` | ❌ | no |
| `enable_pressure_advance` | ❌ | no |
| `pressure_advance` | ❌ | no |

### Extruder / Nozzle / Retraction

| key | col | live |
|---|---|---|
| `deretraction_speed` | ❌ | no |
| `long_retractions_when_cut` | ❌ | no |
| `long_retractions_when_ec` | ❌ | no |
| `retract_before_wipe` | ❌ | no |
| `retract_length_toolchange` | ❌ | no |
| `retract_lift_above` | ❌ | no |
| `retract_lift_below` | ❌ | no |
| `retract_lift_enforce` | ❌ | no |
| `retract_restart_extra` | ❌ | no |
| `retract_restart_extra_toolchange` | ❌ | no |
| `retract_when_changing_layer` | ❌ | no |
| `retraction_distances_when_cut` | ❌ | no |
| `retraction_distances_when_ec` | ❌ | no |
| `retraction_length` | ❌ | no |
| `retraction_minimum_travel` | ❌ | no |
| `retraction_speed` | ❌ | no |
| `travel_slope` | ❌ | no |
| `use_firmware_retraction` | ❌ | no |
| `wipe` | ✅ | no |
| `wipe_distance` | ❌ | no |
| `z_hop` | ❌ | no |
| `z_hop_types` | ❌ | no |
| `z_offset` | ❌ | no |

### Filament / Bed temperature

| key | col | live |
|---|---|---|
| `bed_temperature_formula` | ❌ | no |
| `cool_plate_temp` | ❌ | no |
| `cool_plate_temp_initial_layer` | ❌ | no |
| `curr_bed_type` | ❌ | no |
| `default_bed_type` | ❌ | no |
| `eng_plate_temp` | ❌ | no |
| `eng_plate_temp_initial_layer` | ❌ | no |
| `hot_plate_temp` | ❌ | no |
| `hot_plate_temp_initial_layer` | ❌ | no |
| `supertack_plate_temp` | ❌ | no |
| `supertack_plate_temp_initial_layer` | ❌ | no |
| `support_chamber_temp_control` | ❌ | no |
| `support_multi_bed_types` | ❌ | no |
| `textured_cool_plate_temp` | ❌ | no |
| `textured_cool_plate_temp_initial_layer` | ❌ | no |
| `textured_plate_temp` | ❌ | no |
| `textured_plate_temp_initial_layer` | ❌ | no |

### Filament / Notes

| key | col | live |
|---|---|---|
| `default_filament_colour` | ❌ | no |
| `filament_adaptive_volumetric_speed` | ❌ | no |
| `filament_adhesiveness_category` | ❌ | no |
| `filament_change_length` | ❌ | no |
| `filament_colour` | ❌ | yes |
| `filament_colour_type` | ❌ | no |
| `filament_cooling_final_speed` | ❌ | no |
| `filament_cooling_initial_speed` | ❌ | no |
| `filament_cooling_moves` | ❌ | no |
| `filament_cost` | ❌ | no |
| `filament_density` | ❌ | no |
| `filament_diameter` | ❌ | no |
| `filament_flow_ratio` | ❌ | no |
| `filament_ids` | ❌ | no |
| `filament_ironing_flow` | ❌ | no |
| `filament_ironing_inset` | ❌ | no |
| `filament_ironing_spacing` | ❌ | no |
| `filament_ironing_speed` | ❌ | yes |
| `filament_is_support` | ❌ | no |
| `filament_loading_speed` | ❌ | no |
| `filament_loading_speed_start` | ❌ | no |
| `filament_max_volumetric_speed` | ❌ | no |
| `filament_minimal_purge_on_wipe_tower` | ❌ | no |
| `filament_multi_colour` | ❌ | no |
| `filament_multitool_ramming` | ❌ | no |
| `filament_multitool_ramming_flow` | ❌ | no |
| `filament_multitool_ramming_volume` | ❌ | no |
| `filament_notes` | ❌ | no |
| `filament_printable` | ❌ | no |
| `filament_ramming_parameters` | ❌ | no |
| `filament_settings_id` | ❌ | no |
| `filament_shrink` | ❌ | no |
| `filament_shrinkage_compensation_z` | ❌ | no |
| `filament_soluble` | ❌ | no |
| `filament_stamping_distance` | ❌ | no |
| `filament_stamping_loading_speed` | ❌ | no |
| `filament_toolchange_delay` | ❌ | no |
| `filament_type` | ❌ | no |
| `filament_unloading_speed` | ❌ | no |
| `filament_unloading_speed_start` | ❌ | no |
| `filament_vendor` | ❌ | no |
| `temperature_vitrification` | ❌ | no |
| `volumetric_speed_coefficients` | ❌ | no |

### Filament / Temperature (Nozzle)

| key | col | live |
|---|---|---|
| `chamber_temperature` | ❌ | no |
| `idle_temperature` | ❌ | no |
| `nozzle_temperature` | ❌ | no |
| `nozzle_temperature_initial_layer` | ✅ | yes |
| `nozzle_temperature_range_high` | ❌ | no |
| `nozzle_temperature_range_low` | ❌ | no |

### Multimaterial / Filament for Features

| key | col | live |
|---|---|---|
| `filament_map` | ❌ | no |
| `filament_map_mode` | ❌ | no |
| `solid_infill_filament` | ❌ | no |
| `sparse_infill_filament` | ❌ | no |
| `wall_filament` | ❌ | no |
| `wipe_tower_filament` | ❌ | no |

### Multimaterial / Flush options

| key | col | live |
|---|---|---|
| `filament_flush_temp` | ❌ | no |
| `filament_flush_volumetric_speed` | ❌ | no |
| `flush_into_infill` | ❌ | no |
| `flush_into_objects` | ❌ | no |
| `flush_into_support` | ❌ | no |
| `flush_multiplier` | ❌ | no |
| `flush_volumes_matrix` | ❌ | no |
| `flush_volumes_vector` | ❌ | no |
| `nozzle_flush_dataset` | ❌ | no |
| `wiping_volumes_extruders` | ❌ | no |

### Multimaterial / Multimaterial advanced

| key | col | live |
|---|---|---|
| `interface_shells` | ❌ | no |
| `interlocking_beam` | ❌ | no |
| `interlocking_beam_layer_count` | ❌ | no |
| `interlocking_beam_width` | ❌ | no |
| `interlocking_boundary_avoidance` | ❌ | no |
| `interlocking_depth` | ❌ | no |
| `interlocking_orientation` | ❌ | no |
| `mmu_segmented_region_interlocking_depth` | ❌ | no |
| `mmu_segmented_region_max_width` | ❌ | no |
| `support_object_skip_flush` | ❌ | no |

### Multimaterial / Ooze prevention

| key | col | live |
|---|---|---|
| `ooze_prevention` | ❌ | no |
| `preheat_steps` | ❌ | no |
| `preheat_time` | ❌ | no |
| `standby_temperature_delta` | ❌ | no |

### Multimaterial / Prime tower

| key | col | live |
|---|---|---|
| `enable_filament_ramming` | ❌ | no |
| `enable_prime_tower` | ❌ | no |
| `enable_tower_interface_cooldown_during_tower` | ❌ | no |
| `enable_tower_interface_features` | ❌ | no |
| `filament_tower_interface_pre_extrusion_dist` | ❌ | no |
| `filament_tower_interface_pre_extrusion_length` | ❌ | no |
| `filament_tower_interface_print_temp` | ❌ | no |
| `filament_tower_interface_purge_volume` | ❌ | no |
| `filament_tower_ironing_area` | ❌ | no |
| `manual_filament_change` | ❌ | no |
| `prime_tower_brim_width` | ❌ | no |
| `prime_tower_enable_framework` | ❌ | no |
| `prime_tower_flat_ironing` | ❌ | no |
| `prime_tower_infill_gap` | ❌ | no |
| `prime_tower_skip_points` | ❌ | no |
| `prime_tower_width` | ❌ | no |
| `prime_volume` | ❌ | no |
| `purge_in_prime_tower` | ❌ | no |
| `single_extruder_multi_material` | ❌ | no |
| `single_extruder_multi_material_priming` | ❌ | no |
| `wipe_tower_bridging` | ❌ | no |
| `wipe_tower_cone_angle` | ❌ | no |
| `wipe_tower_extra_flow` | ❌ | no |
| `wipe_tower_extra_rib_length` | ❌ | no |
| `wipe_tower_extra_spacing` | ❌ | no |
| `wipe_tower_fillet_wall` | ❌ | no |
| `wipe_tower_max_purge_speed` | ❌ | no |
| `wipe_tower_no_sparse_layers` | ❌ | no |
| `wipe_tower_rib_width` | ❌ | no |
| `wipe_tower_rotation_angle` | ❌ | no |
| `wipe_tower_wall_type` | ❌ | no |
| `wipe_tower_x` | ✅ | yes |
| `wipe_tower_y` | ✅ | yes |

### Others / Brim

| key | col | live |
|---|---|---|
| `brim_ears` | ❌ | no |
| `brim_ears_detection_length` | ❌ | no |
| `brim_ears_max_angle` | ❌ | no |
| `brim_object_gap` | ❌ | no |
| `brim_type` | ❌ | no |
| `brim_use_efc_outline` | ❌ | no |
| `brim_width` | ✅ | yes |

### Others / Fuzzy Skin

| key | col | live |
|---|---|---|
| `fuzzy_skin` | ✅ | no |
| `fuzzy_skin_first_layer` | ❌ | no |
| `fuzzy_skin_mode` | ❌ | no |
| `fuzzy_skin_noise_type` | ❌ | no |
| `fuzzy_skin_octaves` | ❌ | no |
| `fuzzy_skin_persistence` | ❌ | no |
| `fuzzy_skin_point_distance` | ❌ | no |
| `fuzzy_skin_scale` | ❌ | no |
| `fuzzy_skin_thickness` | ❌ | no |

### Others / G-code output

| key | col | live |
|---|---|---|
| `exclude_object` | ❌ | no |
| `filename_format` | ❌ | no |
| `gcode_add_line_number` | ❌ | no |
| `gcode_comments` | ❌ | no |
| `gcode_flavor` | ❌ | no |
| `gcode_label_objects` | ❌ | no |
| `reduce_infill_retraction` | ❌ | no |
| `use_relative_e_distances` | ✅ | yes |

### Others / Notes

| key | col | live |
|---|---|---|
| `notes` | ❌ | no |

### Others / Post-processing Scripts

| key | col | live |
|---|---|---|
| `post_process` | ❌ | no |

### Others / Skirt

| key | col | live |
|---|---|---|
| `draft_shield` | ❌ | no |
| `min_skirt_length` | ❌ | no |
| `single_loop_draft_shield` | ❌ | no |
| `skirt_distance` | ✅ | yes |
| `skirt_height` | ✅ | yes |
| `skirt_loops` | ✅ | yes |
| `skirt_speed` | ❌ | yes |
| `skirt_start_angle` | ❌ | no |
| `skirt_type` | ❌ | no |

### Others / Special mode

| key | col | live |
|---|---|---|
| `enable_timelapse` | ❌ | no |
| `enable_wrapping_detection` | ❌ | no |
| `print_order` | ❌ | no |
| `print_sequence` | ❌ | no |
| `slicing_mode` | ❌ | no |
| `spiral_finishing_flow_ratio` | ❌ | no |
| `spiral_mode` | ❌ | no |
| `spiral_mode_max_xy_smoothing` | ❌ | no |
| `spiral_mode_smooth` | ❌ | no |
| `spiral_starting_flow_ratio` | ❌ | no |
| `timelapse_type` | ❌ | no |
| `wrapping_detection_layers` | ❌ | no |
| `wrapping_exclude_area` | ❌ | no |

### Printer / Machine / Bed mesh

| key | col | live |
|---|---|---|
| `adaptive_bed_mesh_margin` | ❌ | no |
| `bed_mesh_max` | ❌ | no |
| `bed_mesh_min` | ❌ | no |
| `bed_mesh_probe_distance` | ❌ | no |

### Printer / Machine / Motion limits

| key | col | live |
|---|---|---|
| `machine_max_acceleration_extruding` | ❌ | no |
| `machine_max_acceleration_retracting` | ❌ | no |
| `machine_max_acceleration_travel` | ❌ | no |
| `machine_max_acceleration_x/y/z/e` | ❌ | no |
| `machine_max_jerk_x/y/z/e` | ❌ | no |
| `machine_max_junction_deviation` | ❌ | no |
| `machine_max_speed_x/y/z/e` | ❌ | no |
| `machine_min_extruding_rate` | ❌ | no |
| `machine_min_travel_rate` | ❌ | no |

### Printer / Machine / Power / recovery

| key | col | live |
|---|---|---|
| `bbl_calib_mark_logo` | ❌ | no |
| `disable_m73` | ✅ | no |
| `emit_machine_limits_to_gcode` | ❌ | no |
| `enable_power_loss_recovery` | ❌ | no |
| `head_wrap_detect_zone` | ❌ | no |
| `scan_first_layer` | ❌ | no |
| `silent_mode` | ❌ | no |

### Printer / Machine / Print volume

| key | col | live |
|---|---|---|
| `bed_custom_model` | ❌ | no |
| `bed_custom_texture` | ❌ | no |
| `bed_exclude_area` | ❌ | no |
| `best_object_pos` | ❌ | no |
| `extruder_clearance_height_to_lid` | ❌ | no |
| `extruder_clearance_height_to_rod` | ❌ | no |
| `extruder_clearance_radius` | ❌ | no |
| `extruder_printable_area` | ❌ | no |
| `extruder_printable_height` | ❌ | no |
| `preferred_orientation` | ❌ | no |
| `printable_area` | ❌ | no |
| `printable_height` | ❌ | no |

### Printer / Machine / Printer identity

| key | col | live |
|---|---|---|
| `allow_mix_temp` | ❌ | no |
| `allow_multicolor_oneplate` | ❌ | no |
| `bbl_use_printhost` | ❌ | no |
| `default_filament_profile` | ❌ | no |
| `default_print_profile` | ❌ | no |
| `host_type` | ❌ | no |
| `pellet_flow_coefficient` | ❌ | no |
| `pellet_modded_printer` | ❌ | no |
| `print_host` | ❌ | no |
| `print_host_webui` | ❌ | no |
| `printer_agent` | ❌ | no |
| `printer_model` | ❌ | no |
| `printer_notes` | ❌ | no |
| `printer_settings_id` | ❌ | no |
| `printer_structure` | ❌ | no |
| `printer_technology` | ❌ | no |
| `printer_variant` | ❌ | no |
| `printhost_apikey` | ❌ | no |
| `printhost_authorization_type` | ❌ | no |
| `printhost_cafile` | ❌ | no |
| `printhost_password` | ❌ | no |
| `printhost_port` | ❌ | no |
| `printhost_ssl_ignore_revoke` | ❌ | no |
| `printhost_user` | ❌ | no |
| `upward_compatible_machine` | ❌ | no |

### Printer / Machine / Resonance

| key | col | live |
|---|---|---|
| `max_resonance_avoidance_speed` | ❌ | no |
| `min_resonance_avoidance_speed` | ❌ | no |
| `resonance_avoidance` | ❌ | no |

### Printer / Machine / Timing

| key | col | live |
|---|---|---|
| `machine_load_filament_time` | ❌ | no |
| `machine_tool_change_time` | ❌ | no |
| `machine_unload_filament_time` | ❌ | no |
| `time_cost` | ❌ | no |

### Quality / Bridging

| key | col | live |
|---|---|---|
| `bridge_angle` | ❌ | no |
| `bridge_density` | ❌ | no |
| `bridge_flow` | ❌ | yes |
| `counterbore_hole_bridging` | ❌ | no |
| `dont_filter_internal_bridges` | ❌ | no |
| `enable_extra_bridge_layer` | ❌ | no |
| `internal_bridge_angle` | ❌ | no |
| `internal_bridge_density` | ❌ | no |
| `internal_bridge_flow` | ❌ | no |
| `thick_bridges` | ❌ | yes |
| `thick_internal_bridges` | ❌ | no |

### Quality / Ironing

| key | col | live |
|---|---|---|
| `ironing_angle` | ❌ | no |
| `ironing_angle_fixed` | ❌ | no |
| `ironing_flow` | ✅ | yes |
| `ironing_inset` | ❌ | no |
| `ironing_pattern` | ✅ | yes |
| `ironing_spacing` | ✅ | yes |
| `ironing_speed` | ✅ | yes |
| `ironing_type` | ❌ | no |

### Quality / Layer height

| key | col | live |
|---|---|---|
| `first_layer_print_sequence` | ❌ | no |
| `first_layer_sequence_choice` | ❌ | no |
| `initial_layer_print_height` | ✅ | no |
| `layer_height` | ✅ | yes |
| `other_layers_print_sequence` | ❌ | no |
| `other_layers_print_sequence_nums` | ❌ | no |
| `other_layers_sequence_choice` | ❌ | no |

### Quality / Line width

| key | col | live |
|---|---|---|
| `initial_layer_line_width` | ❌ | yes |
| `inner_wall_line_width` | ❌ | yes |
| `internal_solid_infill_line_width` | ❌ | yes |
| `line_width` | ✅ | yes |
| `outer_wall_line_width` | ❌ | yes |
| `sparse_infill_line_width` | ❌ | yes |
| `support_line_width` | ❌ | no |
| `top_surface_line_width` | ❌ | yes |

### Quality / Overhangs

| key | col | live |
|---|---|---|
| `detect_overhang_wall` | ❌ | yes |
| `extra_perimeters_on_overhangs` | ❌ | yes |
| `make_overhang_printable` | ❌ | no |
| `make_overhang_printable_angle` | ❌ | no |
| `make_overhang_printable_hole_size` | ❌ | no |
| `overhang_reverse` | ❌ | yes |
| `overhang_reverse_internal_only` | ❌ | yes |
| `overhang_reverse_threshold` | ❌ | yes |

### Quality / Precision

| key | col | live |
|---|---|---|
| `elefant_foot_compensation` | ❌ | no |
| `elefant_foot_compensation_layers` | ❌ | no |
| `enable_arc_fitting` | ❌ | no |
| `hole_to_polyhole` | ❌ | no |
| `hole_to_polyhole_threshold` | ❌ | no |
| `hole_to_polyhole_twisted` | ❌ | no |
| `precise_outer_wall` | ❌ | yes |
| `precise_z_height` | ❌ | no |
| `resolution` | ❌ | no |
| `slice_closing_radius` | ✅ | yes |
| `xy_contour_compensation` | ❌ | no |
| `xy_hole_compensation` | ❌ | no |

### Quality / Seam

| key | col | live |
|---|---|---|
| `has_scarf_joint_seam` | ❌ | no |
| `role_based_wipe_speed` | ❌ | no |
| `scarf_angle_threshold` | ❌ | no |
| `scarf_joint_flow_ratio` | ❌ | no |
| `scarf_joint_speed` | ❌ | no |
| `scarf_overhang_threshold` | ❌ | no |
| `seam_gap` | ❌ | no |
| `seam_position` | ❌ | no |
| `seam_slope_conditional` | ❌ | no |
| `seam_slope_entire_loop` | ❌ | no |
| `seam_slope_inner_walls` | ❌ | no |
| `seam_slope_min_length` | ❌ | no |
| `seam_slope_start_height` | ❌ | no |
| `seam_slope_steps` | ❌ | no |
| `seam_slope_type` | ❌ | no |
| `staggered_inner_seams` | ❌ | no |
| `wipe_before_external_loop` | ❌ | no |
| `wipe_on_loops` | ❌ | no |
| `wipe_speed` | ❌ | yes |

### Quality / Wall generator — Arachne

| key | col | live |
|---|---|---|
| `initial_layer_min_bead_width` | ❌ | yes |
| `min_bead_width` | ❌ | yes |
| `min_feature_size` | ❌ | no |
| `min_length_factor` | ❌ | yes |
| `wall_distribution_count` | ❌ | yes |
| `wall_transition_angle` | ❌ | yes |
| `wall_transition_filter_deviation` | ❌ | yes |
| `wall_transition_length` | ❌ | yes |

### Quality / Wall generator — Classic

| key | col | live |
|---|---|---|
| `detect_thin_wall` | ❌ | yes |

### Quality / Wall generator — Shared

| key | col | live |
|---|---|---|
| `wall_generator` | ✅ | yes |

### Quality / Walls and surfaces

| key | col | live |
|---|---|---|
| `bottom_solid_infill_flow_ratio` | ❌ | no |
| `extruder` | ✅ | no |
| `first_layer_flow_ratio` | ❌ | no |
| `gap_fill_flow_ratio` | ❌ | no |
| `inner_wall_flow_ratio` | ❌ | no |
| `internal_solid_infill_flow_ratio` | ❌ | no |
| `is_infill_first` | ❌ | no |
| `max_travel_detour_distance` | ❌ | no |
| `min_width_top_surface` | ❌ | yes |
| `only_one_wall_first_layer` | ❌ | yes |
| `only_one_wall_top` | ❌ | yes |
| `outer_wall_flow_ratio` | ❌ | no |
| `overhang_flow_ratio` | ❌ | no |
| `print_flow_ratio` | ❌ | no |
| `reduce_crossing_wall` | ❌ | no |
| `set_other_flow_ratios` | ❌ | no |
| `small_area_infill_flow_compensation` | ❌ | no |
| `small_area_infill_flow_compensation_model` | ❌ | no |
| `sparse_infill_flow_ratio` | ❌ | no |
| `support_flow_ratio` | ❌ | no |
| `support_interface_flow_ratio` | ❌ | no |
| `top_solid_infill_flow_ratio` | ❌ | no |
| `wall_direction` | ❌ | yes |
| `wall_sequence` | ❌ | yes |

### Speed / Acceleration

| key | col | live |
|---|---|---|
| `accel_to_decel_enable` | ❌ | no |
| `accel_to_decel_factor` | ❌ | no |
| `bridge_acceleration` | ❌ | no |
| `default_acceleration` | ❌ | no |
| `initial_layer_acceleration` | ❌ | no |
| `inner_wall_acceleration` | ❌ | no |
| `internal_solid_infill_acceleration` | ❌ | no |
| `outer_wall_acceleration` | ❌ | no |
| `sparse_infill_acceleration` | ❌ | no |
| `top_surface_acceleration` | ❌ | no |
| `travel_acceleration` | ❌ | no |

### Speed / Advanced (Speed)

| key | col | live |
|---|---|---|
| `extrusion_rate_smoothing_external_perimeter_only` | ❌ | no |
| `max_volumetric_extrusion_rate_slope` | ❌ | no |
| `max_volumetric_extrusion_rate_slope_segment_length` | ❌ | no |

### Speed / Initial layer speed

| key | col | live |
|---|---|---|
| `initial_layer_infill_speed` | ❌ | yes |
| `initial_layer_speed` | ❌ | yes |
| `initial_layer_travel_speed` | ❌ | yes |
| `slow_down_layers` | ❌ | no |

### Speed / Jerk (XY)

| key | col | live |
|---|---|---|
| `default_jerk` | ❌ | no |
| `default_junction_deviation` | ❌ | no |
| `infill_jerk` | ❌ | no |
| `initial_layer_jerk` | ❌ | no |
| `inner_wall_jerk` | ❌ | no |
| `outer_wall_jerk` | ❌ | no |
| `top_surface_jerk` | ❌ | no |
| `travel_jerk` | ❌ | no |

### Speed / Other layers speed

| key | col | live |
|---|---|---|
| `gap_infill_speed` | ❌ | yes |
| `inner_wall_speed` | ✅ | yes |
| `internal_solid_infill_speed` | ❌ | no |
| `outer_wall_speed` | ✅ | yes |
| `small_perimeter_speed` | ❌ | no |
| `small_perimeter_threshold` | ❌ | no |
| `sparse_infill_speed` | ❌ | yes |
| `support_interface_speed` | ❌ | yes |
| `support_speed` | ✅ | yes |
| `top_surface_speed` | ❌ | yes |

### Speed / Overhang speed

| key | col | live |
|---|---|---|
| `bridge_speed` | ❌ | yes |
| `enable_overhang_speed` | ❌ | yes |
| `internal_bridge_speed` | ❌ | yes |
| `overhang_1_4_speed` | ❌ | yes |
| `overhang_2_4_speed` | ❌ | yes |
| `overhang_3_4_speed` | ❌ | yes |
| `overhang_4_4_speed` | ❌ | yes |
| `slowdown_for_curled_perimeters` | ❌ | yes |

### Speed / Travel speed

| key | col | live |
|---|---|---|
| `travel_speed` | ❌ | yes |
| `travel_speed_z` | ❌ | yes |

### Strength / Advanced (Strength)

| key | col | live |
|---|---|---|
| `align_infill_direction_to_model` | ❌ | no |
| `detect_narrow_internal_solid_infill` | ❌ | no |
| `ensure_vertical_shell_thickness` | ❌ | no |
| `extra_solid_infills` | ❌ | no |
| `infill_combination` | ❌ | no |
| `infill_combination_max_layer_height` | ❌ | no |
| `minimum_sparse_infill_area` | ❌ | no |

### Strength / Infill pattern-specific

| key | col | live |
|---|---|---|
| `infill_lock_depth` | ❌ | no |
| `infill_overhang_angle` | ❌ | no |
| `infill_shift_step` | ❌ | yes |
| `lateral_lattice_angle_1` | ❌ | no |
| `lateral_lattice_angle_2` | ❌ | no |
| `skeleton_infill_density` | ❌ | no |
| `skeleton_infill_line_width` | ❌ | no |
| `skin_infill_density` | ❌ | no |
| `skin_infill_depth` | ❌ | no |
| `skin_infill_line_width` | ❌ | no |
| `symmetric_infill_y_axis` | ❌ | no |

### Strength / Infill

| key | col | live |
|---|---|---|
| `fill_multiline` | ❌ | no |
| `filter_out_gap_fill` | ❌ | yes |
| `gap_fill_target` | ❌ | no |
| `infill_anchor` | ✅ | yes |
| `infill_anchor_max` | ✅ | yes |
| `infill_direction` | ❌ | no |
| `infill_wall_overlap` | ❌ | yes |
| `internal_solid_infill_pattern` | ❌ | no |
| `solid_infill_direction` | ❌ | no |
| `solid_infill_rotate_template` | ❌ | no |
| `sparse_infill_density` | ✅ | yes |
| `sparse_infill_pattern` | ❌ | no |
| `sparse_infill_rotate_template` | ❌ | no |
| `top_bottom_infill_wall_overlap` | ❌ | yes |

### Strength / Top/bottom shells

| key | col | live |
|---|---|---|
| `bottom_shell_layers` | ✅ | yes |
| `bottom_shell_thickness` | ❌ | no |
| `bottom_surface_density` | ❌ | no |
| `bottom_surface_pattern` | ❌ | no |
| `top_shell_layers` | ✅ | yes |
| `top_shell_thickness` | ❌ | no |
| `top_surface_density` | ❌ | no |
| `top_surface_pattern` | ❌ | no |

### Strength / Walls

| key | col | live |
|---|---|---|
| `alternate_extra_wall` | ❌ | yes |
| `wall_loops` | ✅ | no |

### Support / Advanced (Support)

| key | col | live |
|---|---|---|
| `bridge_no_support` | ❌ | no |
| `independent_support_layer_height` | ❌ | no |
| `max_bridge_length` | ❌ | no |
| `support_base_pattern` | ❌ | no |
| `support_base_pattern_spacing` | ❌ | no |

### Support / Interface

| key | col | live |
|---|---|---|
| `support_bottom_interface_spacing` | ❌ | no |
| `support_interface_bottom_layers` | ✅ | yes |
| `support_interface_loop_pattern` | ❌ | no |
| `support_interface_pattern` | ❌ | no |
| `support_interface_spacing` | ❌ | no |
| `support_interface_top_layers` | ✅ | yes |

### Support / Raft

| key | col | live |
|---|---|---|
| `raft_contact_distance` | ❌ | no |
| `raft_expansion` | ❌ | no |
| `raft_layers` | ❌ | no |

### Support / Support filament

| key | col | live |
|---|---|---|
| `support_filament` | ✅ | no |
| `support_interface_filament` | ✅ | no |
| `support_interface_not_for_body` | ❌ | no |

### Support / Support ironing

| key | col | live |
|---|---|---|
| `support_air_filtration` | ❌ | no |
| `support_ironing` | ❌ | no |
| `support_ironing_flow` | ❌ | no |
| `support_ironing_pattern` | ❌ | no |
| `support_ironing_spacing` | ❌ | no |

### Support / Support

| key | col | live |
|---|---|---|
| `enable_support` | ✅ | yes |
| `enforce_support_layers` | ❌ | no |
| `raft_first_layer_density` | ❌ | yes |
| `raft_first_layer_expansion` | ❌ | no |
| `support_angle` | ✅ | yes |
| `support_bottom_z_distance` | ❌ | no |
| `support_critical_regions_only` | ❌ | no |
| `support_expansion` | ❌ | no |
| `support_object_first_layer_gap` | ❌ | no |
| `support_object_xy_distance` | ❌ | no |
| `support_on_build_plate_only` | ❌ | yes |
| `support_remove_small_overhang` | ❌ | no |
| `support_style` | ❌ | no |
| `support_threshold_angle` | ❌ | no |
| `support_threshold_overlap` | ❌ | no |
| `support_top_z_distance` | ❌ | no |
| `support_type` | ✅ | no |

### Support / Tree supports

| key | col | live |
|---|---|---|
| `tree_support_angle_slow` | ❌ | no |
| `tree_support_auto_brim` | ❌ | no |
| `tree_support_branch_angle` | ❌ | yes |
| `tree_support_branch_angle_organic` | ❌ | no |
| `tree_support_branch_diameter` | ✅ | yes |
| `tree_support_branch_diameter_angle` | ✅ | yes |
| `tree_support_branch_diameter_organic` | ❌ | no |
| `tree_support_branch_distance` | ✅ | yes |
| `tree_support_branch_distance_organic` | ❌ | no |
| `tree_support_brim_width` | ❌ | no |
| `tree_support_tip_diameter` | ❌ | no |
| `tree_support_top_rate` | ❌ | no |
| `tree_support_wall_count` | ✅ | yes |
| `tree_support_with_infill` | ❌ | no |

---

## Appendix — declared Pinch keys with no exact OrcaSlicer name (62)

These are the **rename-adjudication pool**: every one is either an Orca feature implemented under a different name (so the Orca key is a false gap above), or a Pinch-specific key with no upstream counterpart. Ticket 03 owns the adjudication. Unadjudicated, they set the uncertainty band on the gap count.

- `apply_to_all`
- `base_raft_layers`
- `bed_shape`
- `bed_temperature_initial_layer_single`
- `bottom_fill_holder`
- `bottom_surface_speed`
- `bridge_fill_holder`
- `bridge_line_width`
- `disable_fan_first_layers`
- `enable_overhang_fan`
- `extra_perimeters`
- `fan_speed_max`
- `fan_speed_min`
- `filament_change_extrusion_role_gcode`
- `first_layer_height`
- `flat_bridge_closing_join`
- `gap_fill_medial_axis_on_painted`
- `gcode_resolution`
- `gcode_xy_decimals`
- `infill_angle`
- `infill_density`
- `infill_overlap`
- `infill_resolution`
- `infill_speed`
- `interface_raft_layers`
- `ironing_enabled`
- `ironing_flow_rate`
- `ironing_spacing_mm`
- `min_segment_length`
- `narrow_loop_length_threshold_mm`
- `path_optimization_emit_layer_markers`
- `perimeter_arc_tolerance`
- `point_distance`
- `prime_tower_speed`
- `process_change_extrusion_role_gcode`
- `retract_length`
- `retract_mode`
- `retract_speed`
- `seam_candidate_angle_threshold_deg`
- `seam_mode`
- `skirt_brim_enabled`
- `slice_has_paint`
- `smaller_perimeter_line_width`
- `smaller_perimeter_threshold_mm`
- `sparse_fill_holder`
- `spiral_vase`
- `support_density`
- `support_layer_height_mm`
- `support_raft_layers`
- `support_resolution`
- `support_top_z_distance_mm`
- `thickness`
- `thin_wall_speed`
- `thumbnail_path`
- `top_fill_holder`
- `travel_z_hop`
- `tree_support_interface_spacing_mm`
- `wall_count`
- `wipe_tower_enabled`
- `wipe_tower_purge_volume`
- `wipe_tower_speed`
- `wipe_tower_width`
