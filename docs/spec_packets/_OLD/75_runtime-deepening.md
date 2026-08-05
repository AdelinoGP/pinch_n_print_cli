---
status: implemented
packet: 75
task_ids: [TASK-216, TASK-217, TASK-218, TASK-219]
---

# 75_runtime-deepening

## Goal

Turn four shallow, copy-pasted areas of `slicer-runtime` into deep modules — concentrating wiring bugs in one
place (locality) and paying one implementation back across many call sites (leverage) — with **zero behaviour
change**, validated by the existing suite plus targeted regression tests. Delivered as four phases under one
packet, gated independently, one commit per phase.

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

# Design — Packet 75

Decisions below were grilled against the codebase; they are resolved, not options.

## Phase 1 — PrePass stage runner (TASK-216)

- **Unify the bracket, not the commit.** `run_builtin_stage(spec, blackboard, instr)` owns
  `produce-guard → estimated_size → StageInstrumentationGuard::start → execute → guard.finish`. The execute
  closure performs its own commit and returns `Result<(), PrepassExecutionError>` — decoupled from
  `PrepassStageOutput` (which stays for the guest path only). Built-ins commit internally today
  (`commit_region_map` `:608`, `commit_slice_ir` `:536`, `commit_support_geometry`,
  `replace_slice_ir` `:157`); `replace_slice_ir` has no IR-return shape, so a single commit path is infeasible.
  **(ADR-0001.)**
- **Spec shape.** `BuiltinStageSpec { stage_id, module_id, required_slots: &[BlackboardPrepassSlot],
  produces: BlackboardPrepassSlot, execute: FnMut(&mut Blackboard) -> Result<(),E> }`. Per-stage extras (paint
  rtree build, `build_paint_semantic_configs`) live inside the closure.
- **Preserve interleaving + phase-split exactly.** The runner replaces the six inline brackets in place; the
  `early_stages → fallback → late_stages` skeleton and `stage_requires_region_map` stay.
- **Deferred:** pulling all prepass ordering (guest + built-in) into one declarative sequence — reaches into guest
  dispatch and the guard-gated fallback-vs-claim semantics.

## Phase 2 — Pure IR harvest extraction (TASK-217)

- `harvest_*` take `ctx` **by value** and read one field each → split is a **move**, no clone. Add
  `harvest_*_from(proposals) -> Result<IR,String>`; wrapper = `harvest_*_from(ctx.<field>)`.
- `parse_canonical_region_id` already exists in `wit_host.rs:2512`; make it `pub(crate)`, delete the
  `dispatch.rs:1658` copy, repoint dispatch's three call sites. **Not** moved to `slicer-ir` (guest-rebuild tax;
  host-only validator).

## Phase 3 — WIT marshalling `with:` unification (TASK-218)

- **Layer world canonical.** prepass/finalization/postpass `bindgen!` add
  `with: { "slicer:types/geometry": super::layer::slicer::types::geometry,
  "slicer:config/config-types": super::layer::slicer::config::config_types }`; `pub mod layer` declared first.
  Matches the existing re-exports (`wit_host.rs:262,272`). **(ADR-0002.)**
- **Geometry + config, with fallback.** Geometry remap kills the converter families + polygon-op host-services
  bodies. Config remap additionally kills duplicate `ConfigValue` + three of four `HostConfigView` impls. **If the
  config-interface remap fights bindgen, ship geometry-only and defer config dedup with reason** — config risk must
  not sink the geometry win.
- **Whole-phase escape hatch:** if `with:` fails outright even for geometry, fall back to a declarative
  `impl_host_services!($world,$geo,$hs)` macro ×4. Decide in the first build cycle; do not ship both.
- **Deferred:** the layer-world-only region-view accessors and builder `push_*` methods (intra-world repetition,
  untouched by cross-world unification).

## Phase 4 — Model intake assembly seam (TASK-219)

- `assemble_object(mesh, id, paint_data, modifiers, config) -> ObjectMesh` computes z-extent internally; thin
  `assemble_objects(...)` for the vec case. **All five wrap sites route through it** — `load_model` STL/OBJ/3MF and
  `run_convert`'s split re-assembly (single + multi component). No new public struct or trait; keep match-on-format
  dispatch.
- Delete `compute_z_extent_for_component`; collapse `identity_transform` / `convert_identity_transform` to one.
- Make `parse_3mf_transform`, `compose_transforms`, `apply_transform_to_paint_data`, `validate_non_uniform_scale`
  `pub(crate)`; add file-free unit tests.
- Split-to-objects (`--merge-components`, default split) decision logic in `run_convert` is untouched; only the
  wrap+z-extent flows through the seam.
- **Behaviour caveat (AC-4.3):** single-component reuse→recompute equivalence under identity transform — locked by
  a regression test.
- **Deferred:** extracting `decode_paint_hex_strokes` from the XML loop.

## ADR summaries

- **ADR-0001 — PrePass built-ins commit in-stage; runner unifies only the bracket.** Hard-ish to reverse,
  surprising (a reader expects symmetry with the guest `commit_stage_output` path), real trade-off
  (`replace_slice_ir` resists a single commit path). Prevents re-suggesting commit-path unification.
- **ADR-0002 — Host marshalling unifies WIT types across worlds via bindgen `with:` remap onto the canonical
  layer world.** Explains why prepass/finalization/postpass depend on layer's generated module; instructs a future
  5th world to remap rather than regenerate; trade-off vs a dedicated shared bindgen.
