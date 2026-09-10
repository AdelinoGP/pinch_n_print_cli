# 138 — Author packet P38 + P78 (re-filed) — bed-temperature formula / bed-type selection + multi-bed gate — emitter

Type: task
Status: open
Assignee: —
Blocked by: 06, 125
Map: ../map.md

## Question

Re-filed from [ticket 45](./45-author-packet-p38-filament-bed-temperature-emitter.md),
which re-sized P38 at claim time: **both keys are selectors over data this tree
does not have, and are not authorable under Authoring rule 1 until the per-tool
config model ruling ([125](./125-rule-per-tool-config-model.md), itself gated
on [126](./126-overlay-resolved-field-narrowing.md)) lands.**
**Read ticket 45's answer before starting** — it holds the per-key canonical
grounding, and is not restated here.

**Folded in from [ticket 85](./85-author-packet-p78-filament-bed-temperature-print-orchestration.md)
(P78, 2026-09-10):** `support_multi_bed_types` (`PrintConfig.cpp` `coBool`
default `false`; the `is_BBL_printer() || support_multi_bed_types` gate on the
`Print::validate` filament-vs-plate compatibility arm) is the *gate* over this
ticket's *subject* (`curr_bed_type` → plate vector → zero-check) and must author
with it — a standalone P78 packet would be declaration-only (rule 1).
**Read ticket 85's answer before starting** too — it holds that key's canonical
grounding. Re-derive membership from disk at authoring time; do not freeze it
from here.

Keys (3, all of P38 + all of P78):

`bed_temperature_formula`, `curr_bed_type`, `support_multi_bed_types`

Per-key canonical decision points (oracle: `PrintConfig.cpp` / `GCode.cpp` /
`GCodeProcessor.cpp` / `Print.cpp` / `ModelArrange.cpp`, all selection over
absent vectors):

- `bed_temperature_formula` (`coEnum BedTempFormula`, default
  `btfHighestTemp`; variants `btfFirstFilament` / `btfHighestTemp`) — selects
  highest-filament versus first-extruder temperature in
  `GCode::_print_first_layer_bed_temperature` (first layer),
  `GCode::process_layer` (first-to-second-layer transition), and
  `GCode::_do_export` (plate-specific bed vectors). The vectors it selects
  over (`bed_temperature` / `bed_temperature_initial_layer`, `coInts`
  per-filament) do not exist in this tree.
- `curr_bed_type` (`coEnum BedType`, default `btPC`; variants `btDefault` /
  `btPC` / `btEP` / `btPEI` / `btPTE` / `btPCT` / `btSuperTack` / `btCount`) —
  selects among the six plate-temperature vector pairs
  (`cool_plate_temp`, `eng_plate_temp`, `hot_plate_temp`,
  `textured_plate_temp`, `textured_cool_plate_temp`, `supertack_plate_temp`,
  each plus its `_initial_layer` twin — all `coInts` per-filament, defaults
  `{35}` / `{40}` / `{45}`) in `GCode::get_highest_bed_temperature`,
  `GCode::_print_first_layer_bed_temperature`, `GCode::process_layer`, and
  `GCode::_do_export`, with further reads in
  `GCodeProcessor::apply_config` (bed-type publication),
  `Print::validate` (filament-vs-plate compatibility),
  `get_instance_arrange_poly` (`ModelArrange.cpp`), and the 3MF exporter.
  Every plate vector is Tier D deferred (per-filament config model) with zero
  occurrences in `crates/` / `modules/` / `xtask/`.

Tree state at re-file: no plate model, no per-filament bed-temperature
vectors. The port's only bed-temperature value is the PnP-specific scalar
`bed_temperature_initial_layer_single` (manifest schema
`modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml`,
`MachineGcodeEmit::run_gcode_postprocess` template substitution,
`M190 S[bed_temperature_initial_layer_single]` default) — a single value with
no selection. `GcodeFlavor::set_bed_temperature`
(`crates/slicer-gcode/src/flavor.rs`) formats `M140`/`M190` but no selection
calls it; `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`)
emits no bed temperature.

Authoring obligations:

- **Do not start until 125 lands.** If that ruling defers the per-filament bed
  vectors again, the honest outcome is to defer this packet again — never
  declare the keys (Authoring rule 1). The packet, when authorable, must build
  the missing selection domain (per-filament vectors onto the tool axis, the
  plate-type axis, and the M140/M190 first-layer + transition emission) or shed
  the unimplemented key.
- Per-key re-derive the owner from the *tree's* seams at claim time (ticket
  27's hazard): the tier table's `crates/slicer-gcode` owner was reviewed
  against canonical, not against this port — live bed-temperature emission
  today is template-driven in `machine-gcode-emit`, not in the host emitter.
- Use `/spec-packet-generator`; gate is `/spec-review <packet> --preflight`.
- Apply ticket 02's parity-evidence standard; `OrcaSlicerDocumented/` is
  readable, not runnable.
- Packet number and status derived from disk at authoring time (ticket 06).

Resolved when the packet is authored, preflighted, and its directory linked
here — or when a later ruling rules the plate-selection family out of scope.

## Answer
