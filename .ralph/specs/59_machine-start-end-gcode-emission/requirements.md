# Requirements: 59_machine-start-end-gcode-emission

## Packet Metadata

- Grouped task IDs:
  - `TASK-193`  — emit configurable `machine_start_gcode` / `machine_end_gcode` via a `PostPass::GCodePostProcess` module that prepends/appends `Raw` commands carrying the resolved templates.
  - `TASK-193a` — create `modules/core-modules/machine-gcode-emit/` declaring four `[config.schema.*]` keys; `run_gcode_postprocess` performs real `[key]` substitution against the effective `ConfigView` and rebuilds the command list as `[Raw(start), ...existing..., Raw(end)]`.
  - `TASK-193b` — promote `M82`/`M83` from the hard-coded serializer preamble to a new `GCodeCommand::ExtrusionMode { absolute: bool }` variant pushed by `DefaultGCodeEmitter`, so a downstream `GCodePostProcess` module can prepend before it.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M` (no single step is `L`)

## Problem Statement

The slicer currently produces a G-code file that begins with `HEADER_BLOCK` + width comments and a hard-coded `M82`/`M83` extrusion-mode preamble, then jumps straight to the first layer's commands. It ends with the last layer's commands followed by `THUMBNAIL_BLOCK` + `CONFIG_BLOCK` (packet 55). **No printer start sequence is emitted** — no homing, no temperature waits, no Klipper `PRINT_START` macro invocation. **No printer finish sequence is emitted** either — no `PRINT_END` macro, no shutdown, no heater-off or motors-off sequence.

The produced `.gcode` therefore cannot be sent directly to a printer: extrusion would begin on a cold hotend and un-homed bed. Users would have to prepend boilerplate every print, which defeats one of the core ergonomics of a host slicer.

This is also the first packet to introduce **placeholder substitution** in serialized G-code. OrcaSlicer's full `PlaceholderParser` is a Boost.Spirit Qi grammar of ~2400 lines. Implementing parity in one packet is out of proportion to the immediate need. This packet implements the **smallest viable subset**: `[snake_case_key]` literal substitution against the effective `ConfigView`, performed INSIDE the new core module.

The earlier draft of this packet routed substituted strings through new `FinalizationBuilderPush` variants, new `Option<String>` fields on `GCodeIR`, new dispatch arms, and explicit byte-offset placement in the host serializer. That design was reconsidered because the slicer already has an architecturally clean home for this work: the existing `PostPass::GCodePostProcess` stage (`crates/slicer-host/src/execution_plan.rs:38`), wired in the postpass executor at `crates/slicer-host/src/postpass.rs:215`, whose modules receive `list<gcode-command>` + `gcode-output-builder` + `config-view` (`wit/world-postpass.wit:26`). A `GCodePostProcess` module can rebuild `GCodeIR.commands` as `[Raw(start), ...existing..., Raw(end)]` with zero new WIT, no new IR fields, no new dispatch arms, and no byte-offset arithmetic in the serializer.

The one architectural friction with that simpler design is that `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-host/src/gcode_emit.rs:1107`) currently writes `M82`/`M83` as hard-coded raw strings at `:1154-1156` *between* `HEADER_BLOCK` and the per-command loop. A `GCodePostProcess` module can prepend at the head of `GCodeIR.commands`, but the hard-coded preamble lies *outside* that list — so `Raw(machine_start_gcode)` at index 0 would appear AFTER `M82`/`M83` in output, which deviates from OrcaSlicer ordering (`machine_start_gcode` THEN extrusion-mode preamble).

This packet therefore performs a small companion refactor: promote `M82`/`M83` from the hard-coded serializer preamble to a new `GCodeCommand::ExtrusionMode { absolute: bool }` variant, pushed by `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-host/src/gcode_emit.rs:304`) as the first command. The serializer renders `ExtrusionMode { absolute: true }` as `M82\n` and `ExtrusionMode { absolute: false }` as `M83\n`, matching the existing per-command rendering pattern (see `Temperature` at `:1280-1281`). After the promotion, the entire stream between `; HEADER_BLOCK_END` and `; CONFIG_BLOCK_START` originates from `GCodeIR.commands`, and the `GCodePostProcess` module's prepend lands in the right byte position by construction.

The four config keys (`machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer`) do not exist anywhere in the workspace today. They are declared in this packet by the new module's manifest TOML. The default `machine_start_gcode` template references the two temperature keys via `[bed_temperature_initial_layer_single]` and `[nozzle_temperature_initial_layer]`; without those declarations the substitution would pass through literal placeholder text.

This packet does NOT reopen or supersede any prior packet. Packets 54 (preamble) and 55 (HEADER + CONFIG_BLOCK + ThumbnailAwareSerializer) remain correct as shipped — packet 54's effect (an M82 or M83 between header and first layer) is preserved bit-identically for default configs; only the *origin* of that line shifts from a hard-coded serializer write to a `GCodeCommand` pushed by the emitter.

## In Scope

- Add `GCodeCommand::ExtrusionMode { absolute: bool }` variant to the `GCodeCommand` enum at `crates/slicer-ir/src/slice_ir.rs:1697`. Additive only — the existing `Move, Retract, Unretract, FanSpeed, Temperature, ToolChange, Comment, Raw` variants are unchanged.
- Update `DefaultGCodeEmitter::emit_gcode` (`crates/slicer-host/src/gcode_emit.rs:304`) to push `ExtrusionMode { absolute }` as the head command of the `Vec<GCodeCommand>` it builds. `absolute` is `true` when the existing flavor decision in the preamble path at `:1154-1156` selects `M82`, `false` when it selects `M83`. The existing decision is preserved verbatim — only the *consumer* of the boolean changes from inline-string-write to enum-construction.
- Update `DefaultGCodeSerializer::serialize_gcode` (`crates/slicer-host/src/gcode_emit.rs:1107`):
  - Remove the hard-coded `M82\n` / `M83\n` writes at `:1154-1156`.
  - Add a `GCodeCommand::ExtrusionMode { absolute }` arm in the per-command renderer (the same `match` that currently dispatches `GCodeCommand::Temperature` at `:1280-1281` and the other variants). The arm writes `"M82\n"` when `absolute == true`, `"M83\n"` otherwise.
- Create `modules/core-modules/machine-gcode-emit/` with three files:
  - `machine-gcode-emit.toml` — manifest declaring `[stage] id = "PostPass::GCodePostProcess"` and four `[config.schema.<key>]` blocks. `[module]` header keys (`id`, `version`, `display-name`, `description`, `author`, `license`, `wit-world`) mirror `modules/core-modules/part-cooling/part-cooling.toml`'s shape verbatim. `wit-world` is whichever WIT world the existing `GCodePostProcess` modules use (typically `slicer:world-postpass@1.0.0`; confirm via a single LOCATIONS dispatch in Step 4 if needed).
    - `machine_start_gcode` (`type = "string"`) default = multi-line literal (TOML triple-quoted):
      ```
      M190 S[bed_temperature_initial_layer_single]
      M109 S[nozzle_temperature_initial_layer]
      PRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]
      ```
    - `machine_end_gcode` (`type = "string"`) default = `"PRINT_END"`.
    - `bed_temperature_initial_layer_single` (`type = "int"`) default = `60`, `min = 0`, `max = 120` (declarative only).
    - `nozzle_temperature_initial_layer` (`type = "int"`) default = `215`, `min = 0`, `max = 300` (declarative only).
  - `Cargo.toml` — mirrors `modules/core-modules/part-cooling/Cargo.toml`.
  - `src/lib.rs` — implements the SDK's GCodePostProcess trait (identify the exact trait name and signature via a single SDK dispatch in Step 4; likely named `GCodePostProcessModule` or similar, mirroring `FinalizationModule`'s shape) with a REAL body. The body:
    1. Reads `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer` from the `&ConfigView` argument.
    2. Builds a `HashMap<String, ConfigValue>` lookup containing the two temperature keys.
    3. Calls a private free `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper (≤ 60 LOC, scoped to this module).
    4. Pushes the resolved start as one `Raw` command via the output builder (skip if empty/whitespace).
    5. Re-emits every command from the snapshot input list (via the SDK's idiomatic pass-through pattern — likely `output.extend_from_snapshot(input)` or per-variant push; the implementer mirrors whichever shape the SDK exposes).
    6. Pushes the resolved end as one `Raw` command (skip if empty/whitespace).
    7. Returns `Ok(())`.
- Add new TDD test file `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` covering the 9 positive ACs and 3 negative ACs from `packet.spec.md` (12 tests total). The test exercises END-TO-END: `slicer-cli` invocation → `ResolvedConfig` → emitter builds commands with `ExtrusionMode` at head → `GCodePostProcess` module reads keys, substitutes, prepends `Raw(start)`, re-emits, appends `Raw(end)` → serializer renders the new command list → byte-level file scan.
- Build the new core module's `.wasm` via `./modules/core-modules/build-core-modules.sh` and confirm `--check` returns clean.
- Add TASK-193, TASK-193a, TASK-193b entries to `docs/07_implementation_status.md` via worker dispatch.

## Out of Scope

- Any new method on `wit/world-finalization.wit` or `wit/world-postpass.wit`. The simpler design uses only the existing `gcode-output-builder.push-raw` method at `wit/deps/ir-types.wit:144`. If the `GCodeCommand` variant addition requires extending `wit/deps/ir-types.wit`'s `gcode-command` variant set (likely yes for typed cross-WIT identity), that is in scope BUT limited to the single variant — no new methods, no new resources, no new worlds.
- Any change to `FinalizationBuilderPush`, `crates/slicer-host/src/dispatch.rs`, or new `Option<String>` fields on `GCodeIR`. The earlier draft's plumbing is REMOVED.
- Any byte-offset arithmetic in the serializer. The serializer renders `GCodeIR.commands` in order; positions of `Raw(start)` / `Raw(end)` are guaranteed by their position in the command list, not by serializer logic.
- Range enforcement of `min`/`max` from the manifest. `ResolvedConfig::apply_cli_key` (`crates/slicer-ir/src/resolved_config.rs:194`) does not consult `min`/`max`. The declared ranges are declarative-only. Tracked as a separate future TASK-### packet.
- Arithmetic in placeholders (`[bed*2]`, `[x+5]`). Tracked for a separate future packet.
- Conditionals (`{if cond}...{elsif}...{else}...{endif}`), loops (`{for x in ...}{endfor}`), builtin functions (`min`, `max`, `round`, ...). Tracked for separate future packets.
- `{var}` brace-syntax placeholders. Only `[key]` square-bracket syntax in this packet.
- Per-extruder / per-region / per-object placeholders.
- Adopting OrcaSlicer's stock `machine_start_gcode` / `machine_end_gcode` defaults. We intentionally use Klipper PRINT_START / PRINT_END macros — recorded as an intentional deviation if review requires.
- OrcaSlicer's custom-gcode placeholder validator.
- Adding ANY further temperature / fan / filament / printer-profile keys beyond the four enumerated.
- Multi-extruder M104/M109 tool-index variants.
- Real `PRINT_START` / `PRINT_END` macro semantics — those live in printer firmware / Klipper config.
- Modifying `LayerCollectionIR` or `ConfigView` shape.
- Modifying `ThumbnailAwareSerializer` ordering or CONFIG_BLOCK contents beyond the four new keys naturally flowing in via packet 55's automatic propagation.
- Cross-component log forwarding from the WASM guest to the host (for the unknown-placeholder negative AC).

## Authoritative Docs

- `docs/01_system_architecture.md` — postpass vs finalization stages and serializer role. Likely > 300 lines; delegate a SUMMARY.
- `docs/02_ir_schemas.md` — `GCodeCommand` variants, `ConfigValue` enum, `GCodeIR` struct. > 300 lines; range-read only the `GCodeCommand`/`ConfigValue`/`GCodeIR` sections; delegate everything else.
- `docs/03_wit_and_manifest.md` — `[config.schema.<key>]` manifest section. Load directly only the schema-validation paragraph.
- `docs/05_module_sdk.md` — `#[slicer_module]` macro and the GCodePostProcess module trait. Range-read only one example with a non-no-op body.
- `docs/07_implementation_status.md` — > 500 lines. DELEGATE every read AND every edit. Step 1 of this packet adds three TASK-### rows via a worker.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — register only if review requires (e.g., for the OrcaSlicer end-gcode-after-CONFIG-block ordering difference).

Default rule: delegate any doc > 300 lines. All ranged reads above stay within the budget.

## OrcaSlicer Reference Obligations

All reads delegated; never load OrcaSlicer source into the implementer's context.

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — confirms our key names match OrcaSlicer parity. **Behavior borrowed:** snake_case key names. **Not borrowed:** nothing — these are declarations only.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — **Behavior NOT borrowed:** OrcaSlicer stock defaults. Our defaults use Klipper PRINT_START / PRINT_END per user specification. **Intentional deviation — document in `design.md` and, if review requires, register in `docs/DEVIATION_LOG.md`.**
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (the regions writing `machine_start_gcode` and the extrusion-mode preamble) — **Behavior borrowed:** ordering is `machine_start_gcode` THEN extrusion-mode preamble. Our design produces this ordering naturally because `Raw(start)` is prepended before the `ExtrusionMode` head command. **Not borrowed:** full placeholder grammar (out of scope).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` (the `machine_end_gcode` write region) — **Intentional deviation:** OrcaSlicer emits end gcode AFTER its CONFIG_BLOCK; we emit it BEFORE the `ThumbnailAwareSerializer`-owned `THUMBNAIL_BLOCK` / `CONFIG_BLOCK` because our `CONFIG_BLOCK` is structurally a metadata footer. Comments after the last printable command are ignored by all firmwares, so the difference is transparent to the printer. Document in `design.md`.
- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (`apply_config()` entry point only) — **Behavior borrowed:** placeholder values are sourced from the same config symbol table that holds the temperature keys. Our minimal helper mirrors this by accepting a `HashMap<String, ConfigValue>` view. **Not borrowed:** the full `process()` grammar.

## Acceptance Summary

- **Positive cases (10):** see `packet.spec.md` Acceptance Criteria section. Cover: default-template substitution end-to-end, default `PRINT_END` emission, start-block position between `HEADER_BLOCK_END` and first `M82`/`M83`, end-block position between last `G1` and `CONFIG_BLOCK_START`, M82/M83 promotion regression sentry, full user override of start template, partial user override of temperature values, empty `machine_end_gcode` ⇒ no block, four-key manifest declaration, four new keys appearing in CONFIG_BLOCK.
- **Negative cases (3):** see `packet.spec.md` Negative Test Cases section. Cover: unknown placeholder verbatim passthrough, unclosed-bracket literal passthrough (no panic, no infinite loop), start-block content NOT inside `HEADER_BLOCK` or `CONFIG_BLOCK`.
- **Measurable outcomes:**
  - Exactly one `M190 S60`, exactly one `M109 S215`, exactly one `PRINT_START EXTRUDER=215 BED=60` in default output.
  - Exactly one `PRINT_END` in default output.
  - Exactly one `M82` or `M83` line in default output (whichever the existing flavor decision selects); the line appears between `; HEADER_BLOCK_END` and the first `G1` extrusion move and AFTER the `PRINT_START` line.
  - Four new config keys declared in `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` with types `string`, `string`, `int`, `int` and defaults matching the Goal section of `packet.spec.md`.
  - Four new config keys present in `CONFIG_BLOCK` of every default-config slicing run.
  - `GCodeCommand` grows by exactly one new additive variant (`ExtrusionMode { absolute: bool }`); all existing variants unchanged.
  - `GCodeIR` struct is **unchanged**. No new fields.
  - `FinalizationBuilderPush` is **unchanged**. No new variants.
  - `crates/slicer-host/src/dispatch.rs` is **unchanged** (no new match arms).
  - `wit/world-finalization.wit` is **unchanged**.
- **Cross-packet impact:**
  - Unblocks future "OrcaSlicer placeholder-parser arithmetic" packet (extends the module's substitution helper).
  - Unblocks future "OrcaSlicer placeholder-parser control flow" packet.
  - Unblocks future "printer-profile import" packet.
  - Unblocks future "manifest range enforcement" packet (needed to actually enforce the declared `0..=120` / `0..=300` ranges).
  - Does NOT block, supersede, or modify any landed packet. Packets 54 and 55 effects are preserved bit-identically for default configs.

## Verification Commands

- `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` — primary AC dispatch; FACT pass/fail, SNIPPETS (≤ 20 lines) on first-failing-assertion.
- `cargo test -p slicer-host --test gcode_emit_tdd` — packet-52/54 regression (CRITICAL: M82/M83 promotion).
- `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — packet-55 regression.
- `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` — postpass pipeline regression.
- `./modules/core-modules/build-core-modules.sh --check` — guest WASM freshness (CLAUDE.md Guest WASM Staleness; mandatory after creating the new module AND after any `slicer-ir` change).
- `./test-guests/build-test-guests.sh --check` — test-guest freshness (CLAUDE.md flags `slicer-ir` as a universal guest dep; the `GCodeCommand::ExtrusionMode` variant addition invalidates every test-guest's bindgen output if `gcode-command` crosses the WIT boundary).
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`

**Test discipline reminder (CLAUDE.md):** `cargo test --workspace` is forbidden as a per-AC verification command. It appears only at the packet's final acceptance ceremony in `implementation-plan.md` Step 6, dispatched to a sub-agent that returns FACT pass/fail.

## Step Completion Expectations

For each step in `implementation-plan.md`:

- **Step 1 — Docs/07 task entries**
  - Precondition: TASK-193, TASK-193a, TASK-193b not present in docs/07.
  - Postcondition: three rows appended with status `[ ]` (queued).
  - Falsifying check: `grep -n "TASK-193" docs/07_implementation_status.md` returns 3 hits.
  - Files allowed to read: `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md:3-6` (row-formatting precedent).
  - Files allowed to edit (≤ 3): `docs/07_implementation_status.md` (via worker dispatch only).
  - Expected sub-agent dispatches: 1 LOCATIONS dispatch; 1 edit dispatch; 1 FACT dispatch.
  - Step context cost: `S`.

- **Step 2 — TDD test file with 12 failing assertions**
  - Precondition: no `machine_start_end_gcode_emission_tdd.rs` test file.
  - Postcondition: test file present, all 12 tests compile-pass and assertion-fail (red).
  - Falsifying check: `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -50` returns exit non-zero AND every test is `FAILED`.
  - Files allowed to read: `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full), `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120`, `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80`.
  - Files allowed to edit (≤ 3): `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` (new).
  - Expected sub-agent dispatches: 1 LOCATIONS dispatch (STL fixture path); 1 FACT dispatch (test file compiles, all 12 RED).
  - Step context cost: `M`.

- **Step 3 — Promote M82/M83 to `GCodeCommand::ExtrusionMode`**
  - Precondition: Step 2 complete; 12 tests red.
  - Postcondition: `GCodeCommand` has 9 variants (was 8); `DefaultGCodeEmitter::emit_gcode` pushes `ExtrusionMode { absolute }` as the head command; `DefaultGCodeSerializer::serialize_gcode` no longer hard-codes M82/M83 at `:1154-1156` and has a new arm rendering `ExtrusionMode` in the per-command loop; `cargo test -p slicer-host --test gcode_emit_tdd` passes (the packet-54 regression suite confirms M82/M83 still appear in output); the AC `extrusion_mode_still_emitted_after_promotion` turns green.
  - Falsifying check: `cargo test -p slicer-host --test gcode_emit_tdd` passes; `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- extrusion_mode_still_emitted_after_promotion --nocapture` passes.
  - Files allowed to read: `crates/slicer-ir/src/slice_ir.rs:1697-:1770` (`GCodeCommand` enum); `crates/slicer-host/src/gcode_emit.rs:300-:340` (emitter entry), `:1100-:1170` (serializer body + M82/M83 writes), `:1270-:1300` (per-command renderer arms); `wit/deps/ir-types.wit` (full — short).
  - Files allowed to edit (≤ 3): `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-host/src/gcode_emit.rs`, `wit/deps/ir-types.wit` (only if `gcode-command` is mirrored in WIT and needs the new variant for typed identity across the postpass boundary; if it isn't mirrored, leave WIT untouched).
  - CLAUDE.md WIT/Type Changes Checklist applies IF `wit/deps/ir-types.wit` is touched:
    - Search `wit_host.rs`, `dispatch.rs`, and `wit_guest` for any `gcode-command` reference.
    - Verify type identity across component boundaries.
    - Run `cargo build --tests` after the change.
  - Expected sub-agent dispatches: 1 FACT dispatch (does `wit/deps/ir-types.wit` mirror `gcode-command`?); 1 FACT dispatch (`cargo build --tests` after the change); 2 FACT dispatches (the two targeted test runs).
  - Step context cost: `M`.

- **Step 4 — Create `modules/core-modules/machine-gcode-emit/` with real `run_gcode_postprocess`**
  - Precondition: Step 3 clean.
  - Postcondition: new module folder with three files; `run_gcode_postprocess` body reads four keys → substitutes → prepends `Raw(start)` → re-emits snapshot → appends `Raw(end)`; `./modules/core-modules/build-core-modules.sh --check` clean; the remaining 11 ACs all turn green.
  - Falsifying check: `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` passes (all 12 green).
  - Files allowed to read: `modules/core-modules/part-cooling/{part-cooling.toml, Cargo.toml, src/lib.rs}` (full reads OK), `modules/core-modules/seam-placer/seam-placer.toml` (string-key precedent), `crates/slicer-sdk/src/traits.rs` ranged (find the GCodePostProcess trait — likely a `GCodePostProcessModule` trait or equivalent), `wit/world-postpass.wit` (full — short).
  - Files allowed to edit (≤ 3): `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` (NEW), `modules/core-modules/machine-gcode-emit/Cargo.toml` (NEW), `modules/core-modules/machine-gcode-emit/src/lib.rs` (NEW).
  - Expected sub-agent dispatches: 1 FACT dispatch (SDK trait name + signature for GCodePostProcess); 1 FACT dispatch (`build-core-modules.sh --check` clean); 1 FACT dispatch (`test-guests/build-test-guests.sh --check` clean if any test-guest depends on `slicer-ir`); 1 FACT dispatch (full TDD suite green).
  - Step context cost: `M`.

- **Step 5 — Regression sweep + workspace gates**
  - Precondition: Step 4 complete.
  - Postcondition: regression test suites green; `cargo check --workspace` clean; `cargo clippy --workspace -- -D warnings` clean; both `--check` commands clean.
  - Falsifying check: each regression command returns FACT pass.
  - Files allowed to read: none direct.
  - Files allowed to edit (≤ 3): zero or one (minor clippy fixes only).
  - Expected sub-agent dispatches: 6 FACT dispatches.
  - Step context cost: `S`.

- **Step 6 — Packet completion gate**
  - Precondition: Step 5 clean.
  - Postcondition: every pipe-suffixed AC command re-dispatched and green; `cargo test --workspace` dispatched as the closure ceremony; docs/07 rows updated to `[x]`; `packet.spec.md` ready to flip from `draft` to `implemented` only after explicit user OK.
  - Falsifying check: any re-dispatched AC returns fail.
  - Files allowed to read: none direct.
  - Files allowed to edit (≤ 3): `docs/07_implementation_status.md` (via worker), `.ralph/specs/59_machine-start-end-gcode-emission/packet.spec.md` (status flip — requires user OK).
  - Expected sub-agent dispatches: 13 FACT dispatches (12 ACs + workspace gate); 1 worker dispatch.
  - Step context cost: `S`.

Aggregate per-step context cost: 4×S + 2×M = `M`. No single step is `L`.

## Context Discipline Notes

Context-budget hazards specific to this packet:

- **Large files in the read-only path that MUST be ranged or delegated:**
  - `crates/slicer-host/src/gcode_emit.rs` (> 1100 lines) — only `:300-:340`, `:670-:740`, `:1100-:1170`, and `:1270-:1300` are needed; never load full file.
  - `crates/slicer-ir/src/slice_ir.rs` (> 1600 lines) — only `:1697-:1770` (`GCodeCommand`) and `:1779-:1799` (`GCodeIR`); never load full file.
  - `crates/slicer-host/src/postpass.rs` — range-read `:140-:280`; never load full file.
  - `crates/slicer-sdk/src/traits.rs` (> 1200 lines) — range-read only the GCodePostProcess trait section (locate via dispatch).
  - `docs/07_implementation_status.md` (> 500 lines) — never load; all reads / edits via worker.
  - `docs/02_ir_schemas.md` (> 300 lines) — range-read only.
- **OrcaSlicer trees the implementer must NOT load directly:**
  - All of `OrcaSlicerDocumented/` is delegated. The FACT dispatches enumerated in `packet.spec.md` are the only evidence this packet needs. NEVER read `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` directly.
- **Likely temptation reads (skip):**
  - Reading other core-module manifests beyond `part-cooling.toml` (and `seam-placer.toml` for String-key precedent) — they add nothing.
  - Reading `crates/slicer-cli/src/cmd_run.rs` end-to-end — the relevant invocation harness pattern lives in `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80`.
  - Reading other packets' `design.md` "to compare patterns" — packets 54 / 55 are the only relevant precedents and are quoted here where needed.
- **Sub-agent return-format hints for the heaviest dispatches:**
  - OrcaSlicer FACT dispatches: ≤ 12 lines each; no code blocks > 4 lines.
  - `cargo test` dispatches: FACT pass/fail; SNIPPETS (≤ 20 lines) on failure with the first failing test name + assertion + minimal context.
  - `cargo check` / `cargo clippy` dispatches: FACT pass/fail; SNIPPETS (≤ 30 lines) of the first error / warning only.
  - `docs/07_implementation_status.md` worker dispatches: return only the row insertion point as LOCATIONS + the three new rows verbatim after edit + a `grep` FACT.
  - `./modules/core-modules/build-core-modules.sh --check` and `./test-guests/build-test-guests.sh --check` dispatches: FACT clean/stale.
