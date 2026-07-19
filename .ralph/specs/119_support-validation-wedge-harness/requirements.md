# Requirements: support-validation-wedge-harness

## Packet Metadata

- Grouped task IDs: none retained.
- Removed source-plan ID: `TASK-260` - current `docs/07_implementation_status.md` assigns it to gyroid-infill raw-emission work, not support validation.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The source plan calls for a wedge invariant and self-capture harness, but its old packet assumes an unavailable test binary, a different `SupportPlanIR` shape, and hidden planner state that the current public IR does not expose. The current runtime has a real prepass-only driver in `prepare_prepass_context`, an existing `integration` aggregate target, `resources/regression_wedge.stl`, and `SupportPlanIR.entries[*].branch_segments[*].points[*]` as the observable support output.

This packet creates a harness around those current surfaces. It deliberately does not claim to test `PlannedSupportNode.dist_to_top`, parent links, or `SupportPlanIR.raft_plan`: none is present in the current public output. Those source-plan assertions remain `[BLOCK]` questions rather than being approximated by a fragile graph heuristic.

## In Scope

- Add `crates/slicer-runtime/tests/common/support_wedge.rs` with a real fixture helper that loads `resources/regression_wedge.stl`, calls `slicer_runtime::prepare_prepass_context`, selects `modules/core-modules`, and returns the prepass context with `Blackboard`, `ExecutionPlan`, and committed `SupportPlanIR`/`SupportGeometryIR` available.
- Register `support_wedge` in `crates/slicer-runtime/tests/common/mod.rs` and register the two test modules in `crates/slicer-runtime/tests/integration/main.rs`. The runnable Cargo target is `--test integration`; the files are aggregate submodules, not standalone test binaries.
- Add `support_invariants_wedge_tdd.rs` with six current-observable tests:
  - non-empty finite branch paths at `SupportPlanIR.entries[*].branch_segments[*].points[*]`;
  - branch endpoint exclusion from current `SupportGeometryIR.entries` collision outlines using `Point2::from_mm`/`units_to_mm` at the boundary;
  - point Z equal to the `LayerPlanIR` Z selected by `SupportPlanEntry.global_layer_index`;
  - downward overhang facet centroid coverage at its origin layer;
  - finite non-negative `Point3WithWidth.width / 2` bounded by `6.0` mm;
  - no negative support entries when default `support_raft_layers = 0`.
- Add an explicit disabled-support negative test that asserts `support_enabled = false` produces an empty `SupportPlanIR.entries` rather than silently passing an enabled empty plan.
- Add `support_golden_regression_wedge_tdd.rs` with a symmetric Hausdorff helper, branch-count tolerance, endpoint text parsing, and an in-memory mutated-count negative test. Endpoint goldens contain the first and last point of each branch path, sorted and formatted as `x y z` in mm to six decimal places.
- Add and commit `resources/golden/support_regression_wedge_branch_count.txt` as one integer and `resources/golden/support_regression_wedge_endpoints.txt` as one sorted triple per line. The test owns a guarded regeneration mode, enabled only by `SUPPORT_WEDGE_REGEN_GOLDEN=1`; normal tests never write resources.
- Capture the goldens only after packets 116, 117, and 118 are actually implemented and `cargo xtask build-guests --check` is clean. The capture run must fail if the wedge produces an empty support plan.

## Out of Scope

- Production support planner changes, including `smooth_nodes`, multi-neighbour propagation, build-plate pruning, paint migration, or raft-plan work.
- Any `SupportPlanIR` field, schema-version, WIT, manifest, scheduler, SDK, or host-service change.
- Comparison with real OrcaSlicer output. The self-capture is not `TASK-163b-orca-ref` and does not establish external parity.
- Hidden-state assertions for `PlannedSupportNode.dist_to_top`, parent/child topology, or branch ancestry. The current emitted `ExtrusionPath3D` has points and widths but no parent IDs or `dist_to_top` field.
- The source-plan C1 raft-plan-count assertion. Current `SupportPlanIR` has only `schema_version` and `entries`; packet 124 owns the proposed `raft_plan` field.
- New xtask or shell capture infrastructure. The guarded test regeneration path is sufficient and follows the existing support-planner golden pattern without adding a new dependency.
- Other fixtures, GUI visualization, performance benchmarks, and broad workspace test runs.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` C1 and Validation Strategy - source tolerances and intended invariant context; direct bounded read.
- `docs/02_ir_schemas.md` `IR 9b - SupportPlanIR` - current `entries`, `global_layer_index`, `object_id`, `region_id`, and `branch_segments` fields; direct bounded read.
- `docs/01_system_architecture.md` `PrePass::SupportGeometry` - host/guest ordering and `SupportPlanIR` production; direct bounded read.
- `crates/slicer-runtime/src/run.rs` `prepare_prepass_context` - real prepass-only production driver; range-read around the function.
- `crates/slicer-runtime/tests/integration/main.rs` - aggregate target registration; direct small read.
- `crates/slicer-runtime/tests/common/wasm_cache.rs` and `slicer_cache.rs` - existing artifact and fixture path helpers; targeted reads.
- `docs/07_implementation_status.md` - targeted rows for the `TASK-260` collision, wedge fixture, and support status.
- `CLAUDE.md` - test output tee and Guest WASM Staleness guidance.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-7`.
- Negative: `AC-N1` and `AC-N2`.
- Cross-packet impact: later support packets can add observable invariants to the same harness, but no packet may weaken the six current checks or widen the golden tolerances without an explicit review. Packet 124 may replace AC-6 only after `raft_plan` exists.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the three closure gates.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Confirm support-planner guest artifacts are current before any wedge prepass. | FACT `up to date` or `STALE: <list>` |
| `cargo test -p slicer-runtime --all-targets --test integration -- support_invariants_wedge_tdd 2>&1 \| tee target/test-output.log` | Six current observable invariants plus disabled-support negative test. | FACT per-test pass/fail; SNIPPETS <= 30 lines on failure |
| `cargo test -p slicer-runtime --all-targets --test integration -- support_golden_regression_wedge_tdd 2>&1 \| tee target/test-output.log` | Golden count/endpoint tolerance and intentional-drift detector. | FACT pass/fail; SNIPPETS <= 20 lines on failure |
| `test -s resources/golden/support_regression_wedge_branch_count.txt && test -s resources/golden/support_regression_wedge_endpoints.txt` | Confirm both committed goldens are present and non-empty. | FACT pass/fail |
| `cargo check -p slicer-runtime --all-targets` | Compile the helper, aggregate registration, and both test modules. | FACT pass/fail; SNIPPETS <= 20 lines on first error |
| `cargo clippy -p slicer-runtime --all-targets -- -D warnings` | Test-code lint gate. | FACT pass/fail; SNIPPETS <= 20 lines on first error |

## Step Completion Expectations

- The goldens are captured only after the three prerequisite packets are implemented and after an immediate clean guest freshness check. A draft or stale predecessor blocks capture; it is never silently accepted as a baseline.
- `prepare_prepass_context` is the single production prepass driver for the harness. Do not use `run_slice`, which returns only `SliceOutcome` and does not expose the committed support IR or audit context.
- The helper reads `SupportPlanIR.entries[*].branch_segments[*].points[*]` and `SupportGeometryIR.entries` only. It does not access private support-planner structs.
- `SupportPlanIR` points and widths are f32 millimetres, while `SupportGeometryIR` polygons use scaled `Point2` units. Every comparison converts at the boundary with the canonical helpers.
- Normal tests parse committed goldens and never write them. Regeneration must be an explicit environment-gated invocation and must report branch count and endpoint count without dumping the full plan.
- If the wedge produces zero entries with support enabled, the packet stops and records the failing fixture/prepass command. It does not capture zero-count goldens or weaken AC-1.

## Context Discipline Notes

- Range-read `crates/slicer-runtime/src/run.rs` around `prepare_prepass_context`; do not read the 989-line file end-to-end.
- Read only `IR 9b - SupportPlanIR` from `docs/02_ir_schemas.md`; do not infer fields from the source plan's proposed C6 struct.
- Read `crates/slicer-runtime/tests/common/wasm_cache.rs` and `slicer_cache.rs` only around the named helpers. Do not inspect binary STL or generated guest components.
- Do not read `modules/core-modules/support-planner/src/lib.rs`; the harness uses public IR and host support geometry only.
- Do not read `OrcaSlicerDocumented/**`; this packet is a self-capture validation harness, not an external parity port.
- Return `FACT` for Cargo runs, with bounded failure `SNIPPETS`; never return a full IR dump or full test log.
