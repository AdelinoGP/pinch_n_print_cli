# Implementation Plan: 98_loader-paint-channel-symmetry

## Execution Rules

- One step at a time.
- All `cargo test` and `pnp_cli` runs prefixed with `mkdir -p target &&`.

## Steps

### Step 0: Capture pre-packet baselines

- Task IDs: `TASK-248`
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p98-baseline-wedge.gcode && sha256sum /tmp/p98-baseline-wedge.gcode`; FACT".
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p98-baseline-cube.gcode && sha256sum /tmp/p98-baseline-cube.gcode`; FACT".
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_fuzzyPainted.3mf --module-dir modules/core-modules --output /tmp/p98-baseline-cube-fuzzy.gcode && sha256sum /tmp/p98-baseline-cube-fuzzy.gcode`; FACT".
- Context cost: `S`.
- Exit condition: 3 SHAs recorded.

### Step 1: Inventory existing decoder + check fixture availability

- Task IDs: `TASK-248`
- Expected dispatches:
  - "Open `crates/slicer-model-io/src/loader.rs` lines 1119-1295; return SNIPPETS (≤ 60 lines)".
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp` hex format + per-channel semantic encoding; SUMMARY ≤ 150 words".
  - "Locate any fixture exercising `paint_seam` sub-facet strokes in `resources/`; return FACT".
- Context cost: `S`.
- Exit condition: SNIPPETS + SUMMARY + FACT recorded.

### Step 2: Hoist decoder + add 4 call sites

- Task IDs: `TASK-248`
- Objective: AC-1, AC-2.
- Files allowed to edit (≤ 3):
  - `crates/slicer-model-io/src/loader.rs`.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-model-io 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `S`.
- Exit condition: AC-1, AC-2 satisfied; existing tests still pass.

### Step 3: Add 4 per-channel positive tests + 3 negative tests

- Task IDs: `TASK-248`
- Objective: AC-3, AC-4, AC-5, AC-6, AC-N1, AC-N2, AC-N3.
- Files allowed to edit (≤ 3 per commit):
  - `crates/slicer-model-io/tests/model_loader_tdd.rs`.
  - Optionally `resources/cube_seam_painted.3mf` (if Step 1's FACT said no fixture).
- Expected dispatches:
  - "Run `mkdir -p target && cargo test -p slicer-model-io 2>&1 | tee target/test-output.log`; FACT pass/fail".
- Context cost: `M`.
- Exit condition: ACs satisfied.

### Step 4: Add AC-7 normalization test in slicer-runtime

- Task IDs: `TASK-248`
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/tests/executor/cube_fuzzyPainted_paint_fuzzy_skin_strokes_normalized_tdd.rs` (NEW).
- Expected dispatches:
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_fuzzyPainted_paint_fuzzy_skin_strokes_normalized 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `S`.
- Exit condition: AC-7 satisfied.

### Step 5: Post-packet SHA capture (AC-8, AC-9, AC-10)

- Task IDs: `TASK-248`
- Expected dispatches:
  - Wedge SHA — must match Step 0.
  - Cube_4color SHA — must match Step 0.
  - Cube_fuzzyPainted SHA — may differ (document in closure log).
- Context cost: `S`.
- Exit condition: AC-8, AC-9, AC-10 satisfied (AC-10 may differ — rationale in closure log).

### Step 6: Guest WASM `--check`

- Task IDs: `TASK-248`
- Objective: AC-11.
- Expected dispatches:
  - "Run `cargo xtask build-guests --check`; FACT".
- Context cost: `S`.
- Exit condition: AC-11 satisfied.

### Step 7: Workspace gate

- Task IDs: `TASK-248`
- Expected dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; FACT".
  - "Run `mkdir -p target && cargo test -p slicer-model-io 2>&1 | tee target/test-output.log`; FACT".
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_fuzzyPainted 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `S`.
- Exit condition: all PASS.

## Per-Step Budget Roll-Up

| Step | Context Cost |
| --- | --- |
| Step 0 | S |
| Step 1 | S |
| Step 2 | S |
| Step 3 | M |
| Step 4 | S |
| Step 5 | S |
| Step 6 | S |
| Step 7 | S |

Aggregate: S.

## Packet Completion Gate

- All 8 steps complete.
- AC-1 through AC-11 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: pre/post wedge SHA (match), pre/post cube_4color SHA (match), pre/post cube_fuzzyPainted SHA (may differ; rationale).
- `docs/07_implementation_status.md` updated for `TASK-248` (delegate).
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC; PASS.
- Confirm `cargo test -p slicer-model-io` green; AC-7 normalization test green.
- Peak context usage under 70%.
