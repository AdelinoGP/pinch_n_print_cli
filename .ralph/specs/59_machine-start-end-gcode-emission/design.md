# Design: 59_machine-start-end-gcode-emission

## Controlling Code Paths

- **Primary code path:**
  - `crates/slicer-host/src/gcode_emit.rs::DefaultGCodeSerializer::serialize_gcode()` (`:979-:1166`) — the function that produces the final G-code text from `GCodeIR`. This is where the substituted start block (after HEADER + width comments) and end block (after the last layer's commands) are injected.
  - `crates/slicer-host/src/gcode_emit.rs::serialize_header_block()` (`:626-:658`) and `serialize_width_comments()` (`:671-:687`) — packet-55 helpers; **not modified**, but their position defines the byte boundary AFTER which the start block is inserted.
  - `crates/slicer-host/src/gcode_emit.rs::ThumbnailAwareSerializer::serialize_gcode()` (`:926-:977`) — packet-55 wrapper that injects THUMBNAIL_BLOCK and CONFIG_BLOCK; **not modified**, but its position defines the byte boundary BEFORE which the end block is inserted. The wrapper is structurally separate from `DefaultGCodeSerializer`, so the end-block insertion happens INSIDE `DefaultGCodeSerializer::serialize_gcode()` (the inner call), before its return — the wrapper then appends THUMBNAIL/CONFIG on top of whatever the inner call returned.
  - `crates/slicer-host/src/config_schema.rs::FullConfigSchema::default()` (`:128-:393`) — the host-level config registry. Four new keys added here following the `thumbnail_path` precedent at `:367-:387` and the existing `Int` registration pattern.
  - **NEW helper:** `crates/slicer-host/src/gcode_emit.rs::substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` — a private free function in the same file. Single-pass `[snake_case_key]` substitution against `lookup`; `log::warn!` on unknown keys (uses existing `log = "0.4"` dep at `crates/slicer-host/Cargo.toml:20`; no new dependency); literal passthrough of `[` without matching `]` on the same line. ≤ 60 LOC including a deduplicated unknown-key set.

- **Neighboring tests or fixtures:**
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` — the packet-55 fixture pattern: small STL fixture, slicer invocation, output scan against literal sentinels. This new packet's test file mirrors that structure exactly.
  - `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` — layer fixture pattern with M104/M109 emission (used as a scaffolding reference; the new test installs its own custom `log::Log` impl for WARN capture since `gcode_emit_tdd` does not currently exercise log capture).
  - `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs` — postpass pipeline contract regression target.

- **OrcaSlicer comparison surface:**
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp:1243` / `:1288` — confirms key names.
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp:5599` / `:1927` — stock defaults (intentionally not borrowed).
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3181` / `:3200` / `:3258` — start-block-then-preamble ordering (borrowed).
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3544` — end-block-after-CONFIG ordering (intentionally not borrowed; explained under Risks and Tradeoffs).
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp:164` (`apply_config()` symbol-table contract) — borrowed.

## Architecture Constraints

- All config keys must use snake_case literals (per `CLAUDE.md` Config Key Naming Convention). The four new keys (`machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer`) all already conform.
- Host-level config keys live in `crates/slicer-host/src/config_schema.rs::FullConfigSchema::default()`, NOT in any core-module `.toml` manifest. Precedent: `thumbnail_path` (packet 55).
- The substituted start block must appear BEFORE the M82/M83 preamble (packet-54 invariant: preamble immediately precedes layer 0; we slot in BEFORE the preamble, after HEADER/width). This matches OrcaSlicer ordering.
- The substituted end block must appear BEFORE the THUMBNAIL_BLOCK + CONFIG_BLOCK footer emitted by `ThumbnailAwareSerializer`. The wrapper is opaque from `DefaultGCodeSerializer`'s perspective; the inner serializer simply appends the end block at the end of its own output, and the wrapper concatenates THUMBNAIL/CONFIG after that.
- The four new config keys flow into CONFIG_BLOCK automatically via packet-55's `serialize_config_block()` because that helper iterates the effective `ConfigView`. No explicit CONFIG_BLOCK wiring is needed in this packet (and explicitly is out of scope).
- The substitution helper is a private free function, NOT a public API. It is internal to `gcode_emit.rs`. If a future packet (arithmetic, conditionals) needs it from elsewhere, that future packet may promote it; this packet does not.
- The helper performs ONE pass. A substituted value containing `[other_key]` is NOT re-scanned. This avoids non-termination (a template could otherwise reference itself via a chain) and matches the principle of least surprise for a minimal engine.
- Empty / whitespace-only substituted block ⇒ emit nothing. No header comment line, no blank line, no phantom sentinel. This is what makes the default `machine_end_gcode = "PRINT_END"` distinguishable from a user-set `machine_end_gcode = ""`.
- No IR changes. No WIT changes. No new module. No new crate.

## Code Change Surface

- **Selected approach:** Host-level config registration + private substitution helper + two new insertion points in the existing `DefaultGCodeSerializer::serialize_gcode()`. Minimal blast radius. Matches the architectural mode of packets 54 and 55.

- **Exact functions, traits, manifests, tests, or fixtures expected to change:**
  1. `crates/slicer-host/src/config_schema.rs::FullConfigSchema::default()` — add four new `ConfigSchemaEntry` registrations (two `String`, two `Int` with range validators).
  2. `crates/slicer-host/src/gcode_emit.rs::substitute_placeholders()` — NEW private free function in the same module. ≤ 60 LOC.
  3. `crates/slicer-host/src/gcode_emit.rs::DefaultGCodeSerializer::serialize_gcode()` — two new insertion points:
     - After HEADER + width comments emission (around `:687`), before the preamble emission. Calls `substitute_placeholders(machine_start_gcode_template, &config_lookup)` and appends with trailing `\n` if non-empty.
     - After the last layer's commands, before the function returns. Same call pattern for `machine_end_gcode_template`. The `ThumbnailAwareSerializer` wrapper then appends THUMBNAIL/CONFIG_BLOCK on top.
  4. `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` — NEW test file. 9 positive + 4 negative tests; reuses the predecessor STL fixture and the same `slicer_cli` invocation harness used by the packet-55 tests.
  5. `docs/07_implementation_status.md` — append two rows for TASK-193 and TASK-193a in the appropriate queued/in-progress section. Edit via worker dispatch; never load the full file into the implementer's context.

- **Rejected alternatives:**
  - **A. Add as a new core-module (e.g., `print-start-end-gcode/print-start-end-gcode.toml` with a `LayerFinalization` or `GCodePostProcess` hook).** Rejected: start/end gcode is a print-level (not layer-level) concern, has no WASM-isolated logic worth modularizing, would require defining a new IR view for "print boundary events", and would put a heavyweight wrapper around a one-line config + a one-pass `replace` loop. The thumbnail_path precedent (packet 55) is the right reference, not the cooling-fan / skirt-brim module precedent.
  - **B. Use a `TextPostProcess` module to inject the blocks after serialization.** Rejected: the byte-offset relationship to HEADER_BLOCK_END and to the first `G1` line is contractual and must be guaranteed by the serializer, not by a post-process pass that could see arbitrary input. Doing it in the serializer means the position is structurally correct by construction.
  - **C. Implement the full OrcaSlicer `PlaceholderParser` grammar in one packet.** Rejected at the scope-approval gate (~2400 LOC of Boost.Spirit → Rust port; expressions, conditionals, loops, builtins, type coercion). Tracked as three follow-up packets.
  - **D. Use `{var}` brace syntax in addition to `[key]`.** Rejected: the user's specified default uses only `[key]`. Adding brace syntax now invites future-coupling with conditional `{if}...{endif}` which has different parse semantics; cleaner to add both in the same future packet that introduces conditionals.
  - **E. Re-scan substituted values for further placeholders (recursive substitution).** Rejected: enables non-termination (template references itself transitively), surprises users by silently chaining, and is not in the user's specified default. One-pass is sufficient and predictable.
  - **F. Adopt OrcaSlicer's stock defaults.** Rejected per user direction: the defaults are Klipper PRINT_START / PRINT_END macros. Recorded as an intentional deviation.

## Files in Scope (read + edit)

Primary edit targets (≤ 3):

- `crates/slicer-host/src/gcode_emit.rs` — role: host serializer + new `substitute_placeholders` helper; expected change: add ≤ 60-LOC private function + two ≤ 10-LOC insertion blocks in `DefaultGCodeSerializer::serialize_gcode()`. Total addition ≤ 100 LOC.
- `crates/slicer-host/src/config_schema.rs` — role: host config registry; expected change: register four new entries (two `String`, two `Int` with range validators). Total addition ≤ 40 LOC.
- `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` — role: new TDD test file; expected change: ≤ 400 LOC across 13 tests (9 positive + 4 negative), helper fixture loader, custom `log::Log` capture helper (~30 LOC) using the existing `log = "0.4"` dep.

Secondary edit (worker dispatch only):

- `docs/07_implementation_status.md` — role: backlog; expected change: append two rows (TASK-193, TASK-193a). NEVER loaded into the implementer's context; all reads/edits via worker dispatch.

## Read-Only Context

Files the implementer is allowed to read but not edit. Range-read when > 300 lines.

- `crates/slicer-host/src/gcode_emit.rs:626-:740` — HEADER + width + thumbnail helpers; purpose: confirm the byte-offset boundary AFTER which the start block is inserted.
- `crates/slicer-host/src/gcode_emit.rs:887-:977` — CONFIG_BLOCK + `ThumbnailAwareSerializer`; purpose: confirm that the end block is emitted by the INNER serializer (before the wrapper's THUMBNAIL/CONFIG_BLOCK append).
- `crates/slicer-host/src/gcode_emit.rs:979-:1166` — `DefaultGCodeSerializer::serialize_gcode()` body + preamble emission; purpose: identify both insertion points by inspection.
- `crates/slicer-ir/src/slice_ir.rs:433-:444` — `ConfigValue` enum (`Bool`, `Int`, `Float`, `String`, `List`); purpose: how to stringify a value for substitution.
- `crates/slicer-ir/src/slice_ir.rs:618-:730` — `ResolvedConfig` struct (extensions: `HashMap<String, ConfigValue>`); purpose: how to look up the four new keys at substitute time.
- `crates/slicer-host/src/config_schema.rs` (file is **1044 lines** — ABOVE the 600-line direct-read budget; range-read only, do NOT load full) — purpose: existing `thumbnail_path` String registration at `:367-:387` (range-read) and existing `Int`+range pattern at `:191-:212` (`fan_speed_min` with `min: Some(0.0), max: Some(255.0)`, range-read). Dispatch SNIPPETS for both ranges.
- `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full file) — purpose: TDD scaffolding pattern (slicer invocation, output scan, sentinel substring assertions); the new test file mirrors this structure.
- `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` — purpose: layer fixture + M104/M109 capture pattern.
- `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md` (already loaded in generation context) — purpose: row-formatting precedent for docs/07 edit dispatch.

## Out-of-Bounds Files

Files the implementer must NOT load directly. The implementer should delegate any fact-checks against this list.

- `OrcaSlicerDocumented/**` — delegate ALL parity checks; the five FACT dispatches enumerated in `packet.spec.md` are the only OrcaSlicer evidence this packet needs. In particular, NEVER read `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (~2400 lines of Boost.Spirit grammar; out of scope; will trash context budget).
- `target/`, `Cargo.lock`, generated WASM under `modules/core-modules/*/dist/` — never load.
- Vendored deps — never load.
- `docs/07_implementation_status.md` — > 500 lines; never load full; worker dispatch only.
- `docs/02_ir_schemas.md` — > 300 lines; range-read only at the two ranges listed above.
- `crates/slicer-host/src/gcode_emit.rs` — > 1100 lines; range-read at the three ranges listed above.
- `crates/slicer-ir/src/slice_ir.rs` — > 1600 lines; range-read at the two ranges listed above.
- `modules/core-modules/**` — irrelevant to this packet (no module owns this work). Do NOT browse manifest patterns there; the `thumbnail_path` precedent in `config_schema.rs` is the correct reference.
- All other crates (`slicer-cli`, `slicer-helpers`, etc.) beyond the change surface — delegate trait/impl lookups; do not browse.
- All other packets (`.ralph/specs/01*` through `57_*`) other than packet 55 — delegate any pattern checks.

## Expected Sub-Agent Dispatches

Implementers should plan for at least the following dispatches. List is not exhaustive but covers the predictable ones.

- **Step 1 dispatches:**
  - "In `docs/07_implementation_status.md`, find the line range where queued / in-progress G-code-output TASK entries live (proximity to TASK-184 / TASK-185 / TASK-191 / TASK-192a). Return LOCATIONS, ≤ 5 entries, each with the adjacent row's verbatim text. Do not return the rest of the file." — purpose: find insertion point.
  - "Append two rows to `docs/07_implementation_status.md` after `<insertion-point-line>`: TASK-193 (`Emit configurable machine_start_gcode / machine_end_gcode with [key] placeholder substitution`) and TASK-193a (`Register machine_start_gcode, machine_end_gcode, bed_temperature_initial_layer_single, nozzle_temperature_initial_layer in FullConfigSchema`). Both status `[ ]` queued. Match the row format of the adjacent rows. Return FACT: bytes appended." — purpose: edit.
  - "`grep -n 'TASK-193' docs/07_implementation_status.md` — return FACT (hits = 2 expected)." — purpose: verification.
- **Step 2 dispatches:**
  - "In `crates/slicer-host/tests/`, find the small STL fixture path used by `gcode_header_thumbnail_config_blocks_tdd.rs` and `gcode_emit_tdd.rs`. Return FACT: `concat!(env!('CARGO_MANIFEST_DIR'), '/../../resources/<filename>.stl')` exact filename." — purpose: reuse fixture.
  - "Confirm `log = \"0.4\"` is in `crates/slicer-host/Cargo.toml` (expected at :20) and that no other slicer-host test installs a global logger via `log::set_boxed_logger` or `log::set_logger` that would conflict with the new test's logger install. If a conflicting global logger exists, return its install site (file:line) so the new test can use a thread-local capture workaround. Return FACT (≤ 6 lines)." — purpose: WARN-log capture mechanism confirmation.
- **Step 3 dispatches:**
  - "From `crates/slicer-host/src/config_schema.rs`, return SNIPPETS (≤ 25 lines) of: (a) the `thumbnail_path` String registration (≈`:367-:387`); (b) one existing Int registration with a range validator (any cooling-int key)." — purpose: copy the registration pattern.
  - "Run `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- schema_registers_four_keys_with_expected_types_and_defaults --nocapture`. Return FACT (pass) or SNIPPETS (≤ 20 lines) on fail." — purpose: validate Step 3.
- **Step 4 dispatches:**
  - "From `crates/slicer-host/src/gcode_emit.rs`, return SNIPPETS (≤ 30 lines) of the exact insertion site after `serialize_header_block` + `serialize_width_comments` calls within `DefaultGCodeSerializer::serialize_gcode()` body. Cite file:line." — purpose: locate start-block insertion.
  - "From `crates/slicer-host/src/gcode_emit.rs`, return SNIPPETS (≤ 30 lines) of the inner serializer's return point (the last `push_str` / `writeln!` before the function returns its accumulated buffer). Cite file:line." — purpose: locate end-block insertion.
  - "Run `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` — return FACT (pass/fail) and on fail SNIPPETS of the first failing test name + ≤ 15 lines context." — purpose: AC turn-green.
  - "Run `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` — return FACT pass/fail." — purpose: packet-55 regression.
  - "Run `cargo test -p slicer-host --test gcode_emit_tdd` — return FACT pass/fail." — purpose: packet-52/54 regression.
- **Step 5 dispatches:**
  - "Run `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` — return FACT." — purpose: postpass regression.
  - "Run `cargo check --workspace` — return FACT pass/fail; SNIPPETS (≤ 30 lines) of first error on fail." — purpose: workspace gate.
  - "Run `cargo clippy --workspace -- -D warnings` — return FACT pass/fail; SNIPPETS (≤ 30 lines) of first warning on fail." — purpose: lint gate.
- **Step 6 dispatches:**
  - Re-dispatch every pipe-suffixed AC command from `packet.spec.md` (13 total) — each as FACT pass/fail.
  - "Run `cargo test --workspace` once at closure ceremony — return FACT pass/fail; on fail SNIPPETS (≤ 40 lines) of the first failing test name + assertion. NEVER return the full test output." — purpose: final closure gate per CLAUDE.md test discipline.
  - "Update `docs/07_implementation_status.md` rows for TASK-193 / TASK-193a to status `[x]`; return FACT: rows updated." — purpose: backlog close.

## Data and Contract Notes

- **IR or manifest contracts touched:** NONE. `ConfigValue` enum is consumed (read-only); no new variants. `ResolvedConfig.extensions` is consumed (read-only) as the substitution lookup surface; no new fields. `GCodeIR`, `LayerCollectionIR`, `SliceIR`, `MeshIR` — untouched.
- **WIT boundary considerations:** NONE. No WIT contracts modified. No new module-to-host boundary crossings. The substitution helper and the four config keys live entirely host-side.
- **Determinism or scheduler constraints:** The substituted block must be deterministic given the same effective `ConfigView`. The `substitute_placeholders()` helper iterates the template left-to-right; HashMap key lookup is value-equality and does not introduce ordering nondeterminism. The unknown-key WARN log deduplication uses a `HashSet<String>` accumulated within the call; if the same unknown key appears N times in the template, we emit one WARN and pass through N times — this is the documented contract.
- **CONFIG_BLOCK propagation:** Packet 55's `serialize_config_block()` iterates the effective `ConfigView` and emits `; <key> = <value>` per key. The four new keys flow through automatically once registered. AC `new_keys_appear_in_config_block` verifies this propagation but no additional CONFIG_BLOCK wiring code is added by this packet.
- **Multi-line `String` value handling in CONFIG_BLOCK:** The default `machine_start_gcode` value contains `\n` characters. Packet 55's CONFIG_BLOCK formatter MAY emit this as a single comment line with `\n` literalized as `\\n`, OR via a multi-line convention if packet 55 defined one. AC `new_keys_appear_in_config_block` asserts on `key = value` equality after un-escaping; the exact wire format is whatever packet 55 already does (one targeted SNIPPETS dispatch in Step 4 confirms which).

## Locked Assumptions and Invariants

- The implementer must preserve the byte-offset ordering established by packets 54 / 55:
  - `HEADER_BLOCK_START` < width comments < (NEW: substituted start block) < M82/M83 preamble < first `;LAYER_CHANGE` < first `G1` extrusion move.
  - last `G1` extrusion move < (NEW: substituted end block) < `THUMBNAIL_BLOCK_START` (if present) < `CONFIG_BLOCK_START` < `CONFIG_BLOCK_END` < EOF.
- `substitute_placeholders()` is single-pass — substituted values are not re-scanned.
- `substitute_placeholders()` does not panic, does not infinite-loop, and does not allocate unboundedly. For an N-byte template with K placeholders, runtime is O(N + K · avg-key-length) and allocation is one output `String` of ≤ N + K · max-value-length bytes.
- Empty/whitespace-only substituted block ⇒ emit zero bytes (no header comment, no blank line). A user explicitly setting `machine_end_gcode = ""` must produce a file structurally identical (modulo CONFIG_BLOCK's listing of the empty value) to a file produced with the empty default — i.e., NO end block, NO phantom marker.
- `log::warn!` is the WARN-emission mechanism (the `log = "0.4"` crate is already in `crates/slicer-host/Cargo.toml:20`; `tracing` is NOT a workspace dependency — verified by `grep '^tracing' **/Cargo.toml` returning zero matches). The log target defaults to `module_path!()` (`slicer_host::gcode_emit`); the message format is exactly `unknown placeholder: {key}` with `{key}` being the unresolved snake_case identifier.
- The four new config keys' defaults are EXACTLY as specified in `packet.spec.md` Goal. Any deviation requires an explicit user OK and a packet revision.

## Risks and Tradeoffs

- **Risk: end-block position differs from OrcaSlicer.** OrcaSlicer emits `machine_end_gcode` AFTER its CONFIG_BLOCK (`OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3544`). We emit it BEFORE THUMBNAIL/CONFIG_BLOCK because our CONFIG_BLOCK is structurally a footer wrapper. **Mitigation:** comments after the last printable command are ignored by all firmwares; printers do not execute `; CONFIG_BLOCK_START`. The difference is transparent at the wire level. Documented as an intentional deviation in `requirements.md`; if a future "byte-for-byte parity" packet needs the OrcaSlicer ordering, restructuring `ThumbnailAwareSerializer` will be a separate concern.
- **Risk: multi-line `machine_start_gcode` value in CONFIG_BLOCK may be unparseable by some downstream tools.** Packet 55's CONFIG_BLOCK formatter handles multi-line values somehow (single line with `\\n`, or multi-line with `; ` per continuation). Whichever convention it picked applies. **Mitigation:** Step 4 dispatch confirms the exact wire format; AC `new_keys_appear_in_config_block` validates round-trip via un-escaping.
- **Risk: `log` capture mechanism in test.** The `log` crate provides no built-in test capture, and `tracing` is NOT a workspace dependency. The test installs a custom `log::Log` impl (~30 LOC) via `log::set_boxed_logger`, which buffers `Record`s into a `Mutex<Vec<String>>` that the test reads after the slicer run. **Mitigation:** the custom-Log approach uses only the existing `log = "0.4"` dep; no new dependency is needed. Step 2 dispatch confirms the dep is present and that no other slicer-host test already installs a global logger that would conflict; if such a conflict exists, the test uses a thread-local capture workaround inline (≤ 10 additional LOC).
- **Risk: future "arithmetic" packet breaks one-pass invariant.** Adding `[bed*2]` evaluation means substituted output could contain identifiers that look like further placeholders. **Mitigation:** scope boundary is clear; future packet decides whether to re-scan or to extend `substitute_placeholders` into a tokenizer; this packet does NOT need to anticipate it.
- **Tradeoff: minimal substitution vs full OrcaSlicer parity.** Accepted at scope-approval gate. Users who need conditionals can either wait for the follow-up packet or write the literal expansion themselves.
- **Tradeoff: host-level config registration vs core-module.** Accepted: matches the `thumbnail_path` precedent and avoids inventing a "print boundary" IR view.

## Context Cost Estimate

- Aggregate (sum across all steps): **`M`**.
- Largest single step: Step 4 (implement helper + wire two insertion points + turn 13 ACs green) at `M`.
- Highest-risk dispatch: the `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` failure return. **Required return format:** FACT pass / SNIPPETS ≤ 20 lines (test name + assertion + minimal context). NEVER return the full test runner output (default tests are ≈ 13 cases × ≈ 100-line G-code files; full failure dump would consume thousands of lines).

## Open Questions

All open questions resolved at the scope-approval gate. **No remaining blockers for activation** — no packet is currently `status: active` (see Activation gating below), and the two non-blocking confirmations below are tactical and resolve during Step 2 / Step 4 dispatches:

- **Activation gating:** **No packet currently has `status: active`** (verified by grep `status: active` on `.ralph/specs/**/packet.spec.md` — zero matches; packets 56c, 57, 58 are all `status: draft`). This packet can activate immediately upon explicit user OK. (Packet 58 `58_gcode-toolchange-purge-integration` is a peer-draft; its file surface — toolchange/purge integration — does not overlap with this packet's start/end emission and CONFIG_BLOCK propagation paths, so the two are independently activatable.)
- **Non-blocking confirmation (Step 4):** Whether the inner `DefaultGCodeSerializer::serialize_gcode()` end-point is unambiguously identifiable, or whether the implementation needs a small refactor to expose a clean injection point. If a refactor is needed, surface it as a packet-local risk and split into a follow-up packet rather than expanding scope. (This is a small-probability risk; the existing serializer has a clear final-buffer-return shape per the packet-55 design.)
- **Non-blocking confirmation (Step 2):** Whether any other slicer-host test already installs a global `log::set_boxed_logger` / `log::set_logger` that would conflict with the new test's logger install. If a conflict exists, use a thread-local capture workaround (≤ 10 extra LOC). The `log = "0.4"` dep itself is already present at `crates/slicer-host/Cargo.toml:20` — no new dependency.

Neither of the non-blocking confirmations changes scope, interface, or verification strategy — they are tactical implementation choices. They do NOT block activation; they are resolved during Step 2 / Step 4 dispatches.
