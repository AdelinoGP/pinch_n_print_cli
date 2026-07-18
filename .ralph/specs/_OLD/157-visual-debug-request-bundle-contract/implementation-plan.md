# Implementation Plan: 157-visual-debug-request-bundle-contract

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-267`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and is independently specified.

## Steps

### Step 1: Inventory the existing CLI and test seam

- Task IDs: `TASK-267`
- Objective: Locate the existing `pnp_cli` parser/dispatch entry point and the smallest test target seam for a separate visual-debug command.
- Precondition: The named governing docs establish the command syntax and this packet's directory-level code surface.
- Postcondition: The implementer has an exact symbol/file inventory and confirms `slice` remains outside the edit surface.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/**`
  - `crates/pnp-cli/tests/**`
- Files allowed to edit (at most 3):
  - None; discovery only.
- Files explicitly out of bounds:
  - All runtime, scheduler, module, WIT, IR, progress-event, renderer, G-code parser, and other packet files.
- Expected sub-agent dispatches:
  - Question: Which parser, command enum, dispatch function, and test harness own the new command?; scope: `crates/pnp-cli/src/**`, `crates/pnp-cli/tests/**`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` lines 61-73 and 223-235.
  - `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` lines 15-22.
- OrcaSlicer refs:
  - None; parity does not apply.
- Verification:
  - `git diff --exit-code -- crates/pnp-cli crates/slicer-runtime crates/slicer-scheduler` - FACT clean after discovery.
- Exit condition: Exact command and test ownership locations are recorded without editing any file.

### Step 2: Add request validation and focused red tests

- Task IDs: `TASK-267`
- Objective: Establish tests for version, snake_case request fields, exclusive source modes, standalone width requirement, and bounded resolution scale.
- Precondition: Step 1 identified the parser and test target seam.
- Postcondition: Focused tests fail only for the unimplemented visual-debug request contract and enumerate AC-1 through AC-4 and AC-N1 through AC-N4.
- Files allowed to read, with ranges when over 300 lines:
  - The Step 1 `LOCATIONS` results.
  - `crates/pnp-cli/src/**` at identified command/parser ranges.
  - `crates/pnp-cli/tests/**` at the identified harness ranges.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/<identified command files>`
  - `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs`
- Files explicitly out of bounds:
  - `pnp_cli slice` behavior, runtime/scheduler/module/WIT/IR/event paths, renderers, taps, G-code parsing, and all other packet files.
- Expected sub-agent dispatches:
  - Question: Do the red tests assert exact JSON keys, source-mode exclusivity, scale values, and `gcode_line_width_mm` rejection?; scope: changed command/test files; return: `FACT`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` lines 61-97.
  - `docs/19_visual_debug.md` lines 16-33 and 45-50.
  - `docs/11_operational_governance_and_acceptance_gate.md` lines 64-82.
- OrcaSlicer refs:
  - None; parity does not apply.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd -- --list 2>&1 | rg "visual_debug|request|resolution|source"` - FACT with the focused test names.
- Exit condition: Red tests name every validation rule and no test requires taps, rendering, or scheduler closure.

### Step 3: Implement bundle lifecycle, overwrite policy, and manifest model

- Task IDs: `TASK-267`
- Objective: Implement atomic success/failure lifecycle, explicit overwrite handling, and deterministic `manifest.json` serialization with the documented index and image-entry fields.
- Precondition: Request validation tests exist and the command/test symbols are known.
- Postcondition: Valid requests reach a manifest-producing success state; non-empty output without `--overwrite` and all directory/PNG write failures are fatal; no partial bundle is reported successful.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/<identified command files>`
  - `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs`
  - `docs/specs/visual-pipeline-debug.md` lines 99-131 only.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/<identified command files>`
  - `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs`
- Files explicitly out of bounds:
  - Stage taps, scheduler dependency closure, renderers, PNG encoding, G-code parsing, viewport/palette/legend implementation, ordinary `slice`, runtime events, WIT, IR, module manifests, and other packet files.
- Expected sub-agent dispatches:
  - Question: Does the lifecycle guarantee manifest-only machine-readable indexing, explicit overwrite, fatal write failure, and no successful partial bundle?; scope: changed command/test files; return: `FACT`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` lines 70-73 and 112-131.
  - `docs/19_visual_debug.md` lines 35-50.
  - `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` lines 17-32.
  - `docs/11_operational_governance_and_acceptance_gate.md` lines 40-71 and 102-117.
- OrcaSlicer refs:
  - None; parity does not apply.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_request_bundle_tdd` - FACT pass/fail, bounded failure snippets.
- Exit condition: AC-1 through AC-5 and AC-N1 through AC-N6 pass, with no tap/rendering behavior introduced.

### Step 4: Run packet gates and inspect the bounded diff

- Task IDs: `TASK-267`
- Objective: Compile, lint, and verify only the packet's command/test surface while checking that ordinary slice and unrelated contract files are untouched.
- Precondition: Steps 2 and 3 are implemented and focused tests pass.
- Postcondition: All packet gates pass and the diff contains only the authorized CLI/test changes.
- Files allowed to read, with ranges when over 300 lines:
  - Changed files from Steps 2 and 3.
  - `docs/07_implementation_status.md` lines 239-243 for task cross-check only.
- Files allowed to edit (at most 3):
  - None; verification only.
- Files explicitly out of bounds:
  - Every source, test, doc, packet, generated, target, and lockfile path not changed by Steps 2 and 3.
- Expected sub-agent dispatches:
  - Question: Do the packet gates pass and does the diff stay within TASK-267's no-taps/no-rendering boundary?; scope: `git diff --stat` plus gate results; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` lines 239-243.
  - `docs/09_progress_events.md` lines 1-5 and 111-114.
- OrcaSlicer refs:
  - None; parity does not apply.
- Verification:
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `git diff --check` - FACT pass/fail.
- Exit condition: Focused tests, workspace check, clippy, and diff check pass; no prohibited implementation surface is changed.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | CLI/test seam inventory only |
| Step 2 | M | Request validation and red tests |
| Step 3 | M | Lifecycle and manifest contract |
| Step 4 | S | Bounded verification |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile TASK-267 status with packet 157 and preserve packet 158/160 dependency ordering.
- `packet.spec.md` is ready for `status: implemented` after independent review.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk, especially future manifest evolution required by tap/render packets.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
