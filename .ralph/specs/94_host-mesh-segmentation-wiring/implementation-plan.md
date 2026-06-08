# Implementation Plan: 94_host-mesh-segmentation-wiring

## Execution Rules

- One atomic step at a time.
- TDD where applicable: the new integration tests are red until the driver wiring lands.
- Test output teed to `target/test-output.log`.

## Steps

### Step 0: Capture pre-packet baselines (regression_wedge.stl, cube_4color.3mf)

- Task IDs: `TASK-244`
- Objective: AC-11 prerequisite + AC-12 documentation.
- Precondition: P93 closed.
- Postcondition: two baseline SHAs recorded.
- Files allowed to read / edit: none.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p94-baseline-wedge.gcode && sha256sum /tmp/p94-baseline-wedge.gcode`; return FACT (sha256)".
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p94-baseline-cube.gcode && sha256sum /tmp/p94-baseline-cube.gcode`; return FACT (sha256)".
- Context cost: `S`.
- Verification: two FACTs returned.
- Exit condition: both SHAs recorded in implementer's notes.

### Step 1: Read template files; inventory current state

- Task IDs: `TASK-244`
- Objective: pinpoint exact insertion sites + templates.
- Precondition: Step 0 complete.
- Postcondition: inventory recorded.
- Files allowed to read:
  - `crates/slicer-runtime/src/blackboard.rs` — lines 270-310 (range-read).
  - `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs` (47 LOC, full).
- Files allowed to edit: none.
- Files out-of-bounds: kernel.
- Expected sub-agent dispatches:
  - "Open `crates/slicer-runtime/src/prepass.rs` lines 360-410; return SNIPPETS of the existing `host:mesh_analysis` invocation (≤ 30 lines)".
  - "Open `crates/slicer-runtime/src/prepass.rs` lines 680-720; return SNIPPETS of `required_slots` (≤ 25 lines)".
  - "Locate `PrepassExecutionError` enum + `MeshSegmentationError`; return LOCATIONS".
- Context cost: `S`.
- Verification: all LOCATIONS / SNIPPETS returned.
- Exit condition: inventory recorded.

### Step 2: Add `Blackboard::replace_mesh`

- Task IDs: `TASK-244`
- Objective: AC-1, AC-N1.
- Precondition: Step 1 complete.
- Postcondition: method exists; unit tests pass.
- Files allowed to read:
  - `crates/slicer-runtime/src/blackboard.rs` lines 270-310.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/blackboard.rs`.
  - `crates/slicer-runtime/tests/contract/blackboard_replace_mesh_tdd.rs` (NEW).
- Files out-of-bounds: any other.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime blackboard_replace_mesh 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `S`.
- Authoritative docs: roadmap §"P2".
- OrcaSlicer refs: none.
- Verification: unit test passes.
- Exit condition: AC-1, AC-N1 satisfied.

### Step 3: Create `mesh_segmentation_producer.rs`; register in `mod.rs`

- Task IDs: `TASK-244`
- Objective: AC-2, AC-3.
- Precondition: Step 2 green.
- Postcondition: producer constant exists; registered.
- Files allowed to read:
  - `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs` (full).
  - `crates/slicer-runtime/src/builtins/mod.rs` (full; small).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` (NEW).
  - `crates/slicer-runtime/src/builtins/mod.rs` (one line).
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo check -p slicer-runtime`; return FACT pass/fail".
- Context cost: `S`.
- Verification: workspace check clean.
- Exit condition: AC-2, AC-3 satisfied.

### Step 4: Insert driver hook + error variant + table entry in `prepass.rs`

- Task IDs: `TASK-244`
- Objective: AC-4, AC-9, AC-10, AC-N3.
- Precondition: Steps 2-3 green.
- Postcondition: driver runs `host:mesh_segmentation` first; error variant exists; table entry added.
- Files allowed to read:
  - `crates/slicer-runtime/src/prepass.rs` ranged at insertion sites.
  - `crates/slicer-core/src/algos/mesh_segmentation.rs` lines 1-50 (signature + error type).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/prepass.rs`.
- Files out-of-bounds: kernel.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail with first compile error".
- Context cost: `M`.
- Verification: workspace check clean; AC-N3 grep returns the reference.
- Exit condition: AC-4, AC-9, AC-10, AC-N3 satisfied.

### Step 5: Add `mesh_segmentation_short_circuit_no_strokes` integration test

- Task IDs: `TASK-244`
- Objective: AC-5.
- Precondition: Step 4 green.
- Postcondition: test passes; assertion confirms no `replace_mesh` invocation for unpainted mesh.
- Files allowed to read:
  - `crates/slicer-runtime/tests/common/` for fixture-loading helpers.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/mesh_segmentation_short_circuit_no_strokes_tdd.rs` (NEW).
- Files out-of-bounds: other test files.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor mesh_segmentation_short_circuit_no_strokes 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `S`.
- Verification: PASS.
- Exit condition: AC-5 satisfied.

### Step 6: Add cube_4color + cube_fuzzyPainted stroke-consumption tests; add determinism test

- Task IDs: `TASK-244`
- Objective: AC-6, AC-7, AC-8.
- Precondition: Step 5 green.
- Postcondition: three integration tests pass.
- Files allowed to read:
  - Test-fixture helpers.
- Files allowed to edit (≤ 3 per commit; batch acceptable):
  - `crates/slicer-runtime/tests/executor/cube_4color_mesh_segmentation_strokes_consumed_tdd.rs` (NEW).
  - `crates/slicer-runtime/tests/executor/cube_fuzzyPainted_mesh_segmentation_strokes_consumed_tdd.rs` (NEW).
  - `crates/slicer-runtime/tests/executor/mesh_segmentation_determinism_tdd.rs` (NEW).
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test executor cube_4color_mesh_segmentation_strokes_consumed 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test executor cube_fuzzyPainted_mesh_segmentation_strokes_consumed 2>&1 | tee target/test-output.log`; return FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test executor mesh_segmentation_determinism 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `M`.
- Verification: three PASS.
- Exit condition: AC-6, AC-7, AC-8 satisfied.

### Step 7: AC-11 + AC-12 g-code SHA capture

- Task IDs: `TASK-244`
- Objective: behavior-bound check.
- Precondition: Step 6 green.
- Postcondition: regression_wedge.stl SHA matches Step 0 baseline (unchanged); cube_4color.3mf SHA captured for closure log; rationale paragraph written.
- Files allowed to read / edit: none.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p94-post-wedge.gcode && sha256sum /tmp/p94-post-wedge.gcode`; return FACT (sha256)".
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p94-post-cube.gcode && sha256sum /tmp/p94-post-cube.gcode`; return FACT (sha256)".
- Context cost: `S`.
- Verification: wedge SHA matches; cube SHA captured.
- Exit condition: AC-11 satisfied; AC-12 documented.

### Step 8: Guest WASM `--check`

- Task IDs: `TASK-244`
- Objective: AC-13.
- Precondition: Step 7 green.
- Postcondition: guest clean.
- Files allowed to read / edit: none.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo xtask build-guests --check`; return FACT pass/fail".
- Context cost: `S`.
- Verification: PASS.
- Exit condition: AC-13 satisfied.

### Step 9: Final acceptance ceremony

- Task IDs: `TASK-244`
- Objective: final gate.
- Precondition: Step 8 green.
- Postcondition: clippy clean; targeted tests green.
- Expected sub-agent dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 | tee target/test-output.log`; FACT pass/fail".
  - "Run `cargo test -p slicer-core --test algo_mesh_segmentation_tdd 2>&1 | tee target/test-output.log`; FACT pass/fail" — purpose: kernel still passes.
- Context cost: `S`.
- Verification: all PASS.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baselines. |
| Step 1 | S | Templates + inventory. |
| Step 2 | S | replace_mesh. |
| Step 3 | S | Producer + registration. |
| Step 4 | M | Driver insertion + error + table. |
| Step 5 | S | Short-circuit test. |
| Step 6 | M | Three stroke-consumption / determinism tests. |
| Step 7 | S | SHA capture. |
| Step 8 | S | Guest check. |
| Step 9 | S | Workspace gate. |

Aggregate: M.

## Packet Completion Gate

- All 10 steps complete; each exit condition satisfied.
- AC-1 through AC-13 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: pre-packet wedge SHA, post-packet wedge SHA (match), pre-packet cube SHA, post-packet cube SHA (differ — expected; rationale documented), kernel-test pass count unchanged.
- `docs/07_implementation_status.md` updated for `TASK-244` (delegate).
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC command; confirm PASS.
- Confirm unpainted byte-identical (AC-11) + painted SHA capture (AC-12).
- Peak context usage under 70%.
