---
status: implemented
packet: 94
task_ids: [TASK-244]
---

# 94_host-mesh-segmentation-wiring

## Goal

Retire the orphaned host kernel `execute_mesh_segmentation` in `crates/slicer-core/src/algos/mesh_segmentation.rs`. Pre-implementation inventory (Step 1) confirmed the host path has zero callers in production source: the only references to `execute_mesh_segmentation` / `MeshSegmentationError` / `DegenerateStrokeReason` are (a) the kernel file itself, (b) the re-export in `crates/slicer-runtime/src/lib.rs:193-195`, and (c) two test files (`algo_mesh_segmentation_tdd.rs` + `mesh_segmentation_executor_tdd.rs`) that test the kernel directly with hand-built `MeshIR` fixtures. The kernel was never wired into a prepass stage, never given a `Blackboard::replace_mesh` handoff, and never given a `MESH_SEGMENTATION_PRODUCER` constant — TASK-244's prior P94 framing described a wiring that was never landed. This packet executes the smaller, real retirement: delete the kernel + its two test files + the re-export, leaving the loader's `split_triangle_strokes` (loader.rs:1900-1961) as the canonical TriangleSelector normalization path forward. The WASM-guest infrastructure (`PrepassStageOutput::MeshSegmentation`, `BlackboardPrepassSlot::MeshSegmentation`, `commit_mesh_segmentation`, `MeshSegmentationIR`, `mesh-segmentation.toml`, the `modules/core-modules/mesh-segmentation/` directory) is **P97's territory** and stays untouched.

## Problem Statement

The original TASK-244 framing described wiring `execute_mesh_segmentation` into a `PrePass::MeshSegmentation` host stage with a `Blackboard::replace_mesh` handoff and a `MESH_SEGMENTATION_PRODUCER` constant. The pre-implementation inventory (Step 1) confirmed this wiring was **never landed**: the only references to `execute_mesh_segmentation` / `MeshSegmentationError` / `DegenerateStrokeReason` in the current tree are (a) the kernel file itself, (b) the re-export in `crates/slicer-runtime/src/lib.rs:193-195`, and (c) two test files (`algo_mesh_segmentation_tdd.rs` + `mesh_segmentation_executor_tdd.rs`) that test the kernel directly with hand-built `MeshIR` fixtures.

The TASK-250 architectural finding, restated against the smaller reality:

1. **The host kernel is fully orphaned.** No production source code calls `execute_mesh_segmentation`; the prepass driver does not invoke it; the blackboard has no `replace_mesh` method. The kernel is dead code waiting for a consumer that was never specified.
2. **The kernel's clean-bisection template is structurally incompatible with OrcaSlicer-pattern leaves.** 12+ `TangentToFacetEdge` raise sites in `crates/slicer-core/src/algos/mesh_segmentation.rs` confirm the kernel was never going to work on real `.3mf` paint data.
3. **The loader already does the work a host stage would do.** `crates/slicer-model-io/src/loader.rs:1900-1961` implements `split_triangle_strokes` + `walk_triangle_selector_strokes`, reproducing OrcaSlicer's `TriangleSelector` recursive subdivision and emitting `PaintLayer.strokes` in OrcaSlicer's flat-leaf form at the load boundary.
4. **P95 (parity doc Phase 3) reads `PaintLayer.strokes` directly via `collect_facets()`.** No host-side post-load normalization stage is needed.

The retirement is the right architectural call. The kernel is orphaned, structurally broken, and has no consumer. This packet executes the deletion of the kernel + its re-export + its two direct-call test files, leaving the loader's `split_triangle_strokes` as the canonical TriangleSelector normalization path forward. The WASM-guest infrastructure (`PrepassStageOutput::MeshSegmentation`, `BlackboardPrepassSlot::MeshSegmentation`, `commit_mesh_segmentation`, `MeshSegmentationIR`, `mesh-segmentation.toml`, the `modules/core-modules/mesh-segmentation/` directory) is **P97's territory** and stays untouched.

## Architecture Constraints

- The retirement is purely subtractive. No new code lands; only deletions + re-export removal. The post-packet workspace builds, passes clippy + tests, and the wedge SHA matches the recorded P93 baseline.
- The host kernel is fully orphaned. The pre-implementation inventory (Step 1) confirmed zero callers in production source: no prepass driver invocation, no blackboard handoff, no `Blackboard::replace_mesh` (does not exist), no `MESH_SEGMENTATION_PRODUCER` constant (does not exist). The kernel is dead code.
- The loader's `split_triangle_strokes` + `walk_triangle_selector_strokes` (loader.rs:1900-1961) is the canonical TriangleSelector normalization site. Do not touch it. P95 will consume `PaintLayer.strokes` directly per the parity doc's Phase 3 `collect_facets()` design.
- The cube_4color SHA captured in Step 6 (`P94R_POST_CUBE_SHA`) becomes P95's input baseline — P95 will compare against this for its byte-identical regression contract.
- The WASM-guest `mesh-segmentation` core-module infrastructure stays untouched: `modules/core-modules/mesh-segmentation/mesh-segmentation.toml` (active), `mesh-segmentation.wasm` artifact, `PrepassStageOutput::MeshSegmentation` in `crates/slicer-runtime/src/prepass.rs:280`, `BlackboardPrepassSlot::MeshSegmentation` + `commit_mesh_segmentation` in `crates/slicer-runtime/src/blackboard.rs:158-174`, `MeshSegmentationIR` in `crates/slicer-ir/src/stage_io.rs:285`. P97 handles the full directory deletion; do NOT touch any of this.

## Data and Contract Notes

- IR contracts touched: none. The deleted kernel was internal to `slicer-core`; the deleted re-export was an internal `slicer-runtime` re-export (3 lines). No public IR shape changes.
- WIT boundary considerations: none. The WASM-guest `mesh-segmentation-output` resource still exists (P97 deletes it); this packet does not touch WIT.
- Determinism or scheduler constraints: the wedge byte-identical contract (AC-5) confirms the deletion is purely subtractive. The cube_4color SHA (AC-6) captures the new baseline for P95.

## Locked Assumptions and Invariants

- **Wedge byte-identical contract**: `P93_BASELINE_SHA` (captured at Step 0; value = `AA4DA2FAECA139F2C17909051497D6998F71BFB8A2DD9856D286296252EF1E3B`) equals the post-P94 wedge SHA (computed in AC-5's verification command). Any drift is investigated, not waved off — the wedge has no painted strokes, so the deleted kernel was a no-op for it.
- **The host kernel is fully orphaned**: pre-implementation inventory confirmed zero callers in production source. The kernel, its re-export, and its two test files are the only places these symbols exist. Deletion cannot break a production code path.
- **The loader is the normalization site**: `split_triangle_strokes` + `walk_triangle_selector_strokes` at `loader.rs:1900-1961` reproduce OrcaSlicer's TriangleSelector recursive subdivision and emit `PaintLayer.strokes` in OrcaSlicer's flat-leaf form. P95 consumes this directly. NO host-side post-load normalization stage exists post-packet.
- **WASM-guest infrastructure stays**: `mesh-segmentation.toml`, `mesh-segmentation.wasm`, `PrepassStageOutput::MeshSegmentation`, `BlackboardPrepassSlot::MeshSegmentation`, `commit_mesh_segmentation`, `MeshSegmentationIR`, and the `modules/core-modules/mesh-segmentation/` directory are P97's territory. This packet does not touch any of them.

## Risks and Tradeoffs

- **Risk: a downstream consumer references the deleted kernel that the pre-deletion grep missed.** Mitigation: AC-N1 sweeps the workspace post-deletion; `cargo check --workspace --all-targets` is the cheapest compile-time falsifier; AC-4's workspace test gate catches runtime consumers.
- **Risk: the kernel re-export was depended on by an external consumer (downstream `pnp_cli` or test).** Mitigation: pre-implementation inventory confirmed only the two test files use the symbols; AC-N1 sweeps post-deletion. If a downstream user surfaces, they'd be re-adding the symbol to their own tree.
- **Risk: cube_4color slicing still fails for an unrelated reason (a downstream stage that the kernel was inadvertently masking).** Mitigation: AC-6 asserts the slice completes to a non-empty g-code. If the slice fails post-deletion, the failure is informative — surface the new error to the user before completing the packet.
- **Tradeoff: archival vs. fresh-start closure-log.** The closure-log records `P93_BASELINE_SHA` (kept from prior baseline) + `P94R_POST_CUBE_SHA` (new) + the TASK-250 rationale paragraph. The honest record of the supersession lives in the rationale paragraph appended to the closure-log + the §P2 roadmap addendum + the AC-8 docs/07 row update.
