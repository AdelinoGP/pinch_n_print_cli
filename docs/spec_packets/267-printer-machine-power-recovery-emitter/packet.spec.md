---
status: draft
packet: 267-printer-machine-power-recovery-emitter
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/25-author-packet-p18-printer-machine-power-recovery-emitter.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
---

# Packet Contract: 267-printer-machine-power-recovery-emitter

## Goal

Make the three P18 keys drive real behaviour in the host emitter: declare `disable_m73` in the owner manifest (its M73-suppression gate is already live), build the canonical machine-limit envelope emission behind `emit_machine_limits_to_gcode`, and build the canonical power-loss-recovery emission behind `enable_power_loss_recovery`. `silent_mode` is returned to the queue, unimplemented, with the missing per-variant machine-limit model named.

## Scope Boundaries

This packet changes the host emitter (`crates/slicer-gcode`), the `machine-gcode-emit` manifest and its PrintStart insertion rule, the `run_slice` flavor wiring (`crates/slicer-runtime/src/run.rs`), two new `ResolvedConfig` fields plus their host-key documentation and lock test, the scheduler bounds arm, and the generated config reference. It does **not** touch `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) — under the map's Authoring rule 2 the padding table is not evidence and is never a deliverable. It does not add any P47 machine-limit field (`machine_max_acceleration_x/y/z/e`, `machine_max_acceleration_retracting`, `machine_max_junction_deviation`, `machine_min_extruding_rate`, `machine_min_travel_rate`) — those belong to packet P47 — and it does not declare `silent_mode` anywhere. No IR, WIT, or schema-version change is required.

## Prerequisites and Blockers

- Depends on [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md), [05 - Decide packet granularity and grouping](../specs/orca-feature-gap/issues/05-packet-granularity.md), [04 - Define the cost rubric that makes "cheapest-first" decidable](../specs/orca-feature-gap/issues/04-cost-tiering-rubric.md), [101 - Rename path-optimization keys to Orca names](../specs/orca-feature-gap/issues/101-rename-path-optimization-keys.md), and [107 - Collapse infill duplicate spellings to Orca names](../specs/orca-feature-gap/issues/107-collapse-infill-duplicate-spellings.md); all are resolved map decisions. The closed `disable_m73` predecessor `TASK-279` is a historical backlog row, not an ownership row — this queue packet carries `task_ids: []`.
- Unblocks [25 - Author packet P18 - Printer / Machine / Power / recovery - emitter](../specs/orca-feature-gap/issues/25-author-packet-p18-printer-machine-power-recovery-emitter.md) for packet authoring completion.
- Activation blockers: none for the draft packet; activation remains a separate explicit `/swarm` decision.

## Acceptance Criteria

- **AC-1. Given** `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` after this packet, **when** its `[config.schema]` is parsed, **then** it declares `disable_m73` (bool, default `false`), `emit_machine_limits_to_gcode` (bool, default `true`), and `enable_power_loss_recovery` (enum, values `["printer_configuration", "enable", "disable"]`, default `"printer_configuration"`), each with a `display` and `group = "Machine G-code"`; and it declares **none** of the P47 machine-limit keys (`machine_max_acceleration_x`, `machine_max_acceleration_y`, `machine_max_acceleration_z`, `machine_max_acceleration_e`, `machine_max_acceleration_retracting`, `machine_max_junction_deviation`, `machine_min_extruding_rate`, `machine_min_travel_rate`) nor `silent_mode`. | `cargo test -p machine-gcode-emit --test machine_gcode_emit_config_schema_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-2. Given** the real `pnp_cli slice` pipeline on the M73 fixture model, **when** the run config sets `disable_m73 = true`, **then** the output G-code contains no `M73` line while the default run emits `M73 P0 R` and `M73 P100 R0` — the value reaches the live gate `if !self.resolved_config.disable_m73` in `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`). | `cargo test -p pnp-cli --test m73_progress_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-3. Given** a two-layer fixture with `machine_max_speed_x/y/z/e` and `machine_max_jerk_x/y/z/e` set and flavor `marlin2`, **when** `DefaultGCodeEmitter::emit_gcode` runs with `emit_machine_limits_to_gcode = true` (canonical default), **then** the GCodeIR command stream opens with the envelope Raw commands in canonical order — `M203 X Y Z E`, `M204 P T`, `M205 X Y Z E` — ahead of the `M73 P0` pair and the `ExtrusionMode` command; **when** `emit_machine_limits_to_gcode = false`, the identical config emits none of those lines. | `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-4. Given** the same configured fields, **when** the flavor is `klipper` or `repetier`, **then** no envelope line is emitted (canonical gates the envelope to Marlin legacy, Marlin2, and RepRapFirmware); **when** the flavor is `marlin` (legacy), the M204 line is `M204 P{extruding} T{extruding}`; **when** `marlin2` or `reprapfirmware`, it is `M204 P{extruding} T{travel}`. | `cargo test -p slicer-gcode --test gcode_emit_tdd machine_envelope_flavor 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-5. Given** a two-layer fixture with flavor `marlin2`, **when** `enable_power_loss_recovery = "enable"`, **then** the stream contains `M413 S1` at the start of the second emitted layer and `M413 S0` after the last layer's commands; **when** `"disable"`, it contains only `M413 S0` at the second emitted layer; **when** `"printer_configuration"` (canonical default), it contains no `M413` line. | `cargo test -p slicer-gcode --test gcode_emit_tdd power_loss_recovery 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-6. Given** flavor `marlin` (legacy) or `klipper` with `enable_power_loss_recovery = "enable"`, **then** no `M413` line is emitted — canonical gates M413 to Marlin2 only; the Bambu `M1003` form is a recorded divergence because PnP's `GcodeFlavor` has no Bambu variant. | `cargo test -p slicer-gcode --test gcode_emit_tdd power_loss_recovery_flavor 2>&1 | tee target/test-output.log | grep -E '^test result'`

## Negative Test Cases

- **AC-N1. Given** the real machine-gcode-emit manifest loaded into `ConfigBoundsIndex`, **when** global configuration sets `enable_power_loss_recovery = "bogus"` or `emit_machine_limits_to_gcode = "yes"`, **then** resolution fails with the existing `slicer_ir::resolved_config::ConfigResolutionError::TypeMismatch` variant (enum-value rejection for the recovery string, type mismatch for the bool) rather than silently accepting. | `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'`
- **AC-N2. Given** `silent_mode` is returned to the queue, **then** no manifest table, `ResolvedConfig` field, or emitter read is added for it: `silent_mode` has zero occurrences in `crates/` and `modules/` after the packet. | `rg -q 'silent_mode' crates modules && echo FAIL || echo PASS`
- **AC-N3. Given** Authoring rule 2, **then** this packet's diff contains no change to `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs`. | `git diff --stat -- crates/slicer-gcode/src/serialize.rs | grep -q . && echo FAIL || echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`
- `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 | tee target/test-output.log | grep -E '^test result'` and `cargo xtask build-guests --check; echo "exit=$?"`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` - delegated summary of `[config.schema]` enum, bool, and float forms and host-enforced bounds.
- `docs/15_config_keys_reference.md` - generated output; regenerated and checked, never hand-edited.
- `docs/ORCASLICER_ATTRIBUTION.md` - standard header required if the implementation adds a new Rust file containing translated canonical logic.

## Doc Impact Statement (Required)

- `docs/config/host-keys.toml` `[resolved_config]` section - two new host-key rows (`emit_machine_limits_to_gcode`, `enable_power_loss_recovery`); verify with `rg -q 'emit_machine_limits_to_gcode' docs/config/host-keys.toml` and `rg -q 'enable_power_loss_recovery' docs/config/host-keys.toml`.
- `docs/15_config_keys_reference.md` generated module-key table - regenerated by `cargo xtask gen-config-docs`; verify with `rg -q 'emit_machine_limits_to_gcode' docs/15_config_keys_reference.md` and `rg -q 'enable_power_loss_recovery' docs/15_config_keys_reference.md`. No hand edit is allowed.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::print_machine_envelope` (flavor gate, command order, per-key gating, RRF mm/min factors) and the `GCode::_do_export` call sites for the envelope and for `GCodeWriter::enable_power_loss_recovery` (second-layer and end-of-print placement).
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - `GCodeWriter::enable_power_loss_recovery` (M413 for Marlin2, M1003 for Bambu, empty for all other flavors) and the M201/M203/M204/M205 command forms.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` and `PrintConfig.hpp` - canonical declarations of `emit_machine_limits_to_gcode` (coBool, default true) and `enable_power_loss_recovery` (coEnum `printer_configuration`/`enable`/`disable`, default `printer_configuration`), and the stride-2 normal/stealth variant reads that make `silent_mode` unexpressible in PnP's scalar fields.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
