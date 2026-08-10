# Asset — Packet list for the FFF gap queue (ticket 05)

Derived from ticket 04's per-key tier assignment by applying the grouping rule
below. **91 packets, 356 keys** (Tier A 117, Tier B 224, Tier C 15 — plus 2
fog-blocked Tier A keys and 47 Tier D keys, deferred). Packet order is the
queue order: **tier-major (A, B, C), then owning module, then Orca UI section**.
(Amended by ticket 07: P14 +`ironing_type` [B], P15 +`support_ironing` [A] —
P14 becomes a mixed A/B packet, moving it to the B tier of the 20A/65B/6C
packet counts: now 19 A, 66 B, 6 C.)

## The grouping rule (all rulings confirmed with the human, ticket 05 session)

- **Primary axis: owning module.** A packet's keys all belong to one owner at
  one decision-point seam (04's verified owner map).
- **Secondary axis: Orca UI section.** The section names the theme; keys are
  listed in 04's per-key table order (Orca UI order).
- **Tier is a purity check, not an axis.** In practice the owner split already
  separates tiers almost everywhere (e.g. Prime tower's 26 A keys are all
  wipe-tower, its 2 B keys are emitter). Where a group does mix tiers, split by
  tier. Tier is also the queue-order key: all-A packets first, then B, then C —
  which is the destination's cheapest-first ordering.
- **Size ceilings:** A ≤ 25 keys, B ≤ 12 keys, C ≤ 4 keys. A group over its
  ceiling splits by sub-theme (semantic cluster), split points recorded below.
  Grounded in step size: A keys are S-steps (declare + wire), B keys are M-steps
  (new logic + tests), C keys are new modules (ADR + guest rebuild).
- **Tier C:** one feature = one module, split above 4 keys by feature cluster.
  interlocking (6 keys) splits into two packets: beam definition (3) and
  structure & boundary (3). The first interlocking packet authors the module
  scaffold + its ADR; the second conforms to it. No other C feature is split.
- **ADR:** only interlocking and mmu-segmented-region packets author an ADR
  (algorithm port with port-strategy/seam decisions; undecided seam with
  host-bridge risk per ADR-0033). elefant-foot, polyhole, contour-compensation
  are routine `pnp_cli module new` scaffolds with parity-dictated behavior —
  no ADR (same class as skirt-brim, part-cooling, top-surface-ironing, which
  have zero ADR refs). ADRs are authored inside the packet-authoring ticket,
  number re-derived from disk at authoring time.
- **No merging.** Every (owner, section) group is its own packet even at 1–3
  keys — theme and fine-grained ordering win; 2-key packets are normal
  precedent (packet 212). Merged owner-level "misc" packets would exceed the B
  ceiling immediately (an emitter misc alone would be 15 B keys).
- **Shared-owner keys** go to the owner with the decision point:
  `printable_height`/`extruder_printable_area`/`extruder_printable_height` →
  emitter; `bed_exclude_area` → wipe-tower; `timelapse_type` → wipe-tower
  (primary); `spiral_mode` → emitter (cross-cutting with print/orchestration —
  noted in the packet); ironing keys → top-surface-ironing (shared with
  support-surface-ironing).

## Split points (groups over their ceiling)

| Packet | keys | split |
|---|---|---|
| P02/P03 — Prime tower (A) | 26 | 13+13: interface/tower-feature keys, then purge/geometry keys |
| P36/P37 — Retraction (B) | 20 | 10+10: lift/toolchange/restart keys, then wipe/travel/cut keys |
| P52/P53 — Seam (B) | 16 | 8+8: scarf/slope keys, then slope-variant/wipe keys |
| P54/P55 — Walls and surfaces (B) | 17 | 9+9: flow-ratio keys, then compensation/travel keys |
| P89/P90 — interlocking (C) | 6 | 3+3: beam definition, then structure & boundary |

Split boundaries are proposals; the authoring ticket may adjust the boundary by
sub-theme without changing the packet count. `hole_to_polyhole_max_edges`
(flagged by 04) is still missing from the inventory and not in any packet.

## Deferred (not in this list)

- **2 fog-blocked Tier A keys** — `filament_density`, `filament_diameter`
  (declare-in-manifest work, blocked on the per-filament config model; graduate
  with Tier D).
- **47 Tier D keys** — per-filament config model fog (map "Not yet specified").

---

# Packet list (91)

## Tier A — 20 packets

### P01 — Cooling / Notes — part-cooling (17 keys, Tier A)

`activate_air_filtration`, `activate_chamber_temp_control`, `additional_cooling_fan_speed`, `auxiliary_fan`, `complete_print_exhaust_fan_speed`, `dont_slow_down_outer_wall`, `during_print_exhaust_fan_speed`, `fan_cooling_layer_time`, `fan_kickstart`, `fan_speedup_overhangs`, `fan_speedup_time`, `full_fan_speed_layer`, `internal_bridge_fan_speed`, `ironing_fan_speed`, `overhang_fan_threshold`, `reduce_fan_stop_start_freq`, `support_material_interface_fan_speed`

### P02 — Multimaterial / Prime tower (1/2) — wipe-tower (13 keys, Tier A)

`enable_filament_ramming`, `enable_tower_interface_cooldown_during_tower`, `enable_tower_interface_features`, `filament_tower_interface_pre_extrusion_dist`, `filament_tower_interface_pre_extrusion_length`, `filament_tower_interface_print_temp`, `filament_tower_interface_purge_volume`, `filament_tower_ironing_area`, `prime_tower_brim_width`, `prime_tower_enable_framework`, `prime_tower_flat_ironing`, `prime_tower_infill_gap`, `prime_tower_skip_points`

### P03 — Multimaterial / Prime tower (2/2) — wipe-tower (13 keys, Tier A)

`purge_in_prime_tower`, `single_extruder_multi_material`, `wipe_tower_bridging`, `wipe_tower_cone_angle`, `wipe_tower_extra_flow`, `wipe_tower_extra_rib_length`, `wipe_tower_extra_spacing`, `wipe_tower_fillet_wall`, `wipe_tower_max_purge_speed`, `wipe_tower_no_sparse_layers`, `wipe_tower_rib_width`, `wipe_tower_rotation_angle`, `wipe_tower_wall_type`

### P04 — Printer / Machine / Print volume — wipe-tower (1 keys, Tier A)

`bed_exclude_area`

### P05 — Others / Brim — skirt-brim (6 keys, Tier A)

`brim_ears`, `brim_ears_detection_length`, `brim_ears_max_angle`, `brim_object_gap`, `brim_type`, `brim_use_efc_outline`

### P06 — Others / Skirt — skirt-brim (5 keys, Tier A)

`draft_shield`, `min_skirt_length`, `single_loop_draft_shield`, `skirt_start_angle`, `skirt_type`

### P07 — Others / Fuzzy Skin — fuzzy-skin (7 keys, Tier A)

`fuzzy_skin`, `fuzzy_skin_first_layer`, `fuzzy_skin_mode`, `fuzzy_skin_noise_type`, `fuzzy_skin_octaves`, `fuzzy_skin_persistence`, `fuzzy_skin_scale`

### P08 — Strength / Infill — infill modules (7 keys, Tier A)

`fill_multiline`, `gap_fill_target`, `internal_solid_infill_pattern`, `solid_infill_direction`, `solid_infill_rotate_template`, `sparse_infill_pattern`, `sparse_infill_rotate_template`

### P09 — Strength / Infill pattern-specific — infill modules (10 keys, Tier A)

`infill_lock_depth`, `infill_overhang_angle`, `lateral_lattice_angle_1`, `lateral_lattice_angle_2`, `skeleton_infill_density`, `skeleton_infill_line_width`, `skin_infill_density`, `skin_infill_depth`, `skin_infill_line_width`, `symmetric_infill_y_axis`

### P10 — Strength / Top/bottom shells — infill modules (4 keys, Tier A)

`bottom_surface_density`, `bottom_surface_pattern`, `top_surface_density`, `top_surface_pattern`

### P11 — Support / Interface — support-planner (4 keys, Tier A)

`support_bottom_interface_spacing`, `support_interface_loop_pattern`, `support_interface_pattern`, `support_interface_spacing`

### P12 — Support / Raft — support-planner (2 keys, Tier A)

`raft_contact_distance`, `raft_expansion`

### P13 — Support / Support — support-planner (12 keys, Tier A)

`enforce_support_layers`, `raft_first_layer_expansion`, `support_bottom_z_distance`, `support_critical_regions_only`, `support_expansion`, `support_object_first_layer_gap`, `support_object_xy_distance`, `support_remove_small_overhang`, `support_style`, `support_threshold_angle`, `support_threshold_overlap`, `support_type`

### P14 — Quality / Ironing — top-surface-ironing (4 keys — 3 Tier A plumbing + 1 Tier B logic)

`ironing_angle`, `ironing_angle_fixed`, `ironing_inset`, `ironing_type`

(Amended by ticket 07: `ironing_type` reclassified from "already implemented
(narrowed)" to a genuine gap — widens the shared `ironing_enabled` bool to
Orca's 4-mode enum.)

### P15 — Support / Support ironing — support-surface-ironing (2 keys, Tier A)

`support_ironing_pattern`, `support_ironing`

(Amended by ticket 07: `support_ironing` reclassified from "already
implemented (narrowed)" to a genuine gap — an independent bool so support
ironing no longer rides the shared `ironing_enabled`.)

### P16 — Quality / Wall generator — Arachne — arachne-perimeters (1 keys, Tier A)

`min_feature_size`

### P17 — Quality / Seam — seam-placer (1 keys, Tier A)

`staggered_inner_seams`

### P18 — Printer / Machine / Power / recovery — emitter (4 keys, Tier A)

`disable_m73`, `emit_machine_limits_to_gcode`, `enable_power_loss_recovery`, `silent_mode`

### P19 — Printer / Machine / Print volume — emitter (3 keys, Tier A)

`extruder_printable_area`, `extruder_printable_height`, `printable_height`

### P20 — Printer / Machine / Printer identity — emitter (2 keys, Tier A)

`printer_model`, `printer_structure`

## Tier B — 65 packets

### P21 — Extruder / Nozzle / MMU Hardware — wipe-tower (5 keys, Tier B)

`cooling_tube_length`, `cooling_tube_retraction`, `extra_loading_move`, `high_current_on_filament_swap`, `parking_pos_retraction`

### P22 — Multimaterial / Filament for Features — wipe-tower (1 keys, Tier B)

`wipe_tower_filament`

### P23 — Multimaterial / Flush options — wipe-tower (2 keys, Tier B)

`flush_multiplier`, `flush_volumes_matrix`

### P24 — Others / Special mode — wipe-tower (1 keys, Tier B)

`timelapse_type`

### P25 — Extruder / Nozzle / Nozzle — skirt-brim (1 keys, Tier B)

`nozzle_height`

### P26 — Calibration / Flow / Pressure advance calibration — infill modules (1 keys, Tier B)

`calib_flowrate_topinfill_special_order`

### P27 — Quality / Bridging — infill modules (3 keys, Tier B)

`bridge_density`, `internal_bridge_density`, `thick_internal_bridges`

### P28 — Strength / Advanced (Strength) — infill modules (3 keys, Tier B)

`align_infill_direction_to_model`, `detect_narrow_internal_solid_infill`, `minimum_sparse_infill_area`

### P29 — Quality / Line width — support-planner (1 keys, Tier B)

`support_line_width`

### P30 — Support / Advanced (Support) — support-planner (5 keys, Tier B)

`bridge_no_support`, `independent_support_layer_height`, `max_bridge_length`, `support_base_pattern`, `support_base_pattern_spacing`

### P31 — Support / Support filament — support-planner (2 keys, Tier B)

`support_filament`, `support_interface_filament`

### P32 — Extruder / Nozzle / Extruder geometry / mapping — emitter (7 keys, Tier B)

`extruder_colour`, `extruder_offset`, `extruder_type`, `master_extruder_id`, `physical_extruder_map`, `printer_extruder_id`, `printer_extruder_variant`

### P33 — Extruder / Nozzle / MMU Hardware — emitter (2 keys, Tier B)

`grab_length`, `start_end_points`

### P34 — Extruder / Nozzle / Nozzle — emitter (4 keys, Tier B)

`nozzle_hrc`, `nozzle_type`, `nozzle_volume`, `required_nozzle_HRC`

### P35 — Extruder / Nozzle / Pressure advance — emitter (6 keys, Tier B)

`adaptive_pressure_advance`, `adaptive_pressure_advance_bridges`, `adaptive_pressure_advance_model`, `adaptive_pressure_advance_overhangs`, `enable_pressure_advance`, `pressure_advance`

### P36 — Extruder / Nozzle / Retraction (1/2) — emitter (10 keys, Tier B)

`deretraction_speed`, `long_retractions_when_cut`, `long_retractions_when_ec`, `retract_before_wipe`, `retract_length_toolchange`, `retract_lift_above`, `retract_lift_below`, `retract_lift_enforce`, `retract_restart_extra`, `retract_restart_extra_toolchange`

### P37 — Extruder / Nozzle / Retraction (2/2) — emitter (10 keys, Tier B)

`retract_when_changing_layer`, `retraction_distances_when_cut`, `retraction_distances_when_ec`, `retraction_minimum_travel`, `travel_slope`, `use_firmware_retraction`, `wipe`, `wipe_distance`, `z_hop_types`, `z_offset`

### P38 — Filament / Bed temperature — emitter (2 keys, Tier B)

`bed_temperature_formula`, `curr_bed_type`

### P39 — Multimaterial / Filament for Features — emitter (3 keys, Tier B)

`solid_infill_filament`, `sparse_infill_filament`, `wall_filament`

### P40 — Multimaterial / Flush options — emitter (2 keys, Tier B)

`filament_flush_temp`, `filament_flush_volumetric_speed`

### P41 — Multimaterial / Multimaterial advanced — emitter (1 keys, Tier B)

`support_object_skip_flush`

### P42 — Multimaterial / Ooze prevention — emitter (4 keys, Tier B)

`ooze_prevention`, `preheat_steps`, `preheat_time`, `standby_temperature_delta`

### P43 — Multimaterial / Prime tower — emitter (2 keys, Tier B)

`manual_filament_change`, `single_extruder_multi_material_priming`

### P44 — Others / G-code output — emitter (6 keys, Tier B)

`exclude_object`, `filename_format`, `gcode_comments`, `gcode_flavor`, `gcode_label_objects`, `reduce_infill_retraction`

### P45 — Others / Special mode — emitter (5 keys, Tier B)

`spiral_finishing_flow_ratio`, `spiral_mode`, `spiral_mode_max_xy_smoothing`, `spiral_mode_smooth`, `spiral_starting_flow_ratio`

### P46 — Printer / Machine / Bed mesh — emitter (4 keys, Tier B)

`adaptive_bed_mesh_margin`, `bed_mesh_max`, `bed_mesh_min`, `bed_mesh_probe_distance`

### P47 — Printer / Machine / Motion limits — emitter (9 keys, Tier B)

`machine_max_acceleration_extruding`, `machine_max_acceleration_retracting`, `machine_max_acceleration_travel`, `machine_max_acceleration_x/y/z/e`, `machine_max_jerk_x/y/z/e`, `machine_max_junction_deviation`, `machine_max_speed_x/y/z/e`, `machine_min_extruding_rate`, `machine_min_travel_rate`

### P48 — Printer / Machine / Resonance — emitter (3 keys, Tier B)

`max_resonance_avoidance_speed`, `min_resonance_avoidance_speed`, `resonance_avoidance`

### P49 — Printer / Machine / Timing — emitter (4 keys, Tier B)

`machine_load_filament_time`, `machine_tool_change_time`, `machine_unload_filament_time`, `time_cost`

### P50 — Quality / Bridging — emitter (1 keys, Tier B)

`internal_bridge_flow`

### P51 — Quality / Precision — emitter (1 keys, Tier B)

`enable_arc_fitting`

### P52 — Quality / Seam (1/2) — emitter (8 keys, Tier B)

`has_scarf_joint_seam`, `role_based_wipe_speed`, `scarf_angle_threshold`, `scarf_joint_flow_ratio`, `scarf_joint_speed`, `scarf_overhang_threshold`, `seam_gap`, `seam_slope_conditional`

### P53 — Quality / Seam (2/2) — emitter (8 keys, Tier B)

`seam_slope_entire_loop`, `seam_slope_inner_walls`, `seam_slope_min_length`, `seam_slope_start_height`, `seam_slope_steps`, `seam_slope_type`, `wipe_before_external_loop`, `wipe_on_loops`

### P54 — Quality / Walls and surfaces (1/2) — emitter (9 keys, Tier B)

`bottom_solid_infill_flow_ratio`, `first_layer_flow_ratio`, `gap_fill_flow_ratio`, `inner_wall_flow_ratio`, `internal_solid_infill_flow_ratio`, `is_infill_first`, `max_travel_detour_distance`, `outer_wall_flow_ratio`, `overhang_flow_ratio`

### P55 — Quality / Walls and surfaces (2/2) — emitter (9 keys, Tier B)

`print_flow_ratio`, `reduce_crossing_wall`, `set_other_flow_ratios`, `small_area_infill_flow_compensation`, `small_area_infill_flow_compensation_model`, `sparse_infill_flow_ratio`, `support_flow_ratio`, `support_interface_flow_ratio`, `top_solid_infill_flow_ratio`

### P56 — Speed / Acceleration — emitter (11 keys, Tier B)

`accel_to_decel_enable`, `accel_to_decel_factor`, `bridge_acceleration`, `default_acceleration`, `initial_layer_acceleration`, `inner_wall_acceleration`, `internal_solid_infill_acceleration`, `outer_wall_acceleration`, `sparse_infill_acceleration`, `top_surface_acceleration`, `travel_acceleration`

### P57 — Speed / Advanced (Speed) — emitter (3 keys, Tier B)

`extrusion_rate_smoothing_external_perimeter_only`, `max_volumetric_extrusion_rate_slope`, `max_volumetric_extrusion_rate_slope_segment_length`

### P58 — Speed / Initial layer speed — emitter (1 keys, Tier B)

`slow_down_layers`

### P59 — Speed / Jerk (XY) — emitter (8 keys, Tier B)

`default_jerk`, `default_junction_deviation`, `infill_jerk`, `initial_layer_jerk`, `inner_wall_jerk`, `outer_wall_jerk`, `top_surface_jerk`, `travel_jerk`

### P60 — Speed / Other layers speed — emitter (2 keys, Tier B)

`internal_solid_infill_speed`, `small_perimeter_speed`

### P61 — Support / Support ironing — emitter (1 keys, Tier B)

`support_air_filtration`

### P62 — Cooling / Notes — tool-ordering (1 keys, Tier B)

`max_layer_height`

### P63 — Extruder / Nozzle / Extruder geometry / mapping — tool-ordering (1 keys, Tier B)

`extruder_ams_count`

### P64 — Extruder / Nozzle / Nozzle — tool-ordering (1 keys, Tier B)

`nozzle_volume_type`

### P65 — Multimaterial / Flush options — tool-ordering (3 keys, Tier B)

`flush_into_infill`, `flush_into_objects`, `flush_into_support`

### P66 — Quality / Layer height — tool-ordering (3 keys, Tier B)

`first_layer_print_sequence`, `other_layers_print_sequence`, `other_layers_print_sequence_nums`

### P67 — Support / Support filament — tool-ordering (1 keys, Tier B)

`support_interface_not_for_body`

### P68 — Cooling / Notes — layer-planner (1 keys, Tier B)

`min_layer_height`

### P69 — Others / Special mode — layer-planner (2 keys, Tier B)

`print_sequence`, `slicing_mode`

### P70 — Quality / Precision — layer-planner (1 keys, Tier B)

`precise_z_height`

### P71 — Quality / Overhangs — slice-prepass (3 keys, Tier B)

`make_overhang_printable`, `make_overhang_printable_angle`, `make_overhang_printable_hole_size`

### P72 — Support / Tree supports — tree-support (8 keys, Tier B)

`tree_support_angle_slow`, `tree_support_auto_brim`, `tree_support_branch_angle_organic`, `tree_support_branch_diameter_organic`, `tree_support_branch_distance_organic`, `tree_support_brim_width`, `tree_support_tip_diameter`, `tree_support_top_rate`

### P73 — Strength / Advanced (Strength) — object-level planning (4 keys, Tier B)

`ensure_vertical_shell_thickness`, `extra_solid_infills`, `infill_combination`, `infill_combination_max_layer_height`

### P74 — Strength / Top/bottom shells — object-level planning (2 keys, Tier B)

`bottom_shell_thickness`, `top_shell_thickness`

### P75 — Quality / Bridging — bridge-over-infill (3 keys, Tier B)

`dont_filter_internal_bridges`, `enable_extra_bridge_layer`, `internal_bridge_angle`

### P76 — Multimaterial / Multimaterial advanced — classic-perimeters (1 keys, Tier B)

`interface_shells`

### P77 — Quality / Bridging — classic-perimeters (2 keys, Tier B)

`bridge_angle`, `counterbore_hole_bridging`

### P78 — Filament / Bed temperature — print-orchestration (1 keys, Tier B)

`support_multi_bed_types`

### P79 — Printer / Machine / Print volume — print-orchestration (3 keys, Tier B)

`extruder_clearance_height_to_lid`, `extruder_clearance_height_to_rod`, `extruder_clearance_radius`

### P80 — Quality / Walls and surfaces — print-orchestration (1 keys, Tier B)

`extruder`

### P81 — Extruder / Nozzle / Extruder geometry / mapping — config-resolution (5 keys, Tier B)

`extruder_variant_list`, `filament_extruder_variant`, `filament_self_index`, `print_extruder_id`, `print_extruder_variant`

### P82 — Extruder / Nozzle / Nozzle — config-resolution (1 keys, Tier B)

`default_nozzle_volume_type`

### P83 — Multimaterial / Filament for Features — config-resolution (2 keys, Tier B)

`filament_map`, `filament_map_mode`

### P84 — Others / G-code output — host-export (1 keys, Tier B)

`gcode_add_line_number`

### P85 — Others / Post-processing Scripts — host-export (1 keys, Tier B)

`post_process`

## Tier C — 6 packets

### P86 — Quality / Precision — elefant-foot (new module, 2 keys, Tier C)

`elefant_foot_compensation`, `elefant_foot_compensation_layers`

### P87 — Quality / Precision — polyhole (new module, 3 keys, Tier C)

`hole_to_polyhole`, `hole_to_polyhole_threshold`, `hole_to_polyhole_twisted`

### P88 — Quality / Precision — contour-compensation (new module, 2 keys, Tier C)

`xy_contour_compensation`, `xy_hole_compensation`

### P89 — Multimaterial / Multimaterial advanced (1/2) — interlocking (new module, 3 keys, Tier C)

`interlocking_beam`, `interlocking_beam_layer_count`, `interlocking_beam_width`

### P90 — Multimaterial / Multimaterial advanced (2/2) — interlocking (new module, 3 keys, Tier C)

`interlocking_boundary_avoidance`, `interlocking_depth`, `interlocking_orientation`

### P91 — Multimaterial / Multimaterial advanced — mmu-segmented-region (new module, 2 keys, Tier C)

`mmu_segmented_region_interlocking_depth`, `mmu_segmented_region_max_width`
