# Implementation Plan: macro-prepass-segmentation-output-drain

## Execution Rules

- One atomic step at a time.
- Each step maps back to `TASK-130`, `TASK-130a`, or `TASK-130b`.
- TDD first, then implementation, then narrowest falsifying validation.
- Each step honors the context-discipline preamble. The fields below are the budget contract.
- Production code outside `crates/slicer-macros/src/lib.rs::build_prepass_world_glue::"PrePass::PaintSegmentation"` arm is **out of bounds**. Test code is in scope.

## Steps

### Step 0: Activation gate + open-question lock (no edits)

- Task IDs:
  - `TASK-130`, `TASK-130a`, `TASK-130b`
- Objective: confirm Packet 42 is `implemented` and resolve the three open Step-0 questions before any non-trivial edit.
- Precondition: this packet is `draft`; Packet 42 is referenced and reviewed.
- Postcondition: a Step-0 Notes addendum is appended to this file recording binary answers for:
  1. Is `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` `status: implemented`? If no, this packet stays `draft` and the rest of the plan does not execute.
  2. Which `test-guests/` subdirectory holds the macro-authored prepass guest source?
  3. Does that guest's `run_paint_segmentation` currently call `push_paint_region`? Does its `run_mesh_segmentation` currently call `mark_triangle_paint`?
  4. Did Packet 42's edits shift the line numbers of the PaintSegmentation arm body (originally 1760-1788)?
  5. Is `test-guests/build-test-guests.sh` runnable on the local toolchain?
  6. Which existing test files load `sdk-prepass-guest.component.wasm` (regression sweep target list)?
- Files allowed to read: none directly; this step is **pure dispatch**.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds for this step: every source file. FACT-only.
- Expected sub-agent dispatches:
  - "Read `.ralph/specs/42_paint-region-transport-widening/packet.spec.md` frontmatter; return FACT (`status` value)." — return format: FACT.
  - "List `test-guests/` directory contents; for any directory matching `*prepass*` or `*sdk*`, return its `src/lib.rs` first 100 lines as SNIPPETS." — return format: SNIPPETS.
  - "Show `crates/slicer-macros/src/lib.rs` lines 1700-1800; return SNIPPETS." — return format: SNIPPETS.
  - "Run `which wasm32-wasi cargo-component`; return FACT." — return format: FACT.
  - "Grep workspace for the literal `sdk-prepass-guest.component.wasm`; return LOCATIONS." — return format: LOCATIONS.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: Step-0 Notes addendum has six binary answers.
- Exit condition: every Step-0 question has a recorded answer; if (1) is `no`, halt the packet and notify the user.

### Step 1: Author RED tests for the two new round-trip TDDs

- Task IDs:
  - `TASK-130b`
- Objective: stand up the two new test files with all named ACs in RED state. TDD anchor for the rest of the packet.
- Precondition: Step 0 GREEN, Packet 42 `implemented`.
- Postcondition: `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` and `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` exist with all named tests from `packet.spec.md` AC + Negative sections; tests currently FAIL (RED) because the guest does not yet emit the fixtures (Step 3) and/or the macro arm does not yet drain (Step 2).
- Files allowed to read:
  - `crates/slicer-host/tests/macro_mesh_segmentation_geometry_tdd.rs` — full file (harness patterns).
  - `crates/slicer-host/tests/macro_paint_region_roundtrip_tdd.rs` — full file (assertion patterns).
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (NEW)
  - `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (NEW)
- Files explicitly out-of-bounds for this step: every production source file.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd -- --nocapture`; return FACT (count of failures + first failure assertion or compile error)." — return format: FACT or SNIPPETS.
  - "Run `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd -- --nocapture`; return FACT or SNIPPETS." — return format: FACT or SNIPPETS.
- Context cost: `M`.
- Authoritative docs: `docs/02_ir_schemas.md` (PaintRegionIR + MeshSegmentationIR sections — direct read; narrow).
- OrcaSlicer refs: none.
- Verification: both test files compile; predictable failures on guest-fixture-absent or macro-arm-no-drain.
- Exit condition: every named AC test from `packet.spec.md` is present; failure pattern recorded in commit/step log.

### Step 2: Drain the PaintSegmentation arm (mirror MeshSegmentation pattern)

- Task IDs:
  - `TASK-130a`
- Objective: insert the drain loop in `crates/slicer-macros/src/lib.rs::build_prepass_world_glue` PaintSegmentation arm, mirroring the MeshSegmentation drain at lib.rs:1733-1746. Remove the legacy rationalization comment block at lines 1760-1769.
- Precondition: Step 1 RED tests exist; Step 0 confirmed line numbers.
- Postcondition: PaintSegmentation arm contains a `for entry in sdk_output.regions() { ... _output.push_paint_region(&wit_entry) ... }` loop after the trait call's `Ok(())` and before the arm's return; push failure surfaces as `ModuleError { code: 10, fatal: true }`; legacy comment block is gone; `cargo build --workspace` succeeds.
- Files allowed to read:
  - `crates/slicer-macros/src/lib.rs:1700-1800` — the two prepass arms; mirror the MeshSegmentation arm exactly.
  - `crates/slicer-macros/src/lib.rs:1283-1314` — inline-WIT block (post Packet 42 it carries `paint-value-input`); confirm the WIT type names referenced in the drain.
  - `crates/slicer-sdk/src/prepass_builders.rs::PaintSegmentationOutput::regions()` — accessor signature only.
- Files allowed to edit (≤ 3):
  - `crates/slicer-macros/src/lib.rs`
- Files explicitly out-of-bounds for this step: tests (Step 1 + later steps), guest source, host source, SDK source, WIT, docs.
- Expected sub-agent dispatches:
  - "Run `cargo build --workspace`; return FACT pass/fail." — return format: FACT.
  - "Run `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd macro_arm_drains_regions_to_wit legacy_comment_block_removed -- --exact --nocapture`; return FACT pass/fail." — return format: FACT.
  - "Grep `crates/slicer-macros/src/lib.rs` for the literal `Same disconnect as MeshSegmentation`; return FACT (zero matches expected)." — return format: FACT.
- Context cost: `S` to `M`.
- Authoritative docs: `docs/05_module_sdk.md` — delegate SUMMARY for the prepass arm's expected lifecycle.
- OrcaSlicer refs: none.
- Verification: workspace builds; the two named tests GREEN; legacy comment grep zero.
- Exit condition: macro arm drain in place; build green; legacy comment removed.

### Step 3: Extend the macro test guest to emit the round-trip fixtures

- Task IDs:
  - `TASK-130b`
- Objective: extend `test-guests/sdk-prepass-guest/src/lib.rs` (or whatever path Step 0 confirmed) so its `run_paint_segmentation` and `run_mesh_segmentation` push the fixtures the new TDDs assert (hole-bearing polygon + Custom semantic + Custom value + typed ToolIndex value + symmetric MeshSegmentation marks).
- Precondition: Step 2 GREEN.
- Postcondition: the guest source compiles to a component .wasm; the new fixtures are emitted under recognizable config-key triggers (so the same guest can serve other tests without interference). The `run_mesh_segmentation` exit emits at least one mark with known `(object_id, facet_index, semantic, value)`.
- Files allowed to read:
  - `test-guests/sdk-prepass-guest/src/lib.rs` (or equivalent; Step 0 path) — full file.
  - The bindgen output names confirmed in Step 0 (`PaintRegionEntry`, `PaintValueInput`, `ExPolygon`).
  - `crates/slicer-sdk/src/prepass_builders.rs::PaintSegmentationOutput::push_paint_region` and `MeshSegmentationOutput::mark_triangle_paint` — signatures only.
- Files allowed to edit (≤ 3):
  - `test-guests/sdk-prepass-guest/src/lib.rs` (or equivalent)
  - (optionally) `test-guests/sdk-prepass-guest/Cargo.toml` if a new dependency is needed (unlikely)
- Files explicitly out-of-bounds for this step: macro, host, SDK, WIT, other test guests.
- Expected sub-agent dispatches:
  - "Run `./test-guests/build-test-guests.sh`; return FACT (success line + new size of `sdk-prepass-guest.component.wasm`). If toolchain missing, return FACT including the missing tool + recommendation." — return format: FACT.
  - "Run `cargo build -p sdk-prepass-guest`; return FACT pass/fail." — return format: FACT.
- Context cost: `M`.
- Authoritative docs: `docs/05_module_sdk.md` — delegate SUMMARY ≤ 200 words for the `#[slicer_module]` macro emit pattern (helps if the guest needs adjustment beyond fixture insertion).
- OrcaSlicer refs: none.
- Verification: guest builds; .wasm rebuilt with new size.
- Exit condition: pre-built .wasm regenerated; guest source carries the fixture-emit logic.

### Step 4: PaintSegmentation round-trip TDD GREEN sweep

- Task IDs:
  - `TASK-130b`
- Objective: drive every test in `macro_paint_segmentation_output_roundtrip_tdd.rs` to GREEN.
- Precondition: Steps 2 + 3 GREEN.
- Postcondition: every named test in `macro_paint_segmentation_output_roundtrip_tdd.rs` passes (AC-1, AC-2, AC-3, AC-5, AC-6, plus negatives).
- Files allowed to read:
  - The same harness files from Step 1.
  - `crates/slicer-host/src/dispatch.rs:1954-2045` (`harvest_paint_segmentation_ir`, post Packet 42) — confirm assertion targets.
  - `crates/slicer-ir/src/slice_ir.rs` — only the `PaintRegionIR` / `LayerPaintMap` / `SemanticRegion` / `PaintValue` definitions.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/macro_paint_segmentation_output_roundtrip_tdd.rs` (refinements only — the structure was authored in Step 1)
- Files explicitly out-of-bounds for this step: production source files (no further changes; Steps 2 + 3 already locked them), other test files.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd -- --nocapture`; return FACT pass/fail or SNIPPETS for any failing test." — return format: FACT or SNIPPETS.
- Context cost: `S` to `M`.
- Authoritative docs: `docs/02_ir_schemas.md` PaintRegionIR section — direct read; narrow.
- OrcaSlicer refs: none.
- Verification: every named PaintSegmentation round-trip test GREEN.
- Exit condition: AC-1/2/3/5/6 + negatives all GREEN.

### Step 5: MeshSegmentation round-trip TDD GREEN sweep

- Task IDs:
  - `TASK-130b`
- Objective: drive every test in `macro_mesh_segmentation_output_roundtrip_tdd.rs` to GREEN.
- Precondition: Steps 2 + 3 GREEN (Step 2 modifies the macro arm but not the MeshSegmentation drain; that's already in place. Step 3 extended the guest's `run_mesh_segmentation`).
- Postcondition: every named test in `macro_mesh_segmentation_output_roundtrip_tdd.rs` passes (AC-4).
- Files allowed to read: same as Step 4.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/macro_mesh_segmentation_output_roundtrip_tdd.rs` (refinements only)
- Files explicitly out-of-bounds for this step: production source files, other test files.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test macro_mesh_segmentation_output_roundtrip_tdd -- --nocapture`; return FACT pass/fail or SNIPPETS." — return format: FACT or SNIPPETS.
- Context cost: `S`.
- Authoritative docs: `docs/02_ir_schemas.md` MeshSegmentationIR section — direct; narrow.
- OrcaSlicer refs: none.
- Verification: AC-4 GREEN.
- Exit condition: MeshSegmentation round-trip GREEN.

### Step 6: Regression sweep — existing macro_*_tdd tests + paint pipeline

- Task IDs:
  - `TASK-130`, `TASK-130a`, `TASK-130b`
- Objective: ensure no existing test that exercises the macro path or the post-Packet-42 transport is broken.
- Precondition: Steps 4 + 5 GREEN.
- Postcondition: every test in the regression list passes (named in `packet.spec.md` Verification section).
- Files allowed to read: none directly.
- Files allowed to edit (≤ 3): none — this step is dispatch-only.
- Files explicitly out-of-bounds for this step: every source file.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-host --test macro_paint_segmentation_input_tdd --test macro_mesh_segmentation_geometry_tdd --test macro_paint_region_roundtrip_tdd --test macro_mesh_raycast_z_down_tdd -- --nocapture`; return FACT pass/fail or SNIPPETS for each." — return format: FACT list.
  - For each test file Step 0 enumerated as loading `sdk-prepass-guest.component.wasm`: dispatch a separate FACT-pass/fail run.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: every regression target GREEN.
- Exit condition: no regression in the macro path or paint pipeline.

### Step 7: Backlog + DEV-025 closure docs

- Task IDs:
  - `TASK-130`, `TASK-130a`, `TASK-130b`
- Objective: flip docs/07 checkboxes; remove TASK-130a, TASK-130b from blocker list; close DEV-025 mismatch 3; set DEV-025 overall status to `closed`; update DEV-025 audit row.
- Precondition: Steps 4 + 5 + 6 GREEN.
- Postcondition: all three docs reflect the closure as described in `packet.spec.md` AC-7, AC-8, AC-9.
- Files allowed to read:
  - `docs/07_implementation_status.md` lines 65-72 and 175-185.
  - `docs/DEVIATION_LOG.md` — DEV-025 entry only.
  - `docs/14_deviation_audit_history.md` — DEV-025 row only.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md`
  - `docs/DEVIATION_LOG.md`
  - `docs/14_deviation_audit_history.md`
- Files explicitly out-of-bounds for this step: every source file.
- Expected sub-agent dispatches:
  - "Update `docs/07_implementation_status.md` to flip TASK-130/130a/130b checkboxes and remove TASK-130a/130b from the blocker list; return the diff." — return format: SNIPPETS (≤ 20 lines).
  - "Update `docs/DEVIATION_LOG.md` DEV-025 entry to close mismatch 3 and set overall status to `closed`; return the diff." — return format: SNIPPETS (≤ 20 lines).
  - "Update `docs/14_deviation_audit_history.md` DEV-025 row to reference TASK-130/130a/130b; return the diff." — return format: SNIPPETS (≤ 20 lines).
  - "Run `cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd docs_07_marks_130_cluster_done dev_025_fully_closed dev_025_audit_history_complete -- --exact --nocapture`; return FACT pass/fail." — return format: FACT.
- Context cost: `S`.
- Authoritative docs: `docs/DEVIATION_LOG.md`, `docs/14_deviation_audit_history.md`.
- OrcaSlicer refs: none.
- Verification: AC-7, AC-8, AC-9 GREEN.
- Exit condition: DEV-025 closed; backlog updated.

### Step 8: Acceptance ceremony — full AC sweep

- Task IDs:
  - `TASK-130`, `TASK-130a`, `TASK-130b`
- Objective: run every AC + Negative command from `packet.spec.md`; confirm clippy clean; run `cargo test --workspace` once as the closure gate; transition `packet.spec.md` to `status: implemented`.
- Precondition: Steps 0-7 GREEN.
- Postcondition: every AC test passes; `cargo clippy --workspace -- -D warnings` passes; `cargo test --workspace` passes.
- Files allowed to read:
  - `packet.spec.md` — for the full AC list.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/43_macro-prepass-segmentation-output-drain/packet.spec.md` (status flip)
- Files explicitly out-of-bounds for this step: every source file.
- Expected sub-agent dispatches:
  - "Run each pipe-suffixed verification command from `packet.spec.md`; return one FACT per command." — return format: FACT list.
  - "Run `cargo clippy --workspace -- -D warnings`; return FACT pass/fail." — return format: FACT.
  - "Run `cargo test --workspace`; return FACT (pass/fail + failing test count). Closure gate; not for use during iterations." — return format: FACT.
- Context cost: `S` (pure dispatch + one frontmatter flip).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: AC sweep GREEN; clippy clean; workspace tests clean.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Pure FACT dispatch; activation gate. |
| Step 1 | M | Two new test files; many test stubs. |
| Step 2 | S/M | One macro file edit (~20 lines added, ~10 removed). |
| Step 3 | M | One guest source extension; new fixtures. |
| Step 4 | S/M | Test refinements only — production code already locked. |
| Step 5 | S | Test refinements only. |
| Step 6 | S | Pure dispatch — regression sweep. |
| Step 7 | S | Three doc edits via worker dispatches. |
| Step 8 | S | Acceptance ceremony. |

Aggregate: **M**. No single step is L. If any step trends toward L, split it before continuing.

## Packet Completion Gate

- All steps complete.
- Every step exit condition met.
- Every Acceptance Criterion command from `packet.spec.md` returned PASS via dispatch.
- `docs/07_implementation_status.md` TASK-130 cluster updated; blocker list shrunk by two entries (130a, 130b).
- `docs/DEVIATION_LOG.md` DEV-025 status `closed`; mismatch 3 closed-by-Packet-43.
- `docs/14_deviation_audit_history.md` DEV-025 row references TASK-128a/128b/130/130a/130b/130c.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md` (Acceptance Criteria + Negative Test Cases).
- Confirm packet-level Verification commands GREEN (one workspace `cargo build`, one `cargo clippy --workspace`, one `cargo test --workspace` — the latter only at this ceremony).
- Confirm peak context usage stayed under 70%; if not, log a packet-authoring lesson.
- Record any remaining packet-local risk (most likely: a non-macro-path test that loaded `sdk-prepass-guest.component.wasm` and is sensitive to fixture changes).
