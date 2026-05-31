# Packet 84 — Requirements

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

## Grouped Task IDs

- **TASK-234** (new) — Push pure-algo builtins down into `slicer-core`. Recorded under "Architecture Deepening Phase II" alongside TASK-235 (P85) and TASK-236 (P86).

## In Scope

- **FeedrateConfig prework move**: relocate `pub struct FeedrateConfig` (89 LOC struct + `Default` impl) from `crates/slicer-runtime/src/gcode_emit.rs` to `crates/slicer-ir/src/feedrate.rs` (new file) or alongside an existing config-shape module in `slicer-ir`. Add a minimal `slicer-ir` test asserting the field set is preserved (regression guard against silent removal).
- Move six algorithm files into `crates/slicer-core/src/` (file layout flexible; recommended `crates/slicer-core/src/algos/{mesh_analysis,paint_segmentation,prepass_slice,support_geometry,mesh_segmentation,overhang_classifier}.rs`):
  - `crates/slicer-runtime/src/mesh_analysis.rs` (811 LOC; entry `execute_mesh_analysis_with`).
  - `crates/slicer-runtime/src/paint_segmentation.rs` (634 LOC; entry `execute_paint_segmentation`).
  - `crates/slicer-runtime/src/prepass_slice.rs` (527 LOC; entries `execute_prepass_slice_single_layer`, `execute_prepass_slice_all_layers`).
  - `crates/slicer-runtime/src/support_geometry.rs` (472 LOC; entry `execute_support_geometry`).
  - `crates/slicer-runtime/src/mesh_segmentation.rs` (543 LOC; entry `execute_mesh_segmentation`).
  - `crates/slicer-runtime/src/overhang_classifier.rs` (372 LOC; entry `classify_layers`).
- Strip each moved file of its `BuiltinProducer` `static` declaration and any `commit_*_builtin` glue. The algorithm body is preserved verbatim; only the surrounding wrapper code is stripped.
- Create `crates/slicer-runtime/src/builtins/` (or per-file top-level `*_producer.rs`) holding:
  - `MeshAnalysisProducer` (with the existing `BuiltinProducer` impl + a body that calls `slicer_core::execute_mesh_analysis_with` then commits the result to `Blackboard`).
  - `PaintSegmentationProducer` (analogous).
  - `PrepassSliceProducer` + `ShellClassificationProducer` if the latter was paired in `prepass_slice.rs` originally (it is — `SLICE_PRODUCER` and `SHELL_CLASSIFICATION_PRODUCER` both came from that file).
  - `SupportGeometryProducer`.
  - `MeshSegmentationProducer` (if it existed as a builtin; otherwise a utility callable from prepass).
  - No new `OverhangClassifierProducer` — `overhang_classifier` was not in `runtime_builtins()` (it is called inline by `gcode_emit`). After P84, the call site in `gcode_emit.rs` imports `slicer_core::classify_layers` directly; no wrapper needed.
- Update `crates/slicer-runtime/src/lib.rs`:
  - Drop the six `pub mod ...;` declarations.
  - Drop the matching `pub use ...::*;` re-exports.
  - Declare `pub mod builtins;` (or per-file `pub mod *_producer;`).
  - Update `runtime_builtins()` to reference the new `*Producer` paths.
  - Re-export the kernel entry points if any external test relied on `slicer_runtime::execute_mesh_analysis` (add `pub use slicer_core::execute_mesh_analysis;` etc., OR delete the re-export if no external consumer depends on it).
- Update `crates/slicer-runtime/src/gcode_emit.rs`:
  - Change `use crate::overhang_classifier::classify_layers;` to `use slicer_core::classify_layers;`.
  - Change `FeedrateConfig` declaration (if it remains at line 64) → delete; add `use slicer_ir::FeedrateConfig;` instead.
- Add `slicer-core = { path = "../slicer-core" }` as a dep of `slicer-runtime` if not already present (it should be; pre-existing geometry helpers are used).
- Add `slicer-ir` as a dep of `slicer-core` if not already declared (it should be — slicer-core already uses Point2/3 from slicer-ir).
- Per-algorithm unit tests under `crates/slicer-core/tests/` — one per moved kernel, covering at least the happy-path fixture. Tests must import zero runtime types.
- Migrate any test under `crates/slicer-runtime/tests/` whose SUT is `execute_mesh_analysis`/`execute_paint_segmentation`/etc. into `crates/slicer-core/tests/`. Tests whose SUT is the `*Producer` (i.e., the wrapper + commit) stay in `slicer-runtime/tests/` and update their imports to the new `builtins/` path.

## Out of Scope

- `region_mapping.rs` — its `pub fn execute_region_mapping` signature includes `&ExecutionPlan` (a runtime type today, moving to slicer-scheduler in P85); moving it before P85 would leak the runtime type into `slicer-core`. P87 owns that move.
- Touching the WASM modules under `modules/core-modules/*` — none implement the stages these builtins compute. (Some modules implement neighbouring stages; their .wasm rebuild is a side effect of the slicer-ir / slicer-core edits, not a content change.)
- New abstractions (traits, builders) around the moved algorithms. The function signatures are preserved exactly; only the crate location changes.
- Re-exporting `slicer-core::*` symbols from `slicer-sdk` or `slicer-schema`. Guest modules continue to import from `slicer-sdk` as today; the SDK does not gain `slicer-core` algorithm re-exports.
- `gcode_emit.rs` itself moving to `slicer-gcode` — P86 owns that.
- Modularising `overhang_classifier` into a `FinalizationModule` core-module — P88 owns that. P84 only moves the kernel.

## Authoritative Docs

- `docs/02_ir_schemas.md` — read the sections defining `MeshIR`, `SurfaceClassificationIR`, `LayerPlanIR`, `PaintRegionIR`, `SliceIR`, `SupportGeometryIR`, `LayerCollectionIR`. The kernels' I/O contracts are baked into these shapes.
- `docs/08_coordinate_system.md` — confirms the 1 unit = 100 nm convention and the `Point2::from_mm` / `mm_to_units()` boundary pattern used by every moved algorithm.
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` (60 LOC) — read in full; the wrapper-keeps-commit pattern is the rule this packet preserves.
- `CLAUDE.md` §"Coordinate System Hazard", §"Guest WASM Staleness", §"Config Key Naming Convention" — operational discipline.

## Acceptance Summary

The acceptance contract is enumerated in `packet.spec.md` (AC-1..AC-9, AC-N1..AC-N3). Measurable refinements:

- **AC-1 — `FeedrateConfig` field set**: the relocated struct must declare exactly the 27 speed fields documented at the current `gcode_emit.rs:64-151`. The implementation log records the field list before and after the move; a one-line slicer-ir test iterates `Default::default()` and asserts each field equals the documented default.
- **AC-7 — Per-kernel coverage minimum**: each of the six moved kernels has at least ONE test in `slicer-core/tests/`. Five would-be tests are migrations of existing `slicer-runtime` tests; one (overhang) is new (because `classify_layers` was tested via `gcode_emit` integration today, not directly).
- **AC-8 — Byte-identical g-code**: the SHA must equal the P83 closure SHA. The implementation log records both.

## Verification Commands

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `[ $(rg --files-with-matches 'pub struct FeedrateConfig' crates/ \| wc -l) -eq 1 ] && rg -l 'pub struct FeedrateConfig' crates/ \| grep -qE '^crates/slicer-ir/'` | FACT pass/fail |
| AC-2 | `for f in mesh_analysis paint_segmentation prepass_slice support_geometry mesh_segmentation overhang_classifier; do test ! -f crates/slicer-runtime/src/$f.rs \|\| exit 1; done` | FACT pass/fail |
| AC-3 | `[ $(grep -cE '&[A-Z_]+_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -ge 7 ]` | FACT pass/fail |
| AC-4 | `! grep -qE 'use crate::overhang_classifier' crates/slicer-runtime/src/gcode_emit.rs && grep -qE 'use slicer_core::.*classify_layers' crates/slicer-runtime/src/gcode_emit.rs` | FACT pass/fail |
| AC-5 | `! grep -qE '^slicer-(runtime\|wasm-host\|helpers\|schema\|sdk\|gcode\|model-io) *=' crates/slicer-core/Cargo.toml` | FACT pass/fail |
| AC-6 | `! grep -qE '^pub mod (mesh_analysis\|paint_segmentation\|prepass_slice\|support_geometry\|mesh_segmentation\|overhang_classifier);' crates/slicer-runtime/src/lib.rs` | FACT pass/fail |
| AC-7 | `cargo test -p slicer-core` | FACT pass/fail + count |
| AC-8 | `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p84.gcode && sha256sum /tmp/benchy-p84.gcode` | SNIPPET (SHA) |
| AC-9 | `cargo test -p slicer-core -p slicer-ir -p slicer-runtime -p pnp-cli` | FACT pass/fail + counts |
| AC-N1 | `rg -e '\b(Blackboard\|BuiltinProducer\|ProgressEvent\|ExecutionPlan)\b' crates/slicer-core/src/` (success = empty) | FACT empty/non-empty |
| AC-N2 | `test -f crates/slicer-runtime/src/region_mapping.rs` | FACT pass/fail |
| AC-N3 | `! grep -qE '^slicer-runtime *=' crates/slicer-core/Cargo.toml` | FACT pass/fail |
| gate-1 | `cargo build --workspace` | FACT pass/fail |
| gate-2 | `cargo clippy --workspace --all-targets -- -D warnings` | FACT pass/fail |
| gate-3 | `cargo xtask build-guests` (rebuild) then `cargo xtask build-guests --check` | FACT pass/fail |

## Step Completion Expectations

- `FeedrateConfig` prework MUST land before `overhang_classifier` moves; otherwise `overhang_classifier`'s import from `crate::gcode_emit::FeedrateConfig` becomes uncompilable when the file is in `slicer-core` (which can't depend on `slicer-runtime`).
- Guest rebuild MUST happen after editing `slicer-ir` (FeedrateConfig) and `slicer-core` (new algos). The wasm-staleness sequence: edit → `cargo xtask build-guests` (no `--check`) → `cargo xtask build-guests --check` (must report clean) → run host tests.
- The `runtime_builtins()` list in `lib.rs` MUST keep the same count and ordering of `&dyn Producer` entries before and after the move (rename only — the pipeline order is documented in `docs/04_host_scheduler.md` and is the canonical execution sequence).

## Packet-Specific Context Discipline

- The six moved files total ~3 360 LOC. NEVER load any in full. Each move is section-by-section: identify the pure-algorithm body via grep (`pub fn execute_*`, `pub fn classify_*`) and the runtime-glue boundaries (`BuiltinProducer` impl, `commit_*_builtin` fn, `Blackboard::replace_*` calls).
- `OrcaSlicerDocumented/` is irrelevant for the move itself — the algorithms are preserved verbatim. The existing OrcaSlicer-parity assertions inside the moved code (e.g., `overhang_classifier.rs:18-23` comment referencing `ExtrusionProcessor.hpp` line ~524) carry forward unchanged; do not re-derive them.
