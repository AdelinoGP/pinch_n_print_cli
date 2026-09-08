# Implementation Plan: mixed-support-family-routing

## Execution Rules
- Work one atomic step at a time; map every step to `TASK-334`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Do not implement tree or traditional algorithms in this packet.

## Steps
### Step 1: Register mixed-family test target and red contracts
- Task IDs: `TASK-334`
- Objective: add the planned `support_family_routing` Cargo target and tests for routing ownership, diagnostics, and overlap cases.
- Precondition: confirm integration aggregator shape and no existing target with this name.
- Postcondition: tests compile and fail only on missing routing behavior, with `mod support_family_routing;` registered.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/Cargo.toml` - test-target section.
  - `crates/slicer-runtime/tests/integration/main.rs` - module declarations.
- Files allowed to edit (at most 3): `crates/slicer-runtime/Cargo.toml`; `crates/slicer-runtime/tests/integration/main.rs`; `crates/slicer-runtime/tests/integration/support_family_routing.rs`.
- Files explicitly out of bounds: source implementation, family modules, generated bindings, target, Orca source.
- Expected sub-agent dispatches: Question: verify target wiring; scope: the two existing files; return: `LOCATIONS`.
- Context cost: `S`
- Authoritative docs: plan §§7-9; delegated SUMMARY for scheduler contract.
- OrcaSlicer refs: delegate only, paths listed in `requirements.md`.
- Verification: `cargo test -p slicer-runtime --test support_family_routing -- --exact` - FACT pass/fail.
- Exit condition: target is registered and every planned test name is discoverable.

### Step 2: Implement routing cells and host plan attribution
- Task IDs: `TASK-334`
- Objective: partition baseline feasible space deterministically, dispatch each selected family once, union same-family cells, and reject attribution mismatches.
- Precondition: Step 1 target exists; TASK-331 structural fields are available as a forward dependency.
- Postcondition: every retained entry belongs to its assigned family/cell and preserves all demand/body identities.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/prepass.rs` - support dispatch seam.
  - `crates/slicer-runtime/src/blackboard.rs` lines 190-225 - plan ownership.
- Files allowed to edit (at most 3): host support dispatch file; host aggregation file; `support_family_routing.rs`.
- Files explicitly out of bounds: family planner algorithms, WIT schema, unresolved exact-Z service definition.
- Expected sub-agent dispatches: Question: locate exact host seams; scope: runtime/core support sources; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan §§3, 7, 8.
- OrcaSlicer refs: delegate TreeSupport.cpp and SupportMaterial.cpp locations.
- Verification: `cargo test -p slicer-runtime --test support_family_routing -- routing_cells -- --exact`; `cargo test -p slicer-runtime --test support_family_routing -- family_attribution -- --exact`.
- Exit condition: deterministic serial/parallel routing tests pass and mismatched family entries fail before rendering.

### Step 3: Add complete-body and rendered swept-path degradation
- Task IDs: `TASK-334`
- Objective: validate exact-Z occupancy/routing containment, reject cross-family body and nozzle-sweep overlap, and serialize decline/unmet diagnostics.
- Precondition: Step 2 retains attributed bodies and demand IDs.
- Postcondition: invalid complete bodies and conflicting rendered paths are removed without fallback; diagnostics expose exact reason variants.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/layer_executor.rs` lines 1500-1565, 1810-1845 - support commit ordering.
  - `crates/slicer-runtime/src/visual_debug_render.rs` lines 831-850 - support path extraction.
- Files allowed to edit (at most 3): host validation/commit file; diagnostics file; `support_family_routing.rs`.
- Files explicitly out of bounds: family renderers, visual-debug renderer internals, generated WIT bindings.
- Expected sub-agent dispatches: Question: locate body-drop and unmet-demand mechanisms; scope: runtime/core; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan §§8-9 and invariants 1-7.
- OrcaSlicer refs: delegate listed support files.
- Verification: `cargo test -p slicer-runtime --test support_family_routing -- cross_family_body_overlap -- --exact`; `cargo test -p slicer-runtime --test support_family_routing -- swept_path_overlap -- --exact`; `cargo test -p slicer-runtime --test support_family_routing -- degraded_diagnostics -- --exact`.
- Exit condition: both complete-body and swept-path conflicts mark demands unmet with no fallback paths.

## Per-Step Budget Roll-Up
| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | target wiring and red tests |
| Step 2 | M | host routing and attribution |
| Step 3 | M | geometry and rendered conflict validation |

## Packet Completion Gate
- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `packet.spec.md` is ready for `status: implemented` after inherited blockers close.

## Acceptance Ceremony
- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk and degraded-demand diagnostics.
