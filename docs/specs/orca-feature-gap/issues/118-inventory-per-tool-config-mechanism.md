# 118 — Inventory the port's per-tool (filament / extruder) config mechanism

Type: research
Status: resolved
Assignee: wayfinder session (2026-09-03)
Blocked by: —
Map: ../map.md

## Question

**Does this port already have a per-tool config model, and how far does it
reach?** 47 Tier D keys and at least two Tier B keys are deferred on this
question (map fog patch *"Where filament-level config even lives"*), and
ticket 28 hard-blocked packet P21 on it.

The question is no longer "does one exist at all" — one instance does, and this
ticket measures how general it is. Ground every answer in the tree.

Known starting point (ticket 28): `ResolvedConfig`
(`crates/slicer-ir/src/resolved_config.rs`) declares `filament_density:
Vec<f64>` with a `@filament` scope marker, and
`ResolvedConfig::filament_density_for(tool_index)` indexes it with a
first-entry fallback. Canonical's `coFloats` per-filament keys want exactly
that shape.

Trace the whole path and report where it stops:

1. **Ingest.** Which config sources can supply a list for a `@filament` /
   `@printer`-scoped key — CLI, 3MF, preset? How does a real Orca 3MF's
   per-filament vector arrive? (Ticket 100's `printable_area` lesson: check the
   *value spelling*, not just the key.)
2. **Resolution.** How do `@filament` and `@printer` scope markers behave in the
   `ResolvedConfig` macro, and what other tool-indexed fields already exist?
   Are there scalar fields (`nozzle_diameter: f32`) that canonical has as
   per-extruder vectors, and what would widening one cost?
3. **Module surface.** Does `ResolvedConfig::to_config_map`
   (`crates/slicer-wasm-host/src/marshal/in_.rs`, `marshal/native.rs`) carry a
   list to a module's `ConfigView`, and can a module read it? `float-list` is a
   manifest type (`printable_area` in `wipe-tower.toml` is one), so the type
   exists — the open part is whether a module can *index it by tool id*.
   The wipe-tower module sees each `ToolChange`'s `from`/`to`, so the tool id
   may already be at the seam.
4. **Extruder side.** Same three questions for per-**extruder** vectors
   (`extruder_printable_area`, `extruder_printable_height`,
   canonical's per-extruder `nozzle_diameter`), including whether any
   extruder-count notion reaches a `PostPass` module — the time-lapse fog patch
   records that it does not, and `is_multi_extruder` is approximated by "this
   print performs a toolchange".

Deliverable: a linked markdown asset stating, per stage, what works today and
what is missing — enough that the follow-on ruling (does the port adopt a
general per-tool config model, or explode these keys some other way?) can be
made without re-deriving any of it. **This ticket does not make that ruling**;
it makes it answerable. Do not declare any key anywhere as part of it.

Ledger facts re-derived from disk at edit time, never frozen. In-tree citations
by symbol name + crate-qualified path; OrcaSlicer citations by file + function.

## Answer

**Yes — and it is much more general than the `filament_density_for` instance
ticket 28 found. The mechanism is not the gap.** Full per-stage inventory:
[118-asset-per-tool-config-inventory.md](./118-asset-per-tool-config-inventory.md).
Read the asset before acting on any of the Tier D keys; the summary below is a
signpost, not a substitute.

### Headline

`tool_config:<tool_index>:<key>` (`resolve_per_tool_configs`,
`crates/slicer-scheduler/src/config_resolution.rs`) already produces a **whole
`ResolvedConfig` per tool index**, reaching all 79 CLI-bound fields plus any
module-manifest key via `extensions`. Documented precedence is
`global < per_object < per_paint_semantic < per_tool`, mirroring canonical
applying filament overrides last (`PrintApply.cpp`). No schema, WIT or IR change
is needed to set any key per tool today.

Four things are missing, none of them the mechanism:

1. **Nothing ingests Orca's per-tool vectors into it.** Orca spells per-filament
   config as a `coFloats` *list* on one key, not as `tool_config:1:foo`. Exactly
   one key here is wired that way (`filament_density`); `extract_float_or_first`
   silently keeps element 0 of any list handed to `filament_diameter`, so a real
   two-filament Orca 3MF loads filament 1 and discards filament 2 with no error.
2. **`overlay_resolved` (`crates/slicer-core/src/algos/region_mapping.rs`) is a
   hand-maintained 29-field allowlist** against 83 declared fields. A per-tool
   override outside those 29 resolves correctly and is then silently dropped at
   composition. This gates the per-object and per-paint overlays too.
3. **The tool axis reaches geometry only through a `("material", ToolIndex(n))`
   paint variant chain.** There is no "this object prints with tool 2".
4. **No core module declares `filament_density`**, which `derive_tool_count`
   (`crates/slicer-wasm-host/src/host.rs`) documents as the only per-tool-count
   carrier — so `tool-count()` returns 1 for every shipped module.

### Corrections to beliefs this map was carrying

- **`@filament` / `@printer` are GUI preset-routing labels, nothing more.** The
  marker reaches only `HostConfigKey::scope`, whose sole consumer is the
  `module config-schema` JSON builder (`crates/slicer-scheduler/src/manifest.rs`).
  It carries no indexing or arity semantics. `filament_diameter` is `@filament`
  **and a scalar** — reading the marker as "per-filament" is a trap.
- **An extruder count *does* reach a `PostPass` module.**
  `world gcode-postprocess-module` imports `slicer:common/host-services`, and
  `tool-count` lives there. `machine-gcode-emit`'s ticket-27 comment ("no
  extruder-count key reaches a `PostPass` module here") is wrong about the
  mechanism; it is right about the effect, because that module declares
  `nozzle_diameter` and not `filament_density`. The divergence stands, its
  stated cause does not. `DEV-168` clause ownership stays with ticket 122.
- **`nozzle_diameter` is not a `ResolvedConfig` field at all** — it is a scalar
  in `extensions`, where canonical has `coFloats`. Widening it means moving it
  out of `extensions` first. Its `machine-gcode-emit` manifest description
  calling it "OrcaSlicer coFloat key" is factually wrong.

### Follow-on tickets filed

- [125 — Rule on the port's per-tool config model](./125-rule-per-tool-config-model.md)
  — the ruling this ticket was built to make answerable. Graduated from the map's
  *"Whether the per-tool config model generalises"* fog patch.
- [126 — Close `overlay_resolved`'s 29-of-83 field narrowing](./126-overlay-resolved-field-narrowing.md)
  — prerequisite for any answer to 125, and a silent-drop hazard on every overlay
  axis today. Also carries a **suspected precedence defect** the asset describes:
  because a per-tool config is `global.clone() + overrides` while the overlay test
  is "differs from `ResolvedConfig::default()`", a per-tool overlay writes the
  *global* value back over a paint-semantic override the tool never touched.
  **Read from code, not reproduced with a test in this session** — 126 owns
  proving it.

The lossy `filament_diameter` ingest and the two wrong comments are recorded in
the asset's *Follow-on work* section and are not separately ticketed; they are
small enough to ride whichever ticket next touches that code.

No key was declared and no code was changed by this ticket.
