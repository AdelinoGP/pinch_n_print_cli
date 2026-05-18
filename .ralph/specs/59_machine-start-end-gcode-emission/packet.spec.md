---
status: draft
packet: 59_machine-start-end-gcode-emission
task_ids:
  - TASK-193    # emit machine_start_gcode / machine_end_gcode with minimal [key] substitution at correct serializer positions
  - TASK-193a   # create modules/core-modules/machine-gcode-emit/ owning the four config keys and running real [key] substitution in run_finalization
  - TASK-193b   # extend FinalizationBuilderPush + finalization-output-builder WIT with PrintStartGcode / PrintEndGcode print-boundary variants
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 59_machine-start-end-gcode-emission

## Goal

Emit a configurable printer start sequence before the first extrusion move and a configurable finish sequence after the last move, both produced by a NEW core module that performs real `[key_name]` placeholder substitution against `ResolvedConfig` and ships the resolved strings to the host serializer via two NEW print-boundary variants on the existing `FinalizationBuilderPush` enum. Concretely:

1. Create a new core module `modules/core-modules/machine-gcode-emit/`, mirroring `modules/core-modules/part-cooling/` in file shape, with three files:
   - `machine-gcode-emit.toml` — manifest declaring four `[config.schema.<key>]` blocks (verbatim TOML below).
   - `Cargo.toml` — mirrors `modules/core-modules/part-cooling/Cargo.toml`.
   - `src/lib.rs` — implements `FinalizationModule` with a REAL `run_finalization` body (NOT a no-op). It reads `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, and `nozzle_temperature_initial_layer` from the `&ConfigView` argument, runs a private `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper (≤ 60 LOC, scoped to this module) on both templates, pushes the resolved start string via `output.push_print_start_gcode(resolved_start)` and the resolved end string via `output.push_print_end_gcode(resolved_end)`, then returns `Ok(())`.
   - The four config keys with declared defaults:
     - `machine_start_gcode` (string) — default exactly:
       ```
       M190 S[bed_temperature_initial_layer_single]
       M109 S[nozzle_temperature_initial_layer]
       PRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]
       ```
     - `machine_end_gcode` (string) — default exactly `PRINT_END`.
     - `bed_temperature_initial_layer_single` (int) — default `60`; `min = 0`, `max = 120` (declarative metadata only — see Out of Scope).
     - `nozzle_temperature_initial_layer` (int) — default `215`; `min = 0`, `max = 300` (declarative metadata only — see Out of Scope).

2. Extend the WIT contract (`wit/world-finalization.wit:62-104`, the `finalization-output-builder` resource). Add two new methods (additive — no existing method removed or changed):
   - `push-print-start-gcode: func(text: string) -> result<_, string>;`
   - `push-print-end-gcode: func(text: string) -> result<_, string>;`

3. Extend the SDK trait (`crates/slicer-sdk/src/traits.rs`). The existing `FinalizationOutputBuilder` impl at `:717` and the `FinalizationModule` trait at `:1196` are unchanged in signature. Add two methods on `FinalizationOutputBuilder` impl: `push_print_start_gcode(&mut self, text: String)` and `push_print_end_gcode(&mut self, text: String)`. Add two corresponding variants to the `FinalizationBuilderPush` enum currently defined at `crates/slicer-host/src/wit_host.rs:837` (the enum has 6 variants today; this adds variants 7 and 8): `PrintStartGcode(String)` and `PrintEndGcode(String)`.

4. Extend dispatch routing (`crates/slicer-host/src/dispatch.rs`). The existing apply-site loop at `:2892-2978` (inside `dispatch_finalization_call` at `:1079`) processes 6 variants today. Add match arms for the 2 new variants that DEPOSIT the resolved strings into NEW `Option<String>` fields on `GCodeIR` (`print_start_gcode`, `print_end_gcode`) added to the struct at `crates/slicer-ir/src/slice_ir.rs:1781`. (Selected from the two routing options in `design.md` — see Rejected Alternatives for the trade-off.)

5. Extend the host serializer (`crates/slicer-host/src/gcode_emit.rs`). The serializer has NO `substitute_placeholders` helper — substitution happens inside the WASM guest, not host-side. The serializer READS `gcode_ir.print_start_gcode` and `gcode_ir.print_end_gcode` from the IR and INSERTS them at exact byte positions inside `DefaultGCodeSerializer::serialize_gcode()` body at `:1021`:
   - Start block: AFTER the `serialize_header_block` (`:667`) + `serialize_width_comments` (`:712`) emission, BEFORE the M82/M83 preamble emission (M83 at `:1067`, M82 at `:1069`). Followed by one trailing `\n` when non-empty.
   - End block: AFTER the last layer's commands, BEFORE the `ThumbnailAwareSerializer` (`:973`) wrapper's THUMBNAIL/CONFIG_BLOCK append. The inner serializer appends the end block at the end of its buffer; the wrapper concatenates THUMBNAIL/CONFIG after that.
   - Empty / whitespace-only resolved string ⇒ emit zero bytes (no header comment, no blank line, no phantom marker).

Order of operations matches OrcaSlicer (start gcode BEFORE preamble; see `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3200` then `:3258`). Substitution is INTENTIONALLY a narrow subset of OrcaSlicer's `PlaceholderParser`; arithmetic / conditionals / loops are out of scope and tracked as future packets.

## Scope Boundaries

- In scope:
  - New core module `modules/core-modules/machine-gcode-emit/` with three files exactly as enumerated in the Goal.
  - WIT contract extension: two new methods on `finalization-output-builder` (`wit/world-finalization.wit:62-104`).
  - SDK extension: two new variants on `FinalizationBuilderPush` (`crates/slicer-host/src/wit_host.rs:837` — currently 6 variants) + two new methods on `FinalizationOutputBuilder` impl (`crates/slicer-sdk/src/traits.rs:717`).
  - Dispatch routing extension: two new match arms inside `dispatch_finalization_call` apply-site loop (`crates/slicer-host/src/dispatch.rs:2892-2978`) that deposit resolved strings into new `GCodeIR` fields.
  - IR additive extension: two `Option<String>` fields on `GCodeIR` (`crates/slicer-ir/src/slice_ir.rs:1781`) — `print_start_gcode` and `print_end_gcode`. No variants removed, no existing field changed.
  - Host serializer wiring: read those two fields inside `DefaultGCodeSerializer::serialize_gcode()` and insert at the contractual byte positions specified above.
  - New TDD test file `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` (9 positive + 3 negative = 12 tests).
  - Build the new module's `.wasm` via `./modules/core-modules/build-core-modules.sh` and confirm `--check` returns clean.
  - Append TASK-193, TASK-193a, TASK-193b rows to `docs/07_implementation_status.md` via worker dispatch.
- Out of scope:
  - **Range enforcement of `min`/`max` from module manifest.** `min`/`max` are parsed into `ConfigFieldEntry` (`crates/slicer-host/src/manifest.rs:827-828`) but not enforced at runtime — `ResolvedConfig::apply_cli_key` (`crates/slicer-ir/src/resolved_config.rs:194`) accepts any numeric value. Tracked as a separate future TASK-### packet. The declared ranges in this packet's manifest are declarative-only.
  - Arithmetic in placeholders (`[bed*2]`, `[x+5]`). Tracked as a separate future packet.
  - Conditionals (`{if ...}{endif}`), loops (`{for ...}{endfor}`), builtin functions. Tracked as separate future packets.
  - `{var}` brace-syntax placeholders. Only `[key]` square-bracket syntax in this packet.
  - Per-extruder / per-region / per-object placeholders.
  - OrcaSlicer's custom-gcode placeholder validator.
  - Adopting OrcaSlicer's stock `machine_start_gcode` / `machine_end_gcode` defaults. We intentionally use the user-specified Klipper PRINT_START / PRINT_END macros.
  - Adding ANY further temperature / fan / filament / printer-profile keys beyond the four enumerated.
  - Multi-extruder M104/M109 tool-index variants (`M109 T1 S...`).
  - Real-PRINT_START macro semantics (homing, ABL probing, purge). Those live in printer firmware / Klipper config.
  - Modifying `M82`/`M83`/`G90`/`G21` preamble emission established by packet 54.
  - Modifying `ThumbnailAwareSerializer` ordering or CONFIG_BLOCK contents (beyond the four new keys naturally appearing via packet 55's automatic CONFIG_BLOCK propagation).
  - Cross-component WARN-log forwarding from the WASM guest to the host. With substitution in the guest, host-side `log::warn!` capture for unknown placeholders is not trivially available; the negative AC for unknown placeholders has been relaxed to a verbatim-presence check (renamed `unknown_placeholder_passes_through_verbatim`). Forwarding is tracked as a separate future packet.

## Prerequisites and Blockers

- Depends on:
  - Packet 55 (HEADER_BLOCK + CONFIG_BLOCK + `ThumbnailAwareSerializer`) — landed. This packet's start-block insertion site sits AFTER `serialize_header_block` (`:667`) + `serialize_width_comments` (`:712`); end-block sits BEFORE the `ThumbnailAwareSerializer` (`:973`) wrapper's THUMBNAIL/CONFIG append.
  - Packet 54 (M82/M83 preamble + `with_extrusion_mode` constructor) — landed. This packet's start-block insertion site is "immediately before the preamble line(s) at gcode_emit.rs:1067-1069".
- Unblocks:
  - Future "OrcaSlicer placeholder-parser arithmetic" packet (extends the module's substitution helper to support `[a+b]`, `[a*b]`).
  - Future "OrcaSlicer placeholder-parser control flow" packet (adds `{if}{elsif}{else}{endif}` and `{for}`).
  - Future "printer-profile import" packet (a JSON/INI loader for OrcaSlicer printer profiles would populate these four keys among many others).
  - Future "manifest range enforcement" packet (wires `ConfigFieldEntry::min/max` into `ResolvedConfig::apply_cli_key`).
  - Future "cross-component WARN-log forwarding" packet (re-introduces the dropped WARN assertion against unknown placeholders).
- Activation blockers:
  - No activation blockers remaining. The four keys are owned by the new `machine-gcode-emit` core module; the module runs real `[key]` substitution against `ResolvedConfig` and emits the resolved strings via two new `FinalizationBuilderPush` variants (additive WIT/SDK change). The host serializer consumes the resolved strings from `GCodeIR` fields and places them at the contractual byte offsets.

## Acceptance Criteria

- **Given** a `slicer-cli` invocation with no config overrides on a small fixture model, **when** the produced `out.gcode` is scanned, **then** the three lines `M190 S60`, `M109 S215`, and `PRINT_START EXTRUDER=215 BED=60` each appear exactly once. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_gcode_default_substitutes --nocapture`
- **Given** a `slicer-cli` invocation with no config overrides, **when** `out.gcode` is scanned, **then** `PRINT_END` appears exactly once (the default `machine_end_gcode`) and no other lines beginning with `PRINT_` are present. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- end_gcode_default_emits_print_end --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** byte offsets are compared, **then** the first byte-offset of the `M190 S60` line is strictly greater than the byte-offset of `HEADER_BLOCK_END` and strictly less than the byte-offset of the first `G1` line with a non-zero `E` token (the first extrusion move). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_block_position_after_header_before_first_g1 --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** byte offsets are compared, **then** the byte-offset of the `PRINT_END` line is strictly greater than the byte-offset of the last `G1` line and strictly less than the byte-offset of `CONFIG_BLOCK_START`. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- end_block_position_after_last_g1_before_config_block --nocapture`
- **Given** a `slicer-cli` invocation with `--config user.json` setting `machine_start_gcode = "G28 ; home all\nG1 Z5 F600"`, **when** `out.gcode` is scanned, **then** the two lines `G28 ; home all` and `G1 Z5 F600` each appear exactly once between `HEADER_BLOCK_END` and the first extrusion move, AND no `M190`, `M109`, or `PRINT_START` line appears anywhere in the file (the default has been fully replaced). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- user_override_replaces_default --nocapture`
- **Given** a `--config user.json` setting `bed_temperature_initial_layer_single = 65` and `nozzle_temperature_initial_layer = 220` (and the default `machine_start_gcode`), **when** `out.gcode` is scanned, **then** the start block contains the lines `M190 S65`, `M109 S220`, and `PRINT_START EXTRUDER=220 BED=65` (each exactly once) and contains no `S60`, `S215`, `EXTRUDER=215`, or `BED=60` substring. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- substitution_uses_overridden_temp_values --nocapture`
- **Given** a `--config user.json` setting `machine_end_gcode = ""` (empty string override), **when** `out.gcode` is scanned, **then** `PRINT_END` is absent from the file AND the byte range between the last `G1` line's terminating `\n` and `CONFIG_BLOCK_START` contains zero non-whitespace characters (no phantom block, no stray blank lines beyond one optional separator). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- empty_end_gcode_emits_no_block --nocapture`
- **Given** the `machine-gcode-emit` core module manifest is loaded via the existing host manifest-discovery path, **when** the four keys are looked up, **then** each is registered with type (`string`, `string`, `int`, `int`) respectively and default value matching the Goal section (default start template literal, `"PRINT_END"`, `60`, `215`). If the manifest-discovery API is not directly queryable from a test, the assertion falls back to verifying that all four `; machine_start_gcode = ...`, `; machine_end_gcode = PRINT_END`, `; bed_temperature_initial_layer_single = 60`, `; nozzle_temperature_initial_layer = 215` lines appear in the CONFIG_BLOCK of `out.gcode` produced from a default invocation. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- module_manifest_registers_four_keys_with_expected_types_and_defaults --nocapture`
- **Given** a default slicing run, **when** `CONFIG_BLOCK_START..CONFIG_BLOCK_END` is parsed, **then** each of `; machine_start_gcode = ...`, `; machine_end_gcode = PRINT_END`, `; bed_temperature_initial_layer_single = 60`, and `; nozzle_temperature_initial_layer = 215` appears exactly once (the four new keys flow through packet 55's CONFIG_BLOCK emission without further wiring). Multi-line `machine_start_gcode` value MAY be emitted on a single comment line with `\n` literalized as `\\n` OR via the existing packet-55 multi-line-value convention — the test asserts exact-key-presence and exact-value-equality against the declared default after un-escaping. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- new_keys_appear_in_config_block --nocapture`

## Negative Test Cases

- **Given** a `--config user.json` setting `machine_start_gcode = "TEMP [no_such_key] DONE"`, **when** the slicer is run, **then** the produced output contains the literal `TEMP [no_such_key] DONE`. (Renamed from `unknown_placeholder_passes_through_with_warning`. Substring-presence assertion is preserved verbatim; the prior WARN-log capture clause has been DROPPED because substitution now runs in a WASM guest and cross-component log forwarding is a separate future packet — see Out of Scope.) | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- unknown_placeholder_passes_through_verbatim --nocapture`
- **Given** a `--config user.json` setting `machine_start_gcode = "PREFIX [unclosed SUFFIX"` (no closing `]` on the same line), **when** the slicer is run, **then** the produced output contains the literal `PREFIX [unclosed SUFFIX` exactly as written, the substitution does not panic and does not infinite-loop (an unclosed bracket is treated as literal text). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- unclosed_bracket_treated_as_literal --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** scanned, **then** the literal substring `M190` does NOT appear inside the byte range `HEADER_BLOCK_START..HEADER_BLOCK_END`, does NOT appear after any `G1` line with `E` token (i.e., not after the first extrusion), and does NOT appear inside the byte range `CONFIG_BLOCK_START..CONFIG_BLOCK_END`. This negative case catches regression of any future serializer change that emits `M190` in the wrong band. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_block_not_inside_other_blocks --nocapture`

## Verification

- `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` — dispatch as FACT pass/fail; SNIPPETS (≤ 20 lines) on first-failing-assertion.
- `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — packet 55 regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — packet 52/54 regression.
- `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` — postpass pipeline regression.
- `./modules/core-modules/build-core-modules.sh --check` — guest WASM freshness (CLAUDE.md Guest WASM Staleness).
- `./test-guests/build-test-guests.sh --check` — test-guest freshness (CLAUDE.md Guest WASM Staleness; triggered by the WIT change).
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage, serializer role; delegate a SUMMARY (file is long).
- `docs/02_ir_schemas.md` — `ConfigView`, `ResolvedConfig`, `ConfigValue` enum; range-read only the `ConfigValue` enum section and the `GCodeIR` struct section.
- `docs/03_wit_and_manifest.md` — module-manifest schema, key validation; load directly only the `[config.schema.<key>]` section.
- `docs/05_module_sdk.md` — `#[slicer_module]` macro and `FinalizationModule` trait; range-read only the smallest example with a non-no-op body.
- `docs/07_implementation_status.md` — DELEGATE every read. Step 1 of this packet appends TASK-193 / TASK-193a / TASK-193b rows via a worker.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — no deviation expected; if review requires, file the end-block-before-CONFIG_BLOCK ordering difference here.

For each doc above: if > 300 lines, delegate. Default rule wins.

## OrcaSlicer Reference Obligations

All reads delegated; never load OrcaSlicer source into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` (machine_start/end_gcode field declarations) — FACT, ≤ 6 lines: confirm field names are exactly `machine_end_gcode` and `machine_start_gcode` (snake_case, no `print_` prefix).
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (stock-default `add`/`set_default_value`) — FACT, ≤ 8 lines: confirm OrcaSlicer's stock defaults; we DELIBERATELY do not adopt them.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3181` (placeholder substitution) and `:3200` (start gcode write) and `:3258` (preamble after start gcode) — FACT, ≤ 12 lines: confirm ordering is `machine_start_gcode` THEN preamble. Our serializer must match: resolved start string inserted BEFORE the existing packet-54 M82/M83 preamble.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3544` — FACT, ≤ 6 lines: confirm end gcode is the final block before file close in OrcaSlicer; in our flow it comes BEFORE THUMBNAIL/CONFIG_BLOCK (intentional difference because our CONFIG_BLOCK is structurally a footer wrapper).
- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (`apply_config()`) — FACT, ≤ 10 lines: confirm placeholder substitution sources config values from the same symbol table that holds `bed_temperature_initial_layer_single` etc. Our minimal helper mirrors this by accepting a `HashMap<String, ConfigValue>` view of the effective config. Do NOT read the full `process()` grammar — out of scope.

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

- `crates/slicer-host/src/gcode_emit.rs` is **> 1100 lines** (above the 600-line direct-read budget). Range-read `:667-:740` (HEADER + width comments; `serialize_header_block` at `:667`, `serialize_width_comments` at `:712`), `:928-:1020` (CONFIG_BLOCK at `:928` + `ThumbnailAwareSerializer` wrapper at `:973`), and `:1021-:1166` (`DefaultGCodeSerializer::serialize_gcode` body at `:1021` + M83 at `:1067` + M82 at `:1069`). Never load the full file.
- `crates/slicer-host/src/dispatch.rs` is **> 3000 lines**. Range-read `:1070-:1100` (`dispatch_finalization_call` entry at `:1079`) and `:2885-:2980` (apply-site loop). Never load the full file.
- `crates/slicer-host/src/wit_host.rs` — range-read `:830-:895` (`FinalizationBuilderPush` enum at `:837`). Never load the full file.
- `crates/slicer-host/src/pipeline.rs:435-449` — the `effective_config: HashMap<String, ConfigValue>` build site. Range-read only.
- `crates/slicer-ir/src/slice_ir.rs` is **> 1600 lines**. Range-read `:550-:580` (`ConfigValue` enum at `:557`) and `:1779-:1799` (`GCodeIR` struct at `:1781`). Never load the full file.
- `crates/slicer-sdk/src/traits.rs` — range-read `:700-:730` (`FinalizationOutputBuilder` struct + impl) and `:1196-:1230` (`FinalizationModule` trait).
- `wit/world-finalization.wit:55-110` — full read OK (file is short).
- `modules/core-modules/part-cooling/{part-cooling.toml, Cargo.toml, src/lib.rs}` — full reads OK (each is small). These are the `FinalizationModule` shape precedent the implementer copies.
- `OrcaSlicerDocumented/` MUST be delegated. The five FACT dispatches enumerated above are the only OrcaSlicer evidence this packet needs.
- `docs/07_implementation_status.md` is > 500 lines. NEVER load it directly.

Sub-agent return formats:

- OrcaSlicer FACTs (5 dispatches above): ≤ 12 lines each, no code blocks > 4 lines.
- `cargo test`: FACT pass/fail; SNIPPETS (≤ 20 lines) on first-failing-assertion.
- WIT/dispatch/SDK alignment dispatches (Step 3): LOCATIONS + ≤ 10 lines each.
- Module-manifest registration completeness check (end of Step 4): LOCATIONS list of every `[config.schema.<key>]` block in `machine-gcode-emit.toml` (expected count = 4), ≤ 8 entries.
- `serialize_gcode()` insertion-point lookup (Step 5): SNIPPETS (≤ 30 lines) of the two byte ranges where start/end blocks are inserted.

### Test Fixture Convention

- This packet's tests reuse the same small `.stl` fixture used by `gcode_emit_tdd.rs` and `gcode_header_thumbnail_config_blocks_tdd.rs` (resolve via `concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/<fixture>.stl")` — confirm the exact filename via a single LOCATIONS dispatch against the predecessor test files in Step 2). NO new STL fixture is created.
- The `--config user.json` overrides used in ACs are materialized inline at test runtime via `std::env::temp_dir()` + `serde_json::to_writer`. NO new committed config fixtures.

### Test Discipline Reminder

Per `CLAUDE.md` / Test Discipline: `cargo test --workspace` is FORBIDDEN as a per-AC verification command in this packet. Every AC above uses the targeted `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- <test_name> --nocapture` form. The packet-level workspace gate appears only at the closure ceremony in `implementation-plan.md` Step 7 and is dispatched to a sub-agent that returns FACT pass/fail.

Aggregate context cost: M. No step is L. If implementation reveals that the start-block or end-block insertion point in `serialize_gcode()` is not a clean injection (e.g., requires refactoring the preamble emission path or the wrapper's seam), surface as a packet-local risk in `design.md` Open Questions and split that work into a follow-up packet rather than expanding this packet's scope.
