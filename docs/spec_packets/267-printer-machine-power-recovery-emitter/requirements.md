# Requirements: 267-printer-machine-power-recovery-emitter

## Packet Metadata

- Grouped task IDs: none - queue packet; implementation is recorded against [25 - Author packet P18 - Printer / Machine / Power / recovery - emitter](../specs/orca-feature-gap/issues/25-author-packet-p18-printer-machine-power-recovery-emitter.md). The closed `disable_m73` predecessor `TASK-279` is a historical backlog row, not an ownership row.
- Backlog source: `docs/specs/orca-feature-gap/issues/25-author-packet-p18-printer-machine-power-recovery-emitter.md` (P18 in the wayfinder map "Close the OrcaSlicer FFF feature gap").
- Packet number: 267, allocated from disk at authoring time per [06 - Settle packet numbering and how this queue interleaves with live work](../specs/orca-feature-gap/issues/06-queue-numbering-and-sequencing.md).
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

P18 covers the four canonical Printer / Machine / Power / recovery keys that the host emitter (`crates/slicer-gcode`) owns. Re-derived from disk at authoring time, the four keys split three ways.

`disable_m73` is **already live**: `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-gcode/src/emit.rs`) gates `crate::m73::inject_m73` on `if !self.resolved_config.disable_m73`, and `crates/pnp-cli/tests/m73_progress_tdd.rs` proves the suppression end-to-end. The gap is the map's ticket-04 "ResolvedConfig-only keys" contract violation: the key is a typed `ResolvedConfig` field consumed at a decision point but declared in no module manifest. It is Tier A plumbing: declare it in the owner manifest (`machine-gcode-emit`) and pin the reachability.

`emit_machine_limits_to_gcode` and `enable_power_loss_recovery` are **true zero-occurrence gaps** (verified this session: zero `.rs` / `.toml` / `.wit` occurrences in the tree). Canonical emits a machine-limit envelope (`GCode::print_machine_envelope`: M201/M203/M204/M205, gated to Marlin legacy, Marlin2, and RepRapFirmware) and power-loss-recovery commands (`GCodeWriter::enable_power_loss_recovery`: M413 S1/S0 for Marlin2, M1003 S1/S0 for Bambu) at well-defined stream positions. Neither decision point exists in PnP. Both are Tier B: new emitter logic inside the existing owner.

`silent_mode` is **returned to the queue, unimplemented**. Canonical reads every `machine_max_*` key as a stride-2 normal/stealth variant pair (`printer_options_with_variant_2`) and `silent_mode` selects which variant the envelope and the estimator use. PnP's ten machine-limit fields are scalar `Option<f32>` values with no variant dimension, so the key cannot drive anything this packet builds. The missing feature — a per-variant machine-limit model — is named in § Returned to Queue and in the tier table.

## Key Disposition Table

Classification per the map's Authoring rules: **(a)** live behaviour-changing decision point already in tree; **(b)** decision point this packet builds; **(c)** returned to queue (no decision point, not built here); **(d)** dead-in-canonical.

| Key | Class | Owner | Decision point | Non-default AC |
| --- | --- | --- | --- | --- |
| `disable_m73` | **(a)** | `crates/slicer-gcode` (`DefaultGCodeEmitter::emit_gcode`) | the `if !self.resolved_config.disable_m73` gate around `crate::m73::inject_m73` | AC-2 (`true` suppresses every M73 line) |
| `emit_machine_limits_to_gcode` | **(b)** | `crates/slicer-gcode` (emitter) | new: the machine-limit envelope prepended to the GCodeIR command stream, flavor-gated to Marlin/Marlin2/RepRapFirmware | AC-3, AC-4 |
| `enable_power_loss_recovery` | **(b)** | `crates/slicer-gcode` (emitter) | new: M413 S1/S0 emission at the second emitted layer and at the end of the stream, Marlin2 only | AC-5, AC-6 |
| `silent_mode` | **(c)** | — | none — scalar `Option<f32>` machine-limit fields cannot preserve canonical's normal/stealth stride-2 pairs | AC-N2 (honest absence) |

Counts: **(a) 1 · (b) 2 · (c) 1 · (d) 0**, four keys accounted for. Zero declaration-only keys (map preflight gate (a)); every in-packet key carries at least one AC asserting a behaviour change at a non-default value (map preflight gate (b)).

## Returned to Queue — unimplemented

### `silent_mode` — needs a per-variant machine-limit model

`coBool`, canonical default `false`, declared in `PrintConfig.cpp` as a `comDevelop`-gated key. Canonical reads every `machine_max_*` key through `printer_options_with_variant_2`, so each value is a stride-2 array of (normal, stealth) pairs, and `silent_mode` selects which variant the machine envelope and the estimator consume. PnP's ten machine-limit fields (`machine_max_acceleration_extruding`, `machine_max_acceleration_travel`, `machine_max_speed_x/y/z/e`, `machine_max_jerk_x/y/z/e` in `crates/slicer-ir/src/resolved_config.rs`) are scalar `Option<f32>` values with no variant dimension, and the estimator (`EstimatorLimits::from_config`, `crates/slicer-gcode/src/estimator.rs`) reads them directly. Declaring `silent_mode` would be a declaration-only key under Authoring rule 1: there is no decision point it can drive.

**Owner: the P47 motion-limits packet** (ticket 54), which owns the `machine_max_*` family and would have to widen the fields to a per-variant model (or a `silent_mode`-selected variant index) before this key can select anything. Returned here as *unimplemented, needs a per-variant machine-limit model*. The tier row in `04-asset-tier-assignment.md` and the P18 entry in `05-asset-packet-list.md` are updated with this ruling; AC-N2 pins the honest absence from the tree.

## Ruled Dead-in-Canonical

**None.** All four of ticket 25's keys have at least one read site inside OrcaSlicer's slicing pipeline under `src/libslic3r/`, verified per key at authoring time by a delegated sweep: `disable_m73` gates `GCode::_do_export`'s M73 progress emission; `emit_machine_limits_to_gcode` gates `GCode::print_machine_envelope`; `enable_power_loss_recovery` drives `GCodeWriter::enable_power_loss_recovery` from `GCode::_do_export`; `silent_mode` selects the variant index for the stride-2 `machine_max_*` reads. None is GUI-only, `ConfigManipulation.cpp`-only, or in an `IGNORE`/legacy-alias set. `silent_mode` is *returned*, not ruled dead — the key is live in canonical, but PnP's scalar fields cannot express its variant selection.

## Per-Key Canonical Evidence

Cited by file and function, never by line number (repo rule). All reads delegated per the orca-delegation snippet; the evidence below was captured at authoring time and is not re-read unless a worker disputes it.

| Key | Canonical type | Canonical default | Canonical consumer | Current PnP state | Disposition |
| --- | --- | --- | --- | --- | --- |
| `disable_m73` | coBool | `false` | `GCode::_do_export` gates the M73 progress emission on `!disable_m73` | typed `ResolvedConfig` bool field, consumed by the `if !self.resolved_config.disable_m73` gate in `DefaultGCodeEmitter::emit_gcode`; declared in no module manifest (ticket-04 contract violation) | Declare in `machine-gcode-emit.toml`; pin reachability with the existing end-to-end test |
| `emit_machine_limits_to_gcode` | coBool | `true` | `GCode::print_machine_envelope` (`GCode.cpp`): when set and the flavor is Marlin legacy / Marlin2 / RepRapFirmware, emits M201 (per-axis `machine_max_acceleration_x/y/z/e`), M203 (`machine_max_speed_x/y/z/e`; RRF × 60 for mm/min), M204 (P = `machine_max_acceleration_extruding`, R = `machine_max_acceleration_retracting`, T = `machine_max_acceleration_travel`; Marlin legacy substitutes extruding for T), M205 (`machine_max_jerk_x/y/z/e`; RRF emits M566 × 60), M205 J (`machine_max_junction_deviation`, Marlin2 only), and optional M593 input shaping; Klipper/Repetier emit nothing; values are maxima over the used extruders read at the normal-variant index of the stride-2 pairs | zero occurrences in the tree; the ten scalar `Option<f32>` fields exist on `ResolvedConfig` and feed `EstimatorLimits::from_config` only | New bool field (default `true`), declared in the owner manifest; emitter prepends the PnP envelope subset (M203, M204 P/T, M205) to the GCodeIR stream; missing groups recorded as divergences |
| `enable_power_loss_recovery` | coEnum `PowerLossRecoveryMode` | `printer_configuration` | `GCodeWriter::enable_power_loss_recovery` (`GCodeWriter.cpp`): `printer_configuration` → nothing; Marlin2 → `M413 S1`/`M413 S0`; Bambu → `M1003 S1`/`M1003 S0`; all other flavors → nothing. `GCode::_do_export` calls it at the second-layer start (passing the configured mode: `enable` → S1, `disable` → S0) and at object end / after all layers (only when the mode is `enable` → S0) | zero occurrences in the tree; `GcodeFlavor` has no Bambu variant | New string field (default `"printer_configuration"`), declared in the owner manifest as an enum; emitter pushes M413 S1/S0 at the second emitted layer and M413 S0 at the end, Marlin2 only; Bambu form recorded as a divergence |
| `silent_mode` | coBool | `false` | selects the variant index for the stride-2 `machine_max_*` reads (`printer_options_with_variant_2`) consumed by `GCode::print_machine_envelope` and the estimator | zero occurrences in the tree; scalar `Option<f32>` fields have no variant dimension | Returned to the queue — needs a per-variant machine-limit model (P47) |

### Canonical semantics the port borrows exactly

- **Envelope position and order.** `GCode::print_machine_envelope` is called from `GCode::_do_export` before the start gcode, and the groups emit in the order M201, M203, M204, M205, M205 J, input shaping. PnP's envelope is prepended to the GCodeIR command stream so it precedes `machine_start_gcode` (the postpass inserts the start template after the leading envelope run), the M73 pair, and the `ExtrusionMode` command.
- **Flavor gate.** Only Marlin legacy, Marlin2, and RepRapFirmware emit the envelope; Klipper and Repetier emit nothing. The recovery command is Marlin2-only (M413); Bambu's M1003 is out of reach because PnP's `GcodeFlavor` has no Bambu variant.
- **Recovery placement.** `enable` emits the enable command at the second-layer start and the disable command at the end of the print; `disable` emits only the disable command at the second-layer start; `printer_configuration` emits nothing anywhere.
- **RRF units.** RRF firmware speaks mm/min; canonical multiplies the M203 and M566 values by 60 for RRF. PnP's config values are mm/s and the envelope applies the same factor.

### Canonical behaviour the port deliberately does not borrow

- **M201, M204 R, M205 J, and M593 are not emitted.** PnP has no per-axis `machine_max_acceleration_x/y/z/e`, no `machine_max_acceleration_retracting`, no `machine_max_junction_deviation`, and no input-shaping keys — all P47-family fields. Emitting partial lines with invented values would be fabrication; the missing groups are recorded as divergences in `design.md` with the owning packet named.
- **Per-extruder maxima are not computed.** Canonical takes the maximum over the used extruders of each stride-2 value. PnP's fields are scalar globals; the envelope uses the configured scalar directly. Recorded as a form divergence.

## In Scope

1. Declare `disable_m73` (bool, default `false`), `emit_machine_limits_to_gcode` (bool, default `true`), and `enable_power_loss_recovery` (enum, values `printer_configuration`/`enable`/`disable`, default `printer_configuration`) in `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` under `[config.schema]`, each with a `display` and `group = "Machine G-code"`. The manifest is the owner surface for the host emitter's machine keys (ticket-04 ruling); the module itself does not read them.
2. Add `emit_machine_limits_to_gcode: bool` (default `true`) and `enable_power_loss_recovery: String` (default `"printer_configuration"`) to `ResolvedConfig` via the `cli` macro in `crates/slicer-ir/src/resolved_config.rs`, extend `to_config_map`, the manual `PartialEq`, and the manual `Hash` impls, and add both keys to `docs/config/host-keys.toml` `[resolved_config]` with the lock-test arms in `crates/slicer-runtime/tests/unit/host_keys_doc_lock_tdd.rs` (`resolved_bool` for the bool, `resolved_str` for the string).
3. Wire the resolved `GcodeFlavor` into `DefaultGCodeEmitter` (new `flavor` field, default `GcodeFlavor::Marlin`, plus a `with_flavor` builder) and pass it from `run_slice` (`crates/slicer-runtime/src/run.rs` already resolves the flavor for the serializer).
4. Build the machine-limit envelope in `DefaultGCodeEmitter::emit_gcode`: when `emit_machine_limits_to_gcode` is set and the flavor is Marlin/Marlin2/RepRapFirmware, prepend Raw commands to the command stream in canonical order — M203 (`machine_max_speed_x/y/z/e`, RRF × 60), M204 (P = `machine_max_acceleration_extruding`, T = `machine_max_acceleration_travel`, Marlin legacy substituting extruding for T), M205 (`machine_max_jerk_x/y/z/e`, RRF emitting M566 × 60) — each group emitted when at least one contributing field is `Some`, with only the configured axes present. The envelope lands ahead of the M73 pair and the `ExtrusionMode` command.
5. Build the power-loss-recovery emission in the same function: at the start of the second emitted layer (after its `;LAYER_CHANGE`/`;Z:`/`;HEIGHT:` marker triple), push `M413 S1` when the mode is `"enable"` and `M413 S0` when `"disable"`, Marlin2 only; after the last layer's commands, push `M413 S0` when the mode is `"enable"`, Marlin2 only. `"printer_configuration"` and all non-Marlin2 flavors emit nothing.
6. Change `machine-gcode-emit`'s PrintStart insertion rule (`run_gcode_postprocess` in `modules/core-modules/machine-gcode-emit/src/lib.rs`) from "prepend ahead of every command" to "insert after the leading run of Raw commands that are not M73 progress lines", so the host's envelope precedes the start template while the current ordering (start template ahead of the M73 pair and `ExtrusionMode`) is preserved when no envelope is present. Extend `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_tdd.rs` with the envelope case; the existing `machine_start_gcode_precedes_m73_and_extrusion_mode` test must still pass unchanged.
7. Tests: a new TOML-direct-parse guard `modules/core-modules/machine-gcode-emit/tests/machine_gcode_emit_config_schema_tdd.rs` (auto-discovered); envelope and recovery invariants in `crates/slicer-gcode/tests/gcode_emit_tdd.rs`; a bounds arm in `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`; the host-keys lock arms.
8. Regenerate `docs/15_config_keys_reference.md` through `cargo xtask gen-config-docs` and verify it with `--check`.

## Out of Scope

- `ORCA_CONFIG_PADDING` (`crates/slicer-gcode/src/serialize.rs`) and every CONFIG_BLOCK twin, in both directions: this packet neither adds, corrects, nor asserts one. Under the map's Authoring rule 2 the padding table is not parity evidence and is never a deliverable; AC-N3 asserts the file is untouched.
- The P47 machine-limit fields: `machine_max_acceleration_x/y/z/e`, `machine_max_acceleration_retracting`, `machine_max_junction_deviation`, `machine_min_extruding_rate`, `machine_min_travel_rate`. They belong to packet P47 (ticket 54); this packet records their absence as divergences and AC-1 asserts the manifest does not declare them.
- `silent_mode` — returned to the queue (see § Returned to Queue); AC-N2 asserts its honest absence.
- Bambu identity support — PnP's `GcodeFlavor` has no Bambu variant; the M1003 recovery form is a recorded divergence, not a new flavor.
- Input shaping (`input_shaping_*` keys and the M593 emission) — no such keys exist in the tree; recorded as a divergence.
- The estimator's use of the machine-limit fields (`EstimatorLimits::from_config`) — unchanged; the envelope is emission-only.
- New IR/WIT fields or schema-version changes; new modules; new claims.
- Hand edits to `docs/15_config_keys_reference.md` or `docs/ORCA_CONFIG_REFERENCE.md`.

## Authoritative Docs

- `docs/03_wit_and_manifest.md` - delegated SUMMARY of manifest config-schema types, enum values, and inclusive bounds.
- `docs/15_config_keys_reference.md` - generated, targeted checks only; it is not a source file.
- `docs/ORCASLICER_ATTRIBUTION.md` - standard porting-header contract for any new translated Rust file.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` - `GCode::print_machine_envelope` (flavor gate, command order, per-key gating, RRF mm/min factors) and the `GCode::_do_export` call sites for the envelope and for `GCodeWriter::enable_power_loss_recovery` (second-layer and end-of-print placement).
- `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` - `GCodeWriter::enable_power_loss_recovery` (M413 for Marlin2, M1003 for Bambu, empty for all other flavors) and the M201/M203/M204/M205 command forms.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` and `PrintConfig.hpp` - canonical declarations of `emit_machine_limits_to_gcode` (coBool, default true) and `enable_power_loss_recovery` (coEnum `printer_configuration`/`enable`/`disable`, default `printer_configuration`), and the stride-2 normal/stealth variant reads that make `silent_mode` unexpressible in PnP's scalar fields.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` exact owner-manifest schema (three tables, none of the P47 keys, no `silent_mode`); `AC-2` `disable_m73` reachability through the real pipeline; `AC-3` envelope presence, order, and suppression; `AC-4` envelope flavor forms; `AC-5` recovery modes; `AC-6` recovery flavor gate.
- **Map gate (b) coverage.** Each in-packet key has at least one AC asserting a behaviour change at a non-default value: `disable_m73` -> AC-2 (`true` suppresses every M73 line); `emit_machine_limits_to_gcode` -> AC-3 (`false` suppresses the envelope; the envelope's presence with fields set is the positive half) and AC-4 (flavor forms); `enable_power_loss_recovery` -> AC-5 (`enable` vs `disable` vs `printer_configuration` produce different streams) and AC-6 (flavor gate). No key's only evidence is a default-path identity, and no AC asserts a CONFIG_BLOCK line.
- Negative: `AC-N1` bounds/enum rejection; `AC-N2` `silent_mode` not re-stubbed; `AC-N3` no padding edits.
- Cross-packet impact: P47 (ticket 54) owns the missing machine-limit fields this packet records as divergences; the `silent_mode` follow-up is filed as ticket 117. No other queued packet claims the three in-packet keys.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the closure subset.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p machine-gcode-emit --test machine_gcode_emit_config_schema_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-1 manifest schema guard | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p pnp-cli --test m73_progress_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-2 end-to-end `disable_m73` reachability | FACT pass/fail |
| `cargo test -p slicer-gcode --test gcode_emit_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-3, AC-4, AC-5, AC-6 emitter behaviour | FACT pass/fail |
| `cargo test -p machine-gcode-emit --test machine_gcode_emit_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | PrintStart-after-envelope ordering + existing ordering regression | FACT pass/fail |
| `cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | AC-N1 real manifest enforcement | FACT pass/fail |
| `cargo test -p slicer-runtime --test unit host_keys_doc_lock_tdd 2>&1 \| tee target/test-output.log \| grep -E '^test result'` | host-keys lock with the two new rows | FACT pass/fail |
| `cargo xtask gen-config-docs` | regenerate the generated reference | FACT exit code |
| `cargo xtask gen-config-docs --check` | generated-reference check | FACT exit code |
| `cargo xtask build-guests --check` | guest freshness after the module source/manifest edits | FACT exit code; stale means rebuild without `--check` |
| `cargo check --workspace --all-targets` | workspace compile gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals` | struct-literal churn gate | FACT exit code |

Commands must have small, parseable output suitable for delegation; every test run writes the required `target/test-output.log`.

## Step Completion Expectations

- The manifest step and the schema guard land together; the guard must assert all three new tables and the absence of the P47 keys and `silent_mode`.
- The `ResolvedConfig` field step and the host-keys/lock-test step land together, or the lock test fails on the new rows.
- The emitter behaviour and its emission tests land together so every mode and flavor has an invariant before the module ordering change is made.
- The module PrintStart change must keep `machine_start_gcode_precedes_m73_and_extrusion_mode` green; the envelope case is an addition, never a rewrite of that contract.
- Generated docs are regenerated only after all manifests and source/config tests are final; guest freshness is checked after the final guest-input changes.

## Context Discipline Notes

- `crates/slicer-gcode/src/emit.rs` is long; only `DefaultGCodeEmitter`'s struct, builders, `emit_gcode` head, layer loop, and the M73/estimator tail are needed — ranged reads anchored on those symbols.
- `modules/core-modules/machine-gcode-emit/src/lib.rs` — only `run_gcode_postprocess`'s Step 4 (PrintStart) and the injection registry are in scope.
- `crates/slicer-ir/src/resolved_config.rs` — only the `cli` macro invocation, `to_config_map`, `PartialEq`, and `Hash` regions; the file is long, do not browse it.
- Canonical files remain delegated and all cargo commands remain delegated; retain only FACT/LOCATIONS/SNIPPETS returns.
