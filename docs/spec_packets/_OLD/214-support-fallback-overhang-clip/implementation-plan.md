# Implementation Plan: support-fallback-overhang-clip

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-323`.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently.

## Steps

### Step 1: Add fallback source-selection regressions

- Task IDs: `TASK-323`
- Objective: establish assertions for clipped DefaultEligible fill, full Enforced fill, and blocked rejection.
- Precondition: existing module test harnesses and polygon fixtures are located.
- Postcondition: focused tests express the three policy outcomes without changing production code.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support/src/lib.rs` - lines `107-174`
  - `modules/core-modules/tree-support/src/lib.rs` - lines `130-194`
  - module test files containing `run_support` fixtures
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support/tests/**`
  - `modules/core-modules/tree-support/tests/**`
- Files explicitly out of bounds:
  - host marshalling, IR/WIT, generated guests, unrelated modules
- Expected sub-agent dispatches:
  - Question: locate viable module fixtures; scope: `modules/core-modules/{traditional-support,tree-support}/tests/**`; return: `LOCATIONS`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-generation-defect-verified-findings.md` - lines `88-107`
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p traditional-support --all-targets` - FACT pass/fail
  - `cargo test -p tree-support --all-targets` - FACT pass/fail
- Exit condition: tests fail against whole-polygon DefaultEligible behavior and encode the preserved Enforced/Blocked outcomes.

### Step 2: Implement fallback clipping and host derivation

- Task IDs: `TASK-323`
- Objective: select `overhang_areas()` only for DefaultEligible and derive `needs_support` from the already collected vector.
- Precondition: Step 1 tests identify the policy paths; `overhang_areas()` and `needs_support` fields are confirmed existing.
- Postcondition: both fallback modules clip DefaultEligible, Enforced retains `polygons()`, and host emits `needs_support: !overhang_areas.is_empty()`.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/traditional-support/src/lib.rs` - lines `140-170`
  - `modules/core-modules/tree-support/src/lib.rs` - lines `144-190`
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - lines `342-424`
- Files allowed to edit (at most 3):
  - `modules/core-modules/traditional-support/src/lib.rs`
  - `modules/core-modules/tree-support/src/lib.rs`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
- Files explicitly out of bounds:
  - IR/WIT schemas, SDK accessor, scheduler, support planner, generated output
- Expected sub-agent dispatches:
  - Question: verify no hardcoded host assignment remains and policy branches retain their precedence; scope: the three edited files; return: `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/specs/support-generation-remediation-plan.md` - lines `26-32`
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo test -p slicer-wasm-host --all-targets` - FACT pass/fail
  - `cargo test -p traditional-support --all-targets` - FACT pass/fail
  - `cargo test -p tree-support --all-targets` - FACT pass/fail
- Exit condition: all targeted tests pass and static checks show the exact overhang and boolean expressions.

### Step 3: Prove fallback visual behavior

- Task IDs: `TASK-323`
- Objective: prove non-overhang layers no longer contain pillar-interior fallback paths.
- Precondition: Step 2 passes and guest artifacts are fresh.
- Postcondition: `target/vd-support-fixed/manifest.json` captures Layer::Support at 10, 24, and 30 for visual inspection.
- Files allowed to read, with ranges when over 300 lines:
  - `tmp/visual-debug-support.json` - full request file
- Files allowed to edit (at most 3):
  - None; generated output is evidence only.
- Files explicitly out of bounds:
  - `target/**` source edits, planner code, Orca source
- Expected sub-agent dispatches:
  - Question: inspect manifest and PNGs for absence of support lines inside X=-10..0/Y=0..20 pillar on layers 10/24/30; scope: `target/vd-support-fixed/**`; return: `FACT`
- Context cost: `M`
- Authoritative docs:
  - `docs/specs/support-generation-defect-verified-findings.md` - lines `103-107`, `193-196`, `225-231`, `251-253`
- OrcaSlicer refs:
  - None.
- Verification:
  - `cargo xtask build-guests --check` - FACT pass/fail
  - `cargo run -q -p pnp-cli --bin pnp_cli -- visual-debug --request tmp/visual-debug-support.json --output target/vd-support-fixed --overwrite` - FACT manifest plus bounded evidence
- Exit condition: delegated visual inspection confirms no pillar-interior support lines on layers 10/24/30 and support geometry remains available only where overhang eligibility permits.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | two module harnesses |
| Step 2 | S | three bounded production ranges |
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
