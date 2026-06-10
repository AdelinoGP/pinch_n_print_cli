---
status: draft
packet: 94
task_ids: [TASK-244]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: S
---

# Packet 94 — Retire `host:mesh_segmentation` Host Stage

## Goal

Retire the prepass `PrePass::MeshSegmentation` host stage and delete the dead `execute_mesh_segmentation` kernel — the architectural investigation under TASK-250 established that `crates/slicer-model-io/src/loader.rs:1900-1961` already implements OrcaSlicer-parity `TriangleSelector` recursive subdivision (`split_triangle_strokes` + `walk_triangle_selector_strokes`), producing `PaintLayer.strokes` in OrcaSlicer's flat-leaf form at the load boundary; a second normalization stage on the prepass blackboard duplicates the loader's work, has no downstream consumer (P95 reads strokes directly per parity doc Phase 3), and structurally fails on OrcaSlicer-pattern leaves (12+ `TangentToFacetEdge` raise sites because the kernel's clean-bisection template doesn't fit arbitrary-depth subdivisions); so this packet deletes the kernel + the host built-in + the `Blackboard::replace_mesh` method + the prepass driver insertion + the `PrepassExecutionError::MeshSegmentation` variant + the four integration tests, leaving the loader's `split_triangle_strokes` as the canonical TriangleSelector normalization path forward.

## Scope Boundaries

This packet is a surgical retirement of the host stage introduced under TASK-244's prior framing. The loader's stroke-producing path stays untouched; P95 (paint-segmentation port) will consume `PaintLayer.strokes` and `PaintLayer.facet_values` directly per the parity doc's `collect_facets()` design. The WASM `mesh-segmentation` core-module stays disabled (its manifest renamed to `.toml.disabled` during the original P94 work; P97 handles the full directory deletion). Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 91 (P1a — schema scaffolding) closed. Packets 89, 90, 91, 92, 93 already `implemented`. No upstream blocker.
- Unblocks: P95 (paint-segmentation port) — the parity doc's Phase 3 `collect_facets()` reads both `facet_values` and `strokes`; the data-model fork is intentional and matches OrcaSlicer's operational shape (hex bitstream + transient per-extruder flat-list realized as IR-resident `PaintLayer.strokes`).
- Activation blockers: none. The TASK-250 investigation produced the architectural verdict; this packet executes it.

## Acceptance Criteria

### AC-1 — Kernel + producer + Blackboard::replace_mesh deleted

**Given** the retirement,
**When** the workspace is grepped,
**Then** the following symbols and files no longer exist:

- `crates/slicer-core/src/algos/mesh_segmentation.rs` — DELETED.
- `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` — DELETED.
- `crates/slicer-core/src/algos/mod.rs` — no `pub mod mesh_segmentation;` line.
- `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` — DELETED.
- `crates/slicer-runtime/src/builtins/mod.rs` — no `pub mod mesh_segmentation_producer;` line.
- `crates/slicer-runtime/src/blackboard.rs` — no `pub fn replace_mesh(`.
- `Blackboard::replace_mesh` callers gone (zero hits).

| `test ! -f crates/slicer-core/src/algos/mesh_segmentation.rs && test ! -f crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs && test ! -f crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs && ! rg -q 'pub mod mesh_segmentation;' crates/slicer-core/src/algos/mod.rs && ! rg -q 'pub mod mesh_segmentation_producer;' crates/slicer-runtime/src/builtins/mod.rs && ! rg -q 'pub fn replace_mesh' crates/slicer-runtime/src/blackboard.rs`

### AC-2 — Prepass driver insertion + `required_slots` entry + error variant deleted

**Given** the prepass driver,
**When** `crates/slicer-runtime/src/prepass.rs` is grepped,
**Then** no `PrePass::MeshSegmentation` driver insertion, no `"PrePass::MeshSegmentation"` entry in the `required_slots` table, and no `PrepassExecutionError::MeshSegmentation` variant remain.

| `! rg -q 'PrePass::MeshSegmentation|MESH_SEGMENTATION_PRODUCER|execute_mesh_segmentation|MeshSegmentationError|host:mesh_segmentation' crates/slicer-runtime/src/`

### AC-3 — Four P94-introduced integration / contract tests deleted

**Given** the retirement,
**When** the workspace is grepped,
**Then** the four test files introduced by the prior P94 work are gone:

- `crates/slicer-runtime/tests/contract/blackboard_replace_mesh_tdd.rs` — DELETED.
- `crates/slicer-runtime/tests/contract/prepass_execution_error_mesh_segmentation_variant_tdd.rs` — DELETED.
- `crates/slicer-runtime/tests/executor/cube_4color_mesh_segmentation_strokes_consumed_tdd.rs` — DELETED.
- `crates/slicer-runtime/tests/executor/cube_fuzzy_painted_mesh_segmentation_strokes_consumed_tdd.rs` — DELETED.
- `crates/slicer-runtime/tests/executor/mesh_segmentation_determinism_tdd.rs` — DELETED.
- `crates/slicer-runtime/tests/executor/mesh_segmentation_short_circuit_no_strokes_tdd.rs` — DELETED.
- The matching `mod` declarations in `crates/slicer-runtime/tests/contract/main.rs` and `crates/slicer-runtime/tests/executor/main.rs` are gone.

| `for f in crates/slicer-runtime/tests/contract/blackboard_replace_mesh_tdd.rs crates/slicer-runtime/tests/contract/prepass_execution_error_mesh_segmentation_variant_tdd.rs crates/slicer-runtime/tests/executor/cube_4color_mesh_segmentation_strokes_consumed_tdd.rs crates/slicer-runtime/tests/executor/cube_fuzzy_painted_mesh_segmentation_strokes_consumed_tdd.rs crates/slicer-runtime/tests/executor/mesh_segmentation_determinism_tdd.rs crates/slicer-runtime/tests/executor/mesh_segmentation_short_circuit_no_strokes_tdd.rs; do test ! -f "$f" || { echo "SURVIVED: $f"; exit 1; }; done && ! rg -q 'mesh_segmentation' crates/slicer-runtime/tests/contract/main.rs crates/slicer-runtime/tests/executor/main.rs`

### AC-4 — Workspace clippy + check clean after the deletions

**Given** the retirement is purely subtractive,
**When** clippy + check run,
**Then** both succeed with zero warnings / zero failures. No downstream code path references the deleted symbols.

| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log && cargo check --workspace --all-targets 2>&1 | tee -a target/test-output.log`

### AC-5 — `cargo test --workspace` clean

**Given** the deletions remove tests but no production behavior survives,
**When** the workspace test suite runs (dispatched per `CLAUDE.md` §Test Discipline because the deletion blast is wide),
**Then** every bucket reports `test result: ok` and the net test count delta is non-positive (only deletions).

| `cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`

### AC-6 — Byte-identical g-code on `regression_wedge.stl` vs the P93 baseline

**Given** the wedge has no painted strokes (mesh-segmentation was a no-op for it under the previous wiring),
**When** `pnp_cli slice` runs and Step 0's `P93_BASELINE_SHA` is read from the new `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md`,
**Then** the wedge SHA equals the recorded baseline. The deletion is purely subtractive; the wedge produces byte-identical g-code.

| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output target/p94-wedge.gcode && test "$(sha256sum target/p94-wedge.gcode | awk '{print $1}')" = "$(grep -oE 'P93_BASELINE_SHA=[a-f0-9]+' .ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md | head -1 | cut -d= -f2)"`

### AC-7 — `cube_4color.3mf` slices to completion end-to-end

**Given** that the kernel that raised `DegenerateStroke { TangentToFacetEdge }` on this fixture is now deleted,
**When** `pnp_cli slice` runs against `resources/cube_4color.3mf`,
**Then** the slice completes with exit 0 and a non-empty g-code output. The cube SHA is captured in closure-log as `P94R_POST_CUBE_SHA=<hex>` — this becomes the baseline for P95's cube-fixture acceptance.

| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output target/p94-cube.gcode && test -s target/p94-cube.gcode && sha256sum target/p94-cube.gcode | awk '{print $1}'`

### AC-8 — Guest WASM `--check` clean

**Given** no WIT change in this packet (P97 still owns the WASM-guest mesh-segmentation deletion),
**When** `cargo xtask build-guests --check` runs,
**Then** reports clean.

| `cargo xtask build-guests --check`

### AC-9 — TASK-244 row in `docs/07_implementation_status.md` updated to reflect the retirement

**Given** the prior TASK-244 row described the wiring,
**When** the row is updated,
**Then** it documents that TASK-244 was superseded by this packet's retirement decision (TASK-250 architectural finding), and the closure entry is marked closed.

| `rg -q 'TASK-244.*retired|TASK-244.*superseded|TASK-244.*deleted' docs/07_implementation_status.md`

## Negative Test Cases

### AC-N1 — Zero references to deleted symbols survive

**Given** the deletion sweep,
**When** the full workspace is grepped,
**Then** the symbols `execute_mesh_segmentation`, `MESH_SEGMENTATION_PRODUCER`, `MeshSegmentationError`, `host:mesh_segmentation`, `PrePass::MeshSegmentation`, `replace_mesh` produce zero hits outside this packet's own files under `.ralph/specs/94_host-mesh-segmentation-wiring/` and the roadmap's historical narrative.

| `rg -n --glob '!.ralph/specs/94_host-mesh-segmentation-wiring/**' --glob '!docs/specs/paint-pipeline-orca-parity-roadmap.md' --glob '!docs/07_implementation_status.md' 'execute_mesh_segmentation|MESH_SEGMENTATION_PRODUCER|MeshSegmentationError|host:mesh_segmentation|PrePass::MeshSegmentation|replace_mesh' crates/ modules/ docs/ ; test $? -eq 1`

### AC-N2 — `modules/core-modules/mesh-segmentation/` remains in place (P97's territory)

**Given** that P97 (WASM mesh-segmentation deletion) owns the full directory removal,
**When** the modules directory is inspected,
**Then** `modules/core-modules/mesh-segmentation/` still exists, with the manifest disabled (`.toml.disabled` from the original P94 work stays; P97 deletes the directory). This packet does NOT touch the WASM-guest infrastructure.

| `test -d modules/core-modules/mesh-segmentation && test -f modules/core-modules/mesh-segmentation/mesh-segmentation.toml.disabled && ! test -f modules/core-modules/mesh-segmentation/mesh-segmentation.toml`

### AC-N3 — The loader's `split_triangle_strokes` path is untouched

**Given** that the loader is the canonical TriangleSelector normalization site post-P94,
**When** `crates/slicer-model-io/src/loader.rs:1900-1961` is grepped,
**Then** `split_triangle_strokes` and `walk_triangle_selector_strokes` still exist with the same shape; this packet does NOT touch the loader.

| `rg -q 'fn split_triangle_strokes|fn walk_triangle_selector_strokes' crates/slicer-model-io/src/loader.rs`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace 2>&1 | tee target/test-output.log` (workspace gate per `CLAUDE.md` §Test Discipline rule 2 — the deletion blast spans multiple crates)
4. `cargo xtask build-guests --check`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2 — host:mesh_segmentation kernel wiring" (~80 lines) — historical context; the TASK-250 supersession note appended at the end of §P2 documents this packet's retirement decision.
- `crates/slicer-model-io/src/loader.rs:1900-1961` — read in full only if confirming the loader's TriangleSelector path; otherwise treat as the canonical normalization site post-P94.
- `docs/specs/orca-paint-segmentation-parity.md` §Phase 3 (lines 140-141) — `collect_facets()` design that consumes `PaintLayer.strokes` directly; this packet locks in that design as P95's input contract.

## Doc Impact Statement

A list of specific doc sections that this packet modifies:

- `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` (NEW) — captures (a) `P93_BASELINE_SHA=<hex>` for AC-6's wedge baseline-compare (written in Step 0); (b) `P94R_POST_CUBE_SHA=<hex>` recording the cube_4color SHA that becomes P95's input baseline (written in Step 7); (c) a one-paragraph rationale documenting the TASK-250 investigation and supersession decision.
- `docs/07_implementation_status.md` — TASK-244 row updated to reflect retirement (AC-9).
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §P2 — addendum noting TASK-250 supersession (separate edit; non-blocking, but recommended in the same commit for traceability).

No `docs/04_host_scheduler.md` PrePass-table edit is needed (the table reflects what's actually wired; nothing was wired here to begin with after this retirement).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- None directly. The TASK-250 investigation established the parity surface via delegated reads against `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:2490`, `Model.cpp:3806`, and `TriangleSelector.cpp:1542-1606`. The findings are encoded in this packet's Goal and §Authoritative Docs.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
