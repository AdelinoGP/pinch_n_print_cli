# Implementation Plan: 59_machine-start-end-gcode-emission

## Execution Rules

- One atomic step at a time.
- Each step must map back to TASK-194, TASK-194a, or TASK-194b.
- TDD first (Step 2), then promote M82/M83 to a typed `GCodeCommand` variant (Step 3), then create the GCodePostProcess module (Step 4), then regression sweep (Step 5), then completion gate (Step 6).
- Each step honors the context-discipline preamble. The fields below are not optional metadata — they are the budget contract for this step.
- The implementer MUST stop reading at 60% context and hand off at 85%.
- The implementer MUST delegate every `cargo` invocation, every OrcaSlicer / docs/07 access, and every guest-wasm build.

## Steps

### Step 1: Append TASK-194 / TASK-194a / TASK-194b rows to docs/07

- Task IDs:
  - `TASK-194`
  - `TASK-194a`
  - `TASK-194b`
- Objective: Add three queued backlog rows so the packet's task IDs exist before any code change.
- Precondition: None of TASK-194, TASK-194a, TASK-194b appears in `docs/07_implementation_status.md`.
- Postcondition: Three rows present with status `[ ]` (queued), placed near other in-progress / queued G-code-output tasks. Row texts:
  - TASK-194 — "Emit configurable `machine_start_gcode` / `machine_end_gcode` via a `PostPass::GCodePostProcess` module that prepends/appends `Raw` commands carrying the resolved templates."
  - TASK-194a — "Create `modules/core-modules/machine-gcode-emit/` declaring four `[config.schema.*]` entries; `run_gcode_postprocess` performs real `[key]` substitution against the effective `ConfigView` and rebuilds the command list as `[Raw(start), ...existing..., Raw(end)]`."
  - TASK-194b — "Promote `M82`/`M83` from the hard-coded `DefaultGCodeSerializer` preamble to a new `GCodeCommand::ExtrusionMode { absolute: bool }` variant pushed by `DefaultGCodeEmitter` so a `GCodePostProcess` module can prepend before it."
- Files allowed to read:
  - `.ralph/specs/55_gcode-header-thumbnail-config-blocks/packet.spec.md:3-6` — TASK-184 / TASK-185 row-format precedent.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` — **edit via worker dispatch only**. Implementer never loads the full file.
- Files explicitly out-of-bounds for this step:
  - The rest of `docs/07_implementation_status.md` — only the insertion-point LOCATIONS dispatch return is admitted.
  - All source crates (this step is docs-only).
- Expected sub-agent dispatches:
  - `Q: "In docs/07_implementation_status.md, find the line range where queued / in-progress G-code-output TASK entries live (proximity to TASK-184 / TASK-185 / TASK-191 / TASK-192a). Return LOCATIONS, ≤ 5 entries, each with the adjacent row's verbatim text. Do not return the rest of the file." | Scope: docs/07_implementation_status.md | Return format: LOCATIONS`
  - `Q: "Append three rows to docs/07_implementation_status.md immediately after line <insertion-line>: TASK-194, TASK-194a, TASK-194b (texts above). All status [ ] queued. Match the row format of the adjacent rows verbatim. Return FACT: bytes appended + line numbers of the three new rows." | Scope: docs/07_implementation_status.md | Return format: FACT`
  - `Q: "Run 'grep -n TASK-194 docs/07_implementation_status.md' and return FACT (expected: exactly 3 hits — TASK-194, TASK-194a, TASK-194b)." | Scope: docs/07_implementation_status.md | Return format: FACT`
- Context cost: `S`.
- Authoritative docs:
  - `docs/07_implementation_status.md` — delegate every interaction.
- OrcaSlicer refs: none.
- Verification:
  - The final `grep -n` FACT returns exactly 3 hits.
- Exit condition: TASK-194, TASK-194a, TASK-194b present as queued rows in `docs/07_implementation_status.md`.

### Step 2: Write red TDD test file with all 13 failing assertions

- Task IDs:
  - `TASK-194`
  - `TASK-194a`
  - `TASK-194b`
- Objective: Materialize all 10 positive + 3 negative acceptance criteria as failing Rust tests in a new file. The test exercises the END-TO-END pipeline: `slicer-cli` invocation → `ResolvedConfig` → emitter pushes `ExtrusionMode` head → `GCodePostProcess` module reads keys, substitutes, prepends `Raw(start)`, re-emits snapshot, appends `Raw(end)` → serializer renders the new command list → byte-level file scan.
- Precondition: `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` does not exist; Step 1 complete.
- Postcondition: New test file present; compiles; all 13 tests present by name and all 13 FAIL (red state). No test is `#[ignore]`. No production code modified.
- Files allowed to read:
  - `crates/slicer-host/tests/gcode_header_thumbnail_config_blocks_tdd.rs` (full file) — packet-55 TDD scaffolding.
  - `crates/slicer-host/tests/gcode_emit_tdd.rs:1-120` — layer fixture + M82/M83 capture pattern.
  - `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs:1-80` — slicer-cli invocation harness pattern.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` (NEW).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/**`, `crates/slicer-ir/src/**`, `crates/slicer-sdk/src/**`, `wit/**`, `modules/core-modules/**` — no production code change in this step.
- Expected sub-agent dispatches:
  - `Q: "In crates/slicer-host/tests/, find the small STL fixture path used by gcode_emit_tdd.rs and gcode_header_thumbnail_config_blocks_tdd.rs. Return FACT: the exact 'concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../resources/<filename>.stl\")' literal string." | Scope: crates/slicer-host/tests/ | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -60' after the test file is written. Return FACT: 'all 13 tests FAILED' or SNIPPETS (≤ 20 lines) showing any test that PASSED or was IGNORED unexpectedly." | Scope: workspace | Return format: FACT`
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` `ConfigValue` enum + `GCodeCommand` enum (range read).
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd 2>&1 | tail -60` returns ALL 13 FAILED.
- Exit condition: 13 tests defined by name (10 positive + 3 negative), all failing, test file compiles cleanly.

### Step 3: Promote M82/M83 to `GCodeCommand::ExtrusionMode`

- Task IDs:
  - `TASK-194b`
- Objective: Add `GCodeCommand::ExtrusionMode { absolute: bool }` as a new additive variant; update `DefaultGCodeEmitter::emit_gcode` to push it as the head command; remove the hard-coded `M82`/`M83` writes from `DefaultGCodeSerializer::serialize_gcode` at `:1154-1156` and add an `ExtrusionMode` arm to the per-command renderer. The net effect on default output is bit-identical: the M82 (or M83) line appears in the same byte position as before, just sourced from a typed command instead of a hard-coded preamble write.
- Precondition: Step 2 complete; 13 tests red.
- Postcondition:
  - `GCodeCommand` has one new variant (was 8 variants, now 9).
  - `DefaultGCodeEmitter::emit_gcode` pushes `ExtrusionMode { absolute }` as the first element of the `Vec<GCodeCommand>` it builds.
  - `DefaultGCodeSerializer::serialize_gcode` no longer hard-codes M82/M83 at `:1154-1156` (those lines are removed).
  - The serializer's per-command renderer has a new `ExtrusionMode { absolute }` arm rendering `"M82\n"` / `"M83\n"`.
  - `wit/deps/ir-types.wit` updated ONLY IF `gcode-command` is mirrored in WIT (confirmed by the first Step 3 dispatch). If updated, the addition is one new variant.
  - `cargo build --tests` clean.
  - `cargo test -p slicer-host --test gcode_emit_tdd` GREEN (packet-54 regression — CRITICAL).
  - AC `extrusion_mode_still_emitted_after_promotion` turns GREEN (the regression sentry).
  - The remaining 12 ACs are still red — the new module doesn't exist yet.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs:1697-:1770` — `GCodeCommand` enum.
  - `crates/slicer-host/src/gcode_emit.rs:300-:340` — `DefaultGCodeEmitter::emit_gcode`.
  - `crates/slicer-host/src/gcode_emit.rs:670-:740` — header serialization (ensure no unintended interaction).
  - `crates/slicer-host/src/gcode_emit.rs:1100-:1170` — `serialize_gcode` body + M82/M83 writes at `:1154-1156`.
  - `crates/slicer-host/src/gcode_emit.rs:1270-:1300` — per-command renderer arms (`Temperature` at `:1280-1281` is the precedent shape).
  - `wit/deps/ir-types.wit` (full — ≤ 200 lines).
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`
  - `crates/slicer-host/src/gcode_emit.rs`
  - `wit/deps/ir-types.wit` (only if the first Step 3 dispatch confirms `gcode-command` is mirrored in WIT).
- Files explicitly out-of-bounds for this step:
  - The rest of `crates/slicer-host/src/gcode_emit.rs` outside the three ranges above.
  - `modules/core-modules/**` — Step 4.
  - `crates/slicer-host/tests/machine_start_end_gcode_emission_tdd.rs` — written in Step 2; do not edit.
  - `crates/slicer-host/src/dispatch.rs`, `crates/slicer-host/src/wit_host.rs` — NOT TOUCHED in this packet.
- WIT/Type Changes Checklist (CLAUDE.md, MANDATORY IF `wit/deps/ir-types.wit` is touched):
  - Sub-step (a): Grep `wit_host.rs`, `dispatch.rs`, and `wit_guest` modules for any `gcode-command` reference. The list informs whether bindgen needs a regen.
  - Sub-step (b): Verify type identity across component boundaries — `absolute: bool` on the Rust side corresponds to `absolute: bool` on the WIT side.
  - Sub-step (c): Run `cargo build --tests` after the WIT addition.
- Expected sub-agent dispatches:
  - `Q: "In wit/deps/ir-types.wit, does the WIT 'gcode-command' variant mirror the Rust GCodeCommand enum? Return FACT: yes/no + the line containing the WIT variant declaration if yes, ≤ 8 lines." | Scope: wit/deps/ir-types.wit | Return format: FACT`
  - `Q: "Run 'cargo build --tests' after adding GCodeCommand::ExtrusionMode + emitter push + serializer arm + (conditional) WIT variant. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first error on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test gcode_emit_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail (with first failing test name + assertion)." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- extrusion_mode_still_emitted_after_promotion --nocapture'. Return FACT pass/fail." | Scope: workspace | Return format: FACT`
- Context cost: `M`.
- Authoritative docs:
  - `docs/02_ir_schemas.md` — `GCodeCommand` section (range read).
  - `docs/03_wit_and_manifest.md` — WIT extension protocol (only if WIT is touched).
- OrcaSlicer refs: none in this step (orderings change in Step 4 when the module prepends).
- Verification:
  - `cargo build --tests` clean.
  - `cargo test -p slicer-host --test gcode_emit_tdd` GREEN.
  - `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- extrusion_mode_still_emitted_after_promotion --nocapture` GREEN.
- Exit condition: All four dispatches return FACT pass; 1 of 13 ACs is green; the other 12 are still red.

### Step 4: Create `modules/core-modules/machine-gcode-emit/` with real `run_gcode_postprocess`

- Task IDs:
  - `TASK-194a`
  - `TASK-194` (the module produces TASK-194's emission)
- Objective: Create the new core module that performs real `[key]` substitution and rebuilds `GCodeIR.commands` as `[Raw(start), ...existing..., Raw(end)]`. Three files: `machine-gcode-emit.toml` (manifest with `[stage] id = "PostPass::GCodePostProcess"`), `Cargo.toml` (mirrors part-cooling), `src/lib.rs` (~120-150 LOC including a private `substitute_placeholders` helper). Build via `./modules/core-modules/build-core-modules.sh`; confirm `--check` clean. Confirm `./test-guests/build-test-guests.sh --check` clean (the Step 3 `slicer-ir` variant addition invalidates universal-guest-dep bindgen).
- Precondition: Step 3 complete; the `ExtrusionMode` regression sentry AC is green.
- Postcondition:
  - New module folder present with three files.
  - `run_gcode_postprocess` body: reads four keys → builds HashMap lookup → runs `substitute_placeholders` on both templates → pushes `Raw(start)` (if non-empty) → re-emits each input command → pushes `Raw(end)` (if non-empty).
  - `./modules/core-modules/build-core-modules.sh --check` clean.
  - `./test-guests/build-test-guests.sh --check` clean.
  - All remaining 12 ACs turn green.
- Files allowed to read:
  - `modules/core-modules/part-cooling/part-cooling.toml` (full — ≤ 100 lines).
  - `modules/core-modules/part-cooling/Cargo.toml` (full — ≤ 25 lines).
  - `modules/core-modules/part-cooling/src/lib.rs` (full — ≤ 150 LOC).
  - `modules/core-modules/seam-placer/seam-placer.toml` (full — ≤ 50 lines) — string-type precedent.
  - `crates/slicer-sdk/src/traits.rs` (ranged) — find the GCodePostProcess trait + signature.
  - `wit/world-postpass.wit` (full — short).
  - `wit/deps/ir-types.wit` (full — short) — `gcode-output-builder` resource methods.
  - `design.md` "TOML Manifest Shape (verbatim)" and "src/lib.rs shape" sections.
- Files allowed to edit (≤ 3):
  - `modules/core-modules/machine-gcode-emit/machine-gcode-emit.toml` (NEW).
  - `modules/core-modules/machine-gcode-emit/Cargo.toml` (NEW).
  - `modules/core-modules/machine-gcode-emit/src/lib.rs` (NEW).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/**` — Step 3 closed.
  - `crates/slicer-ir/src/**` — Step 3 closed.
  - All test files — Step 2 wrote them; do not edit.
  - All other core modules beyond `part-cooling` and `seam-placer`.
- Expected sub-agent dispatches:
  - `Q: "In crates/slicer-sdk/src/traits.rs, find the trait that GCodePostProcess WASM modules implement (likely named GCodePostProcessModule or similar, mirroring FinalizationModule). Return FACT: trait name + full signature of the on_print_start and run_gcode_postprocess methods + file:line. ≤ 20 lines." | Scope: crates/slicer-sdk/src/traits.rs | Return format: FACT`
  - `Q: "In wit/world-postpass.wit, what is the WIT world name that GCodePostProcess modules declare in their manifest? (Confirm 'slicer:world-postpass@1.0.0' or whatever the file says.) Return FACT: world identifier string + file:line, ≤ 4 lines." | Scope: wit/world-postpass.wit | Return format: FACT`
  - `Q: "After writing modules/core-modules/machine-gcode-emit/{machine-gcode-emit.toml, Cargo.toml, src/lib.rs}, run './modules/core-modules/build-core-modules.sh' (no --check) to produce the .wasm. Return FACT: 'build succeeded' or SNIPPETS (≤ 30 lines) of first build error." | Scope: modules/core-modules/ | Return format: FACT`
  - `Q: "Run './modules/core-modules/build-core-modules.sh --check'. Return FACT: '--check returned clean' or SNIPPETS (≤ 20 lines) of the STALE list." | Scope: modules/core-modules/ | Return format: FACT`
  - `Q: "Run './test-guests/build-test-guests.sh --check'. Return FACT clean/stale. The Step 3 slicer-ir variant addition invalidates test-guest bindgen." | Scope: test-guests/ | Return format: FACT`
  - `Q: "Locate the host manifest-discovery API for module [config.schema.<key>] lookup (likely crates/slicer-host/src/manifest.rs). Return FACT: API name + file:line (≤ 6 lines). If no test-friendly API exists, return FACT: 'no direct API; AC uses CONFIG_BLOCK fallback'." | Scope: crates/slicer-host/src/manifest.rs | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd'. Return FACT (pass/fail); SNIPPETS (≤ 20 lines) of first failing test on fail." | Scope: workspace | Return format: FACT`
- Context cost: `M`.
- Authoritative docs:
  - `docs/03_wit_and_manifest.md` — `[config.schema.<key>]` section.
  - `docs/05_module_sdk.md` — GCodePostProcess trait shape.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` (key-name spelling, FACT ≤ 6 lines).
  - `OrcaSlicerDocumented/src/libslic3r/PlaceholderParser.cpp` (`apply_config()` only, FACT ≤ 10 lines).
- Verification:
  - Both `--check` commands clean.
  - All 13 ACs in `machine_start_end_gcode_emission_tdd` green.
- Exit condition: Module folder present + builds; both `--check` commands clean; all 13 ACs green; no regression in other test suites (verified in Step 5).

### Step 5: Regression sweep + workspace gates

- Task IDs:
  - `TASK-194`
  - `TASK-194a`
  - `TASK-194b`
- Objective: Confirm no broader regression and that workspace lint gates remain clean.
- Precondition: Step 4 complete; all 13 ACs green.
- Postcondition:
  - `cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd` green.
  - `cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd` green.
  - `./modules/core-modules/build-core-modules.sh --check` clean.
  - `./test-guests/build-test-guests.sh --check` clean.
  - `cargo check --workspace` clean.
  - `cargo clippy --workspace -- -D warnings` clean.
- Files allowed to read: none direct.
- Files allowed to edit (≤ 3): zero or one (for minor clippy fixes only; if a fix requires more than 5 LOC across more than 1 file, STOP and split into a follow-up step).
- Files explicitly out-of-bounds for this step:
  - Any file not flagged by clippy / cargo check.
- Expected sub-agent dispatches:
  - `Q: "Run 'cargo test -p slicer-host --test postpass_gcode_emit_contract_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo test -p slicer-host --test gcode_header_thumbnail_config_blocks_tdd'. Return FACT pass/fail; SNIPPETS (≤ 20 lines) on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run './modules/core-modules/build-core-modules.sh --check'. Return FACT clean/stale." | Scope: modules/core-modules/ | Return format: FACT`
  - `Q: "Run './test-guests/build-test-guests.sh --check'. Return FACT clean/stale." | Scope: test-guests/ | Return format: FACT`
  - `Q: "Run 'cargo check --workspace'. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first error on fail." | Scope: workspace | Return format: FACT`
  - `Q: "Run 'cargo clippy --workspace -- -D warnings'. Return FACT pass/fail; SNIPPETS (≤ 30 lines) of first warning on fail." | Scope: workspace | Return format: FACT`
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - All six FACTs above return pass.
- Exit condition: All six FACTs green; any minor clippy fix is single-file and ≤ 5 LOC.

### Step 6: Packet completion gate

- Task IDs:
  - `TASK-194`
  - `TASK-194a`
  - `TASK-194b`
- Objective: Final acceptance ceremony — re-dispatch every pipe-suffixed AC command from `packet.spec.md`, run `cargo test --workspace` ONCE per CLAUDE.md test discipline, mark docs/07 rows `[x]`, prepare `packet.spec.md` for status flip.
- Precondition: Step 5 complete.
- Postcondition:
  - Every pipe-suffixed AC command in `packet.spec.md` re-dispatched and FACT-pass.
  - `cargo test --workspace` returns FACT pass.
  - `docs/07_implementation_status.md` rows for TASK-194 / TASK-194a / TASK-194b updated to `[x]`.
  - `packet.spec.md` frontmatter ready to flip from `draft` to `implemented` — the implementer asks the user before flipping.
- Files allowed to read: none direct.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` — via worker dispatch only.
  - `.ralph/specs/59_machine-start-end-gcode-emission/packet.spec.md` — status flip ONLY after explicit user OK.
- Files explicitly out-of-bounds for this step:
  - All source crates (closure ceremony is read/dispatch only).
- Expected sub-agent dispatches:
  - 13 FACT dispatches: re-run every pipe-suffixed `cargo test -p slicer-host --test machine_start_end_gcode_emission_tdd -- <test_name> --nocapture` command from `packet.spec.md`. Each returns FACT pass/fail.
  - `Q: "Run 'cargo test --workspace'. Return FACT pass/fail; SNIPPETS (≤ 40 lines) of first failing test name + assertion on fail. NEVER return the full test output (the suite is > 1000 tests)." | Scope: workspace | Return format: FACT`
  - `Q: "Update docs/07_implementation_status.md rows for TASK-194, TASK-194a, TASK-194b from '[ ]' to '[x]'. Return FACT: 'rows updated' + the three new full-row lines after edit." | Scope: docs/07_implementation_status.md | Return format: FACT`
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

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | docs-only edit via worker dispatch; ≤ 3 dispatches; no source code read. |
| Step 2 | M | One new test file (≤ 400 LOC); reads 2 sibling test files + 1 ranged; 2 dispatches; assertion-encode all 13 ACs. |
| Step 3 | M | Additive `GCodeCommand` variant + emitter push + serializer arm (~17 LOC net across 2 source files, conditionally 1 WIT line); CLAUDE.md WIT/Type Changes Checklist when WIT touched; 4 dispatches; 1 AC turns green. |
| Step 4 | M | Three new module files (~120-150 LOC for `src/lib.rs` with real substitution); two `--check` re-runs; 7 dispatches; remaining 12 ACs turn green. |
| Step 5 | S | Dispatch-only; ≤ 1 LOC minor clippy fix if any; 6 FACT dispatches. |
| Step 6 | S | Dispatch-only closure ceremony; 13 + 1 + 1 = 15 FACT dispatches; docs/07 + packet.spec.md status flip. |
| **Aggregate** | **M** | **3×S + 3×M = M aggregate**; no step is L. |

## Packet Completion Gate

- All 6 steps complete.
- Every step exit condition is met.
- All 13 packet acceptance criteria are FACT-green.
- `./modules/core-modules/build-core-modules.sh --check` and `./test-guests/build-test-guests.sh --check` both return clean at packet closure (re-verified during Step 5).
- `docs/07_implementation_status.md` updated for TASK-194, TASK-194a, TASK-194b to status `[x]` (via worker dispatch).
- No prior packet status transition needed.
- `packet.spec.md` ready to flip from `status: draft` to `status: implemented` — the implementer asks the user explicitly before flipping.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (13 total) — each FACT pass.
- Dispatch `cargo test --workspace` ONCE (per CLAUDE.md test discipline: forbidden during implementation, REQUIRED at packet closure). Worker returns FACT pass; on fail, SNIPPETS (≤ 40 lines).
- Confirm packet-level verification commands are green (`cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, both `--check` commands).
- Record any remaining packet-local risk explicitly before moving to `status: implemented`. Known risks from `design.md`:
  - `gcode_emit_tdd.rs` regression after M82/M83 promotion (Step 3 falsifying check).
  - Cross-component WARN-log forwarding dropped from negative AC (documented; future packet tracked).
  - End-block position differs from OrcaSlicer (intentional deviation; documented).
  - Range enforcement declarative-only (intentional; future packet tracked).
  - Multi-line `machine_start_gcode` CONFIG_BLOCK wire format (whatever packet 55 chose; recorded in Step 4 dispatch return).
- Confirm the implementer's peak context usage stayed under 70%. If not, log it as a packet-authoring lesson for future spec-packet-generator runs.
