# Implementation Plan: 58_gcode-toolchange-purge-integration

## Execution Rules

- One atomic step at a time.
- Each step maps back to one or more of `TASK-143`, `TASK-152b`, `TASK-120d2`.
- TDD first: Step 2 lands failing tests; Step 3 lands the rejection guard; Step 4 lands the emission that makes the positive tests pass.
- Each step honors the context-discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. The fields below are not optional metadata — they are the budget contract for this step.

## Steps

### Step 1: Confirm IR landscape and role table (pure dispatch)

- Task IDs:
  - `TASK-143`
  - `TASK-152b`
- Objective: Confirm `ExtrusionRole::WipeTower` variant existence, `ToolChange` shape, and the role-to-`;TYPE:` mapping function location in `gcode_emit.rs`.
- Precondition: packet is `active` (or implementer has user approval to begin).
- Postcondition: implementer knows whether to add a variant in Step 3 and the exact `gcode_emit.rs` arm to patch.
- Files allowed to read:
  - (none direct — pure dispatch step.)
- Files allowed to edit (≤ 3):
  - (none.)
- Files explicitly out-of-bounds for this step:
  - the rest of `slice_ir.rs`; all of `OrcaSlicerDocumented/`; all source files (Steps 2-4 own those).
- Expected sub-agent dispatches:
  - "Confirm `ExtrusionRole::WipeTower` is still present at `crates/slicer-ir/src/slice_ir.rs:1233-1262` and `ToolChange` field shape at `1435-1442` is unchanged; FACT ≤ 5 lines."
  - "Confirm `orca_type_label` at `crates/slicer-host/src/gcode_emit.rs:218-235` still maps `ExtrusionRole::WipeTower → \";TYPE:Wipe tower\"`; FACT pass/fail."
  - "Confirm `PostpassError` at `crates/slicer-host/src/postpass.rs:39-59` still has the shape `FatalModule { stage_id, module_id, message } | GCodeEmit { message } | GCodeSerialization { message }` with no `MissingToolchangePurge` variant yet; FACT ≤ 5 lines."
  - "Summarize OrcaSlicer `WipeTower2.cpp:1557-1640` Unload/Change/Load/Wipe call order; FACT ≤ 5 lines."
  - "Confirm that no standalone `volume_to_length` helper exists in the codebase, and locate the per-segment forward extrusion math at `crates/slicer-host/src/gcode_emit.rs:363-371` (`E = distance * width * flow_factor`); LOCATIONS ≤ 3 entries. Step 4 implements the inverse `length_mm = volume_mm3 / (line_width_mm * layer_height_mm)` inline within `wipe-tower/src/lib.rs`."
- Context cost: **S**
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate SUMMARY about `ExtrusionRole` and `ToolChange` only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp:1557-1640` — delegate FACT.
- Verification:
  - Five FACT/LOCATIONS/SNIPPETS returns recorded in working notes.
- Exit condition: Implementer can answer (a) is `ExtrusionRole::WipeTower` still at `slice_ir.rs:1233-1262`? (b) what is `ToolChange.after_entity_index`'s exact semantic? (c) does `orca_type_label` at `gcode_emit.rs:218-235` still map the variant to `";TYPE:Wipe tower"`? (d) what is `PostpassError`'s current variant set including `FatalModule`'s full field list (`{stage_id, module_id, message}`) — precondition for the additive `MissingToolchangePurge` insertion in Step 3? (e) confirmed no standalone `volume_to_length` helper — Step 4 will compute the inverse inline. Without these, Step 2 cannot start.

### Step 2: TDD — write the failing tests + land fixtures

- Task IDs:
  - `TASK-143`
  - `TASK-152b`
- Objective: Land `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` with three failing tests: `toolchange_emits_retract_prime_wipe`, `bare_toolchange_rejected`, and `purge_volume_within_tolerance`. Drop in the multi-material STL fixture and the OrcaSlicer reference G-code.
- Precondition: Step 1 complete; implementer has the `ExtrusionRole` answer and the role-mapping function location.
- Postcondition: `cargo test -p slicer-host --test gcode_toolchange_wrapping` compiles and reports three failing tests with assertion messages naming the missing retract/prime/marker.
- Files allowed to read:
  - `crates/slicer-host/tests/tool_ordering_tdd.rs` — full read (small, focused) for fixture-building idioms.
  - `crates/slicer-ir/src/slice_ir.rs:1430-1470` — `ToolChange` definition (range).
  - `crates/slicer-ir/src/slice_ir.rs:1520-1545` — `LayerCollectionIR` definition (range).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/gcode_toolchange_wrapping.rs` (new).
  - `crates/slicer-host/tests/fixtures/multi_color_cube.stl` (new — synthetic 2-color cube, ≤ 64 KB).
  - `crates/slicer-host/tests/fixtures/multi_color_cube.orca.gcode` (new — checked-in OrcaSlicer output for parity baseline, ≤ 256 KB).
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs` (Step 3).
  - `modules/core-modules/wipe-tower/src/lib.rs` (Step 4).
  - any source file other than the new test file.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping`; FACT (expect compile success + 3 test failures); if compile fails, SNIPPETS ≤ 20 lines of the compile error."
- Context cost: **M**
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegate fact-check on `LayerCollectionIR` shape used by the test fixture.
- OrcaSlicer refs:
  - None at this step (the OrcaSlicer reference G-code is dropped in as data, not read into the implementer's context).
- Verification:
  - `cargo test -p slicer-host --test gcode_toolchange_wrapping` — expect 3 failing tests; FAILURES list must contain `toolchange_emits_retract_prime_wipe`, `bare_toolchange_rejected`, `purge_volume_within_tolerance`.
- Exit condition: file compiles, all 3 tests run, all 3 fail with assertion messages naming the missing retract/prime/marker. Fixtures committed.

### Step 3: Emitter — missing-purge guard + additive `PostpassError::MissingToolchangePurge` variant

- Task IDs:
  - `TASK-143`
  - `TASK-120d2`
- Objective: In `crates/slicer-host/src/gcode_emit.rs`, add a guard around the existing T<n> emission at lines 1155-1156 so that, when `wipe_tower_enabled=true`, the ±N entities around the `ToolChange` must include at least one retract entity (negative E) before and at least one wipe-tower-role entity after; otherwise return `Err(PostpassError::MissingToolchangePurge { layer_index, tool_change_index })`. Add the additive variant `MissingToolchangePurge { layer_index: usize, tool_change_index: usize }` to `PostpassError` in `crates/slicer-host/src/postpass.rs:39-59`. `ExtrusionRole::WipeTower` at `slice_ir.rs:1233-1262` and the `orca_type_label` arm at `gcode_emit.rs:218-235` already exist — both are read-only verification only (no edits).
- Precondition: Step 2 complete; the 3 failing tests are landed and compile.
- Postcondition: `bare_toolchange_rejected` passes; `toolchange_emits_retract_prime_wipe` and `purge_volume_within_tolerance` still fail (Step 4 lands the emission).
- Files allowed to read:
  - `crates/slicer-host/src/gcode_emit.rs:1140-1170` (range — bare T<n> writeln at 1155-1156).
  - `crates/slicer-host/src/gcode_emit.rs:218-235` — `orca_type_label` (read-only verification).
  - `crates/slicer-host/src/postpass.rs:39-59` — `PostpassError` definition (for the additive variant).
  - `crates/slicer-ir/src/slice_ir.rs:1435-1442` (range — `ToolChange`).
  - Step 1 dispatch returns (in working notes).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/src/gcode_emit.rs` — add the guard around the bare T<n> writeln.
  - `crates/slicer-host/src/postpass.rs` — add the additive `MissingToolchangePurge { layer_index: usize, tool_change_index: usize }` variant.
  - (third slot unused — `slice_ir.rs` is read-only this step.)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-ir/src/slice_ir.rs` — read-only verification only; the `ExtrusionRole::WipeTower` variant already exists at lines 1233-1262.
  - `modules/core-modules/wipe-tower/src/lib.rs` (Step 4).
  - any test file other than running the existing tests.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace`; FACT pass/fail." (after edit)
  - "Run `cargo clippy --workspace -- -D warnings`; FACT pass/fail." (after edit)
  - "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping bare_toolchange_rejected -- --nocapture`; FACT pass/fail."
- Context cost: **S–M** (reduced from M — no `slice_ir.rs` edit, no role-mapping arm edit).
- Authoritative docs:
  - `docs/02_ir_schemas.md` — additive variant rules.
  - Packet 11's emission contract — confirmed `;TYPE:<RoleName>` form (the `WipeTower` arm at `orca_type_label` already complies).
- OrcaSlicer refs:
  - None at this step (Orca ordering is referenced in Step 4).
- Verification:
  - `cargo check --workspace` — must pass.
  - `cargo clippy --workspace -- -D warnings` — must pass.
  - `cargo test -p slicer-host --test gcode_toolchange_wrapping bare_toolchange_rejected -- --nocapture` — must pass.
  - `cargo test -p slicer-host --test gcode_toolchange_wrapping toolchange_emits_retract_prime_wipe -- --nocapture` — expected to still fail.
- Exit condition: `bare_toolchange_rejected` green; clippy clean; check clean; the other two tests still failing with messages naming the missing wipe-tower entities.

### Step 4: Wipe-tower module emits retract/prime/wipe entities + role marker

- Task IDs:
  - `TASK-143`
- Objective: In `modules/core-modules/wipe-tower/src/lib.rs`, for each `ToolChange` in `LayerCollectionIR.tool_changes` (when `wipe_tower_enabled=true`), insert these `PrintEntity` rows around `ToolChange.after_entity_index`: (a) one retract entity (negative E delta sized per `wipe_tower_purge_volume` retract length), (b) one travel entity to `(wipe_tower_x, wipe_tower_y)`, (c) the tower polygon walls + rectilinear infill rows with `ExtrusionRole::WipeTower`, (d) the wipe rows with the same role, (e) one prime entity whose cumulative positive E delta equals `wipe_tower_purge_volume` mm via the project's `volume_to_length` convention (confirmed by Step 1 dispatch). Insert in a single mutation so `ToolChange.after_entity_index` remains consistent across the loop. Add two `#[cfg(test)] mod tests` cases: `emits_wipe_tower_role_marker` (AC4) and `tower_geometry_within_bed_outside_objects` (AC6).
- Precondition: Step 3 complete.
- Postcondition: `toolchange_emits_retract_prime_wipe` passes; `purge_volume_within_tolerance` passes for the fixture; both new module unit tests pass.
- Files allowed to read:
  - `modules/core-modules/wipe-tower/wipe-tower.toml` — full read (small).
  - `crates/slicer-ir/src/slice_ir.rs:1430-1470` and `1520-1545` (ranges).
  - `crates/slicer-host/src/layer_finalization.rs:80-110` (range).
  - The role-to-`;TYPE:` mapping function location (read-only confirmation that Step 3 wired it correctly).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/wipe-tower/src/lib.rs` — emission logic + the new `#[cfg(test)] mod tests` block.
  - (one helper module under `modules/core-modules/wipe-tower/src/` if the existing layout already splits geometry into a helper — keep it within the wipe-tower module's directory; do NOT add a new file unless the existing layout already does so.)
  - (no third slot expected.)
- Files explicitly out-of-bounds for this step:
  - `crates/slicer-host/src/gcode_emit.rs` (Step 3 done; do not re-edit).
  - All other core-modules (`skirt-brim`, `part-cooling`, `top-surface-ironing`, etc.) — must NOT touch.
  - `crates/slicer-ir/src/slice_ir.rs` (Step 3 done).
- Expected sub-agent dispatches:
  - "Confirm no other `PostPass::LayerFinalization` module reads `LayerCollectionIR.entities.len()` or asserts entity-count invariants; LOCATIONS ≤ 10 entries from `modules/core-modules/{skirt-brim,part-cooling,top-surface-ironing}/src/lib.rs`."
  - "Run `./modules/core-modules/build-core-modules.sh`; FACT exit code + last 5 lines."
  - "Run `cargo test -p wipe-tower --lib`; FACT pass/fail (expect 2 new tests green)."
  - "Run `cargo test -p slicer-host --test gcode_toolchange_wrapping`; FACT pass/fail (expect all 3 green)."
- Context cost: **M**
- Authoritative docs:
  - `docs/08_coordinate_system.md` — direct read for unit math (1 mm = 10,000 units).
  - `docs/03_wit_and_manifest.md` — range-read wipe-tower manifest schema only.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/WipeTower2.cpp:1557-1640` — call-ordering reference (delegated; do not load).
- Verification:
  - `./modules/core-modules/build-core-modules.sh` — WASM rebuild succeeds.
  - `cargo test -p wipe-tower --lib` — all module tests green (including the 2 new unit tests).
  - `cargo test -p slicer-host --test gcode_toolchange_wrapping` — all 3 integration tests green.
- Exit condition: WASM build clean; all module + integration tests green.

### Step 5: End-to-end CLI verification + AC scripts

- Task IDs:
  - `TASK-143`
  - `TASK-152b`
- Objective: Slice the multi-material fixture end-to-end through `slicer-cli` and run the awk/python AC and NC scripts from `packet.spec.md` against the produced G-code.
- Precondition: Steps 1-4 complete.
- Postcondition: `target/test-output/multi_color_cube.gcode` exists. Every AC command (AC1, AC2a, AC2b, AC3, AC4, AC5, AC6) exits 0. NC1 (`cargo test ... bare_toolchange_rejected`) exits 0 — the unit test asserts the rejection path is reached. NC2 and NC3 are regression sentinels that exit 0 against correct gcode (they only exit non-zero against regressed output).
- Files allowed to read:
  - The produced G-code (via awk/grep/python only; never load full).
- Files allowed to edit (≤ 3):
  - (none — script-only verification step.)
- Files explicitly out-of-bounds for this step:
  - source code (no edits this step).
- Expected sub-agent dispatches:
  - "Run `cargo run --bin slicer-cli --release --slice --input crates/slicer-host/tests/fixtures/multi_color_cube.stl --output target/test-output/multi_color_cube.gcode`; FACT exit code + last 5 lines."
  - "Run each AC and NC pipe-suffixed command from `packet.spec.md` against `target/test-output/multi_color_cube.gcode`; FACT pass/fail per command, ≤ 1 line per AC/NC."
- Context cost: **S**
- Authoritative docs:
  - None (verification only).
- OrcaSlicer refs:
  - None (reference `.gcode` is already checked in from Step 2).
- Verification:
  - All 7 AC commands exit 0 (the three `cargo test -p ...` already passed in Step 4; the two awks for AC2a/AC2b and the python for AC5 must pass on the produced file).
  - NC1 exits 0 — `cargo test ... bare_toolchange_rejected` is a unit test of the rejection path; it passes when the negative behavior is correctly implemented.
  - NC2 and NC3 exit 0 against the produced correct gcode — they are regression sentinels (silent on correct output; exit non-zero only against regressed output).
  - **Optional sentinel-teeth proof** (recommended once per packet closure): copy `target/test-output/multi_color_cube.gcode` to `target/test-output/multi_color_cube.corrupted.gcode`, hand-corrupt it (delete the retract line preceding one `T<n>` to forge NC2's bug pattern; delete all `;TYPE:Wipe tower` and `;TYPE:Prime tower` lines to forge NC3's bug pattern), re-run the NC2 and NC3 commands against the corrupted file, and confirm both exit non-zero. Record FACT pass/fail in the closure notes; do NOT commit the corrupted file.
- Exit condition: every AC command exits 0 on the fresh end-to-end output; all three NC commands exit 0 on the correct output; the optional sentinel-teeth proof (if run) confirms NC2/NC3 exit non-zero on a hand-corrupted copy.

### Step 6: DEVIATION_LOG entry + docs/07 status update + packet status flip

- Task IDs:
  - `TASK-143`
  - `TASK-152b`
  - `TASK-120d2`
- Objective: Append one `docs/DEVIATION_LOG.md` entry recording (a) the integration completion across packets 17/19/11, and (b) the AC6 stub-bounds follow-up: the wipe-tower module currently lacks host-service `bed_polygon` access, so AC6 ran against module-internal stubs; real cross-module bed-bounds enforcement is deferred to a follow-up packet. Update `docs/07_implementation_status.md` notes for the three task IDs to reference packet 58's closure. Flip this packet's `status:` from `draft` to `implemented` after the acceptance ceremony completes.
- Precondition: Step 5 complete; all ACs green.
- Postcondition: Deviation log entry present; `docs/07` updated at exactly the three TASK-### lines; this packet's `packet.spec.md` is `status: implemented`.
- Files allowed to read:
  - `docs/11_operational_governance_and_acceptance_gate.md` §1 (range, ≤ 60 lines).
  - `docs/DEVIATION_LOG.md` — read only the most recent 3 entries (via `git log -p -n 3 docs/DEVIATION_LOG.md` through a sub-agent) for format reference; do not load full file.
  - `docs/07_implementation_status.md` — narrow line edits only at the three TASK-### lines (LOCATIONS from sub-agent first); do NOT load full file.
- Files allowed to edit (≤ 3):
  - `docs/DEVIATION_LOG.md` — append exactly one entry.
  - `docs/07_implementation_status.md` — narrow line edits at TASK-143, TASK-152b, TASK-120d2 only.
  - `.ralph/specs/58_gcode-toolchange-purge-integration/packet.spec.md` — flip `status: draft` → `status: implemented` after the acceptance ceremony passes.
- Files explicitly out-of-bounds for this step:
  - All other docs.
  - Source code.
  - Other packets' directories — per the cross-packet mutation rule, do NOT touch `.ralph/specs/17_*`, `.ralph/specs/19_*`, `.ralph/specs/11_*`, `.ralph/specs/15_*`, `.ralph/specs/34_*`.
- Expected sub-agent dispatches:
  - "Locate the line ranges for TASK-143, TASK-152b, TASK-120d2 in `docs/07_implementation_status.md`; LOCATIONS ≤ 6 entries."
  - "Show the most recent 3 entries of `docs/DEVIATION_LOG.md`; SNIPPETS ≤ 30 lines each, for format reference."
- Context cost: **S**
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` §1.
- OrcaSlicer refs:
  - None.
- Verification:
  - `git diff docs/DEVIATION_LOG.md` — shows exactly one appended entry.
  - `git diff docs/07_implementation_status.md` — only the three TASK-### lines edited (no unrelated changes).
  - `git diff .ralph/specs/58_gcode-toolchange-purge-integration/packet.spec.md` — only the `status:` line changed.
- Exit condition: deviation log entry committed, docs/07 updated, packet `packet.spec.md` set to `status: implemented` after the acceptance ceremony.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Pure dispatch — four FACT/SNIPPETS/LOCATIONS returns; no direct file reads. |
| Step 2 | M | New test file + STL + reference G-code; reads `tool_ordering_tdd.rs` and ranged IR. |
| Step 3 | S–M | Emitter guard + additive `PostpassError` variant + clippy clean (no IR edit; role-mapping arm already in place). |
| Step 4 | M | Module emission + 2 new unit tests + WASM rebuild + neighbor invariant check. |
| Step 5 | S | Script-only verification; no source edits. |
| Step 6 | S | Docs update; narrow line edits via sub-agent. |

Aggregate: **M** (within budget; no single step is L).

## Packet Completion Gate

- All 6 steps complete with their exit conditions met.
- Every pipe-suffixed AC and NC command in `packet.spec.md` re-runs PASS (or expected-FAIL for NCs).
- `docs/07_implementation_status.md` updated for TASK-143, TASK-152b, TASK-120d2.
- `docs/DEVIATION_LOG.md` entry recorded.
- `packet.spec.md` ready to move to `status: implemented`.
- Final acceptance-ceremony workspace gate: `cargo test --workspace` returns PASS via sub-agent (the only `--workspace` test invocation in the packet, per the project's Test Discipline rule).

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` and every negative case; record FACT pass/fail per AC and per NC.
- Run `cargo clippy --workspace -- -D warnings` (must pass).
- Run `cargo test --workspace` exactly once via sub-agent with FACT pass/fail return — this is the packet's only workspace-wide invocation, used solely as the closure gate.
- Run `./modules/core-modules/build-core-modules.sh` (must pass).
- Confirm the implementer's peak context usage stayed under 70%; if not, log it as a packet-authoring lesson and consider tightening Step 4's scope in any follow-up.
- Flip `packet.spec.md` `status: draft` → `status: implemented`.
