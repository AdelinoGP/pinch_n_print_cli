# Packet 84 — Design

## Controlling Code Paths

The split is uniform across six files: each moved file becomes a pure-function module in `slicer-core` plus a thin wrapper module in `slicer-runtime`. The wrapper module holds the `BuiltinProducer` impl (which the host scheduler calls) and the `Blackboard` commit (ADR-0001).

```
BEFORE (each file):
slicer-runtime/src/<algo>.rs
├── pub fn execute_<algo>(...)  ← pure algorithm
├── pub static <ALGO>_PRODUCER: BuiltinProducer ...
├── fn commit_<algo>_builtin(&mut Blackboard, output)
└── helper fns

AFTER:
slicer-core/src/algos/<algo>.rs
└── pub fn execute_<algo>(...)  ← unchanged

slicer-runtime/src/builtins/<algo>_producer.rs        (≤ 60 LOC each)
├── pub static <ALGO>_PRODUCER: BuiltinProducer ...
├── impl BuiltinProducer { fn run(&self, bb: &mut Blackboard, ...) {
│       let out = slicer_core::execute_<algo>(...);
│       bb.replace_*(out);
│   } }
└── fn commit_<algo>_builtin(&mut Blackboard, output) (unchanged)
```

OrcaSlicer comparison surface: none NEW — existing parity assertions inside the moved code carry forward verbatim.

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

## Selected Approach

Verbatim algorithm move + thin wrapper retention + one prework. No abstraction beyond what already exists.

Rejected alternatives:

- **Move `region_mapping.rs` together with the other five**. Rejected: its `pub fn execute_region_mapping` signature includes `&ExecutionPlan` (a `slicer-runtime` type today, moving to `slicer-scheduler` in P85). Moving it now would either leak `ExecutionPlan` into `slicer-core` or force a contemporaneous redesign of its signature. P87 owns that move after P85 stabilises the planning crate.
- **Move `FeedrateConfig` into a new `slicer-config` crate**. Rejected: typed config schemas already live in `slicer-ir` (per `docs/02_ir_schemas.md`). Adding a fourth small crate (alongside `slicer-ir`, `slicer-schema`, `slicer-helpers`) for one struct is bookkeeping with no depth gain.
- **Introduce a `StageAlgorithm` trait in `slicer-core`** that wraps the six kernels. Rejected: one production impl per kernel, no test fixture path needs a trait, no second adapter justifies the seam. The pure `pub fn execute_*` shape is the simplest deep interface.
- **Keep `runtime_builtins()` definitions inline in `lib.rs`**. Rejected: the existing layout already groups them at the top of `lib.rs` and the `runtime_builtins()` function lives there. The new `pub mod builtins;` subdirectory consolidates the wrappers; `lib.rs` just imports their `*_PRODUCER` statics.

## Code Change Surface

| File | Action | Notes |
|---|---|---|
| `crates/slicer-ir/src/feedrate.rs` | **CREATE** | Move `FeedrateConfig` struct + `Default` impl from `gcode_emit.rs:64-151`. Public. |
| `crates/slicer-ir/src/lib.rs` | **EDIT** | Add `pub mod feedrate;` + `pub use feedrate::FeedrateConfig;`. |
| `crates/slicer-ir/tests/feedrate_default_tdd.rs` | **CREATE** | One test: `FeedrateConfig::default()` returns the documented field set (regression guard). |
| `crates/slicer-core/src/algos/mod.rs` | **CREATE** | `pub mod mesh_analysis; pub mod paint_segmentation; pub mod prepass_slice; pub mod support_geometry; pub mod mesh_segmentation; pub mod overhang_classifier;` plus re-exports. |
| `crates/slicer-core/src/algos/mesh_analysis.rs` | **CREATE (from move)** | Content of `crates/slicer-runtime/src/mesh_analysis.rs` minus its `MESH_ANALYSIS_PRODUCER` static and `BuiltinProducer` impl (those stay in runtime). |
| `crates/slicer-core/src/algos/paint_segmentation.rs` | **CREATE (from move)** | Same pattern. |
| `crates/slicer-core/src/algos/prepass_slice.rs` | **CREATE (from move)** | Same. |
| `crates/slicer-core/src/algos/support_geometry.rs` | **CREATE (from move)** | Same. |
| `crates/slicer-core/src/algos/mesh_segmentation.rs` | **CREATE (from move)** | Same. |
| `crates/slicer-core/src/algos/overhang_classifier.rs` | **CREATE (from move)** | Imports `FeedrateConfig` via `slicer_ir::FeedrateConfig`. |
| `crates/slicer-core/src/lib.rs` | **EDIT** | Add `pub mod algos;` + selective `pub use algos::*` re-exports (kernels: `execute_mesh_analysis_with`, `execute_paint_segmentation`, `execute_prepass_slice_single_layer`, `execute_prepass_slice_all_layers`, `execute_support_geometry`, `execute_mesh_segmentation`, `classify_layers`). |
| `crates/slicer-core/tests/algo_*_tdd.rs` | **CREATE/MOVE** | 6 test files, one per moved kernel. Migrations of existing `slicer-runtime/tests/` tests where applicable; new for overhang_classifier. |
| `crates/slicer-runtime/src/mesh_analysis.rs` | **DELETE** | |
| `crates/slicer-runtime/src/paint_segmentation.rs` | **DELETE** | |
| `crates/slicer-runtime/src/prepass_slice.rs` | **DELETE** | |
| `crates/slicer-runtime/src/support_geometry.rs` | **DELETE** | |
| `crates/slicer-runtime/src/mesh_segmentation.rs` | **DELETE** | |
| `crates/slicer-runtime/src/overhang_classifier.rs` | **DELETE** | |
| `crates/slicer-runtime/src/builtins/mod.rs` | **CREATE** | `pub mod mesh_analysis_producer; pub mod paint_segmentation_producer; pub mod prepass_slice_producer; pub mod support_geometry_producer; pub mod mesh_segmentation_producer;` plus re-exports of the `*_PRODUCER` statics. |
| `crates/slicer-runtime/src/builtins/mesh_analysis_producer.rs` | **CREATE** | The `MESH_ANALYSIS_PRODUCER: BuiltinProducer` static + the `BuiltinProducer` impl that delegates to `slicer_core::algos::mesh_analysis::execute_mesh_analysis_with` and commits via `Blackboard::replace_*`. ≤ 60 LOC. |
| `crates/slicer-runtime/src/builtins/paint_segmentation_producer.rs` | **CREATE** | Same pattern. |
| `crates/slicer-runtime/src/builtins/prepass_slice_producer.rs` | **CREATE** | Same, holds BOTH `SLICE_PRODUCER` and `SHELL_CLASSIFICATION_PRODUCER` if they were paired in the original file. |
| `crates/slicer-runtime/src/builtins/support_geometry_producer.rs` | **CREATE** | Same. |
| `crates/slicer-runtime/src/builtins/mesh_segmentation_producer.rs` | **CREATE** | Same. |
| `crates/slicer-runtime/src/lib.rs` | **EDIT** | Drop 6 `pub mod ...;` lines; add `pub mod builtins;`. Update `runtime_builtins()` to reference `builtins::*` paths. Optionally add `pub use slicer_core::algos::{...};` if external tests need the kernel entry points by the runtime crate path. |
| `crates/slicer-runtime/src/gcode_emit.rs` | **EDIT (imports only)** | `use crate::overhang_classifier::classify_layers;` → `use slicer_core::classify_layers;`. Delete the `pub struct FeedrateConfig {...}` declaration (lines 64-151) and add `use slicer_ir::FeedrateConfig;`. |
| `crates/slicer-core/Cargo.toml` | **EDIT** | Confirm `slicer-ir = { path = "../slicer-ir" }` present. Add `rayon` if `paint_segmentation` uses it (it does — workspace inheritance). |
| `crates/slicer-runtime/Cargo.toml` | **NO EDIT** | The six moved files used no unique deps; `slicer-core` and `slicer-ir` are already declared. |
| `crates/slicer-runtime/tests/**` | **EDIT or MOVE** | Tests whose SUT is `execute_*` move to `slicer-core/tests/`. Tests whose SUT is `*_PRODUCER` rewire to `slicer_runtime::builtins::*`. |

Primary edit target ≤ 3 files: the new `slicer-core/src/algos/` subtree (counted as one), the new `slicer-runtime/src/builtins/` subtree (counted as one), `crates/slicer-runtime/src/lib.rs`. All other edits are mechanical follow-on.

## Files in Scope (read+edit)

The 26 files in the table above, plus the conditional test set from dispatch #2 below.

## Read-Only Context

| File | Why | Hint |
|---|---|---|
| The six moved files | Identify the algorithm/glue boundary per file. | `grep -E 'pub fn execute_\|pub static .*_PRODUCER\|fn commit_.*_builtin\|BuiltinProducer'` per file. Line-range reads around each match (±50 lines). NEVER load any file in full (largest is 811 LOC). |
| `crates/slicer-runtime/src/gcode_emit.rs` | Find `FeedrateConfig` struct (L64–151) and the `classify_layers` call site (L363). | Targeted reads at those line ranges. |
| `crates/slicer-runtime/src/lib.rs` | Find the six `pub mod` declarations and the `runtime_builtins()` body. | L20–48 (mod block), L70–88 (`runtime_builtins`). |
| `crates/slicer-runtime/src/dag.rs` | Confirm `BuiltinProducer` trait signature so the wrappers implement it correctly. | The trait definition (≤ 50 LOC). |
| `crates/slicer-ir/src/lib.rs` | Identify where to insert `pub mod feedrate;`. | First 30 lines (re-export block). |
| `crates/slicer-core/src/lib.rs` | Identify the existing module layout to slot `algos/` cleanly. | First 30 lines. |
| `docs/adr/0001-prepass-builtins-commit-in-stage.md` | The exact pattern this packet preserves. | Full file (60 LOC). |
| `docs/04_host_scheduler.md` | Confirm `STAGE_ORDER` (the canonical pipeline sequence) so `runtime_builtins()` ordering does not drift. | The relevant section; delegate SUMMARY if > 200 LOC. |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted (existing in-code parity assertions carry forward verbatim).
- `target/**`, `Cargo.lock` — never loaded.
- `crates/slicer-test/**`, `crates/slicer-sdk/**` — concurrent work, do not touch.
- `crates/slicer-runtime/src/wit_host.rs`, `dispatch.rs`, `wasm_instance.rs`, `instance_pool.rs` — moved in P83; do not read.
- `crates/slicer-runtime/src/{cli,helpers_cmd,model_loader,model_loader_sidecar,model_writer}.rs` — already gone (P81/P82).
- `crates/slicer-runtime/src/region_mapping.rs` — P87 territory; stays as-is, do not edit.
- `crates/slicer-runtime/src/blackboard.rs`, `prepass.rs`, `postpass.rs`, `layer_executor.rs` — read only their import lines (to spot any stale `crate::mesh_analysis::*` imports that need updating).
- `modules/core-modules/**` — guest module sources unchanged. Their `.wasm` artifacts rebuild because the slicer-ir / slicer-core edits invalidate them.

## Expected Sub-Agent Dispatches

| # | Question | Scope | Return format |
|---|---|---|---|
| 1 | For each of the six files, what is the algorithm body's line range and where do the `BuiltinProducer` static and `commit_*` glue sit? | The six files | LOCATIONS (~12 entries: 2 per file — algo entry + glue boundary) |
| 2 | Which test files under `crates/slicer-runtime/tests/` reference any of `mesh_analysis::*`, `paint_segmentation::*`, `prepass_slice::*`, `support_geometry::*`, `mesh_segmentation::*`, `overhang_classifier::*`, or call `execute_mesh_analysis`/`execute_paint_segmentation`/etc.? | `crates/slicer-runtime/tests/` | LOCATIONS (≤ 30 entries) |
| 3 | Which non-moving files inside `crates/slicer-runtime/src/` import any of the six modules via `crate::<module>::*`? | `crates/slicer-runtime/src/` | LOCATIONS (≤ 20 entries) |
| 4 | What does the `BuiltinProducer` trait signature look like in `crates/slicer-runtime/src/dag.rs`? | `crates/slicer-runtime/src/dag.rs` | SNIPPET (≤ 30 lines) |
| 5 | After step 4 (the bulk move), `cargo build --workspace`. | repo root | FACT pass/fail + first failing crate |
| 6 | After move + slicer-ir/core edits, `cargo xtask build-guests` (rebuild) then `cargo xtask build-guests --check`. | repo root | FACT pass/fail + STALE list |
| 7 | Post-packet g-code SHA. | repo root | FACT `<hex>` |
| 8 | Per-crate test results: `slicer-core`, `slicer-ir`, `slicer-runtime`, `pnp-cli`. | repo root | FACT pass/fail + counts |

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

## Context Cost Estimate

- Aggregate: **M.** No L step. Total step count: 9.
- Largest single step: step 4 (the bulk move + wrapper creation for six algorithms), rated M. The implementer reads section-by-section; never loads any file in full.
- Highest-risk dispatch: dispatch #6 (guest rebuild). Investigate any STALE entries before re-running tests.

## Open Questions

`None — change is reversible via reverting moves; the wrapper-keeps-commit pattern preserves ADR-0001 behavior locks.`
