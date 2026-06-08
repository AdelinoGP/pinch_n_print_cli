# Implementation Plan: 96_paint-segmentation-phase5-width-limit

## Execution Rules

- One step at a time.
- All `cargo test` runs prefixed with `mkdir -p target &&` so the tee target exists.
- Test output teed to `target/test-output.log`.

## Steps

### Step 0: Capture pre-packet baselines (wedge + cube default-config SHAs)

- Task IDs: `TASK-246`
- Objective: AC-8 prerequisite — both must match post-packet SHAs.
- Precondition: P95 closed.
- Postcondition: 2 SHAs recorded.
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-baseline-wedge.gcode && sha256sum /tmp/p96-baseline-wedge.gcode`; return FACT".
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-baseline-cube.gcode && sha256sum /tmp/p96-baseline-cube.gcode`; return FACT".
- Context cost: `S`.
- Exit condition: SHAs recorded.

### Step 1: Locate config-schema landing site + summarize Phase 5 spec

- Task IDs: `TASK-246`
- Objective: pinpoint where the three config keys go + understand the algorithm.
- Precondition: Step 0 complete.
- Postcondition: file:line for schema location + algorithm SUMMARY in implementer notes.
- Expected dispatches:
  - "Locate the config-schema declaration block for paint-segmentation (either in host config schema OR in `modules/core-modules/paint-segmentation-default/<name>.toml` if that module exists post-P95); return FILE:LINE + 5-line context".
  - "Summarize `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` Phase 5 section (`cut_segmented_layers`); return SUMMARY ≤ 150 words covering the even/odd alternation + the constant-beam mode".
  - "Open `crates/slicer-core/src/algos/paint_segmentation/mod.rs` and return SNIPPETS (≤ 30 lines) showing where `compose_variants` is called and where the final `replace_slice_ir` commit happens".
- Context cost: `S`.
- Verification: all three dispatches return.
- Exit condition: inventory recorded.

### Step 2: Implement `cut_segmented_layers` kernel + 3 positive + 3 negative unit tests

- Task IDs: `TASK-246`
- Objective: AC-1, AC-N1, AC-N2, AC-N3.
- Precondition: Step 1 complete.
- Postcondition: kernel exists; 6 unit tests pass.
- Files allowed to read:
  - `crates/slicer-core/src/polygon_ops.rs` — `offset` + `difference_ex` signatures.
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/width_limit.rs` (NEW).
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (export the new module via `pub mod width_limit;`).
- Files out-of-bounds: any other sub-module.
- Expected dispatches:
  - "Run `mkdir -p target && cargo test -p slicer-core paint_segmentation::width_limit 2>&1 | tee target/test-output.log`; FACT pass/fail with per-test breakdown".
- Context cost: `M`.
- Authoritative docs: roadmap §"P4", spec §3 Phase 5.
- OrcaSlicer refs: SUMMARY from Step 1 dispatch.
- Verification: 6 tests pass.
- Exit condition: AC-1, AC-N1, AC-N2, AC-N3 satisfied.

### Step 3: Add config-schema entries for the three keys

- Task IDs: `TASK-246`
- Objective: AC-3.
- Precondition: Step 2 green; Step 1 has identified the schema location.
- Postcondition: three keys exist in the schema TOML with documented defaults.
- Files allowed to edit (≤ 3):
  - The schema location TOML (host config OR module manifest).
- Files out-of-bounds: any other.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `S`.
- Verification: workspace check clean; AC-3 grep checks return PASS.
- Exit condition: AC-3 satisfied.

### Step 4: Integrate `cut_segmented_layers` into `execute_paint_segmentation_v2`

- Task IDs: `TASK-246`
- Objective: AC-2, AC-4.
- Precondition: Step 3 green.
- Postcondition: pipeline reads config keys via `config_for`, calls Phase 5 after Phase 7.
- Files allowed to read:
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (range-read around the existing call sites).
- Files allowed to edit (≤ 3):
  - `crates/slicer-core/src/algos/paint_segmentation/mod.rs`.
- Files out-of-bounds: any other paint-segmentation sub-module.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `S`.
- Verification: workspace check clean.
- Exit condition: AC-2, AC-4 satisfied.

### Step 5: Add 3 integration tests (band width, interlocking alternation, interlocking-beam constancy)

- Task IDs: `TASK-246`
- Objective: AC-5, AC-6, AC-7.
- Precondition: Step 4 green.
- Postcondition: 3 integration tests pass.
- Files allowed to read:
  - `crates/slicer-runtime/tests/common/` (fixture helpers).
- Files allowed to edit (≤ 3 per commit; multi-commit):
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_width_limit_bands_tdd.rs` (NEW).
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_interlocking_alternates_tdd.rs` (NEW).
  - `crates/slicer-runtime/tests/executor/cube_4color_phase5_interlocking_beam_constant_tdd.rs` (NEW).
  - Optionally `resources/cube_4color_tall.3mf` if needed (≤ 100 KB).
- Files out-of-bounds: kernel.
- Expected dispatches:
  - "Determine whether `resources/cube_4color.3mf` produces ≥ 30 mm tall geometry at default layer height; return FACT (height in mm or layer count)" — purpose: decide if `cube_4color_tall.3mf` authoring is required.
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_phase5 2>&1 | tee target/test-output.log`; FACT pass/fail".
- Context cost: `M`.
- Verification: 3 integration tests pass.
- Exit condition: AC-5, AC-6, AC-7 satisfied.

### Step 6: Regression checks — AC-8 (wedge + cube default-config byte-identical) + AC-10 (cube test suites still GREEN)

- Task IDs: `TASK-246`
- Objective: AC-8, AC-10.
- Precondition: Step 5 green.
- Postcondition: wedge SHA matches Step 0; cube SHA matches Step 0 (default config); 24 cube tests still GREEN.
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p96-post-wedge.gcode && sha256sum /tmp/p96-post-wedge.gcode`; FACT".
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-post-cube.gcode && sha256sum /tmp/p96-post-cube.gcode`; FACT".
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 | tee target/test-output.log`; FACT (must show 12/12)".
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor cube_fuzzy_painted_tdd 2>&1 | tee target/test-output.log`; FACT (must show 12/12)".
- Context cost: `S`.
- Verification: 2 SHAs match + 24/24 GREEN.
- Exit condition: AC-8, AC-10 satisfied.

### Step 7: Visual report capture (AC-9)

- Task IDs: `TASK-246`
- Objective: AC-9.
- Precondition: Step 6 green.
- Postcondition: HTML report file exists; closure log notes layer ID + visual confirmation.
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p96-cube-banded.gcode --report /tmp/p96-cube-banded-report.html && test -f /tmp/p96-cube-banded-report.html`; FACT pass/fail".
- Context cost: `S`.
- Verification: file exists.
- Exit condition: AC-9 satisfied (closure-log visual confirmation is a human step).

### Step 8: Guest WASM `--check`

- Task IDs: `TASK-246`
- Objective: AC-11.
- Expected dispatches:
  - "Run `cargo xtask build-guests --check`; FACT pass/fail".
- Context cost: `S`.
- Exit condition: AC-11 satisfied.

### Step 9: Final acceptance ceremony

- Task IDs: `TASK-246`
- Objective: final gate.
- Expected dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; FACT".
  - "Run `mkdir -p target && cargo test -p slicer-core paint_segmentation 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `S`.
- Verification: all PASS.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baselines. |
| Step 1 | S | Schema location + algo SUMMARY. |
| Step 2 | M | Kernel + 6 tests. |
| Step 3 | S | Config-schema entries. |
| Step 4 | S | Integration call. |
| Step 5 | M | 3 integration tests. |
| Step 6 | S | Regression. |
| Step 7 | S | Visual report. |
| Step 8 | S | Guest check. |
| Step 9 | S | Workspace gate. |

Aggregate: M.

## Packet Completion Gate

- All 10 steps complete.
- AC-1 through AC-11 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: pre/post wedge SHAs (match), pre/post cube SHAs (match), 12/12 + 12/12 cube test counts, visual-banding confirmation, the chosen config-schema location.
- `docs/07_implementation_status.md` updated for `TASK-246` (delegate).
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC; PASS.
- Confirm 24 cube tests still GREEN.
- Confirm `cargo xtask build-guests --check` clean.
- Peak context usage under 70%.
