# 54 — Author packet P47 — Printer / Machine / Motion limits — emitter

Type: task
Status: resolved
Assignee: Adelino Penedo
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P47 — Printer / Machine / Motion limits — emitter** — 9 keys, Tier B new logic, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P47 — Printer / Machine / Motion limits — emitter):

`machine_max_acceleration_extruding`, `machine_max_acceleration_retracting`, `machine_max_acceleration_travel`, `machine_max_acceleration_x/y/z/e`, `machine_max_jerk_x/y/z/e`, `machine_max_junction_deviation`, `machine_max_speed_x/y/z/e`, `machine_min_extruding_rate`, `machine_min_travel_rate`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify the owner's seam and the missing decision point per key (04) — re-derive from code. Work: new behaviour inside the existing owner.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

**Authored as packet 281** (`docs/spec_packets/281-machine-motion-limits-emitter/`,
`draft`), preflight **PASS** after one HIGH round (AC-N1 pointed at a
nonexistent `--test scheduler_integration` binary — corrected to the real
`--test integration` aggregator that already hosts
`config_bounds_enforcement_tdd.rs`; the `emit_machine_limits_to_gcode` gate
wording made an explicit reconciled FORWARD-DEP on draft packet 267).
Claim-time re-derivation kept Tier B with all **9 families (18 scalars) in,
none shed, none returned**, no code change.

- Tree grounding: 10 of the 18 scalars already live as `ResolvedConfig`
  `Option<f32>` fields (`machine_max_speed_x/y/z/e`,
  `machine_max_jerk_x/y/z/e`, `machine_max_acceleration_extruding|travel`,
  all `extract_float_or_first`); the other 8 are zero-occurrence
  (per-axis accelerations, retracting acceleration, junction deviation,
  both minimum rates). `machine-gcode-emit` has no machine-limit rows;
  `GcodeFlavor` already ships the M204/M205/JD formatters with no caller.
- Canonical grounding (oracle `D:\slicerProject\pinch_n_print_cli\OrcaSlicerDocumented`):
  all nine pass rule 3 and stay in scope —
  `GCode::print_machine_envelope` (M201/M203/M204/M205 order, Marlin-family
  +RRF flavor gate, RRF ×60 on M203/M566 only, legacy-Marlin travel
  fallback), `GCodeWriter::set_junction_deviation` (M205 J only when JD >
  0), `GCodeProcessor::apply_config` minimum-feedrate clamp (no G-code).
  Defaults: accel x/y 1000, z 500, e 5000; retracting 1500; JD 0.01
  (max 0.3, GUI hint only — not enforced); min rates 0. All are stride-2
  normal/stealth `coFloats` in the `printer_options_with_variant_2` set.
- Packet shape: 8 new scalar-global fields (first-wins ingest, DEV-173 —
  stealth variant stays with ticket 117, `silent_mode` declared nowhere),
  `M201` + `M204 R` + `M205 J` extending 267's builder (activation blocked
  on 267 landing), min-rate estimator clamps, min-0 bounds with no maxima
  (ticket-113 ruling). P47 still covers 9 keys; no queue-count change;
  no 04/05 row change (owner and tier stand as tiered).
