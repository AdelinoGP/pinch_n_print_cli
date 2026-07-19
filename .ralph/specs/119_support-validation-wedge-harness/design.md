# Design: support-validation-wedge-harness

## Controlling Code Paths

- `crates/slicer-runtime/src/run.rs::prepare_prepass_context` is the real prepass-only driver. It returns `PrepassContext { plan, blackboard, ... }`, exposing committed `SupportPlanIR` and `SupportGeometryIR` without running per-layer or G-code stages.
- `crates/slicer-runtime/tests/common/support_wedge.rs::prepare_wedge_context` is the shared test helper. It loads `resources/regression_wedge.stl`, uses `modules/core-modules`, and fails if enabled support produces no plan.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` owns six current-observable invariant tests and the disabled-support negative test.
- `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` owns golden parsing, guarded regeneration, symmetric Hausdorff comparison, and intentional count-drift detection.
- `crates/slicer-runtime/tests/integration/main.rs` and `tests/common/mod.rs` own aggregate registration.
- `resources/golden/support_regression_wedge_branch_count.txt` and `support_regression_wedge_endpoints.txt` are committed self-capture inputs.

## Architecture Constraints

- This packet edits no guest-input path, but it dispatches the real support-planner guest through `prepare_prepass_context`. The worker must run `cargo xtask build-guests --check` immediately before capture and final verification, and rebuild if stale.
- The harness asserts the current public IR only: `SupportPlanIR.entries[*].branch_segments[*].points[*]`, `SupportPlanEntry.global_layer_index`, and `SupportGeometryIR.entries`. It does not inspect private planner state.
- Current `SupportPlanIR` carries `Vec<ExtrusionPath3D>` branch segments, and `ExtrusionPath3D.points` carries `Point3WithWidth` mm-valued points. It does not carry `dist_to_top`, parent IDs, or `raft_plan`.
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
  - Derive collision checks from committed `SupportGeometryIR.entries` and use the current support outlines as the observable collision envelope.
  - Derive overhang contacts from the loaded wedge mesh and `SupportPlanIR` entries, using the current default 45-degree threshold and `tree_support_branch_distance` tolerance.
  - Keep AC-6 as the current pre-C6 negative-index invariant. Packet 124 can replace it after an actual `raft_plan` field exists.
- Exact functions and fields:
  - `prepare_wedge_context` returns a `PrepassContext` and asserts `support_enabled` output is non-empty when enabled.
  - `SupportPlanIR.entries`, `SupportPlanEntry.global_layer_index`, `object_id`, `region_id`, `branch_segments`, `ExtrusionPath3D.points`, and `Point3WithWidth.{x,y,z,width}`.
  - `Blackboard::support_geometry`, `SupportGeometryIR.entries`, `SupportGeometryKey`, `ExPolygon.contour`, and `ExPolygon.holes`.
  - `LayerPlanIR.global_layers` and `GlobalLayer.{index,z}` for layer-Z matching.
  - Golden helpers `branch_segment_count`, `branch_endpoints`, `parse_branch_count`, `parse_endpoints`, and `symmetric_hausdorff`.
- Rejected alternatives:
  - `run_slice`: rejected because it exposes only final G-code and cannot prove `SupportPlanIR` invariants.
  - A new production introspection API: rejected; the current blackboard already exposes the committed IR through `PrepassContext`.
  - Reconstructing parent links from shared endpoints: rejected because the current IR has no parent identity and the heuristic would make `dist_to_top` claims non-falsifiable.
  - A new xtask command: rejected because the existing test can provide a guarded, deterministic regeneration path without adding dependencies to `xtask`.
  - Real Orca goldens: rejected for this packet; that is the separately blocked `TASK-163b-orca-ref` follow-up.

## Files in Scope (read + edit)

- `crates/slicer-runtime/tests/common/mod.rs` - register the shared support-wedge helper module.
- `crates/slicer-runtime/tests/common/support_wedge.rs` - new real prepass setup and reusable IR extraction helpers.
- `crates/slicer-runtime/tests/integration/main.rs` - register both aggregate submodules.
- `crates/slicer-runtime/tests/integration/support_invariants_wedge_tdd.rs` - six current observable invariants and AC-N1.
- `crates/slicer-runtime/tests/integration/support_golden_regression_wedge_tdd.rs` - AC-7, AC-N2, parser, tolerance, and guarded regeneration.
- `resources/golden/support_regression_wedge_branch_count.txt` - one integer captured from the post-prerequisite plan.
- `resources/golden/support_regression_wedge_endpoints.txt` - sorted first/last endpoint triples in mm.

## Read-Only Context

- `crates/slicer-runtime/src/run.rs` - `PrepassContext` and `prepare_prepass_context` only.
- `crates/slicer-runtime/src/blackboard.rs` - `support_geometry` and `support_plan` accessors only.
- `crates/slicer-ir/src/slice_ir.rs` - `SupportPlanIR`, `SupportPlanEntry`, `ExtrusionPath3D`, `Point3WithWidth`, and `SupportGeometryIR` definitions only.
- `docs/02_ir_schemas.md` - `IR 9b - SupportPlanIR` only.
- `docs/01_system_architecture.md` - `PrePass::SupportGeometry` section only.
- `crates/slicer-runtime/tests/integration/main.rs` - full small aggregate file.
- `crates/slicer-runtime/tests/common/wasm_cache.rs` - `compiled_guest` and cache path helpers only.
- `crates/slicer-runtime/tests/common/slicer_cache.rs` - fixture path helpers only; no CLI run for IR capture.
- `.ralph/specs/117_support-planner-geometric-correctness/packet.spec.md` - metadata/status only to confirm it is implemented before capture.
- `resources/regression_wedge.stl` - existence only; do not open binary content directly.
- `docs/07_implementation_status.md` - targeted rows proving fixture and task collision.

## Out-of-Bounds Files

- `modules/core-modules/support-planner/src/lib.rs` - no production or private-state introspection.
- `OrcaSlicerDocumented/**` - no external parity read.
- `crates/slicer-runtime/src/**` - no production changes; `prepare_prepass_context` is consumed as-is.
- `target/**`, generated guest components, Cargo lockfiles, and binary fixture contents.
- Other golden resources and all other packet directories.
- `xtask/**` - no new capture command.

## Expected Sub-Agent Dispatches

- Question: confirm packet 117 `status` and whether `tapered_radius`/`inflate_polygon` are still old or fixed. Scope: packet-117 metadata and named support-planner symbols. Return: `FACT` <= 5 lines.
- Question: confirm the actual Cargo test target and fixture helper paths. Scope: `crates/slicer-runtime/Cargo.toml`, `tests/integration/main.rs`, `tests/common/{mod.rs,wasm_cache.rs,slicer_cache.rs}`. Return: `LOCATIONS` <= 15 entries.
- Question: run the wedge prepass through `prepare_prepass_context` with `support_enabled = true`; scope: fixture and core modules. Return: `FACT` with `entries.len`, total `branch_segments.len`, endpoint count, and `raft` negative-entry count only.
- Question: run `cargo xtask build-guests --check` immediately before capture and final tests. Scope: guest artifacts. Return: `FACT` `up to date` or `STALE: <list>`.
- Question: run the two `--test integration` module filters. Scope: invariant and golden tests. Return: `FACT` per-test pass/fail with bounded failure `SNIPPETS`.

## Data and Contract Notes

- No IR or WIT contract changes are made.
- Branch path coordinates and widths are f32 millimetres. `SupportGeometryIR` polygon coordinates are internal scaled units; use `Point2::from_mm`/`units_to_mm` at every boundary.
- The disabled-support test uses exact snake_case key `support_enabled`; raft uses exact snake_case key `support_raft_layers`.
- Layer lookup uses `SupportPlanEntry.global_layer_index` against `LayerPlanIR.global_layers[index].z`; default raft zero means no negative index needs a Z lookup.
- Collision endpoint checks treat a point inside an outer contour and outside all holes as inside the outline. Boundary points use the existing polygon predicate convention and must not be silently dropped.
- Golden regeneration is deterministic for identical fixture, modules, config, and current guest artifacts.

## Locked Assumptions and Invariants

- `resources/regression_wedge.stl` exists at the exact path and is loaded by `slicer-model-io`.
- Enabled wedge prepass produces a non-empty `SupportPlanIR`; otherwise the packet stops before capture.
- `SupportPlanIR.entries[*].branch_segments[*].points[*]` is the only public branch geometry path used by the harness.
- Count tolerance remains `10%`; endpoint Hausdorff tolerance remains `0.5` mm.
- Normal tests never write resources. Only the explicit `SUPPORT_WEDGE_REGEN_GOLDEN=1` path writes the two named files.
- No invariant is weakened when the current planner fails; a failure is surfaced to the appropriate support algorithm packet.

## Risks and Tradeoffs

- A stale support-planner guest can make an otherwise correct harness fail. Mitigation: freshness check immediately before capture and final verification.
- The source plan's hidden-state invariants cannot be tested from current IR. Mitigation: keep them as activation blockers, not approximate them with inferred parent links.
- Collision checking against `SupportGeometryIR.entries` is an observable envelope, not private `collision_polys`. This keeps the test coupled to committed host IR and makes the assumption explicit.
- Self-capture can preserve an incorrect algorithm. Mitigation: current invariants are independent checks; external Orca replacement remains a separately blocked follow-up.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (real wedge prepass and golden capture).
- Highest-risk dispatch and required return format: wedge prepass capture -> `FACT` with counts and at most ten endpoint triples; never return the full IR.

## Open Questions

- `[BLOCK]` Source-plan `TASK-260` is a current gyroid-infill task in `docs/07_implementation_status.md`; no support-owned backlog ID exists for this C1 slice. A maintainer must map it before closure; no replacement ID is invented.
- `[BLOCK]` Source C1 invariant 3 requires `PlannedSupportNode.dist_to_top` and parent/child links, but current `SupportPlanIR` exposes neither. Decide whether a future public/test-only contract is required; this packet does not fake one.
- `[BLOCK]` Source C1 invariant 6 requires `SupportPlanIR.raft_plan`, but current `SupportPlanIR` contains only `schema_version` and `entries`. Packet 124 owns the proposed field; until then AC-6 is only the current negative-index invariant.
- `[BLOCK]` Goldens cannot be captured until packet 117 is implemented and the enabled wedge prepass proves a non-empty plan. The authoring refresh has not run Cargo commands, so this remains an implementation-worker gate.
