# Requirements: 94_host-mesh-segmentation-wiring

## Packet Metadata

- Grouped task IDs:
  - `TASK-244` — Wire the existing `execute_mesh_segmentation` host kernel into the prepass driver as a new `PrePass::MeshSegmentation` stage.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2 — host:mesh_segmentation kernel wiring"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`crates/slicer-core/src/algos/mesh_segmentation.rs` contains a correct, unit-tested host kernel (lines 39-109) that normalizes sub-facet hex strokes from `paint_data.layers[*].strokes` into whole-triangle `paint_data.layers[*].facet_values` splits, performing TriangleSelector-style subdivision when a stroke covers only part of a triangle. The kernel is dead code — the prepass driver in `crates/slicer-runtime/src/prepass.rs` never invokes it. As a result, every painted mesh that uses sub-facet strokes (the OrcaSlicer-canonical paint-data encoding) leaks un-normalized strokes through every downstream stage. The current paint-segmentation kernel silently drops strokes (it only reads `facet_values`), producing wrong paint assignments on any non-vertical painted facet that was hex-encoded as a stroke. The cherry-picked `cube_4color.3mf` and `cube_fuzzyPainted.3mf` fixtures encode their per-face paints as sub-facet strokes — exactly the case the existing kernel was written to handle.

The fix is wiring plus one minimal manifest edit:

1. **Add `Blackboard::replace_mesh`** — a sibling of the existing `replace_slice_ir` at `blackboard.rs:276-290`. Same shape: `debug_assert!` no Tier 2 outputs committed, atomic `Arc` swap, `Result` return preserved for symmetry. The verified fact that `Blackboard::mesh_ir: Arc<MeshIR>` is non-`Option` removes any need for a `MissingRequiredPrepass` guard on the mesh slot (and `BlackboardPrepassSlot::MeshIR` does not exist in the enum anyway).
2. **Add `MESH_SEGMENTATION_PRODUCER` constant** — sibling of `MESH_ANALYSIS_PRODUCER` at `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs`. Identifies the stage to the scheduler and DAG validator.
3. **Insert prepass driver hook** — at the very first position in the prepass sequence (BEFORE `host:mesh_analysis`), guarded by `has_subfacet_strokes(mesh)` short-circuit so unpainted meshes pay zero cost.
4. **`PrepassExecutionError::MeshSegmentation` variant** — for `?`-propagation of the kernel's error type.
5. **`required_slots` table entry** — `PrePass::MeshSegmentation` has no prerequisites.
6. **Disable the WASM `mesh-segmentation` manifest's stage claim** — the manifest at `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` currently declares `stage = "PrePass::MeshSegmentation"`. Without this packet's minimal edit, the DAG validator would see two producers (the new host built-in and the existing WASM module) claiming the same stage. The implementer picks the smallest viable mechanism: comment out the stage line, rename the manifest to `.disabled`, or use the loader's documented "disabled" pathway. The directory itself remains; P5a (97) still owns the full deletion of `modules/core-modules/mesh-segmentation/`.

Behavior change is bounded: unpainted meshes see no change at all (short-circuit fires; AC-11 byte-identical g-code gate). Painted meshes now produce different downstream behavior — specifically, the strokes that were previously silently dropped are now respected by every downstream stage. The closure log captures the pre/post g-code SHA on `cube_4color.3mf` to make the bounded behavior change visible and traceable.

## In Scope

- Add `Blackboard::replace_mesh(&mut self, new_mesh: Arc<MeshIR>) -> Result<(), BlackboardError>` in `crates/slicer-runtime/src/blackboard.rs` (sibling of `replace_slice_ir`).
- Create `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` with `MESH_SEGMENTATION_PRODUCER` constant matching the shape in the AC-2 spec.
- Add `pub mod mesh_segmentation_producer;` to `crates/slicer-runtime/src/builtins/mod.rs`.
- Insert `run_builtin_stage(...PrePass::MeshSegmentation, host:mesh_segmentation...)` at `crates/slicer-runtime/src/prepass.rs:374` BEFORE the existing `host:mesh_analysis` invocation.
- Add `has_subfacet_strokes(mesh: &MeshIR) -> bool` helper (or use an equivalent existing helper if one is found; locate via Grep).
- Add `PrepassExecutionError::MeshSegmentation { source: MeshSegmentationError }` variant with `From` impl or `#[from]` derive.
- Extend `required_slots(StageId)` table at `prepass.rs:680-708` with `"PrePass::MeshSegmentation" => &[]`.
- Apply the smallest-possible edit to `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` so it no longer registers `PrePass::MeshSegmentation` as a producer stage (AC-3.5). Mechanism: implementer's choice (comment / rename / loader-disabled-pathway). Triggers a guest rebuild via `cargo xtask build-guests` before AC-13's `--check` reports clean.
- Add integration tests covering: short-circuit on unpainted mesh; stroke consumption on cube_4color; stroke consumption on cube_fuzzyPainted; determinism (same mesh → byte-equal normalized mesh); behavior on unpainted regression_wedge.stl (byte-identical g-code).
- Add Blackboard unit test for `replace_mesh` reject-after-Tier-2 (AC-N1) and a contract unit test for `PrepassExecutionError::MeshSegmentation` variant construction + `?`-propagation (AC-10).

## Out of Scope

- The mesh-segmentation kernel itself (already correct and tested).
- Deletion of the `modules/core-modules/mesh-segmentation/` directory or its source files — P5a (97). P94's edit is one-line minimal: only the manifest's stage claim is disabled.
- Paint-segmentation kernel changes — P3 (95).
- Region-mapping changes — P1c (93).
- Doc updates to `docs/04_host_scheduler.md` — P5c (99).
- Any change to `pnp_cli`.
- Widening `BlackboardError` with a `TierViolation` variant. `replace_mesh` uses `debug_assert!`-only Tier-2 guards, matching `replace_slice_ir`'s established contract.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2" (read directly).
- `docs/04_host_scheduler.md` §"PrePass" stage prerequisites — read the table only (range-read).
- `crates/slicer-runtime/src/blackboard.rs` lines 270-310 (range-read; locate `replace_slice_ir`).
- `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs` full (47 LOC).
- `crates/slicer-core/src/algos/mesh_segmentation.rs` — range-read kernel signature + `MeshSegmentationError` definition (lines 39-110 + the error-type block).
- `crates/slicer-runtime/src/prepass.rs` — range-read lines 374-400 (insertion site) and lines 680-720 (`required_slots` table).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` or `SUMMARY` (≤ 200 words).

Files to inspect for this packet:

- None directly. The kernel parity is already established by the unit tests in `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs`. If a question arises about TriangleSelector subdivision behavior, delegate against `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp`.

## Acceptance Summary

- Positive cases: `AC-1`, `AC-2`, `AC-3`, `AC-3.5`, `AC-4` through `AC-11`, `AC-13`. Refinements:
  - The `has_subfacet_strokes` short-circuit (AC-5) is the only thing that keeps unpainted meshes from paying any overhead. Skipping it would slow every unpainted slice by ~50ms (TriangleSelector scan time).
  - The painted `cube_4color.3mf` g-code SHA capture (previously AC-12) is now a closure-log obligation under `packet.spec.md` §Doc Impact Statement, not an AC. The pre-packet SHA is written in Step 0; the post-packet SHA in Step 7; the one-paragraph rationale is human-authored before packet close. This is documentation, not a machine gate.
- Negative cases: `AC-N1` (debug-build `debug_assert!` panic on Tier-2 violation), `AC-N2` (kernel no-op on unpainted), `AC-N3` (no longer dead code).
- Cross-packet impact: unblocks P3 (paint-segmentation can now assume strokes are normalized away). Provides the precedent for P3's `replace_slice_ir`-style mesh swap.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo test -p slicer-core --test algo_mesh_segmentation_tdd 2>&1 \| tee target/test-output.log` | Kernel still passes (regression sanity) | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 \| tee target/test-output.log` | AC-5, AC-6, AC-7, AC-8 — integration tests | FACT pass/fail with breakdown |
| `cargo test -p slicer-runtime --test contract blackboard_replace_mesh 2>&1 \| tee target/test-output.log` | AC-1, AC-N1 — Blackboard contract tests | FACT pass/fail |
| `cargo test -p slicer-runtime --test contract prepass_execution_error_mesh_segmentation_variant 2>&1 \| tee target/test-output.log` | AC-10 — variant constructs + `?`-propagates | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 \| tee target/test-output.log` | Cube paint tests (regression check; pass count vs. P93 baseline) | FACT pass-count + fail-count |
| `! rg -q '^id\s*=\s*"PrePass::MeshSegmentation"' modules/core-modules/mesh-segmentation/mesh-segmentation.toml` | AC-3.5 — WASM manifest no longer claims the host stage (verified field shape: nested `id` under `[stage]`, not a top-level `stage = …` line) | FACT pass/fail |
| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p94-wedge.gcode && test "$(sha256sum /tmp/p94-wedge.gcode \| awk '{print $1}')" = "$(grep -oE 'P93_BASELINE_SHA=[a-f0-9]+' .ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md \| head -1 \| cut -d= -f2)"` | AC-11 — unpainted baseline-compare against closure-log SHA | FACT pass/fail |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p94-cube.gcode && sha256sum /tmp/p94-cube.gcode` | Closure-log SHA capture for painted cube (documentation; not an AC) | FACT (sha256) |
| `cargo xtask build-guests --check` | AC-13 — guest clean (requires rebuild after AC-3.5 manifest edit) | FACT pass/fail |

## Step Completion Expectations

- `Blackboard::replace_mesh` (Step 1) MUST land before the producer constant + prepass wiring (Steps 2-3), otherwise the prepass driver doesn't compile.
- The driver insertion (Step 3) is the most error-prone — inserting BEFORE `host:mesh_analysis` rather than AFTER changes the DAG order and may surface latent ordering assumptions in `host:mesh_analysis`. Test AC-5 catches the short-circuit case; AC-6/7 catch the painted case.
- The two cube fixtures (AC-6, AC-7) exercise different paint semantics (`material` vs `fuzzy_skin`) — both must pass to confirm the kernel doesn't have semantic-specific bugs.
- AC-12 (post-packet cube SHA) is documentation; the SHA changes are EXPECTED (downstream stages now see normalized facet_values). The closure log paragraph explaining the diff is the deliverable.

## Context Discipline Notes

- `crates/slicer-runtime/src/prepass.rs` may be > 700 lines. The two regions of interest (driver around line 374, table around line 680) are < 30 lines each. Range-read.
- `crates/slicer-runtime/src/blackboard.rs` is probably ~400 lines. Range-read around lines 270-310 (replace_slice_ir as template).
- `crates/slicer-core/src/algos/mesh_segmentation.rs` is the kernel — DO NOT edit it. Read only the signature + error type (the kernel body is irrelevant to wiring).
- The MWE binary 3MF fixtures (cube_4color.3mf, cube_fuzzyPainted.3mf) are binary; never `Read`. Test code consumes them via loader.
