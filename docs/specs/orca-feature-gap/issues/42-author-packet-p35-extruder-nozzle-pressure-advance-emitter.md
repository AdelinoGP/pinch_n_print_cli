# 42 — Author packet P35 — Extruder / Nozzle / Pressure advance — emitter

Type: task
Status: closed
Assignee: wayfinder session (ses_f8c3330b1ffeZvpvUQznRMUKUy) — claimed 2026-09-05
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P35 — Extruder / Nozzle / Pressure advance — emitter** — 6 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P35 — Extruder / Nozzle / Pressure advance — emitter):

`adaptive_pressure_advance`, `adaptive_pressure_advance_bridges`, `adaptive_pressure_advance_model`, `adaptive_pressure_advance_overhangs`, `enable_pressure_advance`, `pressure_advance`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Re-sized at claim time: 2 keys live by direct implementation, 4 returned
to the queue as unimplemented — no packet** (the ticket-40/22 shape, under
the "Packets are for complex implementation only" rule).

All six keys are live in canonical (rule 3 keeps every one in scope) and
had zero occurrences in `crates/` / `modules/` / `xtask/` — only the
unwired dialect helper `GcodeFlavor::set_pressure_advance`
(`crates/slicer-gcode/src/flavor.rs`, all five flavors, pinned by
`gcode_flavor_dialect_tdd`) existed. Canonical grounding (oracle
`D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`, re-derive the
path at point of use): `PrintConfig.cpp` types/defaults are
`enable_pressure_advance` `coBools` false, `pressure_advance` `coFloats`
0.02 max 2, `adaptive_pressure_advance` `coBools` false,
`adaptive_pressure_advance_model` `coStrings` `"0,0,0\n0,0,0"`,
`adaptive_pressure_advance_overhangs` `coBools` false,
`adaptive_pressure_advance_bridges` `coFloats` 0.0 max 2 — all per-tool
vectors (`get_at(tool)`); reads are `GCode.cpp` toolchange/start emission
+ `AdaptivePAProcessor.cpp` per-feature prediction + `Print.cpp` model
validation + `GCodeWriter.cpp::set_pressure_advance` dialect.

**Live — `enable_pressure_advance` + `pressure_advance`.** The tier
table's `crates/slicer-gcode (flavor.rs)` owner was half-right: the dialect
shapes live there, but no emission site does. Built it in the host emitter
(P18-packet-267 design, without the packet): `ResolvedConfig` gains both
fields (canonical defaults; intentionally omitted from `to_config_map` —
host-only emission control like `disable_m73`, P18/P96-AC-8 precedent, so
default CONFIG_BLOCK bytes are unchanged); both declared in
`machine-gcode-emit.toml` (canonical defaults/bounds) to close ticket 04's
ResolvedConfig-only contract (the manifest default is dead for this class —
map Notes); `DefaultGCodeEmitter` gains the `flavor` field + `with_flavor`
fed from `run_slice`'s existing `gcode_flavor` resolution (packet 267
designs the same field — it rebases onto this one, queue-order merge
churn); `emit_gcode` pushes one flavor-specific `Raw` line (no trailing
newline — packet-267 Raw rule) after `ExtrusionMode` for the initial tool
and one after every `ToolChange` for the new tool, gated on enable and on
`pa >= 0` (canonical's `pa < 0` early return). Per-tool values resolve
through the existing `tool_config:<idx>:` axis
(`pressure_advance_for_tool`, `retract_length_for_tool` precedent); Orca
vector-vector ingest rides ticket 125, not this ticket. Scalar-vs-vector is
therefore not a divergence here. Pre-existing divergence noted, not
created: the port has no Bambu flavor, so Bambu prints get plain `M900`
rather than canonical's `M900 K.. L1000 M10` branch. 9 new tests (7 emitter
+ 2 manifest guard): default/disabled/negative emit nothing, Marlin head
placement after `ExtrusionMode`, all five flavor forms, per-tool
toolchange values with T<n>-then-PA order, first-layer-T1 preservation,
manifest type/default/min/max. Default path is byte-identical (no PA, no
CONFIG_BLOCK line).

**Returned — the four `adaptive_*` keys.** They need the
AdaptivePAProcessor-style per-feature prediction this tree has nothing of:
per-tool interpolators over the model flow/accel triplets, a
`process_layer` G-code post-pass tracking feedrate/acceleration/flow, the
bridge static override and the overhang in-feature arm, plus the
`Print.cpp` model validation. That is Tier B+ packet work, not a
same-session direct. Missing feature named in the tier table; packet-list
P35 row annotates the split; no packet number taken, no new ticket — the
future claim packets them. No queue-count change (nothing covered beyond
the two, nothing ruled dead).

Gates: `cargo check`, workspace clippy, check-literals clean;
`slicer-gcode` 17 binaries, `slicer-ir` lib, `machine-gcode-emit` green;
`machine_start_end_gcode_emission_tdd` 17/17 + `gcode_flavor_config_block`
2/2 (CONFIG_BLOCK stable); doc-15 regen (+2 rows, deviations stay 26);
all 46 guests rebuilt fresh (`slicer-ir` is in every guest closure).
Pre-existing red noted, not caused here: `check-deviations --check`
(doc 07 map) fails on the clean tree too.
