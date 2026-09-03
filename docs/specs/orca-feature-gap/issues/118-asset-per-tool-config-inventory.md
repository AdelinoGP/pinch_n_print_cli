# 118 asset — the port's per-tool (filament / extruder) config mechanism

Produced by [ticket 118](./118-inventory-per-tool-config-mechanism.md). Every
claim below was read from the tree at the time of writing; re-derive before
depending on a count.

## Executive answer

**A general per-tool config mechanism already exists, and it is far more general
than the `filament_density_for` instance ticket 28 found.** It is
`tool_config:<tool_index>:<key>` — an override namespace on the raw config
source that produces a *whole* `ResolvedConfig` per tool index. Any of the 79
CLI-bound `ResolvedConfig` fields, plus any module-manifest key, can be set per
tool through it today, with no schema, WIT or IR change.

What is missing is not the mechanism. It is four things, in descending order of
how much they block:

1. **Nothing ingests Orca's per-tool vectors into it.** Orca does not spell
   per-filament config as `tool_config:1:foo`; it spells it as a `coFloats`
   *list* on one key. Exactly **one** key in this port is wired that way
   (`filament_density`), and the generic scalar extractor silently keeps
   element 0 of any list it is handed. So a real Orca 3MF with two filaments
   loads filament 1's values and discards filament 2's, without an error.
2. **The composition step that carries a per-tool overlay into region config is
   a hand-maintained 29-field allowlist**, against 83 declared fields. A
   `tool_config:<n>:<key>` naming any other declared field resolves correctly
   and is then dropped.
3. **The tool axis only reaches geometry through the paint variant chain.** A
   region gets its per-tool overlay only if it carries a
   `("material", ToolIndex(n))` chain entry. There is no notion of "this whole
   object prints with tool 2".
4. **No core module declares the one key that carries the tool count**, so
   `tool-count()` returns 1 everywhere in the shipped pipeline.

Ticket 118 does not rule on what to do about any of that. See
[Questions the ruling must answer](#questions-the-ruling-must-answer).

---

## Stage 1 — Ingest

### The config source is a flat `HashMap<String, ConfigValue>`

`ConfigValue` (`crates/slicer-ir/src/slice_ir.rs`) has a `List(Vec<ConfigValue>)`
variant, so a list is expressible at every ingest point.

- **CLI / JSON.** `parse_cli_config_source`
  (`crates/slicer-scheduler/src/execution_plan.rs`) parses a flat JSON object;
  `json_to_config_value` maps a JSON array to `ConfigValue::List`. Both
  `"filament_density": [1.24, 1.27]` and `"tool_config:1:retract_length": 2.0`
  are expressible with no special casing.
- **3MF.** `read_3mf_project_settings` / `parse_project_settings_json`
  (`crates/slicer-model-io/src/loader.rs`) read
  `Metadata/project_settings.config` and ingest **every** key generically. Orca
  writes every scalar as a JSON *string*, and lists as arrays of strings, so a
  real project arrives as `ConfigValue::List` of `ConfigValue::String`. The CLI
  merges the whole sidecar into `config_overrides`
  (`crates/pnp-cli/src/main.rs`), skipping only `thumbnail_path`.
- **Preset files.** No separate preset ingest path was found; presets reach the
  port only as a 3MF sidecar or as CLI JSON.

So **ingest is not the bottleneck** — a per-filament vector physically arrives.

### What happens to it next is the bottleneck

Two extractors sit behind the `ResolvedConfig` DSL:

| extractor | behaviour on a `List` |
| --- | --- |
| `extract_float_list` | keeps every element; accepts `Float` / `Int` / numeric `String` / `Bool`-as-0/1 elements, and expands an Orca point string (`"250x210"`) into two entries |
| `extract_float_or_first` | **keeps element 0 and silently discards the rest** |

`extract_float_or_first` is used by **12** declarations. Ten are the
`machine_max_*` `@printer` keys, where the collapse is intended and documented
(index 0 is normal mode, trailing entries are stealth-mode variants this port
does not model — ticket 117's territory). The other is
**`filament_diameter`**, and there the collapse is a silent per-filament data
loss:

```rust
cli @filament "filament_diameter" filament_diameter: f32 = 1.75 => extract_float_or_first;
cli @filament "filament_density"  filament_density: Vec<f64> = Vec::new() => extract_float_list;
```

Canonical has both as `ConfigOptionFloats`. This port models one as a vector and
one as a scalar-take-first, from the same 3MF, with no diagnostic on the lossy
path.

### The `@filament` / `@printer` markers do nothing at runtime

The scope marker is threaded through `__drc!` into `HostConfigKey::scope`
(`crates/slicer-ir/src/resolved_config.rs`), whose **only** consumer is the
`module config-schema` JSON builder in `crates/slicer-scheduler/src/manifest.rs`.
`SCOPE_FILAMENT` / `SCOPE_PRINTER` are GUI preset-routing labels — which preset
file the key round-trips into. They carry **no** indexing, arity or per-tool
semantics whatever.

Two keys carry `@filament`; ten carry `@printer`. Reading `@filament` as "this
key is per-filament" is a trap: `filament_diameter` is marked `@filament` and is
a scalar.

---

## Stage 2 — Resolution

### The general mechanism: `tool_config:<idx>:<key>`

`resolve_per_tool_configs` (`crates/slicer-scheduler/src/config_resolution.rs`)
groups every `tool_config:<idx>:<sub_key>` entry by integer index and, for each,
returns `apply_overlay(global, sub, bounds)` — a full `ResolvedConfig` per tool.
Non-numeric indices are skipped silently. `apply_overlay` routes each override
through `ResolvedConfig::apply_cli_key`, so **all 79 CLI-bound fields are
reachable**, and anything unrecognised lands in `extensions` (so module-manifest
keys are per-tool too).

The documented precedence is
`global < per_object < per_paint_semantic < per_tool`, mirroring canonical
applying filament overrides last (`PrintApply.cpp`).

This is a real, general axis. It is also **not** how Orca spells per-filament
config, so nothing populates it from a 3MF; it is a PnP-native spelling that
today only a hand-written CLI JSON can reach.

### Other tool-indexed state

`ResolvedConfig` declares **83** fields (79 `cli`/`cli_opt`, 4 `plain`). Exactly
**three** are `Vec`:

| field | shape | tool-indexed? |
| --- | --- | --- |
| `filament_density: Vec<f64>` | Orca `coFloats` | **yes** — `filament_density_for(tool_index)`, first-entry fallback |
| `printable_area: Vec<f64>` | interleaved bed polygon | no |
| `fill_authored_coloring: Vec<String>` | string list | no |

`filament_density_for` is the *only* per-tool accessor on `ResolvedConfig`.

### Scalars that canonical has as per-extruder vectors

- **`nozzle_diameter`** — canonical `coFloats`
  (`PrintConfig.cpp`, `init_fff_params`). In this port it is **not a
  `ResolvedConfig` field at all**: it lives in `extensions` as a scalar
  `ConfigValue::Float`, read by `ext_abs_mm`
  (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`),
  `support_territory_clearance_mm`
  (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) and
  `crates/slicer-runtime/src/run.rs`. Note that
  `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` declares it
  `type = "float"` and its description calls it "OrcaSlicer coFloat key
  nozzle_diameter" — **that description is factually wrong**; canonical is
  `coFloats`.
- **`filament_diameter`** — canonical `coFloats`, scalar here (see Stage 1).
- **`extruder_printable_area` / `extruder_printable_height`** — absent from the
  tree entirely. The only occurrences are in
  `crates/pnp-cli/src/visual_debug_gcode.rs`, where
  `extruder_printable_area` is named solely to *disambiguate* it from
  `printable_area` when parsing a CONFIG_BLOCK.

Widening a scalar to a vector costs: the `ResolvedConfig` field type, its
`to_config_map` emission, every read site, and — for `nozzle_diameter` — moving
it out of `extensions` first.

---

## Stage 3 — Module surface

### A list does reach a module, and there is no typed accessor for it

- **Wire.** `crates/slicer-schema/wit/deps/config.wit` declares
  `variant config-value { …, float-list(list<f64>), string-list(list<string>), … }`
  — so `config-view.get(key)` can return a list to a WASM guest.
- **Host storage.** `ConfigValueStorage::FloatList` / `StringList`
  (`crates/slicer-wasm-host/src/host.rs`); `ct::HostConfigView::get` maps them to
  the WIT `ConfigValue::FloatList` / `StringList`.
- **Emission.** `ResolvedConfig::to_config_map` emits `filament_density` as a
  `ConfigValue::List` of floats (~44 keys are emitted in total, plus the whole
  `extensions` map).
- **Gap:** `ConfigView` (`crates/slicer-ir/src/slice_ir.rs`) has typed accessors
  `get_bool` / `get_int` / `get_float` / `get_string` / `get_abs_value` — and
  **no list accessor**. A module must call `get(key)` and match the variant. The
  WIT `config-view` resource has the same asymmetry (`get-bool` / `get-float` /
  `get-int` / `get-string`, no `get-float-list`).

### A module cannot index a list by tool id, because it does not know the tool

`ConfigView::from_declared` pre-filters to the module's declared keys, so a
module sees `filament_density` only if its manifest declares it. There is no
per-tool `ConfigView` variant and no "resolve this key for tool N" accessor
anywhere on the module boundary. A module holding the list and a tool id can of
course index it itself — but nothing in the SDK does that for it, and the
first-entry fallback semantics of `filament_density_for` are host-side only.

The wipe-tower module does see each `ToolChange`'s `from` / `to`, so the tool id
is at that seam.

### `tool-count()` exists, and returns 1 in the shipped pipeline

`host-services` (`crates/slicer-schema/wit/deps/common.wit`) exports
`tool-count: func() -> u32` — "number of configured tools/filaments for this
print, minimum 1". `derive_tool_count` (`crates/slicer-wasm-host/src/host.rs`)
computes it, and the *same* function backs the host-side range check on authored
`tool_index` values (`crates/slicer-wasm-host/src/marshal/out.rs`), so guest and
host cannot disagree.

Its source of truth is **`filament_density`'s length** — explicitly, "the only
per-tool-count carrier in `ResolvedConfig`". And it reads the module's
*declared-filtered* config (`default_config_fields` is built from
`effective_config_view` in `crates/slicer-wasm-host/src/dispatch.rs`), so:

> a module that wants a truthful count MUST declare `filament_density` in its
> manifest config keys.

**No core module does.** The only manifest in the tree declaring
`filament_density` is the labelled example community module
`modules/community-modules/dragon-curve/dragon-curve.toml`, whose comments
document exactly this hazard. Every core module's `tool-count()` returns the
single-tool default of 1 today.

### Where the per-tool overlay actually lands

Two consumers, and only two:

**1. The emitter** (`crates/slicer-gcode/src/emit.rs`), via
`with_tool_configs`, wired in `crates/slicer-runtime/src/run.rs`. It reads
exactly **two** fields per tool:

- `retract_length_for_tool` → `retract_length` (per-tool override, global
  fallback);
- `filament_diameter`, collected into the print-time estimator's
  `tool_diameters` map.

Everything else in the emitter reads the global `self.resolved_config`.

**2. Region mapping** (`execute_region_mapping_inner`,
`crates/slicer-core/src/algos/region_mapping.rs`), wired through
`crates/slicer-runtime/src/prepass.rs` and
`crates/slicer-runtime/src/builtins/region_mapping_producer.rs`. This is the
interesting one: for a variant chain carrying `("material", ToolIndex(n))`, the
matching `tool_configs[n]` is overlaid **last** onto the region's effective
config — the config that becomes `RegionPlan.config` and reaches modules
per region. The comment names the intent precisely: *"this is the only place a
painted region's tool is known before perimeter generation, so it is where
per-tool geometry (e.g. `line_width`) composes."*

So the per-tool axis reaches module-visible geometry config — but **only for
painted regions**, and only through `overlay_resolved`.

### `overlay_resolved` is a 29-field allowlist

`overlay_resolved` (`crates/slicer-core/src/algos/region_mapping.rs`) copies a
field from the overlay only when it differs from `ResolvedConfig::default()`, and
it enumerates **29** fields by hand, then merges `extensions` wholesale.
`ResolvedConfig` declares **83**. A `tool_config:<n>:<key>` naming a declared
field outside that 29 resolves correctly in `resolve_per_tool_configs` and is
then **silently dropped** at composition. Module-manifest keys (which live in
`extensions`) survive, because that half is a blanket merge.

The same allowlist gates the per-paint-semantic and modifier overlays, so this
is not a per-tool-only narrowing.

### Suspected precedence defect — read from code, not reproduced

`overlay_resolved` compares the overlay's field against
`ResolvedConfig::default()`, but a per-tool `ResolvedConfig` is built as
*`global.clone()` + overrides* (`apply_overlay`). So every field the **global**
config sets away from its default is non-default in the tool config too, and the
per-tool overlay writes the **global** value back over the region's effective
config — clobbering a paint-semantic override that the documented precedence
says the per-tool overlay should only beat when the tool actually overrode that
key.

Shape of the failure: global sets `line_width = 0.6`; a paint semantic sets
`line_width = 0.5`; `tool_config:1:retract_length` is set and says nothing about
`line_width`. A painted region on tool 1 gets `0.6`, not `0.5`.

**This was read from the code and not reproduced with a test in this session.**
It needs one before it is treated as fact. It is stated here because it bears
directly on whether the existing mechanism can be adopted as-is.

---

## Stage 4 — Extruder side

Canonical distinguishes *filament* (per-material) from *extruder* (per-hardware)
vectors. This port has no extruder axis at all:

- No `extruder_printable_area` / `extruder_printable_height` (see Stage 2).
- No `extruder_clearance_*`, no `nozzle_height` — ticket 32 measured this; they
  are sequential-printing keys and now ride
  [ticket 124](./124-author-packet-sequential-printing-and-toolhead-clearance.md).
- `nozzle_diameter` is a single scalar in `extensions` where canonical has a
  per-extruder vector.
- `tool_config:<idx>:` indexes *tools*; nothing distinguishes a tool from the
  extruder carrying it.

### Correction to a comment in the tree

`modules/core-modules/machine-gcode-emit/src/lib.rs` records, as a ticket-27
divergence:

> `is_multi_extruder` is `nozzle_diameter.size() > 1` in canonical; no
> extruder-count key reaches a `PostPass` module here, so the stand-in is "this
> print performs a toolchange".

The **mechanism** half of that is wrong. `world gcode-postprocess-module`
(`crates/slicer-schema/wit/deps/postpass-gcode-postprocess/`) imports
`slicer:common/host-services`, and `tool-count` is in `host-services`. A PostPass
module **can** call it.

What is true is the *effect*: `tool-count()` would return 1 for
`machine-gcode-emit` today, because that module declares `nozzle_diameter`
(scalar) and not `filament_density`. Closing the gap is a one-line manifest
addition plus a call — no WIT change, no new seam. The toolchange stand-in is
still not canonical-equivalent (a two-filament print with no toolchange on this
layer reads as single-extruder), so the divergence is real; only its stated
*cause* needs correcting. This asset does not make that change — `DEV-168` clause
ownership sits with ticket 122.

---

## What works today, per stage

| stage | works | missing |
| --- | --- | --- |
| Ingest | list values arrive from CLI JSON and from a 3MF `project_settings.config`, string elements included | `extract_float_or_first` silently drops elements 1..n for `filament_diameter`; nothing maps an Orca `coFloats` key onto the `tool_config:` axis |
| Resolution | `tool_config:<idx>:<key>` yields a full `ResolvedConfig` per tool, covering all 79 CLI-bound fields plus `extensions` | only `filament_density` is a real per-tool vector; `nozzle_diameter` / `filament_diameter` are scalars; no extruder axis |
| Module surface | `float-list` on the wire; `filament_density` emitted as a list; `tool-count()` available in every world importing `host-services` | no list accessor on `ConfigView` (host or WIT); no per-tool config view or per-tool accessor; no core module declares `filament_density`, so `tool-count()` is 1 |
| Composition | per-tool overlay applied last onto painted-region config, and to `retract_length` / `filament_diameter` in the emitter | `overlay_resolved` covers 29 of 83 declared fields; overlay reaches geometry only via a `("material", ToolIndex(n))` paint chain; suspected precedence defect above |

## Questions the ruling must answer

Not decided here.

1. **Adopt `tool_config:<idx>:<key>` as the model, or model Orca's `coFloats`
   directly?** They are different shapes: one override map per tool, versus one
   list per key. Adopting the first means an ingest layer that explodes an Orca
   vector into per-tool overrides; adopting the second means widening the
   deferred Tier D fields to `Vec` and adding `_for(tool)` accessors. A hybrid —
   ingest explodes vectors onto the existing axis — reuses everything already
   built. (Take the deferred-key count from
   [04's tier table](./04-asset-tier-assignment.md) at the time of the ruling;
   do not freeze it from here.)
2. **Does `overlay_resolved` get closed over all declared fields?** At 29 of 83
   it is a silent-drop hazard for *every* overlay axis, not just per-tool. This
   is prerequisite work for any answer to (1).
3. **Does the tool axis need to reach unpainted regions?** Today a region's tool
   is known only from a paint variant chain.
4. **Does the module boundary get a per-tool surface** (a `get_float_list`
   accessor, a per-tool `ConfigView`, or a `resolve_for_tool(key, tool)` host
   function), or do modules keep indexing raw lists themselves?
5. **Is an extruder axis separate from the tool axis in scope at all?**

## Follow-on work this asset surfaces

- The `overlay_resolved` 29-of-83 narrowing (question 2) — worth a ticket
  regardless of the ruling.
- The suspected `overlay_resolved` precedence defect — needs a test before it is
  treated as fact.
- `extract_float_or_first` silently truncating `filament_diameter` from a real
  Orca 3MF — a lossy-ingest diagnostic at minimum.
- The wrong "coFloat" description on `machine-gcode-emit`'s `nozzle_diameter`
  manifest entry, and the wrong causal claim in its `is_multi_extruder` comment.

None of these are declared or changed by ticket 118.
