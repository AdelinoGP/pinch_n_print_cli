---
status: draft
packet: 94
task_ids: [TASK-244]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 94 — `host:mesh_segmentation` Kernel Wiring + `Blackboard::replace_mesh`

## Goal

Wire the existing `execute_mesh_segmentation` host kernel into the prepass driver as a new `PrePass::MeshSegmentation` stage that runs before `host:mesh_analysis`, short-circuits on unpainted meshes, and normalizes sub-facet hex strokes into `facet_values` before any downstream stage observes the mesh.

## Scope Boundaries

This packet does NOT touch the mesh-segmentation kernel itself — `execute_mesh_segmentation` is already correct and unit-tested in `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs`. The work is wiring plus one minimal manifest edit: new producer constant, new Blackboard method, new prepass driver insertion, new error variant, new integration tests, plus the smallest-possible disable of the WASM `mesh-segmentation` manifest's `stage = "PrePass::MeshSegmentation"` line so the host built-in is the sole producer for that stage (AC-3.5). The WASM module's directory remains in place; P5a (97) still owns the full deletion. Full in/out-of-scope lists in `requirements.md`.

## Prerequisites and Blockers

- Depends on: packet 91 (P1a — schema scaffolding) must be `implemented` so `BuiltinProducer` schema admission shape is stable. P1b and P1c are recommended but not strictly required.
- Unblocks: P3 (95, paint-segmentation port) consumes normalized `facet_values` from this stage. P5a (97) deletes the WASM module surface this packet displaces.
- Activation blockers: P91 closed.

## Acceptance Criteria

### AC-1 — `Blackboard::replace_mesh` added; mirrors `replace_slice_ir` shape

**Given** the precedent at `crates/slicer-runtime/src/blackboard.rs:276-290` and the verified fact that `Blackboard::mesh_ir` is `Arc<MeshIR>` (not `Option<...>`) so the field is always present after construction,
**When** `Blackboard::replace_mesh(&mut self, new_mesh: Arc<MeshIR>) -> Result<(), BlackboardError>` is added,
**Then** the method (a) `debug_assert!`s no Tier 2 output has landed: `self.slice_ir.is_none()` AND the `layer_outputs` slice (if initialized) has every slot still `None` — matching the assertion shape in `replace_slice_ir:276-290`; (b) atomically swaps `self.mesh_ir = new_mesh`; (c) returns `Ok(())`. The `Result` return type is kept for symmetry with `replace_slice_ir`; no error path actually fires in the current contract.

| `rg -q 'pub fn replace_mesh' crates/slicer-runtime/src/blackboard.rs && cargo test -p slicer-runtime --test contract blackboard_replace_mesh 2>&1 | tee target/test-output.log`

### AC-2 — `MESH_SEGMENTATION_PRODUCER` constant exists with correct shape

**Given** the new producer file,
**When** `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` is inspected,
**Then** it exports `pub static MESH_SEGMENTATION_PRODUCER: BuiltinProducer = BuiltinProducer { id: "host:mesh_segmentation", stage: "PrePass::MeshSegmentation", ir_writes: &["MeshIR"], ir_reads: &[], claims_holds: &[], claims_requires: &[], requires_modules: &[], min_ir_schema: SemVer { major: 1, minor: 0, patch: 0 }, max_ir_schema: SemVer { major: 4, minor: 0, patch: 0 }, _cache_ir_writes: OnceLock::new(), _cache_ir_reads: OnceLock::new(), _cache_claims_holds: OnceLock::new(), _cache_claims_requires: OnceLock::new(), _cache_requires_modules: OnceLock::new() };`. (Shape mirrors `MESH_ANALYSIS_PRODUCER` at `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs`.)

| `rg -q 'pub static MESH_SEGMENTATION_PRODUCER' crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs && rg -q 'stage: "PrePass::MeshSegmentation"' crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs && rg -q 'id: "host:mesh_segmentation"' crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs`

### AC-3 — Producer module registered in `crates/slicer-runtime/src/builtins/mod.rs`

**Given** the new producer file,
**When** `crates/slicer-runtime/src/builtins/mod.rs` is inspected,
**Then** it contains a top-level `pub mod mesh_segmentation_producer;` line on its own — matching the convention of every other producer module in the same file (`gcode_emit_producer`, `mesh_analysis_producer`, `paint_segmentation_producer`, `prepass_slice_producer`, `region_mapping_producer`, `support_geometry_producer`). No `pub use` re-export is required (only `region_mapping_producer` follows that pattern, by exception).

| `rg -q '^pub mod mesh_segmentation_producer;' crates/slicer-runtime/src/builtins/mod.rs`

### AC-3.5 — WASM `mesh-segmentation` module no longer claims `PrePass::MeshSegmentation`

**Given** the existing WASM core-module manifest at `modules/core-modules/mesh-segmentation/mesh-segmentation.toml`, whose `[stage]` block declares `id = "PrePass::MeshSegmentation"` (verified field shape: the stage owner is a nested `id` key under the `[stage]` section, NOT a top-level `stage = …` line), and which would create a duplicate-producer DAG conflict with the new host built-in,
**When** P94 applies the smallest possible edit to the manifest (mechanism is implementer's choice — comment out the `id = "PrePass::MeshSegmentation"` line inside `[stage]`, comment out the entire `[stage]` block, rename the manifest to `.disabled`, or use the loader's documented "disabled" pathway) and the guests are rebuilt via `cargo xtask build-guests`,
**Then** the manifest no longer registers `PrePass::MeshSegmentation` as a producer stage, so only the host built-in claims it. The directory itself remains in place; P5a (97) still owns the full deletion.

| `! rg -q '^id\s*=\s*"PrePass::MeshSegmentation"' modules/core-modules/mesh-segmentation/mesh-segmentation.toml`

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

| `rg -q '"PrePass::MeshSegmentation"\s*=>\s*&\[\]' crates/slicer-runtime/src/prepass.rs && cargo check -p slicer-runtime --all-targets 2>&1 | tee target/test-output.log`

### AC-10 — `PrepassExecutionError::MeshSegmentation` variant constructs and `?`-propagates

**Given** the new error variant,
**When** a small unit test in `crates/slicer-runtime/tests/contract/prepass_execution_error_mesh_segmentation_variant_tdd.rs` (a) constructs `PrepassExecutionError::MeshSegmentation { source: MeshSegmentationError::<any-real-variant>(...) }` directly, and (b) exercises a `fn() -> Result<(), PrepassExecutionError>` that invokes a function returning `MeshSegmentationError` with the `?` operator,
**Then** the test compiles and runs to completion — proving the variant exists with the correct field shape AND that a `#[from]` (or equivalent `From` impl) is wired so the driver's `?`-propagation typechecks. Grep is the wrong tool here because the variant may use a `#[from]`-decorated `MeshSegmentationError` shorthand that bare regex would miss.

| `cargo test -p slicer-runtime --test contract prepass_execution_error_mesh_segmentation_variant 2>&1 | tee target/test-output.log`

### AC-11 — Behavior preservation on unpainted meshes (regression_wedge.stl)

**Given** an unpainted mesh and the post-P93 baseline SHA recorded as `P93_BASELINE_SHA=<hex>` in `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` during Step 0,
**When** `pnp_cli slice` runs end-to-end,
**Then** the produced g-code SHA equals the recorded baseline (the stage short-circuits and `replace_mesh` never fires). The comparison shell command below exits 0 only on match — matching the P92 / P93 / P95 baseline-compare pattern.

| `mkdir -p target && cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p94-wedge.gcode && test "$(sha256sum /tmp/p94-wedge.gcode | awk '{print $1}')" = "$(grep -oE 'P93_BASELINE_SHA=[a-f0-9]+' .ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md | head -1 | cut -d= -f2)"`

### AC-13 — Guest WASM `--check` clean

**Given** no WIT change in this packet (the WASM mesh-segmentation surface still exists; P5a deletes it),
**When** `cargo xtask build-guests --check` runs,
**Then** reports clean.

| `cargo xtask build-guests --check`

## Negative Test Cases

### AC-N1 — `Blackboard::replace_mesh` panics via `debug_assert!` after Tier 2 outputs land

**Given** a Blackboard in which `slice_ir` has been committed (or a `layer_outputs` slot has been written),
**When** `replace_mesh` is called in a debug build (the standard test target),
**Then** the matching `debug_assert!` fires and the call panics with the message documented in `design.md` §"Code Change Surface". This mirrors `replace_slice_ir`'s contract exactly — release-mode behavior is undefined (the assertion compiles out) and is deliberately not gated, because adding a runtime error variant for a tier violation would require widening `BlackboardError` (which today has no `TierViolation` variant) and that widening is out of P94 scope.

| `cargo test -p slicer-runtime --test contract blackboard_replace_mesh_after_tier2_panics 2>&1 | tee target/test-output.log`

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
3. `cargo test -p slicer-runtime --test executor mesh_segmentation 2>&1 | tee target/test-output.log` (new integration tests pass)
4. `cargo xtask build-guests --check` (AC-13; will require a rebuild after AC-3.5's manifest edit before `--check` reports clean)

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
- `.ralph/specs/94_host-mesh-segmentation-wiring/closure-log.md` — captures (a) `P93_BASELINE_SHA=<hex>` for AC-11's wedge baseline-compare (written in Step 0); (b) `P94_PRE_PAINTED_CUBE_SHA=<hex>` and `P94_POST_PAINTED_CUBE_SHA=<hex>` for the painted `cube_4color.3mf` slice (written in Step 0 and Step 7 respectively) plus a one-paragraph rationale linking the diff to stroke normalization. The painted-cube diff is expected (downstream stages now see normalized `facet_values` instead of un-normalized strokes); the closure-log entry is the documented audit trail, not a machine gate.

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
