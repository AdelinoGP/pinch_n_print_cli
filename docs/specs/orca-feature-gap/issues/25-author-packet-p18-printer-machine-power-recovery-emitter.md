# 25 — Author packet P18 — Printer / Machine / Power / recovery — emitter

Type: task
Status: resolved
Assignee: wayfinder session (ses_f9a3ffe33ffeWdfi3oejwSsEkZ) — claimed 2026-09-03
Blocked by: 06, 101, 107
Map: ../map.md

## Question

Author the spec packet for **P18 — Printer / Machine / Power / recovery — emitter** — 4 keys, Tier A plumbing, owner host emitter (crates/slicer-gcode). Key membership from [05-asset-packet-list.md](./05-asset-packet-list.md) (packet P18 — Printer / Machine / Power / recovery — emitter):

`disable_m73`, `emit_machine_limits_to_gcode`, `enable_power_loss_recovery`, `silent_mode`

Authoring obligations:
- Use `/spec-packet-generator`; the authoring gate is `/spec-review <packet> --preflight` (must pass).
- Apply 02's parity-evidence standard — canonical function-read + invariant tests; `OrcaSlicerDocumented/` is readable, not runnable; unverifiable behaviour surfaces to the human first, never blocks.
- Packet number + status: derive from disk at authoring time per ticket 06's rule — ledger facts (next free number, `status: draft` vs `active`) are never frozen.
- Verify each key's decision point exists (04's mechanical proxy, refined at authoring time) — re-derive from code, don't trust the tier table. Work: declare in the owner's manifest + wire.

Resolved when the packet is authored, preflighted, and its directory linked here.

## Answer

Resolved 2026-09-03 — packet `docs/spec_packets/267-printer-machine-power-recovery-emitter/` authored as `draft`; preflight **PASS**. Re-derivation split the four scoped keys three ways:

- **`disable_m73` — Tier A plumbing.** The decision point is already live: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) gates `crate::m73::inject_m73` on `if !self.resolved_config.disable_m73`, and `crates/pnp-cli/tests/m73_progress_tdd.rs` proves the suppression end-to-end. The gap is ticket 04's "ResolvedConfig-only keys" contract violation — the key is declared in no module manifest. The packet declares it in `machine-gcode-emit.toml` and pins reachability (AC-2).
- **`emit_machine_limits_to_gcode` — Tier B.** True zero-occurrence gap. The packet builds the canonical machine-limit envelope (`GCode::print_machine_envelope`) as the PnP scalar subset: M203 (`machine_max_speed_x/y/z/e`, RRF × 60), M204 P/T (`machine_max_acceleration_extruding` / `machine_max_acceleration_travel`, Marlin legacy substituting extruding for T), M205 (`machine_max_jerk_x/y/z/e`, RRF M566 × 60), flavor-gated to Marlin/Marlin2/RepRapFirmware, prepended to the GCodeIR stream ahead of `machine_start_gcode`. Missing groups (M201, M204 R, M205 J, M593) are recorded as divergences — the P47 fields do not exist and are not invented.
- **`enable_power_loss_recovery` — Tier B.** True zero-occurrence gap. The packet builds the canonical recovery emission (`GCodeWriter::enable_power_loss_recovery`): `enable` → `M413 S1` at the second emitted layer + `M413 S0` at the end; `disable` → `M413 S0` at the second emitted layer; `printer_configuration` → nothing; Marlin2 only (Bambu's `M1003` is a recorded divergence — PnP has no Bambu flavor).
- **`silent_mode` — returned to the queue as unimplemented.** Canonical reads every `machine_max_*` key as a stride-2 normal/stealth pair and `silent_mode` selects the variant; PnP's scalar `Option<f32>` fields have no variant dimension, so the key cannot drive anything. Missing feature named: a per-variant machine-limit model (P47 family). Follow-up filed as [117](./117-silent-mode-per-variant-machine-limit-model.md); the tier row and packet-list entry are updated.

The packet is **mixed A/B** (was all-A): `disable_m73` A, `emit_machine_limits_to_gcode` B, `enable_power_loss_recovery` B. P18 covers **3 keys, not 4**. The postpass PrintStart rule in `machine-gcode-emit` changes from "prepend ahead of every command" to "insert after the leading non-M73 Raw run" so the envelope precedes the start template while `machine_start_gcode_precedes_m73_and_extrusion_mode` survives. No user rulings required; no deviation rows filed; `ORCA_CONFIG_PADDING` untouched in both directions (AC-N3).
