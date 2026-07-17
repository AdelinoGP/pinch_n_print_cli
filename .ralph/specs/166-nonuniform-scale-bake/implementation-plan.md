# Implementation Plan: 166-nonuniform-scale-bake

## Execution Rules

- Work one atomic step at a time; map every step to grouped task IDs.
- Use TDD, then implementation, then the narrowest falsifying validation.
- Every field below is a context-budget contract and must be filled independently; never write "see Step 1".

## Steps

### Step 1: Downstream uniform-scale-assumption audit (read-only, delegated)

- Task IDs: `TASK-272`
- Objective: prove no consumer of `ObjectMesh.transform` or loaded mesh geometry assumes uniform scale, so the deletion is safe.
- Precondition: clean working tree on the packet branch.
- Postcondition: an audit inventory (checked sites + verdict per site) recorded in `.ralph/specs/166-nonuniform-scale-bake/closure-log.md`; verdict is "clean" or the packet is stopped for re-scope.
- Files allowed to read, with ranges when over 300 lines:
  - none directly — the audit is fully delegated.
- Files allowed to edit (at most 3):
  - `.ralph/specs/166-nonuniform-scale-bake/closure-log.md`
- Files explicitly out of bounds:
  - `crates/slicer-core/**`, `crates/slicer-runtime/**`, `crates/slicer-ir/**` (delegate only); `.claude/worktrees/**`
- Expected sub-agent dispatches:
  - Question: "Does any consumer of `ObjectMesh.transform` or mesh geometry extract a single scalar scale factor or assume uniform scale? Check all `transform_point3` (defined in `crates/slicer-core/src/lib.rs`) call sites in `crates/slicer-core/src/algos/prepass_slice.rs`, `crates/slicer-core/src/algos/mesh_analysis.rs`, `crates/slicer-core/src/algos/paint_segmentation/mod.rs`, and `crates/slicer-core/src/algos/paint_segmentation/painted_line_collection.rs`, plus a workspace grep for sqrt-over-transform-columns and single-scale identifiers"; scope: `crates/slicer-core/src`, `crates/slicer-runtime/src`, `crates/slicer-ir/src`; return: `LOCATIONS` (≤20) + closing `FACT`
- Context cost: `S`
- Authoritative docs:
  - `docs/02_ir_schemas.md` — delegated LOCATIONS lookup of the `ObjectMesh`/`Transform3d` section only
- OrcaSlicer refs:
  - none
- Verification:
  - Audit FACT returned and logged — FACT clean/not-clean
- Exit condition: closure-log contains the site-by-site inventory with an explicit "clean" verdict; a "not clean" verdict stops the packet before Step 2.

### Step 2: Write the RED baking tests

- Task IDs: `TASK-272`
- Objective: add `nonuniform_scale_bake_tdd.rs` with `nonuniform_scale_bakes_vertices_per_axis`, `nonuniform_scale_bakes_paint_triangles`, `uniform_scale_baking_unchanged`.
- Precondition: Step 1 verdict "clean".
- Postcondition: the three tests exist and pass (the baking path already works; these tests are expected GREEN immediately — if any is RED, the failure is a genuine baking bug and must be diagnosed, not asserted around).
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/tests/model_loader_tdd.rs` — lines 150-260 (3MF zip construction pattern)
  - `crates/slicer-model-io/src/loader.rs` — lines 445-530 and 1890-1930 only
- Files allowed to edit (at most 3):
  - `crates/slicer-model-io/tests/nonuniform_scale_bake_tdd.rs` (new)
- Files explicitly out of bounds:
  - `crates/slicer-model-io/src/loader.rs` (read-only this step); all other crates; `.claude/worktrees/**`
- Expected sub-agent dispatches:
  - Question: "run the new test file"; scope: `cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd`; return: `FACT` pass/fail + failing assertion SNIPPETS ≤20 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/08_coordinate_system.md` — direct read only if a unit-space assertion becomes necessary (not expected)
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p slicer-model-io --test nonuniform_scale_bake_tdd 2>&1 | tee target/test-output.log | grep "^test result"` — FACT pass/fail
- Exit condition: all three tests pass; any failure is diagnosed as a real baking defect before proceeding (packet re-scopes if so).

### Step 3: Delete the dead validator, variant, and obsolete test file

- Task IDs: `TASK-272`
- Objective: remove `validate_non_uniform_scale`, `ModelLoadError::NonUniformScaleUnsupported` (+ `Display` arm), `tests/non_uniform_scale_tdd.rs`, and any Cargo.toml `[[test]]` entry for it.
- Precondition: Step 2 tests GREEN.
- Postcondition: AC-3 grep returns no matches; workspace compiles with all targets.
- Files allowed to read, with ranges when over 300 lines:
  - `crates/slicer-model-io/src/loader.rs` — lines 40-95 and 2470-2610 only
  - `crates/slicer-model-io/Cargo.toml` (full; small)
- Files allowed to edit (at most 3):
  - `crates/slicer-model-io/src/loader.rs`
  - `crates/slicer-model-io/tests/non_uniform_scale_tdd.rs` (delete)
  - `crates/slicer-model-io/Cargo.toml` (only if a `[[test]]` block names the deleted file)
- Files explicitly out of bounds:
  - all other crates; `.claude/worktrees/**`
- Expected sub-agent dispatches:
  - Question: "does anything still reference the deleted symbols?"; scope: `grep -rn "NonUniformScaleUnsupported\|validate_non_uniform_scale" --include="*.rs" crates modules`; return: `FACT` (no matches / LOCATIONS of leftovers)
  - Question: "workspace still compiles"; scope: `cargo check --workspace --all-targets`; return: `FACT` pass/fail
- Context cost: `S`
- Authoritative docs:
  - none
- OrcaSlicer refs:
  - none
- Verification:
  - `cd F:/slicerProject/pinch_n_print && grep -rn "NonUniformScaleUnsupported\|validate_non_uniform_scale" --include="*.rs" crates modules; test $? -eq 1 && echo PASS || echo FAIL` — FACT PASS/FAIL
  - `cargo check --workspace --all-targets` — FACT pass/fail
- Exit condition: grep PASS and workspace check clean; a leftover reference falsifies the step.

### Step 4: Regression sweep and crosswalk update

- Task IDs: `TASK-272`
- Objective: prove no loader validation was weakened and no collateral regression; mint TASK-272 in `docs/07_implementation_status.md`.
- Precondition: Step 3 exit met.
- Postcondition: AC-N1/AC-N2 green; clippy clean; docs/07 row added per `task-map.md`.
- Files allowed to read, with ranges when over 300 lines:
  - `target/test-output.log` — grep-driven reads only
- Files allowed to edit (at most 3):
  - `docs/07_implementation_status.md` (via worker dispatch appending the TASK-272 row; never a full read)
  - `.ralph/specs/166-nonuniform-scale-bake/closure-log.md`
- Files explicitly out of bounds:
  - all source files (no code edits in this step)
- Expected sub-agent dispatches:
  - Question: "run the crate suite + world-z test + clippy"; scope: the three commands below; return: `FACT` pass/fail each, failing SNIPPETS ≤20 lines
- Context cost: `S`
- Authoritative docs:
  - `docs/07_implementation_status.md` — delegated append only
- OrcaSlicer refs:
  - none
- Verification:
  - `mkdir -p target && cargo test -p slicer-model-io --test world_z_below_floor_tdd 2>&1 | tee target/test-output.log | grep "^test result"` — FACT pass/fail
  - `mkdir -p target && cargo test -p slicer-model-io 2>&1 | tee target/test-output.log | grep -E "^test result" | grep -E "[1-9][0-9]* failed" && echo FAIL || echo PASS` — FACT PASS/FAIL
  - `cargo clippy --workspace --all-targets -- -D warnings` — FACT pass/fail
- Exit condition: all three FACTs pass and the docs/07 TASK-272 row exists (`grep -c "TASK-272" docs/07_implementation_status.md` ≥ 1).

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 1 | S | fully delegated audit |
| Step 2 | S | one new test file |
| Step 3 | S | pure deletion |
| Step 4 | S | delegated verification + crosswalk |

Split before activation if aggregate cost exceeds M or any step is L.

## Packet Completion Gate

- All steps and exits complete.
- Every pipe-suffixed AC command returns PASS.
- Update `docs/07_implementation_status.md` through a worker dispatch, never a full backlog read.
- Reconcile reopened/superseded status transitions (none expected).
- `packet.spec.md` is ready for `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed AC and packet-level gate command.
- Record remaining packet-local risk.
- Confirm context stayed at or below 150k standard, or at/below 300k only with a logged swarm ESCALATION; otherwise record a packet-authoring lesson.

All `cargo check`, `cargo clippy`, and `cargo test` invocations in gate and verification commands must use `--all-targets` so the test, bench, and example targets compile.
