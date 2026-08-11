# Implementation Plan: support-planner-defect-fix

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-322`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently.

## Steps

### Step 1: Lock focused planner regression coverage

- Task IDs: `TASK-322`
- Objective: add or extend focused assertions for lone-node emission and the radius floor.
- Precondition: current node and helper shapes are confirmed in the bounded source ranges.
- Postcondition: tests assert a lone propagated node produces a degenerate segment and `tapered_radius` returns at least `0.4` at the tip.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - lines `480-585`, `603-694`, `1288-1311`
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs` - lines `1-90`, `140-225`
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/tests/orca_parity_tdd.rs`
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, generated guests, unrelated crates
- Expected sub-agent dispatches:
  - Question: locate the smallest existing test harness for the two assertions; scope: `modules/core-modules/support-planner/tests/**`; return: `LOCATIONS`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-generation-defect-verified-findings.md` - lines `58-86` and `128-136`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegated `LOCATIONS`
- Verification:
  - `cargo test -p support-planner --all-targets` - FACT pass/fail
- Exit condition: focused tests fail before the implementation and pass after it, with assertions naming the segment shape and `0.4` floor.

### Step 2: Implement lone-node continuation and minimum radius

- Task IDs: `TASK-322`
- Objective: update `plan_for_object` and `tapered_radius` without changing propagation, merge, collision, or MST behavior.
- Precondition: Step 1 regression assertions exist and fail for the current implementation.
- Postcondition: all surviving lone propagated nodes emit degenerate current-layer segments; radius clamp is `[MIN_BRANCH_RADIUS, MAX_BRANCH_RADIUS_MM]`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/support-planner/src/lib.rs` - lines `603-694` and `1288-1311`
- Files allowed to edit (at most 3):
  - `modules/core-modules/support-planner/src/lib.rs`
- Files explicitly out of bounds:
  - `crates/slicer-ir/**`, WIT, fallback support modules, scheduler, G-code, `target/**`
- Expected sub-agent dispatches:
  - Question: verify the edited source retains drop and collision guards; scope: `modules/core-modules/support-planner/src/lib.rs`; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-generation-remediation-plan.md` - lines `24-33`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` - delegated `SUMMARY`
- Verification:
  - `cargo test -p support-planner --all-targets` - FACT pass/fail
  - `cargo xtask build-guests --check` - FACT pass/fail; rebuild if stale
- Exit condition: focused tests pass and source has exact `MIN_BRANCH_RADIUS = 0.4` clamp without removing existing guards.

### Step 3: Prove guest and visual behavior

- Task IDs: `TASK-322`
- Objective: prove branch geometry reaches lower layers and contact geometry is renderable.
- Precondition: Step 2 passes and guest artifacts are fresh.
- Postcondition: the visual-debug manifest exists under `target/vd-tree-fixed` and requested planner/consumer layers are captured for inspection.
- Files allowed to read, with ranges when over 300 lines:
  - `tmp/visual-debug-tree-fixed.json` - full request file; both taps at layers `0/50/100/125`
- Files allowed to edit (at most 3):
  - None; generated output is under `target/` and is read-only evidence.
- Files explicitly out of bounds:
  - `target/**` source edits, Orca source, unrelated tests
- Expected sub-agent dispatches:
  - Question: inspect manifests and PNGs for branch geometry at layers `100/50/0` and non-zero contact width; scope: `target/vd-tree-fixed/**`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-generation-defect-verified-findings.md` - lines `178-231`, `240-253`
- OrcaSlicer refs:
  - None beyond Step 2 delegation.
- Verification:
  - `cargo xtask build-guests --check` - FACT pass/fail
  - `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-tree-fixed.json --output target/vd-tree-fixed --overwrite` - FACT manifest plus bounded evidence for planner and consumer taps at layers `0/50/100/125`
- Exit condition: delegated visual inspection confirms planner geometry at layers `100/50/0` and no zero-width filled-area failure at the contact.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | focused test harness |
| Step 2 | S | one implementation file |
| Step 3 | M | visual-debug evidence |

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local visual risk.
- Confirm context stayed within the standard band.
