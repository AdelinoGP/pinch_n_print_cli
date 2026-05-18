# Implementation Plan: live-support-generation

## Execution Rules

- One atomic step at a time.
- Start with live host commitment tests, not broad Benchy assertions.

## Steps

### Step 1: Add failing host tests for tree-support and traditional-support live commitment

- Task IDs:
  - `TASK-120b`
- Objective:
  Add failing host integration tests that prove the live `Layer::Support` stage still commits empty or missing support output today.
- Precondition:
  Current support module unit tests do not prove the live host path commits `SupportIR` correctly.
- Postcondition:
  `live_support_generation_tdd.rs` exists with tree-support, traditional-support, enforcer, determinism, and empty-output assertions.
- Files expected to change:
  - `crates/slicer-host/tests/live_support_generation_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.hpp`
- Verification:
  - `cargo test -p slicer-host --test live_support_generation_tdd tree_support_dispatch_commits_support_material_paths -- --exact --nocapture`
  - `cargo test -p slicer-host --test live_support_generation_tdd traditional_support_dispatch_commits_support_material_paths -- --exact --nocapture`
- Exit condition:
  Focused host tests exist and fail only on live support commitment gaps.

### Step 2: Restore the tree-support live path

- Task IDs:
  - `TASK-120b`
- Objective:
  Make tree-support the canonical live support generator on the production host path.
- Precondition:
  Step 1 host tests are in place.
- Postcondition:
  Tree-support commits non-empty `SupportIR.support_paths` with exact `SupportMaterial` roles on the real host path.
- Files expected to change:
  - `modules/core-modules/tree-support/src/lib.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/tests/live_support_generation_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport3D.hpp`
- Verification:
  - `cargo test -p slicer-host --test live_support_generation_tdd tree_support_dispatch_commits_support_material_paths -- --exact --nocapture`
- Exit condition:
  Tree-support live commitment test passes.

### Step 3: Keep the shared host path honest with a traditional-support control and paint precedence guards

- Task IDs:
  - `TASK-120b`
- Objective:
  Verify the same host path works with traditional-support and respects blocker/enforcer precedence.
- Precondition:
  Tree-support live commitment is green.
- Postcondition:
  Traditional-support commits support on the same host surface, blocker/enforcer precedence stays green, and disabled/ineligible inputs stay empty.
- Files expected to change:
  - `modules/core-modules/traditional-support/src/lib.rs`
  - `crates/slicer-host/tests/live_support_generation_tdd.rs`
  - `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs`
- Authoritative docs:
  - `docs/01_system_architecture.md`
  - `docs/02_ir_schemas.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.hpp`
- Verification:
  - `cargo test -p slicer-host --test live_support_generation_tdd traditional_support_dispatch_commits_support_material_paths -- --exact --nocapture`
  - `cargo test -p slicer-host --test live_support_generation_tdd enforcer_forces_live_support_commit_even_when_needs_support_is_false -- --exact --nocapture`
  - `cargo test -p slicer-host --test live_support_generation_tdd disabled_or_ineligible_support_stage_commits_empty_support_ir -- --exact --nocapture`
  - `cargo test -p tree-support --test enforcer_blocker_tdd blocker_overrides_needs_support_true -- --exact --nocapture`
- Exit condition:
  Control generator and precedence tests pass on the live path.

### Step 4: Add a deterministic live support regression

- Task IDs:
  - `TASK-120b`
- Objective:
  Ensure repeated identical host-stage runs produce byte-stable support output.
- Precondition:
  Steps 2 and 3 are green.
- Postcondition:
  The live support stage is deterministic across repeated runs.
- Files expected to change:
  - `crates/slicer-host/tests/live_support_generation_tdd.rs`
- Authoritative docs:
  - `docs/04_host_scheduler.md`
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.hpp`
- Verification:
  - `cargo test -p slicer-host --test live_support_generation_tdd live_support_dispatch_is_deterministic_across_repeated_runs -- --exact --nocapture`
- Exit condition:
  Determinism test passes.

## Packet Completion Gate

- All steps complete.
- All pipe-suffixed acceptance commands pass.
- `cargo clippy --workspace -- -D warnings` passes.
- `docs/07_implementation_status.md` updated for `TASK-120b`.

## Acceptance Ceremony

- Re-run all acceptance commands from `packet.spec.md`.
- Confirm tree-support remains the canonical live acceptance target.
- Record any remaining packet-local risk before status changes.