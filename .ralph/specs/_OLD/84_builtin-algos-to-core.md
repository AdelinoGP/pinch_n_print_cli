---
status: implemented
packet: 84
task_ids: [TASK-234]
---

# 84_builtin-algos-to-core

## Goal

Move six pure-geometry algorithm files (`mesh_analysis.rs`, `paint_segmentation.rs`, `prepass_slice.rs`, `support_geometry.rs`, `mesh_segmentation.rs`, `overhang_classifier.rs` — ~3 200 LOC total) out of `slicer-runtime/src/` into `slicer-core/src/` as IR-in/IR-out functions, leaving thin `*Producer` wrappers (~40 LOC each, owning the `BuiltinProducer` trait impl and the `Blackboard` commit per ADR-0001) in `slicer-runtime/src/builtins/`; as prework, relocate `FeedrateConfig` (89 LOC struct + `Default` impl) from `gcode_emit.rs` to `slicer-ir` so that `overhang_classifier`'s consumer no longer pulls a g-code-side type from runtime.

## Problem Statement

Six host built-in algorithms live in `slicer-runtime/src/` today, each braiding three concerns into one file:

1. A pure geometry algorithm (signed distance, triangle classification, polygon offset, slab assembly, etc.) — the interesting part.
2. A `BuiltinProducer` trait impl that declares the stage/world/claim wiring.
3. An in-stage `Blackboard` commit (per ADR-0001).

The architecture-review deep-dive on D found that five of seven candidates split cleanly (≥ 70 % pure algorithm, ≤ 30 % runtime glue): `mesh_analysis.rs` (85/15), `paint_segmentation.rs` (75/25), `prepass_slice.rs` (80/20), `support_geometry.rs` (70/30), `mesh_segmentation.rs` (90/10). `overhang_classifier.rs` was flagged as "messy" because of its `FeedrateConfig` dep on `gcode_emit.rs` — once `FeedrateConfig` moves to `slicer-ir` (where typed config schemas naturally live), that coupling dissolves and `overhang_classifier` joins the clean set. `region_mapping.rs` remains messy independently (its public sig leaks `ExecutionPlan`) and stays in `slicer-runtime` until P87.

The fix is the algorithm/glue split: each algorithm becomes a pure function in `slicer-core`, while the runtime keeps a thin wrapper (~40 LOC) holding the `BuiltinProducer` impl and the `Blackboard` commit. Three downstream wins:

- `slicer-core` deepens from "geometry primitives" to "geometry primitives + stage algorithms" — its interface grows narrowly while its implementation absorbs ~3 200 LOC.
- Algorithms unit-test as ordinary Rust functions in `cargo test -p slicer-core` — no `Blackboard`, no `runtime_builtins()` plumbing.
- OrcaSlicer-parity diffs land entirely inside `slicer-core` (e.g., `mesh_analysis` and `overhang_classifier` carry explicit parity baselines today).

## Architecture Constraints

- ADR-0001 preserved: built-in commits stay in-stage. The `commit_*_builtin` body lives in `slicer-runtime/src/builtins/`; only the algorithm body crosses to `slicer-core`.
- ADR-0002 / ADR-0003 untouched: no WIT, no bindgen, no guest-side WIT conversion.
- ADR-0005 / ADR-0006 (from P83) preserved: runner traits and stage-export lookup unchanged. (On-disk filenames are `docs/adr/0005-runner-traits-in-slicer-wasm-host.md` and `docs/adr/0006-export-for-stage-id-sole-lookup.md`; ADR-0004 was claimed by Packet 77.)
- `slicer-core` MUST NOT depend on `slicer-runtime`, `slicer-wasm-host`, `slicer-helpers`, `slicer-sdk`, `slicer-schema`, `slicer-gcode`, or `slicer-model-io`. Path deps in `slicer-core/Cargo.toml` are limited to `slicer-ir` plus crates.io external deps the algorithms need.
- `slicer-ir` MUST NOT depend on anything new. `FeedrateConfig` is plain Rust data plus a `Default` impl — no traits beyond the standard `Debug, Clone, PartialEq` it already has.

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by this edit.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- Each moved kernel preserves its existing signature exactly. No mutability changes (e.g., a kernel that took `&mut [LayerCollectionIR]` keeps that). No error-type changes (`MeshAnalysisError`, `PaintSegmentationError`, etc. move WITH their owning kernel into `slicer-core`).
- `FeedrateConfig`'s field names and types are preserved exactly. The implementation log records the field list pre/post.
- Wrapper `*_PRODUCER` statics retain identical `stage_id`, `world_id`, `claim` (or whatever the existing `BuiltinProducer` fields are). The pipeline sees identical metadata.
- `runtime_builtins()` ordering matches the pre-P84 order (the documented STAGE_ORDER in `docs/04_host_scheduler.md`).

## Locked Assumptions and Invariants

- ADR-0001 preserved: commits stay in `slicer-runtime/src/builtins/` (in-stage).
- Byte-identical g-code: AC-8 SHA equals the P83 closure SHA.
- No new external dep introduced by `slicer-core` unless the moved files used it but `slicer-core/Cargo.toml` did not declare it (verify via dispatch #1's algorithm-body inspection).
- No change in IR shapes: this packet moves code, not types. `MeshIR`, `SurfaceClassificationIR`, etc., are unchanged.
- `FeedrateConfig`'s default values are preserved (the regression test added to `slicer-ir/tests/` asserts each field's default).

## Risks and Tradeoffs

- **Risk: a `pub(crate)` helper in a moved file is referenced from outside the file.** Mitigation: dispatch #3 enumerates external references. Each is either (a) inlined into the wrapper if it was glue, or (b) promoted to `pub` in `slicer-core` if it was algorithm.
- **Risk: a moved algorithm uses a `slicer-helpers` or `slicer-runtime` type that wasn't surfaced in the deep-dive.** Mitigation: dispatch #1 inspects the algorithm bodies for `use` statements; surprises surface there.
- **Risk: guest rebuild fails or shifts artifact bytes** (e.g., `slicer-ir` edit changes generated guest WASM size). Mitigation: gate-3 confirms `--check` clean; AC-8 SHA on the `.gcode` (not the `.wasm`) tolerates guest-build-metadata shifts as long as runtime behavior is unchanged.
- **Tradeoff: `slicer-core` doubles in size** (~2 100 LOC → ~5 300 LOC). Acceptable — the crate's purpose deepens from "primitives" to "primitives + algorithms," which is the point.
- **Tradeoff: `slicer-runtime` keeps the `*_PRODUCER` wrappers**. They are ~360 LOC total (6 wrappers × ~60 LOC). They are the inviolable glue per ADR-0001.
