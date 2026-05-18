---
status: active
packet: 59_machine-start-end-gcode-emission
task_ids:
  - TASK-193    # emit configurable machine_start_gcode / machine_end_gcode via a PostPass::GCodePostProcess module that prepends/appends Raw commands carrying the resolved templates
  - TASK-193a   # create modules/core-modules/machine-gcode-emit/ declaring four [config.schema.*] keys; run_gcode_postprocess performs real [key] substitution against the effective ConfigView and prepends/appends Raw commands
  - TASK-193b   # promote M82/M83 from the hard-coded serializer preamble to a new GCodeCommand::ExtrusionMode { absolute: bool } variant pushed by DefaultGCodeEmitter, so a downstream GCodePostProcess module can prepend before it
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 59_machine-start-end-gcode-emission

## Goal

Emit a configurable printer start sequence before the first move and a configurable finish sequence after the last move, produced by a NEW core module that runs at the existing `PostPass::GCodePostProcess` stage. The module reads four config keys, performs `[key_name]` substitution against the effective `ConfigView` inside its WASM guest, and rebuilds `GCodeIR.commands` as `[Raw(resolved_start), ...existing commands..., Raw(resolved_end)]`.

To make this position-correct, the existing hard-coded `M82`/`M83` strings written by `DefaultGCodeSerializer::serialize_gcode` at `crates/slicer-host/src/gcode_emit.rs:1154-1156` are promoted to a new `GCodeCommand::ExtrusionMode { absolute: bool }` variant that `DefaultGCodeEmitter::emit_gcode` pushes at the head of the commands list. After the promotion, every byte the serializer writes between `HEADER_BLOCK_END` and `CONFIG_BLOCK_START` comes either from `GCodeIR.commands` or from the `ThumbnailAwareSerializer` wrapper — nothing is hard-coded inside the per-command loop. This is what lets the `GCodePostProcess` module prepend `Raw(machine_start_gcode)` *before* `M82`/`M83`, matching OrcaSlicer ordering.

Concretely:

1. Add `GCodeCommand::ExtrusionMode { absolute: bool }` to `crates/slicer-ir/src/slice_ir.rs:1697` (the existing `GCodeCommand` enum, currently `Move, Retract, Unretract, FanSpeed, Temperature, ToolChange, Comment, Raw`).
2. In `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-host/src/gcode_emit.rs:304`), push `ExtrusionMode { absolute }` as the first command (`absolute` derived from whatever the existing preamble decision is — currently the M83/M82 branch at `:1154-1156`).
3. In `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-host/src/gcode_emit.rs:1107`), remove the hard-coded `M82`/`M83` preamble writes at `:1154-1156`; add a `GCodeCommand::ExtrusionMode { absolute }` match arm in the command-loop renderer (which lives around the `Temperature` arm at `:1280-1281`).
4. Create `modules/core-modules/machine-gcode-emit/` with three files (`machine-gcode-emit.toml`, `Cargo.toml`, `src/lib.rs`), mirroring `modules/core-modules/part-cooling/` in file shape but with `[stage] id = "PostPass::GCodePostProcess"`. The module's `run_gcode_postprocess` implementation:
   - Reads `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer` from `&ConfigView`.
   - Builds a `HashMap<String, ConfigValue>` lookup for substitution.
   - Calls a private `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper (≤ 60 LOC, scoped to this module).
   - On the `&mut gcode-output-builder`: pushes the resolved start as one `Raw` command, re-pushes every command from the snapshot `list<gcode-command>` argument (or calls the SDK's `extend_from_snapshot` equivalent — whatever pattern the existing GCodePostProcess SDK trait exposes), then pushes the resolved end as one `Raw` command.
   - If a resolved template is empty or whitespace-only, the corresponding `Raw` push is SKIPPED (no phantom empty command).
   - Returns `Ok(())`.
5. Declare the four `[config.schema.<key>]` blocks in the new module's manifest with defaults:
   - `machine_start_gcode` (string) = `"M190 S[bed_temperature_initial_layer_single]\nM109 S[nozzle_temperature_initial_layer]\nPRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]"` (multi-line, TOML triple-quoted).
   - `machine_end_gcode` (string) = `"PRINT_END"`.
   - `bed_temperature_initial_layer_single` (int) = `60`, `min = 0`, `max = 120` (declarative only — see Out of Scope).
   - `nozzle_temperature_initial_layer` (int) = `215`, `min = 0`, `max = 300` (declarative only).

What this design intentionally does NOT add:
- No new WIT methods on any builder resource.
- No new `FinalizationBuilderPush` variants.
- No new `Option<String>` fields on `GCodeIR`.
- No new dispatch match arms in `dispatch.rs`.
- No byte-offset-targeting code in the serializer (positioning falls out of the command list naturally).

Substitution is INTENTIONALLY a narrow subset of OrcaSlicer's `PlaceholderParser`. Arithmetic, conditionals, loops, and builtins are out of scope and tracked as future packets.

## Scope Boundaries

- In scope:
  - New `GCodeCommand::ExtrusionMode { absolute: bool }` variant in `crates/slicer-ir/src/slice_ir.rs:1697`. Additive only — no existing variant removed.
  - `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-host/src/gcode_emit.rs:304`) pushes `ExtrusionMode { absolute }` as the head command.
  - `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-host/src/gcode_emit.rs:1107`) drops the hard-coded `M82`/`M83` preamble writes at `:1154-1156` and adds an `ExtrusionMode` arm in the per-command renderer.
  - New core module `modules/core-modules/machine-gcode-emit/` with `machine-gcode-emit.toml`, `Cargo.toml`, `src/lib.rs`, declaring `[stage] id = "PostPass::GCodePostProcess"` and four `[config.schema.<key>]` blocks.
  - Module `src/lib.rs` implements the SDK trait corresponding to `run-gcode-postprocess` with a real (non-no-op) body: reads keys → substitutes → prepends `Raw(start)` → re-emits snapshot → appends `Raw(end)`.
  - Private `substitute_placeholders` helper inside the module (≤ 60 LOC).
  - New TDD test file `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` (9 positive + 3 negative = 12 tests).
  - WASM build via `./modules/core-modules/build-core-modules.sh` and `--check` clean.
  - Append TASK-193 / TASK-193a / TASK-193b rows to `docs/07_implementation_status.md` via worker dispatch.
- Out of scope:
  - Any change to `wit/world-finalization.wit`, `wit/world-postpass.wit`, or any other WIT file beyond what the IR variant addition requires (which may be zero — `GCodeCommand` is represented via `ir-types.wit` at `wit/deps/ir-types.wit`; if `ExtrusionMode` is exposed across the WIT boundary, that addition is in scope but limited to the variant itself).
  - Any change to `FinalizationBuilderPush` or `dispatch.rs` — the simpler design does not need them.
  - Any change to `GCodeIR` struct fields. The `commands: Vec<GCodeCommand>` field is the only carrier.
  - Range enforcement of `min`/`max` from the manifest. `ResolvedConfig::apply_cli_key` (`crates/slicer-ir/src/resolved_config.rs:194`) does not consult `min`/`max`. Tracked as a separate future packet.
  - Arithmetic in placeholders (`[bed*2]`), conditionals (`{if}{endif}`), loops (`{for}{endfor}`), builtin functions. Tracked as separate future packets.
  - `{var}` brace-syntax placeholders. Only `[key]` square-bracket syntax.
  - Per-extruder / per-region / per-object placeholders.
  - OrcaSlicer's custom-gcode placeholder validator.
  - Adopting OrcaSlicer's stock `machine_start_gcode` / `machine_end_gcode` defaults. We intentionally use Klipper PRINT_START / PRINT_END macros.
  - Multi-extruder M104/M109 tool-index variants.
  - Modifying `ThumbnailAwareSerializer` ordering or CONFIG_BLOCK contents beyond the four new keys naturally appearing via packet 55's automatic CONFIG_BLOCK propagation.
  - Cross-component WARN-log forwarding from the WASM guest to the host for unknown placeholders. With substitution in the guest, host-side log capture is not trivially available; the negative AC asserts verbatim passthrough only.

## Prerequisites and Blockers

- Depends on:
  - Packet 55 (HEADER_BLOCK + CONFIG_BLOCK + `ThumbnailAwareSerializer`) — landed. The end-of-print insertion is implicit: anything in `GCodeIR.commands` is written by the inner serializer before the wrapper appends `CONFIG_BLOCK`.
  - Packet 54 (M82/M83 preamble emission inside the serializer) — landed. This packet REPLACES the hard-coded `M82`/`M83` strings at `gcode_emit.rs:1154-1156` with a `GCodeCommand::ExtrusionMode` variant pushed by the emitter, but the EFFECT (M82 or M83 written between header and the first layer) is preserved bit-identically for default configs.
  - Existing `PostPass::GCodePostProcess` stage in `crates/slicer-host/src/execution_plan.rs:38` and its dispatcher at `crates/slicer-host/src/postpass.rs:215`. Already wired; no new stage added.
  - Existing `gcode-output-builder.push-raw` method at `wit/deps/ir-types.wit:144` and `run-gcode-postprocess` export at `wit/world-postpass.wit:26`. Already defined; no new WIT surface added.
- Unblocks:
  - Future "OrcaSlicer placeholder-parser arithmetic" packet (extends the module's `substitute_placeholders`).
  - Future "OrcaSlicer placeholder-parser control flow" packet.
  - Future "printer-profile import" packet.
  - Future "manifest range enforcement" packet (would wire `ConfigFieldEntry::min/max` into `ResolvedConfig::apply_cli_key`).
  - Future "cross-component log forwarding" packet.
- Activation blockers:
  - None. The architecture is consistent with the existing scheduler, IR, and SDK contracts. The only IR surface change is one additive variant on `GCodeCommand`.

## Acceptance Criteria

- **Given** a `slicer-cli` invocation with no config overrides on the standard small STL fixture, **when** the produced `out.gcode` is scanned, **then** each of the three lines `M190 S60`, `M109 S215`, and `PRINT_START EXTRUDER=215 BED=60` appears exactly once. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_gcode_default_substitutes --nocapture`
- **Given** a `slicer-cli` invocation with no config overrides, **when** `out.gcode` is scanned, **then** `PRINT_END` appears exactly once and no other line beginning with `PRINT_` is present outside the start block. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- end_gcode_default_emits_print_end --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** byte offsets are compared, **then** the byte-offset of `M190 S60` is strictly greater than the byte-offset of `; HEADER_BLOCK_END`, strictly less than the byte-offset of the first `M82` or `M83` line, and strictly less than the byte-offset of the first `G1` line containing a non-zero `E` token. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_block_position_before_extrusion_mode_and_first_g1 --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** byte offsets are compared, **then** the byte-offset of `PRINT_END` is strictly greater than the byte-offset of the last `G1` line and strictly less than the byte-offset of `; CONFIG_BLOCK_START`. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- end_block_position_after_last_g1_before_config_block --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** scanned, **then** exactly one of `M82` or `M83` (whichever the emitter selects for the default flavor) appears between `; HEADER_BLOCK_END` and the first `G1` extrusion move — this is the regression sentry for promoting M82/M83 from the hard-coded preamble to a `GCodeCommand::ExtrusionMode` variant. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- extrusion_mode_still_emitted_after_promotion --nocapture`
- **Given** a `--config user.json` setting `machine_start_gcode = "G28 ; home all\nG1 Z5 F600"`, **when** `out.gcode` is scanned, **then** `G28 ; home all` and `G1 Z5 F600` each appear exactly once between `; HEADER_BLOCK_END` and the first `M82`/`M83` line, AND no `M190`, `M109`, or `PRINT_START` line appears anywhere in the file. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- user_override_replaces_default --nocapture`
- **Given** a `--config user.json` setting `bed_temperature_initial_layer_single = 65` and `nozzle_temperature_initial_layer = 220` (default `machine_start_gcode`), **when** `out.gcode` is scanned, **then** the start block contains `M190 S65`, `M109 S220`, and `PRINT_START EXTRUDER=220 BED=65` (each exactly once) and contains no `S60`, `S215`, `EXTRUDER=215`, or `BED=60` substring. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- substitution_uses_overridden_temp_values --nocapture`
- **Given** a `--config user.json` setting `machine_end_gcode = ""` (empty string override), **when** `out.gcode` is scanned, **then** `PRINT_END` is absent from the file AND the byte range between the last `G1` line's terminating `\n` and `; CONFIG_BLOCK_START` contains zero non-whitespace characters. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- empty_end_gcode_emits_no_block --nocapture`
- **Given** the `machine-gcode-emit` module manifest is loaded via the existing host manifest-discovery path, **when** the four keys are queried, **then** each is registered with type (`string`, `string`, `int`, `int`) and default value matching the Goal section. If the manifest-discovery API is not directly testable, the assertion falls back to verifying that `; machine_start_gcode = ...`, `; machine_end_gcode = PRINT_END`, `; bed_temperature_initial_layer_single = 60`, and `; nozzle_temperature_initial_layer = 215` each appear in the CONFIG_BLOCK of a default-config `out.gcode`. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- module_manifest_registers_four_keys_with_expected_types_and_defaults --nocapture`
- **Given** a default slicing run, **when** `; CONFIG_BLOCK_START` .. `; CONFIG_BLOCK_END` is parsed, **then** each of `; machine_start_gcode = ...`, `; machine_end_gcode = PRINT_END`, `; bed_temperature_initial_layer_single = 60`, `; nozzle_temperature_initial_layer = 215` appears exactly once. Multi-line `machine_start_gcode` may be emitted on a single comment line with `\n` literalized as `\\n` or via packet-55's existing multi-line-value convention; the test asserts exact-key-presence and exact-value-equality after un-escaping. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- new_keys_appear_in_config_block --nocapture`

## Negative Test Cases

- **Given** a `--config user.json` setting `machine_start_gcode = "TEMP [no_such_key] DONE"`, **when** the slicer is run, **then** the produced output contains the literal substring `TEMP [no_such_key] DONE` exactly once between `; HEADER_BLOCK_END` and the first `M82`/`M83` line; substitution does not panic. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- unknown_placeholder_passes_through_verbatim --nocapture`
- **Given** a `--config user.json` setting `machine_start_gcode = "PREFIX [unclosed SUFFIX"` (no closing `]` on the same line), **when** the slicer is run, **then** the output contains the literal `PREFIX [unclosed SUFFIX` exactly as written, substitution does not panic and does not infinite-loop. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- unclosed_bracket_treated_as_literal --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** scanned, **then** the literal substring `M190` does NOT appear inside the byte range `; HEADER_BLOCK_START` .. `; HEADER_BLOCK_END` and does NOT appear inside the byte range `; CONFIG_BLOCK_START` .. `; CONFIG_BLOCK_END` (the CONFIG_BLOCK `; machine_start_gcode = ...` line is comment-prefixed and trivially excluded by the substring-position check). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_block_not_inside_other_blocks --nocapture`

## Verification

- `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` — dispatch as FACT pass/fail; SNIPPETS (≤ 20 lines) on first-failing-assertion.
- `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — packet-55 regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — packet-52/54 regression (CRITICAL: this suite asserts M82/M83 presence; after the promotion it MUST still pass).
- `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` — postpass pipeline regression.
- `cargo test -p slicer-ir gcode_command` — `GCodeCommand::ExtrusionMode` variant unit tests (if a unit-test module exists; otherwise omit).
- `./modules/core-modules/build-core-modules.sh --check` — guest WASM freshness (CLAUDE.md Guest WASM Staleness).
- `./test-guests/build-test-guests.sh --check` — test-guest freshness (relevant if any test-guest depends on `slicer-ir` via `slicer-macros`/`slicer-sdk`; CLAUDE.md flags those crates as guest deps).
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization vs postpass stages; delegate a SUMMARY (file is long).
- `docs/02_ir_schemas.md` — `GCodeIR`, `GCodeCommand`, `ConfigValue` enum; range-read only the `GCodeCommand` enum section near `:1697` of `slice_ir.rs` and the `ConfigValue` section near `:557`.
- `docs/03_wit_and_manifest.md` — module-manifest schema, `[config.schema.<key>]` blocks. Load only the schema-validation paragraph.
- `docs/05_module_sdk.md` — `#[slicer_module]` macro and the `GCodePostProcessModule` (or equivalently-named) trait. Range-read only the smallest example with a non-no-op body.
- `docs/07_implementation_status.md` — DELEGATE every read. Step 1 of this packet appends three TASK-### rows via a worker.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — no deviation expected in this refined design; if review requires (e.g., for OrcaSlicer's end-gcode-after-CONFIG-block ordering), file there.

For each doc above: if > 300 lines, delegate. Default rule wins.

## OrcaSlicer Reference Obligations

All reads delegated; never load OrcaSlicer source into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — FACT, ≤ 6 lines: confirm field names are exactly `machine_start_gcode` and `machine_end_gcode` (snake_case).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — FACT, ≤ 8 lines: confirm OrcaSlicer stock defaults; we DELIBERATELY do not adopt them.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (the regions around the `machine_start_gcode` write and the M82/M83 preamble) — FACT, ≤ 12 lines: confirm ordering is `machine_start_gcode` THEN extrusion-mode preamble. Our design produces this ordering naturally because `Raw(start)` is prepended before the `ExtrusionMode` command.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (the `machine_end_gcode` write region) — FACT, ≤ 6 lines: confirm end gcode is the final block before file close in OrcaSlicer; in our flow it comes BEFORE the wrapper's `CONFIG_BLOCK` because `CONFIG_BLOCK` is structurally a metadata footer that the printer ignores. Document as an intentional deviation if review requires.
- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (just the `apply_config()` entry point) — FACT, ≤ 10 lines: confirm placeholder values are sourced from the same config symbol table that holds the temperature keys. Do NOT read the full `process()` grammar — out of scope.

## Packet Files

- `requirements.md`
- `design.md`
- `implementation-plan.md`
- `task-map.md`

## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- Treat `design.md`'s code change surface as the authoritative files-in-scope list.
- Honor `design.md`'s out-of-bounds list — those files must not be loaded directly.
- Delegate every `cargo` invocation and every authoritative-doc fact-check.
- Stop reading at 60% context and hand off at 85%.

Specific hazards:

- `crates/slicer-host/src/gcode_emit.rs` is **> 1100 lines** (above the 600-line direct-read budget). Range-read `:300-:340` (`DefaultGCodeEmitter::emit_gcode` entry at `:304`), `:670-:740` (header + width comments), `:1100-:1170` (`serialize_gcode` body + the soon-to-be-removed M82/M83 writes at `:1154-:1156`), and `:1270-:1300` (existing per-command renderer arms — `Temperature` at `:1280-:1281`). Never load the full file.
- `crates/slicer-ir/src/slice_ir.rs` is **> 1600 lines**. Range-read `:1697-:1770` (`GCodeCommand` enum) and `:1779-:1799` (`GCodeIR` struct). Never load the full file.
- `crates/slicer-host/src/postpass.rs` — range-read `:140-:280` (the `GCodeEmitter` / `GCodeSerializer` traits and the `execute_postpass_with_instrumentation` body where `run_gcode_postprocess` is dispatched).
- `crates/slicer-host/src/execution_plan.rs:38-:80` — confirm `PostPass::GCodePostProcess` is in the canonical stage-ID list.
- `wit/world-postpass.wit` — full read OK (file is short).
- `wit/deps/ir-types.wit` — full read OK (≤ 200 lines). Confirm `gcode-output-builder.push-raw` and the `gcode-command` variant set.
- `modules/core-modules/part-cooling/{part-cooling.toml, Cargo.toml, src/lib.rs}` — full reads OK; these are the manifest-shape and `#[slicer_module]` precedent the implementer copies. Note that `part-cooling` uses `PostPass::LayerFinalization`, NOT `GCodePostProcess`; the new module switches stage but reuses the file shape.
- `OrcaSlicerDocumented/` MUST be delegated. The five FACT dispatches enumerated above are the only OrcaSlicer evidence this packet needs.
- `docs/07_implementation_status.md` is > 500 lines. NEVER load it directly.

If the implementer cannot find an existing core module that uses `PostPass::GCodePostProcess` as its stage, the SDK trait shape and the `run-gcode-postprocess` export must be inferred from `wit/world-postpass.wit:26` and `crates/slicer-sdk/src/traits.rs` via ranged read. A single sub-agent dispatch identifying the SDK trait name + signature is permitted.

Sub-agent return formats:

- OrcaSlicer FACTs (5 dispatches above): ≤ 12 lines each, no code blocks > 4 lines.
- `cargo test`: FACT pass/fail; SNIPPETS (≤ 20 lines) on first-failing-assertion.
- SDK / WIT identification dispatch (Step 4): FACT — trait name + signature + file:line, ≤ 8 lines.
- Module-manifest registration check (end of Step 4): LOCATIONS list of `[config.schema.<key>]` blocks (expected count = 4), ≤ 6 entries.
- Serializer rewrite check (Step 3): SNIPPETS (≤ 30 lines) of the pre- and post-change byte ranges for the M82/M83 removal and the new `ExtrusionMode` arm.

### Test Fixture Convention

- Tests reuse the small `.stl` fixture used by `gcode_emit_tdd.rs` and `gcode_header_thumbnail_config_blocks_tdd.rs` (resolve via `concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/<fixture>.stl")` — confirm the exact filename via a single LOCATIONS dispatch in Step 2). NO new STL fixture is committed.
- `--config user.json` overrides are materialized inline at test runtime via `std::env::temp_dir()` + `serde_json::to_writer`. NO new committed config fixtures.

### Test Discipline Reminder

Per `CLAUDE.md` / Test Discipline: `cargo test --workspace` is FORBIDDEN as a per-AC verification command. Every AC uses the targeted `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- <test_name> --nocapture` form. The packet-level workspace gate appears only at the closure ceremony in `implementation-plan.md` Step 6 and is dispatched to a sub-agent that returns FACT pass/fail.

Aggregate context cost: M. No step is L. If implementation reveals that promoting M82/M83 to `GCodeCommand::ExtrusionMode` causes unanticipated test breakage outside `gcode_emit_tdd` (e.g., if any other suite asserts the exact M82/M83 byte position via the now-removed hard-coded preamble), surface as a packet-local risk in `design.md` Open Questions and split that work into a follow-up packet rather than expanding this packet's scope.
