## Execution Rules

- Steps are atomic and ordered. Do not skip ahead.
- Each step declares files-allowed-to-read, files-allowed-to-edit (≤ 3), expected sub-agent dispatches, and a context cost. No step may carry cost L; if a step balloons, split it.
- Verification commands are delegation-friendly: each emits a single PASS/FAIL line via cargo's summary or, where noted, a parseable single-line `rg` count.
- After every step, the implementer reports: step number, the falsifying-check result (PASS / FAIL with the failing assertion line), and remaining context budget.
- The packet-completion gate (Step 8) is the last step and includes the workspace-level acceptance ceremony plus the predecessor-flip on packet 21.

## Steps

### Step 1: Add `RetractMode` enum and extend `GCodeCommand` variants in `slice_ir.rs`

- Task IDs:
  - `TASK-120d2` (extension)
- Objective:
  Introduce a new `RetractMode` enum (`Gcode`, `Firmware`) and add a `mode: RetractMode` field to `GCodeCommand::Retract` and `GCodeCommand::Unretract`. Update every match arm and SDK shim in the workspace that destructures these two variants.
- Precondition:
  Workspace builds clean on master; the failing test `benchy_gcode_contains_balanced_retract_and_unretract_pairs` is the only red test in `benchy_end_to_end_tdd`.
- Postcondition:
  `crates/slicer-ir/src/slice_ir.rs` declares `pub enum RetractMode { Gcode, Firmware }` (plus the IR module's standard derives). `GCodeCommand::Retract` and `GCodeCommand::Unretract` carry `mode: RetractMode`. `cargo build --workspace` is green. Every existing producer of these commands receives `RetractMode::Gcode` to preserve current behavior; the test reframing is NOT done in this step.
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs` (lines 1420-1450 only — symbol-search for `GCodeCommand`, read ±40 lines)
  - Any file surfaced by the Step 1 discovery dispatch (read only the lines containing the match arm)
- Files allowed to edit (≤ 3 in this step; if discovery surfaces more, split into Step 1a):
  - `crates/slicer-ir/src/slice_ir.rs`
  - One additional producer/consumer file if discovery surfaces it (most likely the SDK shim or `gcode_emit.rs`'s match arms)
  - `modules/core-modules/path-optimization-default/src/lib.rs` (only the two call sites at 269-271 and 290-292; receive `RetractMode::Gcode`)
- Expected sub-agent dispatches:
  - **Question:** "Where in the workspace are `GCodeCommand::Retract` and `GCodeCommand::Unretract` constructed (`push_retract`, `push_unretract`, or struct literal) or destructured (`match` arms, `if let`)? Return file:line for each, with a 1-line context. Cap 25 entries." **Scope:** workspace excluding `target/`. **Return:** LOCATIONS.
  - **Question:** "Run `cargo build -p slicer-ir` and report PASS/FAIL with the first error line if any." **Scope:** workspace. **Return:** FACT.
- Context cost: S
- Authoritative docs: `docs/02_ir_schemas.md` (IR additivity rules).
- OrcaSlicer refs: none for this step.
- Verification: `cargo build --workspace` (must be PASS); follow-up sanity `cargo test -p slicer-ir` (must be PASS).
- Falsifying check / exit condition: if `cargo build --workspace` fails or any pre-existing test outside `benchy_end_to_end_tdd` regresses, stop and revert.

### Step 2: Add `retract_mode` config field in path-optimization-default manifest and read it in the module

- Task IDs:
  - `TASK-120d2` (extension)
- Objective:
  Declare `[config.schema.retract_mode]` in the module manifest and read the value in `on_print_start`, storing it on the module struct. Verify the host's manifest validator accepts the field and rejects an out-of-enum value.
- Precondition:
  Step 1 complete; `RetractMode` enum exists and is reachable from `path-optimization-default`.
- Postcondition:
  `modules/core-modules/path-optimization-default/path-optimization-default.toml` contains a `[config.schema.retract_mode]` block with `type = "enum"`, `values = ["gcode", "firmware"]`, `default = "gcode"`, `display = "Retraction Mode"`, `group = "Travel Retraction"`. The module struct in `lib.rs` holds `retract_mode: RetractMode`, defaulted via the manifest. `on_print_start` reads the field once and stores it. `cargo build -p path-optimization-default` is green. A new host-side test `config_schema_rejects_unknown_retract_mode` exists and PASSes by asserting that loading a manifest override `retract_mode = "marlin"` produces a config-validation error that names the field and the value.
- Files allowed to read:
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml` (lines 30-60 only)
  - `modules/core-modules/path-optimization-default/src/lib.rs` (lines 180-310 only)
  - Host config-validator test fixtures — discover via dispatch below
- Files allowed to edit (≤ 3):
  - `modules/core-modules/path-optimization-default/path-optimization-default.toml`
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - The host config-validation test file surfaced by the dispatch below
- Expected sub-agent dispatches:
  - **Question:** "Where in `crates/slicer-host` is module-manifest enum-field validation tested? Find an existing test that asserts a config-validation error for an out-of-enum value, return its file:line and the assertion shape." **Scope:** `crates/slicer-host/`. **Return:** LOCATIONS + 1 SNIPPET ≤ 20 lines of the assertion shape.
  - **Question:** "Run `cargo test -p path-optimization-default` and report PASS/FAIL with failing assertion line if any." **Return:** FACT.
- Context cost: S
- Authoritative docs: `docs/03_wit_and_manifest.md` (Module Manifest Schema, Config Field Types Reference); `docs/05_module_sdk.md` (`on_print_start` lifecycle).
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` (use_firmware_retraction declaration — confirm naming intent only).
- Verification: `cargo test -p slicer-host config_schema_rejects_unknown_retract_mode -- --exact --nocapture` (PASS) and `cargo build -p path-optimization-default` (PASS).
- Falsifying check / exit condition: if the manifest validator silently accepts `retract_mode = "marlin"`, the validator wiring is missing — stop and surface to the user before proceeding.

### Step 3: Propagate `RetractMode` into pushed `GCodeCommand::Retract` / `Unretract`

- Task IDs:
  - `TASK-120d2` (extension)
- Objective:
  In `path-optimization-default`'s `run_path_optimization`, replace `output.push_retract(self.retract_length, self.retract_speed)` and `output.push_unretract(self.retract_length, self.retract_speed)` with calls that carry `self.retract_mode`. Add a unit test that proves propagation in both directions.
- Precondition:
  Steps 1 and 2 complete; the SDK shim accepts the mode parameter; the module struct has `retract_mode`.
- Postcondition:
  Both call sites at `modules/core-modules/path-optimization-default/src/lib.rs:269-271` and `:290-292` pass `self.retract_mode`. A new unit test `retract_mode_propagates_into_ir_commands` runs the module in two configurations (default and `retract_mode = "firmware"`) on a synthetic single-layer fixture and asserts that every `GCodeCommand::Retract` and `GCodeCommand::Unretract` written into the output collection carries the expected `mode`.
- Files allowed to read:
  - `modules/core-modules/path-optimization-default/src/lib.rs` (lines 250-310)
  - Existing unit-test scaffolding within the same crate
- Files allowed to edit (≤ 3):
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - A sibling tests module if the crate uses one
- Expected sub-agent dispatches:
  - **Question:** "Run `cargo test -p path-optimization-default retract_mode_propagates_into_ir_commands -- --exact` and report PASS/FAIL." **Return:** FACT.
- Context cost: S
- Authoritative docs: `docs/02_ir_schemas.md`, `docs/05_module_sdk.md`.
- OrcaSlicer refs: none.
- Verification: `cargo test -p path-optimization-default retract_mode_propagates_into_ir_commands -- --exact --nocapture` (PASS).
- Falsifying check / exit condition: if either configuration produces commands with the wrong mode, the SDK shim or the call-site updates are wrong — fix in this step before moving on.

### Step 4: Branch `DefaultGCodeEmitter` on `mode` and add the per-command dispatch unit test

- Task IDs:
  - `TASK-120d2` (extension)
- Objective:
  Update the two match arms at `crates/slicer-host/src/gcode_emit.rs:410-426` so `Retract { mode: RetractMode::Gcode, length, speed }` writes `G1 E-{length} F{speed}\n`, `Retract { mode: RetractMode::Firmware, .. }` writes `G10\n`, `Unretract { mode: RetractMode::Gcode, length, speed }` writes `G1 E{length} F{speed}\n`, `Unretract { mode: RetractMode::Firmware, .. }` writes `G11\n`. Add `gcode_emit_dispatches_per_command_retract_mode` unit test.
- Precondition:
  Steps 1-3 complete; the IR carries `mode`; the module pushes the right mode.
- Postcondition:
  `gcode_emit.rs` lines 410-426 contain a per-command branch on `mode`. A new unit test feeds a synthetic `LayerCollectionIR` with one Gcode-mode retract immediately followed by one Firmware-mode retract and asserts the output contains exactly one `G1 E-` line and exactly one `G10` line, in order, with no extra retract-style lines. `cargo build -p slicer-host` is green.
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs` (lines 380-440 only)
  - Existing emitter unit-test scaffolding (discover via dispatch below)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs`
  - Existing emitter unit-test file (or an inline `#[cfg(test)]` module in `gcode_emit.rs`)
- Expected sub-agent dispatches:
  - **Question:** "Where are the existing `DefaultGCodeEmitter` unit tests? Return file:line for the test module and one example test signature." **Scope:** `crates/slicer-host/`. **Return:** LOCATIONS + 1 SNIPPET ≤ 15 lines.
  - **Question:** "Run `cargo test -p slicer-host gcode_emit_dispatches_per_command_retract_mode -- --exact` and report PASS/FAIL." **Return:** FACT.
- Context cost: S
- Authoritative docs: `docs/02_ir_schemas.md` (GCodeCommand contract).
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` (firmware vs G-code branch — for parity confirmation only, no code copying).
- Verification: `cargo test -p slicer-host gcode_emit_dispatches_per_command_retract_mode -- --exact --nocapture` (PASS); `cargo build -p slicer-host` (PASS).
- Falsifying check / exit condition: if the emitter regresses on the existing G-code-mode behavior (any change to the `G1 E-{length} F{speed}` line for default mode), revert the emit-arm rewrite and split it into a smaller change.

### Step 5: Reframe the failing E2E assertion against the actual default G-code-mode artifact format

- Task IDs:
  - `TASK-120d2`
  - `TASK-135` (partial — retract/unretract family only)
- Objective:
  Update `crates/slicer-host/tests/benchy_end_to_end_tdd.rs::benchy_gcode_contains_balanced_retract_and_unretract_pairs` (lines 1208-1255) to assert balanced `G1 E-` retracts and balanced `G1 E<positive>` unretracts under the default config, and to assert zero `G10`/`G11` lines (NC-1).
- Precondition:
  Steps 1-4 complete; the live pipeline now emits per-mode retracts; the default mode is `Gcode`.
- Postcondition:
  The test counts `G1 E-` lines (retract) and matches `G1 E[0-9]` lines that are immediately preceded by a retract (unretract). Counts must be `> 0` and equal. The test also asserts `gcode.lines().filter(|l| l.trim() == "G10" || l.trim() == "G11").count() == 0`. Each assertion's failure message includes a `preview(&gcode, 30)` snippet, matching the file's existing diagnostic style.
- Files allowed to read:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (lines 1200-1260 only)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
- Expected sub-agent dispatches:
  - **Question:** "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_balanced_retract_and_unretract_pairs -- --exact` and report PASS/FAIL with assertion line if FAIL." **Return:** FACT.
- Context cost: S
- Authoritative docs: none (test reframing is local to this packet's contract).
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_contains_balanced_retract_and_unretract_pairs -- --exact --nocapture` (PASS).
- Falsifying check / exit condition: if the assertion still fails after Step 4 emitter updates, the producer chain is wrong — diagnose by running Step 3's unit test plus the emitter unit test before re-editing this test.

### Step 6: Add the firmware-mode E2E test with a config override

- Task IDs:
  - `TASK-120d2` (new acceptance)
  - `TASK-135` (partial — firmware family)
- Objective:
  Add `benchy_gcode_firmware_retraction_emits_balanced_g10_g11` to `benchy_end_to_end_tdd.rs`. The test runs the same Benchy fixture as the reframed test but overrides the path-optimization-default module config so `retract_mode = "firmware"`. Asserts: `G10` line count equals `G11` line count, both `> 0`, AND zero `G1 E-` retract-style lines (NC-2).
- Precondition:
  Steps 1-5 complete; default-mode test green.
- Postcondition:
  New test exists and is green. The config override mechanism used is the same one used elsewhere in this test file (discover via dispatch); no new config-injection plumbing is added here.
- Files allowed to read:
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs` (whichever test in the same file already overrides a module config — discover via dispatch)
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
- Expected sub-agent dispatches:
  - **Question:** "In `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`, find a test that overrides a path-optimization-default config field (e.g., `retract_length`, `travel_z_hop`) before running the live pipeline. Return file:line and ≤ 20 lines of the override mechanism." **Scope:** that file only. **Return:** LOCATIONS + 1 SNIPPET ≤ 20 lines.
  - **Question:** "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_firmware_retraction_emits_balanced_g10_g11 -- --exact` and report PASS/FAIL." **Return:** FACT.
- Context cost: M (requires understanding the existing config-override pattern; bounded to one new test function in one file)
- Authoritative docs: none.
- OrcaSlicer refs: `OrcaSlicerDocumented/src/libslic3r/GCodeWriter.cpp` (G10/G11 expected output — parity confirmation only).
- Verification: `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_gcode_firmware_retraction_emits_balanced_g10_g11 -- --exact --nocapture` (PASS).
- Falsifying check / exit condition: if no existing test in the file overrides a path-optimization-default config field, surface to the user before inventing a new injection mechanism — that would be out of scope.

### Step 7: Mark packet 21 superseded and absorb its retract/unretract acceptance evidence

- Task IDs:
  - `TASK-120d2` (housekeeping)
- Objective:
  Edit `.ralph/specs/21_benchy-acceptance-evidence/packet.spec.md`'s YAML frontmatter `status:` from `draft` to `superseded`, and add a 1-line note in this packet's `requirements.md` Cross-Packet Impact section confirming the flip (already pre-written; no edit if already correct). Do NOT modify any other file in packet 21's directory.
- Precondition:
  Steps 1-6 complete; all per-AC tests green.
- Postcondition:
  `.ralph/specs/21_benchy-acceptance-evidence/packet.spec.md` frontmatter shows `status: superseded`. No other change to packet 21.
- Files allowed to read:
  - `.ralph/specs/21_benchy-acceptance-evidence/packet.spec.md` (frontmatter only — first 15 lines)
- Files allowed to edit (≤ 3):
  - `.ralph/specs/21_benchy-acceptance-evidence/packet.spec.md` (frontmatter only)
- Expected sub-agent dispatches: none (single-line edit).
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: `rg "^status: superseded" .ralph/specs/21_benchy-acceptance-evidence/packet.spec.md` returns exactly one match.
- Falsifying check / exit condition: if packet 21's `packet.spec.md` already shows another non-`draft` status (e.g., someone activated it), stop and surface to the user before flipping.

### Step 8: Packet completion gate — workspace acceptance ceremony

- Task IDs:
  - `TASK-120d2`, `TASK-135` (partial)
- Objective:
  Run the workspace-level backpressure gate, confirm all five per-AC tests pass, and report a single green/red status with the failing assertion if anything regressed.
- Precondition:
  Steps 1-7 complete.
- Postcondition:
  All commands below report PASS.
- Files allowed to read: none beyond what previous steps already touched.
- Files allowed to edit: none in this step.
- Expected sub-agent dispatches:
  - **Question:** "Run `cargo build --workspace` and report PASS/FAIL." **Return:** FACT.
  - **Question:** "Run `cargo test --workspace` and report PASS/FAIL with the first failing test name and assertion if any." **Return:** FACT.
  - **Question:** "Run `cargo clippy --workspace -- -D warnings` and report PASS/FAIL with the first warning if any." **Return:** FACT.
  - **Question:** "Run `cargo test -p slicer-host --test benchy_end_to_end_tdd` and report PASS/FAIL with the first failing test name if any." **Return:** FACT.
- Context cost: S (delegated entirely; no reads in this step)
- Authoritative docs: `docs/11_operational_governance_and_acceptance_gate.md` (acceptance gate criteria).
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo test -p slicer-host --test benchy_end_to_end_tdd`
- Falsifying check / exit condition: any FAIL → return to the most recently touched step, do not advance the packet to `implemented` status.
