# 118 — Inventory the port's per-tool (filament / extruder) config mechanism

Type: research
Status: open
Assignee: —
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
