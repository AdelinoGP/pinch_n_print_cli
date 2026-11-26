# 45 — Author packet P38 — Filament / Bed temperature — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f8b45f5ffffe1wUHUP3GOtnJLk) — claimed 2026-09-06, resolved 2026-09-06
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P38 — Filament / Bed temperature — emitter** — 2 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P38 — Filament / Bed temperature — emitter):

`bed_temperature_formula`, `curr_bed_type`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: not authorable now, both keys re-filed, no packet,
no code change** (the ticket-28/39 shape). Both keys are zero-occurrence in
`crates/` / `modules/` / `xtask/`, so Tier B holds — but the two do not belong
in a packet today because each is a *selector over absent data*.

Canonical grounding (oracle `D:/slicerProject/pinch_n_print_cli/OrcaSlicerDocumented`,
`PrintConfig.cpp` / `GCode.cpp` / `GCodeProcessor.cpp`): `bed_temperature_formula`
is `coEnum BedTempFormula` default `btfHighestTemp` (`btfFirstFilament` /
`btfHighestTemp`), selecting highest-filament vs first-extruder temperature in
`GCode::_print_first_layer_bed_temperature`, `GCode::process_layer`, and
`GCode::_do_export`; `curr_bed_type` is `coEnum BedType` default `btPC`
(`btDefault` / `btPC` / `btEP` / `btPEI` / `btPTE` / `btPCT` / `btSuperTack` /
`btCount`), selecting among six plate-temperature `coInts` vector pairs
(`cool_plate_temp`, `eng_plate_temp`, `hot_plate_temp`,
`textured_plate_temp`, `textured_cool_plate_temp`, `supertack_plate_temp`,
each plus `_initial_layer`, defaults `{35}` / `{40}` / `{45}`) in
`GCode::get_highest_bed_temperature`, `GCode::_print_first_layer_bed_temperature`,
`GCode::process_layer`, and `GCode::_do_export` (plus
`GCodeProcessor::apply_config`, `Print::validate`, `ModelArrange.cpp`
`get_instance_arrange_poly`, 3MF export). Both pass rule 3 (live in the
slicing pipeline) and stay in the queue. All twelve plate vectors are Tier D
deferred with zero tree occurrences; the formula's `bed_temperature` vectors
are likewise absent.

Tree grounding: no plate model, no per-filament bed vectors. The only
bed-temperature value is the PnP-specific scalar
`bed_temperature_initial_layer_single` (schema
`modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`,
`MachineGcodeEmit::run_gcode_postprocess` template substitution) — no
selection. `GcodeFlavor::set_bed_temperature`
(`crates/slicer-gcode/src/flavor.rs`) formats `M140`/`M190` but nothing
selects through it; `DefaultGCodeEmitter::emit_gcode`
(`crates/slicer-gcode/src/emit.rs`) emits no bed temperature. A packet today
would be 100% declaration-only — prohibited by rule 1. The tier table's
`crates/slicer-gcode` owner is additionally wrong for this tree (ticket-27
hazard): live bed-temperature emission is template-driven in
`machine-gcode-emit`.

**Re-filed as [138](138-author-packet-p38-bed-temperature-selection-refiled.md),
blocked on 125** (itself gated on 126) — the 28→119 / 39→136 pattern. Tier
table + packet-list P38 rows annotate the re-file; no queue-count change
(selectors stay in scope, nothing covered). No packet number taken, no
deviation rows, no `ORCA_CONFIG_PADDING` edit. Docs-only change: no build,
clippy, or test gates affected.
