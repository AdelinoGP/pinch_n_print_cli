# Implementation Plan: support-planner-geometric-correctness

## Execution Rules

- Work one atomic step at a time; map steps to source-plan B5 or B6 because no current canonical task ID is available.
- Use focused tests before each implementation edit where the current API permits it, then run the narrowest falsifying check.
- Do not close or activate this packet while the B5/B6 backlog crosswalk remains `[BLOCK]`.

## Steps

### Step 1: Re-ground the formula, API, dependency, and existing oracles

- Task IDs: TASK-281, TASK-282; source-plan B5 and B6.
- Objective: confirm the current `tapered_radius` body, `run_support_geometry` offset site, cache/helper types, the existing SDK `offset_polygons` signature and host-service contract, unchanged planner dependency list, old radius test, and raw-coordinate fixture.
- Precondition: source authority and current tree are available.
- Postcondition: implementer has no stale line-pinned API or test assumptions; B5/B6 mapping blocker is recorded without assigning an ID.
- Files allowed to read, with ranges when over 300 lines:
  - `docs/specs/support-modules-orca-port.md` - §B5/B6/D2 only.
  - `docs/08_coordinate_system.md` - coordinate rule/conversion and Clipper2 integration sections only.
  - `docs/05_module_sdk.md` - guest dependency rules and host-geometry seam only.
  - `docs/adr/0023-arachne-port-strategy.md` - current host-side crate strategy only.
  - `crates/slicer-sdk/src/host.rs` - `OffsetJoinType` and `offset_polygons` only.
  - `crates/slicer-schema/wit/deps/common.wit` - existing `offset-polygons` signature only.
  - `modules/core-modules/support-planner/src/lib.rs` - `tapered_radius`, `run_support_geometry`, `LayerCollisionCache`, containment/clamping helpers, and test module only.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - `radius_tapers_with_distance_to_top` and `node_dropped_when_avoidance_rejects_all_moves` only.
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**` - delegate only.
  - `docs/07_implementation_status.md` - bounded survey only; never edit.
  - All other packets and implementation surfaces not named above.
- Expected sub-agent dispatches:
  - Question: Confirm Orca's second `calc_branch_radius` overload; scope: named Orca file/function; return: `SUMMARY`; purpose: formula authority.
  - Question: Confirm current SDK offset signature, hole behavior, and guest boundary; scope: named SDK/WIT excerpts; return: `SNIPPETS`; purpose: API authority without adding a host-only dependency.
  - Question: Confirm the unchanged planner dependency list and existing test symbols; scope: named test excerpt only; return: `FACT` plus bounded `SNIPPETS`; purpose: blast-radius inventory.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` - §B5/B6/D2 direct read.
  - `docs/08_coordinate_system.md` - direct bounded read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::calc_branch_radius` - delegate; never load.
- Verification:
  - Inventory returns exact current symbols, API shape, unchanged dependency boundary, and old-test expectations as bounded facts.
- Exit condition: all implementation and test surfaces are enumerated; no stale source-plan line number is used.

### Step 2: Add radius oracles and migrate the obsolete direct test

- Task IDs: TASK-281; source-plan B5.
- Objective: add the five named radius tests to the existing source unit-test module and update `radius_tapers_with_distance_to_top` so it no longer asserts the branch-radius floor.
- Precondition: Step 1 confirms the public `tapered_radius` signature and the old test's direct assertions.
- Postcondition: the new tests compile; at least the tip, inside-cone, and no-floor assertions are RED against the current implementation; the migrated existing test expresses the intended tip semantics.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - `tapered_radius` and existing `#[cfg(test)] mod tests` only.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - `radius_tapers_with_distance_to_top` only.
  - `docs/specs/support-modules-orca-port.md` - §B5 only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`
- Files explicitly out of bounds:
  - `tapered_radius` production body - Step 3 owns it.
  - B6 cache and offset call - Steps 4-5.
- Expected sub-agent dispatches:
  - Question: Run the five radius filters and the migrated existing test; scope: support-planner tests; return: `FACT` with bounded failure snippets; purpose: establish the RED radius oracle.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` - §B5 direct read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::calc_branch_radius` - delegate; formula already bounded by Step 1.
- Verification:
  - `cargo test -p support-planner --all-targets -- tapered_radius_at_tip_is_zero --nocapture 2>&1 | tee target/test-output.log` - FACT expected assertion failure, not compile failure.
  - `cargo test -p support-planner --test orca_parity_tdd --all-targets -- radius_tapers_with_distance_to_top --nocapture 2>&1 | tee target/test-output.log` - FACT old oracle now fails only until Step 3.
- Exit condition: tests compile and identify the old floor behavior; no implementation workaround is added to make RED pass.

### Step 3: Implement the two-piece tip-cone formula

- Task IDs: TASK-281; source-plan B5.
- Objective: replace `tapered_radius`'s body and function documentation with the mm-to-top tip-cone/linear-above-cone formula and final `[0.0, MAX_BRANCH_RADIUS_MM]` clamp.
- Precondition: Step 2 radius tests are RED and the existing direct oracle has been migrated.
- Postcondition: AC-1 through AC-4, AC-N1, and AC-8 pass; the public signature and `MAX_BRANCH_RADIUS_MM` remain unchanged.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - `tapered_radius` only.
  - `docs/specs/support-modules-orca-port.md` - §B5 only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - B6 cache, `inflate_polygon`, and coordinate fixture.
  - `OrcaSlicerDocumented/**` - delegate only.
- Expected sub-agent dispatches:
  - Question: Run all B5 radius filters plus the migrated direct test and planner build; scope: commands in `requirements.md`; return: `FACT`; purpose: RED-to-GREEN gate.
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` - §B5 direct read.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp::calc_branch_radius` - delegated formula check.
- Verification:
  - `cargo test -p support-planner --all-targets -- tapered_radius_no_longer_floors_at_branch_radius --nocapture 2>&1 | tee target/test-output.log` - FACT pass.
  - `cargo test -p support-planner --test orca_parity_tdd --all-targets -- radius_tapers_with_distance_to_top --nocapture 2>&1 | tee target/test-output.log` - FACT pass.
- Exit condition: `tapered_radius(0)` is zero, the inside-cone value is `mm_to_top`, the above-cone value is linear, and the upper clamp still fires.

### Step 4: Add geometry oracles through the SDK seam

- Task IDs: TASK-282; source-plan B6.
- Objective: add focused unit tests for concave simplicity, hole preservation, and the mm/scaled-unit boundary using the existing `slicer_sdk::host::offset_polygons` API; do not edit the guest manifest.
- Precondition: Step 1 confirmed the SDK seam returns `Vec<ExPolygon>` and the support-planner dependency graph must remain unchanged.
- Postcondition: the geometry tests compile against the guest-compatible API and assert a test-local non-self-intersection invariant, hole count/area, and a 2.0 mm span for a 1.0 mm square inflated by 0.5 mm.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-sdk/src/host.rs` - `OffsetJoinType` and `offset_polygons` only.
  - `crates/slicer-schema/wit/deps/common.wit` - `offset-polygons` only.
  - `crates/slicer-ir/src/slice_ir.rs` - `Point2::from_mm`, `units_to_mm`, `ExPolygon` definitions only.
  - `modules/core-modules/support-planner/src/lib.rs` - existing test module only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/support-planner/Cargo.toml` - no dependency edit is permitted.
  - `run_support_geometry` and cache production code - Step 5.
- Expected sub-agent dispatches:
  - Question: Run the three geometry oracle filters and the direct-manifest static check; scope: support-planner all targets; return: `FACT` with bounded failure snippets; purpose: validate the existing SDK seam before wiring it into the planner.
- Context cost: `M`
- Authoritative docs:
  - `docs/08_coordinate_system.md` - bounded conversion read.
  - `crates/slicer-sdk/src/host.rs` - named SDK API read.
- OrcaSlicer refs: none additional; B6 uses the existing SDK host-geometry seam.
- Verification:
  - `cargo test -p support-planner --all-targets -- offset_concave_l_shape_no_self_intersection --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail with bounded geometry assertion.
  - `cargo test -p support-planner --all-targets -- offset_polygon_with_hole_preserves_hole --nocapture 2>&1 | tee target/test-output.log` - FACT pass/fail with bounded geometry assertion.
- Exit condition: the existing SDK API compiles; test oracles do not use a fabricated `JoinType`, miter-limit parameter, direct `slicer-core` dependency, or flattened polygon type.

### Step 5: Replace the DIY avoidance path and repair coordinate boundaries

- Task IDs: TASK-282; source-plan B6.
- Objective: delete `inflate_polygon`, call `slicer_sdk::host::offset_polygons` over each complete support outline, preserve holes in `LayerCollisionCache`, update containment/clamping/scan-line consumers, and migrate the existing raw-coordinate fixture.
- Precondition: Step 4 SDK geometry oracles are present; Step 1 inventory identifies all consumers of `LayerCollisionCache` and its helper signatures.
- Postcondition: AC-5 through AC-7 and AC-N2 pass; one production offset call remains at the existing avoidance-cache site; `node_dropped_when_avoidance_rejects_all_moves` uses canonical mm-to-unit fixture construction and still passes its intended diagnostic assertion.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - `run_support_geometry`, `LayerCollisionCache`, `point_in_any_polygon`, `clamp_to_avoidance`, `push_interface_scan_lines`, and `inflate_polygon` only.
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - `node_dropped_when_avoidance_rejects_all_moves` only.
  - `crates/slicer-ir/src/slice_ir.rs` - conversion helpers only.
  - `crates/slicer-sdk/src/host.rs` - `offset_polygons` signature only.
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`
- Files explicitly out of bounds:
  - All other support-planner helpers and tests unless compilation proves a named cache consumer must change.
  - `crates/slicer-core/**`, WIT, and host runtime; the existing SDK seam is consumed as-is.
- Expected sub-agent dispatches:
  - Question: Run the static AC-5 compound check, concave/hole/unit filters, the existing node-clamp test, and planner build; scope: commands in `requirements.md`; return: `FACT` PASS/FAIL with at most 20 failure lines; purpose: wire-path gate.
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-modules-orca-port.md` - §B6 direct read.
  - `docs/08_coordinate_system.md` - direct conversion rule.
- OrcaSlicer refs: none additional; the local helper's documented Clipper behavior is the implementation authority.
- Verification:
  - `! rg -q 'fn inflate_polygon|inflate_polygon\(' modules/core-modules/support-planner/src/lib.rs && rg -q 'slicer_sdk::host::offset_polygons' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'slicer-core' modules/core-modules/support-planner/Cargo.toml` - FACT pass.
  - `cargo test -p support-planner --test orca_parity_tdd --all-targets -- node_dropped_when_avoidance_rejects_all_moves --nocapture 2>&1 | tee target/test-output.log` - FACT pass.
- Exit condition: the old helper and all call sites are absent; complete ExPolygon offsets and canonical coordinate comparisons drive the existing avoidance path.

### Step 6: Run planner gates and guest freshness check

- Task IDs: TASK-281, TASK-282; source-plan B5 and B6.
- Objective: run the full targeted planner matrix, including guest freshness, and preserve draft status pending backlog mapping.
- Precondition: Steps 2-5 pass their exits.
- Postcondition: every packet AC and gate command returns pass; guest artifacts are clean after any required rebuild; no backlog row is changed.
- Files allowed to read: none beyond prior step outputs.
- Files allowed to edit (at most 3): none.
- Files explicitly out of bounds:
  - `target/**`, generated WASM, `docs/07_implementation_status.md`, and every other packet directory.
- Expected sub-agent dispatches:
  - Question: Run the full `requirements.md` matrix and freshness check; scope: commands only; return: `FACT` PASS/FAIL list with bounded failure snippets; purpose: closure evidence.
- Context cost: `S`
- Authoritative docs: none additional.
- OrcaSlicer refs: none.
- Verification:
  - `cargo check -p support-planner --all-targets` - FACT pass/fail.
  - `cargo clippy -p support-planner --all-targets -- -D warnings` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT `up to date` or stale/rebuild/recheck result.
- Exit condition: implementation evidence is green; packet remains `draft` until the canonical B5/B6 mapping is supplied.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | Bounded formula/API/ledger survey. |
| Step 2 | S | Radius tests and one existing oracle migration. |
| Step 3 | S | Pure formula implementation. |
| Step 4 | M | SDK seam plus ExPolygon geometry oracles. |
| Step 5 | M | Cache-shape, offset-path, and coordinate-boundary repair. |
| Step 6 | S | Targeted gates and guest freshness. |

Aggregate: `M`. No step is L; no step exceeds `M`.

## Packet Completion Gate

- All steps and exits pass.
- Every pipe-suffixed AC command returns PASS.
- `cargo xtask build-guests --check` is clean after any required rebuild.
- A maintainer supplies non-colliding canonical backlog rows for B5 and B6; until then, do not change `packet.spec.md` from `draft`.
- Packet 119 captures new self-capture validation after the intentional radius/avoidance output change.

## Acceptance Ceremony

- Re-dispatch every AC command and packet-level gate.
- Re-derive the B5/B6 backlog crosswalk at ceremony time; do not rely on this packet's ledger snapshot.
- Confirm the existing radius oracle no longer encodes the old floor and the node-clamp fixture uses canonical coordinate units.
