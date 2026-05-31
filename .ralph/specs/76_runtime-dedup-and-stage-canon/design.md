# Packet 76 — Design notes

## Files in scope

Edited (TASK-220 + TASK-221):

- `crates/slicer-runtime/src/execution_plan.rs` — 3a
- `crates/slicer-runtime/src/{manifest,validation,dag_cli}.rs` — 3d forwarders
- `crates/slicer-runtime/src/{gcode_emit,dispatch}.rs` — 3b delegates
- `crates/slicer-runtime/src/{mesh_analysis,paint_segmentation,model_loader,prepass_slice}.rs` — 3c adapters
- `crates/slicer-runtime/src/region_mapping.rs` — 1a
- `crates/slicer-runtime/src/pipeline.rs` — 1b
- `crates/slicer-runtime/src/lib.rs` — register new `stage_order` module
- `crates/slicer-runtime/src/prepass.rs` — incidental signature update from 1b
- `crates/slicer-runtime/src/wit_host.rs` — incidental signature update from 1b
- `crates/slicer-runtime/tests/integration/run_pipeline_with_instrumentation_tdd.rs` — fixture update from 1b
- `crates/slicer-ir/src/resolved_config.rs` — 3b canonical `to_config_map`
- `crates/slicer-core/src/lib.rs` — 3c canonical `transform_point3` + unit tests

Added:

- `crates/slicer-runtime/src/stage_order.rs` — 3d canon module
- `crates/slicer-runtime/tests/unit/stage_canon_seam_support_tdd.rs` — 3d regression coverage (registered in `tests/unit/main.rs`)
- `docs/adr/0003-macro-per-world-wit-conversions.md` — ADR for Candidate 2 (TASK-222 deferred)

## Read-only (consulted, not edited)

- `crates/slicer-schema/src/lib.rs` — `STAGES` canon (`STAGE_ORDER` is already
  pinned to it by `stage_list_consistency_tdd`).
- `crates/slicer-runtime/tests/{contract,executor,e2e,integration,unit}/*` —
  all existing suites must stay green; existing test bodies are not rewritten.
- `docs/01_system_architecture.md`, `docs/02_ir_schemas.md`,
  `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md`,
  `docs/05_module_sdk.md` — referenced for invariants; no edits required (see
  Doc Impact Statement in `packet.spec.md`).

## Out of bounds

- `crates/slicer-schema/wit/**` — canonical WIT and component ABI are untouched.
- `crates/slicer-macros/**`, `crates/slicer-sdk/**` — TASK-222 is deferred and
  not recommended; the macro WIT↔IR converters are not edited.
- `modules/core-modules/**`, `crates/slicer-runtime/test-guests/**` — guest
  inputs are untouched (no `cargo xtask build-guests` required for TASK-220/221).

## 3a — wildcard matcher
`fn source_key_matches_declared(declared_key, candidate) -> bool` in
`execution_plan.rs`: a `<prefix>:*` declared key matches any candidate of form
`<prefix>:<rest>`; a static key requires exact match. Both
`bind_module_config_view` (forward expansion over source keys) and
`config_key_declared` (membership test) call it.

## 3b — canonical config map
`ResolvedConfig::to_config_map(&self) -> HashMap<String, ConfigValue>` lives in
`slicer-ir` next to the struct. It is the *superset* the gcode CONFIG_BLOCK
already emitted (incl. the `extensions` passthrough), so `gcode_emit`'s output is
byte-identical. `dispatch`'s private fn becomes a one-line delegate, which adds
the 7 keys it was missing to the per-region view (declared-key-filtered, so
modules are unaffected unless they declared those keys).

## 3c — canonical transform
`slicer_core::transform_point3(matrix: &[f64;16], p: Point3) -> Point3`, superset
semantics: all-zero matrix → identity (fixture robustness from `mesh_analysis`);
homogeneous w-divide (no-op for affine `w==1`, correct for any 4×4). The four
prior copies become one-line adapters (two take `&Transform3d`, two take
`&[f64;16]` with differing arg order).

## 3d — stage-order canon
`execution_plan::STAGE_ORDER` (22 entries, already pinned to `slicer_schema`
by `stage_list_consistency_tdd`) is the authority. New `stage_order.rs` exposes
`known_stage_ids` / `is_known_stage` / `stage_order_index` / `tier_of`, all
derived from it. `manifest.rs`, `validation.rs`, `dag_cli.rs` delete their inline
literals and forward. The bug was purely that the two validators carried *short*
copies; deriving from `STAGE_ORDER` fixes the `UnknownStage` rejection for
seam/support/paint-annotation module stages. (No docs/04 edit needed —
`STAGE_ORDER` already encodes the runtime order, SupportGeometry-after-RegionMapping
included.)

## 1a — region single-pass stamping
`execute_region_mapping_inner` gains
`host_config: Option<(&BTreeMap<String, ResolvedConfig>, &ResolvedConfig)>`.
`None` (the `execute_region_mapping` test/e2e path) keeps the module-emitted
`region.resolved_config` as base; `Some` (the commit path) uses the per-object
map / default as base, then stamps modifier deltas + paint overlays once. The
duplicate post-hoc loop in `commit_region_mapping_builtin` is deleted.

## 1b — pipeline core
`run_pipeline_core(config, raw_config, sink, instrumentation)` holds the
prepass→per-layer→finalization→postpass body with phase brackets and
thumbnail/CONFIG_BLOCK serialization. `run_pipeline_with_raw_config` and
`run_pipeline_with_instrumentation` forward to it (`&NoopInstrumentation` for the
former). `run_pipeline_with_events` is left standalone because it routes through
the bare `execute_postpass` (no CONFIG_BLOCK) — a documented behavioural
difference; merging it broke 3 `pipeline_tdd` assertions, confirming the split is
load-bearing.

## 2 — macro conversions
One macro-level emitter generates `impl From<WitT> for IrT` (+ reverse) per world
from a single source, parameterized by the per-world WIT type path; glue call
sites use `.into()`. Stays per-world because each guest runs its own
`wit_bindgen::generate!` (ADR-0003). Verified by guest round-trip contract tests
after `cargo xtask build-guests`.
