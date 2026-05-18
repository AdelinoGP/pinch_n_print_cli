# Implementation Plan: 25_wit-canonical-surface-lock

## Step 1 — Audit current disk WIT file content

**Task IDs**: TASK-144, TASK-145
**Objective**: Read `wit/world-prepass.wit` and `wit/deps/ir-types.wit` to confirm current state before making changes. Record what signatures and members are present.
**Precondition**: None.
**Postcondition**: Current content of both files is known; missing or drifted members are identified.
**Files**: `wit/world-prepass.wit`, `wit/deps/ir-types.wit`
**Verification**: `grep -E 'run-mesh-segmentation|run-paint-segmentation|mesh-object-view|paint-segmentation-object-view' wit/world-prepass.wit | head -10`
**Exit**: File content confirmed; drift list available.
**OrcaSlicer refs**: None.

## Step 2 — Update `wit/world-prepass.wit` segmentation signatures

**Task IDs**: TASK-144, TASK-145
**Objective**: Update `run-mesh-segmentation` and `run-paint-segmentation` in the disk canonical to use `mesh-object-view` and `paint-segmentation-object-view` respectively.
**Precondition**: Step 1 confirmed the drift.
**Postcondition**: Disk `wit/world-prepass.wit` uses `mesh-object-view` for mesh segmentation and `paint-segmentation-object-view` for paint segmentation.
**Files**: `wit/world-prepass.wit`
**Verification**: `grep -A3 'run-mesh-segmentation' wit/world-prepass.wit | head -10`
**Exit**: Both function signatures updated in disk file.
**OrcaSlicer refs**: None.

## Step 3 — Add seam members to `wit/deps/ir-types.wit`

**Task IDs**: TASK-144, TASK-145
**Objective**: Check `wit/deps/ir-types.wit` for existing seam-related members. Add missing `push-reordered-wall-loop`, `push-resolved-seam` to `perimeter-output-builder` and `resolved-seam` to `perimeter-region-view`.
**Precondition**: Step 1 confirmed which members are missing.
**Postcondition**: `wit/deps/ir-types.wit` contains all seam-related members: `resolved-seam` (read) on perimeter-region-view, `push-reordered-wall-loop` and `push-resolved-seam` (write) on perimeter-output-builder.
**Files**: `wit/deps/ir-types.wit`
**Verification**: `grep -E 'resolved-seam|push-reordered-wall-loop|push-resolved-seam' wit/deps/ir-types.wit | head -10`
**Exit**: All three members present in disk file.
**OrcaSlicer refs**: None.

## Step 4 — Expand `wit_drift_detection_tdd.rs` with specific signature assertions

**Task IDs**: TASK-145
**Objective**: Add new assertion blocks or new test functions that assert on the specific members that slipped through: `mesh-object-view` in the prepass world, `paint-segmentation-object-view` in the prepass world, `resolved-seam` in perimeter-region-view, `push-reordered-wall-loop` and `push-resolved-seam` in perimeter-output-builder.
**Precondition**: Steps 2 and 3 complete.
**Postcondition**: `wit_drift_detection_tdd.rs` includes assertions for all the specific drift cases.
**Files**: `crates/slicer-host/tests/wit_drift_detection_tdd.rs`
**Verification**: `cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture 2>&1 | tail -10`
**Exit**: All drift detection tests pass including the new assertions.
**OrcaSlicer refs**: None.

## Step 5 — Update `docs/03_wit_and_manifest.md` perimeter sections

**Task IDs**: TASK-144, TASK-145
**Objective**: Update the perimeter-region-view section to list `resolved-seam` as a readable field. Update the perimeter-output-builder section to list `push-wall-loop`, `push-reordered-wall-loop`, and `push-resolved-seam` as builder methods.
**Precondition**: Steps 2 and 3 complete; disk files are synchronized.
**Postcondition**: `docs/03_wit_and_manifest.md` reflects the current WIT interface for seam-related members.
**Files**: `docs/03_wit_and_manifest.md`
**Verification**: `grep -E 'resolved-seam|push-reordered-wall-loop|push-resolved-seam' docs/03_wit_and_manifest.md | head -10`
**Exit**: Doc updated; section content matches disk WIT.
**OrcaSlicer refs**: None.

## Step 6 — Rebuild WASM artifacts if bindings changed

**Task IDs**: TASK-144
**Objective**: If the WIT changes affect guest bindings, rebuild the core-modules WASM tree using `build-core-modules.sh`.
**Precondition**: Steps 2–5 complete.
**Postcondition**: WASM artifacts are up to date with the new WIT surface.
**Files**: `modules/core-modules/build-core-modules.sh`
**Verification**: `./modules/core-modules/build-core-modules.sh 2>&1 | tail -5; echo "EXIT: $?"`
**Exit**: Build exits 0.
**OrcaSlicer refs**: None.

## Step 7 — Packet completion gate

**Objective**: Run the focused test matrix for Packet 25 and confirm workspace build/clippy.
**Precondition**: Steps 1–6 complete.
**Postcondition**: `cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture` passes; `cargo build --workspace` exits 0; `cargo clippy --workspace -- -D warnings` exits 0 with no warnings.
**Files**: All changed files.
**Verification**:
```
cargo test -p slicer-host --test wit_drift_detection_tdd -- --nocapture 2>&1 | tail -5
cargo build --workspace 2>&1 | tail -3
cargo clippy --workspace -- -D warnings 2>&1 | tail -3
```
**Exit**: All three commands succeed.
**OrcaSlicer refs**: None.
