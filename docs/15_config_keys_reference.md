# ModularSlicer — Config Keys Reference

This file is the canonical catalog of config keys recognised by the host
binary, core modules, and the resolved-config pipeline. It is grouped by
functional domain. For the manifest **schema rules** (table format, valid
types, validation expressions) see `docs/03_wit_and_manifest.md`. For
namespaced override conventions (`object_config:`, `paint_config:`) see also
`docs/02_ir_schemas.md` IR 5 "Config Key Namespaces".

Each entry below lists:

- **Key** — runtime string used in `ConfigView::get_*` and CLI flat-key form.
- **Type** — `bool` / `int` / `float` / `string` / `enum`. See
  `docs/03_wit_and_manifest.md` "Config Field Types Reference".
- **Default** — value when the user does not set the key.
- **Range / values** — clamp bounds or accepted enum strings.
- **Packet** — packet that introduced the key.
- **Module(s)** — primary consumer(s). `[host]` means consumed by a host
  built-in (e.g. `DefaultGCodeEmitter`).
- **Source-of-truth** — definitive file (manifest TOML for module-owned keys;
  the relevant consumer struct under `crates/slicer-host/src/` for
  host-registered keys — e.g. `gcode_emit.rs::FeedrateConfig` for per-role
  print speeds. <!-- VERIFY: there is no single `crates/slicer-host/src/config_schema.rs`
  file; host-registered defaults live alongside their consumers. -->).

---

## Print speeds (packet 52)

26 per-role float keys (mm/s); registered in
the consumer's config struct under `crates/slicer-host/src/` (e.g. `gcode_emit.rs::FeedrateConfig`). Consumed by
`DefaultGCodeEmitter::resolve_feedrate(role, paint_layer, …)` which emits
F-tokens in mm/min (see `docs/08_coordinate_system.md` "F-Token Formatting
Convention").

| Key | Default (mm/s) | Range | Module |
|---|---|---|---|
| `outer_wall_speed` | `60.0` | `> 0` | `[host]` |
| `inner_wall_speed` | `80.0` | `> 0` | `[host]` |
| `thin_wall_speed` | `30.0` | `> 0` | `[host]` |
| `top_surface_speed` | `30.0` | `> 0` | `[host]` |
| `bottom_surface_speed` | `40.0` | `> 0` | `[host]` |
| `sparse_infill_speed` | `100.0` | `> 0` | `[host]` |
| `bridge_speed` | `25.0` | `> 0` | `[host]` |
| `internal_bridge_speed` | `25.0` | `> 0` | `[host]` |
| `support_speed` | `50.0` | `> 0` | `[host]` |
| `support_interface_speed` | `40.0` | `> 0` | `[host]` |
| `gap_infill_speed` | `30.0` | `> 0` | `[host]` |
| `ironing_speed` | `20.0` | `> 0` | `[host]` |
| `skirt_speed` | `40.0` | `> 0` | `[host]` |
| `wipe_tower_speed` | `50.0` | `> 0` | `[host]` |
| `prime_tower_speed` | `50.0` | `> 0` | `[host]` |
| `travel_speed` | `200.0` | `> 0` | `[host]` |
| `travel_speed_z` | `12.0` | `> 0` | `[host]` |
| `initial_layer_speed` | `20.0` | `> 0` | `[host]` |
| `initial_layer_infill_speed` | `30.0` | `> 0` | `[host]` |
| `initial_layer_travel_speed` | `100.0` | `> 0` | `[host]` |
| `wipe_speed` | `80.0` | `> 0` | `[host]` |
| `filament_ironing_speed` | `0.0` (= use `ironing_speed`) | `≥ 0` | `[host]` |
| `overhang_1_4_speed` | `0.0` (= no override) | `≥ 0` | `[host]` (packet 57) |
| `overhang_2_4_speed` | `0.0` | `≥ 0` | `[host]` (packet 57) |
| `overhang_3_4_speed` | `0.0` | `≥ 0` | `[host]` (packet 57) |
| `overhang_4_4_speed` | `0.0` | `≥ 0` | `[host]` (packet 57) |

`filament_ironing_speed > 0.0` overrides `ironing_speed` for the `Ironing`
role. The four `overhang_*_4_speed` keys all-zero short-circuits the overhang
classifier for byte-identical pre-packet-57 output.

---

## Cooling and fan (packet 53)

Eight keys consumed by the `part-cooling` finalization-stage module
(`modules/core-modules/part-cooling/`). Registered in
the consumer's config struct under `crates/slicer-host/src/` (e.g. `gcode_emit.rs::FeedrateConfig`).

| Key | Type | Default | Range | Notes |
|---|---|---|---|---|
| `fan_speed_min` | int | `0` | `0–255` | Lower fan PWM bound. |
| `fan_speed_max` | int | `255` | `0–255` | Upper fan PWM bound. |
| `disable_fan_first_layers` | int | `1` | `0–10` | Force fan off below this layer index. |
| `enable_overhang_fan` | bool | `true` | — | Modulate fan on overhang quartiles 3–4. |
| `overhang_fan_speed` | int | `255` | `0–255` | Fan PWM during enabled overhangs. |
| `slow_down_for_layer_cooling` | bool | `true` | — | Reduce speed when below `slow_down_layer_time`. |
| `slow_down_min_speed` | float | `20.0` (mm/s) | `> 0` | Floor for the slowdown action. |
| `slow_down_layer_time` | float | `5.0` (s) | `> 0` | Threshold layer time below which slowdown engages. |

---

## Support (packet 31b + packet 28/30)

Eleven keys split across `support-planner` and `tree-support` core modules.
Source-of-truth: each module's manifest TOML
(`modules/core-modules/{support-planner,tree-support}/manifest.toml`).

| Key | Type | Default | Range | Module |
|---|---|---|---|---|
| `support_layer_height_mm` | float | `0.0` (= use model layer height) | `0.0`, `[0.05, 1.0]` | `support-planner` |
| `support_top_z_distance_mm` | float | `0.0` | `[0.0, 5.0]` | `support-planner` |
| `tree_support_branch_angle` | float (deg) | `40.0` | `[0, 89]` | `support-planner` |
| `tree_support_branch_diameter` | float (mm) | `2.0` | `> 0` | `support-planner` |
| `tree_support_branch_diameter_angle` | float (deg) | `5.0` | `[0, 89]` | `support-planner` |
| `tree_support_branch_distance` | float (mm) | `1.0` | `> 0` | `support-planner` |
| `tree_support_wall_count` | int | `1` | `≥ 0` | `tree-support` |
| `support_raft_layers` | int | `0` | `≥ 0` | `support-planner` |
| `support_interface_top_layers` | int | `2` | `≥ 0` | `support-planner` |
| `support_interface_bottom_layers` | int | `2` | `≥ 0` | `support-planner` |
| `tree_support_interface_spacing_mm` | float | `0.2` | `> 0` | `tree-support` |

---

## Extrusion mode (packet 54)

| Key | Type | Default | Notes |
|---|---|---|---|
| `use_relative_e_distances` | bool | `true` (M83) | `false` selects M82. The serializer issues `G92 E0` on mode transition and layer reset. Source-of-truth: the consumer's config struct under `crates/slicer-host/src/` (e.g. `gcode_emit.rs::FeedrateConfig`). |

---

## Retraction mode (packet 34)

| Key | Type | Default | Values | Module |
|---|---|---|---|---|
| `retract_mode` | enum | `"gcode"` | `"gcode"`, `"firmware"` | `path-optimization-default` |

- `"gcode"` → inline `G1 E-<length> F<speed>` retract / `G1 E<length> F<speed>` unretract.
- `"firmware"` → `G10` / `G11`. Length and speed remain in IR for diagnostics
  but are not serialized.

Source-of-truth: `modules/core-modules/path-optimization-default/manifest.toml`.

---

## G-code preamble (packet 55)

Four keys feeding the `HEADER_BLOCK_*` envelope. See
`docs/02_ir_schemas.md` "G-code envelope blocks" for the full envelope
format. Registered in the consumer's config struct under `crates/slicer-host/src/` (e.g. `gcode_emit.rs::FeedrateConfig`).

| Key | Type | Default | Range | Notes |
|---|---|---|---|---|
| `filament_diameter` | float (mm) | `1.75` | `[0.5, 5.0]` | Header line; consumed by post-processors. |
| `filament_density` | float (g/cm³) | `1.24` | `[0.5, 5.0]` | Header line. |
| `max_z_height` | float (mm) | `0.0` (= auto) | `≥ 0` | Hard cap reported in header; `0.0` means "use per-print z_max". |
| `thumbnail_path` | string | `""` | — | Alternative to the `--thumbnail` CLI flag. CLI wins when both set. |

CLI flag: `--thumbnail <PATH>` for the PNG file; encoded as
`THUMBNAIL_BLOCK_*` Base64 chunks (76 chars / line, `; ` prefix).

---

## Machine start / end G-code (packet 59)

Four keys read by the designated `PostPass::LayerFinalization` machine-gcode
module (default: `machine-gcode-emit`). Source-of-truth:
`modules/core-modules/machine-gcode-emit/manifest.toml`.

| Key | Type | Default | Range | Notes |
|---|---|---|---|---|
| `machine_start_gcode` | string | `""` | — | Template; supports `[key]` placeholder substitution. |
| `machine_end_gcode` | string | `""` | — | Template; supports `[key]` placeholder substitution. |
| `bed_temperature_initial_layer_single` | int | `60` | `[0, 120]` | `°C`; available as `[bed_temperature_initial_layer_single]` macro. |
| `nozzle_temperature_initial_layer` | int | `215` | `[0, 300]` | `°C`; available as `[nozzle_temperature_initial_layer]` macro. |

Supported macros inside templates (square-bracket placeholders only, no
arithmetic / conditionals):

`[first_layer_temperature]`, `[bed_temperature]`, `[filament_type]`,
`[nozzle_diameter]`, `[tool_count]`, `[layer_count]`,
`[print_time_estimate_s]`, `[x_max]`, `[y_max]`, `[z_max]`,
`[bed_temperature_initial_layer_single]`, `[nozzle_temperature_initial_layer]`.

---

## Slicing precision (packet 60)

Seven keys carried on `ResolvedConfig`; all-zero / defaults short-circuit to
byte-identical pre-packet-60 output. Registered in
the consumer's config struct under `crates/slicer-host/src/` (e.g. `gcode_emit.rs::FeedrateConfig`); `perimeter_arc_tolerance`
additionally registered in `classic-perimeters` and `arachne-perimeters`
manifests.

| Key | Type | Default | Range | Consumer |
|---|---|---|---|---|
| `gcode_resolution` | float (mm) | `0.0125` | `≥ 0` | `[host]` D-P tolerance for walls / brim. |
| `infill_resolution` | float (mm) | `0.0125` | `≥ 0` | `[host]` D-P tolerance for infill / bridge / top / bottom. |
| `support_resolution` | float (mm) | `0.05` | `≥ 0` | `[host]` D-P tolerance for support / interface. |
| `min_segment_length` | float (mm) | `0.025` | `≥ 0` | `[host]` short-segment dropper. |
| `gcode_xy_decimals` | int | `3` | `[1, 6]` | `[host]` X / Y / Z token formatting. |
| `perimeter_arc_tolerance` | float (mm) | `0.0025` | `≥ 0` | `classic-perimeters`, `arachne-perimeters`. |
| `slice_closing_radius` | float (mm) | `0.0` (off) | `≥ 0` | `[host]` per-layer Clipper2 close. |

See `docs/02_ir_schemas.md` "Polyline simplification and precision" for the
per-role tolerance dispatch table.

---

## Multi-layer shell thickness (packet 35)

| Key | Type | Default | Range | Notes |
|---|---|---|---|---|
| `top_shell_layers` | int | `3` | `[1, 10]` | **Default deviates from OrcaSlicer's `4`.** Window for top-surface classification in `classify_region_surfaces`. |
| `bottom_shell_layers` | int | `3` | `[1, 10]` | Window for bottom-surface classification. |

Source-of-truth: the consumer's config struct under `crates/slicer-host/src/` (e.g. `gcode_emit.rs::FeedrateConfig`). Per-region
override is automatic via `RegionMapIR.entries[*].config` once `RegionMapping`
runs.

---

## Fill-role claims (packet 37)

Four global keys select the holder for each fill-role claim. See
`docs/04_host_scheduler.md` validation pass 2 for conflict-resolution rules.

| Key | Type | Default | Values | Selects |
|---|---|---|---|---|
| `claims.top-fill` | string (module ID) | `"rectilinear-infill"` | any loaded module that declares `holds = ["top-fill"]` | `claim:top-fill` holder. |
| `claims.bottom-fill` | string | `"rectilinear-infill"` | as above for `bottom-fill` | `claim:bottom-fill` holder. |
| `claims.bridge-fill` | string | `"rectilinear-infill"` | as above for `bridge-fill` | `claim:bridge-fill` holder. |
| `claims.sparse-fill` | string | `"rectilinear-infill"` | as above for `sparse-fill` | `claim:sparse-fill` holder. |

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
  1. Choose host-registered (`config_schema.rs`) **only if** the key is
     consumed by a host built-in or a finalization-stage module that ships
     in-tree. Otherwise the module manifest is the right home.
  2. Add the new entry to this file in the appropriate section.
  3. Cross-reference from the relevant packet's design doc.
- Removing a key requires a major IR / WIT bump (see
  `docs/02_ir_schemas.md` "IR Versioning Contract").
- This file is enumerated; the `docs/03_wit_and_manifest.md` "Config Field
  Types Reference" remains the source of truth for the meta-format
  (`type`, `min`, `max`, `unit`, `display`, `group`, `advanced`).
