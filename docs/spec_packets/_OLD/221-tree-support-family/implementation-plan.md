# Implementation Plan: tree-support-family

## Execution Rules
- Work one atomic step at a time; map every step to `TASK-332`.
- Use TDD, implementation, then the narrowest falsifying validation.
- TASK-331 blockers closed; TASK-331 implemented (packet 220). Activation pending only this packet's own preflight.

## Steps
### Step 1: Establish the tree family package and claims
- Task IDs: `TASK-332`
- Objective: create the `tree-support-planner` identity and pair it with `tree-support` using `support-family:tree`.
- Precondition: live manifest claims are inventoried at `support-planner.toml:2-17` and `tree-support.toml:2-18`.
- Postcondition: loader sees one tree planner/renderer pair and retains no generic global winner.
- Files allowed to read: the two manifests; scheduler claim validation locations delegated.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support-planner/Cargo.toml`; `modules/core-modules/tree-support-planner/tree-support-planner.toml`; `modules/core-modules/tree-support/tree-support.toml`.
- Files explicitly out of bounds: algorithms, generated WIT, packet 213, `docs/07_implementation_status.md`.
- Expected dispatch: locate claim-pair validation and manifest fixture aggregator; scope `crates/slicer-scheduler`; return `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: plan §6, delegated `docs/03_wit_and_manifest.md` summary.
- OrcaSlicer refs: none.
- Verification: `cargo test -p slicer-scheduler --test scheduler_integration support_family_selection -- --exact`
- Exit condition: `support-family:tree` and both role claims are present with no global fallback selection.

### Step 2: Implement distributed tree planning and exact-Z geometry
- Task IDs: `TASK-332`
- Objective: turn assigned candidates into distributed contacts, radius-aware routed bodies, interfaces, and terminations.
- Precondition: satisfied — TASK-331 exports `SupportAnalysisIR`, and the host exact-Z support query service is `ExactZQueryService` (`crates/slicer-wasm-host/src/exact_z_query.rs`) with its final shape.
- Postcondition: complete structural entries are collision-safe at every body Z and preserve demand/body identity.
- Files allowed to read: `modules/core-modules/support-planner/src/lib.rs` bounded algorithm ranges; delegated exact-Z API.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support-planner/src/lib.rs`; `modules/core-modules/tree-support-planner/Cargo.toml`; `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`.
- Files explicitly out of bounds: host exact-Z implementation, traditional planner, generated bindings.
- Expected dispatch: enumerate `region.polygons`, `overhang_areas`, contact and collision consumers; scope support planner and SDK; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan §§4, 5, 10; delegated TreeSupport locations.
- OrcaSlicer refs: delegated `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388` (`TreeSupport::generate_contact_points()`), `:1839` (`get_collision`), `:1823` (`get_avoidance`), `:1855` (`get_collision_polys`), `:2652` (`drop_nodes`), `:1969` (`draw_circles`), `:2050` (`roof_areas` inside `draw_circles`), `:1772`/`:1792` (`calc_branch_radius`), and `:2143` (top-interface/termination).
- Verification: `cargo test -p tree-support-planner --test tree_family_tdd distributed_contacts -- --exact`; `cargo test -p tree-support-planner --test tree_family_tdd radius_aware_collision -- --exact`
- Test-target wiring: add `[[test]] name = "tree_family_tdd" path = "tests/tree_family_tdd.rs"` in the planner manifest; the new binary is the driver for AC-1, AC-2, AC-3, AC-6, and AC-N1.
- Exit condition: distributed contacts and full-radius body validation reject the prior oversized pillar case.

### Step 3: Migrate structural plan output and renderer
- Task IDs: `TASK-332`
- Objective: emit semantic polygons/skeletons and render only tree-attributed entries into structured `SupportIR`.
- Precondition: TASK-331 structural fields and anchored event hooks are available.
- Postcondition: tree output has printable walls/fill, roles, attribution, and anchored support heights.
- Files allowed to read: tree renderer source/tests; TASK-331 exported shape summary; Layer::Support WIT locations delegated.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support/src/lib.rs`; `modules/core-modules/tree-support/Cargo.toml`; `modules/core-modules/tree-support/tests/tree_family_tdd.rs`.
- Files explicitly out of bounds: traditional renderer, host aggregator, schema decision, generated bindings.
- Blast-radius discipline: inventory every tree `SupportPlanIR` and `SupportIR` literal plus flat-path assertion before edits; update all tree literals/assertions in this step.
- Expected dispatch: locate tree plan consumers and integration target wiring; scope tree module and runtime tests; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan §§2, 5, 9; delegated `docs/02_ir_schemas.md` summary.
- OrcaSlicer refs: delegated `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp:3388` (`TreeSupport::generate_contact_points()`), `:1839` (`get_collision`), `:1823` (`get_avoidance`), `:1855` (`get_collision_polys`), `:2652` (`drop_nodes`), `:1969` (`draw_circles`), `:2050` (`roof_areas` inside `draw_circles`), `:1772`/`:1792` (`calc_branch_radius`), and `:2143` (top-interface/termination).
- Verification: `cargo test -p tree-support --test tree_family_tdd polygon_renderer_identity -- --exact`; `cargo xtask build-guests --check`
- Test-target wiring: add `[[test]] name = "tree_family_tdd" path = "tests/tree_family_tdd.rs"` in the renderer manifest; register the module's existing test helpers in that file.
- Exit condition: trunk diameter is not encoded as one extrusion width and identity survives renderer handoff.

### Step 4: Enforce decline, invalid-body, and disabled-support behavior
- Task IDs: `TASK-332`
- Objective: prove tree-only attribution, structured decline, atomic invalid-body rejection, and no fallback filler.
- Precondition: Steps 1-3 compile against TASK-331 draft shapes.
- Postcondition: all tree rejection paths are observable and degraded rather than silently substituted.
- Files allowed to read: tree tests, support-disabled runtime fixture, validator contract locations delegated.
- Files allowed to edit (at most 3): `modules/core-modules/tree-support-planner/tests/tree_family_tdd.rs`; `modules/core-modules/tree-support/tests/tree_family_tdd.rs`.
- Files explicitly out of bounds: mixed-family routing, packet 213, status ledger, runtime integration wiring (Step 5).
- Expected dispatch: locate validator contract locations delegated; return `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan invariants 1, 3, 5, 13, 14.
- OrcaSlicer refs: none.
- Verification: `cargo test -p tree-support-planner --test tree_family_tdd invalid_body_rejected -- --exact`; `cargo test -p tree-support --test tree_family_tdd mismatched_family_rejected -- --exact`
- Exit condition: no invalid or disabled tree input produces fallback support paths.

### Step 5: Register runtime integration coverage
- Task IDs: `TASK-332`
- Objective: register the runtime integration binary and mount the tree-family end-to-end test module.
- Precondition: Step 4 rejection behavior is covered by the planner and renderer tests.
- Postcondition: `cargo test -p slicer-runtime --test integration` discovers `tree_support_family` through the real aggregator.
- Files allowed to read: `crates/slicer-runtime/tests/integration/main.rs`; `crates/slicer-runtime/Cargo.toml`; existing runtime integration helpers.
- Files allowed to edit (at most 3): `crates/slicer-runtime/tests/integration/main.rs`; `crates/slicer-runtime/tests/integration/tree_support_family.rs`.
- Files explicitly out of bounds: mixed-family routing, packet 213, status ledger.
- Expected dispatch: preserve the aggregator's submodule pattern and add `mod tree_support_family;`; return `LOCATIONS`.
- Context cost: `S`
- Verification: `cargo test -p slicer-runtime --test integration`
- Test-target wiring: confirm the `integration` Cargo target registered by TASK-330 (packet 219 Step 0) exists and mount `tree_support_family` in the aggregator.
- Exit condition: the planned runtime test is registered in the existing aggregator and the `integration` Cargo target is explicit.

## Per-Step Budget Roll-Up
| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | claims |
| Step 2 | M | planner geometry |
| Step 3 | M | structural renderer migration |
| Step 4 | M | rejection/e2e evidence |
| Step 5 | S | runtime integration registration |

## Packet Completion Gate
- All steps and exits complete; every AC command passes.
- TASK-331 blockers are resolved before status changes from draft.
- Guest WASM freshness and model-backed tree visual taps are inspected downstream.

## Acceptance Ceremony
- Re-dispatch all AC commands and inspect tree support at matched heights for `tmp/SupportTest.stl`.
