---
status: implemented
packet: 119
task_ids: [290]
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-validation-wedge-harness

## Goal

Stand up a current-contract wedge harness that runs the real prepass against `resources/regression_wedge.stl`, asserts the observable `SupportPlanIR.entries[*].branch_segments[*].points` invariants, and guards branch-count and endpoint drift with committed self-capture goldens. Close the public `dist_to_top_mm` and `raft_plan` seams needed by the source-plan invariants.

## Scope Boundaries

Touches the runtime test harness, its shared fixture helper, the integration aggregate registration, two text goldens, and the additive support IR/WIT plumbing required by the completed closure implementation. `Point3WithWidth.dist_to_top_mm` and `SupportPlanIR.raft_plan` are public at schema version 1.2.0; the planner emits raft configuration but not raft geometry. The seam ABI continues to use a dedicated six-field WIT point record.

## Prerequisites and Blockers

- Depends on: packets 116, 117, and 118 in the batch queue; packet directories 116, 117, and 118 are implemented and their support-planner prerequisites are present.
- Unblocks: later support validation packets after the current-contract harness and public support seams are green.
- Closure resolutions: the source-plan `TASK-260` collision is re-numbered to `TASK-290`; `dist_to_top_mm` is emitted per point; and `raft_plan` is emitted from support configuration. ADR-0048 records all three decisions.

## Acceptance Criteria

- **AC-1. Given** the real prepass context prepared from `resources/regression_wedge.stl` with `support_enabled = true`, **when** the wedge invariant test runs, **then** `SupportPlanIR.entries` is non-empty, every `branch_segments` path has at least two finite `Point3WithWidth` points, and every `x`, `y`, `z`, and `width` is finite. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::support_plan_has_finite_branch_paths --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** the same context and current `SupportGeometryIR.entries`, **when** every first and last point of every branch path is checked against the support collision outlines after mm conversion, **then** finite endpoints within `1e-6` mm of `dist_to_top_mm == 0.0` are exempt as origin-contact tips because they must reach the overhang centroid and may lie on or inside model collision geometry; every finite propagated endpoint with `dist_to_top_mm > 0.0` must be outside an outer contour excluding its holes, and at least one propagated endpoint must be checked. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::branch_endpoints_are_outside_support_collision_outlines --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** the same context with default `support_raft_layers = 0`, **when** each `SupportPlanEntry` is checked, **then** every path point has a `z` equal to the `LayerPlanIR` layer Z selected by `global_layer_index` within `1e-4` mm. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::branch_points_match_entry_layer_z --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** each downward-facing wedge mesh triangle whose normal z-component is at or below `-sin(45 degrees)`, **when** its centroid is assigned to the first `LayerPlanIR` layer at or above the centroid Z, **then** at least one emitted branch endpoint at that layer is within `tree_support_branch_distance` mm of the centroid. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::overhang_facets_have_wedge_layer_contacts --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** all emitted `Point3WithWidth` values from the wedge plan, **when** branch radii are calculated as `width / 2`, **then** every radius is finite, non-negative, and no greater than the current `MAX_BRANCH_RADIUS_MM = 6.0` contract. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::branch_radii_stay_within_current_bounds --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** the wedge context with default `support_raft_layers = 0`, **when** `SupportPlanIR.entries` is inspected, **then** no entry has a negative `global_layer_index`; this is the current pre-C6 raft-prefix invariant. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::disabled_raft_has_no_negative_entries --nocapture 2>&1 | tee target/test-output.log`
- **AC-7. Given** committed `resources/golden/support_regression_wedge_branch_count.txt` and `resources/golden/support_regression_wedge_endpoints.txt`, **when** the golden test runs, **then** the current `branch_segments.len()` total stays within plus or minus 10 percent of the count baseline and the symmetric endpoint Hausdorff distance stays at or below `0.5` mm. | `cargo test -p slicer-runtime --all-targets --test integration -- support_golden_regression_wedge_tdd::current_wedge_output_stays_within_self_capture_tolerance --nocapture 2>&1 | tee target/test-output.log`
- **AC-8. Given** the emitted branch points, **when** `dist_to_top_mm` is inspected, **then** every value is finite and non-negative and at least one positive value is observed. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::branch_points_carry_finite_nonnegative_dist_to_top_mm --nocapture 2>&1 | tee target/test-output.log`
- **AC-9. Given** support configuration with `support_raft_layers = 2`, `raft_first_layer_density = 0.4`, `base_raft_layers = 1`, and `interface_raft_layers = 1`, **when** the support plan is inspected, **then** `raft_plan` is `Some` and carries exactly `2`, `0.4`, `1`, and `1`. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::enabled_raft_config_is_emitted_as_raft_plan --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** the same wedge fixture with `support_enabled = false`, **when** prepass completes, **then** `SupportPlanIR.entries` is empty and the harness reports the disabled-support case explicitly rather than treating an enabled empty plan as a pass. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::support_disabled_produces_explicit_empty_plan --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** an in-memory golden count mutated to more than 25 percent from its committed baseline, **when** the golden comparison helper runs, **then** it returns a failure containing `branch count drift > 10%` and the test passes only because that failure was detected. | `cargo test -p slicer-runtime --all-targets --test integration -- support_golden_regression_wedge_tdd::detects_intentional_branch_count_drift --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** support configuration with `support_raft_layers = 0`, **when** the support plan is inspected, **then** `raft_plan` is `None`. | `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::disabled_raft_config_has_no_raft_plan --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::branch_points_carry_finite_nonnegative_dist_to_top_mm --nocapture 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::enabled_raft_config_is_emitted_as_raft_plan --nocapture 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd::disabled_raft_config_has_no_raft_plan --nocapture 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --all-targets --test integration -- support_golden_regression_wedge_tdd 2>&1 | tee target/test-output.log`
- `cargo check -p slicer-runtime --all-targets`
- `cargo clippy -p slicer-runtime --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` sections C1, Validation Strategy, and D3 - source invariant names and tolerance values; direct bounded read.
- `docs/02_ir_schemas.md` section `IR 9b - SupportPlanIR` - current field paths; direct bounded read.
- `docs/01_system_architecture.md` `PrePass::SupportGeometry` section - real prepass ordering and guest stage contract; direct bounded read.
- `crates/slicer-runtime/src/run.rs` `prepare_prepass_context` - real production prepass driver; range-read only.
- `crates/slicer-runtime/tests/integration/main.rs` - actual aggregate test target; direct small read.
- `crates/slicer-runtime/tests/common/` - fixture and WIT artifact helpers; targeted symbol lookup.
- `docs/07_implementation_status.md` - targeted lookup proving the `TASK-260` collision and the free `TASK-290` replacement.
- `CLAUDE.md` - test-output tee and Guest WASM Staleness guidance.

## Doc Impact Statement (Required)

This packet changes specific authoritative sections. Each section has one
runnable verification grep:

- `docs/02_ir_schemas.md` — IR 9b `SupportPlanIR`, `RaftPlan`, and shared
  `Point3WithWidth` fields, including schema 1.2.0:
  `rg -n 'SupportPlanIR|RaftPlan|dist_to_top_mm|CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION' docs/02_ir_schemas.md`
- `docs/03_wit_and_manifest.md` — `deps/types.wit` support/seam point records,
  the `world-prepass.wit` `push-raft-plan` seam, and support raft config schema:
  `rg -n 'raft-plan|seam-point3-with-width|dist-to-top-mm|raft_first_layer_density' docs/03_wit_and_manifest.md`
- `docs/05_module_sdk.md` — `PrePass::SupportGeometry` output behavior,
  optional configuration-only raft seam, and packet-124 geometry boundary:
  `rg -n 'support-planner|SupportGeometryOutput|push_raft_plan|raft_plan|raft geometry' docs/05_module_sdk.md`
- `docs/specs/support-modules-orca-port.md` — C1/Validation Strategy origin-contact contract and C6 optional raft seam:
  `rg -n '### C1|### Validation Strategy|### C6|origin-contact|raft_plan: Option' docs/specs/support-modules-orca-port.md`

The repository has no `docs/05_module_catalog.md`; `docs/05_module_sdk.md` is
the existing section-05 authority containing the affected module/config/output
contract. Raft geometry remains out of scope for packet 119 and is deferred to
packet 124.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
