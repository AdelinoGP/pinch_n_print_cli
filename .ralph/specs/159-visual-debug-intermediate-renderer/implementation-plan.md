# Implementation Plan: 159-visual-debug-intermediate-renderer

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-269`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Confirm Packet 158 Renderer Handoff

- Task IDs: `TASK-269`
- Objective: Identify the exact packet-158 renderer-owned capture, typed field, ordering, and manifest image-entry symbols needed by the renderer.
- Precondition: Packet 158 exists as a generated/draft prerequisite and its implementation/export may not be present.
- Postcondition: A bounded inventory confirms `[FWD-158-1]` through `[FWD-158-3]`, or the packet remains draft with a concrete blocker and no implementation edits.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/158-visual-debug-typed-tap-capture/**` - packet contract only; bounded export lookup.
- Files allowed to edit (at most 3):
  - None; read-only discovery.
- Files explicitly out of bounds:
  - Runtime source, generated code, `target/`, lockfiles, and all other packet directories.
- Expected sub-agent dispatches:
  - Question: What exact renderer-owned capture and manifest handoff does packet 158 publish? Scope: `.ralph/specs/158-visual-debug-typed-tap-capture/**`; return: `LOCATIONS` at most 20 entries.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 112-163 and 180-213.
  - `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - complete direct read.
- Verification:
  - Bounded packet-158 export lookup - `LOCATIONS` or an explicit `[BLOCK]` result.
- Exit condition: Exact handoff symbols and all required typed fields are recorded, or activation is explicitly blocked without edits.

### Step 2: Add Failing Intermediate Renderer Contract Tests

- Task IDs: `TASK-269`
- Objective: Encode typed polygon rendering, width sweeps, overlays, shared viewport/palette/scale, byte determinism, and all three negative output cases.
- Precondition: Step 1 confirms a usable packet-158 capture fixture/export or records the concrete missing seam.
- Postcondition: `visual_debug_intermediate_renderer_tdd` compiles and fails only on missing renderer behavior, with exact manifest fields, dimensions, PNG existence/bytes, and no partial-success assertions.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/**` - targeted visual-debug fixture/helper files only.
  - Packet-158-owned export source - exact ranges returned by Step 1.
  - `crates/slicer-ir/src/**` - exact typed field definitions returned by bounded dispatch.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/tests/visual_debug_intermediate_renderer_tdd.rs`
  - Existing packet-158 fixture helper only if the confirmed export requires a minimal additive test constructor.
- Files explicitly out of bounds:
  - CLI parsing/lifecycle, scheduler capture, final G-code renderer, WIT/schema, modules, WASM, skills, Orca references, and ordinary slice tests.
- Expected sub-agent dispatches:
  - Question: What smallest real typed-capture fixture can express polygons, `Point3WithWidth.width`, and documented overlays? Scope: targeted packet-158 exports and `crates/slicer-runtime/tests/**`; return: `SNIPPETS` at most 3 snippets, 30 lines each.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 119-141 and 195-213.
  - `docs/19_visual_debug.md` - lines 30-50.
  - `docs/11_operational_governance_and_acceptance_gate.md` - lines 102-117 and 167-179.
- Verification:
  - `cargo test -p slicer-runtime --all-targets --test visual_debug_intermediate_renderer_tdd 2>&1 | tee target/test-output.log` - FACT pass/fail; expected red before implementation.
- Exit condition: Tests fail on absent renderer behavior, not on invented symbols, missing fixture types, or unrelated compilation failures.

### Step 3: Implement Typed Geometry, Viewport, Overlay, and PNG Rendering

- Task IDs: `TASK-269`
- Objective: Implement the pure-Rust intermediate renderer and narrow packet-158 image-entry handoff using typed polygons, width sweeps, deterministic overlays, shared viewport/palette, scale validation, and PNG output.
- Precondition: Step 2 has precise failing tests and Step 1's handoff inventory is complete.
- Postcondition: All positive and negative renderer tests pass; every image has the shared viewport, fixed legend/palette metadata, exact scale dimensions, deterministic bytes, and no arena-backed borrow.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/**` - exact visual-debug renderer and bundle integration files returned by bounded symbol lookup.
  - Packet-158-owned handoff source - exact ranges returned by Step 1.
  - `crates/slicer-ir/src/**` - exact source fields returned by bounded dispatch.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/` - renderer, palette/viewport, overlay, and PNG integration files only.
  - `crates/slicer-runtime/tests/visual_debug_intermediate_renderer_tdd.rs` - fixture/assertion updates only.
  - Packet-158-owned handoff source - additive image-entry attachment only if confirmed necessary by Step 1.
- Files explicitly out of bounds:
  - `crates/pnp-cli/` parsing/validation/lifecycle, scheduler/executor capture, final G-code renderer, WIT/schema, module manifests, modules, WASM, skills, Orca references, and ordinary slice paths.
- Expected sub-agent dispatches:
  - Question: What exact runtime invocation and image-entry append functions can be used without taking ownership from packets 157 or 158? Scope: targeted `crates/slicer-runtime/src/**` and packet-158 handoff; return: `LOCATIONS` at most 20 entries.
  - Question: What dependency feature and license record are required for the pure-Rust PNG encoder? Scope: manifest/dependency policy only; return: `FACT` in 5 lines or fewer.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/visual-pipeline-debug.md` - lines 119-163 and 195-213.
  - `docs/01_system_architecture.md` - lines 246-387 and 621-665.
  - `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - complete direct read.
- Verification:
  - `cargo test -p slicer-runtime --all-targets --test visual_debug_intermediate_renderer_tdd 2>&1 | tee target/test-output.log` - FACT pass/fail; bounded failure snippets only.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
- Exit condition: Focused tests pass and the renderer has no inferred width, per-image viewport drift, nondeterministic output, partial-success path, or ownership change.

### Step 4: Run Focused Quality and Freshness Gates

- Task IDs: `TASK-269`
- Objective: Verify the renderer slice through focused tests, all-target compilation, clippy, and guest freshness check because shared runtime/schema dependencies may feed guest artifacts.
- Precondition: Step 3 focused renderer tests pass.
- Postcondition: Focused renderer tests, workspace all-target check, and clippy pass; `cargo xtask build-guests --check` is clean or any stale artifacts are rebuilt and the focused test is rerun.
- Files allowed to read, with ranges when over 300 lines:
  - None beyond bounded command summaries and failure snippets.
- Files allowed to edit (at most 3):
  - None unless a gate identifies a packet-local defect; then only Step 3 files.
- Files explicitly out of bounds:
  - All unrelated packet and implementation files, generated artifacts, guest artifacts, and broad test output.
- Expected sub-agent dispatches:
  - Question: Do the focused renderer test, workspace all-target check, clippy, and guest freshness check pass? Scope: packet-159 implementation and commands only; return: `FACT` in 5 lines or fewer.
- Context cost: `S`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - complete direct read.
  - `docs/07_implementation_status.md` - delegated TASK-269 location only.
- Verification:
  - `cargo test -p slicer-runtime --all-targets --test visual_debug_intermediate_renderer_tdd 2>&1 | tee target/test-output.log` - FACT pass/fail.
  - `cargo check --workspace --all-targets` - FACT pass/fail.
  - `cargo clippy --workspace --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT pass/fail; rebuild and rerun the focused test if stale.
- Exit condition: Every packet-local positive and negative test and all required quality/freshness gates pass with no known unintended side effects.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Packet-158 forward-contract inventory only. |
| Step 2 | M | Focused typed renderer contract tests and real fixture lookup. |
| Step 3 | M | Renderer, viewport, overlay, palette, PNG, and narrow handoff integration. |
| Step 4 | S | Bounded quality and freshness results only. |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile packet 158's generated/draft status and record all resolved `[FWD]` contracts.
- `packet.spec.md` is ready for `status: implemented` only after the independent reviewer clears the draft.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk, PNG dependency feature/license evidence, and any unresolved forward contract.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
