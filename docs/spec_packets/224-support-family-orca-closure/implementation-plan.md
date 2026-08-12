# Implementation Plan: support-family-orca-closure

## Execution Rules
- Work one atomic step at a time; map every step to `TASK-335`.
- Use TDD, then implementation, then narrow falsifying validation.
- Never claim parity from uninspected or self-captured goldens.

## Steps
### Step 1: Establish fixture and closure target
- Task IDs: `TASK-335`
- Objective: verify the existing model and Orca references, create a dual-family visual-debug request, and register the closure test target.
- Precondition: `tmp/SupportTest.stl`, `tmp/SupportTest_Tree_Orca.gcode`, `tmp/SupportTest_Normal_Orca.gcode`, `target/vd-orca-tree-compare`, and `target/vd-orca-normal-compare` exist.
- Postcondition: the primary fixture-driven closure path is runnable and the request covers both families plus analysis/routing taps.
- Files allowed to read, with ranges when over 300 lines: `tmp/**`; `crates/slicer-runtime/Cargo.toml`; integration aggregator.
- Files allowed to edit (at most 3): `crates/slicer-runtime/Cargo.toml`; `crates/slicer-runtime/tests/integration/main.rs`; `crates/slicer-runtime/tests/integration/support_family_closure.rs`. Register `[[test]] name = "integration" path = "tests/integration/main.rs"`, its integration `mod`, and the closure tests in this single step.
- Files explicitly out of bounds: Orca source, target, family implementations.
- Expected sub-agent dispatches: Question: verify/provide fixture provenance; scope: `tmp/**`; return: `FACT`.
- Context cost: `S`
- Authoritative docs: plan §§Visual And Differential Gates and Supersession.
- OrcaSlicer refs: delegate all listed paths.
- Verification: `ls tmp/SupportTest* && ls -d target/vd-orca-tree-compare target/vd-orca-normal-compare`; then `cargo test -p slicer-runtime --test integration support_family_closure -- missing_fixture_is_blocking -- --exact` - FACT pass/fail; confirm Cargo resolves the `integration` target from `tests/integration/main.rs`.
- Exit condition: test target is registered and fixture status is explicit.

### Step 2: Implement real-fixture invariants
- Task IDs: `TASK-335`
- Objective: run both family selections through the real pipeline and assert termination, exact-Z collision freedom, demand/body attribution, disabled support, routing, and serial/parallel determinism.
- Precondition: Step 1 confirms the existing decisive fixtures and registered test target.
- Postcondition: closure test fails on invalid geometry, missing termination, overlap, or fallback output.
- Files allowed to read, with ranges when over 300 lines: existing integration support tests; runtime pipeline and visual-debug request helpers via bounded dispatch.
- Files allowed to edit (at most 3): `support_family_closure.rs`; one fixture request/config file; aggregator if needed.
- Files explicitly out of bounds: planner algorithms, Orca source, target artifacts.
- Expected sub-agent dispatches: Question: locate real-slice driver and support invariant helpers; scope: runtime integration tests; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs: plan invariants 1-14.
- OrcaSlicer refs: delegate TreeSupport and SupportMaterial locations.
- Verification: `cargo test -p slicer-runtime --test integration support_family_closure -- fixture_invariants -- --exact`; `cargo test -p slicer-runtime --test integration support_family_closure -- invalid_geometry_fails -- --exact`.
- Exit condition: all listed invariants are asserted against real family output.

### Step 3: Generate and inspect visual/differential evidence
- Task IDs: `TASK-335`
- Objective: render both families and host analysis/routing, plan, support, and final-G-code views at matched heights, inspect PNGs, and record packet 213/TASK-329/TASK-163b disposition.
- Precondition: Step 2 produces valid family output and Step 1 confirms the existing references.
- Postcondition: evidence records source/tap/layer/provenance and inspection findings without exact-path claims.
- Files allowed to read, with ranges when over 300 lines: `crates/pnp-cli/src/visual_debug.rs` lines 743-761, 1500-1680; existing visual-debug tests; generated manifest only through delegated inspection.
- Files allowed to edit (at most 3): `tmp/visual-debug-support-family.json`; closure test; closure documentation section.
- Files explicitly out of bounds: Orca source, packet 213, generated target PNGs as source edits.
- Expected sub-agent dispatches: Question: inspect matched-height manifests and PNGs; scope: `target/vd-support-family-*`; return: `FACT`.
- Context cost: `M`
- Authoritative docs: plan §§Visual And Differential Gates and Supersession.
- OrcaSlicer refs: delegate all listed locations; preserve returned paths verbatim.
- Verification: `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support-family.json --output target/vd-support-family-tree --overwrite`; repeat for `target/vd-support-family-normal`; inspect matched-height PNGs; run the role, differential, supersession, and TASK-163b disposition tests.
- Exit condition: inspected evidence supports only the approved behavioral parity claims and records any external blocker.

## Per-Step Budget Roll-Up
| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | fixture and target gate |
| Step 2 | M | real pipeline invariants |
| Step 3 | M | visual and differential inspection |

## Packet Completion Gate
- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS; the negative missing-copy-fixture test must report its precise path without weakening the primary closure.
- Update `docs/07_implementation_status.md` through a worker dispatch.
- Assert packet 213/TASK-329 supersession with closure evidence, and either close TASK-163b-orca-ref using the authoritative references or record its precise external blocker, before implementation status closure.

## Acceptance Ceremony
- Re-dispatch every AC and packet-level gate command.
- Inspect matched-height PNGs and manifest provenance, not just file existence.
- Record remaining external and inherited blockers.
