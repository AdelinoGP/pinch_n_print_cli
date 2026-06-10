---
status: draft
packet: 97
task_ids: [TASK-247]
backlog_source: docs/specs/paint-pipeline-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet 97 — WASM `mesh-segmentation` Surface Deletion (97-file blast radius)

## Goal

Delete the entire infrastructure for a "guest WASM module can override mesh-segmentation" path — already dead in the post-P94r world (the host `PrePass::MeshSegmentation` stage and its kernel were retired in P94r per the TASK-250 architectural finding; the loader's `split_triangle_strokes` is the canonical TriangleSelector normalization site). This packet completes the cleanup: remove the directory `modules/core-modules/mesh-segmentation/` in full; drop the `mesh-segmentation-output` resource + `run-mesh-segmentation` export from `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit:46-54`; drop the resource implementation + `mesh_segmentation_marks` field + accessor at `crates/slicer-wasm-host/src/host.rs:3588-3622, 767, 1042-1043`; drop the harvest + dispatch arm at `crates/slicer-wasm-host/src/dispatch.rs:1700-1727, 818, 1906-1908`; drop the macro arm at `crates/slicer-macros/src/lib.rs:452, 1439-1480` (triggers guest rebuild); drop `Blackboard::commit_mesh_segmentation` + `mesh_segmentation()` accessor at `crates/slicer-runtime/src/blackboard.rs:159-172` (no consumer remains — P94r retired `replace_mesh`; no consumer ever materialized for the guest-output `commit_mesh_segmentation` either); drop dispatcher-output handling + `BlackboardPrepassSlot::MeshSegmentation` at `crates/slicer-runtime/src/prepass.rs:280, 656, 730`; drop `FacetPaintMark`, `MeshSegmentationIR`, schema constant at `crates/slicer-ir/src/slice_ir.rs:1053-1086, 238-…`; drop `PrepassStageOutput::MeshSegmentation` at `crates/slicer-ir/src/stage_io.rs:30-31, 262-…`; delete the contract roundtrip test `crates/slicer-runtime/tests/contract/macro_mesh_segmentation_output_roundtrip_tdd.rs` and the integration geometry test `crates/slicer-runtime/tests/integration/macro_mesh_segmentation_geometry_tdd.rs`; delete the dispatch contract arms at `crates/slicer-runtime/tests/contract/dispatch_tdd.rs:282, 4771-5074, 6187-…`; drop the scaffolder template at `crates/pnp-cli/src/module_new.rs:388, 521, 569, 571, 681` + the scaffolder test at `crates/pnp-cli/tests/module_new_tdd.rs:136`; drop the stage entry from canonical-stages tables at `crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs:43, 233` and `crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs:653`; drop the bench entry at `crates/slicer-runtime/benches/wasm_modules.rs:89`; force a guest rebuild (`cargo xtask build-guests` no flag, then `--check`); confirm via final grep that ZERO `MeshSegmentation`-style references survive (the kernel, producer constant, host stage, and unit/executor tests were already deleted in P94r — the only surviving references should be historical narrative under `.ralph/specs/`, `docs/specs/paint-pipeline-orca-parity-roadmap.md` §P2, and `docs/07_implementation_status.md` TASK-244 entry).

## Scope Boundaries

97-file blast radius (slightly reduced from the original framing because P94r already deleted the kernel + host built-in + four executor tests + the kernel unit test). Ships as a single packet for atomicity — landing this in pieces would leave the workspace with dangling WIT resources, host-side handlers referencing nonexistent types, or canonical-stages tables claiming a stage no producer can ever claim. The kernel itself at `crates/slicer-core/src/algos/mesh_segmentation.rs` was deleted in P94r; this packet does NOT need to preserve it. The executor test that the original framing planned to rewire (`mesh_segmentation_executor_tdd.rs`) was likewise deleted in P94r. Everything else under P97 is straight deletion. Full file-by-file list in `requirements.md` §In Scope.

## Prerequisites and Blockers

- Depends on: P94r (host stage retirement under TASK-250 verdict) must be `implemented`. With the host stage gone, the WASM-guest surface becomes pure dead code; P97 cleans it up. (The original P94 wiring dependency is moot — the wiring was retired before P97 runs.)
- Unblocks: nothing structurally; this is cleanup.
- Activation blockers: P94 closed.

## Acceptance Criteria

### AC-1 — `modules/core-modules/mesh-segmentation/` entire directory deleted

| `test ! -d modules/core-modules/mesh-segmentation`

### AC-2 — `mesh-segmentation-output` resource + `run-mesh-segmentation` export removed from `world-prepass.wit`

| `! rg -q 'mesh-segmentation-output|run-mesh-segmentation' crates/slicer-schema/wit/`

### AC-3 — `crates/slicer-wasm-host/src/host.rs` no longer carries the resource impl, the `mesh_segmentation_marks` field, or the accessor

| `! rg -q 'mesh_segmentation_marks|MeshSegmentationOutputImpl' crates/slicer-wasm-host/src/host.rs`

### AC-4 — `crates/slicer-wasm-host/src/dispatch.rs` no longer harvests or dispatches mesh-segmentation marks

| `! rg -q 'mesh_segmentation' crates/slicer-wasm-host/src/dispatch.rs`

### AC-5 — `crates/slicer-macros/src/lib.rs` mesh-segmentation macro arm removed; guest rebuild succeeds

**Given** the macro arm gone,
**When** `cargo xtask build-guests` runs,
**Then** every other guest rebuilds without referencing mesh-segmentation; `--check` reports clean.

| `! rg -q 'mesh_segmentation' crates/slicer-macros/src/lib.rs && cargo xtask build-guests && cargo xtask build-guests --check`

### AC-6 — `Blackboard::commit_mesh_segmentation` + `mesh_segmentation()` accessor deleted

| `! rg -q 'commit_mesh_segmentation|fn mesh_segmentation\(' crates/slicer-runtime/src/blackboard.rs`

### AC-7 — Prepass dispatcher-output handling + `BlackboardPrepassSlot::MeshSegmentation` deleted; no `MeshSegmentation`-style stage or slot remains in `slicer-runtime` (P94r retired the host stage; P97 retires the guest-output slot)

| `! rg -q 'BlackboardPrepassSlot::MeshSegmentation' crates/slicer-runtime/src/`

### AC-8 — `FacetPaintMark`, `MeshSegmentationIR`, `PrepassStageOutput::MeshSegmentation`, related schema constants deleted from `slicer-ir`

| `! rg -q 'FacetPaintMark|MeshSegmentationIR|PrepassStageOutput::MeshSegmentation' crates/slicer-ir/`

### AC-9 — Macro-roundtrip contract test + integration geometry test DELETED; kernel unit test PRESERVED; executor test REWIRED to host path

| `test ! -f crates/slicer-runtime/tests/contract/macro_mesh_segmentation_output_roundtrip_tdd.rs && test ! -f crates/slicer-runtime/tests/integration/macro_mesh_segmentation_geometry_tdd.rs && test -f crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs && test -f crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs && ! rg -q 'wasm|guest_mesh_segmentation|run-mesh-segmentation' crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs`

### AC-10 — Dispatch contract arms at `dispatch_tdd.rs:282, 4771-5074, 6187-…` deleted

| `! rg -q 'mesh_segmentation' crates/slicer-runtime/tests/contract/dispatch_tdd.rs`

### AC-11 — `pnp_cli module_new` scaffolder template arm + its test deleted

| `! rg -q 'PrePass::MeshSegmentation|mesh_segmentation' crates/pnp-cli/src/module_new.rs crates/pnp-cli/tests/module_new_tdd.rs`

### AC-12 — Canonical-stages tables in scheduler tests no longer list `PrePass::MeshSegmentation` as a guest-output stage

| `! rg -q 'PrePass::MeshSegmentation' crates/slicer-scheduler/tests/contract/core_module_ir_access_contract_tdd.rs crates/slicer-scheduler/tests/integration/manifest_ingestion_tdd.rs`

### AC-13 — Bench entry for mesh-segmentation removed from `wasm_modules.rs:89`

| `! rg -q 'mesh_segmentation' crates/slicer-runtime/benches/wasm_modules.rs`

### AC-14 — Post-deletion sweep: only historical-narrative references to `MeshSegmentation` survive (no live code)

**Given** the cleanup goal,
**When** `rg -nE 'mesh_segmentation|MeshSegmentation' crates/ modules/` runs,
**Then** the result is bounded to:
- (No live code references survive — the kernel, host BuiltinProducer constant, and driver wiring were already deleted by P94r before P97 runs.)
- `crates/slicer-runtime/src/blackboard.rs` (the `replace_mesh` method's doc-comment may mention "mesh segmentation"; allowed).
- `crates/slicer-core/src/algos/mesh_segmentation.rs` (kernel) and its `mod.rs` / `lib.rs` re-export.
- `crates/slicer-core/tests/algo_mesh_segmentation_tdd.rs` (kernel test — kept).
- `crates/slicer-runtime/tests/executor/mesh_segmentation_executor_tdd.rs` (executor test — kept, rewired).
- A handful of integration tests in P94's surface (kept).
- Documentation hits in `docs/specs/orca-paint-segmentation-parity.md` and `docs/specs/paint-pipeline-orca-parity-roadmap.md` (allowed; these are historical and intentional references).

Manual review of the LOCATIONS dispatch confirms each survivor is intended. The grep below is the count-only gate; the LOCATIONS check is human.

| `rg -c 'mesh_segmentation|MeshSegmentation' crates/ modules/ | awk -F: '{s+=$NF} END {print s}'`

### AC-15 — Workspace tests + workspace clippy clean

| `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace 2>&1 | tee target/test-output.log | grep '^test result' | head -50`

### AC-16 — Guest WASM `--check` clean after rebuild

| `cargo xtask build-guests && cargo xtask build-guests --check`

### AC-17 — Behavior preservation on regression_wedge.stl + cube_4color.3mf

**Given** the deletion targets only dead code paths,
**When** `pnp_cli slice` runs,
**Then** g-code is byte-identical to the post-P96 baseline on both fixtures (the WASM path was dead even before P94r; this packet only deletes already-dead infrastructure).

| `cargo run --bin pnp_cli --release -- slice --model resources/regression_wedge.stl --module-dir modules/core-modules --output /tmp/p97-wedge.gcode && sha256sum /tmp/p97-wedge.gcode && cargo run --bin pnp_cli --release -- slice --model resources/cube_4color.3mf --module-dir modules/core-modules --output /tmp/p97-cube.gcode && sha256sum /tmp/p97-cube.gcode`

## Negative Test Cases

### AC-N1 — `runtime_builtins()` count unchanged from P94

**Given** P94r retired the host `PrePass::MeshSegmentation` stage (along with its `MESH_SEGMENTATION_PRODUCER` constant) and no replacement was wired,
**When** the runtime is inspected,
**Then** the registered-producers count is unchanged from P94 (this packet does not add or remove host producers; it deletes the WASM-guest surface only).

| `[ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq $(git show HEAD~N:crates/slicer-runtime/src/lib.rs | grep -cE '_PRODUCER as &dyn Producer') ] || echo OK`  ← manual: confirm via closure log; the exact bash above depends on N (commits since P94).

(Closure-log check: pre-packet and post-packet producer counts are equal.)

### AC-N2 — No surviving WIT consumer references the deleted resource/export

| `! rg -q 'mesh-segmentation-output|run-mesh-segmentation' crates/ modules/`

## Verification (gate commands only)

1. `cargo check --workspace --all-targets`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests && cargo xtask build-guests --check` (REQUIRED — macro arm change forces full rebuild)
4. `cargo test --workspace 2>&1 | tee target/test-output.log` (workspace gate; this packet's deletion blast is wide enough that workspace gate is the only reliable confirmation per `CLAUDE.md` §Test Discipline)
5. Manual review of `rg -nE 'mesh_segmentation|MeshSegmentation' crates/ modules/` LOCATIONS against AC-14's allow-list.

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §"P5a — WASM mesh-segmentation surface deletion" — exhaustive file-by-file deletion list (≤ 90 lines; read directly).
- Authoritative because the roadmap section enumerates exact line numbers for each deletion site. Cross-check against current line numbers via the Step 1 dispatch (line numbers may have drifted).

## Doc Impact Statement

This packet REMOVES doc references:

- `docs/03_wit_and_manifest.md` — any reference to `mesh-segmentation-output` WIT resource — `! rg -q 'mesh-segmentation-output' docs/03_wit_and_manifest.md` (verified at packet close; if remains, P5c covers).
- `docs/04_host_scheduler.md` — guest-based mesh-segmentation reference — same check.

`docs/02_ir_schemas.md`'s `MeshSegmentationIR` / `FacetPaintMark` sections are removed by packet 99 (P5c). Acceptable lag because the IR types are deleted here and the doc text is informational.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context.

Files to inspect for this packet:

- None. This packet is a pure deletion of an in-house architectural surface; no OrcaSlicer parity is involved.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
