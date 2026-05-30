# ModularSlicer — Config Keys Reference

This file is the canonical catalog of config keys recognised by the host
binary, core modules, and the resolved-config pipeline. For the manifest
**schema rules** (table format, valid types, validation expressions) see
`docs/03_wit_and_manifest.md`. For namespaced override conventions
(`object_config:`, `paint_config:`) see also `docs/02_ir_schemas.md` IR 5
"Config Key Namespaces".

> **Generated, not hand-maintained.** The three tables below marked _(generated)_
> are produced by `cargo xtask gen-config-docs` from the authoritative sources —
> module `[config.schema]` manifests and `docs/config/host-keys.toml` (itself
> locked to the live code defaults — `FeedrateConfig::default()`,
> `ResolvedConfig::default()`, and named `DEFAULT_*` constants — by the
> `gcode_emit::host_keys_doc_lock` slicer-runtime test). Do not edit them by hand;
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
  `gcode_emit.rs::FeedrateConfig`) and are mirrored into
  `docs/config/host-keys.toml`. There is no single `config_schema.rs` file.

---

## Module-owned config keys (generated)

Every `[config.schema]` key across `modules/core-modules/*/<name>.toml`. The
per-domain sections lower in this file add prose for these same keys; this table
is the authoritative catalog of their defaults and ranges.

<!-- BEGIN GENERATED: module-config-keys (cargo xtask gen-config-docs) -->
| Key | Type | Default | Range | Module |
|---|---|---|---|---|
| `inner_wall_speed` | float | `45.0` | [1.0, 300.0] | `arachne-perimeters` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `arachne-perimeters` |
| `outer_wall_speed` | float | `30.0` | [1.0, 300.0] | `arachne-perimeters` |
| `perimeter_arc_tolerance` | float | `0.0125` | [0.0, 1.0] | `arachne-perimeters` |
| `wall_count` | int | `3` | [1.0, 10.0] | `arachne-perimeters` |
| `inner_wall_speed` | float | `45.0` | [1.0, 300.0] | `classic-perimeters` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `classic-perimeters` |
| `outer_wall_speed` | float | `30.0` | [1.0, 300.0] | `classic-perimeters` |
| `perimeter_arc_tolerance` | float | `0.0125` | [0.0, 1.0] | `classic-perimeters` |
| `wall_count` | int | `3` | [1.0, 10.0] | `classic-perimeters` |
| `apply_to_all` | bool | `false` | — | `fuzzy-skin` |
| `point_distance` | float | `0.5` | [0.01, 5.0] | `fuzzy-skin` |
| `thickness` | float | `0.3` | [0.0, 2.0] | `fuzzy-skin` |
| `infill_angle` | float | `45.0` | [0.0, 360.0] | `gyroid-infill` |
| `infill_density` | float | `20.0` | [0.0, 100.0] | `gyroid-infill` |
| `infill_speed` | float | `60.0` | [1.0, 300.0] | `gyroid-infill` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `gyroid-infill` |
| `first_layer_height` | float | `0.3` | [0.01, 1.0] | `layer-planner-default` |
| `layer_height` | float | `0.2` | [0.01, 1.0] | `layer-planner-default` |
| `infill_density` | float | `20.0` | [0.0, 100.0] | `lightning-infill` |
| `infill_speed` | float | `60.0` | [1.0, 300.0] | `lightning-infill` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `lightning-infill` |
| `bed_temperature_initial_layer_single` | int | `60` | [0.0, 120.0] | `machine-gcode-emit` |
| `machine_end_gcode` | string | `"PRINT_END"` | — | `machine-gcode-emit` |
| `machine_start_gcode` | string | `"M190 S[bed_temperature_initial_layer_single]\nM…"` | — | `machine-gcode-emit` |
| `nozzle_temperature_initial_layer` | int | `215` | [0.0, 300.0] | `machine-gcode-emit` |
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
| `wall_sequence` | enum | `"inner_outer"` | — | `path-optimization-default` |
| `infill_angle` | float | `45.0` | [0.0, 360.0] | `rectilinear-infill` |
| `infill_density` | float | `20.0` | [0.0, 100.0] | `rectilinear-infill` |
| `infill_speed` | float | `60.0` | [1.0, 300.0] | `rectilinear-infill` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `rectilinear-infill` |
| `seam_mode` | string | `"nearest"` | — | `seam-placer` |
| `seam_mode` | string | `"nearest"` | — | `seam-planner-default` |
| `brim_width` | float | `8.0` | [0.0, 30.0] | `skirt-brim` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `skirt-brim` |
| `skirt_brim_enabled` | bool | `true` | — | `skirt-brim` |
| `skirt_distance` | float | `3.0` | [0.0, 20.0] | `skirt-brim` |
| `skirt_height` | int | `1` | [1.0, 10.0] | `skirt-brim` |
| `skirt_loops` | int | `6` | [0.0, 20.0] | `skirt-brim` |
| `support_enabled` | bool | `true` | — | `support-planner` |
| `support_interface_bottom_layers` | int | `-1` | [-1.0, 10.0] | `support-planner` |
| `support_interface_top_layers` | int | `2` | [0.0, 10.0] | `support-planner` |
| `support_layer_height_mm` | float | `0.0` | [0.05, 1.0] | `support-planner` |
| `support_raft_layers` | int | `0` | [0.0, 20.0] | `support-planner` |
| `support_top_z_distance_mm` | float | `0.0` | [0.0, 5.0] | `support-planner` |
| `tree_support_branch_angle` | float | `45.0` | [0.0, 75.0] | `support-planner` |
| `tree_support_branch_diameter` | float | `5.0` | [0.5, 20.0] | `support-planner` |
| `tree_support_branch_diameter_angle` | float | `5.0` | [0.0, 90.0] | `support-planner` |
| `tree_support_branch_distance` | float | `1.0` | [0.1, 10.0] | `support-planner` |
| `tree_support_interface_spacing_mm` | float | `0.4` | [0.1, 2.0] | `support-planner` |
| `tree_support_wall_count` | int | `1` | [1.0, 10.0] | `support-planner` |
| `ironing_enabled` | bool | `false` | — | `support-surface-ironing` |
| `ironing_flow_rate` | float | `100.0` | [1.0, 200.0] | `support-surface-ironing` |
| `ironing_spacing` | float | `0.1` | [0.01, 1.0] | `support-surface-ironing` |
| `ironing_speed` | float | `30.0` | [1.0, 300.0] | `support-surface-ironing` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `support-surface-ironing` |
| `ironing_enabled` | bool | `true` | — | `top-surface-ironing` |
| `ironing_flow` | float | `0.1` | [0.01, 1.0] | `top-surface-ironing` |
| `ironing_pattern` | string | `"rectilinear"` | — | `top-surface-ironing` |
| `ironing_spacing_mm` | float | `0.1` | [0.01, 1.0] | `top-surface-ironing` |
| `ironing_speed` | float | `20.0` | [1.0, 300.0] | `top-surface-ironing` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `traditional-support` |
| `support_angle` | float | `60.0` | [0.0, 90.0] | `traditional-support` |
| `support_density` | float | `20.0` | [0.0, 100.0] | `traditional-support` |
| `support_enabled` | bool | `true` | — | `traditional-support` |
| `support_speed` | float | `50.0` | [1.0, 300.0] | `traditional-support` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `tree-support` |
| `support_angle` | float | `60.0` | [0.0, 90.0] | `tree-support` |
| `support_density` | float | `20.0` | [0.0, 100.0] | `tree-support` |
| `support_enabled` | bool | `true` | — | `tree-support` |
| `support_layer_height_mm` | float | `0.0` | [0.05, 1.0] | `tree-support` |
| `support_speed` | float | `50.0` | [1.0, 300.0] | `tree-support` |
| `support_top_z_distance_mm` | float | `0.0` | [0.0, 5.0] | `tree-support` |
| `bed_shape` | float-list | `—` | — | `wipe-tower` |
| `line_width` | float | `0.4` | [0.1, 2.0] | `wipe-tower` |
| `retract_length` | float | `2.0` | [0.0, 20.0] | `wipe-tower` |
| `wipe_tower_enabled` | bool | `true` | — | `wipe-tower` |
| `wipe_tower_purge_volume` | float | `10.0` | [1.0, 50.0] | `wipe-tower` |
| `wipe_tower_width` | float | `60.0` | [1.0, 100.0] | `wipe-tower` |
| `wipe_tower_x` | float | `10.0` | [0.0, 300.0] | `wipe-tower` |
| `wipe_tower_y` | float | `10.0` | [0.0, 300.0] | `wipe-tower` |
<!-- END GENERATED: module-config-keys -->

## Host-registered config keys (generated)

Keys consumed by host built-ins, mirrored from their code source of truth
(`gcode_emit.rs::FeedrateConfig` for per-role speeds in mm/s;
`resolved_config.rs::ResolvedConfig` for shell-window / slicing-precision /
fill-role keys; named `DEFAULT_*` constants in `run.rs` / `pipeline.rs` for keys
read directly from the config source) via `docs/config/host-keys.toml`, which the
`gcode_emit::host_keys_doc_lock` slicer-runtime test holds equal to those defaults
(the speed check is exhaustive — adding a `FeedrateConfig` field fails the build
until it is documented). Per-role speeds feed
`DefaultGCodeEmitter::resolve_feedrate(role, paint_layer, …)`, which emits F-tokens
in mm/min (see `docs/08_coordinate_system.md` "F-Token Formatting Convention").

<!-- BEGIN GENERATED: host-speeds (cargo xtask gen-config-docs) -->
| Key | Type | Default | Range | Source |
|---|---|---|---|---|
| `bottom_surface_speed` | float | `100.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `bridge_speed` | float | `25.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `filament_ironing_speed` | float | `0.0` | >= 0 (0 = use ironing_speed) | `gcode_emit.rs::FeedrateConfig` |
| `gap_infill_speed` | float | `30.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `initial_layer_infill_speed` | float | `60.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `initial_layer_speed` | float | `30.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `initial_layer_travel_speed` | float | `120.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `inner_wall_speed` | float | `60.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `internal_bridge_speed` | float | `37.5` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `ironing_speed` | float | `20.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `outer_wall_speed` | float | `60.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `overhang_1_4_speed` | float | `0.0` | >= 0 (0 = no override (packet 57)) | `gcode_emit.rs::FeedrateConfig` |
| `overhang_2_4_speed` | float | `0.0` | >= 0 (0 = no override (packet 57)) | `gcode_emit.rs::FeedrateConfig` |
| `overhang_3_4_speed` | float | `0.0` | >= 0 (0 = no override (packet 57)) | `gcode_emit.rs::FeedrateConfig` |
| `overhang_4_4_speed` | float | `0.0` | >= 0 (0 = no override (packet 57)) | `gcode_emit.rs::FeedrateConfig` |
| `prime_tower_speed` | float | `90.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `skirt_speed` | float | `50.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `sparse_infill_speed` | float | `100.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `support_interface_speed` | float | `80.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `support_speed` | float | `80.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `thin_wall_speed` | float | `30.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `top_surface_speed` | float | `100.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `travel_speed` | float | `120.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `travel_speed_z` | float | `0.0` | >= 0 (0 = use travel_speed for Z) | `gcode_emit.rs::FeedrateConfig` |
| `wipe_speed` | float | `96.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `wipe_tower_speed` | float | `90.0` | > 0 | `gcode_emit.rs::FeedrateConfig` |
| `bottom_fill_holder` | string | `"rectilinear-infill"` | — (holder of claim:bottom-fill (packet 37)) | `resolved_config.rs::ResolvedConfig` |
| `bottom_shell_layers` | int | `3` | [1, 10] | `resolved_config.rs::ResolvedConfig` |
| `bridge_fill_holder` | string | `"rectilinear-infill"` | — (holder of claim:bridge-fill (packet 37)) | `resolved_config.rs::ResolvedConfig` |
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
<!-- END GENERATED: host-speeds -->

`filament_ironing_speed > 0.0` overrides `ironing_speed` for the `Ironing` role.
The four `overhang_*_4_speed` keys all-zero short-circuits the overhang
classifier for byte-identical pre-packet-57 output.

## Deviations from OrcaSlicer (generated)

Generated keys whose numeric default differs from the matching key in
`docs/ORCA_CONFIG_REFERENCE.md` (the upstream snapshot). Everything else matches
upstream or has no upstream equivalent.

<!-- BEGIN GENERATED: orca-deviations (cargo xtask gen-config-docs) -->
| Key | Owner | ModularSlicer default | OrcaSlicer default |
|---|---|---|---|
| `brim_width` | `skirt-brim` | `8.0` | `0.0` |
| `inner_wall_speed` | `arachne-perimeters` | `45.0` | `60.0` |
| `inner_wall_speed` | `classic-perimeters` | `45.0` | `60.0` |
| `ironing_speed` | `support-surface-ironing` | `30.0` | `20.0` |
| `nozzle_temperature_initial_layer` | `machine-gcode-emit` | `215` | `200.0` |
| `outer_wall_speed` | `arachne-perimeters` | `30.0` | `60.0` |
| `outer_wall_speed` | `classic-perimeters` | `30.0` | `60.0` |
| `skirt_distance` | `skirt-brim` | `3.0` | `2.0` |
| `skirt_loops` | `skirt-brim` | `6` | `1.0` |
| `support_angle` | `traditional-support` | `60.0` | `0.0` |
| `support_angle` | `tree-support` | `60.0` | `0.0` |
| `support_interface_top_layers` | `support-planner` | `2` | `3.0` |
| `support_speed` | `traditional-support` | `50.0` | `80.0` |
| `support_speed` | `tree-support` | `50.0` | `80.0` |
| `top_shell_layers` | `resolved_config.rs::ResolvedConfig` | `3` | `4.0` |
| `tree_support_branch_angle` | `support-planner` | `45.0` | `40.0` |
| `tree_support_branch_distance` | `support-planner` | `1.0` | `5.0` |
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

Keys split across the `support-planner` and `tree-support` core modules.
Defaults and ranges are in the generated **Module-owned config keys** table
above (modules `support-planner`, `tree-support`). Note `support_layer_height_mm
= 0.0` means "use the model layer height".

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

The one user config key here is `thumbnail_path` (default `""`), in the generated
**Host-registered config keys** table above. An absent/empty value emits no
`THUMBNAIL_BLOCK`; the `--thumbnail <PATH>` CLI flag overrides it (CLI wins). The
PNG is encoded as `THUMBNAIL_BLOCK_*` Base64 chunks (76 chars/line, `; ` prefix).

The G-code header also emits `; filament_diameter`, `; filament_density`, and
`; max_z_height` comment lines, but **these are not user config keys** — there is
no `config_source` key for them:

- `filament_diameter` / `filament_density` are emitter constants
  (`1.75 mm` / `1.24 g·cm⁻³`) on `DefaultGCodeEmitter`
  (`crates/slicer-runtime/src/gcode_emit.rs`), overridable only programmatically
  via `with_filament_config(...)`. Wiring them to config keys is a future
  enhancement, not a current capability.
- `max_z_height` in the header is the **computed** top-layer Z (with fallback
  floor `max_z_height_floor_mm = 256.0`), not a settable key.

See `docs/02_ir_schemas.md` "G-code envelope blocks" for the full envelope format.

---

## Machine start / end G-code (packet 59)

Keys read by the designated `PostPass::LayerFinalization` machine-gcode module
(default: `machine-gcode-emit`). Defaults and ranges are in the generated
**Module-owned config keys** table above (module `machine-gcode-emit`).
`machine_start_gcode` / `machine_end_gcode` are templates supporting `[key]`
placeholder substitution.

Supported macros inside templates (square-bracket placeholders only, no
arithmetic / conditionals):

`[first_layer_temperature]`, `[bed_temperature]`, `[filament_type]`,
`[nozzle_diameter]`, `[tool_count]`, `[layer_count]`,
`[print_time_estimate_s]`, `[x_max]`, `[y_max]`, `[z_max]`,
`[bed_temperature_initial_layer_single]`, `[nozzle_temperature_initial_layer]`.

---

## Slicing precision (packet 60)

The host precision keys (`gcode_resolution`, `infill_resolution`,
`support_resolution`, `min_segment_length`, `gcode_xy_decimals`,
`slice_closing_radius`) carried on `ResolvedConfig` are in the generated
**Host-registered config keys** table above; `perimeter_arc_tolerance` is
module-owned (`classic-perimeters`, `arachne-perimeters`) and appears in the
generated **Module-owned config keys** table. Defaults / all-zero short-circuit
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
     `gcode_emit.rs` (`host_keys_doc_lock`), then run `cargo xtask gen-config-docs`.
  4. Cross-reference from the relevant packet's design doc.
- Removing a key requires a major IR / WIT bump (see
  `docs/02_ir_schemas.md` "IR Versioning Contract").
- This file is enumerated; the `docs/03_wit_and_manifest.md` "Config Field
  Types Reference" remains the source of truth for the meta-format
  (`type`, `min`, `max`, `unit`, `display`, `group`, `advanced`).
