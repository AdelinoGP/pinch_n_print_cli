# Requirements: 59_machine-start-end-gcode-emission

## Packet Metadata

- Grouped task IDs:
  - `TASK-193`  — emit `machine_start_gcode` / `machine_end_gcode` with minimal `[key]` substitution at correct positions in serialized output.
  - `TASK-193a` — register four host-level config keys in `FullConfigSchema::default()`: `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer`.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M` (no single step is `L`)

## Problem Statement

The slicer currently produces a G-code file that begins with HEADER_BLOCK + extrusion-width comments and a M82/M83 preamble, then jumps straight to the first layer's commands (G0/G1/G92/M104/M106 etc.). It ends with the last layer's commands followed by the THUMBNAIL_BLOCK + CONFIG_BLOCK footer (per packet 55). **No printer start sequence is emitted** — no homing (`G28`), no absolute-positioning toggle (`G90`), no extruder-mode toggle distinct from the slicer-internal preamble, no hotend/bed temperature set or wait (`M104/M109`, `M140/M190`), no Klipper-style `PRINT_START` macro invocation. **No printer finish sequence is emitted** either — no end macro, no shutdown, no part-cooling-off + heater-off + motors-off sequence.

As a result, the produced `.gcode` cannot be sent directly to a printer: the printer would attempt to extrude with a cold hotend and on an un-homed bed. The user has to manually prepend boilerplate every print, which defeats one of the core ergonomics of a host slicer.

This is also the first packet to introduce **placeholder substitution** in serialized G-code. OrcaSlicer's full `PlaceholderParser` (`OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp`) is a Boost.Spirit Qi grammar of ~2400 lines supporting arithmetic, conditionals, loops, and builtin functions. Implementing parity in one packet is out of proportion to the immediate need. This packet implements the **smallest viable subset**: `[snake_case_key]` literal substitution against the host config schema. Three follow-up packets (arithmetic, conditionals/loops, builtins) are tracked but **not** included here.

The four config keys (`machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer`) do not currently exist in `FullConfigSchema::default()` (`crates/slicer-host/src/config_schema.rs:128-393`). They must be added in this same packet — the default `machine_start_gcode` template references the two temperature keys via `[bed_temperature_initial_layer_single]` and `[nozzle_temperature_initial_layer]`, and without those keys the default substitution would warn-and-passthrough literal placeholder text instead of resolving to printable values.

This packet does NOT reopen or supersede any prior packet. It inserts new emission stages BETWEEN existing serializer sites established by packets 54 (M82/M83 preamble), 55 (HEADER_BLOCK + width comments + ThumbnailAwareSerializer / CONFIG_BLOCK). Predecessor packets are correct as shipped; this packet adds two new sites without modifying them.

## In Scope

- Register `machine_start_gcode` (String) in `FullConfigSchema::default()` with the default literal:
  ```
  M190 S[bed_temperature_initial_layer_single]
  M109 S[nozzle_temperature_initial_layer]
  PRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]
  ```
- Register `machine_end_gcode` (String) in `FullConfigSchema::default()` with the default literal `PRINT_END`.
- Register `bed_temperature_initial_layer_single` (Int) with default `60`, range `0..=120`.
- Register `nozzle_temperature_initial_layer` (Int) with default `215`, range `0..=300`.
- Add a private `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper in `crates/slicer-host/src/gcode_emit.rs` that:
  - Replaces every `[snake_case_key]` token whose key resolves in `lookup` with the stringified `ConfigValue`.
  - Leaves unknown tokens as literal text and emits one `log::warn!("unknown placeholder: {key}")` per unknown key (deduplicated per template scan). Uses the workspace's existing `log = "0.4"` dep (declared at `crates/slicer-host/Cargo.toml:20`); does NOT pull in a new dependency.
  - Treats `[` not followed by a closing `]` on the same line as literal text (no panic, no infinite loop).
  - Performs a single pass — substituted values are NOT re-scanned for further placeholders.
- Wire the substituted `machine_start_gcode` block into `DefaultGCodeSerializer::serialize_gcode()` AFTER the existing HEADER_BLOCK + extrusion-width comments emission (currently at `crates/slicer-host/src/gcode_emit.rs:626-740`; `serialize_header_block` at `:626`, `serialize_width_comments` at `:671`) and BEFORE the existing M82/M83 preamble emission (M83 at `:1026`, M82 at `:1028`). The block is followed by one trailing newline. If the substituted block is empty or whitespace-only, nothing is emitted.
- Wire the substituted `machine_end_gcode` block into the same serializer AFTER the last layer's commands and BEFORE the THUMBNAIL/CONFIG footer emitted by `ThumbnailAwareSerializer` (CONFIG-block emitter `serialize_config_block` at `crates/slicer-host/src/gcode_emit.rs:887`; wrapper `impl ThumbnailAwareSerializer` at `:932`; inner-serializer call at `:950`; THUMBNAIL/CONFIG append at `:953-974`). Same empty/whitespace rule applies.
- Add new TDD test file `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` covering the 9 positive ACs and 4 negative ACs from `packet.spec.md`.
- Add TASK-193 and TASK-193a entries to `docs/07_implementation_status.md` via worker dispatch.

## Out of Scope

- Arithmetic in placeholders (`[bed*2]`, `[x+5]`). Tracked for a separate future TASK-### packet.
- Conditionals (`{if cond}...{elsif}...{else}...{endif}`), loops (`{for x in ...}{endfor}`), builtin functions (`min`, `max`, `round`, ...). Tracked for separate future packets.
- `{var}` brace-syntax placeholders. Only `[key]` square-bracket syntax is supported in this packet.
- Per-extruder / per-region / per-object placeholders (e.g., `[nozzle_temperature_initial_layer_0]`, `[filament_diameter_0]`).
- Adopting OrcaSlicer's stock `machine_start_gcode` / `machine_end_gcode` defaults (`G28 ; home all axes\nG1 Z5 F5000` / `M104 S0\nG28 X0\nM84`). We intentionally use the user-specified Klipper PRINT_START / PRINT_END macros instead — recorded as an intentional deviation under the OrcaSlicer Reference Obligations section.
- OrcaSlicer's custom-gcode placeholder validator (`OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:9700-9701`).
- Adding ANY further temperature / fan / filament / printer-profile keys beyond the four enumerated.
- Multi-extruder support (`M104 T1 S...`, `T0/T1` switches inside start gcode).
- Real macro semantics for `PRINT_START` / `PRINT_END` — those live in printer firmware / Klipper config, not the slicer. The slicer's contract is "emit the literal macro invocation"; what the printer does with it is the printer's concern.
- Modifying `GCodeIR`, `LayerCollectionIR`, `ConfigView`, or any IR contract in `docs/02_ir_schemas.md`.
- Modifying `ThumbnailAwareSerializer` ordering or the CONFIG_BLOCK contents beyond the four new keys naturally flowing in via the effective `ConfigView`.
- Modifying `M82`/`M83`/`G90`/`G21` preamble emission established by packet 54.

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage and serializer role. Likely > 300 lines; delegate a SUMMARY.
- `docs/02_ir_schemas.md` — `ConfigView`, `ResolvedConfig`, `ConfigValue` enum. > 300 lines; range-read `:433-:444` (`ConfigValue` enum) and `:618-:730` (`ResolvedConfig`) directly; delegate everything else.
- `docs/03_wit_and_manifest.md` — host-level vs module-level config schema, key validation rules. Load directly only the schema-validation section.
- `docs/07_implementation_status.md` — > 500 lines. DELEGATE every read AND every edit. Step 1 of this packet adds TASK-193 / TASK-193a rows via a worker; the implementer must never load the full backlog file.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — register the two intentional deviations from OrcaSlicer (stock defaults, end-gcode position relative to CONFIG_BLOCK) if review requires it.

Default rule: delegate any doc > 300 lines. All ranged reads above stay within the budget.

## OrcaSlicer Reference Obligations

All reads delegated; never load OrcaSlicer source into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp:1243` (`machine_end_gcode`) and `:1288` (`machine_start_gcode`) — confirms our key names match OrcaSlicer parity. **Behavior borrowed: snake_case key names.** **Not borrowed: nothing — these are pure declarations.**
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:5599-5606` (start gcode `add` + `set_default_value`) and `:1927-1934` (end gcode `add` + `set_default_value`) — read OrcaSlicer's stock defaults to confirm we are deliberately diverging. The cited `:5599` / `:1927` lines are the `def = this->add("machine_{start,end}_gcode", coString);` declarations; the actual default-value `set_default_value(new ConfigOptionString(...))` calls live at `:5606` and `:1934`. **Behavior NOT borrowed:** stock OrcaSlicer defaults (`"G28 ; home all axes\nG1 Z5 F5000 ; lift nozzle\n"` / `"M104 S0 ; turn off temperature\nG28 X0  ; home X axis\nM84     ; disable motors\n"`). Our defaults instead delegate to Klipper PRINT_START / PRINT_END macros per user specification. **Intentional deviation — document in this packet's design.md and, if review requires, register in `docs/DEVIATION_LOG.md`.**
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3181` (placeholder substitution), `:3200` (start gcode write), `:3258` (preamble after start gcode) — **Behavior borrowed:** ordering is `machine_start_gcode` THEN preamble. Our serializer matches: substituted start block is inserted BEFORE the existing packet-54 M82/M83 preamble. **Not borrowed:** full placeholder grammar (out of scope for this packet).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3544` — OrcaSlicer's end-gcode write site. **Intentional deviation:** OrcaSlicer emits end gcode AFTER its CONFIG_BLOCK; we emit it BEFORE the `ThumbnailAwareSerializer`-owned THUMBNAIL_BLOCK / CONFIG_BLOCK because our CONFIG_BLOCK is structurally a footer wrapper, not the final printed block. Downstream printer parsers ignore comments after the last printable command, so this difference is transparent to the printer; document in design.md.
- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp:164` (`apply_config()`) — **Behavior borrowed:** placeholder values are sourced from the same config symbol table that holds the temperature keys. Our minimal helper mirrors this by accepting a `HashMap<String, ConfigValue>` view of the effective config. **Not borrowed:** the full `process()` grammar implementation at `:2433` (out of scope; deferred to future packets).

## Acceptance Summary

- **Positive cases (9):** see `packet.spec.md` Acceptance Criteria section. Cover: default-template substitution end-to-end, default `PRINT_END` emission, start-block position between HEADER_BLOCK_END and first extrusion `G1`, end-block position between last `G1` and CONFIG_BLOCK_START, full user override of start template, partial user override of just temperature values, empty `machine_end_gcode` ⇒ no block, schema registration of all four keys with correct types and defaults, four new keys appearing in CONFIG_BLOCK.
- **Negative cases (4):** see `packet.spec.md` Negative Test Cases section. Cover: unknown placeholder passthrough + WARN log, unclosed-bracket literal passthrough (no panic, no infinite loop, no warn), temperature value out of registered range → non-zero exit + clear diagnostic + no output file, start-block content does NOT appear inside HEADER_BLOCK / inside CONFIG_BLOCK / after first `G1` (regression sentry).
- **Measurable outcomes:**
  - Exactly one `M190 S60` line, exactly one `M109 S215` line, exactly one `PRINT_START EXTRUDER=215 BED=60` line in default output.
  - Exactly one `PRINT_END` line in default output.
  - Four new config keys present in `FullConfigSchema::default()` registry with types `String`, `String`, `Int`, `Int` and defaults matching the Goal section of `packet.spec.md`.
  - Four new config keys present in the CONFIG_BLOCK of every default-config slicing run.
  - `log::warn!` record emitted exactly once per unique unknown placeholder key per substitution call (uses the existing `log = "0.4"` dep in `crates/slicer-host/Cargo.toml:20`).
  - Non-zero exit + stderr containing `nozzle_temperature_initial_layer` + `300` (or `range`) on temperature out-of-range.
- **Cross-packet impact:**
  - Unblocks future "OrcaSlicer placeholder-parser arithmetic" packet (extends the helper added here).
  - Unblocks future "OrcaSlicer placeholder-parser control flow" packet.
  - Unblocks future "printer-profile import" packet.
  - Does NOT block, supersede, or modify any landed packet. Packets 54 and 55 emission sites are preserved verbatim.

## Verification Commands

- `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` — primary AC dispatch; FACT pass/fail, SNIPPETS (≤ 20 lines) on first-failing-assertion.
- `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — packet-55 regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — packet-52/54 regression.
- `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` — postpass pipeline regression.
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`

All verification commands listed here are delegation-friendly (small, parseable output) so the implementer and reviewer can dispatch them to a sub-agent and consume only a FACT or SNIPPETS return.

**Test discipline reminder (CLAUDE.md):** `cargo test --workspace` is forbidden as a per-AC verification command and is NOT listed here. It may appear only at the packet's final acceptance ceremony in `implementation-plan.md` Step 6, dispatched to a sub-agent that returns FACT pass/fail.

## Step Completion Expectations

For each step in `implementation-plan.md`:

- **Step 1 — Docs/07 task entries**
  - Precondition: TASK-193 / TASK-193a not present in docs/07.
  - Postcondition: both rows appended in the correct phase / status column.
  - Falsifying check: worker dispatch `grep -n "TASK-193" docs/07_implementation_status.md` returns 2 hits, each with status `[ ]` (queued).
  - Files allowed to read: `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md:3-6` (for TASK-184 / TASK-185 row formatting precedent).
  - Files allowed to edit (≤ 3): `docs/07_implementation_status.md` (via worker dispatch only — implementer never loads the full file).
  - Expected sub-agent dispatches: 1 LOCATIONS dispatch (find the right row insertion point near other in-progress G-code tasks); 1 edit dispatch to append the two rows; 1 FACT dispatch to verify the grep result.
  - Step context cost: `S`.

- **Step 2 — TDD test file with all 13 failing assertions**
  - Precondition: no `machine_start_end_gcode_emission_tdd.rs` test file.
  - Postcondition: test file present, all 13 tests compile-pass and assertion-fail (red).
  - Falsifying check: `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -50` returns exit non-zero AND every test is marked `FAILED` (not `ignored`, not `pass`).
  - Files allowed to read: `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full — packet 55 fixture pattern), `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` (line ranges for layer fixture pattern).
  - Files allowed to edit (≤ 3): `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` (new).
  - Expected sub-agent dispatches: 1 LOCATIONS dispatch to find the small STL fixture path used by sibling tests; 1 FACT dispatch to confirm `log = "0.4"` is present in `crates/slicer-host/Cargo.toml` and that no other slicer-host test installs a conflicting global logger via `log::set_boxed_logger` / `log::set_logger`. The test uses a custom `log::Log` impl (~30 LOC) to capture WARN records into a `Mutex<Vec<String>>`; no new dependency is added.
  - Step context cost: `M`.

- **Step 3 — Register four config keys**
  - Precondition: keys absent from `FullConfigSchema::default()`.
  - Postcondition: all four registered; AC `schema_registers_four_keys_with_expected_types_and_defaults` turns green.
  - Falsifying check: targeted `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- schema_registers_four_keys_with_expected_types_and_defaults --nocapture` passes.
  - Files allowed to read: `crates/slicer-host/src/config_schema.rs` — **1044 lines, ABOVE the 600-line direct-read budget; range-read only at `:367-:387` (`thumbnail_path` String precedent) and `:191-:212` (`fan_speed_min` Int+range precedent)**. NEVER load the full file.
  - Files allowed to edit (≤ 3): `crates/slicer-host/src/config_schema.rs`.
  - Expected sub-agent dispatches: 1 SNIPPETS dispatch to capture the existing `String` registration pattern (`thumbnail_path` precedent) and an `Int` registration pattern.
  - Step context cost: `S`.

- **Step 4 — Implement placeholder substitution + wire start/end blocks**
  - Precondition: Step 3 complete; remaining 12 ACs still red.
  - Postcondition: `substitute_placeholders()` helper added; start block emitted in correct position; end block emitted in correct position; empty/whitespace block emits nothing; all 13 ACs green; regression test suites green.
  - Falsifying check: targeted `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` passes; `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` passes; `cargo test -p slicer-host --test gcode_emit_tdd` passes.
  - Files allowed to read: `crates/slicer-host/src/gcode_emit.rs:626-740` (HEADER + width + thumbnail), `:887-:977` (CONFIG_BLOCK + `ThumbnailAwareSerializer`), `:979-:1166` (serialize_gcode body + preamble).
  - Files allowed to edit (≤ 3): `crates/slicer-host/src/gcode_emit.rs`.
  - Expected sub-agent dispatches: 1 SNIPPETS dispatch for the existing `serialize_header_block` insertion point (start-block site); 1 SNIPPETS dispatch for the existing last-layer-emit-then-footer site (end-block site); 1 FACT dispatch per targeted-test run (3 total).
  - Step context cost: `M`.

- **Step 5 — Regression sweep**
  - Precondition: all 13 ACs green from Step 4.
  - Postcondition: regression test suites green; `cargo check --workspace` clean; `cargo clippy --workspace -- -D warnings` clean.
  - Falsifying check: each regression command returns FACT pass.
  - Files allowed to read: none direct.
  - Files allowed to edit (≤ 3): zero or one (for minor clippy fixes only).
  - Expected sub-agent dispatches: 4 FACT dispatches (one per `cargo test` regression, one for `cargo check`, one for `cargo clippy`).
  - Step context cost: `S`.

- **Step 6 — Packet completion gate**
  - Precondition: Step 5 clean.
  - Postcondition: every pipe-suffixed AC command re-dispatched and green; `cargo test --workspace` dispatched as the final closure ceremony per CLAUDE.md test discipline; docs/07 rows updated to `[x]`; `packet.spec.md` ready to flip from `draft` to `implemented`.
  - Falsifying check: any re-dispatched AC returns fail.
  - Files allowed to read: none direct.
  - Files allowed to edit (≤ 3): `docs/07_implementation_status.md` (via worker), `.ralph/specs/59_machine-start-end-gcode-emission/packet.spec.md` (status flip).
  - Expected sub-agent dispatches: 13 FACT dispatches (one per AC); 1 FACT dispatch for `cargo test --workspace`; 1 worker dispatch to update docs/07.
  - Step context cost: `S`.

Aggregate per-step context cost: 5×S + 1×M = `M` total. No single step is `L`.

## Context Discipline Notes

Context-budget hazards specific to this packet:

- **Large files in the read-only path that MUST be ranged or delegated:**
  - `crates/slicer-host/src/gcode_emit.rs` (> 1100 lines) — only `:626-:740`, `:887-:977`, and `:979-:1166` are needed; never load full file.
  - `crates/slicer-ir/src/slice_ir.rs` (> 1600 lines) — only `:433-:444` (`ConfigValue`) and `:618-:730` (`ResolvedConfig`); never load full file.
  - `docs/07_implementation_status.md` (> 500 lines) — never load; all reads / edits via worker.
  - `docs/02_ir_schemas.md` (> 300 lines) — range-read only.
- **OrcaSlicer trees the implementer must NOT load directly:**
  - All of `OrcaSlicerDocumented/` is delegated. The five FACT dispatches enumerated in `packet.spec.md` are the only evidence this packet needs. In particular: NEVER read `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` directly (~2400 lines of Boost.Spirit grammar; will trash context budget and is out of scope anyway).
- **Likely temptation reads (skip):**
  - Reading other core-module manifests under `modules/core-modules/*/*.toml` "to see how a config key is registered" — those are MODULE-level, while this packet's keys are HOST-level. The precedent to follow is `thumbnail_path` in `crates/slicer-host/src/config_schema.rs`, not any module manifest.
  - Reading `crates/slicer-cli/src/cmd_run.rs` end-to-end "to see how config flows" — only the `--thumbnail` precedent surface (a few lines near where `config_source: HashMap<String, ConfigValue>` is constructed) is relevant; delegate a SNIPPETS dispatch if needed.
  - Reading other packets' `design.md` "to compare patterns" — packets 54 / 55 are the only relevant precedents and are quoted here verbatim where needed.
- **Sub-agent return-format hints for the heaviest dispatches:**
  - OrcaSlicer FACT dispatches (5 total): ≤ 12 lines each; no code blocks > 4 lines; cite file:line and quote the relevant identifier or string literal only.
  - `cargo test` dispatches: FACT pass/fail; SNIPPETS (≤ 20 lines) on failure with the first failing assertion + the test name + ≤ 15 lines of context around the assertion.
  - `cargo check` / `cargo clippy` dispatches: FACT pass/fail; SNIPPETS (≤ 30 lines) on failure with the first error / warning only.
  - `docs/07_implementation_status.md` worker dispatches: never return the full file; return the row insertion point as LOCATIONS (file:line + the existing adjacent row's text), and after the edit return FACT (`grep -n "TASK-193" hits = 2`).
