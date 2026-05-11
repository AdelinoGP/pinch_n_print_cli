# Implementation Plan: 51_paint-semantic-region-overrides

## Execution Rules

- One atomic step at a time. Each step has its own precondition + postcondition + falsifying check.
- Each step honors the context-discipline preamble. Files-allowed-to-read, files-allowed-to-edit, expected dispatches, and context cost are budget contracts.
- Stop reading at 60% context. Hand off at 85%.
- `crates/slicer-macros/src/lib.rs`, `crates/slicer-sdk/`, and all `modules/core-modules/*` crates are out of bounds.
- Do not run `cargo test --workspace` at any step. Use targeted `cargo test -p <crate> --test <file>` only.

## Steps

### Step 1: Activation grounding (PaintRegionIR availability + warning surface)

- Task IDs: `TASK-181`
- Objective: Resolve activation-blocking open questions Q1-Q4 and ground three concrete unknowns: (a) the exact function signature in `region_mapping.rs` where `PaintRegionIR` must be available (current signature; what to extend); (b) the existing paint-annotation warning event code to reuse for unknown-semantic warnings; (c) confirm the actual public symbol path of the polygon intersection function (spec assumes `slicer_core::intersection` at `crates/slicer-core/src/polygon_ops.rs:98`; `slicer-helpers` does NOT export `polygon_ops`).
- Precondition: master is clean; failing tests at `benchy_painted_overrides_e2e_tdd.rs` already RED; Packet 50 in flight (not blocking Step 1).
- Postcondition: 7 FACT/SNIPPET answers captured and recorded in design.md "Open Questions" section as resolutions. Packet flips from `draft` to `active` only after Q1-Q4 are recorded.
- Files allowed to read:
  - none directly. All discovery via dispatch.
- Files allowed to edit:
  - `.ralph/specs/51_paint-semantic-region-overrides/design.md` (record Q1-Q4 resolutions).
  - `.ralph/specs/51_paint-semantic-region-overrides/packet.spec.md` (flip `status: draft` → `active` after resolutions).
- Files explicitly out-of-bounds:
  - any source file (read at later steps).
- Expected sub-agent dispatches:
  - `Question: In crates/slicer-host/src/region_mapping.rs, what is the exact signature and call-site of execute_region_mapping (or the function that builds RegionMapIR)? Does it currently have access to PaintRegionIR? If not, where does its caller (likely in prepass.rs or similar) live so we can plumb PaintRegionIR through? Return: SNIPPET ≤ 30 lines with file:line.`
  - `Question: List the existing paint-annotation warning event codes/types in the codebase. rg for "paint_annotation_warning" or similar. Return: SNIPPET ≤ 15 lines.`
  - `Question: Confirm the actual public symbol path of the polygon intersection function. Spec assumes slicer_core::intersection (re-exported from crates/slicer-core/src/polygon_ops.rs:98). Verify (a) crates/slicer-helpers does NOT expose polygon_ops::intersection, (b) the correct public path is slicer_core::intersection (or slicer_core::polygon_ops::intersection), and (c) return the function's full public signature. Return: FACT + SNIPPET ≤ 10 lines.`
  - `Question: Return RegionPlan struct from crates/slicer-ir/src/slice_ir.rs:1006-1080 verbatim. Return: SNIPPET ≤ 80 lines.`
  - `Question: Return resolve_per_object_configs from crates/slicer-host/src/config_resolution.rs:186-216 verbatim. Return: SNIPPET ≤ 30 lines (the template for the new function).`
  - `Question: Inventory tests/fixtures that hash full RegionPlan values and may need re-blessing after the paint_overrides field is added. rg -n 'RegionPlan' crates/slicer-host/tests/ | head -20. Return: LOCATIONS ≤ 20 entries.`
  - `Question: Does PrePass::PaintSegmentation produce PaintRegionIR before PrePass::RegionMapping runs in the current dispatch order? Check crates/slicer-host/src/prepass.rs or dispatch.rs for the stage ordering. Return: FACT yes/no with file:line.`
- Context cost: S (seven FACT/SNIPPET dispatches; activation gate).
- Authoritative docs: none directly.
- OrcaSlicer refs: none.
- Verification: 7 dispatches succeed; resolutions recorded.
- Exit condition: design.md Open Questions section shows Q1-Q4 resolved; packet.spec.md status flipped to `active`.

### Step 2: Author failing unit tests (TDD-RED)

- Task IDs: `TASK-181`
- Objective: Author two test files (`config_resolution_paint_semantic_tdd.rs` and `region_mapping_paint_semantic_tdd.rs`) with their tests in RED state. The compile target may not exist yet (new IR field; new function); the tests should fail with "function not found" / "field not found" / assertion failure, NOT with compile errors that prevent the build.
- Precondition: Step 1 resolved Q1-Q4 and grounded the function signatures.
- Postcondition: Both test files exist; `cargo build --tests` passes (tests compile against the new symbol names we're about to add — using `#[ignore]` on tests blocked by symbols-not-yet-added is acceptable for this step, with `#[ignore]` removed in subsequent steps).
- Files allowed to read:
  - `crates/slicer-host/tests/benchy_painted_overrides_e2e_tdd.rs` (≤ 200 lines; the existing failing E2E test that informs the unit-test contract).
  - `crates/slicer-host/tests/region_mapping_tdd.rs` if it exists (template for region_mapping_paint_semantic_tdd.rs).
- Files allowed to edit (≤ 2):
  - `crates/slicer-host/tests/config_resolution_paint_semantic_tdd.rs` (new).
  - `crates/slicer-host/tests/region_mapping_paint_semantic_tdd.rs` (new).
- Files explicitly out-of-bounds:
  - any source file.
- Expected sub-agent dispatches:
  - `Question: Run cargo build --tests -p slicer-host. Return FACT pass/fail. If fail, return first 5 lines of error.`
- Context cost: S.
- Authoritative docs: none directly.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --tests -p slicer-host` PASS (test files compile; tests may be `#[ignore]`d temporarily).
  - Five new tests exist by name: `resolves_paint_config_namespace`, `unknown_semantic_warns_then_ignores`, `region_overlap_applies_override`, `no_overlap_keeps_object_config`, `overlap_precedence_is_deterministic`.
- Exit condition: test files committed; tests compile.

### Step 3: Extend `config_resolution.rs` with `paint_config:` namespace

- Task IDs: `TASK-181`
- Objective: Add `paint_config:<semantic>:<key>` parsing; add `resolve_per_paint_semantic_configs`; emit structured warning for unknown semantics. Flip the two config_resolution tests from `#[ignore]` to live and confirm they PASS.
- Precondition: Step 2 tests authored.
- Postcondition: AC-1 GREEN; AC-NEG-1 (unknown semantic warns) GREEN; `cargo build --workspace` PASS.
- Files allowed to read:
  - `crates/slicer-host/src/config_resolution.rs` (full file; expected ≤ 350 lines).
  - `crates/slicer-ir/src/slice_ir.rs:172-184` (PaintSemantic; for the string-repr of `Custom(...)`).
- Files allowed to edit (≤ 1):
  - `crates/slicer-host/src/config_resolution.rs`.
- Files explicitly out-of-bounds:
  - any other source file (yet).
- Expected sub-agent dispatches:
  - `Question: Run cargo build --workspace after the config_resolution.rs extension. Return FACT pass/fail.`
  - `Question: Run cargo test -p slicer-host --test config_resolution_paint_semantic_tdd. Return FACT pass/fail per test.`
- Context cost: S.
- Authoritative docs: `docs/02_ir_schemas.md:103-122` for PaintSemantic shape.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` PASS.
  - `cargo test -p slicer-host --test config_resolution_paint_semantic_tdd` all PASS.
- Exit condition: AC-1, AC-NEG-1 GREEN.

### Step 4: Extend `RegionPlan` IR + bump schema_version

- Task IDs: `TASK-181`
- Objective: Add `paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>` field to `RegionPlan` in `crates/slicer-ir/src/slice_ir.rs:1028-1033`. Bump `RegionMapIR.schema_version` from `minor:0` to `minor:1` in `crates/slicer-host/src/region_mapping.rs:201-206`. Re-bless any deterministic-serialization fixtures identified in Step 1's RegionPlan inventory.
- Precondition: Step 3 complete.
- Postcondition: AC-2 GREEN; `cargo build --workspace` PASS; deterministic-serialization regression tests stay GREEN (after re-blessing).
- Files allowed to read:
  - `crates/slicer-ir/src/slice_ir.rs:1006-1080` (≤ 80 lines).
  - `crates/slicer-host/src/region_mapping.rs:190-220` (the schema_version construction site).
  - Re-blessing targets from Step 1 inventory.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs`.
  - `crates/slicer-host/src/region_mapping.rs` (the schema_version bump only — overlap logic in Step 5).
  - Test fixtures requiring re-bless (if any; Step 1 inventory). Cap at 1 file with surgical edits.
- Files explicitly out-of-bounds:
  - any other source file.
- Expected sub-agent dispatches:
  - `Question: Run cargo build --workspace. Return FACT pass/fail.`
  - `Question: Run cargo test -p slicer-ir. Return FACT pass/fail counts.`
  - `Question: Run any RegionPlan-serialization regression tests identified in Step 1 inventory. Return FACT.`
- Context cost: S.
- Authoritative docs: `docs/02_ir_schemas.md` versioning rules.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` PASS.
  - `cargo test -p slicer-ir` all PASS.
  - `rg -q 'paint_overrides: BTreeMap<PaintSemantic, ResolvedConfig>' crates/slicer-ir/src/slice_ir.rs` matches.
  - `rg -q 'minor: 1, patch: 0' crates/slicer-host/src/region_mapping.rs` matches.
- Exit condition: AC-2 GREEN.

### Step 5: Wire paint-aware overlay into `region_mapping.rs`

- Task IDs: `TASK-181`
- Objective: Read `PaintRegionIR`; compute per-region polygon overlap; apply override precedence; stamp `RegionPlan.config` (effective overlay) and `RegionPlan.paint_overrides` (audit map).
- Precondition: Step 4 complete (IR shape ready).
- Postcondition: AC-3 GREEN; AC-NEG-2 (no overlap default) GREEN; AC-NEG-3 (overlap precedence deterministic) GREEN.
- Files allowed to read:
  - `crates/slicer-host/src/region_mapping.rs` (full file; expected ≤ 250 lines).
  - `crates/slicer-host/src/paint_segmentation.rs:70-130` (consumer-side context for PaintRegionIR shape).
  - `crates/slicer-host/src/prepass.rs` if Step 1 grounded the caller-side plumbing path.
  - `crates/slicer-core/src/polygon_ops.rs` only the `intersection` signature at `:98` (≤ 10 lines; public symbol re-exported as `slicer_core::intersection`).
- Files allowed to edit (≤ 2):
  - `crates/slicer-host/src/region_mapping.rs`.
  - `crates/slicer-host/src/prepass.rs` ONLY if Step 1 grounded that the caller needs a plumbing change to forward `PaintRegionIR`. If not needed, leave this file untouched.
- Files explicitly out-of-bounds:
  - any other source file.
- Expected sub-agent dispatches:
  - `Question: Run cargo build --workspace. Return FACT pass/fail.`
  - `Question: Run cargo test -p slicer-host --test region_mapping_paint_semantic_tdd. Return FACT per test.`
- Context cost: M (overlap loop; precedence resolution; multi-file caller plumbing).
- Authoritative docs: `docs/02_ir_schemas.md:436` overlap precedence.
- OrcaSlicer refs: none.
- Verification:
  - `cargo build --workspace` PASS.
  - All three `region_mapping_paint_semantic_tdd` tests PASS.
- Exit condition: AC-3, AC-NEG-2, AC-NEG-3 GREEN.

### Step 6: Documentation + DEV-045 + TASK-181 closure

- Task IDs: `TASK-181`
- Objective: Edit `docs/01_system_architecture.md` to declare RegionMapping paint-aware; edit `docs/02_ir_schemas.md` to document (a) `paint_config:<semantic>:<key>` namespace, (b) `RegionMapIR` schema bump 1.0.0 → 1.1.0, (c) `RegionPlan.paint_overrides` field, (d) override precedence rule (global < per_object < per_paint_semantic), and (e) **a new sub-rule under the RegionMap section explicitly stating: when multiple paint semantics overlap a single region, sort by `PaintSemantic` string representation and overlay in ascending order, so the lexicographically-last semantic wins. This rule is distinct from `docs/02_ir_schemas.md:436`, which governs `paint_order`-based resolution inside `PrePass::PaintSegmentation`.** Flip DEV-045 to Closed; add `[x] TASK-181`; add 2026-MM-DD chronology entry to `docs/14_deviation_audit_history.md`.
- Precondition: Steps 1-5 complete.
- Postcondition: AC-8, AC-10 GREEN; docs reflect implementation.
- Files allowed to read:
  - `docs/01_system_architecture.md` (section `:107-114` only, ≤ 20 lines).
  - `docs/02_ir_schemas.md` (RegionPlan section near `:451-480`, ≤ 50 lines).
- Files allowed to edit (≤ 4 — one above the usual cap because four docs need closure-edits; each edit is small):
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
  - `docs/07_implementation_status.md` (via worker dispatch)
  - `docs/DEVIATION_LOG.md` (via worker dispatch)
  - `docs/14_deviation_audit_history.md` (via worker dispatch)
- Files explicitly out-of-bounds:
  - everything else.
- Expected sub-agent dispatches:
  - `Question: In docs/07_implementation_status.md, locate insertion point for new TASK-181 row (likely after TASK-180). Return: FACT file:line.`
  - `Question: In docs/DEVIATION_LOG.md, locate DEV-045 row. Return: FACT file:line.`
  - `Question: In docs/14_deviation_audit_history.md, locate chronology tail (before "## Legacy Backlog Crosswalk"). Return: FACT file:line.`
  - Per-edit verification: `Question: Verify <grep pattern> matches in <file>. Return: FACT`.
- Context cost: M (four docs; three delegated reads/edits).
- Authoritative docs: the four docs being edited.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q 'paint-semantic|paint_config' docs/01_system_architecture.md`.
  - `rg -q 'RegionMapIR.*1\.1\.0|schema.*1\.1\.0' docs/02_ir_schemas.md && rg -q 'paint_config:<semantic>' docs/02_ir_schemas.md`.
  - `rg -q 'lexicographic|lex order' docs/02_ir_schemas.md` (confirms the new RegionMap-stage multi-semantic precedence rule is documented; the rule must mention sorting `PaintSemantic` string repr and that lexicographically-last wins).
  - `rg -q '^\| DEV-045.*Closed' docs/DEVIATION_LOG.md`.
  - `rg -q '\[x\] TASK-181' docs/07_implementation_status.md`.
  - `rg -q '2026-...DEV-045' docs/14_deviation_audit_history.md` (or equivalent date string).
- Exit condition: AC-8, AC-10 GREEN.

### Step 7: Regression + E2E sweep

- Task IDs: `TASK-181`
- Objective: Run the regression-defense battery + the Packet-50-gated E2E test. AC-4 (E2E) is gated on Packet 50; if Packet 50 has not closed, AC-4 stays RED with a deferred-pending-Packet-50 marker. All other regression commands must be GREEN.
- Precondition: Steps 1-6 complete.
- Postcondition: AC-5, AC-6, AC-7, AC-9 GREEN; AC-4 GREEN if Packet 50 has closed, otherwise documented as deferred.
- Files allowed to read: none.
- Files allowed to edit: none (read-only verification step).
- Expected sub-agent dispatches:
  - `Question: Run cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_e2e_real_pipeline_produces_gcode -- --exact. Return FACT pass/fail.`
  - `Question: Run cargo test -p slicer-host --test benchy_painted_e2e_tdd (Packet 50). Return FACT pass/fail.`
  - `Question: Run cargo test -p slicer-host --test benchy_painted_overrides_e2e_tdd. Return FACT pass/fail and whether the painted fixture is present.`
  - `Question: Run the five Packet-43-rev1 regression commands listed in AC-7. Return FACT per command.`
  - `Question: Run cargo clippy --workspace -- -D warnings. Return FACT pass/fail.`
- Context cost: S.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all regression commands GREEN; clippy GREEN.
- Exit condition: AC-5, AC-6, AC-7, AC-9 GREEN.

### Step 8: Acceptance ceremony

- Task IDs: `TASK-181`
- Objective: Re-dispatch every pipe-suffixed AC verification command; confirm green.
- Precondition: Steps 1-7 complete.
- Postcondition: Packet ready to flip to `status: implemented`.
- Files allowed to read: none.
- Files allowed to edit:
  - `.ralph/specs/51_paint-semantic-region-overrides/packet.spec.md` (status flip from `active` to `implemented` after all ACs green).
- Files explicitly out-of-bounds: everything else.
- Expected sub-agent dispatches:
  - One dispatch per AC (re-run each pipe-suffixed command). Each returns FACT pass/fail.
  - Final dispatch: `cargo clippy --workspace -- -D warnings`. Return FACT.
- Context cost: M (10 positive ACs + 3 negative + clippy = 14 FACT dispatches).
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: all AC commands return PASS; clippy GREEN.
- Exit condition: all 13 ACs (10 positive + 3 negative) GREEN; clippy GREEN; packet ready for implementer to set `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| 1 | S | Seven FACT/SNIPPET dispatches; activation gate. |
| 2 | S | Two new test files committed (RED). |
| 3 | S | config_resolution.rs extension + two unit tests GREEN. |
| 4 | S | IR shape additive change + schema bump. |
| 5 | M | region_mapping.rs overlap loop + precedence + caller plumbing. |
| 6 | M | Four docs (three delegated). |
| 7 | S | Regression verification dispatches. |
| 8 | M | 14 FACT dispatches. |

Aggregate: M. No step is L. If any step measures L during execution, split before proceeding.

## Packet Completion Gate

- All 8 steps complete.
- Every step's exit condition met.
- All 13 ACs (10 positive + 3 negative) green.
- `cargo clippy --workspace -- -D warnings` green.
- DEV-045 flipped to Closed in `docs/DEVIATION_LOG.md`.
- TASK-181 closed `[x]` in `docs/07_implementation_status.md`.
- `docs/14_deviation_audit_history.md` chronology entry committed.
- `docs/01` + `docs/02` document the new mechanism.
- Packet 50 closed (so AC-4 can be GREEN, not deferred).
- `packet.spec.md` ready to move from `status: active` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`. Each returns FACT pass/fail.
- Confirm packet-level verification commands are green (cargo build, cargo clippy, targeted tests).
- Record the implementer's peak context usage. If it exceeded 70%, log it as a packet-authoring lesson — this packet was estimated M.
