# Implementation Plan: 158-visual-debug-typed-tap-capture

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Confirm The `slicer-runtime` Capture Entry Point

- Task IDs: `TASK-268`
- Objective: Packet 157's own symbols are already grounded (`crates/pnp-cli/src/visual_debug.rs:14-370`: `VisualDebugRequest`, `VisualDebugSource::Model`, `TapSelector`, `LayerSelector`, `Manifest`, `ImageEntry`, `run_visual_debug`). This step instead identifies the exact narrowest new `slicer-runtime` pub entry point that `run_visual_debug` can call to run the fixed-stage dependency closure and get back renderer-owned typed capture values, since `slicer-runtime` cannot import `pnp-cli`'s types (dependency direction is `pnp-cli -> slicer-runtime`).
- Precondition: Packet 157 is implemented (commit `3e33ca01`); `run_visual_debug`'s `Model` branch currently performs no real execution (placeholder `ImageEntry` per tap x visualization from `req.taps`/`req.layers.first()`, no tap validation).
- Postcondition: A bounded symbol inventory identifies the existing `slicer-runtime` pub surface (`execution_plan`, `layer_executor`, `run`, `blackboard`) to build on, and the exact new function/module name and runtime-owned return type for the capture entry point, or implementation is blocked without edits.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/lib.rs` - pub surface only.
  - `crates/pnp-cli/src/visual_debug.rs` - complete, 381 lines.
- Files allowed to edit (at most 3):
  - None; read-only discovery.
- Files explicitly out of bounds:
  - Generated code, `target/`, and all other packet directories.
- Expected sub-agent dispatches:
  - Question: Which existing `slicer-runtime` pub function/module is the narrowest seam for running the fixed-stage dependency closure and returning renderer-owned typed capture values to a caller in another crate? Scope: `crates/slicer-runtime/src/lib.rs` pub surface, `crates/slicer-runtime/src/run.rs`, `crates/slicer-runtime/src/layer_executor.rs`; return: `LOCATIONS` at most 20 entries.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug-plan.md` - complete 15-line queue.
  - `docs/specs/visual-pipeline-debug.md` - lines 61-117 and 143-163.
- Verification:
  - Bounded `slicer-runtime` capture-entry-point lookup - `LOCATIONS` or an explicit blocker.
- Exit condition: Exact new `slicer-runtime` entry point name, signature shape, and return type are recorded; otherwise packet remains blocked and no implementation begins.

### Step 2: Add Failing Typed Capture Contract Tests

- Task IDs: `TASK-268`
- Objective: Encode selected-layer capture, post-stage timing, closure stop, expansion accounting, determinism, ordinary-slice no-op, unsupported tap rejection, unavailable-source failure, and empty-layer rejection.
- Precondition: Step 1 provides the exact packet-157 integration seam and a model-backed fixture/request path.
- Postcondition: Focused tests fail for the missing capture behavior and assert exact typed capture/manifest fields without testing PNGs or G-code.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/tests/**` - `visual_debug_request_bundle_tdd.rs` and targeted fixture/helper files only.
  - `crates/pnp-cli/src/visual_debug.rs` - complete, 381 lines.
  - Step 1's identified `slicer-runtime` capture entry point - exact ranges returned by Step 1.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs`
  - `crates/pnp-cli/src/visual_debug.rs` - minimal command-to-runtime dispatch seam only; no changes to `validate_request`, `VisualDebugRequest`, or bundle lifecycle/overwrite/atomic-write logic.
- Files explicitly out of bounds:
  - `modules/`, WIT/schema files, renderer/G-code surfaces, guest artifacts, and unrelated tests.
- Expected sub-agent dispatches:
  - Question: What smallest existing model-backed fixture and test harness in `crates/pnp-cli/tests/visual_debug_request_bundle_tdd.rs` can exercise packet-157's visual-debug command? Scope: `crates/pnp-cli/tests/**`; return: `LOCATIONS` at most 20 entries.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 99-110, 180-213.
  - `docs/01_system_architecture.md` - lines 65-109, 246-500, 567-665.
  - `docs/09_progress_events.md` - lines 74-109 and 139-143.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_typed_tap_capture_tdd` - FACT pass/fail; expected red before implementation.
- Exit condition: Tests compile and fail only on the unimplemented typed capture behavior, with no renderer/G-code assertions.

### Step 3: Implement Request-Gated Typed Closure Capture

- Task IDs: `TASK-268`
- Objective: Wire the minimal `crates/pnp-cli` command-to-runtime dispatch seam and the typed adapter registry/executor boundary capture using packet 157's exported model, fixed stage closure, selected-layer filtering, bounded renderer-owned copies, and manifest expansion/error reporting.
- Precondition: Step 2 has precise failing tests and Step 1's symbol inventory.
- Postcondition: Selected taps capture exact documented typed source fields after commit; closure stops at the furthest tap; extra execution is explained; ordinary slices remain capture-free.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/**` - exact executor/command files returned by bounded dispatch only.
  - `crates/pnp-cli/src/visual_debug.rs` - complete, 381 lines.
  - Step 1's identified `slicer-runtime` capture entry point - exact ranges returned by Step 1.
  - `crates/slicer-ir/src/**` - exact source structs/fields returned by bounded dispatch only.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/` - selected runtime executor/capture files only (new capture-execution module and adapter registry).
  - `crates/pnp-cli/src/visual_debug.rs` - minimal command-to-runtime dispatch seam only: replace the `Model`-source placeholder loop with the new `slicer-runtime` capture call, tap validation, all-selected-layers iteration, and `ImageEntry`/`Manifest` assembly. `validate_request`, `VisualDebugRequest`, and bundle lifecycle/overwrite/atomic-write logic remain unchanged.
  - `crates/pnp-cli/tests/visual_debug_typed_tap_capture_tdd.rs` - test fixtures/assertions only.
- Files explicitly out of bounds:
  - WIT, manifests, IR schema definitions, modules, WASM artifacts, renderers, G-code parser, skills, and coordinates.
- Expected sub-agent dispatches:
  - Question: Which exact executor boundary functions expose each documented typed source after host-hook commit, and what borrow/lifetime rules apply? Scope: `crates/slicer-runtime/src/**` and exact IR definitions; return: `SNIPPETS` at most 3 snippets, 30 lines each.
- Context cost: `M`
- Authoritative docs:
  - `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - complete 44-line decision record.
  - `docs/specs/visual-pipeline-debug.md` - lines 99-110 and 143-163, plus the stage inventory at lines 195-213.
  - `docs/01_system_architecture.md` - lines 246-500 and 633-665.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_typed_tap_capture_tdd` - FACT pass/fail; bounded failure SNIPPETS <=20 lines.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
- Exit condition: All packet-local tests pass and no selected capture retains arena-backed data or executes outside the requested closure.

### Step 4: Run Focused Quality Gates

- Task IDs: `TASK-268`
- Objective: Verify the complete typed capture slice against compilation, lint, focused tests, and the exact negative cases.
- Precondition: Step 3's focused tests pass.
- Postcondition: Workspace all-target compilation and clippy pass, and every AC command is green.
- Files allowed to read, with ranges when over 300 lines:
  - None beyond test output summaries and bounded failure snippets.
- Files allowed to edit (at most 3):
  - None unless a gate identifies a packet-local defect; then only Step 3 files.
- Files explicitly out of bounds:
  - All unrelated packet and implementation files, generated artifacts, and broad test output.
- Expected sub-agent dispatches:
  - Question: Do the packet-local test, workspace all-target check, and workspace all-target clippy commands pass? Scope: packet-158 files and command execution only; return: `FACT` in 5 lines or fewer.
- Context cost: `S`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - complete 179-line acceptance gate.
  - `docs/09_progress_events.md` - lines 84-109.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_typed_tap_capture_tdd` - FACT pass/fail.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
- Exit condition: All targeted positive and negative tests plus both workspace quality gates pass with no known packet-local regressions.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Packet-157 export inventory only. |
| Step 2 | M | Focused fixture and contract tests. |
| Step 3 | M | Runtime boundary and typed IR integration. |
| Step 4 | S | Bounded gate results only. |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
