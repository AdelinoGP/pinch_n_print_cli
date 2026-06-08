# Implementation Plan: 97_wasm-mesh-segmentation-deletion

## Execution Rules

- One step at a time.
- WIT-first; then handlers; then macro; then runtime; then IR; then tests + scaffolder + bench; then guest rebuild; then gates.
- All `cargo test` and `pnp_cli` runs prefixed with `mkdir -p target &&`.

## Steps

### Step 0: Capture pre-packet baselines (wedge + cube SHAs)

- Task IDs: `TASK-247`
- Objective: AC-17 prerequisite.
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p97-baseline-wedge.gcode && sha256sum /tmp/p97-baseline-wedge.gcode`; FACT".
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p97-baseline-cube.gcode && sha256sum /tmp/p97-baseline-cube.gcode`; FACT".
- Context cost: `S`.
- Exit condition: 2 SHAs recorded.

### Step 1: Pre-deletion inventory — locate every surviving reference

- Task IDs: `TASK-247`
- Objective: full inventory before any deletion.
- Expected dispatches:
  - "Run `rg -nE 'mesh_segmentation|MeshSegmentation|mesh-segmentation-output|run-mesh-segmentation|FacetPaintMark|MeshSegmentationIR|PrepassStageOutput::MeshSegmentation|BlackboardPrepassSlot::MeshSegmentation' crates/ modules/`; return LOCATIONS (cap 100 entries) PLUS a per-file count table for completeness".
  - "Run `! rg -q 'PrePass::MeshSegmentation' modules/core-modules/*/<name>.toml`; FACT" — confirm no module declares the stage.
- Context cost: `S`.
- Exit condition: inventory recorded; allow-list of survivors planned.

### Step 2: Delete WIT — `mesh-segmentation-output` resource + `run-mesh-segmentation` export

- Task IDs: `TASK-247`
- Objective: AC-2.
- Files allowed to edit (≤ 3):
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`.
- Expected dispatches:
  - "Run `! rg -q 'mesh-segmentation-output|run-mesh-segmentation' crates/slicer-schema/wit/`; FACT".
- Context cost: `S`.
- Exit condition: AC-2 satisfied.

### Step 3: Delete WASM-host handlers — `host.rs` + `dispatch.rs`

- Task IDs: `TASK-247`
- Objective: AC-3, AC-4.
- Files allowed to edit (≤ 3):
  - `crates/slicer-wasm-host/src/host.rs` — ranged edits at lines 767, 1042-1043, 3588-3622.
  - `crates/slicer-wasm-host/src/dispatch.rs` — ranged edits at lines 818, 1700-1727, 1906-1908.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-wasm-host 2>&1 | tee target/test-output.log`; FACT with first error".
- Context cost: `M`.
- Exit condition: AC-3, AC-4 satisfied; slicer-wasm-host compiles.

### Step 4: Delete macro arm — `slicer-macros/src/lib.rs:452, 1439-1480`

- Task IDs: `TASK-247`
- Objective: AC-5 (partial).
- Files allowed to edit (≤ 3):
  - `crates/slicer-macros/src/lib.rs`.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-macros 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `S`.
- Exit condition: macro arm gone; compiles.

### Step 5: Delete runtime — `blackboard.rs` `commit_mesh_segmentation` + `mesh_segmentation()` accessor; `prepass.rs` dispatcher-output handling + `BlackboardPrepassSlot::MeshSegmentation`

- Task IDs: `TASK-247`
- Objective: AC-6, AC-7.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/blackboard.rs` — ranged at lines 159-172.
  - `crates/slicer-runtime/src/prepass.rs` — ranged at lines 280, 656, 730.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-runtime 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `M`.
- Exit condition: AC-6, AC-7 satisfied.

### Step 6: Delete IR types — `FacetPaintMark`, `MeshSegmentationIR`, schema constant, `PrepassStageOutput::MeshSegmentation`

- Task IDs: `TASK-247`
- Objective: AC-8.
- Files allowed to edit (≤ 3):
  - `crates/slicer-ir/src/slice_ir.rs` — ranged at 238, 1053-1086.
  - `crates/slicer-ir/src/stage_io.rs` — ranged at 30-31, 262-onward.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `M`.
- Exit condition: AC-8 satisfied; workspace compiles.

### Step 7: Delete module directory + macro-roundtrip + integration-geometry tests; rewire executor test

- Task IDs: `TASK-247`
- Objective: AC-1, AC-9.
- Files allowed to read:
  - `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` — full read (small).
- Files allowed to edit (≤ 3 per commit; multi-commit):
  - DELETE `modules/core-modules/mesh-segmentation/` (entire directory).
  - DELETE `crates/slicer-runtime/tests/contract/macro_mesh_segmentation_output_roundtrip_tdd.rs`.
  - DELETE `crates/slicer-runtime/tests/integration/macro_mesh_segmentation_geometry_tdd.rs`.
  - EDIT `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` (drop WASM-roundtrip harness; test host built-in path).
- Expected dispatches:
  - "Run `mkdir -p target && cargo test -p slicer-runtime --test executor mesh_segmentation_executor 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `M`.
- Exit condition: AC-1, AC-9 satisfied.

### Step 8: Drop dispatch contract arms + scaffolder + scheduler-test tables + bench entry

- Task IDs: `TASK-247`
- Objective: AC-10, AC-11, AC-12, AC-13.
- Files allowed to edit (≤ 3 per commit; batch):
  - `crates/slicer-runtime/tests/contract/dispatch_tdd.rs` (ranged at 282, 4771-5074, 6187).
  - `crates/pnp-cli/src/module_new.rs` (ranged at 388, 521, 569, 571, 681).
  - `crates/pnp-cli/tests/module_new_tdd.rs` (line 136).
  - `crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs` (43, 233).
  - `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs` (line 653).
  - `crates/slicer-runtime/benches/wasm_modules.rs` (line 89).
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; FACT".
- Context cost: `M`.
- Exit condition: AC-10, AC-11, AC-12, AC-13 satisfied.

### Step 9: Guest WASM rebuild + `--check`

- Task IDs: `TASK-247`
- Objective: AC-5 (complete), AC-16.
- Expected dispatches:
  - "Run `cargo xtask build-guests && cargo xtask build-guests --check`; FACT pass/fail".
- Context cost: `S`.
- Exit condition: AC-5, AC-16 satisfied.

### Step 10: AC-14 surviving-reference audit (manual review against allow-list)

- Task IDs: `TASK-247`
- Objective: AC-14.
- Expected dispatches:
  - "Run `rg -nE 'mesh_segmentation|MeshSegmentation' crates/ modules/`; return LOCATIONS (cap 100 entries)" — purpose: enumerate survivors.
- Manual: review LOCATIONS against AC-14's allow-list. Each survivor must be on the list.
- Context cost: `S`.
- Exit condition: AC-14 satisfied (no unintended survivor).

### Step 11: AC-17 byte-identical g-code

- Task IDs: `TASK-247`
- Expected dispatches:
  - "Run wedge slice + sha256sum; FACT".
  - "Run cube_4color slice + sha256sum; FACT".
- Verification: SHAs match Step 0 baselines.
- Context cost: `S`.
- Exit condition: AC-17 satisfied.

### Step 12: Workspace gate

- Task IDs: `TASK-247`
- Expected dispatches:
  - "Run `cargo clippy --workspace --all-targets -- -D warnings`; FACT".
  - "Run `mkdir -p target && cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`; FACT".
- Context cost: `S`.
- Exit condition: AC-15 satisfied; packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost |
| --- | --- |
| Step 0 | S |
| Step 1 | S |
| Step 2 | S |
| Step 3 | M |
| Step 4 | S |
| Step 5 | M |
| Step 6 | M |
| Step 7 | M |
| Step 8 | M |
| Step 9 | S |
| Step 10 | S |
| Step 11 | S |
| Step 12 | S |

Aggregate: M.

## Packet Completion Gate

- All 13 steps complete.
- AC-1 through AC-17 + AC-N1, AC-N2 verified.
- Closure log records: pre/post wedge + cube SHAs (match), the AC-14 surviving-references manual review with each survivor justified.
- `docs/07_implementation_status.md` updated for `TASK-247` (delegate).
- `packet.spec.md` to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every AC; PASS.
- Byte-identical g-code on both fixtures (AC-17).
- Workspace tests + clippy + guest check all PASS.
- Peak context usage under 70%.
