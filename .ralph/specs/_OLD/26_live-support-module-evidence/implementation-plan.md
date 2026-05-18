# Implementation Plan: 26_live-support-module-evidence

## Step 1 — Read existing fixtures and manifests

**Task IDs**: TASK-120b
**Objective**: Read `modules/core-modules/tree-support/tree-support.toml`, `modules/core-modules/traditional-support/traditional-support.toml`, `crates/slicer-host/tests/live_seam_path_tdd.rs` (for the loading pattern), and `crates/slicer-host/tests/dispatch_tdd.rs` (for paint helpers) before writing new tests.
**Precondition**: None.
**Postcondition**: Known: accepted JSON config keys from tree-support.toml; loading pattern from live_seam_path_tdd.rs; paint helpers from dispatch_tdd.rs.
**Files**: `modules/core-modules/tree-support/tree-support.toml`, `modules/core-modules/traditional-support/traditional-support.toml`, `crates/slicer-host/tests/live_seam_path_tdd.rs`, `crates/slicer-host/tests/dispatch_tdd.rs`
**Verification**: Files readable; no code changes.
**Exit**: Context confirmed.
**OrcaSlicer refs**: None.

## Step 2 — Split `live_support_generation_tdd.rs` into tiers

**Task IDs**: TASK-120b
**Objective**: Add a clear comment barrier in `live_support_generation_tdd.rs` separating existing commit-path tests (keep as-is) from new real live-dispatch tests (add). Mark the existing tests explicitly as commit-path tests.
**Precondition**: Step 1 complete.
**Postcondition**: Existing tests annotated as "commit-path tier"; new section header "real live-dispatch tier" inserted.
**Files**: `crates/slicer-host/tests/live_support_generation_tdd.rs`
**Verification**: `grep -n 'commit-path\|live-dispatch' crates/slicer-host/tests/live_support_generation_tdd.rs | head -10`
**Exit**: File has two clearly labeled tiers.
**OrcaSlicer refs**: None.

## Step 3 — Add real `tree-support.wasm` live-dispatch test

**Task IDs**: TASK-120b
**Objective**: Add `tree_support_live_dispatch_produces_non_empty_support_ir` that loads `tree-support.wasm` via `WasmInstancePool`, dispatches `Layer::Support` via `WasmRuntimeDispatcher::dispatch_layer_call`, and asserts `SupportIR.support_paths` is non-empty with `SupportMaterial` roles.
**Precondition**: Steps 1–2 complete.
**Postcondition**: New test passes.
**Files**: `crates/slicer-host/tests/live_support_generation_tdd.rs`
**Verification**: `cargo test -p slicer-host --test live_support_generation_tdd tree_support_live_dispatch -- --nocapture 2>&1 | tail -20`
**Exit**: Test passes.
**OrcaSlicer refs**: None.

## Step 4 — Add real `traditional-support.wasm` live-dispatch test

**Task IDs**: TASK-120b
**Objective**: Add `traditional_support_live_dispatch_produces_non_empty_support_ir` with the same pattern for traditional-support.
**Precondition**: Steps 1–3 complete.
**Postcondition**: New test passes.
**Files**: `crates/slicer-host/tests/live_support_generation_tdd.rs`
**Verification**: `cargo test -p slicer-host --test live_support_generation_tdd traditional_support_live_dispatch -- --nocapture 2>&1 | tail -20`
**Exit**: Test passes.
**OrcaSlicer refs**: None.

## Step 5 — Add support determinism test

**Task IDs**: TASK-120b
**Objective**: Add `support_deterministic_across_repeated_runs` that runs an identical `Layer::Support` dispatch twice and asserts byte-identical `SupportIR.support_paths` output.
**Precondition**: Steps 3–4 complete.
**Postcondition**: Determinism test passes.
**Files**: `crates/slicer-host/tests/live_support_generation_tdd.rs`
**Verification**: `cargo test -p slicer-host --test live_support_generation_tdd support_deterministic -- --nocapture 2>&1 | tail -20`
**Exit**: Test passes.
**OrcaSlicer refs**: None.

## Step 6 — Optionally add SupportEnforcer/SupportBlocker paint precedence test

**Task IDs**: TASK-120b
**Objective**: If `dispatch_tdd.rs` has `PaintRegionIR` helpers for enforcer/blocker, add one live paint precedence case. If not available without extra harness work, omit this step.
**Precondition**: Step 1 confirmed helpers availability.
**Postcondition**: Test passes or step is skipped with justification comment.
**Files**: `crates/slicer-host/tests/live_support_generation_tdd.rs`
**Verification**: `cargo test -p slicer-host --test live_support_generation_tdd support_enforcer_blocker -- --nocapture 2>&1 | tail -10`
**Exit**: Test passes or is justifiedly omitted.
**OrcaSlicer refs**: None.

## Step 7 — Extend `run_slicer_host` helper with `--config` support

**Task IDs**: TASK-120
**Objective**: Add `config: Option<&Path>` parameter to `run_slicer_host` and conditionally append `--config path` to the CLI invocation.
**Precondition**: None.
**Postcondition**: `run_slicer_host(model, module_dir, output, Some(config_path))` passes `--config config_path` to the binary.
**Files**: `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
**Verification**: `grep -n 'config' crates/slicer-host/tests/benchy_end_to_end_tdd.rs | grep -i 'run_slicer_host\|--config' | head -10`
**Exit**: Helper accepts optional config path; build passes.
**OrcaSlicer refs**: None.

## Step 8 — Create JSON config fixture for tree-support

**Task IDs**: TASK-120
**Objective**: Create `resources/test_config/benchy-tree-support.json` with `support_enabled: true` and concrete tree-support tuning values. Keys must match those in `tree-support.toml`.
**Precondition**: Step 1 confirmed accepted keys.
**Postcondition**: JSON fixture file exists at `resources/test_config/benchy-tree-support.json` with valid tree-support config.
**Files**: `resources/test_config/benchy-tree-support.json`
**Verification**: `grep -E 'support_enabled|support_module|support_type' resources/test_config/benchy-tree-support.json | head -10`
**Exit**: Fixture created.
**OrcaSlicer refs**: None.

## Step 9 — Add filtered module-dir builder for tree-support

**Task IDs**: TASK-120
**Objective**: Add `filtered_module_dir_for_tree_support(tmp: &tempfile::TempDir) -> PathBuf` helper that copies all core-modules except `traditional-support.wasm` and `traditional-support.toml`.
**Precondition**: Step 1 confirmed traditional-support module ID.
**Postcondition**: Helper function exists and produces a directory where tree-support is the active support-generator holder.
**Files**: `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
**Verification**: `grep -n 'filtered_module_dir_for_tree_support' crates/slicer-host/tests/benchy_end_to_end_tdd.rs | head -5`
**Exit**: Helper implemented; build passes.
**OrcaSlicer refs**: None.

## Step 10 — Add support-enabled Benchy acceptance tests

**Task IDs**: TASK-120, TASK-120b
**Objective**: Add three new tests:
- `benchy_with_support_enabled` — runs Benchy with tree-support filtered dir and JSON config, asserts binary exits 0 and `.gcode` non-empty
- `benchy_support_marker_present` — asserts `.gcode` contains `;TYPE:Support` or `;TYPE:Support interface`
- `benchy_support_deterministic` — runs identical command twice, asserts byte-identical output
**Precondition**: Steps 7–9 complete.
**Postcondition**: All three new tests pass.
**Files**: `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`
**Verification**: `cargo test -p slicer-host --test benchy_end_to_end_tdd benchy_with_support benchy_support benchy_support_deterministic -- --nocapture 2>&1 | tail -20`
**Exit**: All three tests pass.
**OrcaSlicer refs**: None.

## Step 11 — Update `docs/07_implementation_status.md` TASK-120b status

**Task IDs**: TASK-120b
**Objective**: Update the TASK-120b entry in `docs/07_implementation_status.md` to cite the new real live support-module evidence (tree-support.wasm + traditional-support.wasm live dispatch tests) instead of the old synthetic commit-helper tests.
**Precondition**: Steps 3–5 complete.
**Postcondition**: TASK-120b entry cites the new real evidence.
**Files**: `docs/07_implementation_status.md`
**Verification**: `grep -A10 'TASK-120b' docs/07_implementation_status.md | head -15`
**Exit**: Entry updated.
**OrcaSlicer refs**: None.

## Step 12 — Packet completion gate

**Objective**: Run the focused test matrix for Packet 26 and confirm workspace build/clippy.
**Precondition**: Steps 1–11 complete.
**Postcondition**: `cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture` passes; `cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture` passes; `cargo build --workspace` exits 0; `cargo clippy --workspace -- -D warnings` exits 0.
**Files**: All changed files.
**Verification**:
```
cargo test -p slicer-host --test live_support_generation_tdd -- --nocapture 2>&1 | tail -5
cargo test -p slicer-host --test benchy_end_to_end_tdd -- --nocapture 2>&1 | tail -5
cargo build --workspace 2>&1 | tail -3
cargo clippy --workspace -- -D warnings 2>&1 | tail -3
```
**Exit**: All four commands succeed.
**OrcaSlicer refs**: None.
