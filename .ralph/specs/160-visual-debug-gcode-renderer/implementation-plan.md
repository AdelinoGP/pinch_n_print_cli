# Implementation Plan: 160-visual-debug-gcode-renderer

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Establish the packet 157 integration and parser test seam

- Task IDs: `TASK-270`
- Objective: identify the packet 157 request/manifest lifecycle exports after their [FWD] gate and define focused inline fixtures and assertions for the standalone final-G-code path.
- Precondition: the packet 157 request, bundle, and manifest [FWD] contracts in `packet.spec.md` are verified by their named acceptance tests; no typed tap or intermediate-renderer surface is required.
- Postcondition: the test target names the final-G-code entry point, manifest fields, PNG paths, warnings, and deterministic comparison required by AC-1 through AC-N2.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/main.rs` - targeted visual-debug command symbols only.
  - `crates/pnp-cli/src/lib.rs` - targeted module/export symbols only.
  - `.ralph/specs/157-visual-debug-request-bundle-contract/packet.spec.md` - lines 27-65.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - `modules/**`, WIT, IR, scheduler, manifests, typed taps, intermediate renderer, agent skill, ordinary slice paths.
- Expected sub-agent dispatches:
  - Question: identify exact packet 157 command and manifest symbols; scope: `crates/pnp-cli/src/**`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 61-178.
  - `docs/19_visual_debug.md` - lines 16-50.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCodeReader.hpp` - delegate locations only; never load.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- --list` - FACT with the named test inventory.
- Exit condition: all eight AC test names exist and the test fixture exercises both supported and negative inputs.

### Step 2: Implement deterministic PnP G-code parsing and final PNG rendering core

- Task IDs: `TASK-270`
- Objective: parse supported final text in source order, retain unclassified extrusion, collect unsupported line warnings, compute the shared viewport, and render the requested PNG views deterministically.
- Precondition: Step 1's test seam identifies the actual packet 157 integration symbols and fixtures, and the packet 157 [FWD] request, bundle, and manifest acceptance conditions are passing.
- Postcondition: the parser and renderer produce deterministic view bytes for valid supported and negative inputs, including source-line warnings and no-renderable failures.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/visual-pipeline-debug.md` - lines 112-178.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/main.rs`, `crates/pnp-cli/src/lib.rs`, `OrcaSlicerDocumented/**` direct reads, typed taps, intermediate renderer, scheduler closure, WIT/IR/manifests, agent skill, ordinary slice path.
- Expected sub-agent dispatches:
  - Question: return bounded Orca locations for GCodeReader/GCodeProcessor/libvgcode motion and preview behavior; scope: the packet's listed `OrcaSlicerDocumented` paths; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 112-178 and 195-213.
  - `docs/01_system_architecture.md` - lines 477-497.
  - `docs/11_operational_governance_and_acceptance_gate.md` - lines 86-117.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` - delegate only; no direct source load.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd` - FACT pass/fail with bounded failure snippets.
- Exit condition: parser and renderer behavior passes the focused AC-1 through AC-N2 assertions for supported motion, retained unclassified extrusion, warnings, shared viewport, deterministic bytes, invalid width, and no-renderable input.

### Step 3: Integrate the parser and renderer with the packet 157 bundle lifecycle

- Task IDs: `TASK-270`
- Objective: wire the deterministic parser and renderer into the packet 157 request, bundle, and manifest lifecycle without changing typed taps, intermediate rendering, scheduler closure, or ordinary slice paths.
- Precondition: Step 2's parser and renderer pass their focused assertions, and the packet 157 [FWD] request, bundle, and manifest acceptance conditions are passing.
- Postcondition: valid standalone requests produce complete deterministic manifest/PNG bundles; invalid width and no-renderable inputs fail without successful partial bundles.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/main.rs` - targeted visual-debug command and lifecycle symbols.
  - `crates/pnp-cli/src/lib.rs` - targeted module wiring symbols.
  - `docs/specs/visual-pipeline-debug.md` - lines 112-178 and 195-213.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/main.rs`
  - `crates/pnp-cli/src/lib.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug_gcode.rs`, typed taps, intermediate renderer, scheduler closure, WIT/IR/manifests, agent skill, ordinary slice path.
- Expected sub-agent dispatches:
  - Question: identify exact packet 157 command and manifest symbols; scope: `crates/pnp-cli/src/**`; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 112-178 and 195-213.
  - `docs/01_system_architecture.md` - lines 477-497.
  - `docs/11_operational_governance_and_acceptance_gate.md` - lines 86-117.
- OrcaSlicer refs:
  - None beyond Step 2's delegated locations; do not load Orca sources.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd` - FACT pass/fail with bounded failure snippets.
- Exit condition: AC-1 through AC-N2 pass through the packet 157 lifecycle, including complete manifests, requested PNG paths, byte-identical repeated output, source-line warnings, and no successful partial bundles.

### Step 4: Run packet closure gates and inspect only bounded results

- Task IDs: `TASK-270`
- Objective: falsify the renderer against the targeted suite and workspace compile/lint gates without broad implementation reads.
- Precondition: Step 3's targeted renderer tests pass.
- Postcondition: targeted tests, all-target compile, and clippy pass with no known packet-local regression.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - only summary or bounded failure ranges after the test command.
  - changed packet-local files - only diagnostics needed to fix a reported failure.
- Files allowed to edit (at most 4):
  - `crates/pnp-cli/src/main.rs`
  - `crates/pnp-cli/src/lib.rs`
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - all files outside the three allowed implementation/test files and the five packet artifacts; no generated output, lockfile, or unrelated docs.
- Expected sub-agent dispatches:
  - Question: run targeted tests, `cargo check --workspace --all-targets`, and `cargo clippy --workspace --all-targets -- -D warnings`; scope: repository commands; return: `FACT`.
- Context cost: `S`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - lines 86-117.
  - `docs/07_implementation_status.md` - delegated TASK-270 lookup.
- OrcaSlicer refs:
  - None beyond Step 2's delegated locations; do not load Orca sources.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd 2>&1 | tee target/test-output.log` - FACT from the log.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
- Exit condition: every pipe-suffixed AC command and all three packet verification gates pass, with no unclassified-drop, unsupported-approximation, partial-bundle, or ordinary-slice-overhead regression known.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Test seam and bounded integration locations. |
| Step 2 | M | Parser, state, viewport, and raster implementation. |
| Step 3 | S | Packet 157 lifecycle and manifest integration. |
| Step 4 | S | Delegated gates and bounded diagnostics. |

Split before activation if aggregate cost exceeds M or any step is L.

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
