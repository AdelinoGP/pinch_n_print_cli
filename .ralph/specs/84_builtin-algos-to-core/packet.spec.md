---
status: draft
packet: 84
task_ids: [TASK-234]
requires: [83]
backlog_source: docs/07_implementation_status.md
---

# Packet 84 — Push Pure-Algo Builtins Down into `slicer-core`

## Goal

Move six pure-geometry algorithm files (`mesh_analysis.rs`, `paint_segmentation.rs`, `prepass_slice.rs`, `support_geometry.rs`, `mesh_segmentation.rs`, `overhang_classifier.rs` — ~3 200 LOC total) out of `slicer-runtime/src/` into `slicer-core/src/` as IR-in/IR-out functions, leaving thin `*Producer` wrappers (~40 LOC each, owning the `BuiltinProducer` trait impl and the `Blackboard` commit per ADR-0001) in `slicer-runtime/src/builtins/`; as prework, relocate `FeedrateConfig` (89 LOC struct + `Default` impl) from `gcode_emit.rs` to `slicer-ir` so that `overhang_classifier`'s consumer no longer pulls a g-code-side type from runtime.

## Scope Boundaries

This packet deepens `slicer-core` from "geometry primitives" (AabbTree, polygon_ops, slice_mesh_ex, paint_region) into "geometry primitives plus stage algorithms." Six clean-split builtins move (per the D-phase-1 deep-dive: 70–90 % pure-algorithm, 10–30 % runtime glue); the glue stays in `slicer-runtime` as `BuiltinProducer` wrappers. `region_mapping.rs` (628 LOC) does NOT move here — its public API leaks `ExecutionPlan`, so it is deferred to packet 87 (D phase 2) after P85 stabilises `slicer-scheduler`. `FeedrateConfig` is the unblocker for `overhang_classifier`'s move: it currently sits in `gcode_emit.rs` and `overhang_classifier` imports it; moving it to `slicer-ir` (where typed config schemas already live) breaks the cycle so the algorithm can move to `slicer-core` (which must not depend on `slicer-runtime`). Full lists in `requirements.md` §In Scope / §Out of Scope.

## Prerequisites and Blockers

- **Requires packet 83 closed** (slicer-wasm-host extracted; ADR-0004 / ADR-0005 in place). P83's workspace test gate carries the baseline test count this packet's regression check compares against.
- Closure requires `cargo xtask build-guests --check` clean. **This packet edits `crates/slicer-ir/src/` (FeedrateConfig) and `crates/slicer-core/src/` (six new algorithm modules)** — CLAUDE.md §"Guest WASM Staleness" lists both as paths that invalidate guest bindgen output. Implementer MUST rebuild guests with `cargo xtask build-guests` (no `--check`) after the edits and BEFORE running host-integration tests.
- Not a workspace-test checkpoint packet — closes on narrow per-crate gates per the deepening-batch policy.

## Acceptance Criteria

### AC-1 — `FeedrateConfig` lives in `slicer-ir`; `gcode_emit` and `overhang_classifier` import it from there

**Given** the prework move,
**When** the workspace is grepped,
**Then** `pub struct FeedrateConfig` appears exactly once and that occurrence is under `crates/slicer-ir/src/`. The previous declaration in `crates/slicer-runtime/src/gcode_emit.rs` is gone. The two known consumers — `gcode_emit.rs` (still in slicer-runtime through P85) and `overhang_classifier` (moving to slicer-core in this packet) — import it via `use slicer_ir::FeedrateConfig;` (or `slicer_ir::feedrate::FeedrateConfig`, depending on chosen module location).

| `[ $(rg --files-with-matches 'pub struct FeedrateConfig' crates/ | wc -l) -eq 1 ] && rg -l 'pub struct FeedrateConfig' crates/ | grep -qE '^crates/slicer-ir/'`

### AC-2 — Six algorithm files no longer exist under `slicer-runtime/src/`; equivalents exist under `slicer-core/src/`

**Given** the moves,
**When** the working tree is inspected,
**Then** none of `mesh_analysis.rs`, `paint_segmentation.rs`, `prepass_slice.rs`, `support_geometry.rs`, `mesh_segmentation.rs`, `overhang_classifier.rs` exist under `crates/slicer-runtime/src/`. Equivalents exist under `crates/slicer-core/src/` (file names may flatten — e.g., `crates/slicer-core/src/algos/{mesh_analysis,paint_segmentation,...}.rs` or as top-level files). Each `slicer-core` file exposes a pure-function entry point that takes IR types and returns IR types — NO `&mut Blackboard`, `BuiltinProducer`, or `ProgressEvent` types in any signature.

| `for f in mesh_analysis paint_segmentation prepass_slice support_geometry mesh_segmentation overhang_classifier; do test ! -f crates/slicer-runtime/src/$f.rs || exit 1; done && [ $(find crates/slicer-core/src -name '*.rs' | xargs grep -lE 'pub fn (execute_mesh_analysis\|execute_paint_segmentation\|execute_prepass_slice\|execute_support_geometry\|execute_mesh_segmentation\|classify_layers)' | wc -l) -ge 5 ] && ! find crates/slicer-core/src -name '*.rs' | xargs grep -qE 'Blackboard\|BuiltinProducer'`

### AC-3 — Thin `*Producer` wrappers in `slicer-runtime/src/builtins/` retain the `BuiltinProducer` impls and the `Blackboard` commits

**Given** the wrapper split (ADR-0001 preserved: built-in commits stay in-stage),
**When** `crates/slicer-runtime/src/` is inspected,
**Then** a `builtins/` subdirectory (or per-builtin top-level files) carries `MeshAnalysisProducer`, `PaintSegmentationProducer`, `PrepassSliceProducer` (+ `ShellClassificationProducer` if it was paired with prepass_slice), `SupportGeometryProducer`, `MeshSegmentationProducer`. Each is ≤ 60 LOC (including imports, struct decl, `BuiltinProducer` trait impl, and the call into `slicer_core::execute_*`). The `runtime_builtins()` function in `lib.rs` still returns the same set of `&dyn Producer` references as before P84 (entries renamed if needed, but the count and pipeline order are preserved).

| `test -d crates/slicer-runtime/src/builtins || ls crates/slicer-runtime/src/ | grep -qE '_producer\.rs'; grep -E 'fn runtime_builtins' crates/slicer-runtime/src/lib.rs | head -1 | grep -q . && [ $(grep -cE '&[A-Z_]+_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -ge 7 ]`

### AC-4 — `overhang_classifier`'s `classify_layers` is callable from `slicer-runtime::gcode_emit` via `slicer_core::classify_layers`

**Given** the move,
**When** `crates/slicer-runtime/src/gcode_emit.rs` is grepped,
**Then** its previous `use crate::overhang_classifier::classify_layers;` is replaced by `use slicer_core::classify_layers;` (or the equivalent path under the chosen `slicer-core` module layout). The call site at the original `gcode_emit.rs:363` still calls `classify_layers(&mut layers, &feedrate_config)` with identical arguments — only the import path changes.

| `! grep -qE 'use crate::overhang_classifier' crates/slicer-runtime/src/gcode_emit.rs && grep -qE 'use slicer_core::.*classify_layers' crates/slicer-runtime/src/gcode_emit.rs`

### AC-5 — `slicer-core` has zero dependency on `slicer-runtime`, `slicer-wasm-host`, `slicer-helpers` (for the moved code paths), and pulls only `slicer-ir` from first-party deps

**Given** the dep direction invariant,
**When** `crates/slicer-core/Cargo.toml` is inspected,
**Then** it declares `slicer-ir = { path = "../slicer-ir" }` plus whatever external geometry deps the moved algorithms require (e.g., `rayon`, `rstar` if not already present). It does NOT declare path deps on `slicer-runtime`, `slicer-wasm-host`, `slicer-helpers`, `slicer-schema`, `slicer-sdk`, or `slicer-gcode` (the latter doesn't exist yet but the assertion future-proofs it).

| `! grep -qE '^slicer-(runtime\|wasm-host\|helpers\|schema\|sdk\|gcode\|model-io) *=' crates/slicer-core/Cargo.toml && grep -qE '^slicer-ir *=' crates/slicer-core/Cargo.toml`

### AC-6 — `slicer-runtime/src/lib.rs` no longer declares the six moved `pub mod`s; the public re-exports of `execute_*` and `*_PRODUCER` come from the new `builtins/` location

**Given** the move,
**When** `crates/slicer-runtime/src/lib.rs` is grepped,
**Then** none of these lines exist: `pub mod mesh_analysis;`, `pub mod paint_segmentation;`, `pub mod prepass_slice;`, `pub mod support_geometry;`, `pub mod mesh_segmentation;`, `pub mod overhang_classifier;`. Their replacement `pub mod builtins;` (or per-file `pub mod *_producer;`) IS declared, and the `pub use` block at the bottom of lib.rs re-exports the producers and the host-side wrapper entry points (`execute_mesh_analysis`, etc.) from `builtins::*` instead of the deleted modules.

| `! grep -qE '^pub mod (mesh_analysis\|paint_segmentation\|prepass_slice\|support_geometry\|mesh_segmentation\|overhang_classifier);' crates/slicer-runtime/src/lib.rs`

### AC-7 — Per-algorithm unit tests run in `slicer-core` and pass without a `slicer-runtime` build

**Given** the moves,
**When** `cargo test -p slicer-core` runs WITHOUT building `slicer-runtime` (i.e., the test binaries link only what `slicer-core` declares),
**Then** at minimum six per-algorithm unit tests pass: one for each moved kernel, asserting that the function returns the documented IR shape for a small fixture input. The test files live under `crates/slicer-core/tests/` (or as inline `#[cfg(test)] mod tests` blocks). No test imports `slicer_runtime::Blackboard` or `slicer_runtime::*Producer`.

| `cargo test -p slicer-core`

### AC-8 — End-to-end slice produces byte-identical g-code vs the P83 baseline SHA

**Given** the move,
**When** `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p84.gcode` runs after guests are rebuilt,
**Then** the resulting SHA matches the P83 closure SHA. (The algorithms moved here run during prepass + during emit; any divergence in their output would cascade through the pipeline. The byte-identical SHA is the proof that the moves preserved behavior.)

| `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p84.gcode && sha256sum /tmp/benchy-p84.gcode`

### AC-9 — Narrow per-crate test gates pass

**Given** the moves,
**When** `cargo test -p slicer-core -p slicer-ir -p slicer-runtime -p pnp-cli` runs,
**Then** all four crates pass. `slicer-runtime` test count is reduced by the count of tests that moved into `slicer-core/tests/`; `slicer-core` test count is increased by the matching amount. `slicer-ir` test count grows by the `FeedrateConfig`-shape regression test added in step 1.

| `cargo test -p slicer-core -p slicer-ir -p slicer-runtime -p pnp-cli`

## Negative Test Cases

### AC-N1 — No file under `crates/slicer-core/src/` mentions `Blackboard`, `BuiltinProducer`, `ProgressEvent`, or `ExecutionPlan`

**Given** the runtime-glue / algorithm split,
**When** `rg -e 'Blackboard\|BuiltinProducer\|ProgressEvent\|ExecutionPlan' crates/slicer-core/src/` runs,
**Then** the result is empty. This is the structural signal that the algorithms moved truly pure: no leak of runtime types into the geometry crate.

| `! rg -e '\b(Blackboard\|BuiltinProducer\|ProgressEvent\|ExecutionPlan)\b' crates/slicer-core/src/ 2>/dev/null`

### AC-N2 — `region_mapping.rs` is still in `crates/slicer-runtime/src/` (NOT moved in this packet)

**Given** the explicit deferral of `region_mapping` to P87,
**When** the working tree is inspected,
**Then** `test -f crates/slicer-runtime/src/region_mapping.rs` is true. (Negative signal: confirms scope discipline — the deep-dive on D flagged region_mapping as messy because its public sig leaks `ExecutionPlan`; the user-approved plan defers it to P87 after P85 splits the planning crate.)

| `test -f crates/slicer-runtime/src/region_mapping.rs`

### AC-N3 — `crates/slicer-core/Cargo.toml` does NOT regain a path dep on `slicer-runtime`

**Given** the dep direction invariant (AC-5 positive form),
**When** `crates/slicer-core/Cargo.toml` is read,
**Then** no `slicer-runtime` entry appears in `[dependencies]`, `[dev-dependencies]`, or `[build-dependencies]`. This guards against accidental import of a runtime type into a moved algorithm (which would otherwise create a cycle).

| `! grep -qE '^slicer-runtime *=' crates/slicer-core/Cargo.toml`

## Verification (gate commands only)

1. `cargo build --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo xtask build-guests` (rebuild) then `cargo xtask build-guests --check` (clean)
4. `cargo test -p slicer-core -p slicer-ir -p slicer-runtime -p pnp-cli`

Workspace test gate NOT run at P84 close — per the deepening-batch policy, that gate runs only at P83 (done), P85, P88.

Full per-AC matrix lives in `requirements.md`.

## Authoritative Docs

- `docs/02_ir_schemas.md` — `MeshIR`, `SurfaceClassificationIR`, `LayerPlanIR`, `PaintRegionIR`, `SliceIR`, `SupportGeometryIR`, `LayerCollectionIR`. The six algorithms read/write these IR types; read only the shapes relevant to the moved kernels.
- `docs/08_coordinate_system.md` — coordinate units (1 unit = 100 nm). The moved algorithms all operate in integer-unit space; the porting-checklist conventions in this doc are baked in.
- `docs/05_module_sdk.md` §"Test Support" — the `slicer-core` per-algorithm tests use ordinary Rust testing (no `#[module_test]` macro because these are host-side fns); confirm the test convention is consistent.
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` — confirms the wrapper-keeps-commit pattern P84 preserves.
- `CLAUDE.md` §"Coordinate System Hazard" and §"Guest WASM Staleness" — operational discipline for any edit reaching `slicer-ir` or `slicer-core`.

## Doc Impact Statement

No doc files are edited by this packet. `docs/02_ir_schemas.md` already documents the IR types the moved kernels consume — the move does not change those shapes. `docs/05_module_sdk.md`'s "host-side algorithm" framing remains accurate. A future doc-sweep packet may add a one-line mention of `slicer-core` housing the algorithms (currently `docs/01_system_architecture.md` describes pipeline tiers in terms of stages, not algorithm-crate location).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
