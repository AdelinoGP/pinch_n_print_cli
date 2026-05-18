# Requirements: 59_machine-start-end-gcode-emission

## Packet Metadata

- Grouped task IDs:
  - `TASK-193`  — emit `machine_start_gcode` / `machine_end_gcode` at correct serializer byte offsets via two new `FinalizationBuilderPush` variants.
  - `TASK-193a` — create `modules/core-modules/machine-gcode-emit/` owning four config keys and running real `[key]` substitution against `ResolvedConfig` inside `run_finalization`, pushing resolved strings via the new variants.
  - `TASK-193b` — extend WIT (`finalization-output-builder`) + SDK (`FinalizationBuilderPush` + `FinalizationOutputBuilder` impl) + IR (`GCodeIR`) + dispatch routing with the print-boundary variants.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M` (no single step is `L`)

## Problem Statement

The slicer currently produces a G-code file that begins with HEADER_BLOCK + extrusion-width comments and a M82/M83 preamble, then jumps straight to the first layer's commands (G0/G1/G92/M104/M106 etc.). It ends with the last layer's commands followed by the THUMBNAIL_BLOCK + CONFIG_BLOCK footer (per packet 55). **No printer start sequence is emitted** — no homing (`G28`), no absolute-positioning toggle (`G90`), no extruder-mode toggle distinct from the slicer-internal preamble, no hotend/bed temperature set or wait (`M104/M109`, `M140/M190`), no Klipper-style `PRINT_START` macro invocation. **No printer finish sequence is emitted** either — no end macro, no shutdown, no part-cooling-off + heater-off + motors-off sequence.

As a result, the produced `.gcode` cannot be sent directly to a printer: the printer would attempt to extrude with a cold hotend and on an un-homed bed. The user has to manually prepend boilerplate every print, which defeats one of the core ergonomics of a host slicer.

This is also the first packet to introduce **placeholder substitution** in serialized G-code. OrcaSlicer's full `PlaceholderParser` is a Boost.Spirit Qi grammar of ~2400 lines supporting arithmetic, conditionals, loops, and builtin functions. Implementing parity in one packet is out of proportion to the immediate need. This packet implements the **smallest viable subset**: `[snake_case_key]` literal substitution against the effective `ConfigView`, performed INSIDE the new core module's `run_finalization` body. Three follow-up packets (arithmetic, conditionals/loops, builtins) are tracked but **not** included here.

The four config keys (`machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer`) do not currently exist anywhere in the workspace. They are declared in this same packet by the new module's manifest TOML — the default `machine_start_gcode` template references the two temperature keys via `[bed_temperature_initial_layer_single]` and `[nozzle_temperature_initial_layer]`, and without those keys the default substitution would pass through literal placeholder text instead of resolving to printable values.

The canonical schema-declaration site for config keys consumed by the runtime is a core-module manifest TOML (`[config.schema.<key>]` blocks; precedent: `modules/core-modules/part-cooling/part-cooling.toml`'s `fan_speed_min` etc., and `modules/core-modules/seam-placer/seam-placer.toml`'s `seam_mode` for the `string` type). This packet introduces a new core module `modules/core-modules/machine-gcode-emit/` whose manifest declares the four keys AND whose `run_finalization` performs the real substitution work and ships the resolved strings to the host serializer through two NEW `FinalizationBuilderPush` variants (`PrintStartGcode(String)` / `PrintEndGcode(String)`), routed by the dispatch layer into two NEW `Option<String>` fields on `GCodeIR` (`print_start_gcode` / `print_end_gcode`). The host serializer (`crates/slicer-host/src/gcode_emit.rs`) reads those fields and inserts the resolved strings at the contractual byte positions. **The serializer contains NO substitution logic.**

This packet does NOT reopen or supersede any prior packet. It inserts new emission stages BETWEEN existing serializer sites established by packets 54 (M82/M83 preamble), 55 (HEADER_BLOCK + width comments + ThumbnailAwareSerializer / CONFIG_BLOCK). Predecessor packets are correct as shipped; this packet adds two new sites + an additive WIT/SDK/IR extension.

## In Scope

- Create `modules/core-modules/machine-gcode-emit/` with three files:
  - `machine-gcode-emit.toml` — manifest declaring four `[config.schema.<key>]` blocks. Header keys (`id`, `version`, `display-name`, `description`, `author`, `license`, `wit-world`) mirror `modules/core-modules/part-cooling/part-cooling.toml`. `[stage]` block uses `id = "PostPass::LayerFinalization"`. The four `[config.schema.<key>]` blocks:
    - `machine_start_gcode` (`type = "string"`) default = multi-line literal:
      ```
      M190 S[bed_temperature_initial_layer_single]
      M109 S[nozzle_temperature_initial_layer]
      PRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]
      ```
      (emitted as a TOML triple-quoted `"""..."""` literal so newlines are preserved verbatim).
    - `machine_end_gcode` (`type = "string"`) default = `"PRINT_END"`.
    - `bed_temperature_initial_layer_single` (`type = "int"`) default = `60`, `min = 0`, `max = 120` (declarative metadata only).
    - `nozzle_temperature_initial_layer` (`type = "int"`) default = `215`, `min = 0`, `max = 300` (declarative metadata only).
    The exact `[module]` header keys must match `part-cooling.toml`'s shape verbatim — confirm via direct read of `part-cooling.toml` before writing the manifest.
  - `Cargo.toml` — mirrors `modules/core-modules/part-cooling/Cargo.toml` (depends on `slicer-sdk`, `slicer-schema`, `slicer-ir`; `wit-bindgen` under `[target.'cfg(target_arch = "wasm32")'.dependencies]`).
  - `src/lib.rs` — implements `FinalizationModule` via `#[slicer_module]` with a REAL `run_finalization` body. The body:
    1. Reads `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer` from the `&ConfigView` argument.
    2. Builds a `HashMap<String, ConfigValue>` lookup (or equivalent view) containing those four keys.
    3. Calls a private free `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper (≤ 60 LOC, scoped to this module file) on both templates.
    4. Calls `output.push_print_start_gcode(resolved_start)` and `output.push_print_end_gcode(resolved_end)`.
    5. Returns `Ok(())`.
- Extend WIT contract (`wit/world-finalization.wit:62-104`, the `finalization-output-builder` resource). Add two new methods (additive — no existing method removed or changed):
  - `push-print-start-gcode: func(text: string) -> result<_, string>;`
  - `push-print-end-gcode: func(text: string) -> result<_, string>;`
- Extend the SDK trait surface (`crates/slicer-sdk/src/traits.rs`):
  - On the `FinalizationOutputBuilder` impl at `:717`, add `push_print_start_gcode(&mut self, text: String)` and `push_print_end_gcode(&mut self, text: String)`. Internal storage may be two `Option<String>` fields on the struct (mirroring the `entity_pushes` / `annotations` field pattern at `:705-:714`).
  - The `FinalizationModule` trait at `:1196` is UNCHANGED in signature.
- Extend the host enum used by the WIT/host bridge (`crates/slicer-host/src/wit_host.rs:837`, currently 6 variants). Add two variants — `PrintStartGcode(String)` and `PrintEndGcode(String)` — as variants 7 and 8. The host's bindgen-generated `finalization-output-builder` resource bridge inside `wit_host.rs` (the same module that exposes `finalization::Push*` bindings) must mirror the WIT addition: see the existing `data.pushes.push(FinalizationBuilderPush::...)` sites near `:4903-:5010` in `wit_host.rs` and add two new push sites for the new resource methods.
- Extend dispatch routing (`crates/slicer-host/src/dispatch.rs`). The apply-site loop at `:2892-:2978` inside `dispatch_finalization_call` (`:1079`) currently has 6 match arms. Add 2 new match arms that deposit the resolved strings into NEW `GCodeIR` fields. The deposit site is the same site that produces the `GCodeIR` consumed by the serializer (Step 5 of `implementation-plan.md` confirms the exact byte path via a SNIPPETS dispatch). The two new fields:
- Extend the IR (`crates/slicer-ir/src/slice_ir.rs`). The `GCodeIR` struct at `:1781` currently has three fields (`schema_version`, `commands`, `metadata`). Add two additive `Option<String>` fields:
  - `pub print_start_gcode: Option<String>,`
  - `pub print_end_gcode: Option<String>,`
  with `Default::default()` initializing both to `None` (the existing `Default for GCodeIR` impl at `:1790` is extended accordingly). No existing field changed; no variant removed.
- Wire the host serializer (`crates/slicer-host/src/gcode_emit.rs`). The serializer has NO `substitute_placeholders` helper — substitution lives in the WASM guest. The serializer READS `gcode_ir.print_start_gcode` and `gcode_ir.print_end_gcode` inside `DefaultGCodeSerializer::serialize_gcode()` (body at `:1021`) and INSERTS them at the contractual byte positions:
  - Start string AFTER the existing `serialize_header_block` emission (`:667`) and `serialize_width_comments` emission (`:712`), BEFORE the existing M82/M83 preamble emission (M83 at `:1067`, M82 at `:1069`). Followed by one trailing `\n` if non-empty.
  - End string AFTER the last layer's commands, BEFORE the `ThumbnailAwareSerializer` wrapper (`:973`) appends THUMBNAIL/CONFIG_BLOCK. The inner serializer appends the resolved end string at the end of its own buffer; the wrapper then concatenates THUMBNAIL/CONFIG.
  - Empty / whitespace-only resolved string ⇒ emit zero bytes (no header comment, no blank line, no phantom marker).
- Add new TDD test file `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` covering the 9 positive ACs and 3 negative ACs from `packet.spec.md` (12 tests total). The test exercises the END-TO-END pipeline: `slicer-cli` invocation → `ResolvedConfig` → module reads keys → module substitutes → module pushes via new variants → dispatch routes into `GCodeIR` fields → serializer emits at correct byte positions.
- Build the new core module's `.wasm` via `./modules/core-modules/build-core-modules.sh` and confirm `--check` returns clean. The WIT change triggers test-guest re-build as well — confirm `./test-guests/build-test-guests.sh --check` returns clean.
- Add TASK-193, TASK-193a, TASK-193b entries to `docs/07_implementation_status.md` via worker dispatch.

## Out of Scope

- **Range enforcement of `min`/`max` from module manifest.** `min`/`max` are parsed into `ConfigFieldEntry` (`crates/slicer-host/src/manifest.rs:827-828`) but not enforced at runtime — `ResolvedConfig::apply_cli_key` (`crates/slicer-ir/src/resolved_config.rs:194`) accepts any numeric value. Tracked as a separate future TASK-### packet. The declared `0..=120` / `0..=300` ranges in this packet's manifest are declarative-only.
- Arithmetic in placeholders (`[bed*2]`, `[x+5]`). Tracked for a separate future packet.
- Conditionals (`{if cond}...{elsif}...{else}...{endif}`), loops (`{for x in ...}{endfor}`), builtin functions (`min`, `max`, `round`, ...). Tracked for separate future packets.
- `{var}` brace-syntax placeholders. Only `[key]` square-bracket syntax is supported in this packet.
- Per-extruder / per-region / per-object placeholders (e.g., `[nozzle_temperature_initial_layer_0]`, `[filament_diameter_0]`).
- Adopting OrcaSlicer's stock `machine_start_gcode` / `machine_end_gcode` defaults. We intentionally use the user-specified Klipper PRINT_START / PRINT_END macros instead — recorded as an intentional deviation under the OrcaSlicer Reference Obligations section.
- OrcaSlicer's custom-gcode placeholder validator.
- Adding ANY further temperature / fan / filament / printer-profile keys beyond the four enumerated.
- Multi-extruder support (`M104 T1 S...`, `T0/T1` switches inside start gcode).
- Real macro semantics for `PRINT_START` / `PRINT_END` — those live in printer firmware / Klipper config.
- Modifying `LayerCollectionIR` or `ConfigView` in `docs/02_ir_schemas.md`. Only `GCodeIR` is extended (additive: 2 new `Option<String>` fields, both defaulting to `None`).
- Modifying `ThumbnailAwareSerializer` ordering or the CONFIG_BLOCK contents beyond the four new keys naturally flowing in via the effective `ConfigView`.
- Modifying `M82`/`M83`/`G90`/`G21` preamble emission established by packet 54.
- Cross-component log forwarding from the WASM guest to the host (for the unknown-placeholder negative AC). With substitution in the guest, host-side `log::warn!` capture is not trivially available; the previous AC's WARN-capture clause has been DROPPED. Tracked as a separate future packet.

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage and serializer role. Likely > 300 lines; delegate a SUMMARY.
- `docs/02_ir_schemas.md` — `ConfigView`, `ResolvedConfig`, `ConfigValue` enum, `GCodeIR` struct. > 300 lines; range-read the `ConfigValue` and `GCodeIR` sections; delegate everything else.
- `docs/03_wit_and_manifest.md` — `[config.schema.<key>]` manifest section + WIT contract management. Load directly only the schema-validation paragraph + the additive-only WIT-extension paragraph.
- `docs/05_module_sdk.md` — `#[slicer_module]` macro and `FinalizationModule` trait. Range-read only an example with a non-no-op body.
- `docs/07_implementation_status.md` — > 500 lines. DELEGATE every read AND every edit. Step 1 of this packet adds TASK-193 / TASK-193a / TASK-193b rows via a worker.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — register the two intentional deviations from OrcaSlicer (stock defaults, end-gcode position relative to CONFIG_BLOCK) if review requires it.

Default rule: delegate any doc > 300 lines. All ranged reads above stay within the budget.

## OrcaSlicer Reference Obligations

All reads delegated; never load OrcaSlicer source into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` (machine_end_gcode + machine_start_gcode field declarations) — confirms our key names match OrcaSlicer parity. **Behavior borrowed: snake_case key names.** **Not borrowed: nothing — these are pure declarations.**
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (stock-default `add` + `set_default_value`) — confirms OrcaSlicer's stock defaults. **Behavior NOT borrowed:** stock OrcaSlicer defaults. Our defaults instead delegate to Klipper PRINT_START / PRINT_END macros per user specification. **Intentional deviation — document in this packet's design.md and, if review requires, register in `docs/DEVIATION_LOG.md`.**
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3181` (placeholder substitution), `:3200` (start gcode write), `:3258` (preamble after start gcode) — **Behavior borrowed:** ordering is `machine_start_gcode` THEN preamble. Our serializer matches: resolved start string is inserted BEFORE the existing packet-54 M82/M83 preamble. **Not borrowed:** full placeholder grammar (out of scope for this packet).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3544` — OrcaSlicer's end-gcode write site. **Intentional deviation:** OrcaSlicer emits end gcode AFTER its CONFIG_BLOCK; we emit it BEFORE the `ThumbnailAwareSerializer`-owned THUMBNAIL_BLOCK / CONFIG_BLOCK because our CONFIG_BLOCK is structurally a footer wrapper, not the final printed block. Downstream printer parsers ignore comments after the last printable command, so this difference is transparent to the printer; document in design.md.
- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (`apply_config()`) — **Behavior borrowed:** placeholder values are sourced from the same config symbol table that holds the temperature keys. Our minimal helper mirrors this by accepting a `HashMap<String, ConfigValue>` view of the effective config. **Not borrowed:** the full `process()` grammar (out of scope; deferred to future packets).

## Acceptance Summary

- **Positive cases (9):** see `packet.spec.md` Acceptance Criteria section. Cover: default-template substitution end-to-end, default `PRINT_END` emission, start-block position between HEADER_BLOCK_END and first extrusion `G1`, end-block position between last `G1` and CONFIG_BLOCK_START, full user override of start template, partial user override of just temperature values, empty `machine_end_gcode` ⇒ no block, four-key declaration in the new module manifest (verified via manifest-discovery API or, fallback, via CONFIG_BLOCK presence), four new keys appearing in CONFIG_BLOCK.
- **Negative cases (3):** see `packet.spec.md` Negative Test Cases section. Cover: unknown placeholder passthrough (renamed `unknown_placeholder_passes_through_verbatim`; the WARN-log clause has been DROPPED — see Out of Scope), unclosed-bracket literal passthrough (no panic, no infinite loop), start-block content does NOT appear inside HEADER_BLOCK / inside CONFIG_BLOCK / after first `G1` (regression sentry). The previously-included `rejects_temp_out_of_registered_range` negative case is SCOPE-CUT — see Out of Scope: range enforcement is declarative-only as of this packet.
- **Measurable outcomes:**
  - Exactly one `M190 S60` line, exactly one `M109 S215` line, exactly one `PRINT_START EXTRUDER=215 BED=60` line in default output.
  - Exactly one `PRINT_END` line in default output.
  - Four new config keys declared in `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` with types `string`, `string`, `int`, `int` and defaults matching the Goal section of `packet.spec.md`.
  - Four new config keys present in the CONFIG_BLOCK of every default-config slicing run.
  - `FinalizationBuilderPush` grows from 6 to 8 variants; the new `PrintStartGcode(String)` / `PrintEndGcode(String)` are exercised exclusively by the new `machine-gcode-emit` module.
  - `GCodeIR` grows by two additive `Option<String>` fields; both default to `None`.
- **Cross-packet impact:**
  - Unblocks future "OrcaSlicer placeholder-parser arithmetic" packet (extends the module's substitution helper).
  - Unblocks future "OrcaSlicer placeholder-parser control flow" packet.
  - Unblocks future "printer-profile import" packet.
  - Unblocks future "manifest range enforcement" packet (needed to actually enforce the declared `0..=120` / `0..=300` ranges).
  - Unblocks future "cross-component WARN-log forwarding" packet (re-introduces the dropped WARN assertion).
  - Does NOT block, supersede, or modify any landed packet. Packets 54 and 55 emission sites are preserved verbatim.

## Verification Commands

- `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` — primary AC dispatch; FACT pass/fail, SNIPPETS (≤ 20 lines) on first-failing-assertion.
- `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — packet-55 regression.
- `cargo test -p slicer-host --test gcode_emit_tdd` — packet-52/54 regression.
- `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` — postpass pipeline regression.
- `./modules/core-modules/build-core-modules.sh --check` — guest WASM freshness (CLAUDE.md Guest WASM Staleness; mandatory after creating the new module AND after the WIT change).
- `./test-guests/build-test-guests.sh --check` — test-guest freshness (CLAUDE.md Guest WASM Staleness; the WIT change invalidates every test guest's bindgen output).
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`

All verification commands listed here are delegation-friendly so the implementer and reviewer can dispatch them to a sub-agent and consume only a FACT or SNIPPETS return.

**Test discipline reminder (CLAUDE.md):** `cargo test --workspace` is forbidden as a per-AC verification command and is NOT listed here. It may appear only at the packet's final acceptance ceremony in `implementation-plan.md` Step 7, dispatched to a sub-agent that returns FACT pass/fail.

## Step Completion Expectations

For each step in `implementation-plan.md`:

- **Step 1 — Docs/07 task entries**
  - Precondition: TASK-193, TASK-193a, TASK-193b not present in docs/07.
  - Postcondition: three rows appended in the correct phase / status column.
  - Falsifying check: worker dispatch `grep -n "TASK-193" docs/07_implementation_status.md` returns 3 hits, each with status `[ ]` (queued).
  - Files allowed to read: `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md:3-6` (for TASK-184 / TASK-185 row formatting precedent).
  - Files allowed to edit (≤ 3): `docs/07_implementation_status.md` (via worker dispatch only).
  - Expected sub-agent dispatches: 1 LOCATIONS dispatch; 1 edit dispatch; 1 FACT dispatch.
  - Step context cost: `S`.

- **Step 2 — TDD test file with all 12 failing assertions**
  - Precondition: no `machine_start_end_gcode_emission_tdd.rs` test file.
  - Postcondition: test file present, all 12 tests compile-pass and assertion-fail (red).
  - Falsifying check: `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -50` returns exit non-zero AND every test is `FAILED`.
  - Files allowed to read: `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full — packet 55 fixture pattern), `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` (layer-fixture pattern), `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80` (slicer-cli invocation harness).
  - Files allowed to edit (≤ 3): `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` (new).
  - Expected sub-agent dispatches: 1 LOCATIONS dispatch (small STL fixture path); 1 FACT dispatch (test file compiles, all 12 tests RED).
  - Step context cost: `M`.

- **Step 3 — WIT/SDK/dispatch/IR extension**
  - Precondition: Step 2 complete; 12 tests red.
  - Postcondition: `wit/world-finalization.wit` declares two new methods on `finalization-output-builder`; `FinalizationBuilderPush` grows from 6 to 8 variants; `FinalizationOutputBuilder` impl exposes `push_print_start_gcode` / `push_print_end_gcode`; `dispatch.rs` apply-loop has two new match arms; `GCodeIR` has two new `Option<String>` fields with `Default = None`. `cargo build --tests` clean.
  - Falsifying check: `cargo build --tests` returns clean; targeted dispatch confirms `enum FinalizationBuilderPush` has 8 variants now (`grep -c "PrintStartGcode\|PrintEndGcode" crates/slicer-host/src/wit_host.rs` ≥ 2).
  - Files allowed to read: `wit/world-finalization.wit` (full — short file), `crates/slicer-sdk/src/traits.rs:700-:730` + `:1196-:1230`, `crates/slicer-host/src/wit_host.rs:830-:895` + `:4895-:5020`, `crates/slicer-host/src/dispatch.rs:1070-:1100` + `:2885-:2980`, `crates/slicer-ir/src/slice_ir.rs:1779-:1799`.
  - Files allowed to edit (≤ 5): `wit/world-finalization.wit`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-host/src/wit_host.rs`, `crates/slicer-host/src/dispatch.rs`, `crates/slicer-ir/src/slice_ir.rs`.
  - CLAUDE.md WIT/Type Changes Checklist applies — explicit sub-steps:
    - Search all `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for any type identity reference that must mirror the new methods.
    - Verify type identity matches across component boundaries.
    - Run `cargo build --tests` after the WIT change.
    - Update both the inline WIT (`wit/world-finalization.wit`) and any external WIT package references consistently.
  - Expected sub-agent dispatches: 1 GREP dispatch across `wit_guest` modules + `dispatch.rs` + `wit_host.rs` for any type identity reference; 1 FACT dispatch for `cargo build --tests`.
  - Step context cost: `M`.

- **Step 4 — Create `modules/core-modules/machine-gcode-emit/` + build its `.wasm`**
  - Precondition: Step 3 clean (`cargo build --tests` green).
  - Postcondition: new module folder present with three files; module's `run_finalization` runs real substitution and pushes via the new variants; `./modules/core-modules/build-core-modules.sh --check` returns clean; targeted ACs `module_manifest_registers_four_keys_with_expected_types_and_defaults` and `new_keys_appear_in_config_block` turn green.
  - Falsifying check: targeted `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- module_manifest_registers_four_keys_with_expected_types_and_defaults --nocapture` passes; `--check` returns clean.
  - Files allowed to read: `modules/core-modules/part-cooling/part-cooling.toml` (full — ≤ 100 lines), `modules/core-modules/part-cooling/Cargo.toml` (full — ≤ 25 lines), `modules/core-modules/part-cooling/src/lib.rs` (full — 150 LOC; copies the trait shape, replaces the body).
  - Files allowed to edit (≤ 3): `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` (NEW), `modules/core-modules/machine-gcode-emit/Cargo.toml` (NEW), `modules/core-modules/machine-gcode-emit/src/lib.rs` (NEW).
  - Expected sub-agent dispatches: 1 FACT dispatch for `--check` clean; 1 FACT dispatch per targeted AC (2 total).
  - Step context cost: `M`.

- **Step 5 — Wire serializer to read GCodeIR fields**
  - Precondition: Step 4 complete; module pushes resolved strings; dispatch routes them into `GCodeIR` fields; 10 ACs still red because serializer does not yet read them.
  - Postcondition: `DefaultGCodeSerializer::serialize_gcode()` reads `gcode_ir.print_start_gcode` / `gcode_ir.print_end_gcode` and inserts at the contractual byte positions; empty/whitespace ⇒ no bytes; all 12 ACs green; regression suites green.
  - Falsifying check: `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` passes; `cargo test -p slicer-host --test gcode_emit_tdd` passes; `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` passes.
  - Files allowed to read: `crates/slicer-host/src/gcode_emit.rs:667-:740` (HEADER + width), `:928-:1020` (CONFIG_BLOCK + `ThumbnailAwareSerializer`), `:1021-:1166` (`serialize_gcode` body + preamble).
  - Files allowed to edit (≤ 1): `crates/slicer-host/src/gcode_emit.rs`.
  - Expected sub-agent dispatches: 1 SNIPPETS dispatch for start-block site; 1 SNIPPETS dispatch for end-block site; 1 FACT dispatch per targeted test run (3 total).
  - Step context cost: `S`.

- **Step 6 — Regression sweep**
  - Precondition: all 12 ACs green from Step 5.
  - Postcondition: regression test suites green; `cargo check --workspace` clean; `cargo clippy --workspace -- -D warnings` clean; `./modules/core-modules/build-core-modules.sh --check` and `./test-guests/build-test-guests.sh --check` both return clean.
  - Falsifying check: each regression command returns FACT pass.
  - Files allowed to read: none direct.
  - Files allowed to edit (≤ 3): zero or one (minor clippy fixes only).
  - Expected sub-agent dispatches: 6 FACT dispatches.
  - Step context cost: `S`.

- **Step 7 — Packet completion gate**
  - Precondition: Step 6 clean.
  - Postcondition: every pipe-suffixed AC command re-dispatched and green; `cargo test --workspace` dispatched as the final closure ceremony per CLAUDE.md test discipline; docs/07 rows updated to `[x]`; `packet.spec.md` ready to flip from `draft` to `implemented` only after explicit user OK.
  - Falsifying check: any re-dispatched AC returns fail.
  - Files allowed to read: none direct.
  - Files allowed to edit (≤ 3): `docs/07_implementation_status.md` (via worker), `.ralph/specs/59_machine-start-end-gcode-emission/packet.spec.md` (status flip — requires user OK).
  - Expected sub-agent dispatches: 12 FACT dispatches (one per AC); 1 FACT dispatch for `cargo test --workspace`; 1 worker dispatch to update docs/07.
  - Step context cost: `S`.

Aggregate per-step context cost: 5×S + 2×M = `M` total. No single step is `L`.

## Context Discipline Notes

Context-budget hazards specific to this packet:

- **Large files in the read-only path that MUST be ranged or delegated:**
  - `crates/slicer-host/src/gcode_emit.rs` (> 1100 lines) — only `:667-:740`, `:928-:1020`, and `:1021-:1166` are needed; never load full file.
  - `crates/slicer-host/src/dispatch.rs` (> 3000 lines) — only `:1070-:1100` and `:2885-:2980` are needed; never load full file.
  - `crates/slicer-host/src/wit_host.rs` (multi-thousand lines) — only `:830-:895` and `:4895-:5020`; never load full file.
  - `crates/slicer-ir/src/slice_ir.rs` (> 1600 lines) — only `:550-:580` (`ConfigValue`) and `:1779-:1799` (`GCodeIR`); never load full file.
  - `crates/slicer-sdk/src/traits.rs` (> 1200 lines) — only `:700-:730` and `:1196-:1230`; never load full file.
  - `docs/07_implementation_status.md` (> 500 lines) — never load; all reads / edits via worker.
  - `docs/02_ir_schemas.md` (> 300 lines) — range-read only.
- **OrcaSlicer trees the implementer must NOT load directly:**
  - All of `OrcaSlicerDocumented/` is delegated. The FACT dispatches enumerated in `packet.spec.md` are the only evidence this packet needs. NEVER read `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` directly (~2400 lines of Boost.Spirit grammar; out of scope).
- **Likely temptation reads (skip):**
  - Reading other core-module manifests beyond `part-cooling.toml` (and `seam-placer.toml` for String-key precedent) — they add nothing.
  - Reading `crates/slicer-cli/src/cmd_run.rs` end-to-end — the relevant build site is `crates/slicer-host/src/pipeline.rs:435-449`.
  - Reading other packets' `design.md` "to compare patterns" — packets 54 / 55 are the only relevant precedents and are quoted here where needed.
- **Sub-agent return-format hints for the heaviest dispatches:**
  - OrcaSlicer FACT dispatches: ≤ 12 lines each; no code blocks > 4 lines; cite file:line and quote the relevant identifier or string literal only.
  - `cargo test` dispatches: FACT pass/fail; SNIPPETS (≤ 20 lines) on failure with the first failing assertion + the test name + ≤ 15 lines of context.
  - `cargo check` / `cargo clippy` dispatches: FACT pass/fail; SNIPPETS (≤ 30 lines) of the first error / warning only.
  - `docs/07_implementation_status.md` worker dispatches: never return the full file; return the row insertion point as LOCATIONS (file:line + adjacent row text), and after the edit return FACT (`grep -n "TASK-193" hits = 3`).
  - `./modules/core-modules/build-core-modules.sh --check` and `./test-guests/build-test-guests.sh --check` dispatches: FACT clean/stale.
