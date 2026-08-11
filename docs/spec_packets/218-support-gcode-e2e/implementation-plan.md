# Implementation Plan: support-gcode-e2e

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-327`.
- Use TDD, then verification implementation, then the narrowest falsifying validation.
- This packet must not modify production emission behavior.

## Steps

### Step 1: Add the real support-artifact G-code-mode check

- Task IDs: `TASK-327`
- Objective: extend the existing G-code renderer integration test to exercise both named support requests and assert manifest/layer/role evidence.
- Precondition: the gitignored on-disk fixtures `tmp/SupportTest_Normal_Orca.gcode`, `tmp/visual-debug-gcode.json`, and `tmp/visual-debug-gcode2.json` exist; activation-blocking FORWARD-DEPs `TASK-322` through `TASK-325` are implemented.
- Postcondition: one targeted test invokes `run_visual_debug` on the real artifact, validates layer 30 and layers 31-34, checks `final_gcode` entries, and checks both exact raw support labels without asserting typed manifest roles.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - lines `48-194,464-544`
  - `crates/pnp-cli/src/visual_debug.rs` - lines `125-197,686-774`
  - `crates/slicer-gcode/src/emit.rs` - lines `195-249`
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - all production source and emission files
  - named tmp input artifacts
  - generated bundles, `target/`, lockfiles, and Orca source
- Expected sub-agent dispatches:
  - Question: verify `visual_debug_gcode_renderer_tdd` can drive the real filesystem request and manifest path; scope: `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-generation-defect-verified-findings.md` - direct ranges `205-231`
  - `docs/19_visual_debug.md` - direct ranges `17-46,158-180`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/ExtrusionEntity.cpp` - delegate; canonical role labels
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd` - FACT
  - `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-gcode2.json --output target/vd-gcode-e2e-roles --overwrite` - bounded manifest assertion
- Exit condition: targeted test passes against the named artifact after activation-blocking FORWARD-DEPs `TASK-322` through `TASK-325` are implemented, and no production file is changed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Existing integration binary and real artifact |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record any artifact availability risk; do not replace the artifact with synthetic input.
- Confirm context stayed within the standard band.
