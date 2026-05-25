# Implementation Plan: 67_3mf-fixture-e2e-hardening

## Execution Rules

- One atomic step at a time.
- Each step maps to TASK-208.
- TDD pattern: write tests RED first, then confirm they fail/pass as expected.
- No production code edits — test-only packet.
- Aggregate context cost is **M**. All steps are S or M.
- This packet depends on Packet 56c being `status: implemented`. Step 0 verifies the precondition.
- The RED test (AC-R1) intentionally fails — it documents expected behavior for Packet 68. AC-R2 was downgraded to a metadata check (GREEN) per the packet deviation (D3); only AC-R1 remains RED.

## Steps

### Step 0: Precondition gate

- Task IDs: TASK-208 (precursor)
- Objective: Verify Packet 56c is `status: implemented` and all three 3MF fixtures exist on disk.
- Precondition: Packet activated.
- Postcondition: All preconditions confirmed OR halt.
- Files allowed to read: none directly. Pure dispatch step.
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: everything.
- Expected sub-agent dispatches:
  - Question: "What is the `status:` value in the frontmatter of `.ralph/specs/56c_threemf-negative-and-support-subtype-routing/packet.spec.md`? Return FACT one-line value." → FACT. Expected: `implemented`.
  - Question: "Do the following files exist? `resources/cube_positive_n_negative.3mf`, `resources/bridge_support_enforcers.3mf`, `resources/benchy_4color.3mf`. Return FACT yes/no per file." → FACT. Expected: all yes.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: All FACTs return expected values.
- Exit condition: Step 1 may begin.

### Step 1: Author the fixture E2E test file (TDD-RED/GREEN)

- Task IDs: TASK-208
- Objective: Create `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` with all 12 test functions. Tests load real 3MF fixtures from `resources/` and exercise `load_model()`, `execute_paint_segmentation()`, `apply_negative_part_subtract()`. 11 GREEN tests assert existing behavior; 1 RED test asserts expected extruder behavior (for Packet 68).
- Precondition: Step 0 clean.
- Postcondition: Test file compiles. 11 GREEN tests pass. 1 RED test fails with a specific assertion message (not a panic).
- Files allowed to read:
  - `crates/slicer-host/src/model_loader.rs` — narrow read at `load_model` (line 145) for function signature.
  - `crates/slicer-host/src/paint_segmentation.rs` — narrow read at `execute_paint_segmentation` (line 253) for 4-param signature.
  - `crates/slicer-host/src/negative_part_subtract.rs` — full (63 lines) for signature and behavior reference.
  - `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs` — narrow read for `Path::new(env!("CARGO_MANIFEST_DIR"))` fixture path pattern.
  - `crates/slicer-host/tests/threemf_subtypes_synthetic_e2e_tdd.rs` — narrow read for area comparison tolerance pattern.
- Files allowed to edit (≤ 3):
  - `crates/slicer-host/tests/threemf_fixture_e2e_tdd.rs` — NEW.
- Files explicitly out-of-bounds: all `src/` files except those listed for signature reads; WIT, SDK, macros, OrcaSlicer source.
- Expected sub-agent dispatches:
  - Question: "Return the signature of `load_model` in `crates/slicer-host/src/model_loader.rs`. FACT with file:line." → FACT.
  - Question: "Return the signature of `execute_paint_segmentation` in `crates/slicer-host/src/paint_segmentation.rs`. FACT with file:line." → FACT.
  - Question: "Return the signature of `apply_negative_part_subtract` in `crates/slicer-host/src/negative_part_subtract.rs`. FACT with file:line." → FACT.
  - Question: "Return the fixture path resolution pattern from `crates/slicer-host/tests/threemf_sidecar_classification_tdd.rs`. SNIPPETS, ≤ 10 lines." → SNIPPETS.
  - Question: "Run `cargo test -p slicer-host --test threemf_fixture_e2e_tdd -- --nocapture`. Return FACT pass/fail per test function (list all 12, marking GREEN or RED). For RED tests, return the exact assertion message." → FACT.
- Context cost: M
- Authoritative docs:
  - `docs/02_ir_schemas.md` — narrow search for `ModifierVolume`, `PaintRegionIR`, `SemanticRegion` shapes.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-host --test threemf_fixture_e2e_tdd` — 11 GREEN, 1 RED with specific assertion message.
  - `cargo check --workspace --tests` — compiles clean.
- Exit condition: 11 GREEN tests pass, 1 RED test (AC-R1) fails with the documented assertion message. File compiles clean.

### Step 2: Regression sweep

- Task IDs: TASK-208
- Objective: Re-run Packet 56/56b/56c regression suites. Assert all GREEN.
- Precondition: Step 1 complete.
- Postcondition: All regression suites GREEN.
- Files allowed to read: none.
- Files allowed to edit (≤ 3): `threemf_fixture_e2e_tdd.rs` (only if a regression failure points to a test bug).
- Files explicitly out-of-bounds: all production source.
- Expected sub-agent dispatches:
  - Question: "Run `cargo test -p slicer-host --test threemf_subtypes_synthetic_e2e_tdd && cargo test -p slicer-host --test threemf_sidecar_classification_tdd && cargo test -p slicer-host --test benchy_4color_modifier_part_e2e_tdd && cargo test -p slicer-host --test benchy_painted_e2e_tdd && cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd`. Return FACT pass/fail per test file." → FACT.
  - Question: "Run `cargo clippy --workspace -- -D warnings`. FACT pass/fail with first warning if fail." → FACT.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: All regression suites GREEN; clippy clean.
- Exit condition: Clean workspace; no regressions.

### Step 3: Doc registration

- Task IDs: TASK-208
- Objective: Append TASK-208 row to `docs/07_implementation_status.md` after TASK-193.
- Precondition: Step 2 clean.
- Postcondition: `docs/07` reflects packet outcome.
- Files allowed to read:
  - `docs/07_implementation_status.md` — narrow read around line 147 (TASK-193 location) to confirm insertion point.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md`
- Files explicitly out-of-bounds: all source; `docs/DEVIATION_LOG.md` (no deviations in this packet).
- Expected sub-agent dispatches:
  - Question: "Append `[x] TASK-208` row to `docs/07_implementation_status.md` immediately after TASK-193 (line 147), naming packet `67_3mf-fixture-e2e-hardening`. Return the resulting line verbatim. SNIPPETS, ≤ 3 lines." → SNIPPETS.
- Context cost: S
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification:
  - `rg -c 'TASK-208.*67_3mf-fixture-e2e-hardening' docs/07_implementation_status.md` → 1.
- Exit condition: `rg` check passes.

### Step 4: Pre-ceremony verification

- Task IDs: TASK-208
- Objective: Re-run every pipe-suffixed AC command from `packet.spec.md` to confirm 11 GREEN / 1 RED status before closure.
- Precondition: Step 3 complete.
- Postcondition: All AC commands return expected results (11 pass, 1 fails with documented message).
- Files allowed to read: `packet.spec.md` (this packet).
- Files allowed to edit (≤ 3): none.
- Files explicitly out-of-bounds: every source file.
- Expected sub-agent dispatches:
  - One dispatch per AC command, each returning FACT pass/fail with assertion message for failures.
- Context cost: S
- Authoritative docs: this packet's `packet.spec.md`.
- OrcaSlicer refs: none.
- Verification: All AC commands return expected results.
- Exit condition: 11 GREEN, 1 RED with documented messages.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
|---|---|---|
| Step 0 | S | Precondition gate (two FACTs). |
| Step 1 | M | Author 12 test functions with fixture loading, pipeline calls, area assertions. |
| Step 2 | S | Regression sweep + clippy dispatches. |
| Step 3 | S | Doc registration. |
| Step 4 | S | Pre-ceremony AC verification dispatches. |

Aggregate: **M** (1 M + 4 S).

## Packet Completion Gate

- All 5 steps complete.
- Every step exit condition met.
- 11 GREEN tests pass; 1 RED test fails with a documented assertion message.
- `docs/07_implementation_status.md` updated with TASK-208 row.
- All regression suites GREEN; clippy clean.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md` (Step 4).
- Confirm 11 GREEN / 1 RED status matches expectations.
- No `cargo test --workspace` required — this is a test-only packet with zero production code changes. The regression sweep (Step 2) covers all affected suites.
- The RED test serves as hardening — it documents the extruder gap and will turn GREEN when Packet 68 lands.
