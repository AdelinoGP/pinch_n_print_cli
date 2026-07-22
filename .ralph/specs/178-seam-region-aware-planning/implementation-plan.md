# Implementation Plan: 178-seam-region-aware-planning

## Execution Rules

- Work one atomic step at a time; map every step to `TASK-291` (re-derived 2026-07-22 against `docs/07_implementation_status.md`; the previously quoted `TASK-284` row is the closed `claim:raft-fill` row of packet 124; the original `TASK-281` row is closed under packet 117).
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract; do not discover struct-literal fallout after the step.

## Steps

### Step 1: Versioned seam-planning input and identity inventory

- Task IDs: `TASK-291`
- Objective: Add the WIT/SDK records for per-region seam-planning input and variant-aware seam-plan output, bump the prepass world major version, and inventory every generated-shim and struct-literal consumer before implementation.
- Precondition: packet 168's `run-seam-planning` world version is present and guest freshness is clean.
- Postcondition: the trait and generated shim signatures describe the new view; all affected struct literals are listed; no production planner behavior changes yet.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` - full file, 150 lines.
  - `crates/slicer-schema/wit/deps/ir-types.wit` - lines 1-121.
  - `crates/slicer-sdk/src/traits.rs` - lines 584-640.
  - `crates/slicer-sdk/src/prepass_types.rs` - lines 240-304.
- Files allowed to edit (at most 3):
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`
  - `crates/slicer-schema/wit/deps/ir-types.wit`
  - `crates/slicer-sdk/src/traits.rs`
- Blast-radius discipline: dispatch a `LOCATIONS` worker for all generated guest shim, SDK struct-literal, and test-guest call sites before editing; include those sites in the same step's allowed-edit inventory if compilation requires mechanical updates.
- Files explicitly out of bounds:
  - `OrcaSlicerDocumented/**`, `target/**`, `Cargo.lock`, and final placement modules.
- Expected sub-agent dispatches:
  - Question: identify all `run-seam-planning` bindgen/macro call sites and SDK struct literals; scope: `crates/slicer-macros/**`, `modules/core-modules/*/wit-guest/**`, `crates/slicer-wasm-host/test-guests/**`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/11_operational_governance_and_acceptance_gate.md` - delegated WIT bump policy.
  - `docs/02_ir_schemas.md` - delegated identity and coordinate locations.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` - delegate candidate/perimeter identity fields.
- Verification:
  - `cargo build --workspace --all-targets` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT pass/fail.
- Exit condition: the new WIT view and variant-chain field compile through every prepass guest shim, and the world version is exactly `3.0.0`.

### Step 2: Late prepass projection and full-key harvest

- Task IDs: `TASK-291`
- Objective: Schedule seam planning after required region/slice products, project deterministic per-active-region SliceIR views, and preserve full identity through harvest and blackboard commit.
- Precondition: Step 1 signatures compile.
- Postcondition: the host dispatch supplies only committed active-region records; `harvest_seam_plan_ir_from` preserves `variant_chain`; duplicate and malformed keys reject.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-runtime/src/prepass.rs` - lines 385-679 and 714-783.
  - `crates/slicer-wasm-host/src/dispatch.rs` - lines 656-849.
  - `crates/slicer-wasm-host/src/marshal/in_.rs` - lines 180-324 and 491-568.
  - `crates/slicer-ir/src/slice_ir.rs` - lines 1202-1234 and 1340-1372.
- Files allowed to edit (at most 3):
  - `crates/slicer-runtime/src/prepass.rs`
  - `crates/slicer-wasm-host/src/dispatch.rs`
  - `crates/slicer-wasm-host/src/marshal/in_.rs`
- Files explicitly out of bounds:
  - `modules/core-modules/seam-planner-default/src/visibility.rs` and `align.rs`; packet 2 owns scoring.
  - `target/**`, generated code, and direct Orca source.
- Expected sub-agent dispatches:
  - Question: verify deterministic projection order and exact `SlicedRegion` annotation/variant conversion; scope: `crates/slicer-wasm-host/src/marshal/**`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/01_system_architecture.md` - delegated stage execution order.
  - `docs/04_host_scheduler.md` - delegated IR access and stage order.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate `extract_perimeter_polygons` and `process_perimeter_polygon` locations.
- Verification:
  - `cargo test -p slicer-runtime --test contract -- dispatch_prepass_harvest_tdd 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail.
  - `cargo test -p slicer-runtime --test contract -- seam_plan_ir_rejects_duplicate_region_keys 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail.
- Exit condition: a committed seam plan contains exact active-region identity and the seam guest runs only when all declared input slots are present.

### Step 3: Planner and perimeter identity vertical slice

- Task IDs: `TASK-291`
- Objective: Make `run_aligned_planning` consume supplied per-region polygons, emit one entry per active key, and make perimeter-region injection match the same variant identity without changing canonical scoring.
- Precondition: Steps 1-2 compile and contract tests pass.
- Postcondition: a two-variant fixture receives two independent plans and each seam placer input receives only its matching plan.
- Files allowed to read, with ranges when over 300 lines:
  - `modules/core-modules/seam-planner-default/src/lib.rs` - lines 68-199 and 221-453.
  - `crates/slicer-ir/src/slice_ir.rs` - lines 1924-1959 and schema/version declarations.
  - `modules/core-modules/seam-placer/src/lib.rs` - lines 245-353.
- Files allowed to edit (at most 3):
  - `modules/core-modules/seam-planner-default/src/lib.rs`
  - `crates/slicer-ir/src/slice_ir.rs`
  - `modules/core-modules/seam-placer/src/lib.rs`
- Blast-radius discipline: include every `PerimeterRegion` literal, `PerimeterRegionView` builder, WIT conversion, and test assertion affected by variant identity in the same step.
- Files explicitly out of bounds:
  - canonical comparator/visibility/spline code; packet 2.
  - continuous path insertion/default changes; packet 3.
- Expected sub-agent dispatches:
  - Question: locate every `PerimeterRegion` and `SeamPlanEntry` struct literal affected by the identity field; scope: `crates/**/src`, `crates/**/tests`, `modules/**/src`, `modules/**/tests`; return: `LOCATIONS`.
- Context cost: `M`
- Authoritative docs:
  - `docs/02_ir_schemas.md` - delegated `PerimeterRegion` and `SeamPlanIR` contract.
  - `docs/05_module_sdk.md` - delegated seam candidate/wall preservation behavior.
- OrcaSlicer refs:
  - `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` - delegate final candidate ownership only; do not port algorithm in this step.
- Verification:
  - `cargo test -p seam-planner-default --test seam_region_aware_planning_tdd 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail.
  - `cargo test -p slicer-runtime --test contract -- prepass_seam_planning_commits_seam_plan_ir 2>&1 | tee target/test-output.log | grep '^test result'` - FACT pass/fail.
  - `cargo xtask build-guests --check` - FACT pass/fail.
- Exit condition: multi-region active-region plans are independently harvested, injected, and observable in the seam placer; no test or implementation path uses contour ordinal identity.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | M | WIT version and struct-literal blast radius. |
| Step 2 | M | Late prepass projection and harvest identity. |
| Step 3 | M | Guest planner plus perimeter identity vertical slice. |

Split before activation if any step becomes L or if the packet's aggregate exceeds M.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- `docs/07_implementation_status.md` receives the `TASK-291` crosswalk through a worker dispatch (free ID re-derived 2026-07-22; the previously quoted `TASK-284` is the closed `claim:raft-fill` row of packet 124; `TASK-281`/`TASK-282` are closed under packet 117).
- `D-168-SEAM-PREPASS-SOURCE` is **narrowed** with evidence: part (1) source-geometry divergence is closed by per-region `SliceIR` input; parts (2)-(5) (sample budget, B-spline solver, `curling_influence`, short-string retry) stay Open and are explicitly handed off to packet 179. Update `docs/DEVIATION_LOG.md` row to reflect "Narrowed by packet 178 (part 1 closed); parts 2-5 Open for packet 179."
- `packet.spec.md` is ready for `status: implemented` once all of the above hold. (Originally drafted to require packets 179/180 to consume the exports; flipped 2026-07-22 because the technical surface is on disk and green, and the gating rule blocked the two successor packets the parity plan itself names as the rationale.)

## Acceptance Ceremony

- Re-dispatch every AC and packet-level gate command.
- Re-run `cargo xtask build-guests --check` after all guest-input edits.
- Record the exact new WIT/IR version literals and any remaining source-geometry limitation.
- Confirm context stayed within the standard packet budget.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` where the command supports that flag.