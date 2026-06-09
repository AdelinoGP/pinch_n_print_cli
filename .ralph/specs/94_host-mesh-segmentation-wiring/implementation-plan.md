# Implementation Plan: 94_host-mesh-segmentation-wiring

## Execution Rules

- One atomic step at a time.
- TDD where applicable: the new integration tests are red until the driver wiring lands.
- Test output teed to `target/test-output.log`.

## Steps

### Step 0: Capture pre-packet baselines into `closure-log.md`

- Task IDs: `TASK-244`
- Objective: write `P93_BASELINE_SHA` (wedge) and `P94_PRE_PAINTED_CUBE_SHA` (cube_4color) into the packet's closure-log.md so AC-11's baseline-compare command can find them.
- Precondition: P93 closed.
- Postcondition: `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` exists and contains both `KEY=hex` lines on their own lines.
- Files allowed to read: none.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (CREATE if absent).
- Files out-of-bounds: anything else.
- Expected sub-agent dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p94-baseline-wedge.gcode && sha256sum /tmp/p94-baseline-wedge.gcode | awk '{print $1}'`; return FACT (sha256)".
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p94-baseline-cube.gcode && sha256sum /tmp/p94-baseline-cube.gcode | awk '{print $1}'`; return FACT (sha256)".
  - "Append (or create+append) the lines `P93_BASELINE_SHA=<hex>` and `P94_PRE_PAINTED_CUBE_SHA=<hex>` to `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md`; return FACT (created vs appended)".
- Context cost: `S`.
- Verification: `grep -q '^P93_BASELINE_SHA=' .ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md && grep -q '^P94_PRE_PAINTED_CUBE_SHA=' .ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md`.
- Exit condition: both lines present in closure-log.md.

### Step 0.5: Open `TASK-244` row in `docs/07_implementation_status.md`

- Task IDs: `TASK-244`
- Objective: backfill the docs/07 row that establishes `TASK-244` as the canonical id for this work, matching the P89–P93 convention (row opened at activation, closed at packet end).
- Precondition: Step 0 complete.
- Postcondition: a `TASK-244 — host:mesh_segmentation wiring` row exists in `docs/07_implementation_status.md` with status `in-progress` (or whatever in-progress glyph the file uses — verify by reading neighbouring rows TASK-239..TASK-243).
- Files allowed to read: `docs/07_implementation_status.md` (range-read the TASK-239..TASK-243 region only).
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md`.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Range-read `docs/07_implementation_status.md` around the TASK-239..TASK-243 rows; return SNIPPETS (≤ 30 lines) showing the row format + in-progress marker".
  - "Append a `TASK-244 — host:mesh_segmentation wiring` row matching that format, marked in-progress; return FACT diff applied".
- Context cost: `S`.
- Verification: `rg -q '^.*TASK-244.*host:mesh_segmentation' docs/07_implementation_status.md`.
- Exit condition: row present and in-progress.

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

### Step 3.5: Disable the WASM `mesh-segmentation` manifest's stage claim

- Task IDs: `TASK-244`
- Objective: AC-3.5. Prevent the DAG validator from rejecting Step 4's driver hook by ensuring only the host built-in claims `PrePass::MeshSegmentation`.
- Precondition: Step 3 green (host producer module registered).
- Postcondition: `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` no longer matches the AC-3.5 grep; guests rebuilt clean.
- Files allowed to read:
  - `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` (full; the manifest is small).
- Files allowed to edit (≤ 3):
  - `modules/core-modules/mesh-segmentation/mesh-segmentation.toml`.
- Files out-of-bounds: the `src/` and `wit-guest/` subtrees of `mesh-segmentation/` — P5a's territory.
- Expected sub-agent dispatches:
  - "Read `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` in full; return SNIPPETS (≤ 30 lines) showing the `[stage]` block with `id = \"PrePass::MeshSegmentation\"`".
  - "Apply the smallest viable disable mechanism (comment out the `id = \"PrePass::MeshSegmentation\"` line inside `[stage]`, comment the whole `[stage]` block, rename manifest to `.disabled`, or loader-disabled flag); return FACT (mechanism chosen + diff)".
  - "Run `! rg -q '^id\\s*=\\s*\"PrePass::MeshSegmentation\"' modules/core-modules/mesh-segmentation/mesh-segmentation.toml`; return FACT pass/fail".
  - "Run `cargo xtask build-guests`; return FACT (build success)".
  - "Run `cargo xtask build-guests --check`; return FACT (clean)".
- Context cost: `S`.
- Verification: AC-3.5 grep + `--check` clean.
- Exit condition: AC-3.5 satisfied.

### Step 4: Insert driver hook + error variant + table entry in `prepass.rs`

- Task IDs: `TASK-244`
- Objective: AC-4, AC-9, AC-N3 (AC-10's compile-test ships in Step 4.5).
- Precondition: Steps 2-3.5 green.
- Postcondition: driver runs `host:mesh_segmentation` first; error variant exists; table entry added.
- Files allowed to read:
  - `crates/slicer-runtime/src/prepass.rs` ranged at insertion sites.
  - `crates/slicer-core/src/algos/mesh_segmentation.rs` lines 1-50 (signature + error type).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/prepass.rs`.
- Files out-of-bounds: kernel.
- Expected sub-agent dispatches:
  - "Run `cargo check --workspace --all-targets`; return FACT pass/fail with first compile error".
  - "Run `rg -q '\"PrePass::MeshSegmentation\"\\s*=>\\s*&\\[\\]' crates/slicer-runtime/src/prepass.rs`; return FACT pass/fail (AC-9 anchor)".
  - "Run `rg -q 'execute_mesh_segmentation' crates/slicer-runtime/src/prepass.rs`; return FACT pass/fail (AC-N3 dead-code gone)".
- Context cost: `M`.
- Verification: workspace check clean; AC-9 + AC-N3 greps pass.
- Exit condition: AC-4, AC-9, AC-N3 satisfied.

### Step 4.5: Add `PrepassExecutionError::MeshSegmentation` variant compile-test

- Task IDs: `TASK-244`
- Objective: AC-10. Prove the variant constructs with the correct field shape AND that `?`-propagation typechecks (a `From<MeshSegmentationError>` impl is wired, via `#[from]` or an equivalent `impl From` block).
- Precondition: Step 4 green (variant exists in the enum).
- Postcondition: a new contract test compiles and runs to completion.
- Files allowed to read:
  - `crates/slicer-runtime/src/prepass.rs` (range-read at the `PrepassExecutionError` definition only).
  - `crates/slicer-core/src/algos/mesh_segmentation.rs` lines 1-50 (signature + error type variants).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/contract/prepass_execution_error_mesh_segmentation_variant_tdd.rs` (NEW).
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run `cargo test -p slicer-runtime --test contract prepass_execution_error_mesh_segmentation_variant 2>&1 | tee target/test-output.log`; return FACT pass/fail".
- Context cost: `S`.
- Verification: PASS.
- Exit condition: AC-10 satisfied.

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

### Step 7: AC-11 baseline-compare + closure-log painted-cube SHA capture

- Task IDs: `TASK-244`
- Objective: AC-11 (unpainted wedge byte-identical against `P93_BASELINE_SHA`) + closure-log obligation (painted `cube_4color.3mf` post-packet SHA + rationale).
- Precondition: Step 6 green.
- Postcondition: AC-11 baseline-compare command exits 0; `P94_POST_PAINTED_CUBE_SHA=<hex>` written to closure-log.md; one-paragraph rationale appended.
- Files allowed to read:
  - `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (range-read to confirm the Step 0 baselines are present).
- Files allowed to edit (≤ 3):
  - `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md`.
- Files out-of-bounds: any.
- Expected sub-agent dispatches:
  - "Run AC-11 baseline-compare verbatim from `packet.spec.md` (the `mkdir -p target && cargo run ... && test ...` command); return FACT pass/fail".
  - "Run `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p94-post-cube.gcode && sha256sum /tmp/p94-post-cube.gcode | awk '{print $1}'`; return FACT (sha256)".
  - "Append `P94_POST_PAINTED_CUBE_SHA=<hex>` and a one-paragraph rationale to `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md`; return FACT (lines added)".
- Context cost: `S`.
- Verification: AC-11 baseline-compare passes; `grep -q '^P94_POST_PAINTED_CUBE_SHA=' closure-log.md`.
- Exit condition: AC-11 satisfied; closure-log obligation discharged.

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
- Postcondition: clippy clean; targeted tests green; `TASK-244` row transitioned to implemented in `docs/07_implementation_status.md`.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/94_host-mesh-segmentation-wiring/packet.spec.md` (status update, after /spec-review green).
  - `docs/07_implementation_status.md` (transition the TASK-244 row).
- Expected sub-agent dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 | tee target/test-output.log`; FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test contract blackboard_replace_mesh 2>&1 | tee target/test-output.log`; FACT pass/fail".
  - "Run `cargo test -p slicer-runtime --test contract prepass_execution_error_mesh_segmentation_variant 2>&1 | tee target/test-output.log`; FACT pass/fail".
  - "Run `cargo test -p slicer-core --test algo_mesh_segmentation_tdd 2>&1 | tee target/test-output.log`; FACT pass/fail (kernel regression sanity)".
  - "Transition the `TASK-244` row in `docs/07_implementation_status.md` from in-progress to implemented, matching the glyph convention used for TASK-239..TASK-243; return FACT diff applied".
- Context cost: `S`.
- Verification: all PASS.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baselines into closure-log. |
| Step 0.5 | S | docs/07 row open. |
| Step 1 | S | Templates + inventory. |
| Step 2 | S | replace_mesh. |
| Step 3 | S | Producer + registration. |
| Step 3.5 | S | WASM manifest disable + guest rebuild. |
| Step 4 | M | Driver insertion + error + table. |
| Step 4.5 | S | Variant compile-test (AC-10). |
| Step 5 | S | Short-circuit test. |
| Step 6 | M | Three stroke-consumption / determinism tests. |
| Step 7 | S | AC-11 baseline-compare + closure-log painted-cube capture. |
| Step 8 | S | Guest check. |
| Step 9 | S | Workspace gate + TASK-244 row close. |

Aggregate: M.

## Packet Completion Gate

- All 13 steps complete; each exit condition satisfied.
- AC-1, AC-2, AC-3, AC-3.5, AC-4, AC-5, AC-6, AC-7, AC-8, AC-9, AC-10, AC-11, AC-13 verified. AC-N1, AC-N2, AC-N3 verified.
- Closure log records: `P93_BASELINE_SHA=<hex>`, `P94_PRE_PAINTED_CUBE_SHA=<hex>`, `P94_POST_PAINTED_CUBE_SHA=<hex>`, and one-paragraph normalization rationale.
- `docs/07_implementation_status.md` `TASK-244` row transitioned to implemented (Step 9).
- `/spec-review 94_host-mesh-segmentation-wiring` returns activation-gate green.
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC command; confirm PASS.
- Confirm AC-11 baseline-compare exits 0 and closure-log obligations are present.
- Peak context usage under 70%.
