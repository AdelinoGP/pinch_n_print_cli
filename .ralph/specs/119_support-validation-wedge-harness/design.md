# Design: support-validation-wedge-harness

## Controlling Code Paths

- `crates/slicer-runtime/src/run.rs::prepare_prepass_context` is the real prepass-only driver. It returns `PrepassContext { plan, blackboard, ... }`, exposing committed `SupportPlanIR` and `SupportGeometryIR` without running per-layer or G-code stages.
- `crates/slicer-runtime/tests/common/support_wedge.rs::prepare_wedge_context` is the shared test helper. It loads `resources/regression_wedge.stl`, uses `modules/core-modules`, and fails if enabled support produces no plan.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` owns the current-observable invariant tests and the disabled-support/disabled-raft negative tests.
- `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` owns golden parsing, guarded regeneration, symmetric Hausdorff comparison, and intentional count-drift detection.
- `crates/slicer-runtime/tests/integration/main.rs` and `tests/common/mod.rs` own aggregate registration.
- `resources/golden/support_regression_wedge_branch_count.txt` and `support_regression_wedge_endpoints.txt` are committed self-capture inputs.

## Architecture Constraints

- The harness dispatches the real support-planner guest through `prepare_prepass_context`. `cargo xtask build-guests --check` must be clean immediately before capture and final verification, and stale guests must be rebuilt rather than attributed to the harness.
- `SupportPlanIR` is now schema version 1.2.0. `Point3WithWidth.dist_to_top_mm` is an additive per-point field, and `SupportPlanIR.raft_plan` is an additive optional configuration seam. The planner emits raft configuration only; packet 124 owns raft geometry.
- The public WIT geometry record has seven support-point fields, including `dist-to-top-mm`. Seam candidates intentionally use the separate six-field `seam-point3-with-width` record. Keeping the seam record separate avoids the component-model ABI flattening failure caused by routing seam exports through the widened support-point shape.
- The harness asserts public IR only: `SupportPlanIR.entries[*].branch_segments[*].points[*]`, `SupportPlanEntry.global_layer_index`, `SupportPlanIR.raft_plan`, and `SupportGeometryIR.entries`. It does not inspect private planner state or reconstruct parent links.
- `dist_to_top_mm` is checked on every emitted point. Values must be finite and non-negative, with at least one positive value; the public IR does not carry parent identity or a chain-order contract.
- `SupportGeometryIR.entries` uses scaled `Point2`/`ExPolygon` units, so endpoint checks convert branch coordinates at the boundary. The test must not compare raw f32 mm values to raw internal units.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The golden branch-count baseline is the total `entry.branch_segments.len()` across all entries. Endpoint goldens contain the first and last point of each branch path, sorted lexicographically and formatted to six decimal places.
- The committed goldens are read-only during normal tests. Regeneration is explicit with `SUPPORT_WEDGE_REGEN_GOLDEN=1` and must fail on an empty enabled plan.
- Tolerances are locked to source-plan values: count relative drift <= `0.10`, symmetric endpoint Hausdorff <= `0.5` mm.

## Code Change Surface

- Selected approach:
  - Reuse `prepare_prepass_context` instead of `run_slice`, because `run_slice` returns only `SliceOutcome` and hides support IR.
  - Put shared setup in `tests/common/support_wedge.rs` so both invariant and golden modules use one driver and one config path.
  - Derive collision checks from committed `SupportGeometryIR.entries` and use the current support outlines as the observable collision envelope. Exempt finite origin-contact endpoints within `1e-6` mm of `dist_to_top_mm == 0.0`, because those raw contact centroids are required to reach the overhang and may be on or inside the model outline; require every positive propagated endpoint to pass the outside predicate.
  - Derive overhang contacts from the loaded wedge mesh and `SupportPlanIR` entries, using the current default 45-degree threshold and `tree_support_branch_distance` tolerance.
  - Assert per-point `dist_to_top_mm` directly on emitted points rather than infer parent links.
  - Assert the optional `raft_plan` seam for both enabled and disabled raft configuration; leave geometry generation to packet 124.
- Exact functions and fields:
  - `prepare_wedge_context` returns a `PrepassContext` and asserts `support_enabled` output is non-empty when enabled.
  - `SupportPlanIR.entries`, `SupportPlanIR.raft_plan`, `SupportPlanEntry.global_layer_index`, `object_id`, `region_id`, `branch_segments`, `ExtrusionPath3D.points`, and `Point3WithWidth.{x,y,z,width,dist_to_top_mm}`.
  - `Blackboard::support_geometry`, `SupportGeometryIR.entries`, `SupportGeometryKey`, `ExPolygon.contour`, and `ExPolygon.holes`.
  - `LayerPlanIR.global_layers` and `GlobalLayer.{index,z}` for layer-Z matching.
  - Golden helpers `branch_segment_count`, `branch_endpoints`, `parse_branch_count`, `parse_endpoints`, and `symmetric_hausdorff`.
- Rejected alternatives:
  - `run_slice`: rejected because it exposes only final G-code and cannot prove `SupportPlanIR` invariants.
  - A new production introspection API: rejected; the current blackboard already exposes the committed IR through `PrepassContext`.
  - Reconstructing parent links from shared endpoints: rejected because the current IR has no parent identity; the emitted per-point distance and order are the public contract instead.
  - A new xtask command: rejected because the existing test can provide a guarded, deterministic regeneration path without adding dependencies to `xtask`.
  - Real Orca goldens: rejected for this packet; that is the separately blocked `TASK-163b-orca-ref` follow-up.

## Files in Scope (read + edit)

- `crates/slicer-schema/wit/deps/types.wit` - widened support point and dedicated six-field seam point.
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` - support plan records and `push-raft-plan` output seam.
- `crates/slicer-macros/src/lib.rs` - macro support-point mapping, including `dist_to_top_mm` forwarding.
- `crates/slicer-wasm-host/src/dispatch.rs` and `crates/slicer-wasm-host/src/host.rs` - support-plan harvest and output resource state.
- `crates/slicer-wasm-host/src/marshal/in_.rs`, `crates/slicer-wasm-host/src/marshal/leaf.rs`, and `crates/slicer-wasm-host/src/marshal/out.rs` - host boundary point and raft-plan marshaling.
- `crates/slicer-sdk/src/prepass_types.rs` and `crates/slicer-sdk/src/prepass_builders.rs` - SDK support types and output builder seam.
- `crates/slicer-ir/src/slice_ir.rs` - `Point3WithWidth`, `RaftPlan`, `SupportPlanIR`, and schema version 1.2.0.
- `modules/core-modules/support-planner/src/lib.rs` and `modules/core-modules/support-planner/support-planner.toml` - config parsing/defaults and support-plan emission.
- `modules/core-modules/seam-planner-default/wit-guest/Cargo.toml` - seam guest package name required by the WIT shape update.
- `crates/slicer-runtime/tests/common/mod.rs` - register the shared support-wedge helper module.
- `crates/slicer-runtime/tests/common/support_wedge.rs` - new real prepass setup and reusable IR extraction helpers.
- `crates/slicer-runtime/tests/integration/main.rs` - register both aggregate submodules.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` - current observable invariants and AC-N1/AC-N3.
- `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` - AC-7, AC-N2, parser, tolerance, and guarded regeneration.
- `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` and affected support point-construction tests - contract construction and planner regression coverage.
- `resources/golden/support_regression_wedge_branch_count.txt` - one integer captured from the post-prerequisite plan.
- `resources/golden/support_regression_wedge_endpoints.txt` - sorted first/last endpoint triples in mm.

## Read-Only Context

- `crates/slicer-runtime/src/run.rs` - `PrepassContext` and `prepare_prepass_context` only.
- `crates/slicer-runtime/src/blackboard.rs` - `support_geometry` and `support_plan` accessors only.
- `crates/slicer-ir/src/slice_ir.rs` - support IR definitions and schema constants only.
- `docs/02_ir_schemas.md` - `IR 9b - SupportPlanIR` only.
- `docs/01_system_architecture.md` - `PrePass::SupportGeometry` section only.
- `crates/slicer-runtime/tests/integration/main.rs` - full small aggregate file.
- `crates/slicer-runtime/tests/common/wasm_cache.rs` - `compiled_guest` and cache path helpers only.
- `crates/slicer-runtime/tests/common/slicer_cache.rs` - fixture path helpers only; no CLI run for IR capture.
- `.ralph/specs/117_support-planner-geometric-correctness/packet.spec.md` - metadata/status only to confirm it is implemented before capture.
- `resources/regression_wedge.stl` - existence only; do not open binary content directly.
- `docs/07_implementation_status.md` - targeted rows proving fixture, task collision, and `TASK-290` allocation.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` - no external parity read.
- `crates/slicer-runtime/src/**` - no production changes; `prepare_prepass_context` is consumed as-is.
- `target/**`, generated guest components, Cargo lockfiles, and binary fixture contents.
- Other golden resources and all other packet directories.
- `xtask/**` - no new capture command.
- All other `modules/core-modules/support-planner/` files remain out of bounds except the named manifest and test surface.

## Expected Sub-Agent Dispatches

- Question: confirm packet 117 `status` and whether `tapered_radius`/`inflate_polygon` are still old or fixed. Scope: packet-117 metadata and named support-planner symbols. Return: `FACT` <= 5 lines.
- Question: confirm the actual Cargo test target and fixture helper paths. Scope: `crates/slicer-runtime/Cargo.toml`, `tests/integration/main.rs`, `tests/common/{mod.rs,wasm_cache.rs,slicer_cache.rs}`. Return: `LOCATIONS` <= 15 entries.
- Question: run the wedge prepass through `prepare_prepass_context` with `support_enabled = true`; scope: fixture and core modules. Return: `FACT` with `entries.len`, total `branch_segments.len`, endpoint count, and `raft` negative-entry count only.
- Question: run `cargo xtask build-guests --check` immediately before capture and final tests. Scope: guest artifacts. Return: `FACT` `up to date` or `STALE: <list>`.
- Question: run the two `--test integration` module filters. Scope: invariant and golden tests. Return: `FACT` per-test pass/fail with bounded failure `SNIPPETS`.

## Data and Contract Notes

- The additive public contract is limited to `dist_to_top_mm`, `raft_plan`, the three snake_case raft config keys, and their WIT/macro/host/SDK plumbing. `SupportPlanIR` is 1.2.0.
- Branch path coordinates and widths are f32 millimetres. `SupportGeometryIR` polygon coordinates are internal scaled units; use `Point2::from_mm`/`units_to_mm` at every boundary.
- The normal WIT `point3-with-width` carries seven fields; seam candidates use the separate six-field `seam-point3-with-width` record so the seam ABI remains flattening-safe.
- The disabled-support test uses exact snake_case key `support_enabled`; raft uses exact snake_case key `support_raft_layers`.
- Layer lookup uses `SupportPlanEntry.global_layer_index` against `LayerPlanIR.global_layers[index].z`; default raft zero means no negative index needs a Z lookup.
- Collision endpoint checks treat a point inside an outer contour and outside all holes as inside the outline. Boundary points use the existing polygon predicate convention and must not be silently dropped. The sole exception is a finite endpoint within `1e-6` mm of `dist_to_top_mm == 0.0`: it is an owner-approved origin-contact tip and is exempt because the raw centroid is the required overhang contact. Every endpoint with `dist_to_top_mm > 0.0` remains subject to the unchanged outside predicate, and the test requires at least one such endpoint.
- Golden regeneration is deterministic for identical fixture, modules, config, and current guest artifacts.

## Locked Assumptions and Invariants

- `resources/regression_wedge.stl` exists at the exact path and is loaded by `slicer-model-io`.
- Enabled wedge prepass produces a non-empty `SupportPlanIR`; otherwise the packet stops before capture.
- `SupportPlanIR.entries[*].branch_segments[*].points[*]` is the public branch geometry path used by the harness, with per-point `dist_to_top_mm` available for direct checking.
- Enabled raft configuration produces one `RaftPlan` with the exact configured values; `support_raft_layers = 0` produces no plan.
- Count tolerance remains `10%`; endpoint Hausdorff tolerance remains `0.5` mm.
- Normal tests never write resources. Only the explicit `SUPPORT_WEDGE_REGEN_GOLDEN=1` path writes the two named files.
- No invariant is weakened when the current planner fails; a failure is surfaced to the appropriate support algorithm packet.

## Risks and Tradeoffs

- A stale support-planner guest can make an otherwise correct harness fail. Mitigation: freshness check immediately before capture and final verification.
- Parent identity is not part of the public IR. Mitigation: assert the emitted per-point distance directly, without inferring ancestry or a chain order.
- Collision checking against `SupportGeometryIR.entries` is an observable envelope, not private `collision_polys`. This keeps the test coupled to committed host IR and makes the assumption explicit.
- Self-capture can preserve an incorrect algorithm. Mitigation: current invariants are independent checks; external Orca replacement remains a separately blocked follow-up.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (real wedge prepass and golden capture).
- Highest-risk dispatch and required return format: wedge prepass capture -> `FACT` with counts and at most ten endpoint triples; never return the full IR.

## Resolved Closure Decisions

- The source-plan `TASK-260` collision is resolved by the free `TASK-290` allocation in `docs/07_implementation_status.md`. ADR-0048 records the re-numbering; the existing gyroid `TASK-260` row is unchanged.
- `dist_to_top_mm` is resolved as a public per-point IR/WIT field forwarded by the macro and host seams. AC-8 checks finite, non-negative values and observes at least one positive value; it does not infer parent-chain ordering. ADR-0048 records the shape and rationale.
- `SupportPlanIR.raft_plan` is resolved as an optional configuration seam emitted by `support-planner` and carried through WIT/SDK/host marshaling. AC-9 and AC-N3 cover enabled and disabled configuration; packet 124 owns geometry. ADR-0048 records the boundary.
- Golden capture is resolved: the committed self-captures were produced after the prerequisite packets and clean guest freshness check, and the normal harness remains read-only.
