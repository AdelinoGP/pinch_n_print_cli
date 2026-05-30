# Requirements — Packet 75

## Problem Statement

An architecture review surfaced four shallow areas in `slicer-runtime` — interface nearly as complex as
implementation, knowledge duplicated across files, pure logic untestable without a WASM store:

1. **PrePass orchestration repeats one bracket six times.** The six host-built-in stages each hand-roll
   `guard → estimated_size → StageInstrumentationGuard::start → execute → commit → finish` (`prepass.rs:407–621`).
   The wiring bugs (prereq timing, instrument bracketing, phase order) have no single owning module.
2. **Dispatch harvest logic is pure but WASM-bound.** `harvest_layer_plan_ir` & siblings take
   `HostExecutionContext` by value yet read a single proposal vector each; they can only be tested by standing up
   a WASM instance. `parse_canonical_region_id` is duplicated verbatim in `dispatch.rs` and `wit_host.rs`.
3. **Host↔guest marshalling is generated four times.** All four WIT worlds share one `slicer:types/geometry`
   interface, but each `bindgen!` regenerates it as a distinct Rust type, forcing ~730 lines of per-world IR↔WIT
   converters and host-services bodies that differ only by type namespace.
4. **`ObjectMesh` assembly + z-extent is copied five times.** `load_model`'s three format branches and
   `run_convert`'s split re-assembly each wrap a mesh into an `ObjectMesh` and compute the z-extent; the z-extent
   function itself is cloned (`compute_z_extent_from_mesh` / `compute_z_extent_for_component`).

All four hurt agent navigability and testability. None require any behaviour change.

## Task IDs

- **TASK-216** — PrePass stage runner (Phase 1).
- **TASK-217** — Pure IR harvest extraction (Phase 2).
- **TASK-218** — WIT marshalling `with:` type unification (Phase 3).
- **TASK-219** — Model intake assembly seam (Phase 4).

## In Scope

- `prepass.rs`: a `BuiltinStageSpec` + `run_builtin_stage` owning only the bracket; the six built-ins driven
  through it in their current positions; commit stays in each stage's execute closure.
- `dispatch.rs`: `harvest_*_from(proposals)` pure cores; wrappers delegate. Make `wit_host::parse_canonical_region_id`
  `pub(crate)`; delete the dispatch copy; repoint call sites.
- `wit_host.rs`: `with:` remaps (geometry, and config-types if it builds) on prepass/finalization/postpass
  pointing at the layer world; delete the redundant converters and three of four `HostConfigView` impls.
- `model_loader.rs` / `helpers_cmd.rs`: `assemble_object` atom serving all five wrap sites; delete
  `compute_z_extent_for_component`; collapse the two identity-transform helpers; expose the pure 3MF helpers for
  file-free tests.
- Regression tests: instrumentation-spy (Phase 1); pure `_from` tests (Phase 2); z-extent equivalence (Phase 4).
- Docs: ADR-0001, ADR-0002; sharpen CONTEXT.md **Split to objects**; note Phase 3's ABI stability in packet
  notes.

## Out of Scope

- Any edit to `crates/slicer-schema/wit/**`, guest crates, or the component ABI. No guest rebuild.
- Routing PrePass built-ins through `commit_stage_output` (infeasible for `replace_slice_ir`; see ADR-0001).
- Moving `parse_canonical_region_id` into `slicer-ir` (would force a guest rebuild).
- The all-prepass-ordering declarative graph; the layer-world-only region-view/builder repetition; the 3MF
  XML-parser decomposition. Each is noted as a future deepening.
- Changing the `--merge-components` split-to-objects decision logic in `run_convert`.

## Behaviour-Preservation Notes

- Phase 1: commit position relative to the instrument bracket is unchanged; built-ins keep committing inside their
  own functions.
- Phase 3: WIT contract and component ABI are byte-stable — only host-side Rust type generation changes.
- Phase 4: convert's single-component branch currently *reuses* the parent's `world_z_extent`; routing through
  `assemble_object` *recomputes* from the identical component mesh. Equivalent under the identity transform convert
  uses; locked by AC-4.3.
