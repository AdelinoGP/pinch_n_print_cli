# Requirements: 94_host-mesh-segmentation-wiring

## Packet Metadata

- Grouped task IDs:
  - `TASK-244` — Wire the existing `execute_mesh_segmentation` host kernel into the prepass driver as a new `PrePass::MeshSegmentation` stage.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2 — host:mesh_segmentation kernel wiring"
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`crates/slicer-core/src/algos/mesh_segmentation.rs` contains a correct, unit-tested host kernel (lines 39-109) that normalizes sub-facet hex strokes from `paint_data.layers[*].strokes` into whole-triangle `paint_data.layers[*].facet_values` splits, performing TriangleSelector-style subdivision when a stroke covers only part of a triangle. The kernel is dead code — the prepass driver in `crates/slicer-runtime/src/prepass.rs` never invokes it. As a result, every painted mesh that uses sub-facet strokes (the OrcaSlicer-canonical paint-data encoding) leaks un-normalized strokes through every downstream stage. The current paint-segmentation kernel silently drops strokes (it only reads `facet_values`), producing wrong paint assignments on any non-vertical painted facet that was hex-encoded as a stroke. The cherry-picked `cube_4color.3mf` and `cube_fuzzyPainted.3mf` fixtures encode their per-face paints as sub-facet strokes — exactly the case the existing kernel was written to handle.

The fix is wiring, not algorithm work:

1. **Add `Blackboard::replace_mesh`** — a sibling of the existing `replace_slice_ir` at `blackboard.rs:276-290`. Same shape: `debug_assert!` no Tier 2 outputs committed, `MissingRequiredPrepass` guard, atomic `Arc` swap. Without this method, the prepass driver has no contract-checked way to swap the mesh after committing the initial one.
2. **Add `MESH_SEGMENTATION_PRODUCER` constant** — sibling of `MESH_ANALYSIS_PRODUCER` at `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs`. Identifies the stage to the scheduler and DAG validator.
3. **Insert prepass driver hook** — at the very first position in the prepass sequence (BEFORE `host:mesh_analysis`), guarded by `has_subfacet_strokes(mesh)` short-circuit so unpainted meshes pay zero cost.
4. **`PrepassExecutionError::MeshSegmentation` variant** — for `?`-propagation of the kernel's error type.
5. **`required_slots` table entry** — `PrePass::MeshSegmentation` has no prerequisites.

Behavior change is bounded: unpainted meshes see no change at all (short-circuit fires). Painted meshes now produce different downstream behavior — specifically, the strokes that were previously silently dropped are now respected by every downstream stage. The closure log captures the pre/post g-code SHA on `cube_4color.3mf` to make the bounded behavior change visible and traceable.

The WASM `modules/core-modules/mesh-segmentation/` core-module (the dead "guest can override mesh-segmentation" path) is NOT deleted in this packet — P5a (97) does that, with a 97-file blast radius the roadmap calls out. During this packet's life both paths coexist; the host built-in claims `PrePass::MeshSegmentation` because no WASM module declares it for this stage (the WASM module's stage declaration mismatches the new host-stage name by design).

## In Scope

- Add `Blackboard::replace_mesh(&mut self, new_mesh: Arc<MeshIR>) -> Result<(), BlackboardError>` in `crates/slicer-runtime/src/blackboard.rs` (sibling of `replace_slice_ir`).
- Create `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` with `MESH_SEGMENTATION_PRODUCER` constant matching the shape in the AC-2 spec.
- Add `pub mod mesh_segmentation_producer;` to `crates/slicer-runtime/src/builtins/mod.rs`.
- Insert `run_builtin_stage(...PrePass::MeshSegmentation, host:mesh_segmentation...)` at `crates/slicer-runtime/src/prepass.rs:374` BEFORE the existing `host:mesh_analysis` invocation.
- Add `has_subfacet_strokes(mesh: &MeshIR) -> bool` helper (or use an equivalent existing helper if one is found; locate via Grep).
- Add `PrepassExecutionError::MeshSegmentation { source: MeshSegmentationError }` variant with `From` impl or `#[from]` derive.
- Extend `required_slots(StageId)` table at `prepass.rs:680-708` with `"PrePass::MeshSegmentation" => &[]`.
- Add integration tests covering: short-circuit on unpainted mesh; stroke consumption on cube_4color; stroke consumption on cube_fuzzyPainted; determinism (same mesh → byte-equal normalized mesh); behavior on unpainted regression_wedge.stl (byte-identical g-code).
- Add Blackboard unit test for `replace_mesh` reject-after-Tier-2 (AC-N1).

## Out of Scope

- The mesh-segmentation kernel itself (already correct and tested).
- Deletion of the WASM `mesh-segmentation` core-module — P5a (97).
- Paint-segmentation kernel changes — P3 (95).
- Region-mapping changes — P1c (93).
- Doc updates to `docs/04_host_scheduler.md` — P5c (99).
- Any change to `pnp_cli`.

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

- Positive cases: `AC-1` through `AC-13`. Refinements:
  - The `has_subfacet_strokes` short-circuit (AC-5) is the only thing that keeps unpainted meshes from paying any overhead. Skipping it would slow every unpainted slice by ~50ms (TriangleSelector scan time).
  - AC-12's g-code-diff bound is documentation-only (the SHA capture is automated but the rationale paragraph is human-written). The pre-packet SHA on `cube_4color.3mf` is captured in Step 0; the post-packet SHA in Step 9.
- Negative cases: `AC-N1` (Blackboard tier guard), `AC-N2` (kernel no-op on unpainted), `AC-N3` (no longer dead code).
- Cross-packet impact: unblocks P3 (paint-segmentation can now assume strokes are normalized away). Provides the precedent for P3's `replace_slice_ir`-style mesh swap.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Compiles | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings | FACT pass/fail |
| `cargo test -p slicer-core --test algo_mesh_segmentation_tdd 2>&1 \| tee target/test-output.log` | Kernel still passes | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 \| tee target/test-output.log` | AC-5, AC-6, AC-7, AC-8 — integration tests | FACT pass/fail with breakdown |
| `cargo test -p slicer-runtime blackboard_replace_mesh 2>&1 \| tee target/test-output.log` | AC-1, AC-N1 — Blackboard tests | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor cube_4color_paint_tdd 2>&1 \| tee target/test-output.log` | Cube paint tests (regression check; pass count vs. P93 baseline) | FACT pass-count + fail-count |
| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p94-wedge.gcode && sha256sum /tmp/p94-wedge.gcode` | AC-11 — unpainted byte-identical | FACT (sha256) |
| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p94-cube.gcode && sha256sum /tmp/p94-cube.gcode` | AC-12 — painted SHA captured for closure log | FACT (sha256) |
| `cargo xtask build-guests --check` | AC-13 — guest clean | FACT pass/fail |

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
