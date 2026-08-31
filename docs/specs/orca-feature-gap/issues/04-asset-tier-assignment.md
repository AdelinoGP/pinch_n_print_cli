# Asset — Cost-tier rubric and tier assignment (ticket 04)

Derived from ticket 03's scoped list (414 keys) by applying the rubric below.
**Amended by ticket 07: +2 keys** — `ironing_type` and `support_ironing`
reclassified from 03's "already implemented (narrowed)" rows to genuine gaps
(see 07's answer). **Amended by ticket 99: +2 keys** — `fan_max_speed` and
`fan_min_speed` reclassified from false gaps (rename-exposed percent-vs-raw
scale deviation). Tier counts below include both amendments.
Every owner was **verified against the code** — first by read-site inspection
in PnP, then by **five adversarial review passes against canonical
OrcaSlicer** (parallel reviewers, each tracing canonical consumers). The
reviews corrected ~90 placements and ruled 11 keys out of scope. This asset
reflects the final converged map — pass 5 confirmed convergence (30-key
sample, zero real findings).

## The rubric

### Tier A — plumbing into an existing decision point

The owning module/crate exists **and** the decision point exists (the
behaviour is already implemented — under a different key, a typed
`ResolvedConfig` field, or a hardcoded path). Work: declare the key in the
owner's manifest + wire it to the existing decision point. No IR change, no
WIT change, no new module.

Detection (mechanical proxy, refined at authoring time): the owner reads a
sibling key from the same Orca section — exact Orca name or a Pinch rename
(03's alias table). The proxy sizes the queue; it is not a proof. Each packet
verifies its own keys' decision points at authoring time (in-map execution).

### Tier B — new logic in an existing owner

The owner exists (assigned at the correct seam) but the decision point does
not. Work: new behaviour inside the owner. No new module.

### Tier C — new module at a new seam

No existing owner at the correct seam. Work: a new core-module (granular —
one feature per module, per the effort's modularity principle), plus an ADR
where the repo gates new surfaces. New modules are **plural and
feature-coherent**, not one catch-all.

### Tier D — deferred (fog)

The seam itself is unresolved. Currently: the per-filament config model
(does Pinch 'n Print have a filament-profile concept at all?). All 47
deferred keys verified genuinely per-filament (coFloats/coStrings/coInts/
coBools in `PrintConfigDef::init_fff_params`, resolved via `get_at` /
`FILAMENT_CONFIG`). Tiered C pending; graduates when the fog clears.

### Out of scope (X)

Keys ruled out by the reviews and the human: dead-in-canonical (OrcaSlicer
itself never reads them in the pipeline), preset-management (matching
ticket 03's ruling), and dead alternate spellings. 11 keys; the scoped
target is now **407** (403 + ticket 07's two reclassified ironing keys +
ticket 99's two fan-scale reclassifications).

### Special rulings

- **ResolvedConfig-only keys** (5): `disable_m73`, `filament_density`,
  `filament_diameter`, `mmu_segmented_region_interlocking_depth`,
  `mmu_segmented_region_max_width` are implemented via typed fields and
  consumed at decision points, but are **not declared in any module
  manifest** — a contract violation. They are Tier A work: declare in the
  owning module's manifest + wire. (User ruling, ticket 04 session.)
- **Tie-breaker within a tier:** owning module — keeps each owner's diff
  local across the whole queue. (User ruling.)
- **Decision-point detection:** mechanical proxy (sibling-key read) for the
  bulk; the ambiguous remainder is flagged for per-key judgment at
  packet-authoring time. (User ruling.)
- **`hole_to_polyhole_max_edges`** — a 4th polyhole key exists in canonical
  but is missing from the 414 inventory (ticket 01 blind spot). Flagged for
  the inventory; not added to the queue here.
- **Citation fixes** (owners unchanged): `has_scarf_joint_seam` consumer is
  `GCodeProcessor.cpp` (layer-tag detection), not `extrude_loop`;
  `extruder_ams_count` consumer is `ToolOrdering::build_filament_group_context`
  → `FilamentGroupUtils::calc_max_group_size`, not `calc_extruder_count`;
  `default_bed_type`'s function is defined in `Preset.cpp` (preset-management),
  called from GUI.

## Tier counts

| Tier | keys | meaning |
|---:|---:|---|
| A | 119 | plumbing into an existing decision point (incl. `support_ironing`, +1 from ticket 07) |
| B | 226 | new logic in an existing owner (incl. `ironing_type` +1 from 07; `fan_max_speed`/`fan_min_speed` +2 from 99) |
| C | 15 | new granular modules (Precision 8, interlocking 6, mmu-segmented-region 2, minus precise_z_height folded into layer-planner) |
| D | 47 | deferred — per-filament config model (58 minus 11 global keys now assignable) |
| X | 11 | out of scope (dead-in-canonical 6+2, preset-management 3) |
| **in scope** | **407** | |

## Owner map (verified + five times adversarially reviewed)

| Section | Tier | Owner | Evidence |
|---|---|---|---|
| Support / Support, Interface, Raft, Advanced, Support filament | A/B | support-planner | canonical consumers in SupportMaterial.cpp / TreeSupport.cpp / SupportCommon.cpp / Slicing.cpp |
| Support / Tree supports | B | tree-support | canonical consumers in TreeSupport.cpp / TreeSupport3D.cpp / TreeSupportCommon.hpp |
| Support / Support ironing | A/B | support-surface-ironing | `support_ironing_pattern` in SupportParameters.hpp; `support_air_filtration` is emission-time → emitter |
| Multimaterial / Prime tower | A/B | wipe-tower | canonical consumers in WipeTower2.cpp / WipeTower.cpp ctors; `manual_filament_change` + `single_extruder_multi_material_priming` are emission-time → emitter |
| Multimaterial / Flush options | B | wipe-tower + tool-ordering + emitter | `flush_multiplier`/`flush_volumes_matrix` in WipeTower2.cpp; `flush_into_*` in ToolOrdering.cpp; `filament_flush_*` in GCode.cpp |
| Extruder / Nozzle / MMU Hardware | B | wipe-tower + emitter | cooling-tube/parking/ramming in WipeTower2.cpp; `grab_length`/`start_end_points` in GCode.cpp toolchange |
| Extruder / Nozzle / Retraction | B | **crates/slicer-gcode** | canonical `GCode::retract`/`GCode::travel`/`GCodeWriter::retract` — emission-time, NOT path-optimization |
| Quality / Seam | A/B | **crates/slicer-gcode** (16) + seam-placer (1) | canonical `GCode::extrude_loop` clipping + `GCodeProcessor.cpp`; only `staggered_inner_seams` in SeamPlacer.cpp |
| Quality / Ironing | A | top-surface-ironing + support-surface-ironing | canonical `Fill.cpp::Layer::make_ironing` |
| Quality / Layer height | B | tool-ordering | canonical `ToolOrdering.cpp::apply_first_layer_order`; 2 keys out of scope (dead spellings) |
| Quality / Walls and surfaces | B | crates/slicer-gcode (flow scaling, travel, ordering) + print/orchestration (`extruder`) | canonical `GCode::_extrude` mm3_per_mm scaling, `AvoidCrossingPerimeters.cpp`, `Print.cpp` per-object extruder |
| Quality / Wall generator — Arachne | A | arachne-perimeters | `min_feature_size` in Arachne/WallToolPaths.cpp |
| Quality / Line width | B | support-planner | `support_line_width` in Flow.cpp / TreeSupport.cpp (support flow) |
| Quality / Overhangs | B | slice-prepass | canonical `PrintObjectSlice.cpp::apply_conical_overhang` |
| Quality / Bridging | B (split) | pattern keys (2) → infill modules; bridge-over-infill (3) → slicing stage; bridge-angle + flow (2) → perimeters/emitter; perimeter (1) → classic/arachne | canonical `Fill.cpp::Layer::make_fills`, `PrintObject.cpp::bridge_over_infill`, `GCode::_extrude`, `PerimeterGenerator::process_no_bridge`, `LayerRegion.cpp` |
| Quality / Precision | C/B | new modules: elefant-foot (2), polyhole (3), contour-compensation (2); `enable_arc_fitting` → emitter; `precise_z_height` → layer-planner; `resolution` → emitter/generation-time (re-adjudicated in ticket 105) | canonical `PrintObject::slice` transforms; arc fitting is G2/G3 emission; z-height is layer-z generation |
| Others / Fuzzy Skin | A | fuzzy-skin | canonical `Feature/FuzzySkin/FuzzySkin.cpp::apply_fuzzy_skin` |
| Others / Brim, Skirt | A | skirt-brim | canonical `Brim.cpp::make_brim`, `Print.cpp::_make_skirt`, `GCode.cpp::generate_skirt` |
| Cooling / Notes | A/B | part-cooling (17) + tool-ordering (1) + layer-planner (1) | canonical CoolingBuffer.cpp / GCode.cpp emission-time cooling; `max_layer_height` in ToolOrdering.cpp; `min_layer_height` in Slicing.cpp |
| Strength / Infill, pattern-specific | A | infill modules | canonical `Fill.cpp::group_fills` |
| Strength / Top/bottom shells | A/B | infill modules (pattern/density) + object-level solid-fill planning (thickness) | canonical `Fill.cpp::group_fills` vs `PrintObject.cpp::discover_vertical_shells` |
| Strength / Advanced (Strength) | B | infill modules + object-level planning | `combine_infill`, `discover_vertical_shells` in PrintObject.cpp |
| Others / Special mode | B | emitter (spiral, timelapse) + layer-planner (print_sequence, slicing_mode) + wipe-tower (timelapse_type) | canonical SpiralVase.cpp, Print.cpp object ordering, PrintObjectSlice.cpp; `spiral_mode` is cross-cutting |
| Speed / Acceleration, Jerk, Advanced, Other layers, Initial layer | B | **crates/slicer-gcode** (host emitter) | canonical `GCode::_extrude`, `GCodeWriter::set_print_acceleration` / `set_jerk_xy`, PressureEqualizer.cpp |
| Others / G-code output | B | crates/slicer-gcode (6) + host export orchestration (1) | canonical GCodeWriter.cpp, GCode.cpp, Print.cpp::output_filename; `gcode_add_line_number` is GUI post-processor → host export |
| Others / Post-processing Scripts | B | host export orchestration (crates/slicer-runtime) | canonical GUI/PostProcessor.cpp::run_post_process_scripts |
| Printer / Machine / Motion limits, Timing, Resonance, Bed mesh | B | crates/slicer-gcode | canonical `GCode::print_machine_envelope` (M201/M203), GCodeProcessor.cpp, GCode.cpp G29 |
| Printer / Machine / Power / recovery | A | crates/slicer-gcode | `disable_m73` consumed in emit.rs; canonical GCodeWriter::set_m73 |
| Printer / Machine / Printer identity | A | crates/slicer-gcode | `printer_model`/`printer_structure` in GCode.cpp / GCodeProcessor.cpp; 3 keys out of scope (preset-management) |
| Printer / Machine / Print volume | A/B | wipe-tower + crates/slicer-gcode + print/orchestration | `bed_shape` read by wipe-tower; `printable_height` in emitter; clearance keys in Print.cpp arrangement |
| Extruder / Nozzle / Extruder geometry, Nozzle, Pressure advance | B | crates/slicer-gcode + config-resolution + tool-ordering + skirt-brim | canonical GCode.cpp toolchange, GCodeProcessor.cpp, AdaptivePAProcessor.cpp; `extruder_ams_count`/`nozzle_volume_type` in ToolOrdering.cpp; `default_nozzle_volume_type` in PresetBundle.cpp; `nozzle_height` in Print.cpp skirt check |
| Multimaterial / Ooze prevention | B | crates/slicer-gcode | canonical `GCode::OozePrevention::pre_toolchange` |
| Calibration / Flow / PA calibration | B | infill modules | canonical `FillBase.cpp::fill` |
| Multimaterial / Multimaterial advanced | C/B | new interlocking module (6), new mmu-segmented-region module (2); `interface_shells` → classic-perimeters; `support_object_skip_flush` → emitter | canonical InterlockingGenerator.cpp (Feature/Interlocking/), MultiMaterialSegmentation.cpp, PrintObject.cpp, GCode.cpp |
| Filament / Notes, Bed temperature, Temperature (Nozzle), Filament for Features | D/B | 47 keys deferred (per-filament, verified); 9 global keys assignable now (bed-type 3 → emitter/print-orchestration, filament_map 2 → config-resolution, feature-filament 4 → emitter/wipe-tower); 2 out of scope | canonical coFloats/coStrings per-filament arrays vs global keys; `default_bed_type`/`support_chamber_temp_control` GUI-only |

## Adversarial review corrections (five passes, summary)

**Pass 1** (six reviewers) corrected ~50: flow-ratio keys (15) are
emission-time scaling; spiral keys (5) are emission-time; shell thickness /
infill combination (6) are object-level planning; `enable_arc_fitting` is
G2/G3 emission; toolchange keys (6) are emission-time; `flush_into_*` (3) are
tool-ordering; `print_sequence`/`slicing_mode` are object-ordering / slicing
prepass; 11 filament keys are global; bridging split refined.

**Pass 2** (three reviewers) corrected the corrections: Seam (16/17) and
Retraction (20) are emission-time; print-volume clearance keys (3) are
arrangement; 5 tree-support keys are live (not config-only); **9 keys ruled
out of scope** (user ruling); `precise_z_height` folded into layer-planner;
`spiral_mode` recorded cross-cutting.

**Pass 3** (three reviewers) corrected 7: `post_process` +
`gcode_add_line_number` are host-export (GUI post-processor in canonical);
`extruder_ams_count` + `nozzle_volume_type` are tool-ordering;
`default_nozzle_volume_type` is config-resolution; `nozzle_height` is
skirt-brim; `bridge_angle` is consumed in LayerRegion.cpp + PerimeterGenerator.cpp
(config value), not the fill stage.

**Pass 4** (two reviewers) corrected 5: `max_layer_height` is tool-ordering;
`min_layer_height` is layer-planner; `support_multi_bed_types` is
print/orchestration; `default_bed_type` + `support_chamber_temp_control` have
no pipeline consumer → out of scope (matching the established classes).

**Pass 5** (two reviewers) — **converged**: all pass-4 corrections
re-verified and confirmed; 30-key random-sample audit found zero real
findings (the one flagged row was a stale-asset artifact).

## Per-key assignment


### Calibration / Flow / Pressure advance calibration
| `calib_flowrate_topinfill_special_order` | B | infill modules (FillBase.cpp) |

### Cooling / Notes
| `activate_air_filtration` | A | part-cooling (emission-time cooling) |
| `activate_chamber_temp_control` | A | part-cooling (emission-time cooling) |
| `additional_cooling_fan_speed` | A | part-cooling (emission-time cooling) |
| `auxiliary_fan` | A | part-cooling (emission-time cooling) |
| `complete_print_exhaust_fan_speed` | A | part-cooling (emission-time cooling) |
| `dont_slow_down_outer_wall` | A | part-cooling (emission-time cooling) |
| `during_print_exhaust_fan_speed` | A | part-cooling (emission-time cooling) |
| `fan_cooling_layer_time` | A | part-cooling (emission-time cooling) |
| `fan_kickstart` | A | part-cooling (emission-time cooling) |
| `fan_max_speed` | B | part-cooling (emission-time cooling) | (ticket 99 finding — rename exposed percent-vs-raw scale gap; Orca 0–100 % vs Pinch raw 0–255) |
| `fan_min_speed` | B | part-cooling (emission-time cooling) | (ticket 99 finding — same scale gap; key declared but never read; wire to consumer via P01's reduce_fan_stop_start_freq work) |
| `fan_speedup_overhangs` | A | part-cooling (emission-time cooling) |
| `fan_speedup_time` | A | part-cooling (emission-time cooling) |
| `full_fan_speed_layer` | A | part-cooling (emission-time cooling) |
| `internal_bridge_fan_speed` | A | part-cooling (emission-time cooling) |
| `ironing_fan_speed` | A | part-cooling (emission-time cooling) |
| `max_layer_height` | B | tool-ordering (ToolOrdering.cpp calc_max_layer_height) |
| `min_layer_height` | B | layer-planner (Slicing.cpp min_layer_height_from_nozzle) |
| `overhang_fan_threshold` | A | part-cooling (emission-time cooling) |
| `reduce_fan_stop_start_freq` | A | part-cooling (emission-time cooling) |
| `support_material_interface_fan_speed` | A | part-cooling (emission-time cooling) |

### Extruder / Nozzle / Extruder geometry / mapping
| `extruder_ams_count` | B | tool-ordering (ToolOrdering.cpp calc_max_group_size) |
| `extruder_colour` | B | crates/slicer-gcode (toolchange emission) |
| `extruder_offset` | B | crates/slicer-gcode (toolchange emission) |
| `extruder_type` | B | crates/slicer-gcode (toolchange emission) |
| `extruder_variant_list` | B | config-resolution (Print.cpp get_config_index) |
| `filament_extruder_variant` | B | config-resolution (Print.cpp get_config_index) |
| `filament_self_index` | B | config-resolution (Print.cpp update_filament_self_index_cache) |
| `master_extruder_id` | B | crates/slicer-gcode (toolchange emission) |
| `physical_extruder_map` | B | crates/slicer-gcode (toolchange emission) |
| `print_extruder_id` | B | config-resolution (Print.cpp get_config_index) |
| `print_extruder_variant` | B | config-resolution (Print.cpp get_config_index) |
| `printer_extruder_id` | B | crates/slicer-gcode (toolchange emission) |
| `printer_extruder_variant` | B | crates/slicer-gcode (toolchange emission) |

### Extruder / Nozzle / MMU Hardware
| `cooling_tube_length` | B | wipe-tower |
| `cooling_tube_retraction` | B | wipe-tower |
| `extra_loading_move` | B | wipe-tower |
| `grab_length` | B | crates/slicer-gcode (toolchange) |
| `high_current_on_filament_swap` | B | wipe-tower |
| `parking_pos_retraction` | B | wipe-tower |
| `start_end_points` | B | crates/slicer-gcode (get_path_of_change_filament) |

### Extruder / Nozzle / Nozzle
| `default_nozzle_volume_type` | B | config-resolution (PresetBundle.cpp) |
| `nozzle_height` | B | skirt-brim (Print.cpp skirt/draft-shield height) |
| `nozzle_hrc` | B | crates/slicer-gcode (nozzle_diameter lives in perimeters) |
| `nozzle_type` | B | crates/slicer-gcode (nozzle_diameter lives in perimeters) |
| `nozzle_volume` | B | crates/slicer-gcode (nozzle_diameter lives in perimeters) |
| `nozzle_volume_type` | B | tool-ordering (ToolOrdering.cpp + MultiNozzleUtils.cpp) |
| `required_nozzle_HRC` | B | crates/slicer-gcode (nozzle_diameter lives in perimeters) |

### Extruder / Nozzle / Pressure advance
| `adaptive_pressure_advance` | B | crates/slicer-gcode (flavor.rs) |
| `adaptive_pressure_advance_bridges` | B | crates/slicer-gcode (flavor.rs) |
| `adaptive_pressure_advance_model` | B | crates/slicer-gcode (flavor.rs) |
| `adaptive_pressure_advance_overhangs` | B | crates/slicer-gcode (flavor.rs) |
| `enable_pressure_advance` | B | crates/slicer-gcode (flavor.rs) |
| `pressure_advance` | B | crates/slicer-gcode (flavor.rs) |

### Extruder / Nozzle / Retraction
| `deretraction_speed` | B | crates/slicer-gcode (GCode::retract) |
| `long_retractions_when_cut` | B | crates/slicer-gcode (GCode::retract) |
| `long_retractions_when_ec` | B | crates/slicer-gcode (GCode::retract) |
| `retract_before_wipe` | B | crates/slicer-gcode (GCode::retract) |
| `retract_length_toolchange` | B | crates/slicer-gcode (GCode::retract) |
| `retract_lift_above` | B | crates/slicer-gcode (GCode::retract) |
| `retract_lift_below` | B | crates/slicer-gcode (GCode::retract) |
| `retract_lift_enforce` | B | crates/slicer-gcode (GCode::retract) |
| `retract_restart_extra` | B | crates/slicer-gcode (GCode::retract) |
| `retract_restart_extra_toolchange` | B | crates/slicer-gcode (GCode::retract) |
| `retract_when_changing_layer` | B | crates/slicer-gcode (GCode::retract) |
| `retraction_distances_when_cut` | B | crates/slicer-gcode (GCode::retract) |
| `retraction_distances_when_ec` | B | crates/slicer-gcode (GCode::retract) |
| `retraction_minimum_travel` | B | crates/slicer-gcode (GCode::retract) |
| `travel_slope` | B | crates/slicer-gcode (GCode::travel) |
| `use_firmware_retraction` | B | crates/slicer-gcode (GCodeWriter::retract) |
| `wipe` | B | crates/slicer-gcode (GCode::retract) |
| `wipe_distance` | B | crates/slicer-gcode (GCode::retract) |
| `z_hop_types` | B | crates/slicer-gcode (GCode::travel) |
| `z_offset` | B | crates/slicer-gcode (GCode::travel) |

### Filament / Bed temperature
| `bed_temperature_formula` | B | crates/slicer-gcode (bed-temp selection, global) |
| `cool_plate_temp` | D | deferred (per-filament config model) |
| `cool_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `curr_bed_type` | B | crates/slicer-gcode (bed-type, global) |
| `default_bed_type` | X | out of scope — no pipeline consumer (GUI Plater.cpp only, preset-management) |
| `eng_plate_temp` | D | deferred (per-filament config model) |
| `eng_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `hot_plate_temp` | D | deferred (per-filament config model) |
| `hot_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `supertack_plate_temp` | D | deferred (per-filament config model) |
| `supertack_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `support_chamber_temp_control` | X | out of scope — no pipeline consumer (GUI only, dead-in-canonical) |
| `support_multi_bed_types` | B | print/orchestration (Print.cpp validate) |
| `textured_cool_plate_temp` | D | deferred (per-filament config model) |
| `textured_cool_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `textured_plate_temp` | D | deferred (per-filament config model) |
| `textured_plate_temp_initial_layer` | D | deferred (per-filament config model) |

### Filament / Notes
| `filament_adaptive_volumetric_speed` | D | deferred (per-filament config model) |
| `filament_change_length` | D | deferred (per-filament config model) |
| `filament_cooling_final_speed` | D | deferred (per-filament config model) |
| `filament_cooling_initial_speed` | D | deferred (per-filament config model) |
| `filament_cooling_moves` | D | deferred (per-filament config model) |
| `filament_cost` | D | deferred (per-filament config model) |
| `filament_density` | A | declare in manifest (consumed in emit.rs); blocked on filament fog |
| `filament_diameter` | A | declare in manifest (consumed in emit.rs); blocked on filament fog |
| `filament_flow_ratio` | D | deferred (per-filament config model) |
| `filament_ironing_flow` | D | deferred (per-filament config model) |
| `filament_ironing_inset` | D | deferred (per-filament config model) |
| `filament_ironing_spacing` | D | deferred (per-filament config model) |
| `filament_is_support` | D | deferred (per-filament config model) |
| `filament_loading_speed` | D | deferred (per-filament config model) |
| `filament_loading_speed_start` | D | deferred (per-filament config model) |
| `filament_max_volumetric_speed` | D | deferred (per-filament config model) |
| `filament_minimal_purge_on_wipe_tower` | D | deferred (per-filament config model) |
| `filament_multitool_ramming` | D | deferred (per-filament config model) |
| `filament_multitool_ramming_flow` | D | deferred (per-filament config model) |
| `filament_multitool_ramming_volume` | D | deferred (per-filament config model) |
| `filament_ramming_parameters` | D | deferred (per-filament config model) |
| `filament_shrink` | D | deferred (per-filament config model) |
| `filament_shrinkage_compensation_z` | D | deferred (per-filament config model) |
| `filament_soluble` | D | deferred (per-filament config model) |
| `filament_stamping_distance` | D | deferred (per-filament config model) |
| `filament_stamping_loading_speed` | D | deferred (per-filament config model) |
| `filament_toolchange_delay` | D | deferred (per-filament config model) |
| `filament_type` | D | deferred (per-filament config model) |
| `filament_unloading_speed` | D | deferred (per-filament config model) |
| `filament_unloading_speed_start` | D | deferred (per-filament config model) |
| `temperature_vitrification` | D | deferred (per-filament config model) |
| `volumetric_speed_coefficients` | D | deferred (per-filament config model) |

### Filament / Temperature (Nozzle)
| `chamber_temperature` | D | deferred (per-filament config model) |
| `idle_temperature` | D | deferred (per-filament config model) |
| `nozzle_temperature` | D | deferred (per-filament config model) |
| `nozzle_temperature_range_high` | D | deferred (per-filament config model) |
| `nozzle_temperature_range_low` | D | deferred (per-filament config model) |

### Multimaterial / Filament for Features
| `filament_map` | B | config-resolution (Print.cpp get_filament_maps, print-level) |
| `filament_map_mode` | B | config-resolution (Print.cpp get_filament_map_mode, global) |
| `solid_infill_filament` | B | crates/slicer-gcode (per-region filament selection) |
| `sparse_infill_filament` | B | crates/slicer-gcode (per-region filament selection) |
| `wall_filament` | B | crates/slicer-gcode (per-region filament selection) |
| `wipe_tower_filament` | B | wipe-tower (tower filament, global) |

### Multimaterial / Flush options
| `filament_flush_temp` | B | crates/slicer-gcode (toolchange flush) |
| `filament_flush_volumetric_speed` | B | crates/slicer-gcode (toolchange flush) |
| `flush_into_infill` | B | tool-ordering (ToolOrdering.cpp) |
| `flush_into_objects` | B | tool-ordering (ToolOrdering.cpp) |
| `flush_into_support` | B | tool-ordering (ToolOrdering.cpp) |
| `flush_multiplier` | B | wipe-tower |
| `flush_volumes_matrix` | B | wipe-tower |
| `flush_volumes_vector` | X | out of scope — preset-management metadata (03 class) |
| `wiping_volumes_extruders` | X | out of scope — dead in canonical |

### Multimaterial / Multimaterial advanced
| `interface_shells` | B | classic-perimeters (shell planning, PrintObject.cpp) |
| `interlocking_beam` | C | new interlocking module |
| `interlocking_beam_layer_count` | C | new interlocking module |
| `interlocking_beam_width` | C | new interlocking module |
| `interlocking_boundary_avoidance` | C | new interlocking module |
| `interlocking_depth` | C | new interlocking module |
| `interlocking_orientation` | C | new interlocking module |
| `mmu_segmented_region_interlocking_depth` | C | new mmu-segmented-region module (consumed host-side in paint_segmentation) |
| `mmu_segmented_region_max_width` | C | new mmu-segmented-region module (consumed host-side in paint_segmentation) |
| `support_object_skip_flush` | B | crates/slicer-gcode (exclude-object emission) |

### Multimaterial / Ooze prevention
| `ooze_prevention` | B | crates/slicer-gcode (standby_temperature in serialize.rs) |
| `preheat_steps` | B | crates/slicer-gcode (standby_temperature in serialize.rs) |
| `preheat_time` | B | crates/slicer-gcode (standby_temperature in serialize.rs) |
| `standby_temperature_delta` | B | crates/slicer-gcode (standby_temperature in serialize.rs) |

### Multimaterial / Prime tower
| `enable_filament_ramming` | A | wipe-tower |
| `enable_tower_interface_cooldown_during_tower` | A | wipe-tower |
| `enable_tower_interface_features` | A | wipe-tower |
| `filament_tower_interface_pre_extrusion_dist` | A | wipe-tower |
| `filament_tower_interface_pre_extrusion_length` | A | wipe-tower |
| `filament_tower_interface_print_temp` | A | wipe-tower |
| `filament_tower_interface_purge_volume` | A | wipe-tower |
| `filament_tower_ironing_area` | A | wipe-tower |
| `manual_filament_change` | B | crates/slicer-gcode (toolchange emission) |
| `prime_tower_brim_width` | A | wipe-tower |
| `prime_tower_enable_framework` | A | wipe-tower |
| `prime_tower_flat_ironing` | A | wipe-tower |
| `prime_tower_infill_gap` | A | wipe-tower |
| `prime_tower_skip_points` | A | wipe-tower |
| `purge_in_prime_tower` | A | wipe-tower |
| `single_extruder_multi_material` | A | wipe-tower |
| `single_extruder_multi_material_priming` | B | crates/slicer-gcode (toolchange emission) |
| `wipe_tower_bridging` | A | wipe-tower |
| `wipe_tower_cone_angle` | A | wipe-tower |
| `wipe_tower_extra_flow` | A | wipe-tower |
| `wipe_tower_extra_rib_length` | A | wipe-tower |
| `wipe_tower_extra_spacing` | A | wipe-tower |
| `wipe_tower_fillet_wall` | A | wipe-tower |
| `wipe_tower_max_purge_speed` | A | wipe-tower |
| `wipe_tower_no_sparse_layers` | A | wipe-tower |
| `wipe_tower_rib_width` | A | wipe-tower |
| `wipe_tower_rotation_angle` | A | wipe-tower |
| `wipe_tower_wall_type` | A | wipe-tower |

### Others / Brim
| `brim_ears` | A | skirt-brim |
| `brim_ears_detection_length` | A | skirt-brim |
| `brim_ears_max_angle` | A | skirt-brim |
| `brim_object_gap` | A | skirt-brim |
| `brim_type` | A | skirt-brim |
| `brim_use_efc_outline` | A | skirt-brim |

### Others / Fuzzy Skin
| `fuzzy_skin` | A | fuzzy-skin |
| `fuzzy_skin_first_layer` | A | fuzzy-skin |
| `fuzzy_skin_mode` | A | fuzzy-skin |
| `fuzzy_skin_noise_type` | A | fuzzy-skin |
| `fuzzy_skin_octaves` | A | fuzzy-skin |
| `fuzzy_skin_persistence` | A | fuzzy-skin |
| `fuzzy_skin_scale` | A | fuzzy-skin |

### Others / G-code output
| `exclude_object` | B | crates/slicer-gcode (flavor.rs) |
| `filename_format` | B | crates/slicer-gcode (flavor.rs) |
| `gcode_add_line_number` | B | host export orchestration (crates/slicer-runtime; GUI post-processor in canonical) |
| `gcode_comments` | B | crates/slicer-gcode (flavor.rs) |
| `gcode_flavor` | B | crates/slicer-gcode (flavor.rs) |
| `gcode_label_objects` | B | crates/slicer-gcode (flavor.rs) |
| `reduce_infill_retraction` | B | crates/slicer-gcode (flavor.rs) |

### Others / Post-processing Scripts
| `post_process` | B | host export orchestration (crates/slicer-runtime) |

### Others / Skirt
| `draft_shield` | A | skirt-brim |
| `min_skirt_length` | A | skirt-brim |
| `single_loop_draft_shield` | A | skirt-brim |
| `skirt_start_angle` | A | skirt-brim |
| `skirt_type` | A | skirt-brim |

### Others / Special mode
| `enable_timelapse` | X | out of scope — dead in canonical (superseded by timelapse_type) |
| `print_sequence` | B | layer-planner (object ordering, Print.cpp) |
| `slicing_mode` | B | layer-planner (PrintObjectSlice.cpp) |
| `spiral_finishing_flow_ratio` | B | crates/slicer-gcode (SpiralVase.cpp) |
| `spiral_mode` | B | print/orchestration + crates/slicer-gcode (cross-cutting: slicing + SpiralVase) |
| `spiral_mode_max_xy_smoothing` | B | crates/slicer-gcode (SpiralVase.cpp) |
| `spiral_mode_smooth` | B | crates/slicer-gcode (SpiralVase.cpp) |
| `spiral_starting_flow_ratio` | B | crates/slicer-gcode (SpiralVase.cpp) |
| `timelapse_type` | B | wipe-tower (primary) + crates/slicer-gcode |

### Printer / Machine / Bed mesh
| `adaptive_bed_mesh_margin` | B | crates/slicer-gcode (G29 emission) |
| `bed_mesh_max` | B | crates/slicer-gcode (G29 emission) |
| `bed_mesh_min` | B | crates/slicer-gcode (G29 emission) |
| `bed_mesh_probe_distance` | B | crates/slicer-gcode (G29 emission) |

### Printer / Machine / Motion limits
| `machine_max_acceleration_extruding` | B | crates/slicer-gcode (M201/M203 emission) |
| `machine_max_acceleration_retracting` | B | crates/slicer-gcode (M201/M203 emission) |
| `machine_max_acceleration_travel` | B | crates/slicer-gcode (M201/M203 emission) |
| `machine_max_acceleration_x/y/z/e` | B | crates/slicer-gcode (M201/M203 emission) |
| `machine_max_jerk_x/y/z/e` | B | crates/slicer-gcode (M201/M203 emission) |
| `machine_max_junction_deviation` | B | crates/slicer-gcode (M201/M203 emission) |
| `machine_max_speed_x/y/z/e` | B | crates/slicer-gcode (M201/M203 emission) |
| `machine_min_extruding_rate` | B | crates/slicer-gcode (M201/M203 emission) |
| `machine_min_travel_rate` | B | crates/slicer-gcode (M201/M203 emission) |

### Printer / Machine / Power / recovery
| `disable_m73` | A | crates/slicer-gcode (consumed in emit.rs) + declare in machine-gcode-emit manifest |
| `emit_machine_limits_to_gcode` | A | crates/slicer-gcode (disable_m73 consumed) |
| `enable_power_loss_recovery` | A | crates/slicer-gcode (disable_m73 consumed) |
| `silent_mode` | A | crates/slicer-gcode (disable_m73 consumed) |

### Printer / Machine / Print volume
| `bed_exclude_area` | A | wipe-tower (bed_shape) + crates/slicer-gcode (printable_height) |
| `extruder_clearance_height_to_lid` | B | print/orchestration (Print.cpp arrangement) |
| `extruder_clearance_height_to_rod` | B | print/orchestration (Print.cpp arrangement) |
| `extruder_clearance_radius` | B | print/orchestration (Print.cpp arrangement) |
| `extruder_printable_area` | A | wipe-tower (bed_shape) + crates/slicer-gcode (printable_height) |
| `extruder_printable_height` | A | wipe-tower (bed_shape) + crates/slicer-gcode (printable_height) |
| `printable_height` | A | wipe-tower (bed_shape) + crates/slicer-gcode (printable_height) |

### Printer / Machine / Printer identity
| `allow_mix_temp` | X | out of scope — dead in canonical |
| `printer_model` | A | crates/slicer-gcode (printer_technology in serialize.rs) |
| `printer_structure` | A | crates/slicer-gcode (printer_technology in serialize.rs) |
| `printer_technology` | X | out of scope — preset-management (03 class) |
| `printer_variant` | X | out of scope — preset-management (03 class; SLA-only metadata consumer in Format/SL1.cpp) |

### Printer / Machine / Resonance
| `max_resonance_avoidance_speed` | B | crates/slicer-gcode |
| `min_resonance_avoidance_speed` | B | crates/slicer-gcode |
| `resonance_avoidance` | B | crates/slicer-gcode |

### Printer / Machine / Timing
| `machine_load_filament_time` | B | crates/slicer-gcode (estimator.rs) |
| `machine_tool_change_time` | B | crates/slicer-gcode (estimator.rs) |
| `machine_unload_filament_time` | B | crates/slicer-gcode (estimator.rs) |
| `time_cost` | B | crates/slicer-gcode (estimator.rs) |

### Quality / Bridging
| `bridge_angle` | B | classic-perimeters + arachne-perimeters (LayerRegion.cpp + PerimeterGenerator.cpp) |
| `bridge_density` | B | infill modules (bridge-fill holder) |
| `counterbore_hole_bridging` | B | classic-perimeters + arachne-perimeters |
| `dont_filter_internal_bridges` | B | bridge-over-infill (slicing stage, PrintObject.cpp) |
| `enable_extra_bridge_layer` | B | bridge-over-infill (slicing stage, PrintObject.cpp) |
| `internal_bridge_angle` | B | bridge-over-infill (slicing stage, PrintObject.cpp) |
| `internal_bridge_density` | B | infill modules (bridge-fill holder) |
| `internal_bridge_flow` | B | crates/slicer-gcode (emission flow scaling) |
| `thick_internal_bridges` | B | infill modules (bridge-fill holder, Fill.cpp) |

### Quality / Ironing
| `ironing_angle` | A | top-surface-ironing + support-surface-ironing |
| `ironing_angle_fixed` | A | top-surface-ironing + support-surface-ironing |
| `ironing_inset` | A | top-surface-ironing + support-surface-ironing |
| `ironing_type` | B | top-surface-ironing + support-surface-ironing | (ticket 07 reclassification — enum modes unexpressible via the shared `ironing_enabled` bool; mode-selection logic) |

### Quality / Layer height
| `first_layer_print_sequence` | B | tool-ordering (ToolOrdering.cpp) |
| `first_layer_sequence_choice` | X | out of scope — dead alternate spelling |
| `other_layers_print_sequence` | B | tool-ordering (ToolOrdering.cpp) |
| `other_layers_print_sequence_nums` | B | tool-ordering (ToolOrdering.cpp) |
| `other_layers_sequence_choice` | X | out of scope — dead alternate spelling |

### Quality / Line width
| `support_line_width` | B | support-planner (support flow) |

### Quality / Overhangs
| `make_overhang_printable` | B | slice-prepass (apply_conical_overhang) |
| `make_overhang_printable_angle` | B | slice-prepass (apply_conical_overhang) |
| `make_overhang_printable_hole_size` | B | slice-prepass (apply_conical_overhang) |

### Quality / Precision
| `elefant_foot_compensation` | C | new elefant-foot module |
| `elefant_foot_compensation_layers` | C | new elefant-foot module |
| `enable_arc_fitting` | B | crates/slicer-gcode (G2/G3 emission) |
| `hole_to_polyhole` | C | new polyhole module |
| `hole_to_polyhole_threshold` | C | new polyhole module |
| `hole_to_polyhole_twisted` | C | new polyhole module |
| `precise_z_height` | B | layer-planner (Slicing.cpp generate_object_layers) |
| `resolution` | B | crates/slicer-gcode / generation-time simplify — re-adjudicated in ticket 105 (canonical `PerimeterGenerator.cpp` `ex.simplify_p`, `Brim.cpp`, `Fill.cpp`, `GCodeWriter.cpp` arc density; the host's emit-time per-role `gcode_resolution` is not the same decision point) |
| `xy_contour_compensation` | C | new contour-compensation module |
| `xy_hole_compensation` | C | new contour-compensation module |

### Quality / Seam
| `has_scarf_joint_seam` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `role_based_wipe_speed` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `scarf_angle_threshold` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `scarf_joint_flow_ratio` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `scarf_joint_speed` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `scarf_overhang_threshold` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `seam_gap` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `seam_slope_conditional` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `seam_slope_entire_loop` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `seam_slope_inner_walls` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `seam_slope_min_length` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `seam_slope_start_height` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `seam_slope_steps` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `seam_slope_type` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `staggered_inner_seams` | A | seam-placer (SeamPlacer.cpp) |
| `wipe_before_external_loop` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |
| `wipe_on_loops` | B | crates/slicer-gcode (GCode::extrude_loop clipping) |

### Quality / Wall generator — Arachne
| `min_feature_size` | A | arachne-perimeters |

### Quality / Walls and surfaces
| `bottom_solid_infill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `extruder` | B | print/orchestration (per-object extruder assignment) |
| `first_layer_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `gap_fill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `inner_wall_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `internal_solid_infill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `is_infill_first` | B | crates/slicer-gcode (emission ordering) |
| `max_travel_detour_distance` | B | crates/slicer-gcode (travel planning) |
| `outer_wall_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `overhang_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `print_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `reduce_crossing_wall` | B | crates/slicer-gcode (travel planning) |
| `set_other_flow_ratios` | B | crates/slicer-gcode (emission flow scaling) |
| `small_area_infill_flow_compensation` | B | crates/slicer-gcode (emission flow scaling) |
| `small_area_infill_flow_compensation_model` | B | crates/slicer-gcode (emission flow scaling) |
| `sparse_infill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `support_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `support_interface_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |
| `top_solid_infill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) |

### Speed / Acceleration
| `accel_to_decel_enable` | B | crates/slicer-gcode (estimator.rs) |
| `accel_to_decel_factor` | B | crates/slicer-gcode (estimator.rs) |
| `bridge_acceleration` | B | crates/slicer-gcode (estimator.rs) |
| `default_acceleration` | B | crates/slicer-gcode (estimator.rs) |
| `initial_layer_acceleration` | B | crates/slicer-gcode (estimator.rs) |
| `inner_wall_acceleration` | B | crates/slicer-gcode (estimator.rs) |
| `internal_solid_infill_acceleration` | B | crates/slicer-gcode (estimator.rs) |
| `outer_wall_acceleration` | B | crates/slicer-gcode (estimator.rs) |
| `sparse_infill_acceleration` | B | crates/slicer-gcode (estimator.rs) |
| `top_surface_acceleration` | B | crates/slicer-gcode (estimator.rs) |
| `travel_acceleration` | B | crates/slicer-gcode (estimator.rs) |

### Speed / Advanced (Speed)
| `extrusion_rate_smoothing_external_perimeter_only` | B | crates/slicer-gcode (estimator.rs) |
| `max_volumetric_extrusion_rate_slope` | B | crates/slicer-gcode (estimator.rs) |
| `max_volumetric_extrusion_rate_slope_segment_length` | B | crates/slicer-gcode (estimator.rs) |

### Speed / Initial layer speed
| `slow_down_layers` | B | crates/slicer-gcode (feedrate.rs) |

### Speed / Jerk (XY)
| `default_jerk` | B | crates/slicer-gcode (estimator.rs) |
| `default_junction_deviation` | B | crates/slicer-gcode (estimator.rs) |
| `infill_jerk` | B | crates/slicer-gcode (estimator.rs) |
| `initial_layer_jerk` | B | crates/slicer-gcode (estimator.rs) |
| `inner_wall_jerk` | B | crates/slicer-gcode (estimator.rs) |
| `outer_wall_jerk` | B | crates/slicer-gcode (estimator.rs) |
| `top_surface_jerk` | B | crates/slicer-gcode (estimator.rs) |
| `travel_jerk` | B | crates/slicer-gcode (estimator.rs) |

### Speed / Other layers speed
| `internal_solid_infill_speed` | B | crates/slicer-gcode (feedrate.rs) |
| `small_perimeter_speed` | B | crates/slicer-gcode (feedrate.rs) |

### Strength / Advanced (Strength)
| `align_infill_direction_to_model` | B | infill modules |
| `detect_narrow_internal_solid_infill` | B | infill modules |
| `ensure_vertical_shell_thickness` | B | object-level solid-fill planning (PrintObject.cpp) |
| `extra_solid_infills` | B | object-level solid-fill planning (PrintObject.cpp) |
| `infill_combination` | B | object-level infill planning (PrintObject.cpp) |
| `infill_combination_max_layer_height` | B | object-level infill planning (PrintObject.cpp) |
| `minimum_sparse_infill_area` | B | infill modules |

### Strength / Infill
| `fill_multiline` | A | infill modules |
| `gap_fill_target` | A | infill modules |
| `internal_solid_infill_pattern` | A | infill modules |
| `solid_infill_direction` | A | infill modules |
| `solid_infill_rotate_template` | A | infill modules |
| `sparse_infill_pattern` | A | infill modules |
| `sparse_infill_rotate_template` | A | infill modules |

### Strength / Infill pattern-specific
| `infill_lock_depth` | A | infill modules |
| `infill_overhang_angle` | A | infill modules |
| `lateral_lattice_angle_1` | A | infill modules |
| `lateral_lattice_angle_2` | A | infill modules |
| `skeleton_infill_density` | A | infill modules |
| `skeleton_infill_line_width` | A | infill modules |
| `skin_infill_density` | A | infill modules |
| `skin_infill_depth` | A | infill modules |
| `skin_infill_line_width` | A | infill modules |
| `symmetric_infill_y_axis` | A | infill modules |

### Strength / Top/bottom shells
| `bottom_shell_thickness` | B | object-level solid-fill planning (PrintObject.cpp) |
| `bottom_surface_density` | A | infill modules |
| `bottom_surface_pattern` | A | infill modules |
| `top_shell_thickness` | B | object-level solid-fill planning (PrintObject.cpp) |
| `top_surface_density` | A | infill modules |
| `top_surface_pattern` | A | infill modules |

### Support / Advanced (Support)
| `bridge_no_support` | B | support-planner |
| `independent_support_layer_height` | B | support-planner |
| `max_bridge_length` | B | support-planner |
| `support_base_pattern` | B | support-planner |
| `support_base_pattern_spacing` | B | support-planner |

### Support / Interface
| `support_bottom_interface_spacing` | A | support-planner |
| `support_interface_loop_pattern` | A | support-planner |
| `support_interface_pattern` | A | support-planner |
| `support_interface_spacing` | A | support-planner |

### Support / Raft
| `raft_contact_distance` | A | support-planner |
| `raft_expansion` | A | support-planner |

### Support / Support
| `enforce_support_layers` | A | support-planner |
| `raft_first_layer_expansion` | A | support-planner |
| `support_bottom_z_distance` | A | support-planner |
| `support_critical_regions_only` | A | support-planner |
| `support_expansion` | A | support-planner |
| `support_object_first_layer_gap` | A | support-planner |
| `support_object_xy_distance` | A | support-planner |
| `support_remove_small_overhang` | A | support-planner |
| `support_style` | A | support-planner |
| `support_threshold_angle` | A | support-planner |
| `support_threshold_overlap` | A | support-planner |
| `support_type` | A | support-planner |

### Support / Support filament
| `support_filament` | B | support-planner |
| `support_interface_filament` | B | support-planner |
| `support_interface_not_for_body` | B | tool-ordering (ToolOrdering.cpp) |

### Support / Support ironing
| `support_air_filtration` | B | crates/slicer-gcode (air-filtration emission) |
| `support_ironing_pattern` | A | support-surface-ironing |
| `support_ironing` | A | support-surface-ironing | (ticket 07 reclassification — independent bool so support ironing no longer rides the shared `ironing_enabled`) |

### Support / Tree supports
| `tree_support_angle_slow` | B | tree-support |
| `tree_support_auto_brim` | B | tree-support |
| `tree_support_branch_angle_organic` | B | tree-support |
| `tree_support_branch_diameter_organic` | B | tree-support |
| `tree_support_branch_distance_organic` | B | tree-support |
| `tree_support_brim_width` | B | tree-support |
| `tree_support_tip_diameter` | B | tree-support |
| `tree_support_top_rate` | B | tree-support |
| `tree_support_with_infill` | X | out of scope — obsolete in canonical (IGNORE set) |
