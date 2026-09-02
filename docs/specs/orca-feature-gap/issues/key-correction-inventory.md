# Key correction inventory — audit of rename tickets 99–107 and packets 253–266

- **Date:** 2026-09-01
- **HEAD:** `49c39a9005989d2527c0de58737401fbe4edaf0d` (branch `wayfinder/ticket-100-wipe-tower-rename`)
- **Working tree at audit time:** packet `263-infill-pattern-specific-keys` is mid-re-authoring
  (`design.md` / `implementation-plan.md` / `preflight-report.md` / `task-map.md` deleted, uncommitted);
  packet `266-top-surface-ironing-keys` is **untracked** (no commit yet). Both audited from the
  working tree, not from a commit.
- **Working tree moved during this audit (read this before trusting a packet path).** Packet
  directories are mutable shared state and two of them changed while the audit ran. At enumeration
  time `docs/spec_packets/262-infill-pattern-keys/` existed and was read; by the time the audit
  finished, `git status` showed it renamed and split into
  `docs/spec_packets/262a-infill-angle-and-multiline-keys/` (tracked rename) plus a new untracked
  `docs/spec_packets/262b-infill-pattern-holder-mapping/`. Packet 263's deleted files had also been
  restored and modified. **Every row in this inventory citing "packet 262" was derived from the
  pre-split `262-infill-pattern-keys` tables**, and the `262b` directory was never read. Re-derive
  packet paths and their tables at point of use; do not treat this document's packet directory names
  as current.
- **Canonical reference:** `../pinch_n_print_cli/OrcaSlicerDocumented/src/libslic3r/` (read-only sibling checkout).
- **Method:** read-only. `rg`/`grep`/`git` only; no `cargo build`, no `cargo test`. Every count below was
  derived by a search in this session; anything not derived is written `unverified`.

## Commits audited

Rename workstream (tickets 99–107):

| ticket | commit | subject |
|---|---|---|
| 99  | `dbf3449c` | part-cooling keys renamed to Orca names; P01 gains fan-scale gap keys |
| 100 | `e1da8b36` | rename wipe-tower keys to Orca names |
| 101 | `a716c17d` | rename path-optimization keys to Orca names |
| 102 | `3904c361` | rename classic-perimeters and seam keys to Orca names |
| 103 | `1be6a5af` | feat(fuzzy-skin): adopt Orca config keys |
| 104 | `f03d8408` | rename support/layer-planner keys to Orca names |
| 105 | `cd02cc32` | `infill_direction` rename; `resolution` re-adjudicated as gap |
| 106 | `d718eb6a` | ironing renames merged; `support_ironing_flow` default aligned to 10% |
| 107 | `f8606585`* | collapse infill duplicate spellings to Orca names |

\* the task brief listed this commit as `f8586585`, which `git rev-parse` rejects
(`fatal: Needed a single revision`). The real commit, resolved this session by subject line, is
**`f8606585`**. Corrected here.

Packet-authoring workstream (tickets 08–21):

| packet | commit |
|---|---|
| 253, 254, 255 | `0e6dadd3` |
| 256 | `3d621f82` |
| 257 | `79896f8e` |
| 258 | `06c551ca` |
| 259 | `4c32155c` |
| 260 | `d8b7e1fd` |
| 261 | `f4f5bcf6` |
| 262 | `d7b6b1f4` |
| 263, 264 | `725265ef` |
| 265 | `49c39a90` |
| 266 | *(untracked — no commit)* |

## Enumeration totals (all derived this session)

| source | count |
|---|---|
| distinct `[config.schema.*]` keys, `modules/core-modules/**/*.toml` | 170 |
| distinct `[config.schema.*]` keys incl. `modules/community-modules/` | 173 |
| `ORCA_CONFIG_PADDING` entries (`crates/slicer-gcode/src/serialize.rs`) | 69 |
| `SUPPORT_CONFIG_DEFAULTS` entries (same file) | 3 |
| `ResolvedConfig` declared fields (`crates/slicer-ir/src/resolved_config.rs`) | 69 (+`extensions`) |
| literal keys emitted by `ResolvedConfig::to_config_map` | 51 (+ all `extensions` keys) |
| keys named in packets 253–266 disposition tables | 107 |
| keys in the ticket 99–108 rename pool | 34 |
| **rows in this inventory** | **212** (211 distinct keys; `support_type` carries a second padding-twin row) |

## Counts by disposition

| disposition | count |
|---|---|
| OK | 91 |
| NOT-YET-BUILT (packet proposal, no code) | 72 |
| PADDING-ONLY | 17 |
| STUB | 12 |
| MECHANISM-VIOLATION | 9 |
| DEFAULT-MISMATCH | 9 |
| BROKEN-RENAME | 1 |
| DEAD-IN-CANONICAL | 1 |
| **total rows** | **212** |

Counts derived by reading the disposition column out of the table itself, not tallied by hand.
Several rows carry a second disposition in their evidence note (for example `fan_min_speed` is STUB
+ DEFAULT-MISMATCH, `support_base_pattern` is MECHANISM-VIOLATION + STUB); only the primary
disposition is counted above.

`NOT-YET-BUILT` is not one of the six required dispositions. It is used only for keys that exist
**solely** as a line in an unimplemented packet's disposition table — no manifest entry, no read site,
no padding entry. Calling those STUB would be wrong (nothing has been declared yet); the finding on
them is against the *packet*, and is carried in the "proposed action" and note columns. See
**Questions for grilling** Q1.

---

## Inventory

Sorted by owner, then key. Column notes:

- **in-tree read sites** — code under `modules/*/src/**` or `crates/*/src/**` that branches or computes
  on the value. Manifest lines, `tests/`, fixtures, padding entries, and pure CONFIG_BLOCK
  pass-throughs are excluded by construction.
- **canonical read site** — a consumer inside `libslic3r/`. `PrintConfig.cpp` declarations,
  `ConfigManipulation.cpp`, `Preset.cpp` lists, `Print.cpp::invalidate_state_by_config_options`
  opt-key lists, and `src/slic3r/` GUI code do **not** count.
- **default port / canonical** — `-` means the key is not declared in-tree, so there is no port default
  to compare.

### Owner: `part-cooling`

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `close_fan_the_first_x_layers` | `part-cooling.toml` | 99 `dbf3449c` | 2 — `part-cooling/src/lib.rs::PartCooling::from_config`, `::layer_fan_speed` | `GCode.cpp::_do_export` | 1 / coInts 1 | in-module branch | OK | no-change | Rename landed and the new spelling is read where the old one was. |
| `enable_overhang_bridge_fan` | `part-cooling.toml` | 99 `dbf3449c` | 3 — `part-cooling/src/lib.rs::PartCooling::from_config`, `::cooling_decision_for_event`, `::run_finalization` | `GCode.cpp::process_layer` | true / coBools true | in-module branch | OK | no-change | Ticket 99 claimed "true=1 match"; ticket 100 found the bool gate was vacuous. Re-derived here: both are `true`. Claim is correct, but it was correct by luck. |
| `fan_max_speed` | `part-cooling.toml` | 99 `dbf3449c` | 4 — `part-cooling/src/lib.rs::PartCooling::from_config`, `::layer_fan_speed`, `::cooling_decision_for_event`, `::run_finalization` | `CoolingBuffer.cpp::apply_layer_cooldown` (via `EXTRUDER_CONFIG`) | **255 / coFloats 100** | in-module branch | DEFAULT-MISMATCH | align-default | Scale mismatch, not an off-by-value: the port is raw PWM 0–255, canonical is percent 0–100. Ticket 99 saw this and deferred it to P01 rather than fixing it. |
| `fan_min_speed` | `part-cooling.toml` | 99 `dbf3449c` | **0 — none** | `CoolingBuffer.cpp::apply_layer_cooldown` (via `EXTRUDER_CONFIG`) | **51 / coFloats 20** | none | STUB | implement | Independently confirmed: `grep` for `fan_min_speed` across `part-cooling/src` returns nothing. Second disposition: DEFAULT-MISMATCH (same 0–255 vs 0–100 scale problem as `fan_max_speed`). Canonical uses it as the interpolation floor and as the `reduce_fan_stop_start_freq` idle value. |
| `overhang_fan_speed` | `part-cooling.toml` | 99 `dbf3449c` | 2 — `part-cooling/src/lib.rs::PartCooling::from_config`, `::cooling_decision_for_event` | `CoolingBuffer.cpp::apply_layer_cooldown` (via `EXTRUDER_CONFIG`) | 100 / coInts 100 | in-module branch | OK | no-change | Numerically equal, but the port's fan scale is 0–255 and canonical's is 0–100, so "100" does not mean the same thing on both sides. Flagged for grilling (Q4), not scored as a mismatch. |
| `slow_down_for_layer_cooling` | `part-cooling.toml` + `ORCA_CONFIG_PADDING` | 99 `dbf3449c` | **0 — none** | `CoolingBuffer.cpp::parse_layer_gcode` | true / coBools true | none | STUB | implement | Declared and padded, never read. Canonical gates the whole layer-time slowdown on it. |
| `slow_down_layer_time` | `part-cooling.toml` + `ORCA_CONFIG_PADDING` | 99 `dbf3449c` | **0 — none** | `CoolingBuffer.cpp::parse_layer_gcode`, `::calculate_layer_slowdown` | 5.0 / coFloats 5.0 | none | STUB | implement | Manifest default matches canonical, but the padding twin emits `8` — a third value that agrees with neither. See Q5. |
| `slow_down_min_speed` | `part-cooling.toml` | 99 `dbf3449c` | **0 — none** | `CoolingBuffer.cpp::parse_layer_gcode`, `::extruder_range_slow_down_non_proportional` | 10.0 / coFloats 10. | none | STUB | implement | Defaults match; there is simply no consumer. |

### Owner: `wipe-tower`

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `enable_prime_tower` | `wipe-tower.toml`, `ResolvedConfig` | 100 `e1da8b36` | 5 — `wipe-tower/src/lib.rs::WipeTower::from_config`, `::enabled`, `slicer-gcode/src/emit.rs::emit`, `slicer-runtime/src/run.rs` auto-enable | `Print.cpp::has_wipe_tower`, `ToolOrdering.cpp::insert_wipe_tower_extruder`, `GCode.cpp::set_extruder` | false / coBool false | host field + in-module gate | OK | no-change | Rename verified live on the new spelling; default flipped true to false to match canonical. |
| `prime_tower_width` | `wipe-tower.toml` + padding | 100 `e1da8b36` | 2 — `wipe-tower/src/lib.rs::WipeTower::from_config`, `::tower_width` | `WipeTower.cpp::WipeTower`, `WipeTower2.cpp::WipeTower2`, `Print.cpp::wipe_tower_data` | 60.0 / coFloat 60. | in-module | OK | no-change | — |
| `prime_volume` | `wipe-tower.toml` | 100 `e1da8b36` | 2 — `wipe-tower/src/lib.rs::WipeTower::from_config`, `::purge_volume` | `Print.cpp::_make_wipe_tower`, `ToolOrdering.cpp::reorder_extruders_for_minimum_flush_volume` | 45.0 / coFloat 45. | in-module | OK | no-change | Default moved 10.0 to 45.0 to adopt canonical. |
| `printable_area` | `wipe-tower.toml`, `ResolvedConfig` | 100 `e1da8b36` | 4 — `wipe-tower/src/lib.rs::WipeTower::from_config`, `::run_finalization`, `pnp-cli/src/visual_debug.rs`, `pnp-cli/src/visual_debug_gcode.rs` | `GCode.cpp::do_export`, `GCodeProcessor.cpp::apply_config` | interleaved float list / coPoints | host field | OK | no-change | Value-format divergence is real and known: canonical serialises point strings, the port an interleaved float list. Ticket 100 added `parse_orca_point_string` in `slicer-ir` so Orca 3MFs resolve. Adopting the name alone had broken 3MF ingestion. |
| `retract_length` (wipe-tower-owned) | `wipe-tower.toml`, `ResolvedConfig` | 100 / 101 | 3 — `wipe-tower/src/lib.rs::WipeTower::from_config`, `::retract_length`, `slicer-gcode/src/emit.rs` retract emission | canonical counterpart is `retract_length_toolchange`, not `retract_length` | 2.0 / n-a | in-module | OK | no-change | Deliberately not renamed: this is toolchange purge retraction. Ticket 101 ruled it out of the rename by owner. Its canonical twin `retract_length_toolchange` is unported and rides P36/P43. |
| `wipe_tower_x` | `wipe-tower.toml` + padding | 100 `e1da8b36` | 2 — `wipe-tower/src/lib.rs::WipeTower::from_config`, `::tower_x` | `WipeTower.cpp::WipeTower`, `PrintExtents.cpp::get_wipe_tower_extrusions_extents` | 15 / coFloats 15. | in-module | OK | no-change | — |
| `wipe_tower_y` | `wipe-tower.toml` + padding | 100 `e1da8b36` | 2 — `wipe-tower/src/lib.rs::WipeTower::from_config`, `::tower_y` | `WipeTower.cpp::WipeTower`, `GCode.cpp::process_layer` | 220 / coFloats 220. | in-module | OK | no-change | — |
| `wipe_tower_max_purge_speed` | `FeedrateConfig` (`slicer-ir/src/feedrate.rs`), `SPEED_KEYS` | 108 | 2 — `slicer-gcode/src/emit.rs::resolve_feedrate`, `slicer-ir/src/feedrate.rs::SPEED_KEYS` | canonical `wipe_tower_max_purge_speed` — `WipeTower.cpp::WipeTower`, `WipeTower2.cpp::toolchange_Wipe` | 90.0 / coFloat 90. | host field | OK | implemented | Renamed and live. `resolve_feedrate` caps wipe-tower paths at `min(max_purge_speed, sparse_infill_speed)`; canonical min-10 validation is deferred to ticket 113. |
| `wipe_tower_no_sparse_layers` | `ORCA_CONFIG_PADDING` only | 255 | 0 — none | `WipeTower.cpp::WipeTower`, `GCode.cpp::WipeTowerIntegration::tool_change` | padding 0 / coBool false | none | PADDING-ONLY | shed-to-queue | Packet 255 records it as "declared + emitted; gap: no opposite behaviour" — a rule-2 padding twin counted as coverage. |
| `wipe_tower_rotation_angle` | `ORCA_CONFIG_PADDING` only | 255 | 0 — none | `WipeTower.cpp::WipeTower`, `Print.cpp::first_layer_wipe_tower_corners` | padding 0 / coFloat 0. | none | PADDING-ONLY | shed-to-queue | Same. Port tower is axis-aligned only. |
| `prime_tower_brim_width` | `ORCA_CONFIG_PADDING` only | 254 | 0 — none | `WipeTower.cpp::WipeTower`, `Print.cpp::wipe_tower_data` | padding 3 / coFloat 3. | none | PADDING-ONLY | shed-to-queue | Same. |
| `single_extruder_multi_material` | `ORCA_CONFIG_PADDING` only | 255 | 0 — none | `GCode.cpp::_do_export`, `Print.cpp::_make_wipe_tower` | padding 1 / coBool true | none | PADDING-ONLY | shed-to-queue | Same. |
| `enable_filament_ramming` | packet 254 only | 254 | 0 — none | `WipeTower2.cpp::WipeTower2` | - / coBool true | none | NOT-YET-BUILT | shed-to-queue | Packet disposition is "declared + emitted; decision-point gap" — prohibited by authoring rule 1. |
| `enable_tower_interface_cooldown_during_tower` | packet 254 only | 254 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation ("gap: no interface-temp machinery"). Canonical liveness not checked this session. |
| `enable_tower_interface_features` | packet 254 only | 254 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. Canonical liveness unverified. |
| `filament_tower_interface_pre_extrusion_dist` | packet 254 only | 254 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation; also declared "scalar-global" for a per-filament key. See Q7. |
| `filament_tower_interface_pre_extrusion_length` | packet 254 only | 254 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Same. |
| `filament_tower_interface_print_temp` | packet 254 only | 254 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Same. |
| `filament_tower_interface_purge_volume` | packet 254 only | 254 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Same. |
| `filament_tower_ironing_area` | packet 254 only | 254 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Same. |
| `prime_tower_enable_framework` | packet 254 only | 254 | 0 — none | `WipeTower.cpp::WipeTower` | - / coBool false | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `prime_tower_flat_ironing` | packet 254 only | 254 | 0 — none | `WipeTower.cpp::WipeTower` | - / coBool false | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `prime_tower_infill_gap` | packet 254 only | 254 | **0 — none** | `WipeTower.cpp::WipeTower`, `Print.cpp::wipe_tower_data` | - / coPercent 150 | none | NOT-YET-BUILT | implement | **Packet 254 claims this key is "WIRED to scan-line advance". Verified false against the tree: zero reads, not declared in any manifest.** See Q2. |
| `prime_tower_skip_points` | packet 254 only | 254 | 0 — none | `WipeTower.cpp::WipeTower`, `GCode.cpp::WipeTowerIntegration::append_tcr` | - / coBool true | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `purge_in_prime_tower` | packet 255 only | 255 | 0 — none | `ToolOrdering.cpp::reorder_extruders_for_minimum_flush_volume`, `Print.cpp::_make_wipe_tower` | - / coBool true | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `wipe_tower_bridging` | packet 255 only | 255 | 0 — none | `WipeTower2.cpp::WipeTower2` | - / coFloat 10. | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `wipe_tower_cone_angle` | packet 255 only | 255 | 0 — none | `WipeTower2.cpp::WipeTower2`, `Print.cpp::_make_wipe_tower` | - / coFloat 30.0 | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `wipe_tower_extra_flow` | packet 255 only | 255 | **0 — none** | `WipeTower2.cpp::WipeTower2` | - / coPercent 100. | none | NOT-YET-BUILT | implement | **Packet 255 claims "WIRED to purge paths' flow_factor; identity at defaults". Verified false: zero reads, not declared.** The packet's own hedge ("identity at defaults") is itself the rule-6(b) failure — an AC whose only evidence is default-path identity. See Q2. |
| `wipe_tower_extra_rib_length` | packet 255 only | 255 | 0 — none | `WipeTower.cpp::WipeTower`, `Print.cpp::wipe_tower_data` | - / coFloat 0 | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `wipe_tower_extra_spacing` | packet 255 only | 255 | 0 — none | `WipeTower2.cpp::WipeTower2` | - / coPercent 100. | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `wipe_tower_fillet_wall` | packet 255 only | 255 | 0 — none | `WipeTower.cpp::WipeTower`, `Print.cpp::_make_wipe_tower` | - / coBool true | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `wipe_tower_rib_width` | packet 255 only | 255 | 0 — none | `WipeTower.cpp::WipeTower`, `Print.cpp::wipe_tower_data` | - / coFloat 8 | none | NOT-YET-BUILT | shed-to-queue | Rule-1 violation. |
| `wipe_tower_wall_type` | packet 255 only | 255 | 0 — none | `WipeTower.cpp::WipeTower`, `WipeTower2.cpp::use_gap_wall` | - / coEnum wtwRib | packet proposes an enum on one module | MECHANISM-VIOLATION | re-route-through-claims | Enum selecting wall-geometry algorithms (rib / cone / fillet / gap). Rule 4 makes these `claim:*` holders, not an enum parked on `wipe-tower`. |
| `bed_exclude_area` | packet 256 only | 256 `3d621f82` | **0 — none** | `GCode.cpp::get_path_of_change_filament`, `TimelapsePosPicker.cpp::construct_printable_area_by_printer` | - / coPoints {(0,0)} | none | NOT-YET-BUILT | implement | **Packet 256 claims "WIRED at bed-validation decision point". Verified false: zero reads, not declared in any manifest.** See Q2. |

### Owner: `path-optimization-default` / `layer-planner-default` / `seam`

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `retraction_length` | `path-optimization-default.toml` | 101 `a716c17d` | 2 — `path-optimization-default/src/lib.rs::from_config`, `::run_path_optimization` | `Extruder.cpp::retraction_length`, `GCode.cpp::set_extruder` | 0.8 / coFloats 0.8 | in-module | OK | no-change | Rename verified live on the new spelling. |
| `retraction_speed` | `path-optimization-default.toml` | 101 `a716c17d` | 2 — `path-optimization-default/src/lib.rs::from_config`, `::run_path_optimization` | `Extruder.cpp::retract_speed`, `WipeTower.cpp::set_extruder` | 30.0 / coFloats 30. | in-module | OK | no-change | Default aligned 25.0 to 30.0 by ticket 101. |
| `z_hop` | `path-optimization-default.toml` | 101 `a716c17d` | 2 — `path-optimization-default/src/lib.rs::from_config`, `::run_path_optimization` | `GCodeWriter.cpp::lazy_lift`, `::eager_lift`, `GCode.cpp::needs_retraction` | 0.4 / coFloats 0.4 | in-module | OK | no-change | Default aligned 0.0 to 0.4; canonical range min 0 / max 5 adopted. |
| `retract_mode` | `path-optimization-default.toml` | 101 `a716c17d` | 2 — `path-optimization-default/src/lib.rs::from_config`, `::run_path_optimization` | none found (canonical spells lift behaviour as `z_hop_types` / `retract_lift_enforce`) | unverified / n-a | in-module enum | OK | no-change | PnP-specific spelling; not part of the ticket-101 rename set. Canonical counterpart unverified this session. |
| `path_optimization_emit_layer_markers` | `path-optimization-default.toml` | — | 1 — `path-optimization-default/src/lib.rs::from_config` (marker-emission branch) | n/a — PnP-invented | unverified / n-a | in-module | OK | no-change | PnP-specific diagnostic key, out of the parity scope. |
| `initial_layer_print_height` | `layer-planner-default.toml`, `ResolvedConfig` | 104 `f03d8408` | 5 — `layer-planner-default/src/lib.rs::from_config`, `::run_layer_planning`, `slicer-gcode/src/emit.rs::emit`, `slicer-core/src/algos/region_mapping.rs` overlay merge | `Flow.cpp::support_material_1st_layer_flow`, `GCode.cpp::_do_export` | 0.2 / coFloat 0.2 | host field + in-module | OK | no-change | Manifest default corrected 0.3 to 0.2; host `ResolvedConfig` and live behaviour were already 0.2, so this was doc-only with no slice delta. |
| `seam_position` | `seam-placer.toml`, `seam-planner-default.toml`, padding | 102 `3904c361` | 2 — `seam-placer/src/lib.rs` mode resolution, `seam-planner-default/src/lib.rs` mode resolution | `SeamPlacer.cpp::init`, `::place_seam` | "aligned" / coEnum spAligned | in-module enum, 5 live values | OK | no-change | Read directly this session: `from_config` branches over nearest / rear / random / aligned / aligned_back and hard-errors on anything else. This is a genuine live enum, not a stub. Whether seam modes should be claim holders under rule 4 is a judgement call — see Q8. |

### Owner: `classic-perimeters` / `arachne-perimeters`

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `wall_loops` | both perimeter manifests, `wave-overhangs.toml`, `ResolvedConfig`, padding | 102 `3904c361` | 5 — `classic-perimeters/src/lib.rs::from_config`, `::run_perimeters`, `arachne-perimeters/src/lib.rs::arachne_params_from_config`, `wave-overhangs/src/lib.rs::from_config` | `PerimeterGenerator.cpp::split_top_surfaces`, `LayerRegion.cpp::process_external_surfaces`, `Fill.cpp::make_ironing` | 2 / coInt 2 | host field + in-module | OK | no-change | Manifest default corrected 3 to 2 (host `ResolvedConfig` was already 2). Real behaviour change at defaults: one outer + one inner wall. |
| `small_perimeter_threshold` | `classic-perimeters.toml` | 102 `3904c361` | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `::emit_walls` | `GCode.cpp::extrude_loop` | 0.0 / coFloats 0 | in-module | OK | no-change | Default corrected 0.8 to 0.0; narrow-island inner-width override is now off at defaults. |
| `smaller_perimeter_line_width` | `classic-perimeters.toml` | 102 `3904c361` | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `::emit_walls` | none found in `libslic3r/` | unverified / n-a | in-module | OK | no-change | PnP-invented companion to `small_perimeter_threshold`. Not a parity key. |
| `narrow_loop_length_threshold_mm` | `classic-perimeters.toml` | — | 1 — `classic-perimeters/src/lib.rs::run_perimeters` (loop-length filter) | none found in `libslic3r/` | unverified / n-a | in-module | OK | no-change | PnP-invented. |
| `precise_outer_wall` | both perimeter manifests | 100 `e1da8b36` | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::arachne_params_from_config` | `PerimeterGenerator.cpp::process_arachne`, `::process_classic` | **false / coBool true** | in-module | DEFAULT-MISMATCH | align-default | Ticket 100 found the deviation and deliberately did **not** flip it, because the port's precise-outer-wall path changes wall ordering. Recorded as DEV-158, alignment deferred to packet work. Still open. |
| `detect_thin_wall` | `classic-perimeters.toml`, `arachne-perimeters.toml`, padding | 100 `e1da8b36` | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::arachne_params_from_config` | `PerimeterGenerator.cpp::process_classic`, `Layer.cpp::is_perimeter_compatible` | false / coBool false | in-module | OK | no-change | Not a rename — ticket 100 aligned the classic-perimeters default true to false (arachne was already false). Padding twin still emits `1`, disagreeing with both. See Q5. |
| `wall_sequence` | both perimeter manifests | 102 `3904c361` | 3 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::arachne_params_from_config`, `::run_perimeters` | unverified | unverified / unverified | in-module enum | OK | no-change | Live on both perimeter generators. Canonical counterpart (`wall_sequence` / `is_infill_first`) not checked this session. |
| `infill_wall_overlap` | `classic-perimeters.toml` | 107 `f8606585` | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `::emit_walls` | `PerimeterGenerator.cpp::process_classic`, `::process_arachne`, `::add_infill_contour_for_arachne` | unverified / coPercent 15 | in-module | OK | no-change | Ticket 107 re-adjudicated: this is the real canonical key and it was already ported. The port default was not re-derived this session — **unverified**. |
| `top_bottom_infill_wall_overlap` | `classic-perimeters.toml` | — | 1 — `classic-perimeters/src/lib.rs::run_perimeters` (overlap-key selection) | unverified | unverified / unverified | in-module | OK | no-change | Live. Canonical counterpart unverified. |
| `overhang_reverse` | `arachne-perimeters.toml`, `classic-perimeters.toml` | — | 1 — `arachne-perimeters/src/lib.rs::run_perimeters` (`g7_reverse`) | `PerimeterGenerator.cpp` | unverified / unverified | in-module | OK | no-change | Live on arachne only; the classic-perimeters declaration has no read. |
| `overhang_reverse_internal_only` | `arachne-perimeters.toml`, `classic-perimeters.toml` | — | **0 — none** | `PerimeterGenerator.cpp`, `PrintObject.cpp` | unverified / unverified | none | STUB | implement | Bound in `arachne-perimeters/src/lib.rs` to `_overhang_reverse_internal_only` — the leading underscore is the compiler silencing an unused binding. Declared on two manifests, read by neither. Not touched by any audited commit; found by the broad sweep. |
| `overhang_reverse_threshold` | `arachne-perimeters.toml` | — | **0 — none** | `Layer.cpp`, `PerimeterGenerator.cpp` | unverified / unverified | none | STUB | implement | Same underscore-bound-and-dropped pattern. Found by the broad sweep. |
| `detect_overhang_wall` | both perimeter manifests, padding | — | 1 — `arachne-perimeters/src/lib.rs::run_perimeters` (`g7_reverse`) | unverified | unverified / unverified | in-module | OK | no-change | Live on arachne; the classic-perimeters declaration has no read. |
| `only_one_wall_top` | both perimeter manifests | — | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::run_perimeters` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `only_one_wall_first_layer` | both perimeter manifests | — | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::run_perimeters` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `alternate_extra_wall` | both perimeter manifests | — | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::run_perimeters` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `min_width_top_surface` | both perimeter manifests | — | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::emit_only_one_wall_top_second_pass` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `extra_perimeters` | both perimeter manifests, padding | — | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs` (`max_bead_count`) | unverified | unverified / unverified | in-module | OK | no-change | — |
| `extra_perimeters_on_overhangs` | both perimeter manifests, padding | — | 1 — `classic-perimeters/src/lib.rs::run_perimeters` | unverified | unverified / unverified | in-module | OK | no-change | Declared on arachne too, read only by classic. |
| `spiral_vase` | both perimeter manifests | — | 2 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::run_perimeters` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `filter_out_gap_fill` | `classic-perimeters.toml`, padding | — | 1 — `classic-perimeters/src/lib.rs::run_perimeters` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `gap_fill_medial_axis_on_painted` | `classic-perimeters.toml` | — | 1 — `classic-perimeters/src/lib.rs::run_perimeters` | n/a — PnP-invented | unverified / n-a | in-module | OK | no-change | — |
| `slice_has_paint` | `classic-perimeters.toml` | — | 1 — `classic-perimeters/src/lib.rs::run_perimeters` | n/a — pipeline output, not a user key | unverified / n-a | in-module | OK | no-change | — |
| `gap_fill_target` | `ORCA_CONFIG_PADDING` only | 262 | **0 — none** | `FillBase.cpp` | padding "nowhere" / unverified | none | PADDING-ONLY | shed-to-queue | Packet 262 records it "Declared-with-gap — no fill-side gap fill". A padding twin is being counted toward parity, which rule 2 prohibits. |
| `slowdown_for_curled_perimeters` | `overhang-classifier-default.toml` | 100 `e1da8b36` | 1 — `overhang-classifier-default/src/lib.rs::from_config` | `GCode.cpp::_extrude` | **true / coBools false** | in-module | DEFAULT-MISMATCH | align-default | Not a rename. Ticket 100 changed the port default false to true, described as "aligned to Orca". **Verified false this session:** the canonical declaration `add("slowdown_for_curled_perimeters", coBools)` sets `new ConfigOptionBoolsNullable{ false }`, and the port manifest now reads `default = true`. The alignment ran in the wrong direction — this is exactly the class of vacuous boolean check ticket 100 was filed to fix. See Q9. |

### Owner: `fuzzy-skin`

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `fuzzy_skin_thickness` | `fuzzy-skin.toml`, padding | 103 `1be6a5af` | 2 — `fuzzy-skin/src/lib.rs::FuzzySkin::from_config`, displacement generator | `FuzzySkin.cpp::group_region_by_fuzzify`, `PrintObject.cpp::region_config_from_model_volume` | 0.2 / coFloat 0.2 | in-module | OK | no-change | Rename `thickness` to `fuzzy_skin_thickness` verified live on the new spelling; default aligned 0.3 to 0.2. |
| `fuzzy_skin_point_distance` | `fuzzy-skin.toml`, padding | 103 `1be6a5af` | 2 — `fuzzy-skin/src/lib.rs::FuzzySkin::from_config`, displacement generator | `FuzzySkin.cpp::group_region_by_fuzzify`, `PrintObject.cpp::region_config_from_model_volume` | 0.3 / coFloat 0.3 | in-module | OK | no-change | Default aligned 0.5 to 0.3; the stale `from_config` fallback of 0.8 was reconciled at the same time. |
| `apply_to_all` | `fuzzy-skin.toml` | 103 `1be6a5af` | 3 — `fuzzy-skin/src/lib.rs::FuzzySkin::from_config`, `::run` per-wall gate, displacement generator | none found in `libslic3r/` | unverified / n-a | in-module | OK | no-change | Ticket 103 ruled it PnP-specific and left the name alone. Its canonical role is played by the `fuzzy_skin` enum (`external` vs `all`), which is unported — see the `fuzzy_skin` row. **Naming hazard:** the key is spelled `apply_to_all`, with no `fuzzy_skin_` prefix, in a shared config namespace. See Q10. |
| `fuzzy_skin` | `ORCA_CONFIG_PADDING` only | 259 `4c32155c` | **0 — none** | `FuzzySkin.cpp` | padding "none" / unverified | none | PADDING-ONLY | implement | **Packet 259 claims "Wired (loop-selection gate)". Verified false: zero reads, not declared in any manifest.** The only in-tree hits are the padding entry and an unrelated paint-semantic string literal in `classic-perimeters/src/lib.rs`. This is the master on/off + scope enum for the whole feature. See Q2. |
| `fuzzy_skin_first_layer` | packet 259 only | 259 `4c32155c` | **0 — none** | `FuzzySkin.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | implement | **Packet 259 claims "Wired (layer gate)". Verified false: zero reads, not declared.** See Q2. |
| `fuzzy_skin_mode` | `ORCA_CONFIG_PADDING` only | 259 `4c32155c` | 0 — none | `FuzzySkin.cpp` (via `PrintObject.cpp::region_config_from_model_volume`) | padding "displacement" / unverified | none | PADDING-ONLY | shed-to-queue | Packet 259: "Declared-with-gap — no Arachne junction path". Rule-1 and rule-2 violation together. |
| `fuzzy_skin_noise_type` | packet 259 only | 259 `4c32155c` | 0 — none | `FuzzySkin.cpp`, `PrintObject.cpp` | - / unverified | packet proposes an enum on one module | MECHANISM-VIOLATION | re-route-through-claims | Packet's own words: "Declared-with-gap — five non-classic modules unimplemented". This enum selects among *noise algorithms* (perlin, billow, ridged, voronoi, …). The map names `fuzzy_skin_noise_type` explicitly in rule 4 as a set of `claim:*` holders, one per shipped value. |
| `fuzzy_skin_octaves` | packet 259 only | 259 `4c32155c` | 0 — none | `FuzzySkin.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | shed-to-queue | "Declared-with-gap — unused by classic". Only meaningful once a noise-type claim holder exists; ships with it or not at all. |
| `fuzzy_skin_persistence` | packet 259 only | 259 `4c32155c` | 0 — none | `FuzzySkin.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Same. |
| `fuzzy_skin_scale` | packet 259 only | 259 `4c32155c` | 0 — none | `FuzzySkin.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Same. |

### Owner: `top-surface-ironing` / `support-surface-ironing`

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `ironing_enabled` | both ironing manifests | 106 `d718eb6a` | 2 — `top-surface-ironing/src/lib.rs::from_config`, `support-surface-ironing/src/lib.rs::from_config` | none — canonical has no `ironing_enabled` | false / n-a | in-module gate | OK | no-change | Ticket 106 ruled it PnP-specific. It is the port's stand-in for canonical's `ironing_type` enum and `support_ironing` bool. Keeping both a PnP bool and the canonical enum would give two gates for one decision — see Q11. |
| `ironing_flow` | `top-surface-ironing.toml` | 106 `d718eb6a` | 2 — `top-surface-ironing/src/lib.rs::from_config`, `::generate_zigzag_strokes_for_polygon` | `Fill.cpp::make_ironing` | 0.10 / coPercent 10 | in-module | OK | no-change | Port stores the fraction 0.10 where canonical stores the percent 10. Same quantity, different convention — consistent with the `support_ironing_flow` fix. |
| `ironing_spacing` | `top-surface-ironing.toml` | 106 `d718eb6a` | 2 — `top-surface-ironing/src/lib.rs::from_config`, `::generate_zigzag_strokes_for_polygon` | `Fill.cpp::make_ironing` | 0.1 / coFloat 0.1 | in-module | OK | no-change | Renamed from `ironing_spacing_mm`; verified live on the new spelling. |
| `ironing_speed` | both ironing manifests, `FEEDRATE_KEYS` | 106 `d718eb6a` | 3 — `top-surface-ironing/src/lib.rs::from_config`, `support-surface-ironing/src/lib.rs::from_config`, `slicer-ir/src/feedrate.rs::FEEDRATE_KEYS` | `Fill.cpp::make_ironing`, `GCode.cpp::_extrude` | **top 20.0 / support 30.0 / coFloat 20** | host feedrate + in-module | DEFAULT-MISMATCH | align-default | One canonical key, two port manifests, two different defaults. `top-surface-ironing` matches canonical at 20.0; `support-surface-ironing` declares 30.0. Canonical derives support ironing speed inside `SupportParameters`, not from a second `ironing_speed` default. See Q12. |
| `ironing_pattern` | `top-surface-ironing.toml`, padding | 106 | 1 — `top-surface-ironing/src/lib.rs::from_config` (validation only) | `Fill.cpp::make_ironing` | "rectilinear" / coEnum ipRectilinear | rejects every value but one | MECHANISM-VIOLATION | re-route-through-claims | Read this session: `from_config` accepts `"rectilinear"` and hard-errors on anything else. That is a validation gate, not a decision point — no value changes behaviour, so it fails rule 1 despite having a nominal read. Canonical selects a real `InfillPattern` here. Should be claim holders, one per shipped pattern. |
| `support_ironing_flow` | `support-surface-ironing.toml` | 106 `d718eb6a` | 2 — `support-surface-ironing/src/lib.rs::from_config`, `::fill_expolygon` | `SupportParameters.hpp::SupportParameters`, `SupportCommon.cpp::generate_support_toolpaths` | 0.10 / coPercent 10 | in-module | OK | no-change | Ticket 106's most consequential fix: the port previously declared `100.0` on a `[1.0, 200.0]` range and multiplied flow directly, so at defaults it emitted **100x** nominal flow. Now 0.10 on `[0.01, 1.0]`. |
| `support_ironing_spacing` | `support-surface-ironing.toml` | 106 `d718eb6a` | 2 — `support-surface-ironing/src/lib.rs::from_config`, `::fill_expolygon` | `SupportParameters.hpp::SupportParameters`, `SupportCommon.cpp::generate_support_toolpaths` | 0.1 / coFloat 0.1 | in-module | OK | no-change | Renamed from `ironing_spacing`; ticket 106 sequenced this before the top-surface rename to avoid crossing wires. Verified live on the new spelling. |
| `ironing_type` | `ORCA_CONFIG_PADDING` only | 106, 266 (untracked) | **0 — none** | `Fill.cpp::make_ironing` | padding "no ironing" / coEnum NoIroning | none | PADDING-ONLY | implement | Ticket 106 reclassified this from rename to **gap**; packet 266 proposes "Replace bool gate with four-mode enum". Today it exists only as a padding twin. Canonical branches four ways (NoIroning / AllSolid / TopSurfaces / TopmostOnly) in `Fill.cpp::make_ironing` — verified this session. |
| `support_ironing` | nowhere | 106, 22 (P15) | **0 — none** | `SupportParameters.hpp::SupportParameters` | - / coBool false | none | NOT-YET-BUILT | implement | Ticket 106 reclassified it from rename to gap. Zero occurrences anywhere in the tree — not even a padding entry. |
| `ironing_angle` | packet 266 only (untracked) | 266 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | implement | Packet 266 proposes "Add float table; rotate scan generator" — a real decision point, satisfying rule 1. |
| `ironing_angle_fixed` | packet 266 only (untracked) | 266 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | implement | Packet 266 proposes a bool + deterministic fixed orientation. Rule-1 compliant. |
| `ironing_inset` | packet 266 only (untracked) | 266 | 0 — none | unverified | - / unverified | none | NOT-YET-BUILT | implement | Packet 266 proposes inward-offset surface polygons. Rule-1 compliant. |

### Owner: infill modules (`rectilinear-infill`, `gyroid-infill`, `lightning-infill`, `infill-linker`) and the fill-holder seam

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `sparse_infill_density` | 5 infill/perimeter manifests, `ResolvedConfig`, padding | 107 `f8606585` | 8 — `rectilinear-infill/src/lib.rs::from_config`, `gyroid-infill/src/lib.rs::from_config`, `lightning-infill/src/lib.rs::from_config`, `infill-linker/src/orchestrate.rs::orchestrate` | `Fill.cpp::group_fills`, `FillAdaptive.cpp::adaptive_fill_line_spacing`, `Lightning/Generator.cpp::Generator` | 20.0 percent / coPercent 20 | host field + in-module | OK | no-change | Duplicate collapse of `infill_density`. Surviving key is percent `[0,100]` everywhere; modules divide by 100 at the read site. Ticket 107 added `extract_percent_float` so Orca 3MF percent strings resolve, and made the loader preserve percent strings raw (it previously coerced them to fraction floats). Padding twin emits `15%` against a `20` default — see Q5. |
| `sparse_infill_speed` | 4 infill manifests, `ResolvedConfig`, `FEEDRATE_KEYS` | 107 `f8606585` | 4 — `rectilinear-infill/src/lib.rs::from_config`, `gyroid-infill/src/lib.rs::from_config`, `lightning-infill/src/lib.rs::from_config`, `slicer-ir/src/feedrate.rs::FEEDRATE_KEYS` | `Fill.cpp::group_fills`, `WipeTower2.cpp::WipeTower2` | manifests 100 / `ResolvedConfig` 50.0 / coFloats 100 | host field + in-module | OK | no-change | Duplicate collapse of `infill_speed`. Manifests aligned to canonical 100; `ResolvedConfig`'s 50.0 was deliberately left as the speed-factor base, and host `FeedrateConfig.sparse_infill_speed` is 100.0. Three numbers for one key name is defensible but fragile — see Q13. |
| `infill_direction` | `rectilinear-infill.toml`, `gyroid-infill.toml`, `ResolvedConfig`, padding | 105 `cd02cc32` | 4 — `rectilinear-infill/src/lib.rs::from_config`, `gyroid-infill/src/lib.rs::from_config`, `slicer-core/src/algos/lightning/mod.rs`, `slicer-core/src/algos/region_mapping.rs` overlay merge | `Fill.cpp::group_fills` | 45.0 / coFloat 45 | host field + in-module | OK | no-change | Rename of `infill_angle`; defaults byte-identical, zero deviation rows. Verified live on the new spelling. |
| `infill_overlap` | `infill-linker.toml`, `ResolvedConfig` | 107 `f8606585` | 3 — `infill-linker/src/lib.rs::from_config`, `::link_infill`, `slicer-ir/src/resolved_config.rs` binding | none — canonical's `infill_wall_overlap` is a different decision point | 0.45 fraction-of-spacing / n-a | host field + in-module | OK | no-change | Ticket 107 re-adjudicated this **out** of the duplicate-collapse set: it is a PnP-invented infill-side post-pass, not a spelling of `infill_wall_overlap`. That correction moved the collapse count 3 to 2 and the rename pool 25 to 24. |
| `infill_anchor` | `infill-linker.toml` | — | 1 — `infill-linker/src/connect.rs::from_config` (`anchor_length`) | `Fill.cpp::group_fills` | unverified / coFloatOrPercent(400, true) | in-module | OK | no-change | Live. Port default not re-derived — **unverified**. Canonical type is float-or-percent; whether the port models that dual type is unchecked. |
| `infill_anchor_max` | `infill-linker.toml` | — | 1 — `infill-linker/src/connect.rs::from_config` (`anchor_length_max` clamp) | `Fill.cpp::group_fills` | unverified / coFloatOrPercent(20, false) | in-module | OK | no-change | Same. |
| `bridge_density` | `rectilinear-infill.toml`, `wave-overhangs.toml` | — | 1 — `rectilinear-infill/src/lib.rs::from_config` (spacing scaling) | unverified | unverified / unverified | in-module | OK | no-change | — |
| `internal_bridge_density` | `rectilinear-infill.toml` | — | 1 — `rectilinear-infill/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `internal_bridge_angle` | `rectilinear-infill.toml` | — | 1 — `rectilinear-infill/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `dont_filter_internal_bridges` | `rectilinear-infill.toml` | — | 1 — `rectilinear-infill/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `enable_extra_bridge_layer` | `rectilinear-infill.toml` | — | 1 — `rectilinear-infill/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `thick_internal_bridges` | `rectilinear-infill.toml` | — | 1 — `rectilinear-infill/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `thick_bridges` | `rectilinear-infill.toml`, both perimeter manifests | — | 3 — `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::run_perimeters`, `rectilinear-infill/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `sparse_infill_pattern` | `ORCA_CONFIG_PADDING` only | 262 `d7b6b1f4` | **0 — none** | `Fill.cpp::group_fills`, `FillAdaptive.cpp`, `Layer.cpp` | padding "grid" / unverified | `*_fill_holder` claim resolution | MECHANISM-VIOLATION | re-route-through-claims | Packet 262's own finding — "pattern is module identity" — is correct and matches rule 4. Verified this session: the real seam is `sparse_fill_holder` / `top_fill_holder` / `bottom_fill_holder` / `bridge_fill_holder` in `slicer-ir/src/resolved_config.rs`, merged per-region in `slicer-core/src/algos/region_mapping.rs` and gated in `slicer-core/src/algos/lightning/mod.rs`. Shipped `claim:sparse-fill` holders today: `rectilinear-infill`, `gyroid-infill`, `lightning-infill` (+ `wave-overhangs` for `claim:bridge-fill`). So three of canonical's ~14 pattern values have a holder. The disposition should be "3 values shipped, the rest unimplemented", not one declared-with-gap enum. See Q3. |
| `internal_solid_infill_pattern` | nowhere | 262 `d7b6b1f4` | 0 — none | `Fill.cpp::group_fills`, `GCode.cpp`, `PrintObject.cpp` | - / unverified | `*_fill_holder` claim resolution | MECHANISM-VIOLATION | re-route-through-claims | Same finding, same seam. |
| `top_surface_pattern` | `ORCA_CONFIG_PADDING` only | 264 `725265ef` | **0 — none** | `Fill.cpp::group_fills`, `GCode.cpp` | padding "monotonic" / unverified | `top_fill_holder` claim resolution | MECHANISM-VIOLATION | re-route-through-claims | Packet 264: "Declared-with-gap — filler selection is module identity". Correct diagnosis, prohibited disposition. Note the padding twin says `monotonic` while `top_fill_holder` defaults to `rectilinear-infill` — the emitted CONFIG_BLOCK advertises a pattern the pipeline cannot produce. See Q5. |
| `bottom_surface_pattern` | `ORCA_CONFIG_PADDING` only | 264 `725265ef` | **0 — none** | `Fill.cpp::group_fills`, `GCode.cpp` | padding "monotonic" / unverified | `bottom_fill_holder` claim resolution | MECHANISM-VIOLATION | re-route-through-claims | Same. |
| `top_fill_pattern` | `ORCA_CONFIG_PADDING` only | — | 0 — none | unverified (canonical spells it `top_surface_pattern`) | padding "monotonic" / unverified | none | PADDING-ONLY | rule-out-of-scope | A **second** top-pattern padding twin alongside `top_surface_pattern`, emitting the same value under a PrusaSlicer-era name. Not touched by any audited commit; found by the broad sweep. See Q5. |
| `top_surface_density` | packet 264 only | 264 `725265ef` | **0 — none** | `Fill.cpp`, `PerimeterGenerator.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | implement | **Packet 264 claims "Wired — rectilinear top block spacing". Verified false: zero reads, not declared in any manifest.** See Q2. |
| `bottom_surface_density` | packet 264 only | 264 `725265ef` | **0 — none** | `Fill.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | implement | **Packet 264 claims "Wired — rectilinear bottom block spacing". Verified false: zero reads, not declared.** See Q2. |
| `fill_multiline` | packet 262 only | 262 `d7b6b1f4` | **0 — none** | `Fill.cpp`, `FillAdaptive.cpp`, `Lightning/Generator.cpp` | - / unverified | none | NOT-YET-BUILT | implement | **Packet 262 claims "Wired (rectilinear sparse); declared-with-gap (gyroid, lightning)". Verified false: zero reads, not declared.** Double finding — a false wired claim *and* a rule-1 declared-with-gap on the same row. See Q2. |
| `sparse_infill_rotate_template` | packet 262 only | 262 `d7b6b1f4` | **0 — none** | `Fill.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | implement | **Packet 262 claims "Wired (rectilinear, gyroid)". Verified false: zero reads, not declared.** See Q2. |
| `solid_infill_direction` | packet 262 only | 262 `d7b6b1f4` | **0 — none** | `Fill.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | implement | **Packet 262 claims "Wired (rectilinear, gyroid)". Verified false: zero reads, not declared.** See Q2. |
| `solid_infill_rotate_template` | packet 262 only | 262 `d7b6b1f4` | **0 — none** | `Fill.cpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | implement | **Packet 262 claims "Wired (rectilinear, gyroid)". Verified false: zero reads, not declared.** See Q2. |
| `lateral_lattice_angle_1` | packet 263 only (mid-re-authoring) | 263 `725265ef` | 0 — none | unverified | - / unverified | packet proposes a new claim-holding module | NOT-YET-BUILT | implement | Packet 263 class (b) — "decision point this packet builds", on a new `lateral-lattice-infill` module. This is the rule-4-shaped answer: a new module per pattern rather than an enum value. |
| `lateral_lattice_angle_2` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module | NOT-YET-BUILT | implement | Same. |
| `infill_overhang_angle` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module (`lateral-honeycomb-infill`) | NOT-YET-BUILT | implement | Same. |
| `infill_lock_depth` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module (`locked-zag-infill`) | NOT-YET-BUILT | implement | Same. |
| `skin_infill_depth` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module | NOT-YET-BUILT | implement | Same. |
| `skin_infill_density` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module | NOT-YET-BUILT | implement | Same. |
| `skeleton_infill_density` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module | NOT-YET-BUILT | implement | Same. |
| `skin_infill_line_width` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module | NOT-YET-BUILT | implement | Same. |
| `skeleton_infill_line_width` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module | NOT-YET-BUILT | implement | Same. |
| `symmetric_infill_y_axis` | packet 263 only | 263 `725265ef` | 0 — none | unverified | - / unverified | new claim-holding module | NOT-YET-BUILT | implement | Same. Note packet 263 is the one the map flagged as "zero module reads for 10 keys"; the working-tree re-authoring has moved all 10 to class (b), which reads as a rule-1 fix in progress. See Q14. |

### Owner: support modules and planners

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `enable_support` | 4 support manifests, `ResolvedConfig`, padding | 100 `e1da8b36` | 8 — `traditional-support-planner/src/lib.rs::from_config`, `tree-support-planner/src/lib.rs::from_config`, `slicer-runtime/src/builtins/support_analysis_producer.rs::produce`, `pnp-cli/src/support_preview.rs` | `Slicing.cpp::create_from_config`, `TreeSupport3D.cpp::group_meshes`, `SupportMaterial.cpp::top_contact_layers` | false / coBool false | host field + in-module | OK | no-change | Not a rename. Ticket 100 flipped the boolean default true to false across all four support manifests. |
| `support_top_z_distance` | both planner manifests, `ResolvedConfig`, `SUPPORT_CONFIG_DEFAULTS` | 104 `f03d8408` | 5 — `traditional-support-planner/src/lib.rs::from_config`, target-top-z computation, `tree-support-planner/src/lib.rs::from_config`, `slicer-core/src/algos/support_geometry.rs` | `Slicing.cpp::create_from_config`, `TreeSupport.cpp::TreeSupport`, `GCode.cpp::collect_layers_to_print` | 0.2 / coFloat 0.2 | host field + in-module | OK | no-change | Rename of `support_top_z_distance_mm`. The rename also connected a pre-existing host-side Orca spelling to the module view, so one user value now reaches both the prepass and the planner modules. |
| `support_object_xy_distance` | both planner manifests | 265 `49c39a90` | 2 — `traditional-support-planner/src/lib.rs::from_config`, `tree-support-planner/src/lib.rs::from_config` | unverified | 0.35 / unverified | in-module | OK | no-change | Packet 265's "Wired + verified — no change" claim **holds** against the tree. |
| `support_threshold_angle` | `traditional-support-planner.toml`, `ResolvedConfig` | 265 `49c39a90` | 3 — `slicer-runtime/src/builtins/support_analysis_producer.rs::commit_support_analysis_builtin`, `slicer-core/src/algos/region_mapping.rs` overlay merge, `slicer-core/src/algos/overhang_annotation.rs::lower_layer_offset_mm` | `Slicing.cpp` / `SupportMaterial.cpp` (see `enable_support` row) | 30.0 / unverified | host field | OK | no-change | Packet 265's "Wired + verified — canonical-faithful" claim **holds**. |
| `support_style` | `traditional-support.toml`, `tree-support-planner.toml`, padding | 265 `49c39a90` | 3 — `traditional-support/src/lib.rs::from_config`, `tree-support-planner/src/lib.rs::TreeStyle::from_config` | `SupportCommon.cpp::generate_interface_layers`, `::generate_support_toolpaths`, `SupportMaterial.cpp::fill_contact_layer` | "default" / coEnum smsDefault | in-module enum | OK | no-change | Packet 265's "Wired + verified, type corrected" claim **holds**. `traditional-support` resolves `smooth_supports` as canonical's `support_style != smsGrid`. Whether tree/organic/snug styles should be separate claim holders is a rule-4 judgement — see Q8. |
| `support_type` | `ResolvedConfig` only (not in any module manifest) | 265 `49c39a90` | 2 — `traditional-support-planner/src/lib.rs::canonical_support_family`, `tree-support-planner/src/lib.rs::canonical_support_family` | unverified | unverified / unverified | host field selecting a planner family | OK | no-change | Packet 265's "Wired + declared" claim is **half right**: it is wired, but it is *not* declared in either planner manifest — it is a `ResolvedConfig` field. |
| `support_expansion` | `ResolvedConfig`, `SUPPORT_CONFIG_DEFAULTS` | 265 `49c39a90` | 2 — `slicer-runtime/src/builtins/support_analysis_producer.rs::resolve_contact_params`, `slicer-core/src/algos/overhang_annotation.rs::detect_support_contacts` | unverified | unverified / coFloat 0 | host field | OK | no-change | Wired via the host, not the manifests. Same "declared" over-claim as `support_type`. |
| `support_threshold_overlap` | `ResolvedConfig` | 265 `49c39a90` | 2 — `slicer-runtime/src/builtins/support_analysis_producer.rs::resolve_contact_params`, `slicer-core/src/algos/overhang_annotation.rs::lower_layer_offset_mm` | unverified | unverified / unverified | host field | OK | no-change | Wired via the host. Same "declared" over-claim. |
| `enforce_support_layers` | `ResolvedConfig` | 265 `49c39a90` | **0 — none** | unverified | 0 / unverified | host field, plumbing severed | BROKEN-RENAME | implement | **Packet 265 claims "Wired by this packet". The tree disagrees, and this is the sharpest finding in the audit.** A live consumer exists — `slicer-core/src/algos/overhang_annotation.rs` computes `force_support = params.layer_id < params.enforce_support_layers`. But `slicer-runtime/src/builtins/support_analysis_producer.rs::resolve_contact_params` hardcodes `enforce_support_layers: 0` under a comment saying these knobs "have no production config source yet". The config value is read into `ResolvedConfig`, emitted into the config map, and then dropped on the floor. Same pattern hardcodes `bridge_no_support: false`, `support_sharp_tails: false`, `layer_id: 0`. Verified directly this session. See Q15. |
| `support_bottom_z_distance` | `ResolvedConfig`, `SUPPORT_CONFIG_DEFAULTS` | 265 `49c39a90` | **0 — none** | unverified | 0.2 / unverified | none | STUB | implement | Packet 265: "Declared-with-gap — unread; AC-N1 pins non-perturbation". An AC that pins *non*-perturbation is precisely the rule-6(b) failure. Second disposition: PADDING-ONLY (its only emission path is `SUPPORT_CONFIG_DEFAULTS`). |
| `support_critical_regions_only` | `ResolvedConfig` | 265 `49c39a90` | **0 — none** | unverified | unverified / unverified | none | STUB | shed-to-queue | "Declared-with-gap — unread". Rule-1 violation. |
| `support_object_first_layer_gap` | `ResolvedConfig` | 265 `49c39a90` | **0 — none** | unverified | unverified / unverified | none | STUB | shed-to-queue | Same. |
| `support_remove_small_overhang` | `ResolvedConfig` | 265 `49c39a90` | **0 — none** | unverified | unverified / unverified | none | STUB | shed-to-queue | Same. |
| `support_overhang_angle` | `traditional-support-planner.toml` | — | **0 — none** | none found in `libslic3r/` | 30.0 / n-a | alias only | STUB | rule-out-of-scope | Its only in-tree occurrence outside the manifest is a `CONFIG_KEY_ALIASES` rename entry in `slicer-scheduler/src/config_resolution.rs`. It appears to be a legacy alias of `support_threshold_angle` that outlived its rename. No canonical counterpart. Not touched by any audited commit; found by the broad sweep. |
| `support_base_pattern` | `traditional-support-planner.toml` | — | **0 behavioural** (1 non-branching) — `traditional-support-planner/src/lib.rs::from_config` | `SupportCommon.cpp::generate_support_toolpaths`, `Print.cpp::validate` | "rectilinear" / coEnum smpDefault | write-only capability label | MECHANISM-VIOLATION | re-route-through-claims | Verified directly: the value is read into a field and then `format!`-ed into a capability string `"traditional-base-pattern:{}"`. Grepping the whole tree for `traditional-base-pattern` returns **only that one line** — nothing consumes it. Canonical selects a real `SupportMaterialPattern` (rectilinear / rectilinear-grid / honeycomb / lightning / default). Second disposition: STUB. Found by the broad sweep. |
| `support_base_pattern_spacing` | 3 support manifests | — | 2 — `tree-support/src/lib.rs::from_config`, `traditional-support/src/lib.rs::from_config` | unverified | 2.5 / coFloat 2.5 | in-module | OK | no-change | The *spacing* is live even though the *pattern* it spaces is not. |
| `support_angle` | `traditional-support.toml` | — | 1 — `traditional-support/src/lib.rs::from_config` (`base_angle` + layer rotation) | unverified | **60.0 / coFloat 0** | in-module | DEFAULT-MISMATCH | align-default | Derived this session from `PrintConfig.cpp`. Canonical 0 means "alternate by layer"; the port's 60.0 is a fixed base angle. Not touched by any audited commit; found by the broad sweep. |
| `support_speed` | `traditional-support.toml`, `tree-support.toml` | — | 2 — `traditional-support/src/lib.rs::from_config`, `tree-support/src/lib.rs::from_config` | unverified | 50.0 / unverified | in-module | OK | no-change | — |
| `support_interface_spacing` | `traditional-support.toml`, `tree-support.toml` | 260 `d8b7e1fd` | 2 — `traditional-support/src/lib.rs::from_config`, `tree-support/src/lib.rs::from_config` | unverified | **0.4 / coFloat 0.5** | in-module | DEFAULT-MISMATCH | align-default | **Packet 260 claims "Wired + default aligned". Wired is true; "default aligned" is false** — canonical `PrintConfig.cpp` sets `ConfigOptionFloat(0.5)`, the port manifests set 0.4. Derived this session. See Q2. |
| `support_bottom_interface_spacing` | `traditional-support.toml`, `tree-support.toml` | 260 `d8b7e1fd` | 2 — `traditional-support/src/lib.rs::from_config`, `tree-support/src/lib.rs::from_config` | unverified | 0.5 / coFloat 0.5 | in-module | OK | no-change | Packet 260's "Wired + divergence pinned" claim holds; defaults match. |
| `support_interface_flow` | `traditional-support.toml`, `tree-support.toml` | — | 2 — `traditional-support/src/lib.rs::from_config`, `tree-support/src/lib.rs::from_config` | unverified | "100%" / not declared under this name | in-module | OK | no-change | Canonical has no `support_interface_flow` option (no `set_default_value` found); the flow is derived inside `SupportParameters`. PnP-shaped seam. |
| `support_interface_pattern` | packet 260 only | 260 `d8b7e1fd` | 0 — none | `SupportParameters.hpp::SupportParameters` (branches on `smipGrid` / `smipRectilinearInterlaced` / `smipAuto` / `smipConcentric`) | - / unverified | packet proposes an enum, no dispatch | MECHANISM-VIOLATION | re-route-through-claims | Packet 260: "Declared-with-gap — no pattern dispatch". The map names `support_interface_pattern` explicitly in rule 4 as a claim-holder set. |
| `support_interface_loop_pattern` | packet 260 only | 260 `d8b7e1fd` | 0 — none | `SupportCommon.cpp`, `SupportMaterial.hpp`, `PrintObject.cpp` | - / unverified | none | NOT-YET-BUILT | shed-to-queue | "Declared-with-gap — no contact-loop generator". Rule-1 violation. |
| `support_interface_top_layers` | both planner manifests | — | 2 — `tree-support-planner/src/lib.rs::from_config`, `traditional-support-planner/src/lib.rs::from_config` | unverified | 2 / unverified | in-module | OK | no-change | — |
| `support_interface_bottom_layers` | both planner manifests | — | 2 — `tree-support-planner/src/lib.rs::from_config`, `traditional-support-planner/src/lib.rs::from_config` | unverified | -1 / unverified | in-module | OK | no-change | The `-1` sentinel ("mirror the top count") is canonical's convention; not re-verified this session. |
| `support_layer_height_mm` | both planner manifests, `ResolvedConfig` | — | 2 — `traditional-support-planner/src/lib.rs::from_config`, `slicer-ir/src/resolved_config.rs` validation | none found in `libslic3r/` | 0.0 / n-a | host field + in-module | OK | no-change | PnP-specific spelling (`_mm` suffix). Not in the ticket-104 rename set despite matching that pattern. See Q15. |
| `support_on_build_plate_only` | `tree-support-planner.toml` | — | 1 — `tree-support-planner/src/lib.rs::from_config` (node pruning + interface assignment) | `PrintObject.cpp`, `SupportMaterial.hpp` | unverified / unverified | in-module | OK | no-change | — |
| `support_max_branches_per_layer` | `tree-support-planner.toml` | — | 1 — `tree-support-planner/src/lib.rs::from_config` | none found in `libslic3r/` | unverified / n-a | in-module | OK | no-change | PnP-invented performance cap. |
| `support_branch_merge_distance_mm` | `tree-support-planner.toml` | — | **0 — none** | none found in `libslic3r/` | unverified / n-a | none | STUB | shed-to-queue | Only non-manifest occurrence is a doc comment in `tree-support-planner/src/lib.rs`. PnP-invented and unimplemented, so this is a self-inflicted stub rather than a parity gap. Found by the broad sweep. |
| `max_bridge_length` | `tree-support-planner.toml` | — | 1 — `tree-support-planner/src/lib.rs::from_config` | `Print.hpp`, `PrintObject.cpp` | unverified / unverified | in-module | OK | no-change | — |
| `tree_support_branch_angle` | `tree-support-planner.toml`, padding | — | 1 — `tree-support-planner/src/lib.rs::from_config` (`branch_scale_factor`) | unverified | unverified / unverified | in-module | OK | no-change | — |
| `tree_support_branch_diameter` | `tree-support-planner.toml`, padding | — | 1 — `tree-support-planner/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `tree_support_branch_diameter_angle` | `tree-support-planner.toml`, padding | — | 1 — `tree-support-planner/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `tree_support_branch_distance` | `tree-support-planner.toml` | — | 1 — `tree-support-planner/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `tree_support_wall_count` | `tree-support-planner.toml`, `tree-support.toml` | — | 2 — `tree-support-planner/src/lib.rs::from_config`, `tree-support/src/lib.rs::from_config` | unverified | unverified / unverified | in-module | OK | no-change | — |
| `support_raft_layers` | `tree-support-planner.toml`, both perimeter manifests | — | 3 — `tree-support-planner/src/lib.rs::from_config` (raft-plan gate), `classic-perimeters/src/lib.rs::run_perimeters`, `arachne-perimeters/src/lib.rs::run_perimeters` | none under this name (canonical spells it `raft_layers`) | unverified / n-a | in-module | OK | no-change | The map's documented `raft_layers` 1-to-3 split, ruled a strict superset rather than a gap. Padding still emits a separate `raft_layers` twin. |
| `base_raft_layers` | `tree-support-planner.toml` | — | 1 — `tree-support-planner/src/lib.rs::from_config` (into `RaftPlan`, min-merged in `slicer-wasm-host/src/support_aggregation.rs`) | `Slicing.cpp`, `TimelapsePosPicker.cpp` | unverified / unverified | in-module | OK | no-change | Part of the 1-to-3 raft split. |
| `interface_raft_layers` | `tree-support-planner.toml` | — | 1 — `tree-support-planner/src/lib.rs::from_config` (into `RaftPlan`) | `Slicing.cpp`, `TimelapsePosPicker.cpp` | unverified / unverified | in-module | OK | no-change | Part of the 1-to-3 raft split. |
| `num_top_base_interface_layers` | `tree-support-planner.toml` | — | 1 — `tree-support-planner/src/lib.rs::from_config` | `SupportCommon.cpp`, `SupportParameters.hpp` | unverified / unverified | in-module | OK | no-change | — |
| `raft_first_layer_density` | `tree-support-planner.toml`, padding | — | 1 — `tree-support-planner/src/lib.rs::from_config` (into `RaftPlan`) | unverified | unverified / padding twin "90%" | in-module | OK | no-change | — |
| `raft_first_layer_expansion` | packet 265 only | 265 `49c39a90` | **0 — none** | `PrintObject.cpp`, `SupportCommon.cpp`, `TreeSupport.cpp` | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Packet 265: "Declared-with-gap — unread". Rule-1 violation. |
| `raft_contact_distance` | packet 261 only | 261 `f4f5bcf6` | 0 — none | `GCode.cpp`, `PrintObject.cpp`, `Slicing.cpp` | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Packet 261: "Declared-with-gap — zero occurrences at authoring". The packet states the zero itself and declares the key anyway — the clearest rule-1 violation in the set. |
| `raft_expansion` | packet 261 only | 261 `f4f5bcf6` | 0 — none | `PrintObject.cpp`, `SupportMaterial.cpp`, `TreeSupport3D.cpp` | - / unverified | none | NOT-YET-BUILT | shed-to-queue | Packet 261: "Declared-with-gap — raft generator absent". Packet 261 declares 2 keys and wires 0. See Q14. |
| `raft_layers` | `ORCA_CONFIG_PADDING` only | — | 0 — none | unverified | padding "0" / unverified | none | PADDING-ONLY | rule-out-of-scope | Superseded in-tree by the `support_raft_layers` / `base_raft_layers` / `interface_raft_layers` split; the padding twin keeps emitting the retired single-key spelling. Found by the broad sweep. |
| `support_material` | `ORCA_CONFIG_PADDING` only | — | 0 — none | unverified (PrusaSlicer-era name; canonical uses `enable_support`) | padding "0" / unverified | none | PADDING-ONLY | rule-out-of-scope | A second support on/off twin alongside `enable_support`. Found by the broad sweep. |
| `support_type` (padding twin) | `ORCA_CONFIG_PADDING` | — | see `support_type` row above | — | padding "normal(auto)" / — | — | PADDING-ONLY | no-change | Listed for completeness; the behavioural row is the `ResolvedConfig` one above. |

### Owner: `skirt-brim`

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `skirt_brim_enabled` | `skirt-brim.toml` | — | 1 — `skirt-brim/src/lib.rs::from_config` (enabled gate) | none found in `libslic3r/` | true / n-a | in-module gate | OK | no-change | PnP-invented master switch. Canonical has no single skirt+brim enable; it derives from `skirt_loops` and `brim_width`. See Q14. |
| `skirt_loops` | `skirt-brim.toml`, padding | — | unverified (declared; not in this session's read-site sweep) | `Print.cpp::_make_skirt` | **6 / coInt 1** | in-module | DEFAULT-MISMATCH | align-default | Derived this session from `PrintConfig.cpp`. The padding twin emits `1` — matching canonical while the manifest says 6, so the CONFIG_BLOCK misreports the port's own behaviour. Not touched by any audited commit. |
| `skirt_distance` | `skirt-brim.toml`, padding | — | unverified | unverified | **3.0 / coFloat 2** | in-module | DEFAULT-MISMATCH | align-default | Derived this session. Padding twin emits `2`, again matching canonical and contradicting the manifest. |
| `skirt_height` | `skirt-brim.toml`, padding | — | 1 — `skirt-brim/src/lib.rs::from_config` (max-layer bound) | unverified | 1 / unverified | in-module | OK | no-change | Padding twin agrees at `1`. |
| `brim_width` | `skirt-brim.toml`, padding | — | 1 — `skirt-brim/src/lib.rs::from_config` (loop count / offsets) | unverified | **8.0 / coFloat 0.** | in-module | DEFAULT-MISMATCH | align-default | Derived this session. Canonical default is 0 (no brim); the port prints an 8 mm brim by default. Padding twin emits `0`. This is a visible, material behaviour difference at defaults. Not touched by any audited commit. |
| `brim_type` | `ORCA_CONFIG_PADDING` only | 257 `79896f8e` | **0 — none** | `Brim.cpp::outer_inner_brim_area`, `PerimeterGenerator.cpp::process_classic`, `SupportCommon.cpp::generate_raft_base` | padding "auto_brim" / coEnum btAutoBrim | none | PADDING-ONLY | implement | **Packet 257 claims "Wired (gate): no_brim forces no brim". Verified false: zero reads, not declared in any manifest.** The only in-tree occurrences are the padding entry and a name-keyed pass-through in `slicer-model-io/src/loader.rs` object-metadata coercion, which the brief excludes as a read site. Canonical also selects *brim geometry algorithms* (outer / inner / outer-and-inner / ears), so a claim-holder shape applies. See Q2 and Q3. |
| `brim_object_gap` | `ORCA_CONFIG_PADDING` only | 257 `79896f8e` | 0 — none | `Brim.cpp::outer_inner_brim_area`, `SupportCommon.cpp::generate_raft_base` | padding "0" / coFloat 0. | none | PADDING-ONLY | shed-to-queue | Packet 257: "Declared-with-gap". Rule-1 and rule-2 violation. |
| `brim_ears_max_angle` | packet 257 only | 257 `79896f8e` | 0 — none | `Brim.cpp::outer_inner_brim_area`, `Brim.cpp::make_brim_ears_auto` | - / coFloat 125 | none | NOT-YET-BUILT | shed-to-queue | Packet 257: "Declared-with-gap". Note this key is **live in canonical** even though `brim_ears` itself was ruled dead by ticket 12 — the ears feature is reached through `brim_type == btBrimEars`, not through the retired `brim_ears` bool. See Q14. |
| `brim_ears_detection_length` | packet 257 only | 257 `79896f8e` | 0 — none | `Brim.cpp::outer_inner_brim_area`, `Brim.cpp::make_brim_ears_auto` | - / coFloat 1 | none | NOT-YET-BUILT | shed-to-queue | Same. |
| `brim_use_efc_outline` | packet 257 only | 257 `79896f8e` | 0 — none | `Brim.cpp::use_brim_efc_outline` | - / coBool false | none | NOT-YET-BUILT | shed-to-queue | Packet 257: "Declared-with-gap". Rule-1 violation. |
| `brim_ears` | none | 257 `79896f8e` | 0 — none | **none** | - / unverified | n/a | DEAD-IN-CANONICAL | rule-out-of-scope | Already ruled out of scope by ticket 12 under the ticket-04 precedent, and packet 257 records it as such. Included here only to confirm the ruling still holds; not counted in the disposition totals as a live finding. |
| `skirt_type` | packet 258 only | 258 `06c551ca` | 0 — none | `GCode.cpp::generate_object_skirt_group`, `Print.cpp::_make_skirt` | - / coEnum stCombined | none | NOT-YET-BUILT | shed-to-queue | Packet 258: "Declared-with-gap — no per-object skirt grouping". Rule-1 violation. |
| `min_skirt_length` | packet 258 only | 258 `06c551ca` | 0 — none | `Print.cpp::_make_skirt` | - / coFloat 0.0 | none | NOT-YET-BUILT | shed-to-queue | Packet 258: "Declared-with-gap — no extruded-length model". Rule-1 violation. |
| `skirt_start_angle` | packet 258 only | 258 `06c551ca` | **0 — none** | `GCode.cpp::generate_skirt`, `::generate_object_skirt_group`, `::process_layer` | - / coFloat -135 | none | NOT-YET-BUILT | implement | **Packet 258 claims "Wired (start corner)". Verified false: zero reads, not declared.** See Q2. |
| `draft_shield` | packet 258 only | 258 `06c551ca` | **0 — none** | `Print.cpp::has_infinite_skirt`, `::object_skirt_offset` | - / coEnum dsDisabled | none | NOT-YET-BUILT | implement | **Packet 258 claims "Wired (span gate)". Verified false: zero reads, not declared.** See Q2. |
| `single_loop_draft_shield` | packet 258 only | 258 `06c551ca` | **0 — none** | `GCode.cpp::generate_skirt` | - / coBool false | none | NOT-YET-BUILT | implement | **Packet 258 claims "Wired (per-layer loop count)". Verified false: zero reads, not declared.** Packet 258 makes three "Wired" claims and none of them hold. See Q2. |

### Owner: `part-cooling` / `machine-gcode-emit` — packet 253 proposals

All of these exist only as rows in packet 253's table. Packet 253's table has no disposition column
at all (its columns are Key / Canonical declaration / Canonical consumer / Ported behaviour), so the
"disposition" recorded for each is the packet's *intended* behaviour, not a claim about the tree.

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `fan_cooling_layer_time` | packet 253 + `ORCA_CONFIG_PADDING` | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::apply_layer_cooldown` | padding "100" / coFloats 60.0 | none | PADDING-ONLY | implement | Padding twin emits `100` against a canonical default of `60.0`. Second disposition: DEFAULT-MISMATCH on the twin. See Q5. |
| `reduce_fan_stop_start_freq` | packet 253 + `ORCA_CONFIG_PADDING` | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::apply_layer_cooldown` | padding "1" / coBools false | none | PADDING-ONLY | implement | Padding twin emits `1` (true) against a canonical default of `false`. Second disposition: DEFAULT-MISMATCH on the twin. This is the key that would give `fan_min_speed` its purpose. |
| `fan_kickstart` | packet 253 only | 253 `0e6dadd3` | 0 — none | `GCode.cpp::process_layers` | - / coFloat 0 | none | NOT-YET-BUILT | implement | Rule-1 compliant proposal (real decision point described). |
| `fan_speedup_time` | packet 253 only | 253 `0e6dadd3` | 0 — none | `GCode.cpp::process_layers` | - / coFloat 0 | none | NOT-YET-BUILT | implement | Same. |
| `fan_speedup_overhangs` | packet 253 only | 253 `0e6dadd3` | 0 — none | `GCode.cpp::process_layers` | - / coBool true | none | NOT-YET-BUILT | implement | Same. |
| `full_fan_speed_layer` | packet 253 only | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::apply_layer_cooldown` | - / coInts 0 | none | NOT-YET-BUILT | implement | Same. |
| `dont_slow_down_outer_wall` | packet 253 only | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::parse_layer_gcode` | - / coBools false | none | NOT-YET-BUILT | shed-to-queue | Packet 253's own wording: "declared + emitted; gap recorded" — the one explicit rule-1 violation in packet 253. |
| `overhang_fan_threshold` | packet 253 only | 253 `0e6dadd3` | 0 — none | `GCode.cpp::_extrude` | - / coEnums Overhang_threshold_bridge | packet proposes a quartile-band classifier | NOT-YET-BUILT | implement | An enum, but of *thresholds* rather than algorithms, so an in-module mapping table is the right shape under rule 4. |
| `internal_bridge_fan_speed` | packet 253 only | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::apply_layer_cooldown` | - / coInts -1 | none | NOT-YET-BUILT | implement | Rule-1 compliant (role fan with `-1` = disabled sentinel). |
| `ironing_fan_speed` | packet 253 only | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::apply_layer_cooldown`, `GCode.cpp::_extrude` | - / coInts -1 | none | NOT-YET-BUILT | implement | Same. |
| `support_material_interface_fan_speed` | packet 253 only | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::apply_layer_cooldown`, `GCode.cpp::_extrude` | - / coInts -1 | none | NOT-YET-BUILT | implement | Same. |
| `additional_cooling_fan_speed` | packet 253 only | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::apply_layer_cooldown`, `ToolOrdering.cpp::cal_max_additional_fan` | - / coInts 0 | none | NOT-YET-BUILT | implement | Same. |
| `auxiliary_fan` | packet 253 only | 253 `0e6dadd3` | 0 — none | `CoolingBuffer.cpp::apply_layer_cooldown`, `GCodeWriter.cpp::set_fan` | - / coBool false | none | NOT-YET-BUILT | implement | Packet routes it to `machine-gcode-emit` as a "P2-channel enable + placeholder". "Placeholder" is rule-1 language — see Q1. |
| `activate_air_filtration` | packet 253 only | 253 `0e6dadd3` | 0 — none | `GCode.cpp::_do_export` | - / coBools false | none | NOT-YET-BUILT | shed-to-queue | Packet wording: "placeholder for custom templates". Rule-1 violation. |
| `activate_chamber_temp_control` | packet 253 only | 253 `0e6dadd3` | 0 — none | `GCode.cpp::_do_export` | - / coBools false | none | NOT-YET-BUILT | shed-to-queue | Same. |
| `during_print_exhaust_fan_speed` | packet 253 only | 253 `0e6dadd3` | 0 — none | `GCode.cpp::_do_export` | - / coInts 60 | none | NOT-YET-BUILT | shed-to-queue | Same ("placeholder (raw percent)"). |
| `complete_print_exhaust_fan_speed` | packet 253 only | 253 `0e6dadd3` | 0 — none | `GCode.cpp::_do_export` | - / coInts 80 | none | NOT-YET-BUILT | shed-to-queue | Same. |

### Other owners — broad-sweep rows

| key | declared where | commit(s) | in-tree read sites (file::fn, count) | canonical read site | default port / canonical | mechanism | disposition | proposed action | evidence note |
|---|---|---|---|---|---|---|---|---|---|
| `wave_overhang_pattern` | `wave-overhangs.toml` | — | 1 — `wave-overhangs/src/lib.rs` (`WavePattern::from_str_or_default`) | none found in `libslic3r/` | "smart" / n-a | in-module enum, unknown-value fallback | OK | no-change | PnP/fork-specific. Unlike `ironing_pattern`, this enum has multiple live values, so it is a real decision point. Note it silently falls back on an unknown string rather than erroring — the opposite convention from `seam_position` and `ironing_pattern`. See Q3. |
| `gcode_resolution` | `ResolvedConfig` | 105 `cd02cc32` | 1 — `slicer-gcode/src/serialize.rs::tolerance_for_role` | n/a — canonical `resolution` is a different decision point | 0.0125 / n-a | host field | OK | no-change | Ticket 105 re-adjudicated this **out** of the rename pool: canonical `resolution` is a generation-time global simplify, this is emit-time and per-role. The in-session alignment to 0.01 was withdrawn. This is the row that moved the scoped target 406 to 407. |
| `resolution` | `ORCA_CONFIG_PADDING` only | 105 `cd02cc32` | 0 — none | `Fill.cpp::make_fills`, `LayerRegion.cpp::simplify_path`, `GCode.cpp::apply_print_config` | padding "0.012" / coFloat 0.01 | none | PADDING-ONLY | implement | Now a queue key (P51, Tier B) rather than a rename. Padding twin emits `0.012`, matching neither canonical's `0.01` nor `gcode_resolution`'s `0.0125`. See Q5. |

---

## Questions for grilling

Fifteen bullets. Each names the keys it covers and the decision it needs.

**Q1 — What counts as a "declaration", and does a template variable count as a decision point?**
Covers: all 72 NOT-YET-BUILT rows; specifically `auxiliary_fan`, `activate_air_filtration`,
`activate_chamber_temp_control`, `during_print_exhaust_fan_speed`, `complete_print_exhaust_fan_speed`
(packet 253's "placeholder" keys).
72 of 212 rows are keys that exist *only* as a line in an unimplemented packet's disposition table —
no manifest entry, no read site, no padding entry. Rule 1 bans "declared-with-gap", but these are not
yet declared anywhere; the violation is in the plan, not the tree. Separately, packet 253 routes five
keys to `machine-gcode-emit` as "placeholders for custom templates" — all five are live in canonical
`GCode.cpp::_do_export`, and if the port's answer is "the user can reference them in a custom
start-gcode template", template substitution reading the value is arguably a real decision point.
**Decision:** (a) does a packet-table row count as a declaration for rule 1, so these 72 are already
violations to shed, or does rule 1 bite only at merge? (b) does template-variable availability
satisfy "behaviour-changing decision point"? Together these two rulings move 72 rows.

**Q2 — Sixteen packet evidence claims do not hold against the tree.**
Covers: "Wired" claims on `prime_tower_infill_gap` (254), `wipe_tower_extra_flow` (255),
`bed_exclude_area` (256), `brim_type` (257), `skirt_start_angle` / `draft_shield` /
`single_loop_draft_shield` (258), `fuzzy_skin` / `fuzzy_skin_first_layer` (259), `fill_multiline` /
`sparse_infill_rotate_template` / `solid_infill_direction` / `solid_infill_rotate_template` (262),
`top_surface_density` / `bottom_surface_density` (264); plus packet 260's "default aligned" claim on
`support_interface_spacing` (canonical 0.5, port 0.4) and packet 265's "declared" claim on
`support_type` / `support_expansion` / `support_threshold_overlap` (wired via the host, not declared
in any manifest).
Every one was verified this session. Packet 258 makes three "Wired" claims and none hold; packet 262
makes four and none hold. Packet 265 uses "Wired + verified" for claims that *do* hold, which
suggests the authors did distinguish the two — and that bare "Wired" marks the unverified rows.
**Decision:** is "Wired" shorthand for "this packet will wire it" (a wording fix), or a factual claim
about the current tree (in which case eleven preflight PASSes rest on unverified assertions and the
gate needs re-running)?

**Q3 — Algorithm-selecting enums: how many values ship before the key counts, and what happens to
the rest?**
Covers: `sparse_infill_pattern`, `internal_solid_infill_pattern`, `top_surface_pattern`,
`bottom_surface_pattern`, `support_interface_pattern`, `fuzzy_skin_noise_type`, `ironing_pattern`,
`wipe_tower_wall_type`, `brim_type`, `support_base_pattern`; and the unknown-value convention on
`seam_position`, `wave_overhang_pattern`, `support_style`, `retract_mode`.
The `*_fill_holder` seam already works: `sparse_fill_holder` / `top_fill_holder` /
`bottom_fill_holder` / `bridge_fill_holder` resolve to module names, merged per-region in
`slicer-core/src/algos/region_mapping.rs`. Three modules hold `claim:sparse-fill` today, against
roughly fourteen canonical `sparse_infill_pattern` values. Meanwhile the port has two conventions for
unshipped values: `seam_position` and `ironing_pattern` hard-error, `wave_overhang_pattern` silently
falls back to `smart`.
**Decision:** (a) does `sparse_infill_pattern` become a host-side alias mapping Orca's enum string
onto a `*_fill_holder` module name (real 3MF compatibility for shipped values), or stay unported with
users selecting modules directly? (b) error or fall back on an unshipped value? This shapes packets
262 and 264 entirely.

**Q4 — The part-cooling fan scale: 0–255 or 0–100?**
Covers: `fan_min_speed`, `fan_max_speed`, `overhang_fan_speed`, and every packet-253 fan key.
The port is raw PWM 0–255 (`fan_max_speed` 255, `fan_min_speed` 51); canonical is percent 0–100 (100
and 20). `overhang_fan_speed` is 100 on both sides, which *looks* aligned and is not — 100 means
"full" in Orca and about 39% in the port. Ticket 99 spotted the scale problem and deferred it to P01,
so every fan key added since inherits the ambiguity.
**Decision:** convert the port to percent (matching Orca, invalidating existing user configs), or keep
0–255 and convert at the config boundary? Cheaper to settle before packet 253 is authored.

**Q5 — `ORCA_CONFIG_PADDING` twins that contradict the port's own behaviour.**
Covers: `slow_down_layer_time` (padding 8, manifest 5.0, canonical 5.0), `sparse_infill_density`
(padding 15%, default 20), `detect_thin_wall` (padding 1, manifest false), `fan_cooling_layer_time`
(padding 100, canonical 60.0), `reduce_fan_stop_start_freq` (padding 1, canonical false), `resolution`
(padding 0.012, canonical 0.01), `skirt_loops` (padding 1, manifest 6), `skirt_distance` (padding 2,
manifest 3.0), `brim_width` (padding 0, manifest 8.0), `top_surface_pattern` (padding `monotonic`,
but `top_fill_holder` defaults to `rectilinear-infill`), plus the duplicate twin pairs
`top_fill_pattern` / `top_surface_pattern`, `raft_layers` / `support_raft_layers`, and
`support_material` / `enable_support`.
The table's own comment calls it "neutral cosmetic". It is not neutral: it emits values contradicting
what the slicer did, into the CONFIG_BLOCK that viewers and re-slicers read.
**Decision:** is the padding table (a) retired, (b) mechanically derived from the resolved config so
it cannot drift, or (c) declared decorative and exempt from correctness? Rule 2 says padding is never
a deliverable, which leaves the existing wrong values unowned.

**Q6 — `wipe_tower_speed`: rename, and does it adopt canonical's bound?**
Covers: `wipe_tower_speed` / `wipe_tower_max_purge_speed`.
Ticket 108 resolved 2026-09-02: rename to `wipe_tower_max_purge_speed` and cap
the `ExtrusionRole::WipeTower` feedrate at `sparse_infill_speed`, matching the
purge-grid form of canonical `WipeTower2::toolchange_Wipe` / `finish_layer`.
Defaults remain 90.0. Canonical declares a minimum of 10; `FeedrateConfig`
fields are unbounded floats, so range validation is deferred to ticket 113.

**Q7 — Per-filament keys declared as scalar globals.**
Covers: `filament_tower_interface_pre_extrusion_dist`, `..._pre_extrusion_length`, `..._print_temp`,
`..._purge_volume`, `filament_tower_ironing_area` (all packet 254).
Packet 254 declares each "(scalar-global)". In canonical these are per-filament vectors, which is why
the wipe tower can vary purge behaviour per material — the point of the feature.
**Decision:** is a scalar-global declaration an acceptable first step, or does it bake in a shape the
real feature must undo? Rule 4 arguably forbids it as a host-side special case.

**Q8 — Does rule 4 apply retroactively to already-live enums?**
Covers: `seam_position` (5 live values, in-module branch), `support_style` (live, in-module),
`wall_sequence`, `retract_mode`, `wave_overhang_pattern`.
These are live, working, in-module enums. Under a strict reading of rule 4, `seam_position`'s `rear`
vs `aligned` vs `random` are arguably different placement algorithms, and `support_style`'s
grid/snug/organic certainly are.
**Decision:** does rule 4 govern new packets only, or does it make these refactor targets? I scored
them OK on the "new work only" reading — confirm or overturn. If overturned, those OK rows are wrong.

**Q9 — `slowdown_for_curled_perimeters` was aligned in the wrong direction.**
Covers: `slowdown_for_curled_perimeters`.
Ticket 100 changed the port default from `false` to `true`, described as "aligned to Orca". Verified
this session: the canonical declaration `add("slowdown_for_curled_perimeters", coBools)` sets
`ConfigOptionBoolsNullable{ false }`, and the port manifest now reads `default = true`. The ticket
filed *because* the boolean deviation gate was vacuous introduced a boolean deviation.
**Decision:** revert to `false`, or was there a deliberate reason (a fork default, a different
upstream revision) to record as a divergence? This is the one row where I assert a committed ticket
got a fact backwards, so it deserves a second look.

**Q10 — Two gates for one decision.**
Covers: `apply_to_all` vs the `fuzzy_skin` enum; `ironing_enabled` vs `ironing_type` and
`support_ironing`.
Ticket 103 renamed `thickness` and `point_distance` to `fuzzy_skin_*` to stop generic names
colliding, then left `apply_to_all` — an unprefixed key in a shared namespace whose canonical
counterpart is the `fuzzy_skin` enum's `external` vs `all`. Likewise the port has a PnP
`ironing_enabled` bool on both ironing modules, while canonical has a four-mode enum for top surfaces
(verified in `Fill.cpp::make_ironing`) plus a separate `support_ironing` bool; packet 266 proposes
replacing the bool gate with the enum.
**Decision:** when `fuzzy_skin` and `ironing_type` land, do `apply_to_all` and `ironing_enabled`
retire into them, or coexist as master switches? Coexistence is the outcome rule 5 warns about.

**Q11 — One key name, several defaults.**
Covers: `ironing_speed`, `sparse_infill_speed`.
`top-surface-ironing.toml` declares `ironing_speed` 20.0 (matching canonical `coFloat 20`);
`support-surface-ironing.toml` declares 30.0 — and ticket 106 touched both files without reconciling
them. `sparse_infill_speed` carries three: manifests 100 (canonical), `ResolvedConfig` 50.0
(deliberately, as the speed-factor base), host `FeedrateConfig` 100.0.
**Decision:** align, or record divergences? And is the 50.0 speed-factor base a genuinely different
quantity that deserves a different name — because as it stands a future agent will see the mismatch
and "fix" it.

**Q12 — Packets that declare more keys than they wire.**
Covers: packet 261 (`raft_contact_distance`, `raft_expansion` — 2 declared, 0 wired), 259 (7 keys, 0
verified wired), 254 (13 keys, 1 claimed wired and that claim fails), 255 (12 keys, 1 claimed and
failing), 257 (5 keys, 1 claimed and failing), 263 (10 keys, all class (b), mid-re-authoring in the
working tree).
Packet 261 states the zero occurrences itself and declares the keys anyway — the clearest rule-1
violation in the set.
**Decision:** per packet — re-author to rule 1, or withdraw and return the keys to the queue as
unimplemented? Packet 263's uncommitted state suggests re-authoring is already underway there; the
others are committed as-is.

**Q13 — `enforce_support_layers` is read, emitted, then discarded.**
Covers: `enforce_support_layers`, and the same pattern on `bridge_no_support`, `support_sharp_tails`,
`layer_id`.
The consumer exists — `slicer-core/src/algos/overhang_annotation.rs` computes
`force_support = params.layer_id < params.enforce_support_layers`. The producer
`slicer-runtime/src/builtins/support_analysis_producer.rs::resolve_contact_params` hardcodes
`enforce_support_layers: 0` under a comment saying these knobs "have no production config source yet"
— but the config source demonstrably exists as a `ResolvedConfig` field emitted into the config map.
This is the most misleading state in the audit: plumbed from both ends, severed in the middle.
**Decision:** this looks like a small fix (pass the resolved value instead of the literal). In scope
for packet 265's re-authoring, or its own ticket? And are the three sibling hardcodes the same bug?

**Q14 — Skirt and brim defaults diverge, and the port has a switch canonical lacks.**
Covers: `skirt_brim_enabled`, `skirt_loops` (6 vs 1), `skirt_distance` (3.0 vs 2), `brim_width`
(8.0 vs 0), `brim_ears`, `brim_ears_max_angle`, `brim_ears_detection_length`.
Canonical derives skirt/brim existence from `skirt_loops` and `brim_width` being non-zero; the port
adds a `skirt_brim_enabled` master switch and defaults `brim_width` to 8.0 against canonical's 0, so
an Orca 3MF meaning "no brim" may produce one here. Separately: ticket 12 ruled `brim_ears`
dead-in-canonical (confirmed still true), but `brim_ears_max_angle` and `brim_ears_detection_length`
*are* live in `Brim.cpp::make_brim_ears_auto`, reached through `brim_type == btBrimEars` rather than
the retired bool.
**Decision:** (a) keep the master switch as a recorded improvement (rule 4 allows it) or retire it so
Orca configs round-trip — and align the three defaults either way? (b) does the ears feature return to
scope via `brim_type`, given that ticket 12's ruling was about the bool, not the feature?

**Q15 — PnP `_mm`-suffixed keys the rename workstream did not sweep.**
Covers: `support_layer_height_mm`, `support_branch_merge_distance_mm`,
`narrow_loop_length_threshold_mm`, `wave_overhang_anchor_depth_mm`, and the alias-only
`support_overhang_angle`.
Ticket 104 renamed `support_top_z_distance_mm` and ticket 106 renamed `ironing_spacing_mm`, but
several `_mm` keys remain. Some have no canonical counterpart at all, so they are not renames — the
suffix now signals two different things. `support_overhang_angle` is a related leftover: its only
non-manifest occurrence is a `CONFIG_KEY_ALIASES` entry in
`slicer-scheduler/src/config_resolution.rs`, an alias of `support_threshold_angle` that outlived its
rename.
**Decision:** is `_mm` a deliberate marker for PnP-specific keys (document it) or pre-rename residue
(sweep it)? And is `support_overhang_angle` deleted outright?

## Search coverage

### What was searched

- **Manifests.** Every `[config.schema.*]` heading in `modules/core-modules/**/*.toml` and
  `modules/community-modules/**/*.toml`. **170 distinct keys** in core-modules, **173** including
  the `dragon-curve` community example. (An early count of 155 was wrong: it anchored the heading
  regex at column 0 and missed `arachne-perimeters`' indented block, which alone declares 42 keys.
  The 170/173 figures use an unanchored match and are the ones carried into this document.)
- **Padding tables.** `ORCA_CONFIG_PADDING` (**69 entries**) and `SUPPORT_CONFIG_DEFAULTS`
  (**3 entries**), both in `crates/slicer-gcode/src/serialize.rs`, read directly and counted with
  `awk` over the literal.
- **Host config.** `ResolvedConfig` in `crates/slicer-ir/src/resolved_config.rs` — **69 declared
  fields** plus `extensions`, of which **51** are emitted as literal keys by `to_config_map` (which
  also merges every `extensions` key verbatim). `FeedrateConfig` / `FEEDRATE_KEYS` in
  `crates/slicer-ir/src/feedrate.rs`.
- **Packet tables.** All 14 packet directories 253–266, `requirements.md` (falling back to
  `packet.spec.md`), **107 key rows** extracted.
- **Rename tickets.** Issue bodies 99–108, yielding a **34-key** rename pool: 24 renames, 2
  duplicate collapses, 6 keys ruled out (gap or PnP-specific), 1 pending (ticket 108).
- **Read sites.** 155 distinct keys were put through a read-site classification pass over
  `modules/*/src/**` and `crates/*/src/**`, excluding `tests/`, `benches/`, `resources/`, `docs/`,
  fixtures, padding entries, and CONFIG_BLOCK pass-throughs.
- **Canonical.** `../pinch_n_print_cli/OrcaSlicerDocumented/src/libslic3r/` for 109 keys, excluding
  `PrintConfig.cpp` / `.hpp` declarations, `ConfigManipulation.cpp`, `Preset.cpp` lists,
  `Print.cpp::invalidate_state_by_config_options` opt-key lists, and all of `src/slic3r/` (GUI).
- **Canonical defaults.** `PrintConfig.cpp` `set_default_value` lines read directly for the keys
  carrying a DEFAULT-MISMATCH disposition, plus `skirt_loops`, `skirt_distance`, `brim_width`,
  `support_interface_spacing`, `support_bottom_interface_spacing`, `support_angle`,
  `support_expansion`, `support_base_pattern_spacing`, `slowdown_for_curled_perimeters`.
- **Claim system.** `[claims]` blocks across all core-module manifests, and the `*_fill_holder`
  resolution path through `slicer-ir/src/resolved_config.rs`, `slicer-core/src/algos/region_mapping.rs`,
  and `slicer-core/src/algos/lightning/mod.rs`.

### What was skipped

- **`cargo` was not run at all** — no `build`, no `test`, no `metadata`. Nothing here is compiler-verified;
  a key that is read only through a macro-generated path could in principle have been missed by grep,
  though `declare_resolved_config!` was read directly to guard against exactly that.
- **`modules/community-modules/dragon-curve`** was enumerated but not given rows. It is a labeled
  example, not a shipped module, and no audited commit touched it.
- **Filament and machine-limit keys** (`machine_max_*`, `filament_*` beyond the packet-254 tower
  keys, `nozzle_temperature*`, `bed_temperature*`) were enumerated in `ResolvedConfig` and
  `machine-gcode-emit.toml` but not given rows — no audited commit touched them and they are not in
  packets 253–266.
- **`line_width` and its variants** (`line_width`, `initial_layer_line_width`, `bridge_line_width`,
  `outer_wall_line_width`, `inner_wall_line_width`, `sparse_infill_line_width`,
  `internal_solid_infill_line_width`, `top_surface_line_width`, `support_line_width`,
  `smaller_perimeter_line_width`) — declared on up to 12 manifests each, plainly live, untouched by
  the audited commits. Only `smaller_perimeter_line_width` has a row, because ticket 102 touched it.
- **`arachne-perimeters`' 24 beading-strategy keys** (`min_bead_width`, `wall_transition_*`,
  `min_length_factor`, `max_bead_count`, …). Untouched by the audited commits and out of the packet
  253–266 range; they belong to the packet-16x arachne parity workstream.
- **Speed keys** beyond those the rename tickets touched (`outer_wall_speed`, `inner_wall_speed`,
  `bridge_speed`, `overhang_*_speed`, `gap_infill_speed`, `top_surface_speed`,
  `internal_solid_infill_speed`, `thin_wall_speed`).
- **Packet 253's canonical-consumer column** was not independently re-verified row by row against
  the packet text; canonical liveness for its keys was derived fresh instead.

### Counts that could not be verified

- **The ticket-107 commit SHA in the task brief (`f8586585`) does not exist.** `git rev-parse`
  returns `fatal: Needed a single revision`. The real commit, found by subject line, is `f8606585`.
  All other eight rename SHAs resolved cleanly.
- **Port defaults marked `unverified`** — roughly 40 rows. The manifest `default =` line was read
  only for the owners where a mismatch was suspected or a ticket claimed an alignment. A blank is a
  gap in this audit, not evidence of a match.
- **Canonical read sites marked `unverified`** — roughly 45 rows, mostly perimeter and tree-support
  keys. These were confirmed to have in-tree reads, so their disposition (OK) does not depend on the
  canonical check; but their *scope* does, and a dead-in-canonical key among them would change its
  disposition.
- **Canonical defaults for most OK rows** were not read. The disposition gate for OK rows in this
  audit was "has a behavioural read site", not "default matches". **A row marked OK is not a claim
  that its default matches canonical.** Given that ticket 100 found the boolean gate had been
  vacuous, and that this audit found `slowdown_for_curled_perimeters` aligned backwards and
  `support_interface_spacing` mis-claimed as aligned, the OK rows should be assumed unchecked on
  defaults.
- **`skirt_loops` and `skirt_distance` read-site counts** are `unverified` — both keys were caught
  by the defaults comparison after the read-site sweep had already run, and were not re-dispatched.
  They are declared in `skirt-brim.toml` and their sibling keys in the same manifest are all live,
  so reads almost certainly exist, but this audit did not confirm them.
- **Packet 254's five `filament_tower_*` keys and two `enable_tower_interface_*` keys** were not
  checked for canonical liveness. If any is dead in `libslic3r/`, rule 3 puts it out of scope
  regardless of the rule-1 question.

## Decisions — 2026-09-01

Grilling session against this inventory, run at HEAD `72fedac9` (branch
`wayfinder/ticket-100-wipe-tower-rename`). Scope was the 140 in-scope rows
(`PADDING-ONLY`, `STUB`, `MECHANISM-VIOLATION`, `DEFAULT-MISMATCH`,
`BROKEN-RENAME`, `DEAD-IN-CANONICAL`, plus challenged `OK` rows); the 72
`NOT-YET-BUILT` rows and questions Q1, Q2, Q7, Q12 were out of scope and owned by
a parallel packet-authoring session. Q2's one live fragment
(`support_interface_spacing`) was folded into Q11. Every fact below was
re-derived in the grilling session; where it corrects this document, the
correction is stated.

| key(s) | question | ruling | action | rationale (one line) | follow-up ticket needed? |
|---|---|---|---|---|---|
| `sparse_infill_pattern`, `internal_solid_infill_pattern`, `top_surface_pattern`, `bottom_surface_pattern`, `support_interface_pattern`, `fuzzy_skin_noise_type`, `ironing_pattern`, `wipe_tower_wall_type`, `brim_type`, `support_base_pattern` | Q3(a) | holder-only, always | rule-out-of-scope | Algorithm-selecting enums are never declared keys; selection is by claim holder. Literal reading of rule 4. | no — return to queue as unimplemented |
| `sparse_fill_holder` and siblings | Q3(b) | unmatched holder must fail validation | implement | `resolve_held_claims` currently yields empty for every module, producing a silently hollow part; no `SchedulerError` variant covers it. | **yes** — new `SchedulerError` variant, scheduler-scoped |
| unimplemented Orca keys generally | Q3(c) | silent drop is correct | no-change | Port has no opinion on keys it does not implement; a reject list is itself a form of declaration and drifts. | no |
| `ironing_pattern`, `support_base_pattern` | Q3 impl. | remove existing manifest declarations | shed-to-queue | Both reads are non-behavioural (one validates, one builds a dead capability string); removal changes no slice output. | no |
| `seam_position`, `support_style`, `wall_sequence`, `retract_mode`, `wave_overhang_pattern` | Q8 | all OK / no-change | no-change | Rule 4 is triggered by cross-module algorithm selection, not in-module mode branching; these are the latter. map.md scope left unamended. | no |
| `fan_min_speed`, `fan_max_speed`, `overhang_fan_speed` | Q4(a) | convert port to percent 0–100 | align-default | Canonical declares all three min 0 / max 100; port's PWM scale makes an Orca `fan_max_speed = 100` slice at ~39% fan. Defaults are physically identical, so this is a unit fix. | **yes** — also binds packet 253's fan keys |
| `overhang_fan_speed` | Q4(b) | absolute, matching canonical | implement | Port computes it as a percentage of `fan_max_speed`; canonical assigns it directly and compares against current speed. | folded into Q4(a) |
| `ORCA_CONFIG_PADDING` (17 PADDING-ONLY rows) | Q5 | derive mechanically from resolved config | implement | Padding is load-bearing (Orca throws below 80 keys), so it cannot be retired — but hardcoded values must not be able to drift. | **yes** — must still guarantee ≥80 pairs |
| all 17 PADDING-ONLY rows | Q5 | padding is never coverage | shed-to-queue | Rule 2. Each returns to the queue as unimplemented unless separately ruled. | no |
| `enforce_support_layers`, `bridge_no_support` | Q13 | wire from `ResolvedConfig` | implement | `resolve_contact_params` hardcodes both under a comment claiming no config source; both are live fields, emitted into the config map. Defaults preserved. | no — in-session, no packet needed |
| `support_sharp_tails` | Q13 | remove field, hardcode `true` | rule-out-of-scope | Canonical lists it in `PrintConfig.cpp`'s obsolete-key `ignore` set (rule 3) and froze behaviour at `g_config_support_sharp_tails = true`; port runs it off. | **yes** — geometry change, own verification |
| `layer_id` | Q13 | struck from the finding | no-change | Overridden per-contact at the call site (`layer_id: *layer_index, ..base_params`); not severed. | no |
| `slowdown_for_curled_perimeters` | Q9 | revert `true` → `false` | align-default | Canonical is `ConfigOptionBoolsNullable{ false }`; ticket 100's "aligned to Orca" moved it the wrong way. | no |
| `ironing_speed` (support module) | Q11(a) | rename to `support_ironing_speed`, keep 30.0 | implement | Matches its siblings `support_ironing_flow` / `support_ironing_spacing`; removes a name collision with no canonical basis. | **yes** — decide `FEEDRATE_KEYS` membership |
| `sparse_infill_speed` | Q11(b) | `ResolvedConfig` default 50.0 → 100.0; `speed_factor` relative to resolved default, not `BASE_SPEED` | align-default | Modules receive the `ResolvedConfig` value via `to_config_map`, shadowing the manifests' 100.0; `BASE_SPEED = 50.0` only coincidentally yields factor 1.0. | **yes** — touches 3 infill modules |
| `support_interface_spacing` | Q11(c) | align both manifests 0.4 → 0.5 | align-default | Canonical `ConfigOptionFloat(0.5)`; packet 260's "default aligned" claim was false. | no |
| `skirt_loops`, `skirt_distance`, `brim_width` | Q14(a) | align to 1 / 2 / 0 | align-default | Port prints an 8 mm brim and 6 skirt loops by default; Orca prints none and one. | no |
| `skirt_brim_enabled` | Q14(b) | retire; derive from `skirt_loops > 0` OR `brim_width > 0` | shed-to-queue | Exact Orca round-trip; removes a PnP master switch canonical lacks. | no |
| `brim_ears` | Q14(c) | stays out of scope | rule-out-of-scope | Bool is genuinely dead in canonical; ticket 12's ruling holds. | no |
| `brim_ears_max_angle`, `brim_ears_detection_length` | Q14(c) | return to scope, queued | shed-to-queue | Live in `Brim.cpp::make_brim_ears_auto`, reached via `btBrimEars`, not the retired bool. Selected by a brim claim holder per Q3. | no |
| `apply_to_all` | Q10(a) | retire into `fuzzy_skin` | shed-to-queue | Canonical's scope enum (`disabled_fuzzy`/`external`/`hole`/`all`/`allwalls`) subsumes it; rule 5 forbids two gates. Not an algorithm enum, so Q3 does not apply. | no |
| `ironing_enabled` | Q10(b) | retire into `ironing_type` + `support_ironing` | shed-to-queue | `ironing_type = no ironing` is already the off state; the two modules stop sharing one gate. | no |
| `wipe_tower_max_purge_speed` | Q6(a) | rename from `wipe_tower_speed`, adopt cap semantic | implemented | `SPEED_KEYS` (`crates/slicer-ir/src/feedrate.rs`) exposes the canonical name; `DefaultGCodeEmitter::resolve_feedrate` uses `min(max_purge_speed, sparse_infill_speed)` for wipe-tower paths. Ticket 108 resolved; canonical min-10 validation rides ticket 113. | no |
| `FeedrateConfig` (all feedrate fields) | Q6(b) | add range validation | implement | Canonical declares `min = 10` here; the struct has no bounds machinery at all. | **yes** — canonical min/max per field **not derived this session** |
| `narrow_loop_length_threshold_mm`, `support_branch_merge_distance_mm`, `support_layer_height_mm`, `wave_overhang_anchor_depth_mm` | Q15(a) | `_mm` is a deliberate marker — document it | no-change | Suffix signals a PnP-invented dimensional key with no canonical counterpart; renamed keys had counterparts. | **yes** — document the convention |
| `support_overhang_angle` | Q15(b) | delete key, alias, and both tests | rule-out-of-scope | Removes a manifest declaration nothing reads. **Deliberate back-compat break** — old profiles fall into `extensions` silently. | no |

### In-scope keys not ruled on

| key(s) | disposition | why not ruled |
|---|---|---|
| `precise_outer_wall` | DEFAULT-MISMATCH | No question covered it. Ticket 100 deliberately declined to flip it (the port's precise-outer-wall path changes wall ordering); recorded as DEV-158 and still open. |
| `support_angle` | DEFAULT-MISMATCH | No question covered it. Port 60.0 vs canonical 0 ("alternate by layer") — a semantic difference, not just a value. |
| `slow_down_for_layer_cooling`, `slow_down_layer_time`, `slow_down_min_speed` | STUB | Q4 ruled the fan *scale*; these are the layer-time slowdown consumers, which no question reached. |
| `overhang_reverse_internal_only`, `overhang_reverse_threshold` | STUB | Underscore-bound-and-dropped in `arachne-perimeters`; found by the broad sweep, no question covered them. |
| `support_bottom_z_distance`, `support_critical_regions_only`, `support_object_first_layer_gap`, `support_remove_small_overhang` | STUB | Packet 265's declared-with-gap set; Q13 covered only the severed-plumbing keys. |
| `support_branch_merge_distance_mm` | STUB | Q15(a) documented its suffix but did not rule on its zero read sites; still an unimplemented PnP-invented key. |
| `fan_min_speed` (wiring) | STUB | Q4 ruled its unit; its zero read sites were not separately ruled. |
| the ~85 unchallenged `OK` rows | OK | Scored on "has a behavioural read site", never on default parity — and per carried finding 1 below, manifest-derived default checks are unreliable for this whole class. |

### Carried findings (not rulings)

1. **Manifest defaults are dead for plain-typed keys that also exist as `ResolvedConfig` fields.**
   `resolve_global_config` (`crates/slicer-scheduler/src/config_resolution.rs`) seeds from
   `ResolvedConfig::default()`, and its schema-default back-fill loop iterates
   `ConfigBoundsIndex::schema_defaults`, documented as holding `percent` / `float_or_percent`
   fields only. Module config is built from `ResolvedConfig::to_config_map()`
   (`crates/slicer-wasm-host/src/marshal/in_.rs`, `marshal/native.rs`), so for a plain-typed key
   the macro default reaches the module and the manifest `default =` is never consulted.
   **Every "default aligned to canonical" claim in tickets 99–107 and packets 253–266 that was
   verified by reading a manifest is unverified for this class of key.** Not enumerated this
   session. `sparse_infill_speed` (Q11(b)) is the confirmed instance.
2. `slowdown_for_curled_perimeters` is a scalar `bool` in the port and a nullable per-nozzle
   `coBools` in canonical — an unrecorded shape divergence. Raised in Q9, not ruled.
3. The port's `skirt_loops` declares `max = 20`; canonical declares `max = 10`. A bounds
   divergence this document did not record. Raised in Q14, not ruled.

### Corrections to this document

- **`overhang_fan_speed` is not on the 0–255 scale.** Its declared range in `part-cooling.toml` is
  0–100 and `part-cooling/src/lib.rs` computes `(overhang_fan_speed * fan_max_speed) / 100`. The
  row's "100 means full in Orca and about 39% in the port" is wrong; at defaults
  `(100 x 255)/100 = 255`, i.e. full. The real divergence is semantic (relative-to-max vs absolute).
- **`fan_min_speed` and `fan_max_speed` defaults are not value mismatches.** 51/255 = 20% and
  255/255 = 100%, exactly canonical's 20 and 100. Both are unit divergences, not wrong values.
- **`layer_id` is not a fourth instance of the severed-plumbing bug.** The consumer call site in
  `support_analysis_producer.rs` rebuilds the struct per contact with
  `layer_id: *layer_index, ..base_params.clone()`, so the `0` is always overridden. The genuinely
  severed set is `enforce_support_layers`, `bridge_no_support`, `support_sharp_tails`.
- **`support_sharp_tails` is additionally hardcoded against its own declared default.**
  `ResolvedConfig` declares it `true`; `resolve_contact_params` hardcodes `false`. It is also
  dead in canonical as a config key (in `PrintConfig.cpp`'s obsolete-key `ignore` set), with
  behaviour frozen at `static constexpr bool g_config_support_sharp_tails = true`
  (`src/libslic3r/libslic3r.h`).
- **`support_overhang_angle` is not an alias that "outlived its rename".** It is a deliberate,
  documented, tested back-compat alias — one of two entries in `CONFIG_KEY_ALIASES`, covered by
  `legacy_support_overhang_angle_alias_resolves` and an alias-conflict test in
  `crates/slicer-scheduler/tests/integration/config_resolution_tdd.rs`. The genuine oddity is that
  it is *also* still declared in `traditional-support-planner.toml`, which is what produced the
  zero-read-sites finding.
- **`ORCA_CONFIG_PADDING` is load-bearing, not cosmetic.**
  `ConfigBase::load_from_gcode_file` (`Config.cpp`) throws `Slic3r::RuntimeError` when a
  CONFIG_BLOCK yields fewer than 80 key-value pairs, on the modern delimited path this port emits.
  The table's `emitted.len() >= 96` break is a deliberate margin over that floor. Its doc comment
  calling the table "neutral cosmetic" is false.
- **The `_mm` key set is five, not four.** `wave_overhang_flow_mm3_per_mm` was missed by the
  broad sweep; its suffix is a unit (mm3/mm), not the PnP-provenance marker.
