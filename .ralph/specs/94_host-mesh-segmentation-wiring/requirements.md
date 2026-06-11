# Requirements: 94_host-mesh-segmentation-wiring

## Packet Metadata

- Grouped task IDs:
  - `TASK-244` — Retire the orphaned `execute_mesh_segmentation` host kernel. Closes the original TASK-244 framing via the TASK-250 architectural finding that the host kernel has zero callers in production source.
- Backlog source: `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2 — host:mesh_segmentation kernel wiring" (historical context); TASK-250 architectural finding is the proximate driver.
- Packet status: `active`
- Aggregate context cost: `S`

## Problem Statement

The original TASK-244 framing described wiring `execute_mesh_segmentation` into a `PrePass::MeshSegmentation` host stage with a `Blackboard::replace_mesh` handoff and a `MESH_SEGMENTATION_PRODUCER` constant. The pre-implementation inventory (Step 1) confirmed this wiring was **never landed**: the only references to `execute_mesh_segmentation` / `MeshSegmentationError` / `DegenerateStrokeReason` in the current tree are (a) the kernel file itself, (b) the re-export in `crates/slicer-runtime/src/lib.rs:193-195`, and (c) two test files (`algo_mesh_segmentation_tdd.rs` + `mesh_segmentation_executor_tdd.rs`) that test the kernel directly with hand-built `MeshIR` fixtures.

The TASK-250 architectural finding, restated against the smaller reality:

1. **The host kernel is fully orphaned.** No production source code calls `execute_mesh_segmentation`; the prepass driver does not invoke it; the blackboard has no `replace_mesh` method. The kernel is dead code waiting for a consumer that was never specified.
2. **The kernel's clean-bisection template is structurally incompatible with OrcaSlicer-pattern leaves.** 12+ `TangentToFacetEdge` raise sites in `crates/slicer-core/src/algos/mesh_segmentation.rs` confirm the kernel was never going to work on real `.3mf` paint data.
3. **The loader already does the work a host stage would do.** `crates/slicer-model-io/src/loader.rs:1900-1961` implements `split_triangle_strokes` + `walk_triangle_selector_strokes`, reproducing OrcaSlicer's `TriangleSelector` recursive subdivision and emitting `PaintLayer.strokes` in OrcaSlicer's flat-leaf form at the load boundary.
4. **P95 (parity doc Phase 3) reads `PaintLayer.strokes` directly via `collect_facets()`.** No host-side post-load normalization stage is needed.

The retirement is the right architectural call. The kernel is orphaned, structurally broken, and has no consumer. This packet executes the deletion of the kernel + its re-export + its two direct-call test files, leaving the loader's `split_triangle_strokes` as the canonical TriangleSelector normalization path forward. The WASM-guest infrastructure (`PrepassStageOutput::MeshSegmentation`, `BlackboardPrepassSlot::MeshSegmentation`, `commit_mesh_segmentation`, `MeshSegmentationIR`, `mesh-segmentation.toml`, the `modules/core-modules/mesh-segmentation/` directory) is **P97's territory** and stays untouched.

## In Scope

- DELETE `crates/slicer-core/src/algos/mesh_segmentation.rs` (host kernel; 545 lines).
- DELETE `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` (kernel unit tests; 79 lines).
- DELETE `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` (executor-scope direct-call test; 185 lines).
- DROP `pub mod mesh_segmentation;` from `crates/slicer-core/src/algos/mod.rs` (line 4).
- DROP the `pub use slicer_core::algos::mesh_segmentation::{...}` block from `crates/slicer-runtime/src/lib.rs:193-195` (3 lines; surrounding `mesh_analysis` and `paint_segmentation` re-exports stay).
- DROP `mod mesh_segmentation_executor_tdd;` from `crates/slicer-runtime/tests/executor/main.rs` (line 28).
- UPDATE `docs/07_implementation_status.md` TASK-244 row to reflect the retirement.
- APPEND a TASK-250 supersession note to `docs/specs/paint-pipeline-orca-parity-roadmap.md` §P2 (one paragraph at the end of that section).
- `closure-log.md` already exists from Step 0; Step 6 appends `P94R_POST_CUBE_SHA=<hex>`.

## Out of Scope

- `modules/core-modules/mesh-segmentation/` directory deletion — P97 (WASM mesh-segmentation deletion, TASK-247).
- WASM-guest infrastructure: `PrepassStageOutput::MeshSegmentation`, `BlackboardPrepassSlot::MeshSegmentation`, `commit_mesh_segmentation`, `mesh_segmentation()` getter, `MeshSegmentationIR`, `FacetPaintMark`, `MeshSegmentationIR` in `crates/slicer-ir/src/stage_io.rs:285`, `mesh-segmentation.toml` manifest, `mesh-segmentation.wasm` artifact. P97 owns the deletion.
- The loader's `split_triangle_strokes` / `walk_triangle_selector_strokes` path — NOT touched; this is the canonical normalization site post-P94.
- Paint-segmentation kernel changes — P95 (TASK-245) territory.
- `Blackboard::replace_mesh` — does not exist; no deletion needed.
- `MESH_SEGMENTATION_PRODUCER` constant — does not exist; no deletion needed.
- `PrePass::MeshSegmentation` driver insertion — does not exist; no deletion needed.
- The cherry-pick 5c272ef RED tests in `cube_4color_paint_tdd.rs` and `cube_fuzzy_painted_tdd.rs` — stay RED awaiting P95.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2" (~80 lines; read directly) — historical roadmap intent; the TASK-250 supersession note appended at the end of §P2 documents the retirement.
- `crates/slicer-model-io/src/loader.rs:1900-1961` — read only if confirming the loader's `split_triangle_strokes` path is intact (AC-N3); otherwise treat as out-of-scope for this packet.
- `docs/specs/orca-paint-segmentation-parity.md` §Phase 3 (lines 140-141) — `collect_facets()` design — read once to confirm the canonical input contract P95 inherits.

## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- None directly. The TASK-250 investigation already established the parity surface via delegated reads against `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2490`, `Model.cpp:3806`, and `TriangleSelector.cpp:1542-1606`. The findings are encoded in the Goal of `packet.spec.md`.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-8` from `packet.spec.md`. The deletion is purely subtractive; no new behavior is introduced.
- Negative cases: `AC-N1` (no surviving references to deleted symbols), `AC-N2` (WASM-guest infrastructure stays), `AC-N3` (loader's stroke path untouched).
- Cross-packet impact: P95 unblocks — its `collect_facets()` reads `PaintLayer.strokes` directly. P97 unchanged — still owns the WASM-guest deletion. P96 unaffected.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | AC-3 — workspace compiles after the deletions | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | AC-2 — no lint warnings introduced | FACT pass/fail |
| `cargo test --workspace 2>&1 \| tee target/test-output.log` | AC-4 — workspace gate (deletion blast spans multiple crates; per `CLAUDE.md` §Test Discipline rule 2) | FACT per-bucket count |
| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p94-wedge.gcode && test "$(sha256sum target/p94-wedge.gcode \| awk '{print tolower($1)}')" = "$(grep -oE 'P93_BASELINE_SHA=[a-fA-F0-9]+' .ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md \| head -1 \| cut -d= -f2 \| tr 'A-Z' 'a-z')"` | AC-5 — wedge byte-identical vs P93 baseline | FACT pass/fail (exit 0 == match) |
| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output target/p94-cube.gcode && test -s target/p94-cube.gcode && sha256sum target/p94-cube.gcode \| awk '{print $1}'` | AC-6 — cube slices to completion; capture P94R_POST_CUBE_SHA | FACT (sha256 hash) |
| `cargo xtask build-guests --check` | AC-7 — guest WASM clean | FACT pass/fail |
| `rg -n --glob '!.ralph/specs/94_host-mesh-segmentation-wiring/**' --glob '!docs/specs/paint-pipeline-orca-parity-roadmap.md' --glob '!docs/07_implementation_status.md' 'execute_mesh_segmentation\|MeshSegmentationError\|DegenerateStrokeReason\|mesh_segmentation_executor' crates/ modules/ docs/ ; test $? -eq 1` | AC-N1 — zero surviving references | FACT pass/fail |

## Step Completion Expectations

- Step 0 (baseline capture) MUST have `P93_BASELINE_SHA=<hex>` written to `closure-log.md` BEFORE any deletion begins. AC-5 reads the value back; without the closure-log line the gate cannot pass. **(DONE in initial run: SHA = `AA4DA2FAECA139F2C17909051497D6998F71BFB8A2DD9856D286296252EF1E3B`.)**
- Step 2 (cargo check after kernel + re-export + test deletion) is the cheapest falsifying check. If `cargo check` fails after the deletions, a downstream consumer was missed; pause and resolve before continuing.
- Step 6 (`cube_4color.3mf` end-to-end slice) captures the new cube SHA. This becomes P95's input baseline — P95's acceptance ceremony will assert against `P94R_POST_CUBE_SHA`.
- AC-8 (`docs/07` TASK-244 row update) is a delegated edit; never load the full backlog file.

## Context Discipline Notes

- `crates/slicer-runtime/src/lib.rs` is small but the re-export block sits inside a tightly grouped 8-line section. Range-read at lines 189-200 only.
- `crates/slicer-core/src/algos/mesh_segmentation.rs` — never load. The file is being deleted; reading it is wasted context.
- `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` — never load. The file is being deleted.
- `docs/07_implementation_status.md` — delegate the TASK-244 row update; never load the full backlog.
- Out-of-bounds: `modules/core-modules/mesh-segmentation/**` (P97 territory), `OrcaSlicerDocumented/**` (delegated), the loader's stroke path (AC-N3 sole reference), WASM-guest host infrastructure (`crates/slicer-runtime/src/prepass.rs` `PrepassStageOutput::MeshSegmentation` block at line 280; `crates/slicer-runtime/src/blackboard.rs` `commit_mesh_segmentation` block at lines 158-174).
