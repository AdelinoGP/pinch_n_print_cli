# Implementation Plan: 59_machine-start-end-gcode-emission

## Execution Rules

- One atomic step at a time.
- Each step must map back to TASK-193, TASK-193a, or TASK-193b.
- TDD first (Step 2), then WIT/SDK/dispatch/IR extension (Step 3), then create the module with real `run_finalization` (Step 4), then wire the serializer (Step 5), then narrowest falsifying validation (Step 6), then completion gate (Step 7).
- Each step honors the context-discipline preamble. The fields below are not optional metadata — they are the budget contract for this step.
- The implementer MUST stop reading at 60% context and hand off at 85%.
- The implementer MUST delegate every `cargo` invocation, every OrcaSlicer / docs/07 access, and every guest-wasm build.

## Steps

### Step 1: Append TASK-193 / TASK-193a / TASK-193b rows to docs/07

- Task IDs:
  - `TASK-193`
  - `TASK-193a`
  - `TASK-193b`
- Objective: Add three queued backlog rows so the packet's task IDs exist before any code change.
- Precondition: None of TASK-193, TASK-193a, TASK-193b appears in `docs/07_implementation_status.md`.
- Postcondition: Three rows present, each with status `[ ]` (queued), placed near other in-progress / queued G-code-output tasks. Row texts:
  - TASK-193 — "Emit configurable `machine_start_gcode` / `machine_end_gcode` at correct serializer byte offsets via two new `FinalizationBuilderPush` variants."
  - TASK-193a — "Create `modules/core-modules/machine-gcode-emit/` declaring four `[config.schema.*]` entries; module's `run_finalization` performs real `[key]` substitution against `ResolvedConfig` and pushes resolved strings via the new variants."
  - TASK-193b — "Extend `wit/world-finalization.wit` (`finalization-output-builder`), SDK trait (`FinalizationBuilderPush` + `FinalizationOutputBuilder` impl), dispatch routing, and `GCodeIR` with print-boundary variants."
- Files allowed to read:
  - `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md:3-6` — TASK-184 / TASK-185 row-format precedent.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` — **edit via worker dispatch only**. Implementer never loads the full file.
- Files explicitly out-of-bounds for this step:
  - The rest of `docs/07_implementation_status.md` — only the insertion-point LOCATIONS dispatch return is admitted into context.
  - All source crates (this step is docs-only).
- Expected sub-agent dispatches:
  - `Q: "In docs/07_implementation_status.md, find the line range where queued / in-progress G-code-output TASK entries live (proximity to TASK-184 / TASK-185 / TASK-191 / TASK-192a). Return LOCATIONS, ≤ 5 entries, each with the adjacent row's verbatim text. Do not return the rest of the file." | Scope: docs/07_implementation_status.md | Return format: LOCATIONS`
  - `Q: "Append three rows to docs/07_implementation_status.md immediately after line <insertion-line>: TASK-193, TASK-193a, TASK-193b (texts above). All status [ ] queued. Match the row format of the adjacent rows verbatim. Return FACT: bytes appended + line numbers of the three new rows." | Scope: docs/07_implementation_status.md | Return format: FACT`
  - `Q: "Run 'grep -n TASK-193 docs/07_implementation_status.md' and return FACT (expected: exactly 3 hits — TASK-193, TASK-193a, TASK-193b)." | Scope: docs/07_implementation_status.md | Return format: FACT`
- Context cost: `S`.
- Authoritative docs:
  - `docs/07_implementation_status.md` — delegate every interaction.
- OrcaSlicer refs: none.
- Verification:
  - The final `grep -n` FACT returns exactly 3 hits.
- Exit condition: TASK-193, TASK-193a, TASK-193b present as queued rows in `docs/07_implementation_status.md`.

### Step 2: Write red TDD test file with all 12 failing assertions

- Task IDs:
  - `TASK-193`
  - `TASK-193a`
  - `TASK-193b`
- Objective: Materialize all 9 positive + 3 negative acceptance criteria as failing Rust tests in a new file. The test exercises the END-TO-END pipeline: `slicer-cli` invocation → `ResolvedConfig` → module reads keys → module substitutes → module pushes via new variants → dispatch routes to `GCodeIR` fields → serializer emits at correct positions.
- Precondition: `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` does not exist; Step 1 complete.
- Postcondition: New test file present; the file compiles; all 12 tests are present by name and all 12 FAIL (red state). No test is `#[ignore]`. No production code modified.
- Files allowed to read:
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full file) — packet-55 TDD scaffolding pattern.
  - `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` — layer fixture + M104/M109 capture pattern.
  - `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80` — slicer-cli invocation harness pattern.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` (NEW).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/**`, `crates/slicer-ir/src/**`, `crates/slicer-sdk/src/**`, `wit/**`, `modules/core-modules/**` — no production code change in this step.
- Expected sub-agent dispatches:
  - `Q: "In crates/slicer-host/tests/, find the small STL fixture path used by gcode_header_thumbnail_config_blocks_tdd.rs and gcode_emit_tdd.rs. Return FACT: the exact 'concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../resources/<filename>.stl\")' literal string." | Scope: crates/slicer-host/tests/ | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -50' after the test file is written. Return FACT: 'all 12 tests FAILED' or SNIPPETS (≤ 20 lines) showing any test that PASSED or was IGNORED unexpectedly." | Scope: workspace | Return format: FACT`
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` `ConfigValue` enum (range read at `:550-:580`).
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -50` returns ALL 12 FAILED.
- Exit condition: 12 tests defined by name, all failing, test file compiles cleanly.

### Step 3: Extend WIT + SDK + dispatch + IR (additive)

- Task IDs:
  - `TASK-193b`
  - (also exercises `TASK-193` plumbing)
- Objective: Make the new print-boundary push channel exist as a typed contract. Add two methods on `wit/world-finalization.wit`'s `finalization-output-builder` resource; add two variants on `FinalizationBuilderPush` (host enum) + two methods on `FinalizationOutputBuilder` impl (SDK); add two match arms in the dispatch apply-site loop; add two `Option<String>` fields on `GCodeIR`. All changes are ADDITIVE — no existing item removed or changed.
- Precondition: Step 2 complete; 12 tests red.
- Postcondition: `cargo build --tests` clean; `FinalizationBuilderPush` has 8 variants; `GCodeIR` has 5 fields (3 existing + 2 new `Option<String>`); the apply-site loop has 8 match arms; SDK `FinalizationOutputBuilder` exposes `push_print_start_gcode` / `push_print_end_gcode`. No AC turns green yet — the module that produces the pushes does not exist until Step 4 and the serializer does not consume the GCodeIR fields until Step 5.
- Files allowed to read:
  - `wit/world-finalization.wit` (full — ≤ 130 lines).
  - `crates/slicer-sdk/src/traits.rs:700-:730` and `:1196-:1230` (range).
  - `crates/slicer-host/src/wit_host.rs:830-:895` and `:4895-:5020` (range).
  - `crates/slicer-host/src/dispatch.rs:1070-:1100` and `:2885-:2980` (range).
  - `crates/slicer-ir/src/slice_ir.rs:1779-:1799` (range).
- Files allowed to edit (≤ 5):
  - `wit/world-finalization.wit`
  - `crates/slicer-sdk/src/traits.rs`
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-ir/src/slice_ir.rs`
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs` — Step 5 wires the serializer; this step only adds the typed contract.
  - `modules/core-modules/**` — Step 4 creates the new module.
  - The rest of every read-restricted file outside the listed ranges.
- WIT/Type Changes Checklist (CLAUDE.md, MANDATORY for this step):
  - Sub-step (a): Grep all `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for any reference to `FinalizationBuilderPush` or the `finalization-output-builder` resource. List every site so the new methods/variants mirror the existing add/match patterns.
  - Sub-step (b): Verify type identity across component boundaries — the `text: string` argument on the new WIT methods MUST come through as `String` on the SDK side and the same `String` on the host-enum `PrintStartGcode(String)` variant. No type drift.
  - Sub-step (c): Run `cargo build --tests` after the WIT addition (this triggers wit-bindgen regeneration).
  - Sub-step (d): If there are external WIT package references (e.g., a `wit/deps/` mirror), update them consistently. Inline `wit/world-finalization.wit` is the canonical source.
- Expected sub-agent dispatches:
  - `Q: "Run 'grep -rn FinalizationBuilderPush crates/ test-guests/ modules/' and 'grep -rn finalization-output-builder wit/ crates/ test-guests/ modules/'. Return FACT: every match path:line, ≤ 20 lines total." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo build --tests' after the WIT/SDK/dispatch/IR additions. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first error on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'grep -c PrintStartGcode crates/slicer-host/src/wit_host.rs' (expected ≥ 2: enum variant declaration + bridge push site) and 'grep -c PrintEndGcode crates/slicer-host/src/wit_host.rs' (expected ≥ 2). Return FACT." | Scope: crates/slicer-host/src/wit_host.rs | Return format: FACT`
- Context cost: `M`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — WIT extension protocol; range-read only the additive-only paragraph.
  - `docs/05_module_sdk.md` — `FinalizationOutputBuilder` API patterns; range-read.
- OrcaSlicer refs: none for this step.
- Verification:
  - `cargo build --tests` clean.
  - `FinalizationBuilderPush` has 8 variants (was 6).
  - `GCodeIR` has 5 fields (was 3).
- Exit condition: All four `cargo build --tests` / grep dispatches return FACT pass; no existing test regression introduced (regression suites still pass — verified in Step 6, not here).

### Step 4: Create `modules/core-modules/machine-gcode-emit/` with real `run_finalization`

- Task IDs:
  - `TASK-193a`
- Objective: Create the new core module that performs real `[key]` substitution and pushes resolved strings through the new `FinalizationOutputBuilder` methods. Three files: `machine-gcode-emit.toml` (manifest), `Cargo.toml` (Rust manifest mirroring part-cooling), `src/lib.rs` (~120-150 LOC including a private `substitute_placeholders` helper). Build via `./modules/core-modules/build-core-modules.sh`; confirm `--check` clean. Confirm `./test-guests/build-test-guests.sh --check` clean (the Step 3 WIT change invalidates test-guest bindgen).
- Precondition: Step 3 complete; `cargo build --tests` green; `FinalizationBuilderPush` has 8 variants.
- Postcondition: New module folder present with three files; module builds to `.wasm`; both `--check` commands return clean; targeted ACs `module_manifest_registers_four_keys_with_expected_types_and_defaults` and `new_keys_appear_in_config_block` turn green (the latter via packet-55's automatic CONFIG_BLOCK propagation). The remaining 10 ACs are still red — substitution happens but the serializer does not yet read the new `GCodeIR` fields.
- Files allowed to read:
  - `modules/core-modules/part-cooling/part-cooling.toml` (full — ≤ 100 lines).
  - `modules/core-modules/part-cooling/Cargo.toml` (full — ≤ 25 lines).
  - `modules/core-modules/part-cooling/src/lib.rs` (full — 150 LOC).
  - `modules/core-modules/seam-placer/seam-placer.toml` (full — ≤ 50 lines) — `string`-type precedent.
  - `design.md` "TOML Manifest Shape (verbatim)" and "src/lib.rs shape" sections (this packet's own design.md).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` (NEW).
  - `modules/core-modules/machine-gcode-emit/Cargo.toml` (NEW).
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` (NEW).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs` — Step 5.
  - All test files — Step 2 wrote them.
  - All other core modules beyond `part-cooling` and `seam-placer`.
- Expected sub-agent dispatches:
  - `Q: "After writing modules/core-modules/machine-gcode-emit/{machine-gcode-emit.toml, Cargo.toml, src/lib.rs}, run './modules/core-modules/build-core-modules.sh' (no --check) to produce the .wasm. Return FACT: 'build succeeded' or SNIPPETS (≤ 30 lines) of the first build error." | Scope: modules/core-modules/ | Return format: FACT`
  - `Q: "Run './modules/core-modules/build-core-modules.sh --check'. Return FACT: '--check returned clean' or SNIPPETS (≤ 20 lines) of the STALE list. Per CLAUDE.md Guest WASM Staleness, this is mandatory after any new module's source change AND after the WIT change in Step 3." | Scope: modules/core-modules/ | Return format: FACT`
  - `Q: "Run './test-guests/build-test-guests.sh --check'. Return FACT clean/stale. The Step 3 WIT change invalidates test-guest bindgen output." | Scope: test-guests/ | Return format: FACT`
  - `Q: "Locate the host manifest-discovery API for module [config.schema.<key>] lookup (likely crates/slicer-host/src/manifest.rs near the ConfigFieldEntry parse site at :827-:828). Return FACT: API name + file:line (≤ 6 lines). If no test-friendly API exists, return FACT: 'no direct API; AC uses CONFIG_BLOCK fallback'." | Scope: crates/slicer-host/src/manifest.rs | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- module_manifest_registers_four_keys_with_expected_types_and_defaults --nocapture'. Return FACT (pass/fail); SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- new_keys_appear_in_config_block --nocapture'. Return FACT (pass/fail); SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd'. Return FACT pass/fail; SNIPPETS on fail (≤ 20 lines)." | Scope: workspace | Return format: FACT`
- Context cost: `M`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — `[config.schema.<key>]` section.
  - `docs/05_module_sdk.md` — `FinalizationModule` trait signature.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` (key-name spelling, FACT ≤ 6 lines).
- Verification:
  - Both `--check` commands clean.
  - Two targeted ACs green; packet-55 CONFIG_BLOCK regression green.
- Exit condition: Module folder present + builds; both `--check` commands clean; two ACs green; no regression; clippy clean for the three new files.

### Step 5: Wire serializer to read GCodeIR fields and emit at byte positions

- Task IDs:
  - `TASK-193`
- Objective: Inside `DefaultGCodeSerializer::serialize_gcode()` body (at `crates/slicer-host/src/gcode_emit.rs:1021`), read `gcode_ir.print_start_gcode` and `gcode_ir.print_end_gcode` (the new fields added in Step 3). Insert the start string AFTER `serialize_header_block` + `serialize_width_comments` emission, BEFORE the M82/M83 preamble. Insert the end string AFTER the last layer's commands, BEFORE the inner serializer returns (the `ThumbnailAwareSerializer` wrapper then appends THUMBNAIL/CONFIG_BLOCK on top). Empty / whitespace-only ⇒ no bytes. The serializer contains NO `substitute_placeholders` helper.
- Precondition: Step 4 complete; the module pushes resolved strings; dispatch routes them into `GCodeIR` fields; 2 ACs green; 10 ACs red.
- Postcondition: All 12 ACs green; all regression suites green; clippy clean.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs:667-:740` — HEADER + width (range).
  - `crates/slicer-host/src/gcode_emit.rs:928-:1020` — CONFIG_BLOCK at `:928` + `ThumbnailAwareSerializer` at `:973` (range).
  - `crates/slicer-host/src/gcode_emit.rs:1021-:1166` — `serialize_gcode` body at `:1021` + preamble emission (M83 at `:1067`, M82 at `:1069`) (range).
  - `crates/slicer-ir/src/slice_ir.rs:1779-:1799` — `GCodeIR` (Step 3 extension is already merged; confirm the new fields are present).
- Files allowed to edit (≤ 1):
  - `crates/slicer-host/src/gcode_emit.rs`.
- Files explicitly out-of-bounds for this step:
  - The rest of `crates/slicer-host/src/gcode_emit.rs` outside the three ranges above.
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` — out of scope.
  - `modules/core-modules/machine-gcode-emit/**` — Step 4 closed.
  - All test files — Step 2 wrote them; do not edit them in this step.
- Expected sub-agent dispatches:
  - `Q: "From crates/slicer-host/src/gcode_emit.rs, return SNIPPETS (≤ 30 lines) of the exact byte-range inside DefaultGCodeSerializer::serialize_gcode() body where (a) serialize_header_block and serialize_width_comments are appended (start-block insertion site), and (b) the M82/M83 preamble line is emitted (M83 at :1067, M82 at :1069). Cite file:line ranges for both." | Scope: crates/slicer-host/src/gcode_emit.rs:1021-1080 | Return format: SNIPPETS`
  - `Q: "From crates/slicer-host/src/gcode_emit.rs, return SNIPPETS (≤ 30 lines) of the inner DefaultGCodeSerializer::serialize_gcode() last accumulation point before the function returns its String buffer (end-block insertion site). Cite file:line." | Scope: crates/slicer-host/src/gcode_emit.rs:1100-1170 | Return format: SNIPPETS`
  - `Q: "How does packet 55's serialize_config_block at :928 emit multi-line String values (with \\n)? Return SNIPPETS (≤ 15 lines) of the formatter or value-conversion site." | Scope: crates/slicer-host/src/gcode_emit.rs:928-970 | Return format: SNIPPETS`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd'. Return FACT (pass/fail); SNIPPETS (first failing test name + ≤ 15 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test gcode_emit_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
- Context cost: `S`.
- Authoritative docs:
  - `docs/01_system_architecture.md` — finalization stage / serializer role; delegate a SUMMARY.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3181` / `:3200` / `:3258` (start-before-preamble ordering, FACT ≤ 12 lines).
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (`apply_config()`) — delegate FACT (≤ 10 lines).
- Verification:
  - `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` green (all 12).
  - `cargo test -p slicer-host --test gcode_emit_tdd` green (no regression).
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` green (no regression).
- Exit condition: All 12 ACs green; 2 regression suites green; ≤ 25 LOC added in gcode_emit.rs; no `unwrap()` introduced without explanatory comment.

### Step 6: Regression sweep + workspace gates

- Task IDs:
  - `TASK-193`
  - `TASK-193a`
  - `TASK-193b`
- Objective: Confirm no broader regression and that workspace lint gates remain clean. Re-verify guest-wasm and test-guest freshness.
- Precondition: Step 5 complete; all 12 ACs green.
- Postcondition: `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` green; `./modules/core-modules/build-core-modules.sh --check` clean; `./test-guests/build-test-guests.sh --check` clean; `cargo check --workspace` clean; `cargo clippy --workspace -- -D warnings` clean.
- Files allowed to read: none direct.
- Files allowed to edit (≤ 3): zero or one (for minor clippy fixes only; if a fix requires more than 5 LOC across more than 1 file, STOP and split into a follow-up step).
- Files explicitly out-of-bounds for this step:
  - Any file not flagged by clippy / cargo check.
- Expected sub-agent dispatches:
  - `Q: "Run 'cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run './modules/core-modules/build-core-modules.sh --check'. Return FACT clean/stale." | Scope: modules/core-modules/ | Return format: FACT`
  - `Q: "Run './test-guests/build-test-guests.sh --check'. Return FACT clean/stale." | Scope: test-guests/ | Return format: FACT`
  - `Q: "Run 'cargo check --workspace'. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first error on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo clippy --workspace -- -D warnings'. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first warning on fail." | Scope: workspace | Return format: FACT`
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - All five FACTs above return pass.
- Exit condition: All five FACTs green; any minor clippy fix is single-file and ≤ 5 LOC.

### Step 7: Packet completion gate

- Task IDs:
  - `TASK-193`
  - `TASK-193a`
  - `TASK-193b`
- Objective: Final acceptance ceremony — re-dispatch every pipe-suffixed AC command from `packet.spec.md`, run `cargo test --workspace` ONCE per CLAUDE.md test discipline, mark docs/07 rows `[x]`, prepare `packet.spec.md` for status flip.
- Precondition: Step 6 complete.
- Postcondition: Every pipe-suffixed AC command in `packet.spec.md` re-dispatched and FACT-pass; `cargo test --workspace` returns FACT pass; docs/07 TASK-193 / TASK-193a / TASK-193b rows updated to `[x]`; `packet.spec.md` frontmatter is ready to flip from `draft` to `implemented` (the implementer asks the user before flipping).
- Files allowed to read: none direct.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` — via worker dispatch only.
  - `.ralph/specs/59_machine-start-end-gcode-emission/packet.spec.md` — status flip ONLY after explicit user OK.
- Files explicitly out-of-bounds for this step:
  - All source crates (closure ceremony is read/dispatch only).
- Expected sub-agent dispatches:
  - 12 FACT dispatches: re-run every pipe-suffixed `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- <test_name> --nocapture` command from `packet.spec.md`. Each returns FACT pass/fail.
  - `Q: "Run 'cargo test --workspace'. Return FACT pass/fail; SNIPPETS (≤ 40 lines) of first failing test name + assertion on fail. NEVER return the full test output (the suite is > 1000 tests)." | Scope: workspace | Return format: FACT`
  - `Q: "Update docs/07_implementation_status.md rows for TASK-193, TASK-193a, TASK-193b from '[ ]' to '[x]'. Return FACT: 'rows updated' + the three new full-row lines after edit." | Scope: docs/07_implementation_status.md | Return format: FACT`
- Context cost: `S`.
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` — closure ceremony.
  - `docs/12_architecture_gate_metrics.md` — consulted only on workspace-gate fail.
- OrcaSlicer refs: none.
- Verification:
  - Every pipe-suffixed AC command in `packet.spec.md` returns FACT pass.
  - `cargo test --workspace` returns FACT pass.
  - docs/07 rows updated to `[x]`.
- Exit condition: All FACTs green; docs/07 updated; user explicitly OK'd the status flip to `implemented` (or chose to leave `draft`).

For read-only discovery steps: Step 1 has expected output count = 5 LOCATIONS + FACT (3 hits); Step 6 has expected output = 5 FACT pass; Step 7 has expected output = 12 + 1 + 1 FACT pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | docs-only edit via worker dispatch; ≤ 3 dispatches; no source code read. |
| Step 2 | M | One new test file (≤ 400 LOC); reads 2 sibling test files + 1 ranged; 2 dispatches; assertion-encode all 12 ACs. |
| Step 3 | M | Additive WIT/SDK/dispatch/IR extension across 5 files (~50 LOC total); CLAUDE.md WIT/Type Changes Checklist sub-steps; 3 dispatches; no AC green yet. |
| Step 4 | M | Three new module files (~120-150 LOC for `src/lib.rs` with real substitution); two `--check` re-runs (modules + test-guests); 7 dispatches; 2 ACs green. |
| Step 5 | S | One file edit (gcode_emit.rs, ranged reads only); ≤ 25 LOC added; 6 dispatches; 10 ACs turn green. |
| Step 6 | S | Dispatch-only; ≤ 1 LOC minor clippy fix if any; 5 FACT dispatches. |
| Step 7 | S | Dispatch-only closure ceremony; 12 + 1 + 1 = 14 FACT dispatches; docs/07 + packet.spec.md status flip. |
| **Aggregate** | **M** | **4×S + 3×M = M aggregate**; no step is L. |

If the sum exceeds M aggregate, or any single step is L, the packet must be split before activation. Both conditions are satisfied: aggregate is M, max step is M.

## Packet Completion Gate

- All 7 steps complete.
- Every step exit condition is met.
- All 12 packet acceptance criteria are FACT-green (every pipe-suffixed verification command dispatched and returned PASS).
- `./modules/core-modules/build-core-modules.sh --check` and `./test-guests/build-test-guests.sh --check` both return clean at packet closure (re-verified during Step 6).
- `docs/07_implementation_status.md` updated for TASK-193, TASK-193a, TASK-193b to status `[x]` (via worker dispatch).
- No prior packet status transition needed (this packet does not reopen, supersede, or correct any predecessor).
- `packet.spec.md` ready to flip from `status: draft` to `status: implemented` — but the implementer asks the user explicitly before flipping.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (12 total) — each FACT pass.
- Dispatch `cargo test --workspace` ONCE (per CLAUDE.md test discipline: forbidden during implementation, REQUIRED at packet closure). Worker returns FACT pass; on fail, SNIPPETS (≤ 40 lines).
- Confirm packet-level verification commands are green (`cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, both `--check` commands).
- Record any remaining packet-local risk explicitly before moving to `status: implemented`. Known risks from `design.md`:
  - WIT contract addition (additive; documented; CLAUDE.md WIT/Type Changes Checklist passed in Step 3).
  - Cross-component WARN-log forwarding dropped from the negative AC (documented; future packet tracked).
  - End-block position differs from OrcaSlicer (intentional deviation; documented).
  - Range enforcement declarative-only (intentional; future TASK-### packet tracked).
  - Multi-line `machine_start_gcode` CONFIG_BLOCK wire format (whatever packet 55 chose; recorded in Step 5 SNIPPETS dispatch return).
- Confirm the implementer's peak context usage stayed under 70%. If not, log it as a packet-authoring lesson for future spec-packet-generator runs.
