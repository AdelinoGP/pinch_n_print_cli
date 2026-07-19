# Implementation Plan: support-validation-wedge-harness

## Execution Rules

- Work one atomic test-harness step at a time. No production source, WIT, manifest, scheduler, or IR edits are allowed.
- The actual Cargo target is `--test integration`; each new test file is a submodule of `crates/slicer-runtime/tests/integration/main.rs`.
- Do not capture goldens until packets 116, 117, and 118 are implemented, packet 117's geometric fixes are present in the tree, and `cargo xtask build-guests --check` is clean.
- Never weaken an invariant because current planner output fails. Surface the failure to the owning support packet.
- Do not run Cargo commands in this authoring session. Commands below are worker dispatch contracts.

## Steps

### Step 1: Confirm prerequisites, driver, and current IR shape

- Task IDs: none retained; source-plan `TASK-260` is recorded as a collision.
- Objective: confirm packet 117 is implemented, locate the real integration aggregate and fixture helpers, and verify the current `prepare_prepass_context`/`SupportPlanIR` shapes before writing tests.
- Precondition: current tree and authority docs are available.
- Postcondition: the worker can name the actual `integration` target, `prepare_prepass_context`, `PrepassContext.blackboard`, `SupportPlanIR.entries`, `ExtrusionPath3D.points`, and `SupportGeometryIR.entries`; all source-plan shape mismatches are recorded.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/117_support-planner-geometric-correctness/packet.spec.md` - metadata only.
  - `docs/07_implementation_status.md` - targeted fixture and `TASK-260` rows.
  - `crates/slicer-runtime/Cargo.toml` - test target declarations.
  - `crates/slicer-runtime/tests/integration/main.rs` - full small file.
  - `crates/slicer-runtime/tests/common/{mod.rs,wasm_cache.rs,slicer_cache.rs}` - named helper blocks.
  - `crates/slicer-runtime/src/run.rs` - `PrepassContext` and `prepare_prepass_context` only.
  - `crates/slicer-ir/src/slice_ir.rs` - current support structs only.
  - `docs/02_ir_schemas.md` - `IR 9b - SupportPlanIR` only.
- Files allowed to edit: none.
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/src/lib.rs`, `OrcaSlicerDocumented/**`, generated guests, `target/**`, binary fixture contents.
- Expected sub-agent dispatches:
  - Question: is packet 117 `status: implemented`, and are `tapered_radius` and `inflate_polygon` fixed? Scope: packet-117 metadata and named support-planner symbols. Return: `FACT` <= 5 lines.
  - Question: confirm the Cargo test target and helper paths. Scope: the named Cargo/test/common files. Return: `LOCATIONS` <= 15 entries.
  - Question: confirm current `SupportPlanIR` fields and the absence of `dist_to_top`/`raft_plan`. Scope: `slicer-ir` and `docs/02_ir_schemas.md`. Return: `FACT` <= 5 lines.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` C1 and Validation Strategy - direct bounded read.
  - `docs/07_implementation_status.md` - targeted read.
- OrcaSlicer refs: none.
- Verification:
  - `rg -q '^status: implemented$' .ralph/specs/117_support-planner-geometric-correctness/packet.spec.md` must pass before capture.
  - The current test target is recorded as `integration`, not a standalone new binary.
- Exit condition: prerequisite, driver, and IR shape are grounded, or the packet stays draft.

### Step 2: Add the shared real-prepass wedge helper

- Task IDs: none retained.
- Objective: create `prepare_wedge_context` that loads the exact STL, invokes `prepare_prepass_context` with `modules/core-modules`, supplies `support_enabled = true` by default, and returns a context with non-empty `SupportPlanIR` for enabled support.
- Precondition: Step 1 confirms the production prepass driver and packet 117 status.
- Postcondition: both test modules can use one helper; enabled empty output fails with a diagnostic assertion instead of being captured.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` - function call signature only.
  - `crates/slicer-runtime/tests/common/wasm_cache.rs` - path/cache style only.
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` - repository fixture path only.
  - `crates/slicer-runtime/tests/common/mod.rs` - helper module registration style.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/common/support_wedge.rs`
  - `crates/slicer-runtime/tests/common/mod.rs`
- Files explicitly out of bounds:
  - Runtime production source and support-planner internals.
  - Integration test files; Step 3 registers and consumes the helper.
- Expected sub-agent dispatches:
  - Question: run the helper once with `support_enabled = true`. Scope: wedge prepass. Return: `FACT` with `entries.len`, total branch count, and whether the plan is empty; never return full IR.
  - Question: run `cargo xtask build-guests --check` before the helper run. Scope: guest artifacts. Return: `FACT` `up to date` or `STALE: <list>`.
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` `PrePass::SupportGeometry`.
  - `docs/02_ir_schemas.md` `IR 9b`.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p slicer-runtime --all-targets` passes after helper registration.
  - Enabled wedge output is non-empty, or the step stops with the exact prepass failure.
- Exit condition: one real, non-empty enabled wedge context is available to both test modules.

### Step 3: Register the integration modules and write six observable invariants

- Task IDs: none retained.
- Objective: add the integration aggregate registrations and implement AC-1 through AC-6 using only public committed IR and canonical unit conversion.
- Precondition: Step 2 helper is registered and enabled wedge output is non-empty.
- Postcondition: six named tests compile and each asserts a falsifiable current-contract property: finite paths, endpoint collision exclusion, layer Z, overhang coverage, radius bounds, and no negative default raft entries.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - helper API.
  - `crates/slicer-runtime/tests/integration/main.rs` - aggregate registration.
  - `crates/slicer-ir/src/slice_ir.rs` - support fields and point types only.
  - `crates/slicer-runtime/src/blackboard.rs` - support geometry accessors only.
  - `docs/specs/support-modules-orca-port.md` - C1/Validation Strategy only.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/integration/main.rs`
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- Files explicitly out of bounds:
  - Runtime production source, support-planner source, golden resources, and golden test module.
  - Private support-planner structs or implementation-derived parent graph logic.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd`. Scope: new invariant module. Return: `FACT` per-test pass/fail and bounded failure `SNIPPETS`.
  - Question: inspect every mm/internal-unit comparison in the new test. Scope: new invariant file. Return: `FACT` that `Point2::from_mm`/`units_to_mm` is used at each boundary.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` `IR 9b`.
  - `docs/08_coordinate_system.md` - direct read for conversion helpers.
- OrcaSlicer refs: none.
- Verification:
  - `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd 2>&1 | tee target/test-output.log` runs the six tests.
- Exit condition: the six current observable invariants are implemented without hidden-state heuristics.

### Step 4: Add disabled-support negative coverage

- Task IDs: none retained.
- Objective: add AC-N1 to the invariant module using exact `support_enabled = false` config and distinguish an intentionally empty plan from an enabled empty-plan regression.
- Precondition: Step 3 invariant module compiles.
- Postcondition: disabled support yields an existing `SupportPlanIR` with empty `entries`, while the helper's enabled path still requires non-empty output.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - config override helper.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` - current assertions.
  - `crates/slicer-runtime/tests/integration/support_geometry_config_normalization_tdd.rs` - exact support config spelling.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/common/support_wedge.rs`
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
- Files explicitly out of bounds:
  - Production source, support planner, and golden files.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::support_disabled_produces_explicit_empty_plan`. Scope: AC-N1. Return: `FACT` pass/fail with bounded failure snippets.
- Context cost: `S`
- Authoritative docs:
  - `docs/01_system_architecture.md` support stage contract.
  - `docs/07_implementation_status.md` support config key row.
- OrcaSlicer refs: none.
- Verification:
  - AC-N1 test passes.
- Exit condition: empty disabled support is explicitly tested and cannot mask enabled fixture failure.

### Step 5: Add golden comparison and guarded regeneration

- Task IDs: none retained.
- Objective: implement the golden test, branch-count parser, endpoint parser, symmetric Hausdorff comparison, and `SUPPORT_WEDGE_REGEN_GOLDEN=1` write path without normal-test side effects.
- Precondition: Steps 2-4 are green and packet 117 is implemented.
- Postcondition: test fails clearly when files are absent, captures only from non-empty enabled output, and normal comparison uses count <= 10 percent and Hausdorff <= 0.5 mm.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - shared context and branch extraction.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` - output extraction pattern only.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - existing guarded golden convention only; do not copy private fixtures.
  - `docs/specs/support-modules-orca-port.md` - Validation Strategy tolerance values only.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs`
  - `crates/slicer-runtime/tests/integration/main.rs`
- Files explicitly out of bounds:
  - Golden files until the capture sub-step; no hand-authored baseline.
  - `xtask/**`, production source, support-planner source, and other resources.
- Expected sub-agent dispatches:
  - Question: run the golden test before capture. Scope: integration target. Return: `FACT` expected missing-golden failure only.
  - Question: inspect the endpoint parser and tolerance helper. Scope: new golden test. Return: `FACT` that first/last path points and symmetric Hausdorff are used.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` Validation Strategy.
  - `docs/02_ir_schemas.md` `IR 9b`.
- OrcaSlicer refs: none.
- Verification:
  - Test code compiles; pre-capture failure names missing goldens, not a hidden production failure.
- Exit condition: golden test is ready for an explicit capture run.

### Step 6: Capture committed wedge goldens

- Task IDs: none retained.
- Objective: run the guarded capture after all prerequisites and write the two named text resources from the actual current wedge plan.
- Precondition: packet 116, packet 117, and packet 118 are implemented; `cargo xtask build-guests --check` is clean; AC-1 enabled output is non-empty.
- Postcondition: branch count file contains one positive integer; endpoint file contains the sorted first/last triples for every branch path; no hand-editing is used.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` - guarded capture path.
  - `resources/golden/` - named output files only.
- Files allowed to edit:
  - `resources/golden/support_regression_wedge_branch_count.txt`
  - `resources/golden/support_regression_wedge_endpoints.txt`
- Files explicitly out of bounds:
  - All production files, packet directories, other resources, and `xtask/**`.
- Expected sub-agent dispatches:
  - Question: run the guarded capture with `SUPPORT_WEDGE_REGEN_GOLDEN=1 cargo test -p slicer-runtime --all-targets --test integration -- support_golden_regression_wedge_tdd::current_wedge_output_stays_within_self_capture_tolerance`. Scope: named goldens. Return: `FACT` with branch count and endpoint count only; never paste file contents.
  - Question: run `test -s` for both named golden files. Scope: resources. Return: `FACT` pass/fail.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` Validation Strategy.
- OrcaSlicer refs: none.
- Verification:
  - Both files are non-empty and the normal golden test passes without the regeneration environment variable.
- Exit condition: committed self-capture baseline exists and is reproducible.

### Step 7: Add intentional golden-drift negative test

- Task IDs: none retained.
- Objective: prove AC-N2 by mutating the parsed count in memory and asserting the comparison rejects drift above 10 percent without modifying committed resources.
- Precondition: Step 6 goldens exist and normal AC-7 passes.
- Postcondition: the negative test observes an error containing `branch count drift > 10%`; the files on disk remain unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` - comparison helper.
  - `resources/golden/support_regression_wedge_branch_count.txt` - read-only baseline.
- Files allowed to edit:
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs`
- Files explicitly out of bounds:
  - Both golden files, production source, and capture helper.
- Expected sub-agent dispatches:
  - Question: run `cargo test -p slicer-runtime --all-targets --test integration -- support_golden_regression_wedge_tdd::detects_intentional_branch_count_drift`. Scope: AC-N2. Return: `FACT` pass/fail with bounded failure snippets.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` Validation Strategy.
- OrcaSlicer refs: none.
- Verification:
  - AC-N2 passes and `git diff -- resources/golden/...` shows no test-time mutation.
- Exit condition: intentional large count drift is decisively rejected in memory.

### Step 8: Run final harness gates

- Task IDs: none retained.
- Objective: run freshness, all AC commands, targeted check, and clippy; record unresolved source-plan blockers without marking a nonexistent task complete.
- Precondition: Steps 1-7 complete and goldens are committed.
- Postcondition: all current ACs and negatives pass; no known side effects or stale guest artifacts remain; packet stays draft while `[BLOCK]` questions are unresolved.
- Files allowed to read, with ranges when over 300 lines:
  - Packet 119 artifacts.
  - `target/test-output.log` through targeted Grep/Read only.
  - The two named golden files.
- Files allowed to edit: none.
- Files explicitly out of bounds:
  - Implementation files, other packet directories, `docs/07_implementation_status.md`, and `target/**` except delegated test output.
- Expected sub-agent dispatches:
  - Question: run `cargo xtask build-guests --check`. Scope: guest artifacts. Return: `FACT` `up to date` or `STALE: <list>`.
  - Question: run all packet AC commands. Scope: packet 119 `packet.spec.md`. Return: `FACT` PASS/FAIL list.
  - Question: run `cargo check -p slicer-runtime --all-targets` and `cargo clippy -p slicer-runtime --all-targets -- -D warnings`. Scope: runtime test harness. Return: one `FACT` pass/fail result per command.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - All current ACs and negatives pass.
  - Freshness, check, and clippy pass.
- Exit condition: harness closure evidence exists; status cannot become `implemented` until the three `[BLOCK]` issues are resolved by the owner.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Prerequisite, target, and IR inventory. |
| Step 2 | M | Real wedge prepass helper. |
| Step 3 | M | Six current observable invariants. |
| Step 4 | S | Disabled-support negative. |
| Step 5 | M | Golden comparison and guarded regeneration. |
| Step 6 | M | Actual post-prerequisite capture. |
| Step 7 | S | In-memory drift detector. |
| Step 8 | S | Final harness gates. |

Aggregate: `M`. No step is `L`.

## Packet Completion Gate

- All eight steps and exits complete.
- AC-1 through AC-7 and AC-N1 through AC-N2 pass.
- Both goldens are committed, non-empty, and not written by normal tests.
- `cargo xtask build-guests --check` returns `up to date` immediately before final tests.
- `docs/07_implementation_status.md` receives no invented task ID; source-plan `TASK-260` remains a mapping blocker.
- The `dist_to_top`/parent-link and `raft_plan` blockers have explicit owner decisions before any implemented transition.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm the commands use the existing `--test integration` aggregate target.
- Confirm the golden capture happened after packet 117 and packet 118 closure and after a clean freshness check.
- Confirm no implementation file or other packet directory changed during this packet.
