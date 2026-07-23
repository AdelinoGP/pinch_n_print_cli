# Implementation Plan: support-validation-wedge-harness

## Execution Rules

- Work one atomic step at a time and keep the implementation surface limited to the packet's harness and additive support contract closure. No unrelated production, manifest, scheduler, or IR edits are allowed.
- The actual Cargo target is `--test integration`; each new test file is a submodule of `crates/slicer-runtime/tests/integration/main.rs`.
- Do not capture goldens until packets 116, 117, and 118 are implemented, packet 117's geometric fixes are present in the tree, and `cargo xtask build-guests --check` is clean.
- Never weaken an invariant because current planner output fails. Surface the failure to the owning support packet.
- The commands below are the verification contracts used for the completed implementation and final documentation closure.

## Steps

### Step 1: Confirm prerequisites, driver, and current IR shape

- Task IDs: packet-local; the source-plan `TASK-260` collision is resolved as `TASK-290` for the absorbed closure work.
- Objective: confirm packet 117 is implemented, locate the real integration aggregate and fixture helpers, and verify the current `prepare_prepass_context`/`SupportPlanIR` shapes before writing tests.
- Precondition: current tree and authority docs are available.
- Postcondition: the worker can name the actual `integration` target, `prepare_prepass_context`, `PrepassContext.blackboard`, `SupportPlanIR.entries`, `ExtrusionPath3D.points`, and `SupportGeometryIR.entries`; all source-plan shape mismatches are recorded. Two planner bugs are discovered during this step: (a) the MST segment-emission branch in `plan_for_object` (in `modules/core-modules/support-planner/src/lib.rs`) has no collision guard, (b) `clamp_to_avoidance` returns scaled-unit values for some callsites; the related defensive conversions in `point_in_any_expoly` and `push_interface_scan_lines` are also at risk. Both are absorbed into this packet and resolved in Step 9.
- Files allowed to read, with ranges when over 300 lines:
  - `.ralph/specs/117_support-planner-geometric-correctness/packet.spec.md` - metadata only.
  - `docs/07_implementation_status.md` - targeted fixture, `TASK-260`, and `TASK-290` rows.
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
  - Question: confirm the initial `SupportPlanIR` shape and the additive `dist_to_top_mm`/`raft_plan` closure targets. Scope: `slicer-ir` and `docs/02_ir_schemas.md`. Return: `FACT` <= 5 lines.
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

- Task IDs: packet-local.
- Objective: create `prepare_wedge_context` that loads the exact STL, invokes `prepare_prepass_context` with `modules/core-modules`, supplies `support_enabled = true` by default, and returns a context with non-empty `SupportPlanIR` for enabled support.
- Precondition: Step 1 confirms the production prepass driver and packet 117 status.
- Postcondition: both test modules can use one helper; enabled empty output fails with a diagnostic assertion instead of being captured.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/run.rs` - function call signature only.
  - `crates/slicer-runtime/tests/common/wasm_cache.rs` - path/cache style only.
  - `crates/slicer-runtime/tests/common/slicer_cache.rs` - repository fixture path only.
  - `crates/slicer-runtime/tests/common/mod.rs` - helper module registration style.
- Files to edit:
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

- Task IDs: packet-local.
- Objective: add the integration aggregate registrations and implement AC-1 through AC-6 using only public committed IR and canonical unit conversion. AC-2 permits finite origin-contact tips within `1e-6` mm of `dist_to_top_mm == 0.0` to remain on or inside collision outlines because they are raw centroids required to contact the overhang; positive propagated endpoints must still pass the existing outside predicate.
- Precondition: Step 2 helper is registered and enabled wedge output is non-empty.
- Postcondition: six named tests compile and each asserts a falsifiable current-contract property: finite paths; AC-2's origin-contact exemption plus outside checks for every propagated endpoint and at least one propagated check; layer Z; overhang coverage; radius bounds; and no negative default raft entries.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - helper API.
  - `crates/slicer-runtime/tests/integration/main.rs` - aggregate registration.
  - `crates/slicer-ir/src/slice_ir.rs` - support fields and point types only.
  - `crates/slicer-runtime/src/blackboard.rs` - support geometry accessors only.
  - `docs/specs/support-modules-orca-port.md` - C1/Validation Strategy only.
- Files to edit:
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

- Task IDs: packet-local.
- Objective: add AC-N1 to the invariant module using exact `support_enabled = false` config and distinguish an intentionally empty plan from an enabled empty-plan regression.
- Precondition: Step 3 invariant module compiles.
- Postcondition: disabled support yields an existing `SupportPlanIR` with empty `entries`, while the helper's enabled path still requires non-empty output.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - config override helper.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` - current assertions.
  - `crates/slicer-runtime/tests/integration/support_geometry_config_normalization_tdd.rs` - exact support config spelling.
- Files to edit:
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

- Task IDs: packet-local.
- Objective: implement the golden test, branch-count parser, endpoint parser, symmetric Hausdorff comparison, and `SUPPORT_WEDGE_REGEN_GOLDEN=1` write path without normal-test side effects.
- Precondition: Steps 2-4 are green and packet 117 is implemented.
- Postcondition: test fails clearly when files are absent, captures only from non-empty enabled output, and normal comparison uses count <= 10 percent and Hausdorff <= 0.5 mm.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/common/support_wedge.rs` - shared context and branch extraction.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` - output extraction pattern only.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - existing guarded golden convention only; do not copy private fixtures.
  - `docs/specs/support-modules-orca-port.md` - Validation Strategy tolerance values only.
- Files to edit:
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

- Task IDs: packet-local.
- Objective: run the guarded capture after all prerequisites and write the two named text resources from the actual current wedge plan.
- Precondition: packet 116, packet 117, and packet 118 are implemented; `cargo xtask build-guests --check` is clean; AC-1 enabled output is non-empty.
- Postcondition: branch count file contains one positive integer; endpoint file contains the sorted first/last triples for every branch path; no hand-editing is used.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` - guarded capture path.
  - `resources/golden/` - named output files only.
- Files to edit:
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

- Task IDs: packet-local.
- Objective: prove AC-N2 by mutating the parsed count in memory and asserting the comparison rejects drift above 10 percent without modifying committed resources.
- Precondition: Step 6 goldens exist and normal AC-7 passes.
- Postcondition: the negative test observes an error containing `branch count drift > 10%`; the files on disk remain unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` - comparison helper.
  - `resources/golden/support_regression_wedge_branch_count.txt` - read-only baseline.
- Files to edit:
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

- Task IDs: packet-local.
- Objective: run freshness, all current AC commands, targeted check, and clippy before the public contract closure steps.
- Precondition: Steps 1-7 complete and goldens are committed.
- Postcondition: all current ACs and negatives pass; no known side effects or stale guest artifacts remain; packet remains draft until its separately requested final status flip.
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
- Exit condition: harness closure evidence exists and the implementation is ready for the additive contract closure steps.

### Step 9: Apply planner unit-consistency and collision-guard fixes

- Task IDs: `TASK-290` (absorbed packet-119 planner closure work).
- Objective: apply the collision guard in the MST segment-emission branch, correct the scaled-unit conversion in `clamp_to_avoidance`, and preserve the defensive conversions in `point_in_any_expoly` and `push_interface_scan_lines`. The implementation is confined to `modules/core-modules/support-planner/src/lib.rs`.
- Precondition: Steps 1-8 complete; AC-2 fails due to planner bugs.
- Postcondition: AC-2 passes with origin-contact exemptions only for finite `dist_to_top_mm == 0.0` tips within `1e-6` mm, every checked propagated endpoint outside its outline, and at least one propagated endpoint checked; all current wedge invariants pass; goldens re-capture cleanly with 0% drift and 0.0 mm Hausdorff.
- Files to read, with targeted ranges where needed:
  - `modules/core-modules/support-planner/src/lib.rs` - `run_support_geometry`, `plan_for_object`, `point_in_any_expoly`, `tapered_radius`, `clamp_to_avoidance`, and `push_interface_scan_lines`.
- Files to edit:
  - `modules/core-modules/support-planner/src/lib.rs` only.
- Files explicitly out of bounds:
  - All other support-planner files, all other packet files, production source outside the four fix sites.
- Expected sub-agent dispatches:
  - Question: verify the four named fix sites are present and correct. Scope: `modules/core-modules/support-planner/src/lib.rs`. Return: `FACT` naming the functions and summarizing the guards/conversions.
  - Question: run AC-2 after applying fixes. Scope: integration test. Return: `FACT` pass/fail.
  - Question: re-capture goldens and confirm 0% drift, 0.0 mm Hausdorff. Scope: golden test. Return: `FACT` with branch count and Hausdorff value.
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - unit-system reminder for the SCALING_FACTOR fix.
- OrcaSlicer refs: none.
- Verification:
  - `test -f modules/core-modules/support-planner/src/lib.rs`
  - `rg -q 'fn clamp_to_avoidance|fn point_in_any_expoly' modules/core-modules/support-planner/src/lib.rs`
  - `rg -q 'fn push_interface_scan_lines' modules/core-modules/support-planner/src/lib.rs`
  - AC-2 passes.
  - Goldens re-capture with 0% drift and 0.0 mm Hausdorff.
- Exit condition: AC-2 passes; goldens are reproducible; planner output is ready for the public support contract closure.

### Step 10: Add the public support IR and WIT seams

- Task IDs: `TASK-290`.
- Objective: add the additive `Point3WithWidth.dist_to_top_mm` and `SupportPlanIR.raft_plan: Option<RaftPlan>` contract at schema version 1.2.0, expose `push-raft-plan` in the canonical prepass WIT, and preserve an ABI-safe seam-specific six-field WIT point shape.
- Precondition: Steps 1-9 are complete and the source-plan collision is assigned to the free `TASK-290`.
- Postcondition: canonical WIT, IR, macro support mapping, host marshal, and SDK prepass types/builders agree on the widened public support shape; seam candidates continue to use `seam-point3-with-width` without ABI flattening failure.
- Files to read, with targeted ranges where needed:
  - `crates/slicer-schema/wit/deps/types.wit` - `point3-with-width` and `seam-point3-with-width` records.
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` - support-plan records and `support-geometry-output`.
  - `crates/slicer-ir/src/slice_ir.rs` - `Point3WithWidth`, `RaftPlan`, `SupportPlanIR`, and the support schema constant.
  - `crates/slicer-macros/src/lib.rs`, `crates/slicer-wasm-host/src/marshal/`, and `crates/slicer-sdk/src/prepass_{types,builders}.rs` - existing support point and raft mappings.
- Files to edit:
  - `crates/slicer-schema/wit/deps/types.wit`
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
  - `crates/slicer-macros/src/lib.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/src/host.rs`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
  - `crates/slicer-wasm-host/src/marshal/leaf.rs`
  - `crates/slicer-wasm-host/src/marshal/out.rs`
  - `crates/slicer-sdk/src/prepass_types.rs`
  - `crates/slicer-sdk/src/prepass_builders.rs`
  - `crates/slicer-ir/src/slice_ir.rs`
- Expected sub-agent dispatches:
  - Question: verify WIT record identity across host and guest consumers. Scope: the two canonical WIT files and named marshal/macro paths. Return: `FACT` naming the seven-field support point, six-field seam point, and raft push method.
  - Question: run `cargo xtask build-guests --check` after the WIT/IR edits. Scope: guest artifacts. Return: `FACT` `up to date` or `STALE: <list>`.
- Context cost: `M`
- Verification:
  - `cargo xtask build-guests --check`
  - `cargo check -p slicer-runtime --all-targets`
  - WIT type identity and seam record shape are checked against the canonical files under `crates/slicer-schema/wit/`.
- Exit condition: the host, macro, guest, SDK, and IR support seams compile against one canonical WIT shape, with schema 1.2.0 recorded.

### Step 11: Emit planner configuration and finish guest integration

- Task IDs: `TASK-290`.
- Objective: parse and emit `support_raft_layers`, `raft_first_layer_density`, `base_raft_layers`, and `interface_raft_layers`; forward planner `dist_to_top_mm`; and reconcile the seam planner guest Cargo package name required by the updated seam ABI.
- Precondition: Step 10's IR, WIT, macro, host, and SDK seams are present.
- Postcondition: enabled raft configuration emits one plan with values `2, 0.4, 1, 1` in the wedge test, disabled raft emits no plan, and branch points carry finite, non-negative distance values with at least one positive observation.
- Files to read, with targeted ranges where needed:
  - `modules/core-modules/support-planner/src/lib.rs` - `on_print_start`, `run_support_geometry`, `plan_for_object`, and support point construction.
  - `modules/core-modules/support-planner/support-planner.toml` - the support and raft config schema entries.
  - `modules/core-modules/seam-planner-default/wit-guest/Cargo.toml` - package identity and WIT guest dependency shape.
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` - AC-8, AC-9, and AC-N3 filters.
- Files to edit:
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/support-planner.toml`
  - `modules/core-modules/seam-planner-default/wit-guest/Cargo.toml`
- Expected sub-agent dispatches:
  - Question: verify the four config keys, the `Option<RaftPlan>` emission condition, and per-point distance forwarding. Scope: the named planner source and manifest. Return: `FACT` with symbol names and exact defaults.
  - Question: run the enabled-raft, disabled-raft, and distance integration filters. Scope: `--test integration`. Return: `FACT` per-filter pass/fail.
- Context cost: `S`
- Verification:
  - `cargo xtask build-guests --check`
  - `cargo check -p slicer-runtime --all-targets`
  - `cargo clippy -p slicer-runtime --all-targets -- -D warnings`
- Exit condition: planner config/source and the seam guest package build with fresh guests and no ABI mismatch.

### Step 12: Add closure invariants and run the final gates

- Task IDs: `TASK-290`.
- Objective: add AC-8, AC-9, and AC-N3 to the wedge integration harness and verify the complete packet against the updated public IR/WIT contract.
- Precondition: Steps 10-11 are complete and guest freshness is clean.
- Postcondition: AC-1 through AC-9 and AC-N1 through AC-N3 pass; AC-8 observes finite, non-negative distances with a positive value; AC-9 observes exact raft values `2, 0.4, 1, 1`; AC-N3 observes `None` for `support_raft_layers = 0`.
- Files to read, with targeted ranges where needed:
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` - all current invariant functions and the three closure filters.
  - `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` - self-capture comparison and negative drift test.
  - `packet.spec.md` - acceptance commands and current-contract wording.
  - `docs/02_ir_schemas.md`, `docs/03_wit_and_manifest.md`, and `docs/05_module_sdk.md` - updated public support contract sections.
- Files to edit:
  - `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs`
  - `crates/slicer-runtime/tests/integration/main.rs`
  - affected support point-construction tests
- Expected sub-agent dispatches:
  - Question: run every packet AC and negative filter with the required test-output tee. Scope: packet 119 integration target. Return: `FACT` PASS/FAIL per command.
  - Question: run `cargo xtask build-guests --check`, `cargo check -p slicer-runtime --all-targets`, and `cargo clippy -p slicer-runtime --all-targets -- -D warnings`. Scope: final packet gates. Return: one `FACT` pass/fail result per command.
- Context cost: `M`
- Verification:
  - `cargo xtask build-guests --check`
  - Every pipe-suffixed command in `packet.spec.md`, including the AC-8, AC-9, and AC-N3 filters.
  - `cargo check -p slicer-runtime --all-targets`
  - `cargo clippy -p slicer-runtime --all-targets -- -D warnings`
- Exit condition: all acceptance tests and freshness/check/clippy gates pass; documentation can retain `status: draft` until the separate status-flip step.

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
| Step 9 | S | Planner unit-consistency and collision-guard fixes (33 lines, one file). |
| Step 10 | M | Additive IR/WIT/macro/host-marshal/SDK support seams, including ABI-safe seam point shape. |
| Step 11 | S | Planner config/source emission and seam guest Cargo package reconciliation. |
| Step 12 | M | AC-8, AC-9, AC-N3 and final freshness/check/clippy gates. |

Aggregate: `M`. No step is `L`.

## Packet Completion Gate

- All twelve steps and exits complete.
- AC-1 through AC-9 and AC-N1 through AC-N3 pass.
- Both goldens are committed, non-empty, and not written by normal tests.
- `cargo xtask build-guests --check` returns `up to date` immediately before final tests.
- `SupportPlanIR` schema version is 1.2.0 and the canonical WIT seam uses the dedicated six-field seam point shape.
- `docs/07_implementation_status.md` records the free `TASK-290` row without changing the existing gyroid `TASK-260` row.
- `cargo check -p slicer-runtime --all-targets` and `cargo clippy -p slicer-runtime --all-targets -- -D warnings` pass.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC command from `packet.spec.md`.
- Confirm the commands use the existing `--test integration` aggregate target.
- Confirm the golden capture happened after packet 117 and packet 118 closure and after a clean freshness check.
- Confirm AC-8, AC-9, and AC-N3 cover the public distance and raft seams, including the six-field seam WIT shape and schema 1.2.0.
- Confirm no implementation file or other packet directory changed during this packet.
