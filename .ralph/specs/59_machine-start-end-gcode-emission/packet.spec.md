---
status: draft
packet: 59_machine-start-end-gcode-emission
task_ids:
  - TASK-193    # emit machine_start_gcode / machine_end_gcode with minimal [key] substitution
  - TASK-193a   # register machine_start_gcode, machine_end_gcode, bed_temperature_initial_layer_single, nozzle_temperature_initial_layer in FullConfigSchema
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 59_machine-start-end-gcode-emission

## Goal

Emit a configurable printer start sequence before the first extrusion move and a configurable finish sequence after the last move, both substituted from a host-level config schema using a minimal `[key_name]` placeholder pass. Concretely:

1. Register four new host-level config keys in `crates/slicer-host/src/config_schema.rs::FullConfigSchema::default()` (parallel to the `thumbnail_path` precedent from packet 55):
   - `machine_start_gcode` (String) — default exactly:
     ```
     M190 S[bed_temperature_initial_layer_single]
     M109 S[nozzle_temperature_initial_layer]
     PRINT_START EXTRUDER=[nozzle_temperature_initial_layer] BED=[bed_temperature_initial_layer_single]
     ```
   - `machine_end_gcode` (String) — default exactly `PRINT_END`.
   - `bed_temperature_initial_layer_single` (Int) — default `60`, registered range `0..=120`.
   - `nozzle_temperature_initial_layer` (Int) — default `215`, registered range `0..=300`.
2. Add a private `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper in `crates/slicer-host/src/gcode_emit.rs` that replaces every `[snake_case_key]` token whose key resolves in `lookup`. Unknown tokens pass through verbatim and emit a `log::warn!("unknown placeholder: {key}")` line (the `log` crate is already in `crates/slicer-host/Cargo.toml:20` as `log = "0.4"`; no new dependency). `[` not followed by a matching `]` on the same line is treated as literal text. No arithmetic, no conditionals, no loops, no `{var}` syntax.
3. In `DefaultGCodeSerializer::serialize_gcode()`:
   - After the existing HEADER_BLOCK + extrusion-width comments (currently emitted at file head, see `crates/slicer-host/src/gcode_emit.rs:626-740`; `serialize_header_block` at `:626`, `serialize_width_comments` at `:671`) and before the existing M82/M83 preamble (currently emitted near `:979-1166`; M83 at `:1026`, M82 at `:1028`), insert the substituted `machine_start_gcode` block followed by a single trailing newline.
   - After the last layer's commands and before the `ThumbnailAwareSerializer`-owned THUMBNAIL_BLOCK / CONFIG_BLOCK footer (CONFIG-block emitter `serialize_config_block` at `:887`; wrapper `impl ThumbnailAwareSerializer` at `:932`; inner-serializer call at `:950`; THUMBNAIL/CONFIG append at `:953-974`), insert the substituted `machine_end_gcode` block followed by a single trailing newline.
   - When the resolved (post-substitution) block is empty (or whitespace-only), emit nothing — no header comment, no blank line, no phantom block.
4. Add no new IR fields, no new module manifests, no new WIT contracts.

Order of operations matches OrcaSlicer (start gcode BEFORE preamble; see `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3200` then `:3258`). Substitution is INTENTIONALLY a narrow subset of OrcaSlicer's `PlaceholderParser`; arithmetic / conditionals / loops are out of scope and tracked as future TASK-### entries.

## Scope Boundaries

- In scope:
  - New TDD test file `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs`.
  - Registering `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer` in `crates/slicer-host/src/config_schema.rs::FullConfigSchema::default()` with the defaults listed in the Goal.
  - One private helper `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` in `crates/slicer-host/src/gcode_emit.rs` (≤ 60 LOC including unknown-key warning and unclosed-bracket passthrough).
  - Wiring the substituted start block into `DefaultGCodeSerializer::serialize_gcode()` after HEADER_BLOCK + width comments, before the M82/M83 preamble.
  - Wiring the substituted end block into the same serializer after the last layer's commands, before the THUMBNAIL/CONFIG footer (which is emitted by `ThumbnailAwareSerializer` per packet 55).
  - Adding TASK-193 and TASK-193a entries to `docs/07_implementation_status.md` (worker dispatch — never load the full backlog file into the implementer's context).
- Out of scope:
  - Arithmetic expressions in placeholders (`[bed*2]`, `[x+5]`). Tracked as a separate future packet.
  - Conditionals (`{if cond}...{elsif}...{else}...{endif}`), loops (`{for x in ...}{endfor}`), builtin functions. Tracked as separate future packets.
  - `{var}` brace-syntax placeholders. Only `[key]` square-bracket syntax in this packet.
  - Per-extruder / per-region / per-object placeholders.
  - OrcaSlicer's custom-gcode placeholder validator (`OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:9700-9701`).
  - Adding ANY further temperature, fan, filament, or printer-profile keys beyond the four enumerated above.
  - Modifying `ThumbnailAwareSerializer` ordering or `CONFIG_BLOCK` contents (other than the four new keys naturally appearing in the block by virtue of being in the effective `ConfigView`).
  - Multi-extruder M104/M109 tool-index variants (`M109 T1 S...`).
  - Real-PRINT_START macro semantics (homing, ABL probing, purge, etc.) — those live in printer firmware/Klipper config, not in the slicer.
  - Emitting M82/M83 / G90 / G21 changes beyond what packet 54 already produces.
  - Modifying `GCodeIR`, `LayerCollectionIR`, `ConfigView`, or any IR contract in `docs/02_ir_schemas.md`.

## Prerequisites and Blockers

- Depends on:
  - Packet 55 (HEADER_BLOCK + CONFIG_BLOCK + `ThumbnailAwareSerializer`) — landed; this packet inserts BETWEEN existing emission sites and relies on the `;TYPE:` and CONFIG_BLOCK_START byte-offset anchors that packet 55 established.
  - Packet 54 (M82/M83 preamble + `with_extrusion_mode` constructor) — landed; this packet's start-block insertion site is "immediately before the preamble line(s) that packet 54 emits". Verify the preamble's serialization offset at implementation time (Step 4) via a small dispatch.
- Unblocks:
  - Future "OrcaSlicer placeholder-parser arithmetic" packet (would extend the helper added here to support `[a+b]`, `[a*b]` etc.).
  - Future "OrcaSlicer placeholder-parser control flow" packet (would add `{if}/{elsif}/{else}/{endif}` and `{for}`).
  - Future "printer-profile import" packet (a JSON/INI loader for OrcaSlicer printer profiles would populate these four keys among many others).
- Activation blockers:
  - **No packet currently has `status: active`** (verified by grep `status: active` on `.ralph/specs/**/packet.spec.md` — zero matches; packets 56c, 57, 58 are all `status: draft`). This packet can activate immediately upon explicit user OK; no swap is needed. (Packet 58 `58_gcode-toolchange-purge-integration` is a peer-draft; its file surface — toolchange / purge integration — does not overlap with this packet's start/end emission file surface, so the two are independently activatable.)
  - **Goal §1 is invalidated and must be redesigned before activation.** The `crates/slicer-host/src/config_schema.rs::FullConfigSchema::default()` registration site that this packet depends on (parallel to the packet-55 `thumbnail_path` precedent) has been removed: `config_schema.rs` and the entire `FullConfigSchema` / `ConfigFieldSchema` / `validate_*` parallel hierarchy were deleted because no production code consumed them (the runtime path uses `slicer_ir::ResolvedConfig` and the per-key `FeedrateConfig::default()` in `gcode_emit.rs`). Before activating, this packet must pick a new home for the four host-level keys (`machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer`) — recommended options: (a) introduce a `MachineGcodeConfig` struct in `crates/slicer-host/src/gcode_emit.rs` with `impl Default` analogous to `FeedrateConfig`, or (b) have the CLI insert the defaults into `config_source` if no override is supplied. The AC at line 79 (`schema_registers_four_keys_with_expected_types_and_defaults`) and the AC at line 86 (`rejects_temp_out_of_registered_range`, which relies on the deleted `validate_config` range checker) must be rewritten against whichever home is chosen.

## Acceptance Criteria

- **Given** a `slicer-cli` invocation with no config overrides on a small fixture model, **when** the produced `out.gcode` is scanned, **then** the three lines `M190 S60`, `M109 S215`, and `PRINT_START EXTRUDER=215 BED=60` each appear exactly once. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_gcode_default_substitutes --nocapture`
- **Given** a `slicer-cli` invocation with no config overrides, **when** `out.gcode` is scanned, **then** `PRINT_END` appears exactly once (the default `machine_end_gcode`) and no other lines beginning with `PRINT_` are present. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- end_gcode_default_emits_print_end --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** byte offsets are compared, **then** the first byte-offset of the `M190 S60` line is strictly greater than the byte-offset of `HEADER_BLOCK_END` and strictly less than the byte-offset of the first `G1` line with a non-zero `E` token (the first extrusion move). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_block_position_after_header_before_first_g1 --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** byte offsets are compared, **then** the byte-offset of the `PRINT_END` line is strictly greater than the byte-offset of the last `G1` line and strictly less than the byte-offset of `CONFIG_BLOCK_START`. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- end_block_position_after_last_g1_before_config_block --nocapture`
- **Given** a `slicer-cli` invocation with `--config user.json` setting `machine_start_gcode = "G28 ; home all\nG1 Z5 F600"`, **when** `out.gcode` is scanned, **then** the two lines `G28 ; home all` and `G1 Z5 F600` each appear exactly once between `HEADER_BLOCK_END` and the first extrusion move, AND no `M190`, `M109`, or `PRINT_START` line appears anywhere in the file (the default has been fully replaced). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- user_override_replaces_default --nocapture`
- **Given** a `--config user.json` setting `bed_temperature_initial_layer_single = 65` and `nozzle_temperature_initial_layer = 220` (and the default `machine_start_gcode`), **when** `out.gcode` is scanned, **then** the start block contains the lines `M190 S65`, `M109 S220`, and `PRINT_START EXTRUDER=220 BED=65` (each exactly once) and contains no `S60`, `S215`, `EXTRUDER=215`, or `BED=60` substring. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- substitution_uses_overridden_temp_values --nocapture`
- **Given** a `--config user.json` setting `machine_end_gcode = ""` (empty string override), **when** `out.gcode` is scanned, **then** `PRINT_END` is absent from the file AND the byte range between the last `G1` line's terminating `\n` and `CONFIG_BLOCK_START` contains zero non-whitespace characters (no phantom block, no stray blank lines beyond one optional separator). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- empty_end_gcode_emits_no_block --nocapture`
- **Given** the host config schema is queried via `FullConfigSchema::default()`, **when** the keys `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, and `nozzle_temperature_initial_layer` are looked up, **then** each is registered with type (`String`, `String`, `Int`, `Int`) respectively and default value matching the Goal section (default start template literal, `"PRINT_END"`, `60`, `215`). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- schema_registers_four_keys_with_expected_types_and_defaults --nocapture`
- **Given** a default slicing run, **when** `CONFIG_BLOCK_START..CONFIG_BLOCK_END` is parsed, **then** each of `; machine_start_gcode = ...`, `; machine_end_gcode = PRINT_END`, `; bed_temperature_initial_layer_single = 60`, and `; nozzle_temperature_initial_layer = 215` appears exactly once (the four new keys flow through packet 55's CONFIG_BLOCK emission without further wiring). Multi-line `machine_start_gcode` value MAY be emitted on a single comment line with `\n` literalized as `\\n` OR via the existing packet-55 multi-line-value convention — the test asserts exact-key-presence and exact-value-equality against the registered default after un-escaping. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- new_keys_appear_in_config_block --nocapture`

## Negative Test Cases

- **Given** a `--config user.json` setting `machine_start_gcode = "TEMP [no_such_key] DONE"`, **when** the slicer is run, **then** the produced output contains the literal `TEMP [no_such_key] DONE` (passthrough — the unknown token is left exactly as written), AND a `log` record at WARN level is emitted whose formatted message contains the substring `unknown placeholder: no_such_key`. The test captures `log` output via a custom `log::Log` impl installed with `log::set_boxed_logger` that buffers records into a `Mutex<Vec<String>>` (~30 LOC; no new dependency — `log = "0.4"` is already in `crates/slicer-host/Cargo.toml:20`). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- unknown_placeholder_passes_through_with_warning --nocapture`
- **Given** a `--config user.json` setting `machine_start_gcode = "PREFIX [unclosed SUFFIX"` (no closing `]` on the same line), **when** the slicer is run, **then** the produced output contains the literal `PREFIX [unclosed SUFFIX` exactly as written, the substitution does not panic, does not infinite-loop, and does not emit a WARN log for this case (an unclosed bracket is not a "placeholder" — it is literal text). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- unclosed_bracket_treated_as_literal --nocapture`
- **Given** a `--config user.json` setting `nozzle_temperature_initial_layer = 999` (above registered max of 300), **when** `slicer-cli` is run, **then** it exits with a non-zero status code and stderr contains the substring `nozzle_temperature_initial_layer` AND a numeric reference to the violated bound (`300` or `range`). No `.gcode` output file is produced (or, if pre-existing, is not modified — assertion uses file mtime equality). | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- rejects_temp_out_of_registered_range --nocapture`
- **Given** the produced `out.gcode` from a default invocation, **when** scanned, **then** the literal substring `M190` does NOT appear inside the byte range `HEADER_BLOCK_START..HEADER_BLOCK_END`, does NOT appear after any `G1` line with `E` token (i.e., not after the first extrusion), and does NOT appear inside the byte range `CONFIG_BLOCK_START..CONFIG_BLOCK_END`. This negative case catches regression of any future serializer change that emits `M190` in the wrong band. | `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- start_block_not_inside_other_blocks --nocapture`

## Verification

- `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` — dispatch as FACT pass/fail; SNIPPETS (≤ 20 lines) on first-failing-assertion.
- `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — packet 55 regression; HEADER/THUMBNAIL/CONFIG block ordering must not break.
- `cargo test -p slicer-host --test gcode_emit_tdd` — packet 52/54 regression; M82/M83 preamble and per-role feedrate emission must not break.
- `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` — postpass pipeline regression.
- `cargo check --workspace`
- `cargo clippy --workspace -- -D warnings`

## Authoritative Docs

- `docs/01_system_architecture.md` — finalization stage, serializer role; delegate a SUMMARY (file is long).
- `docs/02_ir_schemas.md` — `ConfigView`, `ResolvedConfig`, `ConfigValue` enum; load directly only the ≤ 40-line `ConfigValue` enum section and the `ResolvedConfig` struct section. Delegate ranged lookups if uncertain.
- `docs/03_wit_and_manifest.md` — host-level vs module-level config schema, key validation; load directly only the schema-validation section.
- `docs/07_implementation_status.md` — DELEGATE every read. Step 1 of this packet appends TASK-193 / TASK-193a rows via a worker; the implementer must never load the full backlog file.
- `docs/14_deviation_audit_history.md` + `docs/DEVIATION_LOG.md` — no deviation expected; if a host-level config key registration requires one, file it here.

For each doc above: if > 300 lines, delegate. Default rule wins.

## OrcaSlicer Reference Obligations

All reads delegated; never load OrcaSlicer source into the implementer's context. The five FACTs below are the only OrcaSlicer evidence this packet needs.

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp:1243` and `:1288` — FACT, ≤ 6 lines: confirm field names are exactly `machine_end_gcode` and `machine_start_gcode` (snake_case, no `print_` prefix). Confirms our key naming.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:5599-5606` (start) and `:1927-1934` (end) — FACT, ≤ 8 lines: at `:5599` / `:1927` is the `def = this->add("machine_{start,end}_gcode", coString);` declaration; the actual default-value `set_default_value(new ConfigOptionString(...))` lives at `:5606` and `:1934` and contains the stock OrcaSlicer defaults (`"G28 ; home all axes\nG1 Z5 F5000 ; lift nozzle\n"` and `"M104 S0 ; turn off temperature\nG28 X0  ; home X axis\nM84     ; disable motors\n"`). We DELIBERATELY do not adopt OrcaSlicer's defaults — ours are user-specified Klipper PRINT_START / PRINT_END macros. Record this as an intentional deviation in `requirements.md`.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3181` (placeholder substitution) and `:3200` (start gcode write) and `:3258` (preamble after start gcode) — FACT, ≤ 12 lines: confirm OrcaSlicer's ordering is `machine_start_gcode` THEN preamble (G90/M83). Our serializer must match this ordering: substituted start block inserted BEFORE the existing packet-54 M82/M83 preamble.
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3544` — FACT, ≤ 6 lines: confirm end gcode is the final block before file close (in OrcaSlicer's flow it comes AFTER CONFIG_BLOCK; in our flow it comes BEFORE — this is an intentional difference because our CONFIG_BLOCK is emitted by `ThumbnailAwareSerializer` as a wrapper and must remain at file tail). Record as intentional deviation.
- `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp:164` (`apply_config()`) — FACT, ≤ 10 lines: confirm placeholder substitution sources config values from the same symbol table that holds `bed_temperature_initial_layer_single` etc. Our minimal helper mirrors this by accepting a `HashMap<String, ConfigValue>` view of the effective config. Do NOT read the full `process()` grammar implementation (`:2433`) — out of scope.

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

- `crates/slicer-host/src/gcode_emit.rs` is **1286 lines** (above the 600-line direct-read budget). Range-read `:626-:740` (HEADER + width + thumbnail; `serialize_header_block` at `:626`, `serialize_width_comments` at `:671`), `:887-:977` (CONFIG_BLOCK + `ThumbnailAwareSerializer` wrapper at `:932`; inner-serializer call at `:950`; THUMBNAIL/CONFIG append at `:953-974`), and `:979-:1166` (`serialize_gcode` body at `:980` + M82/M83 preamble at `:1026`/`:1028`). Never load the full file.
- `crates/slicer-host/src/config_schema.rs` is **1044 lines** (above the 600-line direct-read budget — range-read only, do NOT load full). New key registrations follow the existing `String` pattern at `:367-:387` (the `thumbnail_path` precedent) and the existing `Int`+range pattern at `:191-:212` (`fan_speed_min` with `min: Some(0.0), max: Some(255.0)`). Dispatch SNIPPETS for those two ranges plus ≤ 1 additional SNIPPETS to find the appropriate insertion section for the 4 new entries.
- `crates/slicer-ir/src/slice_ir.rs` is **1654 lines** (above the 600-line direct-read budget). Range-read `:433-:444` (`ConfigValue` enum) and `:618-:730` (`ResolvedConfig`; `extensions: HashMap<String, ConfigValue>` field at `:693`) only.
- `OrcaSlicerDocumented/` MUST be delegated. The packet's parity claims rest on the five FACT dispatches enumerated above and nothing else.
- `docs/07_implementation_status.md` is > 500 lines. NEVER load it directly. All reads and edits go through worker dispatches.

Sub-agent return formats:

- OrcaSlicer FACTs (5 dispatches above): ≤ 12 lines each, no code blocks > 4 lines.
- `cargo test`: FACT pass/fail; SNIPPETS (≤ 20 lines) on first-failing-assertion.
- Schema-registration completeness check (end of Step 3): LOCATIONS list of every registered key matching `machine_*` or `*_temperature_initial_layer*`, ≤ 8 entries.
- `serialize_gcode()` insertion-point lookup (Step 4): SNIPPETS (≤ 30 lines) of the two byte ranges where start/end blocks are inserted.

### Test Fixture Convention

- This packet's tests reuse the same small `.stl` fixture used by `gcode_emit_tdd.rs` and `gcode_header_thumbnail_config_blocks_tdd.rs` (resolve the path via `concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/<fixture>.stl")` — confirm the exact filename via a single LOCATIONS dispatch against the predecessor test files in Step 2). NO new STL fixture is created.
- The `--config user.json` overrides used in ACs are materialized inline at test runtime via `std::env::temp_dir()` + `serde_json::to_writer`. NO new committed config fixtures.

### Test Discipline Reminder

Per `CLAUDE.md` / Test Discipline: `cargo test --workspace` is FORBIDDEN as a per-AC verification command in this packet. Every AC above uses the targeted `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- <test_name> --nocapture` form. The packet-level workspace gate appears only at the closure ceremony in `implementation-plan.md` Step 6 and is dispatched to a sub-agent that returns FACT pass/fail.

Aggregate context cost: M. No step is L. If implementation reveals that the start-block insertion point in `serialize_gcode()` is not a clean injection (e.g., requires refactoring the preamble emission path), surface as a packet-local risk in `design.md` Open Questions and split that work into a follow-up packet rather than expanding this packet's scope.
