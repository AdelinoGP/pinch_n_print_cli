# Implementation Plan: 94_host-mesh-segmentation-wiring

## Execution Rules

- One atomic step at a time. Each step ends with `cargo check --workspace --all-targets` clean (once deletions begin).
- All `cargo test` / `pnp_cli slice` invocations prefixed with `mkdir -p target &&`.
- Test output teed to `target/test-output.log` per `CLAUDE.md` §Test Discipline.
- Step 0 (baseline capture) is already complete. AC-5 reads `P93_BASELINE_SHA` from closure-log; the value is `AA4DA2FAECA139F2C17909051497D6998F71BFB8A2DD9856D286296252EF1E3B`.
- Steps 1-5 are the actual deletion work; Step 6 is the regression + cube SHA capture; Step 7 is the negative-case sweep + guest WASM check; Step 8 is docs; Step 9 is the acceptance ceremony.

## Steps

### Step 0: Capture pre-packet baseline + create closure-log ✅ DONE

- Task IDs: `TASK-244`
- Objective: AC-5 prerequisite — record the wedge SHA that the post-packet slice must match.
- Precondition: working tree clean.
- Postcondition: `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` exists with `P93_BASELINE_SHA=<hex>` line + a header paragraph documenting the TASK-250 supersession rationale.
- **Status: DONE. Captured SHA: `AA4DA2FAECA139F2C17909051497D6998F71BFB8A2DD9856D286296252EF1E3B`.**
- Files changed: `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (created).

### Step 1: Pre-deletion inventory ✅ DONE

- Task IDs: `TASK-244`
- Objective: catch every reference to the symbols being deleted, so the post-deletion sweep is meaningful.
- **Status: DONE. Inventory results:**
  - `execute_mesh_segmentation` / `MeshSegmentationError` / `DegenerateStrokeReason`: kernel file (`crates/slicer-core/src/algos/mesh_segmentation.rs`), re-export (`crates/slicer-runtime/src/lib.rs:193-195`), kernel unit test (`crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs`), executor test (`crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs`). No production source uses any of the symbols.
  - `mesh_segmentation_executor`: `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` (file content) + `crates/slicer-runtime/tests/executor/main.rs:28` (mod declaration). No other references.
  - `mesh_segmentation` (the `pub mod` declaration): only `crates/slicer-core/src/algos/mod.rs:4`.
- **Conclusion**: the host path is fully orphaned. 5 files / lines are the entire deletion surface (3 file deletions + 2 line drops in mod files).

### Step 2: Delete kernel + kernel unit tests + executor test + mod declarations

- Task IDs: `TASK-244`
- Objective: AC-1 (3 of 4 parts — kernel, kernel unit test, executor test deleted; mod lines dropped).
- Precondition: Step 1 inventory complete.
- Postcondition: three files gone, two mod lines dropped; workspace will fail to compile (the kernel unit test and executor test reference deleted symbols via the re-export).
- Files allowed to read: `crates/slicer-core/src/algos/mod.rs` (12 lines; read in full OK) and `crates/slicer-runtime/tests/executor/main.rs` (40 lines; read in full OK).
- Files allowed to edit (5 edits; multi-commit acceptable):
  - DELETE `crates/slicer-core/src/algos/mesh_segmentation.rs` (545 lines; never load).
  - DELETE `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` (79 lines; never load).
  - DELETE `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` (185 lines; never load).
  - `crates/slicer-core/src/algos/mod.rs` — drop line 4: `pub mod mesh_segmentation;`.
  - `crates/slicer-runtime/tests/executor/main.rs` — drop line 28: `mod mesh_segmentation_executor_tdd;`.
- Files out-of-bounds: `crates/slicer-runtime/src/lib.rs` (Step 3); production source outside the listed files.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-core --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail with first error" — purpose: confirm the kernel deletion didn't break `slicer-core` standalone (it should — the kernel had no internal callers).
  - "Run `mkdir -p target && cargo check -p slicer-runtime --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail with first error" — purpose: confirm the runtime test compilation is broken (expected — the re-export is still there but the test files that used the kernel are gone; this should be a clean PASS because the runtime itself doesn't use the kernel, only the re-export is still in place).
- Context cost: `S`.
- Verification: `cargo check -p slicer-core --all-targets` and `cargo check -p slicer-runtime --all-targets` both pass after the deletions (the kernel + 2 test files are self-contained).
- Exit condition: three files gone + two mod lines dropped; per-crate checks clean.

### Step 3: Drop the `mesh_segmentation` re-export from `slicer-runtime/src/lib.rs`

- Task IDs: `TASK-244`
- Objective: AC-1 (final part — re-export block removed).
- Precondition: Step 2 complete.
- Postcondition: `crates/slicer-runtime/src/lib.rs` no longer carries the `pub use slicer_core::algos::mesh_segmentation::{...}` block. The `mesh_analysis` and `paint_segmentation` re-exports stay untouched.
- Files allowed to read:
  - `crates/slicer-runtime/src/lib.rs` — range-read at lines 189-200 only.
- Files allowed to edit (1 edit):
  - `crates/slicer-runtime/src/lib.rs` (lines 193-195: delete the 3-line `pub use slicer_core::algos::mesh_segmentation::{...}` block).
- Files out-of-bounds: production source outside `crates/slicer-runtime/src/lib.rs`; docs (Step 8).
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: workspace gate (AC-3).
  - "Run `mkdir -p target && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: lint gate (AC-2).
- Context cost: `S`.
- Verification: workspace check + clippy clean (AC-2 + AC-3 satisfied at this point).
- Exit condition: AC-1 + AC-2 + AC-3 satisfied.

### Step 4: AC-4 workspace test gate

- Task IDs: `TASK-244`
- Objective: AC-4 — workspace tests green after the deletions.
- Precondition: Step 3 complete.
- Postcondition: every test bucket reports `test result: ok`; net test count delta is non-positive.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files out-of-bounds: any.
- Expected dispatches:
  - "Run `mkdir -p target && cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`; return FACT per-bucket counts (e.g., `unit: 12 passed, 0 failed`)" — purpose: AC-4.
- Context cost: `S`.
- Verification: all 5 test buckets pass; no `FAILED` lines in the log.
- Exit condition: AC-4 satisfied.

### Step 5: AC-5 wedge byte-identical regression

- Task IDs: `TASK-244`
- Objective: AC-5 — wedge produces byte-identical g-code vs the P93 baseline.
- Precondition: Step 4 complete.
- Postcondition: wedge SHA matches `P93_BASELINE_SHA` in closure-log; AC-5 acceptance command exits 0.
- Files allowed to read: none.
- Files allowed to edit: none.
- Files out-of-bounds: any.
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p94-wedge.gcode && test "$(sha256sum target/p94-wedge.gcode | awk '{print tolower($1)}')" = "$(grep -oE 'P93_BASELINE_SHA=[a-fA-F0-9]+' .ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md | head -1 | cut -d= -f2 | tr 'A-Z' 'a-z')"`; return FACT exit 0/non-0" — purpose: AC-5 byte-identical regression check (case-normalized on both sides — closure-log uses uppercase, sha256sum emits lowercase).
- Context cost: `S`.
- Verification: AC-5 acceptance command exits 0.
- Exit condition: AC-5 satisfied.

### Step 6: AC-6 cube slice + SHA capture into closure-log

- Task IDs: `TASK-244`
- Objective: AC-6 — `cube_4color.3mf` slices to completion; capture `P94R_POST_CUBE_SHA`.
- Precondition: Step 5 complete.
- Postcondition: cube_4color slice completes with exit 0 and a non-empty g-code; closure-log contains `P94R_POST_CUBE_SHA=<hex>` line.
- Files allowed to read: none.
- Files allowed to edit (1 edit):
  - `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (append `P94R_POST_CUBE_SHA=<hex>` after the `P93_BASELINE_SHA` line).
- Files out-of-bounds: any source.
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output target/p94-cube.gcode && test -s target/p94-cube.gcode && sha256sum target/p94-cube.gcode | awk '{print $1}'`; return FACT (sha256 hash)" — purpose: AC-6 cube SHA capture.
- Context cost: `S`.
- Verification: cube_4color slice completes; closure-log contains the new SHA line.
- Exit condition: AC-6 satisfied.

### Step 7: AC-7 + AC-N1 + AC-N2 + AC-N3 negative-case gates

- Task IDs: `TASK-244`
- Objective: AC-7 (guest WASM `--check` clean) + AC-N1 (zero surviving references) + AC-N2 (WASM-guest infrastructure stays) + AC-N3 (loader stroke path intact).
- Precondition: Step 6 complete.
- Postcondition: all four AC commands return PASS.
- Files allowed to read: none (all dispatches are pure shell).
- Files allowed to edit: none.
- Files out-of-bounds: any.
- Expected dispatches:
  - "Run `cargo xtask build-guests --check`; return FACT pass/fail" — purpose: AC-7.
  - "Run `rg -n --glob '!.ralph/specs/94_host-mesh-segmentation-wiring/**' --glob '!docs/specs/paint-pipeline-orca-parity-roadmap.md' --glob '!docs/07_implementation_status.md' 'execute_mesh_segmentation|MeshSegmentationError|DegenerateStrokeReason|mesh_segmentation_executor' crates/ modules/ docs/ ; test $? -eq 1`; return FACT pass/fail (exit 1 == no matches == pass)" — purpose: AC-N1.
  - "Run `test -d modules/core-modules/mesh-segmentation && test -f modules/core-modules/mesh-segmentation/mesh-segmentation.toml && test -f modules/core-modules/mesh-segmentation/mesh-segmentation.wasm && rg -q 'PrepassStageOutput::MeshSegmentation' crates/slicer-runtime/src/prepass.rs && rg -q 'BlackboardPrepassSlot::MeshSegmentation' crates/slicer-runtime/src/blackboard.rs && rg -q 'MeshSegmentationIR' crates/slicer-runtime/src/blackboard.rs`; return FACT pass/fail" — purpose: AC-N2.
  - "Run `rg -q 'fn split_triangle_strokes|fn walk_triangle_selector_strokes' crates/slicer-model-io/src/loader.rs`; return FACT pass/fail" — purpose: AC-N3.
- Context cost: `S`.
- Verification: all four dispatches return PASS.
- Exit condition: AC-7 + AC-N1 + AC-N2 + AC-N3 satisfied.

### Step 8: AC-8 docs/07 TASK-244 row update + roadmap §P2 supersession note

- Task IDs: `TASK-244`
- Objective: AC-8 — TASK-244 row reflects the retirement; roadmap §P2 has the supersession paragraph.
- Precondition: Step 7 complete.
- Postcondition: TASK-244 row updated; roadmap §P2 has the addendum.
- Files allowed to read:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` — range-read §P2 only.
- Files allowed to edit (2 edits):
  - `docs/07_implementation_status.md` (delegated edit; never load the full backlog).
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` (append one paragraph at the end of §P2).
- Files out-of-bounds: any source.
- Expected dispatches:
  - "Locate the TASK-244 row in `docs/07_implementation_status.md` (use rg with line numbers to find it without loading the full file) and rewrite it to: `[x] TASK-244 — Retired. The execute_mesh_segmentation host kernel was orphaned (zero callers in production source per pre-implementation inventory); deleted in packet 94. The loader's split_triangle_strokes (loader.rs:1900-1961) is the canonical TriangleSelector normalization site; P95 will consume PaintLayer.strokes directly. Closed 2026-06-10.`; return FACT pass/fail" — purpose: AC-8 delegated edit.
  - "Append a one-paragraph TASK-250 supersession note to the end of §P2 in `docs/specs/paint-pipeline-orca-parity-roadmap.md` documenting that the host kernel was orphaned and the loader's split_triangle_strokes is the canonical normalization site; return FACT pass/fail" — purpose: roadmap addendum.
  - "Run `rg -q 'TASK-244.*retired|TASK-244.*superseded|TASK-244.*deleted|TASK-244.*orphan' docs/07_implementation_status.md`; return FACT pass/fail" — purpose: AC-8 grep gate.
- Context cost: `S`.
- Verification: AC-8 grep gate PASS; roadmap addendum present.
- Exit condition: AC-8 satisfied.

### Step 9: Final acceptance ceremony + status flip

- Task IDs: `TASK-244`
- Objective: re-dispatch every AC; confirm all PASS; flip status.
- Precondition: Steps 0-8 complete.
- Postcondition: AC-1 through AC-8 + AC-N1 through AC-N3 all PASS; `packet.spec.md` frontmatter flipped to `status: implemented`.
- Files allowed to read: none.
- Files allowed to edit (1 edit):
  - `.ralph/specs/94_host-mesh-segmentation-wiring/packet.spec.md` (status flip only; append a `## Deviations` block if any in-flight deviations were discovered during Steps 2-8).
- Files out-of-bounds: any.
- Expected dispatches:
  - Re-dispatch every AC command from `packet.spec.md`; confirm each PASS.
  - Final `cargo clippy --workspace --all-targets -- -D warnings` and `cargo check --workspace --all-targets` runs.
- Context cost: `S`.
- Verification: every AC PASS.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baseline capture + closure-log creation. **DONE.** |
| Step 1 | S | Pre-deletion inventory. **DONE.** |
| Step 2 | S | Three file deletions + two mod line drops. |
| Step 3 | S | Drop re-export (3-line edit) + workspace gate. |
| Step 4 | S | Workspace test gate. |
| Step 5 | S | Wedge byte-identical regression. |
| Step 6 | S | Cube slice + SHA capture. |
| Step 7 | S | Guest WASM check + AC-N1/N2/N3 sweep. |
| Step 8 | S | docs/07 TASK-244 row update + roadmap §P2 addendum. |
| Step 9 | S | Acceptance ceremony + status flip. |

Aggregate: `S` (no L step; no M step; the deletion is mechanical).

## Packet Completion Gate

- All 10 steps complete; each step's exit condition satisfied.
- AC-1 through AC-8 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: `P93_BASELINE_SHA`, `P94R_POST_CUBE_SHA`, and the TASK-250 supersession rationale paragraph.
- `docs/07_implementation_status.md` TASK-244 row reflects the retirement.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §P2 has the supersession addendum.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`; confirm each PASS.
- Confirm `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo xtask build-guests --check` all green via sub-agent FACT.
- Confirm byte-identical g-code on `regression_wedge.stl` (AC-5).
- Confirm `cube_4color.3mf` slices to completion (AC-6); record `P94R_POST_CUBE_SHA` for P95.
- Peak context usage under 70%.
