# Implementation Plan: 59_machine-start-end-gcode-emission

## Execution Rules

- One atomic step at a time.
- Each step must map back to TASK-193 or TASK-193a.
- TDD first (Step 2), then implementation (Steps 3-4), then narrowest falsifying validation (Step 5), then completion gate (Step 6).
- Each step honors the context-discipline preamble. The fields below are not optional metadata — they are the budget contract for this step.
- The implementer MUST stop reading at 60% context and hand off at 85%.
- The implementer MUST delegate every `cargo` invocation and every OrcaSlicer / docs/07 access.

## Steps

### Step 1: Append TASK-193 / TASK-193a rows to docs/07

- Task IDs:
  - `TASK-193`
  - `TASK-193a`
- Objective: Add two queued backlog rows so the packet's task IDs exist before any code change. This unblocks downstream tooling that cross-references `docs/07_implementation_status.md`.
- Precondition: Neither TASK-193 nor TASK-193a appears in `docs/07_implementation_status.md`.
- Postcondition: Both rows present, each with status `[ ]` (queued), placed near other in-progress / queued G-code-output tasks. No other rows modified.
- Files allowed to read:
  - `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md:3-6` — for the TASK-184 / TASK-185 row-format precedent (already loaded in generator context; trivial direct read here is fine).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` — **edit via worker dispatch only**. Implementer never loads the full file.
- Files explicitly out-of-bounds for this step:
  - The rest of `docs/07_implementation_status.md` — only the insertion-point LOCATIONS dispatch return is admitted into context.
  - All source crates (this step is docs-only).
- Expected sub-agent dispatches:
  - `Q: "In docs/07_implementation_status.md, find the line range where queued / in-progress G-code-output TASK entries live (proximity to TASK-184 / TASK-185 / TASK-191 / TASK-192a). Return LOCATIONS, ≤ 5 entries, each with the adjacent row's verbatim text. Do not return the rest of the file." | Scope: docs/07_implementation_status.md | Return format: LOCATIONS`
  - `Q: "Append two rows to docs/07_implementation_status.md immediately after line <insertion-line>: 'TASK-193 — Emit configurable machine_start_gcode / machine_end_gcode with [key] placeholder substitution' (status [ ] queued) and 'TASK-193a — Register machine_start_gcode, machine_end_gcode, bed_temperature_initial_layer_single, nozzle_temperature_initial_layer in FullConfigSchema' (status [ ] queued). Match the row format of the adjacent rows verbatim. Return FACT: bytes appended + line numbers of the two new rows." | Scope: docs/07_implementation_status.md | Return format: FACT`
  - `Q: "Run 'grep -n TASK-193 docs/07_implementation_status.md' and return FACT (expected: exactly 2 hits — TASK-193 and TASK-193a)." | Scope: docs/07_implementation_status.md | Return format: FACT`
- Context cost: `S`.
- Authoritative docs:
  - `docs/07_implementation_status.md` — delegate every interaction; never load directly.
- OrcaSlicer refs: none for this step.
- Verification:
  - The final `grep -n` FACT returns exactly 2 hits.
- Exit condition: TASK-193 and TASK-193a present as queued rows in `docs/07_implementation_status.md`.

### Step 2: Write red TDD test file with all 13 assertions

- Task IDs:
  - `TASK-193`
  - `TASK-193a`
- Objective: Materialize all 9 positive + 4 negative acceptance criteria as failing Rust tests in a new file. This locks the contract before any implementation.
- Precondition: `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` does not exist; Step 1 complete.
- Postcondition: New test file present; the file compiles; all 13 tests are present by name and all 13 FAIL (red state). No test is `#[ignore]`. No production code modified.
- Files allowed to read:
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full file, ≈ 600-800 LOC) — packet-55 TDD scaffolding pattern.
  - `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` — layer fixture + M104/M109 capture pattern.
  - `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80` — slicer-cli invocation harness pattern.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` (NEW).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/**` — no production code change in this step.
  - `crates/slicer-host/src/config_schema.rs` — no schema change yet (the `schema_registers_four_keys_with_expected_types_and_defaults` test will fail because the keys are not registered — that is the desired red state).
  - `crates/slicer-host/src/gcode_emit.rs` — no serializer change yet.
- Expected sub-agent dispatches:
  - `Q: "In crates/slicer-host/tests/, find the small STL fixture path used by gcode_header_thumbnail_config_blocks_tdd.rs and gcode_emit_tdd.rs. Return FACT: the exact 'concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../resources/<filename>.stl\")' literal string." | Scope: crates/slicer-host/tests/ | Return format: FACT`
  - `Q: "Confirm 'log = \"0.4\"' is in crates/slicer-host/Cargo.toml (expected at :20) and that no other slicer-host test installs a global logger via 'log::set_boxed_logger' or 'log::set_logger' that would conflict with the new test's logger install. If a conflicting global logger exists, return its install site (file:line) so the new test can use a thread-local capture workaround instead. Return FACT (≤ 6 lines)." | Scope: crates/slicer-host/Cargo.toml, crates/slicer-host/tests/ | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -50' after the test file is written. Return FACT: 'all 13 tests FAILED' or SNIPPETS (≤ 20 lines) showing any test that PASSED or was IGNORED unexpectedly." | Scope: workspace | Return format: FACT`
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md:433-444` — `ConfigValue` enum (range read).
  - `docs/02_ir_schemas.md:618-730` — `ResolvedConfig` (range read).
- OrcaSlicer refs: none for this step (test scaffolding does not need OrcaSlicer evidence; the assertions encode the contract).
- Verification:
  - `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -50` — dispatched as FACT: ALL 13 tests FAILED.
- Exit condition: 13 tests defined by name, all failing, test file compiles cleanly.

### Step 3: Register four host config keys in FullConfigSchema

- Task IDs:
  - `TASK-193a`
- Objective: Add `machine_start_gcode`, `machine_end_gcode`, `bed_temperature_initial_layer_single`, `nozzle_temperature_initial_layer` to `FullConfigSchema::default()` with the defaults and ranges from `packet.spec.md` Goal. Turns the `schema_registers_four_keys_with_expected_types_and_defaults` AC green; also turns `new_keys_appear_in_config_block` green via packet-55's automatic CONFIG_BLOCK propagation.
- Precondition: Step 2 complete; 13 tests red.
- Postcondition: 4 keys registered; `schema_registers_four_keys_with_expected_types_and_defaults` and `new_keys_appear_in_config_block` ACs green; other 11 ACs still red (substitution helper + serializer wiring not yet done); no regression in any existing test.
- Files allowed to read:
  - `crates/slicer-host/src/config_schema.rs` — **1044 lines, ABOVE the 600-line direct-read budget; range-read only**. Required SNIPPETS dispatches: `:367-:387` (`thumbnail_path` String precedent) and `:191-:212` (`fan_speed_min` Int+range precedent). One additional SNIPPETS dispatch (≤ 30 lines) may be needed to find the appropriate insertion section for the 4 new entries. NEVER load the full file.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/config_schema.rs`.
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs` — serializer wiring is Step 4.
  - All test files — Step 2 already wrote them.
- Expected sub-agent dispatches:
  - `Q: "From crates/slicer-host/src/config_schema.rs, return SNIPPETS (≤ 25 lines) of: (a) the thumbnail_path String registration at :367-:387; (b) the fan_speed_min Int+range registration at :191-:212. Both with file:line citations. Do NOT load the full file (1044 LOC, above the 600-line direct-read budget)." | Scope: crates/slicer-host/src/config_schema.rs | Return format: SNIPPETS`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- schema_registers_four_keys_with_expected_types_and_defaults --nocapture'. Return FACT (pass/fail); SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- new_keys_appear_in_config_block --nocapture'. Return FACT (pass/fail); SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd'. Return FACT pass/fail; SNIPPETS on fail (≤ 20 lines)." | Scope: workspace | Return format: FACT`
- Context cost: `S`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — host-level vs module-level config schema; load directly only the relevant section.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp:1243` and `:1288` — delegate FACT (≤ 6 lines) to confirm key-name spelling.
- Verification:
  - Both targeted ACs green; packet-55 CONFIG_BLOCK regression green.
- Exit condition: Two ACs green; no regression in any existing test; clippy clean for the schema file.

### Step 4: Implement substitute_placeholders + wire start/end blocks

- Task IDs:
  - `TASK-193`
- Objective: Add the private `substitute_placeholders(template: &str, lookup: &HashMap<String, ConfigValue>) -> String` helper in `gcode_emit.rs`, then wire the substituted start block (after HEADER + width comments, before preamble) and end block (after last layer's commands, before the `ThumbnailAwareSerializer` wrapper's THUMBNAIL/CONFIG_BLOCK append). Turn the remaining 11 ACs green.
- Precondition: Step 3 complete; 2 ACs green; 11 ACs red.
- Postcondition: Helper present; both insertion sites in place; all 13 ACs green; all regression suites green; clippy clean.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs:626-740` — HEADER + width + thumbnail (range).
  - `crates/slicer-host/src/gcode_emit.rs:887-977` — CONFIG_BLOCK + `ThumbnailAwareSerializer` (range).
  - `crates/slicer-host/src/gcode_emit.rs:979-1166` — `serialize_gcode` body + preamble emission (range).
  - `crates/slicer-ir/src/slice_ir.rs:433-444` — `ConfigValue` enum (range).
  - `crates/slicer-ir/src/slice_ir.rs:618-730` — `ResolvedConfig` (range).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs`.
- Files explicitly out-of-bounds for this step:
  - The rest of `crates/slicer-host/src/gcode_emit.rs` outside the three ranges above. NEVER load the full file.
  - The rest of `crates/slicer-ir/src/slice_ir.rs` outside the two ranges above. NEVER load the full file.
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` — out of scope; do NOT consult its grammar.
  - `crates/slicer-host/src/config_schema.rs` — Step 3 closed.
  - All test files — Step 2 wrote them; do not edit them in this step.
- Expected sub-agent dispatches:
  - `Q: "From crates/slicer-host/src/gcode_emit.rs, return SNIPPETS (≤ 30 lines) of the exact byte-range inside DefaultGCodeSerializer::serialize_gcode() where (a) serialize_header_block and serialize_width_comments are appended (start-block insertion site), and (b) the M82/M83 preamble line is emitted (boundary). Cite file:line ranges for both." | Scope: crates/slicer-host/src/gcode_emit.rs | Return format: SNIPPETS`
  - `Q: "From crates/slicer-host/src/gcode_emit.rs, return SNIPPETS (≤ 30 lines) of the inner DefaultGCodeSerializer::serialize_gcode() last accumulation point before the function returns its String buffer (end-block insertion site). Cite file:line." | Scope: crates/slicer-host/src/gcode_emit.rs | Return format: SNIPPETS`
  - `Q: "Confirm crates/slicer-host/src/gcode_emit.rs already imports HashMap and the ConfigValue type used in Step 3's registration. Return FACT: yes/no + which use statements exist (≤ 5 lines)." | Scope: crates/slicer-host/src/gcode_emit.rs:1-40 | Return format: FACT`
  - `Q: "How does packet 55's serialize_config_block emit multi-line String values (with \\n)? Return SNIPPETS (≤ 15 lines) of the formatter or value-conversion site." | Scope: crates/slicer-host/src/gcode_emit.rs:887-920 | Return format: SNIPPETS`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd'. Return FACT (pass/fail); SNIPPETS (first failing test name + ≤ 15 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test gcode_emit_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
- Context cost: `M`.
- Authoritative docs:
  - `docs/01_system_architecture.md` — finalization stage / serializer role; delegate a SUMMARY (file long).
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode.cpp:3181` (substitution), `:3200` (start write), `:3258` (preamble after) — delegate one FACT dispatch (≤ 12 lines) confirming ordering: substituted start block BEFORE preamble. Already cited in `packet.spec.md` and `requirements.md`; the implementer may rely on this packet's record unless a contradiction surfaces.
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp:164` (`apply_config()`) — delegate FACT (≤ 10 lines) confirming substitution sources values from config symbol table.
- Verification:
  - `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd` green (all 13).
  - `cargo test -p slicer-host --test gcode_emit_tdd` green (no regression).
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` green (no regression).
- Exit condition: All 13 ACs green; 2 regression suites green; no `unwrap()` introduced in the helper without an explanatory comment; helper ≤ 60 LOC.

### Step 5: Regression sweep + workspace gates

- Task IDs:
  - `TASK-193`
  - `TASK-193a`
- Objective: Confirm no broader regression and that workspace lint gates remain clean.
- Precondition: Step 4 complete; all 13 ACs green.
- Postcondition: `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` green; `cargo check --workspace` clean; `cargo clippy --workspace -- -D warnings` clean.
- Files allowed to read: none direct.
- Files allowed to edit (≤ 3): zero or one (for minor clippy fixes only; if a fix requires more than 5 LOC across more than 1 file, STOP and split into a follow-up step rather than expanding this one).
- Files explicitly out-of-bounds for this step:
  - Any file not flagged by clippy / cargo check.
  - The packet's own test file (already validated in Step 4).
- Expected sub-agent dispatches:
  - `Q: "Run 'cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo check --workspace'. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first error on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo clippy --workspace -- -D warnings'. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first warning on fail." | Scope: workspace | Return format: FACT`
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - Three FACTs above all return pass.
- Exit condition: All three FACTs green; any minor clippy fix is single-file and ≤ 5 LOC.

### Step 6: Packet completion gate

- Task IDs:
  - `TASK-193`
  - `TASK-193a`
- Objective: Final acceptance ceremony — re-dispatch every pipe-suffixed AC command from `packet.spec.md`, run `cargo test --workspace` ONCE per CLAUDE.md test discipline, mark docs/07 rows `[x]`, prepare `packet.spec.md` for status flip.
- Precondition: Step 5 complete.
- Postcondition: Every pipe-suffixed AC command in `packet.spec.md` re-dispatched and FACT-pass; `cargo test --workspace` returns FACT pass; docs/07 TASK-193 / TASK-193a rows updated to `[x]`; `packet.spec.md` frontmatter is ready to flip from `draft` to `implemented` (the implementer asks the user before flipping, since this is an `active`-equivalent state change).
- Files allowed to read: none direct.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` — via worker dispatch only.
  - `.ralph/specs/59_machine-start-end-gcode-emission/packet.spec.md` — status flip ONLY after explicit user OK.
- Files explicitly out-of-bounds for this step:
  - All source crates (closure ceremony is read/dispatch only — no further code change).
- Expected sub-agent dispatches:
  - 13 FACT dispatches: re-run every pipe-suffixed `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- <test_name> --nocapture` command from `packet.spec.md`. Each returns FACT pass/fail.
  - `Q: "Run 'cargo test --workspace'. Return FACT pass/fail; SNIPPETS (≤ 40 lines) of first failing test name + assertion on fail. NEVER return the full test output (the suite is > 1000 tests)." | Scope: workspace | Return format: FACT`
  - `Q: "Update docs/07_implementation_status.md rows for TASK-193 and TASK-193a from '[ ]' to '[x]'. Return FACT: 'rows updated' + the two new full-row lines after edit." | Scope: docs/07_implementation_status.md | Return format: FACT`
- Context cost: `S`.
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` — closure ceremony reference; load directly only the acceptance-gate section.
  - `docs/12_architecture_gate_metrics.md` — gate metrics reference; consult only if a workspace-gate FACT returns fail.
- OrcaSlicer refs: none for this step.
- Verification:
  - Every pipe-suffixed AC command in `packet.spec.md` returns FACT pass.
  - `cargo test --workspace` returns FACT pass.
  - docs/07 rows updated to `[x]`.
- Exit condition: All FACTs green; docs/07 updated; user explicitly OK'd the status flip to `implemented` (or chose to leave `draft` for further review).

For read-only discovery steps: Step 1 has expected output count = 5 LOCATIONS + FACT (2 hits); Step 5 has expected output = 3 FACT pass; Step 6 has expected output = 13 + 1 + 1 FACT pass.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | docs-only edit via worker dispatch; ≤ 3 dispatches; no source code read. |
| Step 2 | M | One new test file (≤ 400 LOC); reads 2 sibling test files (one full ≤ 800 LOC + one ranged) + 1 ranged; 3 dispatches; assertion-encode all 13 ACs. |
| Step 3 | S | One file edit; ranged reads of config_schema.rs (1044 LOC — range only at `:367-:387` and `:191-:212`); 4 dispatches; 2 ACs turn green. |
| Step 4 | M | One file edit (gcode_emit.rs, ranged reads only); ≤ 100 LOC added; 7 dispatches; 11 ACs turn green. |
| Step 5 | S | Dispatch-only; ≤ 1 LOC minor clippy fix if any; 3 FACT dispatches. |
| Step 6 | S | Dispatch-only closure ceremony; 13 + 1 + 1 = 15 FACT dispatches; docs/07 + packet.spec.md status flip. |
| **Aggregate** | **M** | **5×S + 1×M = M**; no step is L. |

If the sum exceeds M aggregate, or any single step is L, the packet must be split before activation. Both conditions are satisfied: aggregate is M, max step is M.

## Packet Completion Gate

- All 6 steps complete.
- Every step exit condition is met.
- All 13 packet acceptance criteria are FACT-green (every pipe-suffixed verification command dispatched and returned PASS).
- `docs/07_implementation_status.md` updated for TASK-193 and TASK-193a to status `[x]` (via worker dispatch — implementer never edits by loading the full backlog into context).
- No prior packet status transition needed (this packet does not reopen, supersede, or correct any predecessor).
- `packet.spec.md` ready to flip from `status: draft` to `status: implemented` — but the implementer asks the user explicitly before flipping, since two other packets are currently `active` and the closure ceremony is a state-change moment.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (13 total) — each FACT pass.
- Dispatch `cargo test --workspace` ONCE (per CLAUDE.md test discipline: forbidden during implementation, REQUIRED at packet closure). Worker returns FACT pass; on fail, SNIPPETS (≤ 40 lines) of the first failing test name + assertion. The implementer NEVER receives the full > 1000-test output.
- Confirm packet-level verification commands are green (`cargo check --workspace`, `cargo clippy --workspace -- -D warnings`).
- Record any remaining packet-local risk explicitly before moving to `status: implemented`. Known risks from `design.md`:
  - End-block position differs from OrcaSlicer (intentional deviation; documented). Confirm `requirements.md` Out-of-scope language and `design.md` Risks-and-Tradeoffs language both stand.
  - Multi-line `machine_start_gcode` CONFIG_BLOCK wire format (whatever packet 55 chose — recorded in Step 4 SNIPPETS dispatch return).
  - `log` capture mechanism choice — custom `log::Log` impl using existing `log = "0.4"` dep (recorded in Step 2 FACT dispatch return; no new workspace dependency added).
- Confirm the implementer's peak context usage stayed under 70%. If not, log it as a packet-authoring lesson for future spec-packet-generator runs (specifically: which dispatch returned more than the contracted format, and how to tighten next time).
