# Implementation Plan: 94_host-mesh-segmentation-wiring

## Execution Rules

- One atomic step at a time. Each step ends with `cargo check --workspace --all-targets` clean.
- All `cargo test` / `pnp_cli slice` invocations prefixed with `mkdir -p target &&`.
- Test output teed to `target/test-output.log` per `CLAUDE.md` §Test Discipline.
- Step 0 (baseline capture) MUST land before any deletion. AC-6 reads `P93_BASELINE_SHA` from closure-log; without the closure-log line the gate cannot pass.

## Steps

### Step 0: Capture pre-packet baseline + create closure-log

- Task IDs: `TASK-244`
- Objective: AC-6 prerequisite — record the wedge SHA that the post-packet slice must match.
- Precondition: working tree clean. Prior P94 implementation commits (3113083 + 89b3517) present in local history.
- Postcondition: `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` exists with `P93_BASELINE_SHA=<hex>` line + a header paragraph documenting the TASK-250 supersession rationale.
- Files allowed to read: none.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (NEW).
- Files out-of-bounds: any.
- Expected dispatches:
  - "Run `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p94-baseline-wedge.gcode && sha256sum target/p94-baseline-wedge.gcode | awk '{print $1}'`; return FACT (sha256)" — captures `P93_BASELINE_SHA`.
- Context cost: `S`.
- Verification: closure-log contains the `P93_BASELINE_SHA=...` line.
- Exit condition: SHA recorded.

### Step 1: Pre-deletion inventory

- Task IDs: `TASK-244`
- Objective: catch every reference to the symbols being deleted, so the post-deletion sweep is meaningful.
- Precondition: Step 0 complete.
- Postcondition: LOCATIONS list of every reference recorded in implementer's notes.
- Files allowed to read: none directly.
- Files allowed to edit: none.
- Files out-of-bounds: any.
- Expected dispatches:
  - "Run `rg -nE 'execute_mesh_segmentation\|MESH_SEGMENTATION_PRODUCER\|MeshSegmentationError\|host:mesh_segmentation\|PrePass::MeshSegmentation\|replace_mesh\|has_subfacet_strokes\|BlackboardPrepassSlot::MeshSegmentation' crates/ modules/ docs/`; return LOCATIONS (≤ 60 entries) PLUS a per-file count summary if total > 60. NOT scoped to test files — production source first" — purpose: inventory.
- Context cost: `S`.
- Authoritative docs: none.
- OrcaSlicer refs: none.
- Verification: inventory recorded; the implementer can name every file that needs editing.
- Exit condition: inventory complete.

### Step 2: Delete the kernel + kernel unit tests + mod declaration

- Task IDs: `TASK-244`
- Objective: AC-1 (partial — kernel + tests deleted, mod declaration dropped).
- Precondition: Step 1 inventory complete.
- Postcondition: three files gone, one line dropped from `mod.rs`; `cargo check -p slicer-core --all-targets` reports failures from external consumers (host built-in + tests) — those are the next steps' targets.
- Files allowed to read: none (the file being deleted does not need to be re-read).
- Files allowed to edit (≤ 3):
  - DELETE `crates/slicer-core/src/algos/mesh_segmentation.rs`.
  - DELETE `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs`.
  - `crates/slicer-core/src/algos/mod.rs` (drop the `pub mod mesh_segmentation;` line).
- Files out-of-bounds: production source outside `crates/slicer-core/src/algos/`; the producer constant (Step 3); the prepass driver (Step 4).
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-core --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail with first error" — purpose: confirm the slicer-core crate compiles after the kernel deletion (it should — the kernel had no internal callers; only external).
- Context cost: `S`.
- Verification: `slicer-core` compiles standalone; workspace check will fail until Steps 3-6 land.
- Exit condition: three files gone + mod line dropped.

### Step 3: Delete the host built-in producer constant + mod declaration

- Task IDs: `TASK-244`
- Objective: AC-1 (partial).
- Precondition: Step 2 complete.
- Postcondition: producer constant file gone; mod declaration dropped.
- Files allowed to read: none.
- Files allowed to edit (≤ 3):
  - DELETE `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs`.
  - `crates/slicer-runtime/src/builtins/mod.rs` (drop the `pub mod mesh_segmentation_producer;` line).
- Files out-of-bounds: prepass driver (Step 4); blackboard (Step 5); tests (Step 6).
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-runtime --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail with first error" — purpose: locate the next downstream consumer (likely prepass.rs).
- Context cost: `S`.
- Verification: file gone; mod line dropped. The check is expected to FAIL with errors pointing at prepass.rs's MESH_SEGMENTATION_PRODUCER references — that's Step 4's target.
- Exit condition: producer file gone.

### Step 4: Revert prepass driver insertion + required_slots entry + error variant + has_subfacet_strokes helper

- Task IDs: `TASK-244`
- Objective: AC-2.
- Precondition: Steps 2-3 complete.
- Postcondition: prepass driver no longer references mesh-segmentation; `crates/slicer-runtime/src/prepass.rs` is clean.
- Files allowed to read:
  - `crates/slicer-runtime/src/prepass.rs` — ranged reads at the four deletion sites (driver block, required_slots entry, error variant, has_subfacet_strokes helper).
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/prepass.rs`.
- Files out-of-bounds: blackboard (Step 5); tests (Step 6).
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-runtime --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail with first error" — purpose: gate.
  - "Run `rg -nE 'has_subfacet_strokes' crates/slicer-runtime/src/`; return LOCATIONS" — purpose: confirm the helper has no other caller before deletion (resolves the `[FWD]` open question in design.md).
- Context cost: `S`.
- Verification: `cargo check -p slicer-runtime --all-targets` reports failures pointing at `replace_mesh` callers (Step 5's target) and test files (Step 6's target) — not at prepass.rs itself.
- Exit condition: AC-2 satisfied (prepass.rs clean of mesh-segmentation references).

### Step 5: Revert Blackboard::replace_mesh

- Task IDs: `TASK-244`
- Objective: AC-1 (final part).
- Precondition: Steps 2-4 complete.
- Postcondition: `crates/slicer-runtime/src/blackboard.rs` no longer carries `replace_mesh`; the only `replace_*` method is `replace_slice_ir` (the pre-P94 baseline).
- Files allowed to read:
  - `crates/slicer-runtime/src/blackboard.rs` — ranged read at the `replace_mesh` definition site.
- Files allowed to edit (≤ 3):
  - `crates/slicer-runtime/src/blackboard.rs` (delete the method + doc-comment block).
- Files out-of-bounds: tests (Step 6); docs (Step 7).
- Expected dispatches:
  - "Run `mkdir -p target && cargo check -p slicer-runtime --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: gate.
- Context cost: `S`.
- Verification: `cargo check -p slicer-runtime --all-targets` reports failures only in the six P94-introduced test files (Step 6's target).
- Exit condition: AC-1 fully satisfied.

### Step 6: Delete the six P94-introduced test files + their mod declarations

- Task IDs: `TASK-244`
- Objective: AC-3.
- Precondition: Step 5 complete.
- Postcondition: six test files gone; harness `main.rs` files clean.
- Files allowed to read:
  - `crates/slicer-runtime/tests/contract/main.rs` (≤ 50 lines).
  - `crates/slicer-runtime/tests/executor/main.rs` (≤ 100 lines).
- Files allowed to edit (≤ 8 deletions + 2 edits; multi-commit acceptable):
  - DELETE the six test files listed in `design.md` §Code Change Surface.
  - `crates/slicer-runtime/tests/contract/main.rs` (drop two `mod` lines).
  - `crates/slicer-runtime/tests/executor/main.rs` (drop four `mod` lines).
- Files out-of-bounds: any production source.
- Expected dispatches:
  - "Run `mkdir -p target && cargo check --workspace --all-targets 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: workspace gate.
  - "Run `mkdir -p target && cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log`; return FACT pass/fail" — purpose: lint gate.
- Context cost: `S`.
- Verification: workspace check + clippy clean (AC-4 satisfied at this point).
- Exit condition: AC-3 + AC-4 satisfied.

### Step 7: AC-5 + AC-6 + AC-7 regression + cube SHA capture

- Task IDs: `TASK-244`
- Objective: AC-5 (workspace tests green), AC-6 (wedge byte-identical), AC-7 (cube_4color slices end-to-end + SHA capture).
- Precondition: Steps 0-6 complete.
- Postcondition: workspace gates green; wedge SHA matches `P93_BASELINE_SHA`; cube SHA captured into closure-log as `P94R_POST_CUBE_SHA`.
- Files allowed to read: none.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (append `P94R_POST_CUBE_SHA=<hex>` line + paragraph documenting that cube_4color now slices end-to-end without the DegenerateStroke failure — this becomes P95's input baseline).
- Files out-of-bounds: any source.
- Expected dispatches:
  - "Run `mkdir -p target && cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`; return FACT per-bucket counts" — AC-5.
  - "Run the AC-6 baseline-compare command from `packet.spec.md` line 121; return FACT exit 0/non-0" — purpose: byte-identical regression check.
  - "Run the AC-7 cube slice command from `packet.spec.md`; return FACT (sha256 hash)" — purpose: cube SHA capture.
- Context cost: `S`.
- Verification: all three dispatches return PASS (or sha256 hash for AC-7).
- Exit condition: AC-5, AC-6, AC-7 satisfied.

### Step 8: Guest WASM `--check` + AC-N1 sweep + roadmap §P2 supersession note

- Task IDs: `TASK-244`
- Objective: AC-8 + AC-N1 + roadmap addendum.
- Precondition: Step 7 green.
- Postcondition: guest WASM clean; AC-N1 grep returns zero hits; roadmap §P2 has a one-paragraph supersession note appended.
- Files allowed to read:
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` — range-read §P2 (lines ~507-580) only.
- Files allowed to edit (≤ 3):
  - `docs/specs/paint-pipeline-orca-parity-roadmap.md` (append one paragraph at the end of §P2).
- Files out-of-bounds: any source.
- Expected dispatches:
  - "Run `cargo xtask build-guests --check`; return FACT pass/fail" — AC-8.
  - "Run the AC-N1 sweep command from `packet.spec.md`; return FACT pass/fail (exit 1 == no matches == pass)" — purpose: surviving-references gate.
- Context cost: `S`.
- Verification: AC-8 + AC-N1 PASS; roadmap addendum present.
- Exit condition: AC-8 + AC-N1 satisfied; roadmap updated.

### Step 9: Update TASK-244 row in docs/07_implementation_status.md

- Task IDs: `TASK-244`
- Objective: AC-9.
- Precondition: Step 8 complete.
- Postcondition: TASK-244 row updated to reflect the retirement supersession.
- Files allowed to read: none.
- Files allowed to edit (≤ 3):
  - `docs/07_implementation_status.md` (delegated edit only; never load the full backlog).
- Files out-of-bounds: any other file.
- Expected dispatches:
  - "Locate the TASK-244 row in `docs/07_implementation_status.md` (likely near line 218 per prior session memory) and rewrite it to: `[x] TASK-244 — Retired by TASK-250 architectural finding. The PrePass::MeshSegmentation host stage was wired but the kernel structurally fails on OrcaSlicer-pattern subdivision leaves; the loader's split_triangle_strokes (loader.rs:1900-1961) is the canonical TriangleSelector normalization site. Closed YYYY-MM-DD — packet 94.`; replace the date placeholder with today's date; return FACT pass/fail" — purpose: delegated edit.
- Context cost: `S`.
- Verification: AC-9 grep returns the new row content.
- Exit condition: AC-9 satisfied.

### Step 10: Final acceptance ceremony

- Task IDs: `TASK-244`
- Objective: re-dispatch every AC; confirm all PASS; flip status.
- Precondition: Steps 0-9 complete.
- Postcondition: AC-1 through AC-9 + AC-N1 through AC-N3 all PASS; `packet.spec.md` frontmatter flipped to `status: implemented`.
- Files allowed to read: none.
- Files allowed to edit (≤ 3):
  - `.ralph/specs/94_host-mesh-segmentation-wiring/packet.spec.md` (status flip only; append a `## Deviations` block if any in-flight deviations were discovered during Steps 1-9).
- Files out-of-bounds: any.
- Expected dispatches:
  - Re-dispatch every AC command from `packet.spec.md`; confirm each PASS.
  - Run `cargo clippy --workspace --all-targets -- -D warnings` and `cargo check --workspace --all-targets` as final gates.
- Context cost: `S`.
- Verification: every AC PASS.
- Exit condition: packet ready for `status: implemented`.

## Per-Step Budget Roll-Up

| Step | Context Cost | Notes |
| --- | --- | --- |
| Step 0 | S | Baseline capture + closure-log creation. |
| Step 1 | S | Pre-deletion inventory (pure dispatch). |
| Step 2 | S | Kernel + tests + mod declaration. |
| Step 3 | S | Producer constant + mod declaration. |
| Step 4 | S | Prepass driver revert (four narrow blocks in one file). |
| Step 5 | S | Blackboard::replace_mesh revert. |
| Step 6 | S | Six test file deletions + harness mod cleanup. |
| Step 7 | S | Workspace test + wedge baseline + cube SHA capture. |
| Step 8 | S | Guest WASM check + AC-N1 sweep + roadmap addendum. |
| Step 9 | S | docs/07 TASK-244 row update (delegated). |
| Step 10 | S | Acceptance ceremony + status flip. |

Aggregate: `S` (no L step; no M step; the deletion is mechanical).

## Packet Completion Gate

- All 11 steps complete; each step's exit condition satisfied.
- AC-1 through AC-9 + AC-N1, AC-N2, AC-N3 verified.
- Closure log records: `P93_BASELINE_SHA`, `P94R_POST_CUBE_SHA`, and the TASK-250 supersession rationale paragraph.
- `docs/07_implementation_status.md` TASK-244 row reflects the retirement.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §P2 has the supersession addendum.
- `packet.spec.md` ready to move to `status: implemented`.

## Acceptance Ceremony

- Re-dispatch every pipe-suffixed acceptance criterion command from `packet.spec.md`; confirm each PASS.
- Confirm `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo xtask build-guests --check` all green via sub-agent FACT.
- Confirm byte-identical g-code on `regression_wedge.stl` (AC-6).
- Confirm `cube_4color.3mf` slices to completion (AC-7); record `P94R_POST_CUBE_SHA` for P95.
- Peak context usage under 70%.
