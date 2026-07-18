# Implementation Plan: 160-visual-debug-gcode-renderer

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Establish the parser test seam against the verified integration surface

- Task IDs: `TASK-270`
- Objective: write the new test file's fixtures and named test stubs against the already-verified `visual_debug.rs` integration seam (`VisualDebugRequest`/`VisualDebugSource::Gcode` at lines 16-48, `Manifest`/`ImageEntry` at lines 220-288, the placeholder `Gcode` arm at lines 519-561) — no further symbol discovery dispatch is needed, since this packet's `packet.spec.md` "Grounded Packet 157/158 Integration Facts" already names every symbol.
- Precondition: `packet.spec.md`'s grounded facts are treated as ground truth; packets 157/158 are `status: implemented`.
- Postcondition: the test target names the final-G-code entry point, manifest fields, PNG paths, warnings, and deterministic comparison required by AC-1 through AC-N2, including one fixture whose request selects two or more layers (to falsify the placeholder's `req.layers.first()`-only behavior once Step 2/3 land).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` - lines 16-48, 220-288, 519-561 only.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/main.rs`, `crates/pnp-cli/src/lib.rs` (verified: no changes needed), `modules/**`, WIT, IR, scheduler, manifests, typed taps, intermediate renderer, agent skill, ordinary slice paths.
- Expected sub-agent dispatches: none — the integration seam is already grounded in `packet.spec.md`.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 61-98.
  - `docs/19_visual_debug.md` - lines 16-33 and 35-43.
- OrcaSlicer refs:
  - None for this step; deferred to Step 2.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- --list` - FACT with the named test inventory.
- Exit condition: all eight AC test names exist and the test fixtures exercise both supported and negative inputs, including a multi-layer request fixture.

### Step 2: Implement deterministic PnP G-code parsing and final PNG rendering core

- Task IDs: `TASK-270`
- Objective: in the new private submodule `crates/pnp-cli/src/visual_debug_gcode.rs`, parse supported final text in source order, retain unclassified extrusion, collect unsupported line warnings, compute one internal model-wide XY bounding box (mm-space, fixed margin) from parsed geometry, add and wire a pure-Rust PNG-encoding dependency, and render the requested PNG views deterministically. This step does not yet touch `visual_debug.rs`'s dispatch match — it only builds the callable parser/renderer core Step 3 wires in.
- Precondition: Step 1's fixtures exist; `packet.spec.md`'s grounded facts confirm `Viewport { width, height }` (`visual_debug.rs:264-267`) is pixel-only and unrelated to this internal bounding box, and confirm no PNG dependency exists in `crates/pnp-cli/Cargo.toml` today.
- Postcondition: the parser and renderer produce deterministic PNG bytes for valid supported and negative inputs, including source-line warnings and no-renderable failures, callable as a plain function/struct from `visual_debug.rs` in Step 3.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/visual-pipeline-debug.md` - lines 218-231 and 186-195.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
  - `crates/pnp-cli/Cargo.toml`
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/visual_debug.rs` (wiring is Step 3), `crates/pnp-cli/src/main.rs`, `crates/pnp-cli/src/lib.rs`, `OrcaSlicerDocumented/**` direct reads, typed taps, intermediate renderer, scheduler closure, WIT/IR/manifests, agent skill, ordinary slice path.
- Expected sub-agent dispatches:
  - Question: return bounded Orca locations for GCodeReader/GCodeProcessor/libvgcode motion and preview behavior; scope: the packet's listed `OrcaSlicerDocumented` paths; return: `LOCATIONS`.
  - Question: identify the smallest pure-Rust PNG-encoding crate to add (feature set, license); scope: `cargo metadata --format-version=1 --no-deps` summary; return: `FACT`.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 218-231 and 186-195.
  - `docs/01_system_architecture.md` - lines 477-497.
  - `docs/11_operational_governance_and_acceptance_gate.md` - lines 86-117.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` - delegate only; no direct source load.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd` - FACT pass/fail with bounded failure snippets.
- Exit condition: parser and renderer behavior passes the focused AC-1 through AC-N2 assertions for supported motion, retained unclassified extrusion, warnings, internal bounding-box consistency, deterministic bytes, invalid width, and no-renderable input.

### Step 3: Replace the placeholder `Gcode` arm and integrate with the bundle lifecycle

- Task IDs: `TASK-270`
- Objective: replace the placeholder `VisualDebugSource::Gcode { path, .. }` match-arm body (`visual_debug.rs:519-561`) so it opens `path`, calls Step 2's parser/renderer once per requested/resolved layer (not just `req.layers.first()`), writes real PNG files under `output_dir/images/...` before the existing atomic `manifest.json` commit (`visual_debug.rs:604-620`), and populates real `gcode_parser_version`/`warnings`/`png_path` values — without touching `main.rs`, `lib.rs`, typed taps, intermediate rendering, or scheduler closure.
- Precondition: Step 2's parser/renderer pass their focused assertions in isolation.
- Postcondition: valid standalone requests produce complete deterministic manifest/PNG bundles with one entry per requested layer; invalid width and no-renderable inputs fail without successful partial bundles or leftover PNGs.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/pnp-cli/src/visual_debug.rs` - lines 480-621 only (the `run_visual_debug` lifecycle and dispatch match).
  - `docs/specs/visual-pipeline-debug.md` - lines 218-231.
- Files allowed to edit (at most 3):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/main.rs`, `crates/pnp-cli/src/lib.rs` (verified: no changes needed), `crates/pnp-cli/src/visual_debug_gcode.rs` (Step 2 owns it; Step 3 only calls it), typed taps, intermediate renderer, scheduler closure, WIT/IR/manifests, agent skill, ordinary slice path.
- Expected sub-agent dispatches: none — the integration point (`visual_debug.rs:519-561`) is already grounded.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 218-231.
  - `docs/01_system_architecture.md` - lines 477-497.
  - `docs/11_operational_governance_and_acceptance_gate.md` - lines 86-117.
- OrcaSlicer refs:
  - None beyond Step 2's delegated locations; do not load Orca sources.
- Verification:
  - `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd` - FACT pass/fail with bounded failure snippets.
- Exit condition: AC-1 through AC-N2 pass through the real bundle lifecycle, including complete manifests, requested PNG paths for every selected layer, byte-identical repeated output, source-line warnings, and no successful partial bundles.

### Step 4: Run packet closure gates and inspect only bounded results

- Task IDs: `TASK-270`
- Objective: falsify the renderer against the targeted suite and workspace compile/lint gates without broad implementation reads.
- Precondition: Step 3's targeted renderer tests pass.
- Postcondition: targeted tests, all-target compile, and clippy pass with no known packet-local regression.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` - only summary or bounded failure ranges after the test command.
  - changed packet-local files - only diagnostics needed to fix a reported failure.
- Files allowed to edit (at most 4):
  - `crates/pnp-cli/src/visual_debug.rs`
  - `crates/pnp-cli/src/visual_debug_gcode.rs`
  - `crates/pnp-cli/Cargo.toml`
  - `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`
- Files explicitly out of bounds:
  - `crates/pnp-cli/src/main.rs`, `crates/pnp-cli/src/lib.rs` (verified: no changes needed), and all files outside the four allowed implementation/test files and the five packet artifacts; no generated output, lockfile, or unrelated docs.
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
| Step 1 | S | Test seam against the already-grounded integration surface; no discovery dispatch. |
| Step 2 | M | New submodule: parser, state, internal bounding box, PNG dependency, and raster implementation. |
| Step 3 | S | Replace the placeholder `Gcode` arm; wire into the existing bundle lifecycle. |
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
