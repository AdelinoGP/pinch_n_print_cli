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
ticket 03's ruling), and dead alternate spellings. 12 keys; the scoped
target is now **409** (403 + ticket 07's two reclassified ironing keys +
ticket 99's two fan-scale reclassifications + ticket 46's three
source-missing `*_filament_id` siblings − ticket 89's preset-management
`default_nozzle_volume_type`).

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
| B | 228 | new logic in an existing owner (incl. `ironing_type` +1 from 07; `fan_max_speed`/`fan_min_speed` +2 from 99; `top/bottom_surface_filament_id` + `inner_wall_filament_id` +3 from 46; − ticket 89's `default_nozzle_volume_type` to X) |
| C | 15 | new granular modules (Precision 8, interlocking 6, mmu-segmented-region 2, minus precise_z_height folded into layer-planner) |
| D | 47 | deferred — per-filament config model (58 minus 11 global keys now assignable) |
| X | 11 | out of scope (dead-in-canonical 6+2, preset-management 3) |
| **in scope** | **410** | |

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
| Quality / Walls and surfaces | B | crates/slicer-gcode (flow scaling, travel, ordering) + ticket 124 (`extruder` — **P80 dissolved, folded into 124** by ticket 87, 2026-09-10: canonical's `extruder` assigns objects/volumes to tools and normalises onto the six `*_filament_id` selectors (`apply_to_print_region_config` + `normalize_fdm`, `PrintObject.cpp`; ticket 46's runtime seam already resolves those), so a standalone packet would be declaration-only (rule 1); the per-object identity the assignment needs is 124's sequential-printing feature) | canonical `GCode::_extrude` mm3_per_mm scaling, `AvoidCrossingPerimeters.cpp`, `Print.cpp` per-object extruder |
| Quality / Wall generator — Arachne | A | arachne-perimeters | `min_feature_size` in Arachne/WallToolPaths.cpp |
| Quality / Line width | B | support-planner | `support_line_width` in Flow.cpp / TreeSupport.cpp (support flow) |
| Quality / Overhangs | B | slice-prepass | canonical `PrintObjectSlice.cpp::apply_conical_overhang` |
| Quality / Bridging | B (split) | pattern keys (2) → infill modules; bridge-over-infill (3) → host prepass qualification + InfillPostProcess construction seam; bridge-angle + flow (2) → perimeters/emitter; perimeter (1) → classic/arachne | canonical `Fill.cpp::Layer::make_fills`, `PrintObject.cpp::bridge_over_infill`, `GCode::_extrude`, `PerimeterGenerator::process_no_bridge`, `LayerRegion.cpp` |
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
| Printer / Machine / Power / recovery | A/B | crates/slicer-gcode | `disable_m73` consumed in emit.rs; canonical GCodeWriter::set_m73; envelope + recovery emission built by packet 267 (mixed A/B) |
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
| `max_layer_height` | B | tool-ordering (ToolOrdering.cpp calc_max_layer_height) — **blocked, unimplemented** by ticket 69: per-extruder `coFloats` vector (`0` = auto → `0.75 × nozzle_diameter[i]`) whose every live consumer rides a missing subsystem — tower partitions (purge-only tower, no partitions/idle layers/marking; sequences after ticket 122), skirt intermediate marking (skirt emits first-N-layers by count), slicing min/max envelope (uniform layer steps, no variable profile; canonical's own adaptive switch commented out); `Print.cpp::object_skirt_offset` is a named non-borrow (offset never reaches skirt gen — ticket 32; its live caller is ticket 124's validator). No per-extruder vector model (`nozzle_diameter` is an `extensions` scalar); re-filed as ticket 141, blocked on 122 + 125 |
| `min_layer_height` | B | layer-planner (Slicing.cpp min_layer_height_from_nozzle) — **blocked, unimplemented** by ticket 75: per-nozzle `coFloats` (default `{0.07}`) whose only live consumer is the `SlicingParameters` min/max envelope + adaptive profile clamp — the port's planner is uniform-only (no variable profile) and has no per-extruder vector model; re-filed as ticket 144, blocked on 06 + 125 (fold candidate with 141 at claim time — same envelope; kept separate now so it is not over-blocked behind 141's tower-body gate) |
| `overhang_fan_threshold` | A | part-cooling (emission-time cooling) |
| `reduce_fan_stop_start_freq` | A | part-cooling (emission-time cooling) |
| `support_material_interface_fan_speed` | A | part-cooling (emission-time cooling) |

### Extruder / Nozzle / Extruder geometry / mapping
| `extruder_ams_count` | B | tool-ordering (ToolOrdering.cpp calc_max_group_size) — **blocked, unimplemented** by ticket 70: machine-inventory `coStrings` (per-extruder `"<slots>#<count>"` tokens, default `{}`) whose live reads all sit in `build_filament_group_context` (group-slot capacity + machine filament inventory, `has_filament_switcher` override) feeding the absent `FilamentGroup.cpp` grouping scorer (ticket-39 `master_extruder_id` subject); `Print.cpp` gate entry is invalidation bookkeeping, `PrintApply`/`PresetBundle` are GUI/preset plumbing; zero tree occurrences, no per-extruder vector model; re-filed as ticket 142, blocked on 06 + 125 |
| `extruder_colour` | B | **covered** (ticket 39): canonical's only pipeline read is the CONFIG_BLOCK alias to `filament_colour` (`GCode::append_full_config`); the port emits the directive in HEADER_BLOCK + CONFIG_BLOCK with the authored palette when supplied |
| `extruder_offset` | B | — **blocked, unimplemented** by ticket 39: per-extruder XY offsets at emission (`GCode::point_to_gcode`, `WipeTowerIntegration::post_process_wipe_tower_moves` toolchange bridge move); the port has no offset term and no per-extruder vector model; re-filed as ticket 136, blocked on ticket 125 |
| `extruder_type` | B | — **blocked, unimplemented** by ticket 39: per-extruder bowden/direct feeding `ToolOrdering.cpp::build_filament_group_context` and `Print::update_filament_maps_to_config`; no such machinery in the tree; re-filed as ticket 136, blocked on ticket 125 |
| `extruder_variant_list` | B | — **blocked, unimplemented** by ticket 88: per-extruder `coStrings` vector (default `{"Direct Drive Standard"}`, dynamic-only) gating variant grouping (`DynamicPrintConfig::support_different_extruders`, `PrintConfig.cpp`) + fallback slot (`get_index_for_extruder`); zero tree occurrences, no per-extruder vector model or variant index maps; re-filed as ticket 146, blocked on ticket 125 |
| `filament_extruder_variant` | B | — **blocked, unimplemented** by ticket 88: per-filament-variant `coStrings` feeding `Print::get_filament_config_indx` + `Print::get_filament_unprintable_flow` (`Print.cpp`); zero tree occurrences; re-filed as ticket 146, blocked on ticket 125 |
| `filament_self_index` | B | — **blocked, unimplemented** by ticket 88: per-filament-variant `coInts` (default `{1}`) loaded by `Print::update_filament_self_index_cache` and consumed by `get_filament_config_indx` + `get_filament_unprintable_flow`; zero tree occurrences; re-filed as ticket 146, blocked on ticket 125 |
| `master_extruder_id` | B | — **blocked, unimplemented** by ticket 39: feeds the absent filament-grouping algorithm (`FilamentGroup.cpp`, `FilamentGroup::calc_group_by_kmedoids`); re-filed as ticket 136, blocked on ticket 125 |
| `physical_extruder_map` | B | — **blocked, unimplemented** by ticket 39: internal→physical T-index map at emission (`GCode::_do_export` reorder, placeholder seeds, `WipeTower` M104/M109 targets); port emits plain runtime tool indices; re-filed as ticket 136, blocked on ticket 125 |
| `print_extruder_id` | B | — **blocked, unimplemented** by ticket 88: per-process-variant `coInts` (default `{1}`) feeding `Print::get_nozzle_config_index` + `get_index_for_extruder` (`Print.cpp`); zero tree occurrences; re-filed as ticket 146, blocked on ticket 125 |
| `print_extruder_variant` | B | — **blocked, unimplemented** by ticket 88: per-process-variant `coStrings` feeding `Print::get_nozzle_config_index` + `get_index_for_extruder`; zero tree occurrences; re-filed as ticket 146, blocked on ticket 125 |
| `printer_extruder_id` | B | — **blocked, unimplemented** by ticket 39: shape key of per-extruder option arrays (`ParameterUtils.cpp::get_index_for_extruder_parameter`, `update_values_to_printer_extruders`); port has no per-extruder vector model; re-filed as ticket 136, blocked on ticket 125 |
| `printer_extruder_variant` | B | — **blocked, unimplemented** by ticket 39: same variant-array machinery + `Print::get_filament_unprintable_flow`; re-filed as ticket 136, blocked on ticket 125 |

### Extruder / Nozzle / MMU Hardware
| `cooling_tube_length` | B | wipe-tower (Type2 SEMM unload/load choreography) — **blocked, unimplemented** by ticket 28: dead in canonical's BBS `WipeTower` (`#if 0`, delegated to `change_filament_gcode`), live only in `WipeTower2::toolchange_Unload`, fused with Tier D per-filament ramming values and gated on `single_extruder_multi_material` + `enable_filament_ramming` (neither in this tree); re-filed as ticket 119, blocked on ticket 118 |
| `cooling_tube_retraction` | B | wipe-tower (Type2 SEMM unload/load choreography) — **blocked, unimplemented** by ticket 28: dead in canonical's BBS `WipeTower` (`#if 0`, delegated to `change_filament_gcode`), live only in `WipeTower2::toolchange_Unload`, fused with Tier D per-filament ramming values and gated on `single_extruder_multi_material` + `enable_filament_ramming` (neither in this tree); re-filed as ticket 119, blocked on ticket 118 |
| `extra_loading_move` | B | wipe-tower (Type2 SEMM unload/load choreography) — **blocked, unimplemented** by ticket 28: dead in canonical's BBS `WipeTower` (`#if 0`, delegated to `change_filament_gcode`), live only in `WipeTower2::toolchange_Load`, fused with Tier D per-filament ramming values and gated on `single_extruder_multi_material` + `enable_filament_ramming` (neither in this tree); re-filed as ticket 119, blocked on ticket 118 |
| `grab_length` | B | wipe-tower — **live** (ticket 40, direct implementation): declared on `wipe-tower.toml` (scalar float, default 0, min 0) and consumed by `WipeTower::purge_volume_for`, which subtracts `grab_length × 2.4` (canonical's `(diameter/2)^2*PI` cross-section) from the purge volume, clamped at 0 — canonical `GCode.cpp` toolchange path + `Print.cpp` wipe-tower planning. Scalar, not canonical's per-extruder `coFloats` (blocked on ticket 118); divergences in `DEV-170` |
| `high_current_on_filament_swap` | B | wipe-tower (Type2 SEMM unload/load choreography) — **blocked, unimplemented** by ticket 28: dead in canonical's BBS `WipeTower` (`#if 0`, delegated to `change_filament_gcode`), live only in `WipeTower2::toolchange_Load`, fused with Tier D per-filament ramming values and gated on `single_extruder_multi_material` + `enable_filament_ramming` (neither in this tree); re-filed as ticket 119, blocked on ticket 118 |
| `parking_pos_retraction` | B | wipe-tower (Type2 SEMM unload/load choreography) — **blocked, unimplemented** by ticket 28: dead in canonical's BBS `WipeTower` (`#if 0`, delegated to `change_filament_gcode`), live only in `WipeTower2::toolchange_Unload/Load`, fused with Tier D per-filament ramming values and gated on `single_extruder_multi_material` + `enable_filament_ramming` (neither in this tree); re-filed as ticket 119, blocked on ticket 118 |
| `start_end_points` | B | — **blocked, unimplemented** by ticket 40: canonical's only read site is `get_path_of_change_filament` (`GCode.cpp`), which computes the `travel_point_*` placeholders for `change_filament_gcode` from `start_end_points` + `bed_exclude_area` + object bounding boxes — and returns the safe default path when `bed_exclude_area.size() != 4`. `bed_exclude_area` is packet 256's scope (authored, not implemented), so wiring this key alone would be declaration-only (Authoring rule 1); the path computation also needs object bounding boxes at a seam that reaches the postpass substitution (new `ResolvedConfig` fields or extensions + `travel_point_*` schema on `machine-gcode-emit`). Missing feature: the filament-change travel path. Re-file when packet 256's implementation lands |

### Extruder / Nozzle / Nozzle
| `default_nozzle_volume_type` | X | out of scope — preset-management (ticket 89): the printer-profile side of the default/current pair; every canonical read is `PresetBundle` seeding / GUI-plate volume-map composition (`load_selections`, `reset_default_nozzle_volume_type`, `get_default_nozzle_volume_types_for_filaments`), zero slicing-pipeline decision points (`default_bed_type` precedent) |
| `nozzle_height` | B | skirt-brim (Print.cpp skirt/draft-shield height) |
| `nozzle_hrc` | B | — **blocked, unimplemented** by ticket 41: canonical's reads are all in `GCodeProcessor` (`apply_config` both overloads copy the scalar to every extruder; `update_slice_warnings` compares vs per-filament `required_nozzle_HRC` with the `Print::get_hrc_by_nozzle_type` fallback → non-fatal `NOZZLE_HRC_CHECKER` warning); the port has no warning-list seam and the comparison needs the per-tool axis; re-filed as ticket 137, blocked on ticket 125 |
| `nozzle_type` | B | — **blocked, unimplemented** by ticket 41: `coEnums` per-extruder vector (default `{ntUndefine}`), canonical's only role is the fallback-HRC source on the same `update_slice_warnings` path; no tree decision point; re-filed as ticket 137, blocked on ticket 125 |
| `nozzle_volume` | B | — **blocked, unimplemented** by ticket 41: `coFloats` per-extruder vector whose only behavioural effect is Elegoo-`M6211` flush attribution (`process_filaments` remaining-volume reset + `process_elegoo_M6211` statistics, ignored on non-Elegoo); no Elegoo seam and no per-tool ingestion in tree; re-filed as ticket 137, blocked on ticket 125 (vendor-scope ruling rides the re-file) |
| `nozzle_volume_type` | B | — **blocked, unimplemented** by ticket 71: per-extruder `coEnums` machine-inventory key (default `nvtStandard`), every live slicing consumer inside the absent multi-nozzle grouping subject (`ToolOrdering.cpp::build_nozzle_groups` / `build_default_nozzle_list` nozzle list + `add_volume_type_limits` unprintable-volume marking, feeding the `FilamentGroup.cpp` scorer — same subject as tickets 136/142); no grouping engine, no `NozzleGroupInfo`/`NozzleInfo` model, no per-extruder vector model in tree; re-filed as ticket 143, blocked on 06 + 125 (fold candidate with 136/142; sibling `default_nozzle_volume_type` stays ticket 89/P82) |
| `required_nozzle_HRC` | B | — **blocked, unimplemented** by ticket 41: `coInts` per-filament vector, the requirement side of canonical's `update_slice_warnings` HRC comparison; needs the per-tool axis (today `extract_float_or_first` keeps element 0); re-filed as ticket 137, blocked on ticket 125 |

### Extruder / Nozzle / Pressure advance
| `adaptive_pressure_advance` | B | — **unimplemented** by ticket 42: needs the AdaptivePAProcessor-style per-feature prediction (per-tool interpolators over the `adaptive_pressure_advance_model` flow/accel triplets, `process_layer` G-code post-pass, bridge/overhang overrides); no tree decision point; returned to the queue with the missing feature named |
| `adaptive_pressure_advance_bridges` | B | — **unimplemented** by ticket 42: static bridge override inside the same processor (`AdaptivePAProcessor.cpp` bridge branch, `coFloats` 0.0 max 2, 0 = follow walls); returned with the processor |
| `adaptive_pressure_advance_model` | B | — **unimplemented** by ticket 42: per-tool calibration triplets (`coStrings` `"0,0,0\n0,0,0"`, parsed + validated per `Print.cpp`); returned with the processor |
| `adaptive_pressure_advance_overhangs` | B | — **unimplemented** by ticket 42: in-feature flow-change arm of the same processor (`GCode.cpp` overhang branches); returned with the processor |
| `enable_pressure_advance` | B | — **live** by ticket 42: host emitter gates the PA prefix on this (`coBools` false) at print start + after every toolchange |
| `pressure_advance` | B | — **live** by ticket 42: host emitter emits the flavor-specific PA line for this value (`coFloats` 0.02 max 2) via `GcodeFlavor::set_pressure_advance`; per-tool values ride the existing `tool_config:<idx>:` axis, Orca vector ingest rides ticket 125 |

### Extruder / Nozzle / Retraction
| `deretraction_speed` | B | crates/slicer-gcode (GCode::retract) |
| `long_retractions_when_cut` | B | machine-gcode-emit placeholder seam — **in packet 276** by ticket 43: canonical's only reads are the export flag + `update_placeholder_parser_with_variant_params` publication, so the port publishes it through the module's existing substitution with a key-specific `1`/`0` guarantee (ADR-0050); scalar-global + `ResolvedConfig` field, vector model stays with ticket 125 |
| `long_retractions_when_ec` | B | — **returned to the queue, unimplemented** by ticket 43: `ConfigOptionBoolsNullable` with no geometric read site (placeholder publication only); the `_cut` wiring in packet 276 names its landing pattern when claimed |
| `retract_before_wipe` | B | crates/slicer-gcode (GCode::retract) — **shed to P37** by ticket 43: partitions retraction around wipe moves this tree does not emit yet; wiring it in 276 would be declaration-only (rule 1) |
| `retract_length_toolchange` | B | crates/slicer-gcode (GCode::retract) |
| `retract_lift_above` | B | crates/slicer-gcode (GCode::retract) |
| `retract_lift_below` | B | crates/slicer-gcode (GCode::retract) |
| `retract_lift_enforce` | B | crates/slicer-gcode (GCode::retract) |
| `retract_restart_extra` | B | crates/slicer-gcode (GCode::retract) |
| `retract_restart_extra_toolchange` | B | crates/slicer-gcode (GCode::retract) |
| `retract_when_changing_layer` | B | crates/slicer-gcode (GCode::retract) |
| `retraction_distances_when_cut` | B | machine-gcode-emit placeholder seam — **in packet 277** by ticket 44: canonical's reads are `append_tcr` / `update_placeholder_parser_with_variant_params` publication plus the `do_export` long-retraction flag (no cut motion is generated), so the port publishes it through the module's existing substitution with float spelling (276 `_cut`-bool precedent); scalar-global + `ResolvedConfig` field, vector model stays with ticket 125 |
| `retraction_distances_when_ec` | B | machine-gcode-emit placeholder seam — **in packet 277** by ticket 44: placeholder-only in canonical (`update_placeholder_parser_with_variant_params`; EC remains placeholder-only in `GCode.cpp`), published through the module's existing substitution with float spelling; canonical's null state is not represented (unset means default); scalar-global + `ResolvedConfig` field, vector model stays with ticket 125 |
| `retraction_minimum_travel` | B | crates/slicer-gcode (GCode::retract) |
| `travel_slope` | B | crates/slicer-gcode (GCode::travel) |
| `use_firmware_retraction` | B | crates/slicer-gcode (GCodeWriter::retract) |
| `wipe` | B | crates/slicer-gcode (GCode::retract) |
| `wipe_distance` | B | crates/slicer-gcode (GCode::retract) |
| `z_hop_types` | B | crates/slicer-gcode (GCode::travel) |
| `z_offset` | B | crates/slicer-gcode (GCode::travel) |

### Filament / Bed temperature
| `bed_temperature_formula` | B | — **blocked, unimplemented** by ticket 45: highest-vs-first-filament selector (`GCode::_print_first_layer_bed_temperature`, `GCode::process_layer`, `GCode::_do_export`) over per-filament `bed_temperature` vectors that do not exist in this tree (only PnP scalar `bed_temperature_initial_layer_single` in `machine-gcode-emit`); re-filed as ticket 138, blocked on ticket 125 |
| `cool_plate_temp` | D | deferred (per-filament config model) |
| `cool_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `curr_bed_type` | B | — **blocked, unimplemented** by ticket 45: plate-type selector (`GCode::get_highest_bed_temperature`, `GCode::_print_first_layer_bed_temperature`, `GCode::process_layer`, `GCode::_do_export`) over six Tier D plate-temperature vector pairs with zero occurrences in tree; re-filed as ticket 138, blocked on ticket 125 |
| `default_bed_type` | X | out of scope — no pipeline consumer (GUI Plater.cpp only, preset-management) |
| `eng_plate_temp` | D | deferred (per-filament config model) |
| `eng_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `hot_plate_temp` | D | deferred (per-filament config model) |
| `hot_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `supertack_plate_temp` | D | deferred (per-filament config model) |
| `supertack_plate_temp_initial_layer` | D | deferred (per-filament config model) |
| `support_chamber_temp_control` | X | out of scope — no pipeline consumer (GUI only, dead-in-canonical) |
| `support_multi_bed_types` | B | — **blocked, unimplemented** by ticket 85: gate over the absent plate-selection domain (`is_BBL_printer() \|\| support_multi_bed_types` on `Print::validate`'s filament-vs-plate arm, `get_bed_temp_key(curr_bed_type)` vector lookup over six Tier D plate pairs); folded into ticket 138 with P38's `curr_bed_type`, blocked on ticket 125 |
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
| `solid_infill_filament` → `internal_solid_filament_id` | B | runtime entity assembly (`assemble_ordered_entities_with_support_identities`, `crates/slicer-runtime/src/layer_executor.rs`) — **live** (ticket 46, direct implementation): canonical coInt 0=Default/inherit adopted as the declared name; explicit 1..N rebased to 0-based and clamped to tool count, resolved per entity by role below all paint-derived tools |
| `sparse_infill_filament` → `sparse_infill_filament_id` | B | runtime entity assembly — **live** (ticket 46, direct implementation, same seam) |
| `wall_filament` → `outer_wall_filament_id` | B | runtime entity assembly — **live** (ticket 46, direct implementation, same seam) |
| `top_surface_filament_id` | B | runtime entity assembly — **live** (ticket 46, direct implementation, same seam; never queued — absent from the gap source, the queue, and the tree; queue +1) |
| `bottom_surface_filament_id` | B | runtime entity assembly — **live** (ticket 46, direct implementation, same seam; never queued; queue +1) |
| `inner_wall_filament_id` | B | runtime entity assembly — **live** (ticket 46, direct implementation, same seam; never queued; queue +1) |
| `wipe_tower_filament` | B | wipe-tower (tower filament, global) — **blocked, unimplemented** by ticket 29: a selector over the tower's *finish extrusions* (`ToolOrdering::insert_wipe_tower_extruder`, `WipeTower2::first_toolchange_to_nonsoluble_nonsupport`), and this port's tower is purge-only — no shell/brim/infill, no idle-layer body, every path stamped `tool_index = tc.to_tool`; folded into ticket 122 (prime tower body parity), which the user ruled in scope at full canonical parity |

### Multimaterial / Flush options
| `filament_flush_temp` | B | machine-gcode-emit placeholder seam — **live** (ticket 47, direct implementation): scalar-global `ResolvedConfig` field (canonical default 0) + manifest row, published as-is through the module's generic `[key]` substitution into custom templates; per-tool via `tool_config:<idx>:` axis, Orca vector ingest rides 125. Owner re-derived from `crates/slicer-gcode` (ticket-27 hazard — canonical's reads are all `GCode.cpp` placeholder publication, and this tree's substitution lives in the module). Divergences in `DEV-171` (scalar not per-filament, 0 with no Tier D fallback, raw names not canonical's plural derived names) |
| `filament_flush_volumetric_speed` | B | machine-gcode-emit placeholder seam — **live** (ticket 47, direct implementation, same seam): scalar-global `ResolvedConfig` field (canonical default 0.0, max 200) + manifest row; same owner note and `DEV-171` |
| `flush_into_infill` | B | wipe-tower — **packet 294** (ticket 72): tier-table `tool-ordering` owner corrected — ordering ignores config and owns sequence only; the purge decision point is `WipeTower::purge_volume_for` |
| `flush_into_objects` | B | wipe-tower — **packet 294** (ticket 72, same owner correction) |
| `flush_into_support` | B | wipe-tower — **packet 294** (ticket 72, same owner correction) |
| `flush_multiplier` | B | wipe-tower — **live** (ticket 30, direct implementation): scales `flush_volumes_matrix` entries in `WipeTower::purge_volume_for`. Scalar, not canonical's per-extruder `coFloats` (blocked on ticket 118); divergences in `DEV-169` |
| `flush_volumes_matrix` | B | wipe-tower — **live** (ticket 30, direct implementation): flat row-major `N*N` per-pair purge volumes driving the purge-box depth and prime length in `WipeTower::generate_purge_paths`; falls back to `prime_volume` when unset; divergences in `DEV-169` |
| `flush_volumes_vector` | X | out of scope — preset-management metadata (03 class) |
| `wiping_volumes_extruders` | X | out of scope — dead in canonical |

### Multimaterial / Multimaterial advanced
| `interface_shells` | B | classic-perimeters (shell planning, PrintObject.cpp) — **in packet 301** by ticket 83 |
| `interlocking_beam` | C | new interlocking module |
| `interlocking_beam_layer_count` | C | new interlocking module |
| `interlocking_beam_width` | C | new interlocking module |
| `interlocking_boundary_avoidance` | C | new interlocking module |
| `interlocking_depth` | C | new interlocking module |
| `interlocking_orientation` | C | new interlocking module |
| `mmu_segmented_region_interlocking_depth` | C | new mmu-segmented-region module (consumed host-side in paint_segmentation) |
| `mmu_segmented_region_max_width` | C | new mmu-segmented-region module (consumed host-side in paint_segmentation) |
| `support_object_skip_flush` | B | crates/slicer-gcode (exclude-object emission) — **returned to queue, unimplemented** by ticket 48: both canonical reads (`GCode.cpp` sequential-toolchange + by-layer extrusion loop) are gated on `m_enable_exclude_object` (BBL + non-calib + exclude) and emit `M624` label codes — none of which exists in this tree (P44 / ticket 51 scope); wiring alone would be declaration-only (rule 1). Sequences after (or folds into) P44 when ticket 51 lands |

### Multimaterial / Ooze prevention
| `ooze_prevention` | B | crates/slicer-gcode (standby_temperature in serialize.rs) — **blocked, unimplemented** by ticket 49: standby arms need per-filament nozzle-temp vectors (Tier D) the host cannot see (its only temp field is the unrelated `filament_flush_temp`); re-filed as ticket 139, blocked on ticket 125 |
| `preheat_steps` | B | crates/slicer-gcode (standby_temperature in serialize.rs) — **blocked, unimplemented** by ticket 49: `GCodeProcessor` backtrace injector has no port seam (no usage-block builder, no XL concept, no filament count); re-filed as ticket 139, blocked on ticket 125 |
| `preheat_time` | B | crates/slicer-gcode (standby_temperature in serialize.rs) — **blocked, unimplemented** by ticket 49: same backtrace injector gap as `preheat_steps`; re-filed as ticket 139, blocked on ticket 125 |
| `standby_temperature_delta` | B | crates/slicer-gcode (standby_temperature in serialize.rs) — **blocked, unimplemented** by ticket 49: same missing base-temp source as `ooze_prevention` (`idle_temperature` is Tier D); re-filed as ticket 139, blocked on ticket 125 |

### Multimaterial / Prime tower
| `enable_filament_ramming` | A | wipe-tower |
| `enable_tower_interface_cooldown_during_tower` | A | wipe-tower |
| `enable_tower_interface_features` | A | wipe-tower |
| `filament_tower_interface_pre_extrusion_dist` | A | wipe-tower |
| `filament_tower_interface_pre_extrusion_length` | A | wipe-tower |
| `filament_tower_interface_print_temp` | A | wipe-tower |
| `filament_tower_interface_purge_volume` | A | wipe-tower |
| `filament_tower_ironing_area` | A | wipe-tower |
| `manual_filament_change` | B | crates/slicer-gcode (toolchange emission) + machine-gcode-emit (first-`change_filament_gcode` skip) — **live** (ticket 50, direct implementation): `ResolvedConfig` bool (default false) + serializer `; MANUAL_TOOL_CHANGE T<n>` tag line + module `FilamentChange` skip at 1-based count 1; later toolchanges unaffected |
| `prime_tower_brim_width` | A | wipe-tower |
| `prime_tower_enable_framework` | A | wipe-tower |
| `prime_tower_flat_ironing` | A | wipe-tower |
| `prime_tower_infill_gap` | A | wipe-tower |
| `prime_tower_skip_points` | A | wipe-tower |
| `purge_in_prime_tower` | A | wipe-tower |
| `single_extruder_multi_material` | A | wipe-tower |
| `single_extruder_multi_material_priming` | B | crates/slicer-gcode (toolchange emission) — **returned to queue, unimplemented** by ticket 50: every canonical read sits in `WipeTowerType::Type2` priming-tower flows (`initial_extruder` selection, `has_single_extruder_multi_material_priming` placeholder, `set_extruder` skip, `m_wipe_tower->prime()`), none of which exists in this tree (purge-only tower, ticket 29 census); sequences after ticket 122 (prime tower body parity), the same seat as `wipe_tower_filament` |
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
| `disable_m73` | A | crates/slicer-gcode (consumed in emit.rs) + declare in machine-gcode-emit manifest | (packet 267: declared in machine-gcode-emit.toml; gate already live) |
| `emit_machine_limits_to_gcode` | B | crates/slicer-gcode (envelope emission) | (packet 267 re-tier: new emitter logic — canonical `GCode::print_machine_envelope`) |
| `enable_power_loss_recovery` | B | crates/slicer-gcode (recovery emission) | (packet 267 re-tier: new emitter logic — canonical `GCodeWriter::enable_power_loss_recovery`) |
| `silent_mode` | — | returned to queue, unimplemented | (packet 267 ruling: needs a per-variant machine-limit model — canonical reads stride-2 normal/stealth `machine_max_*` pairs; PnP's scalar `Option<f32>` fields have no variant dimension. Follow-up ticket 117.) |

### Printer / Machine / Print volume
| `bed_exclude_area` | A | wipe-tower (bed_shape) + crates/slicer-gcode (printable_height) |
| `extruder_clearance_height_to_lid` | B | — **folded into ticket 124** by ticket 86: validator input (`sequential_print_clearance_valid`, `Print.cpp`; canonical default 120, min 0), zero-occurrence in tree; the TimelapsePosPicker rod/radius reads are a separable non-borrow (no picker seam in this tree) |
| `extruder_clearance_height_to_rod` | B | — **folded into ticket 124** by ticket 86: validator input (`sequential_print_clearance_valid`; canonical default 40, min 0), zero-occurrence in tree; the TimelapsePosPicker rod/radius reads are a separable non-borrow |
| `extruder_clearance_radius` | B | — **folded into ticket 124** by ticket 86: validator input (horizontal hull-inflation arm + `extruder_clearance_max_radius` legacy alias; canonical default 40, min 0), zero-occurrence in tree; the TimelapsePosPicker radius read is a separable non-borrow |
| `extruder_printable_area` | B | wipe-tower (multi-extruder shared print bed) — **returned to queue, unimplemented** by ticket 26: per-extruder polygon group, inert single-extruder; canonical's only behaviour path is `Print::get_extruder_shared_printable_polygon` → `WipeTower::set_shared_print_bed`; **adopted by ticket 119** (ticket 28), blocked on ticket 118 — `nozzle_diameter` is a scalar `f32` in `ResolvedConfig`, this port has no per-extruder vector model |
| `extruder_printable_height` | B | wipe-tower (multi-extruder last-layer validity) — **returned to queue, unimplemented** by ticket 26: per-extruder float vector, inert single-extruder; canonical's only behaviour path is `WipeTower::is_valid_last_layer`, gated on `m_is_multi_extruder`; **adopted by ticket 119** (ticket 28), blocked on ticket 118 — no per-extruder vector model in this port |
| `printable_height` | A | **implemented** by ticket 26 (2026-09-03): `ResolvedConfig::printable_height` → `validate_printable_height` (`crates/slicer-model-io/src/loader.rs`), called from `run_slice`; emitted from resolved config, shadowing the padding literal. Default 250.0 is a recorded deviation from canonical's 100.0 |

### Printer / Machine / Printer identity
| `allow_mix_temp` | X | out of scope — dead in canonical |
| `printer_model` | A | crates/slicer-gcode — **closed by ticket 27, no code change**: already live through `serialize_config_block`'s `raw_config.contains_key` synthesis guard; canonical's other pipeline reads are vendor-proprietary (out-of-scope class) |
| `printer_structure` | A | **owner corrected by ticket 27: machine-gcode-emit, not crates/slicer-gcode** — its only non-GUI canonical behaviour is the time-lapse injection gate in `GCode::process_layer`; implemented directly, divergences in `DEV-168` |
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
| `bridge_angle` | B | classic-perimeters + arachne-perimeters (LayerRegion.cpp + PerimeterGenerator.cpp) — **in packet 302** by ticket 84 |
| `bridge_density` | B | infill modules — **live** (ticket 34, direct implementation): divides bridge line spacing in `RectilinearInfill::run_infill` and `wave-overhangs`' fallback fill. `max` corrected 120→125 against the oracle. Not read by `gyroid-infill` (ticket 127); bare-number spelling misread (ticket 128) |
| `counterbore_hole_bridging` | B | classic-perimeters + arachne-perimeters — **in packet 302** by ticket 84 |
| `dont_filter_internal_bridges` | B | bridge-over-infill (host prepass qualification + InfillPostProcess construction seam, PrintObject.cpp) — **live** (ticket 82, no packet: multiplier 3/1 + partial-gate bypass in `gate_internal_bridge_sites` + short-line filter in the construction arm; bool `false` = canonical `ibfDisabled`) |
| `enable_extra_bridge_layer` | B | bridge-over-infill (host prepass qualification + InfillPostProcess construction seam, PrintObject.cpp) — **live** (ticket 82, no packet: carrier-free duplicate pass in `gate_internal_bridge_sites`; bool `false` = canonical `eblDisabled`) |
| `internal_bridge_angle` | B | bridge-over-infill (host prepass qualification + InfillPostProcess construction seam, PrintObject.cpp) — **live** (ticket 82, no packet: `angle_override` into `determine_bridging_angle`; default 0.0 = canonical automatic; range [0, 180] matches) |
| `internal_bridge_density` | B | infill modules — **live** (ticket 34, direct implementation): internal-bridge twin of `bridge_density`, selected off `is_internal_bridge` in both bridge-fill holders. Not read by `gyroid-infill` (ticket 127) |
| `internal_bridge_flow` | B | infill modules — **live** (ticket 57, no packet: already landed by ticket 34's direct implementation in both bridge-fill holders + host harvest; owner corrected from `crates/slicer-gcode` — the emitter must not scale, `flow_factor` already carries it) |
| `thick_internal_bridges` | B | infill modules — **live** (ticket 34, direct implementation): selects `canonical_bridging_flow`'s round-thread spacing for internal bridges in both bridge-fill holders. Canonical's second read site (`Print::validate`'s `allow_thin_bridge_width`) is config-range validation, deferred to ticket 113 |

### Quality / Ironing
| `ironing_angle` | A | top-surface-ironing + support-surface-ironing |
| `ironing_angle_fixed` | A | top-surface-ironing + support-surface-ironing |
| `ironing_inset` | A | top-surface-ironing + support-surface-ironing |
| `ironing_type` | B | top-surface-ironing + support-surface-ironing | (ticket 07 reclassification — enum modes unexpressible via the shared `ironing_enabled` bool; mode-selection logic) |

### Quality / Layer height
| `first_layer_print_sequence` | B | `crates/slicer-gcode` (emission stage; packet 295) |
| `first_layer_sequence_choice` | X | out of scope — dead alternate spelling |
| `other_layers_print_sequence` | B | `crates/slicer-gcode` (emission stage; packet 295) |
| `other_layers_print_sequence_nums` | B | `crates/slicer-gcode` (emission stage; packet 295) |
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
| `precise_z_height` | B | layer-planner (Slicing.cpp generate_object_layers) — **blocked, unimplemented** by ticket 77: live coBool (default false, per-object) whose last-5-layer redistribution clamps every adjusted height to the min/max envelope the port lacks (no ticket-27 hazard — `layer-planner-default`'s `generate_object_layers` is the direct analog); user ruling (grilled 2026-09-09) waits for the real envelope rather than a substitute clamp; re-filed as ticket 145, blocked on 141 + 144 |
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
| `bottom_solid_infill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 287** by ticket 61 |
| `extruder` | B | — **P80 dissolved, folded into ticket 124** by ticket 87 (2026-09-10): canonical's `extruder` (coInt, 0 = inherit) assigns objects/volumes to tools and normalises onto the six `*_filament_id` selectors (`apply_to_print_region_config`, `PrintObject.cpp`, plus `normalize_fdm` / `auto_assign_extruders` / 3MF/Model bookkeeping); this port already resolves those six per entity at runtime (ticket 46) and carries per-object `extruder` metadata only as `ResolvedConfig.extensions` — a standalone packet would be declaration-only (rule 1); the per-object identity the assignment needs is 124's sequential-printing feature |
| `first_layer_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 287** by ticket 61 |
| `gap_fill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 287** by ticket 61 |
| `inner_wall_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 287** by ticket 61 |
| `internal_solid_infill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 287** by ticket 61 |
| `is_infill_first` | B | — **returned to the queue, unimplemented** by ticket 61: per-region walls-vs-infill order (+ first-layer-always-walls exception) lives in runtime orchestration (`assemble_ordered_entities_with_support_identities`), not emission — the emitter preserves `ordered_entities` and must not reorder; wiring it in the emitter would be the wrong seam (ticket-27 hazard) |
| `max_travel_detour_distance` | B | — **returned to the queue, unimplemented** by ticket 61: zero-disables detour cap over a perimeter-avoiding planner the port does not have (path-optimization emits direct inter-region travel; the emitter consumes precomputed travels) — a limit with no planner would be declaration-only (rule 1); missing feature: avoid-crossing-perimeters planner |
| `outer_wall_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 287** by ticket 61 |
| `overhang_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 287** by ticket 61 (overhang selection via point-level `overhang_quartile` marking, DEV-179(b) — the port has no `OverhangPerimeter` role) |
| `print_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 288** by ticket 62 |
| `reduce_crossing_wall` | B | — **returned to the queue, unimplemented** by ticket 62: the enable for perimeter-avoiding travel (`AvoidCrossingPerimeters::travel_to` + `init_layer`, `GCode.cpp`) over a planner the port does not have (path-optimization emits direct inter-region travel; the emitter consumes precomputed travels — ticket-61's detour precedent); wiring the bool alone would be declaration-only (rule 1); missing feature: avoid-crossing-perimeters planner (shared with `max_travel_detour_distance`) |
| `set_other_flow_ratios` | B | crates/slicer-gcode (emission flow scaling) — **adopted into packet 287** by ticket 61 (split-boundary adjustment: the gate arms P54's ratios, so the decision lives in one place — P55 sheds it 9→8 with a backward dep) |
| `small_area_infill_flow_compensation` | B | crates/slicer-gcode (emission flow scaling) — **in packet 288** by ticket 62 |
| `small_area_infill_flow_compensation_model` | B | crates/slicer-gcode (emission flow scaling) — **in packet 288** by ticket 62 |
| `sparse_infill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 288** by ticket 62 |
| `support_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 288** by ticket 62 |
| `support_interface_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 288** by ticket 62 |
| `top_solid_infill_flow_ratio` | B | crates/slicer-gcode (emission flow scaling) — **in packet 288** by ticket 62 |

### Speed / Acceleration
| `accel_to_decel_enable` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `accel_to_decel_factor` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `bridge_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `default_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `initial_layer_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `inner_wall_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `internal_solid_infill_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `outer_wall_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `sparse_infill_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `top_surface_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |
| `travel_acceleration` | B | crates/slicer-gcode (per-entity accel selection + flavor M204/SET_VELOCITY_LIMIT) — **in packet 289** by ticket 63 |

### Speed / Advanced (Speed)
| `extrusion_rate_smoothing_external_perimeter_only` | B | crates/slicer-gcode (post-loop smoothing stage over `GCodeIR` moves) — **in packet 290** by ticket 64 |
| `max_volumetric_extrusion_rate_slope` | B | crates/slicer-gcode (post-loop smoothing stage over `GCodeIR` moves) — **in packet 290** by ticket 64 |
| `max_volumetric_extrusion_rate_slope_segment_length` | B | crates/slicer-gcode (post-loop smoothing stage over `GCodeIR` moves) — **in packet 290** by ticket 64 |

### Speed / Initial layer speed
| `slow_down_layers` | B | crates/slicer-gcode (per-entity slow-down blend in emit.rs over the feedrate.rs table) — **in packet 291** by ticket 65 |

### Speed / Jerk (XY)
| `default_jerk` | B | crates/slicer-gcode (per-entity jerk-selection stage in emit.rs) — **in packet 292** by ticket 66 |
| `default_junction_deviation` | B | crates/slicer-gcode (per-entity jerk-selection stage in emit.rs) — **in packet 292** by ticket 66 |
| `infill_jerk` | B | crates/slicer-gcode (per-entity jerk-selection stage in emit.rs) — **in packet 292** by ticket 66 |
| `initial_layer_jerk` | B | crates/slicer-gcode (per-entity jerk-selection stage in emit.rs) — **in packet 292** by ticket 66 |
| `inner_wall_jerk` | B | crates/slicer-gcode (per-entity jerk-selection stage in emit.rs) — **in packet 292** by ticket 66 |
| `outer_wall_jerk` | B | crates/slicer-gcode (per-entity jerk-selection stage in emit.rs) — **in packet 292** by ticket 66 |
| `top_surface_jerk` | B | crates/slicer-gcode (per-entity jerk-selection stage in emit.rs) — **in packet 292** by ticket 66 |
| `travel_jerk` | B | crates/slicer-gcode (per-entity jerk-selection stage in emit.rs) — **in packet 292** by ticket 66 |

### Speed / Other layers speed
| `internal_solid_infill_speed` | B | crates/slicer-gcode (internal-solid reseat in `resolve_feedrate` over the feedrate.rs table) — **in packet 293** by ticket 67 |
| `small_perimeter_speed` | B | crates/slicer-gcode (loop-length-gated small-loop `F` override at the per-entity site) — **in packet 293** by ticket 67 |

### Strength / Advanced (Strength)
| `align_infill_direction_to_model` | B | infill modules |
| `detect_narrow_internal_solid_infill` | B | infill modules |
| `ensure_vertical_shell_thickness` | B | object-level solid-fill planning (PrintObject.cpp) — **in packet 299** by ticket 80 |
| `extra_solid_infills` | B | object-level solid-fill planning (PrintObject.cpp) — **in packet 299** by ticket 80 |
| `infill_combination` | B | object-level infill planning (PrintObject.cpp) — **in packet 299** by ticket 80 |
| `infill_combination_max_layer_height` | B | object-level infill planning (PrintObject.cpp) — **in packet 299** by ticket 80 |
| `minimum_sparse_infill_area` | B | infill modules — **live** (ticket 35, direct implementation): small sparse islands convert to internal solid fill in the host `PrePass::ShellClassification` prepass, measured pre-wall-inset (strictly conservative). Islands land in the dedicated `internal_solid_fill` bucket (five-way fill partition; emitted as `InternalSolidInfill` by the `claim:top-fill` holders) |

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
| `support_interface_not_for_body` | B | slicer-runtime entity assembly (`assemble_ordered_entities_with_support_identities` over `SupportToolSelection`) — **live** (ticket 74, direct implementation): tier-table `tool-ordering` owner corrected — no `ToolOrdering` module exists here; canonical's `ToolOrdering::collect_extruders` + `GCode::process_layer` fallback maps to the runtime's support/interface tool resolution (ticket-38 seam). Scalar coBool default true; `WipingExtrusions::mark_wiping_extrusions` arm named non-borrow (no port analogue) |

### Support / Support ironing
| `support_air_filtration` | B | machine-gcode-emit (printer-level master enable on packet 253's header/footer exhaust emission) — **folded into packet 253** by ticket 68 (ticket-35 precedent: operator on a decision another packet builds; owner corrected — canonical's reads are `_do_export` header/footer, not a host-emitter speed) |
| `support_ironing_pattern` | ~~A~~ → **returned to queue, unimplemented** | support-surface-ironing (holder seam absent) | (ticket 22: algorithm-selecting enum over `InfillPattern` — canonical `Fill::new_from_type(support_params.ironing_pattern)`. Holder-only under Authoring rule 4 / grilling Q3(a), so it is never declared as an input key. This port has no support-ironing claim, no holder key, and no concentric filler; standing that seam up is Tier C, not the Tier A this row assumed. Missing feature: **support-ironing filler selection through a claim seam, shipping at least canonical's default `rectilinear` as a holder**. Scope it with packet `260b-support-interface-fill-claim-holders`.) |
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
