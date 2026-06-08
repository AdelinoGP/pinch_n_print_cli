---
status: draft
packet: 94
task_ids: [TASK-244]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 94 — `host:mesh_segmentation` Kernel Wiring + `Blackboard::replace_mesh`

## Goal

Wire the already-implemented `execute_mesh_segmentation` kernel at `crates/slicer-core/src/algos/mesh_segmentation.rs:39-109` (which correctly normalizes sub-facet strokes from `paint_data.layers[*].strokes` into whole-triangle `facet_values` splits but is presently dead code — never invoked) into the prepass driver as a new built-in `PrePass::MeshSegmentation` stage that runs FIRST in the prepass sequence (before `host:mesh_analysis`), reading `MeshIR` from the Blackboard and writing back a normalized `MeshIR` via a new `Blackboard::replace_mesh` (sibling of `replace_slice_ir` at `blackboard.rs:276-290`); add the `MESH_SEGMENTATION_PRODUCER: BuiltinProducer` constant in a new sibling file `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` (mirroring the existing `mesh_analysis_producer.rs` shape), with `id: "host:mesh_segmentation"`, `stage: "PrePass::MeshSegmentation"`, `ir_writes: &["MeshIR"]`, `ir_reads: &[]`, and the standard `SemVer { major: 1, minor: 0, patch: 0 }` to `{ major: 4, minor: 0, patch: 0 }` admission; register the producer in `crates/slicer-runtime/src/builtins/mod.rs`; insert the stage invocation at `crates/slicer-runtime/src/prepass.rs:374` with a `has_subfacet_strokes(mesh)` short-circuit guard so unpainted meshes pay zero overhead; add a `PrepassExecutionError::MeshSegmentation { source: MeshSegmentationError }` variant; extend the `required_slots(StageId)` table at `prepass.rs:680-708` with `"PrePass::MeshSegmentation" => &[]`; ensure integration tests confirm sub-facet hex strokes parsed from `cube_4color.3mf` are normalized into `facet_values` before any downstream stage observes the mesh, that the existing `cube_fuzzyPainted.3mf` paint patterns survive normalization, and that the kernel is a structural no-op on unpainted meshes (assert `replace_mesh` is NOT called).

## Scope Boundaries

This packet does NOT touch the kernel itself — `execute_mesh_segmentation` is correct and unit-tested in `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` and stays. The work is pure wiring: new producer constant, new Blackboard method, new prepass driver insertion, new error variant, new integration test. The existing WASM `mesh-segmentation` core-module path remains in place (its deletion is P5a — 97 files of blast radius); during this packet, both paths coexist temporarily, but only the host path runs on `PrePass::MeshSegmentation` because no WASM module declares it for this stage anymore (host built-in claims the stage). Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 91 (P1a — schema scaffolding) must be `implemented` so `BuiltinProducer` schema admission shape is stable. P1b and P1c are recommended but not strictly required.
- Unblocks: P3 (95, paint-segmentation port) consumes normalized `facet_values` from this stage. P5a (97) deletes the WASM module surface this packet displaces.
- Activation blockers: P91 closed.

## Acceptance Criteria

### AC-1 — `Blackboard::replace_mesh` added; mirrors `replace_slice_ir` shape

**Given** the precedent at `crates/slicer-runtime/src/blackboard.rs:276-290`,
**When** `Blackboard::replace_mesh(&mut self, new_mesh: Arc<MeshIR>) -> Result<(), BlackboardError>` is added,
**Then** the method (a) `debug_assert!`s no Tier 2 layer outputs are committed (no `slice_ir`, no `layer_plan`, no `region_map`, no `paint_regions`); (b) returns `Err(BlackboardError::MissingRequiredPrepass { stage: "host:mesh", reason: "mesh slot was never committed" })` if `self.mesh.is_none()` — this is vacuous in production (`mesh` is always committed at construction) but the guard catches reordering bugs; (c) atomically swaps the mesh `Arc`; (d) returns `Ok(())` on success.

| `rg -q 'pub fn replace_mesh' crates/slicer-runtime/src/blackboard.rs && cargo test -p slicer-runtime blackboard_replace_mesh 2>&1 | tee target/test-output.log`

### AC-2 — `MESH_SEGMENTATION_PRODUCER` constant exists with correct shape

**Given** the new producer file,
**When** `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` is inspected,
**Then** it exports `pub static MESH_SEGMENTATION_PRODUCER: BuiltinProducer = BuiltinProducer { id: "host:mesh_segmentation", stage: "PrePass::MeshSegmentation", ir_writes: &["MeshIR"], ir_reads: &[], claims_holds: &[], claims_requires: &[], requires_modules: &[], min_ir_schema: SemVer { major: 1, minor: 0, patch: 0 }, max_ir_schema: SemVer { major: 4, minor: 0, patch: 0 }, _cache_ir_writes: OnceLock::new(), _cache_ir_reads: OnceLock::new(), _cache_claims_holds: OnceLock::new(), _cache_claims_requires: OnceLock::new(), _cache_requires_modules: OnceLock::new() };`. (Shape mirrors `MESH_ANALYSIS_PRODUCER` at `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs`.)

| `rg -q 'pub static MESH_SEGMENTATION_PRODUCER' crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs && rg -q 'stage: "PrePass::MeshSegmentation"' crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs && rg -q 'id: "host:mesh_segmentation"' crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs`

### AC-3 — Producer registered in `crates/slicer-runtime/src/builtins/mod.rs`

**Given** the new producer,
**When** `crates/slicer-runtime/src/builtins/mod.rs` is inspected,
**Then** it contains `pub mod mesh_segmentation_producer;` (and re-exports `MESH_SEGMENTATION_PRODUCER` if the module-level convention exports producer constants); the count of `_PRODUCER as &dyn Producer` style entries (if such a registry exists in `lib.rs` or similar) increments by 1 vs the pre-packet baseline.

| `rg -q 'pub mod mesh_segmentation_producer' crates/slicer-runtime/src/builtins/mod.rs`

### AC-4 — Prepass driver runs `host:mesh_segmentation` FIRST, before `host:mesh_analysis`

**Given** the prepass driver entry at `crates/slicer-runtime/src/prepass.rs:374`,
**When** the driver runs and `has_subfacet_strokes(bb.mesh())` returns true,
**Then** `execute_mesh_segmentation(bb.mesh().clone())` is invoked and its result is committed via `bb.replace_mesh(normalized)`; the `host:mesh_analysis` stage that previously ran first now runs AFTER `host:mesh_segmentation` (verified by reading the driver source in order).

| `rg -B2 -A20 'PrePass::MeshSegmentation' crates/slicer-runtime/src/prepass.rs | rg -q 'host:mesh_segmentation' && rg -B2 -A20 'PrePass::MeshAnalysis' crates/slicer-runtime/src/prepass.rs | rg -q 'host:mesh_analysis'`

### AC-5 — Short-circuit: `host:mesh_segmentation` does nothing when mesh has no sub-facet strokes

**Given** an unpainted mesh (e.g., `resources/regression_wedge.stl`),
**When** the prepass driver runs,
**Then** `has_subfacet_strokes(mesh)` returns false; `execute_mesh_segmentation` is NOT called; `bb.replace_mesh` is NOT called; the mesh `Arc` is unchanged after the stage; a structured progress event records "PrePass::MeshSegmentation skipped (no sub-facet strokes)".

| `cargo test -p slicer-runtime --test executor mesh_segmentation_short_circuit_no_strokes 2>&1 | tee target/test-output.log`

### AC-6 — Sub-facet strokes from `cube_4color.3mf` are normalized into `facet_values` after this stage

**Given** the painted cube fixture `resources/cube_4color.3mf` whose paint data carries sub-facet hex strokes,
**When** prepass runs to completion through `PrePass::MeshSegmentation`,
**Then** for each painted object, `object.paint_data.layers[*].strokes.is_empty()` evaluates true (strokes consumed); `object.paint_data.layers[*].facet_values.len()` exceeds the original triangle count (splits occurred); the normalized mesh's triangle count is greater than or equal to the original (sub-facet splits add triangles); and the test asserts a deterministic post-normalization triangle count (the cube fixture's known paint pattern produces a fixed count).

| `cargo test -p slicer-runtime --test executor cube_4color_mesh_segmentation_strokes_consumed 2>&1 | tee target/test-output.log`

### AC-7 — `cube_fuzzyPainted.3mf` sub-facet paint also normalizes; fuzzy_skin semantic preserved

**Given** the painted cube fixture `resources/cube_fuzzyPainted.3mf` whose paint data carries fuzzy_skin sub-facet strokes,
**When** prepass runs,
**Then** the strokes are normalized; the fuzzy_skin `PaintSemantic` value is preserved on the normalized `facet_values` (the kernel does not lose semantic identity during stroke-to-facet conversion); a deterministic post-normalization facet count is asserted.

| `cargo test -p slicer-runtime --test executor cube_fuzzyPainted_mesh_segmentation_strokes_consumed 2>&1 | tee target/test-output.log`

### AC-8 — Determinism: same input mesh → byte-identical normalized mesh across runs

**Given** the same painted input mesh,
**When** the prepass runs twice on different invocations,
**Then** the produced normalized `MeshIR` (vertices + triangles + facet_values) is byte-equal across the two runs.

| `cargo test -p slicer-runtime --test executor mesh_segmentation_determinism 2>&1 | tee target/test-output.log`

### AC-9 — `required_slots` table extended with the new stage

**Given** the table at `crates/slicer-runtime/src/prepass.rs:680-708`,
**When** it is inspected,
**Then** an entry `"PrePass::MeshSegmentation" => &[]` (no required slots — runs first) exists; the table compiles; existing entries are unchanged.

| `rg -q 'PrePass::MeshSegmentation' crates/slicer-runtime/src/prepass.rs && cargo check -p slicer-runtime 2>&1 | tee target/test-output.log`

### AC-10 — `PrepassExecutionError::MeshSegmentation` variant exists

**Given** the new error variant,
**When** `crates/slicer-runtime/src/` is inspected,
**Then** `PrepassExecutionError::MeshSegmentation { source: MeshSegmentationError }` exists (the `MeshSegmentationError` type is the kernel's error from `crates/slicer-core/src/algos/mesh_segmentation.rs`); a `#[from]` or equivalent allows seamless `?`-propagation in the driver.

| `rg -q 'MeshSegmentation\s*\{\s*source: MeshSegmentationError' crates/slicer-runtime/src/`

### AC-11 — Behavior preservation on unpainted meshes (regression_wedge.stl)

**Given** an unpainted mesh,
**When** `pnp_cli slice` runs end-to-end,
**Then** g-code is byte-identical to the post-P93 baseline (the stage short-circuits and `replace_mesh` never fires).

| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p94-wedge.gcode && sha256sum /tmp/p94-wedge.gcode`

### AC-12 — Behavior change on painted meshes is bounded to the stroke-normalization output

**Given** a painted mesh,
**When** `pnp_cli slice` runs end-to-end,
**Then** the g-code may differ from the post-P93 baseline (because downstream stages now see normalized facet_values instead of un-normalized strokes), BUT every difference is explainable by the normalization. The closure log captures: pre-packet SHA, post-packet SHA, and a one-paragraph rationale linking the diff to stroke normalization. (This is documentation, not a machine gate.)

Manual check via closure-log review. The two SHAs are captured automatically by the test below.

| `cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p94-cube.gcode && sha256sum /tmp/p94-cube.gcode`

### AC-13 — Guest WASM `--check` clean

**Given** no WIT change in this packet (the WASM mesh-segmentation surface still exists; P5a deletes it),
**When** `cargo xtask build-guests --check` runs,
**Then** reports clean.

| `cargo xtask build-guests --check`

## Negative Test Cases

### AC-N1 — `Blackboard::replace_mesh` rejects calls after Tier 2 outputs land

**Given** a Blackboard in which `slice_ir` has been committed,
**When** `replace_mesh` is called,
**Then** `debug_assert!` fires (in debug builds) and/or the method returns `Err(BlackboardError::TierViolation { stage: "host:mesh_segmentation" })` in release builds.

| `cargo test -p slicer-runtime blackboard_replace_mesh_after_tier2_rejected 2>&1 | tee target/test-output.log`

### AC-N2 — A direct call to `execute_mesh_segmentation` on a mesh with NO strokes returns a no-op result

**Given** an unpainted mesh,
**When** `execute_mesh_segmentation` is called directly,
**Then** the returned mesh is structurally identical to the input (same triangle count, same vertices, no facet_values added).

| `cargo test -p slicer-core mesh_segmentation_unpainted_noop 2>&1 | tee target/test-output.log`

### AC-N3 — The dead-code path is gone — `execute_mesh_segmentation` is no longer unreferenced

**Given** the original problem (kernel dead code),
**When** `crates/slicer-runtime/src/` is grepped,
**Then** at least one reference to `execute_mesh_segmentation` exists in the prepass driver wiring.

| `rg -q 'execute_mesh_segmentation' crates/slicer-runtime/src/prepass.rs`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p slicer-core --test algo_mesh_segmentation_tdd 2>&1 | tee target/test-output.log` (kernel still passes)
4. `cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 | tee target/test-output.log` (new integration tests pass)
5. `cargo xtask build-guests --check`

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P2 — host:mesh_segmentation kernel wiring" (~80 lines).
- `docs/04_host_scheduler.md` §"PrePass" stage prerequisites (range-read).
- `crates/slicer-runtime/src/blackboard.rs` — read `replace_slice_ir` at lines 276-290 as the implementation template for `replace_mesh`.
- `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs` — read in full (47 LOC) as the constant-shape template.
- `crates/slicer-core/src/algos/mesh_segmentation.rs` — range-read lines 39-109 (kernel signature + error type only).

## Doc Impact Statement

A list of specific doc sections that this packet adds or modifies:

- `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` doc-comment naming the stage and explaining the short-circuit — `rg -q 'host:mesh_segmentation' crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs`.
- `crates/slicer-runtime/src/blackboard.rs` doc-comment for `replace_mesh` mirroring `replace_slice_ir`'s — `rg -q 'pub fn replace_mesh' crates/slicer-runtime/src/blackboard.rs`.

`docs/04_host_scheduler.md` PrePass-table update is deferred to packet 99 (P5c — Doc updates).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- None directly — the kernel itself was already ported per `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs`. If a question arises about TriangleSelector subdivision parity (see `docs/specs/orca-paint-segmentation-parity.md` H561-H567 hazard list), delegate a SUMMARY against `OrcaSlicerDocumented/src/libslic3r/TriangleSelector.cpp`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
