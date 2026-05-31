# Packet 87 — Requirements

## Problem Statement

`region_mapping.rs` was the only one of seven D-phase builtin candidates that did NOT split cleanly in P84. The deep-dive on D rated it 65 % pure / 35 % glue with a "MESSY" verdict because its public signature took `&ExecutionPlan` (a runtime type pre-P85; now a scheduler type). The kernel could not move into `slicer-core` without either (a) leaking a planning type into the geometry crate or (b) adding a `slicer-core → slicer-scheduler` back-edge to the dep graph. Both were rejected.

After P85, `ExecutionPlan` lives in `slicer-scheduler` and `slicer-runtime` can read what it needs without re-exporting the type. The fix for P87 is a small **signature refactor**: replace `&ExecutionPlan` in `execute_region_mapping`'s public sig with the smaller projection of plan data the kernel actually reads (the `module_region_index: &HashMap<(u32, ModuleId), Vec<ActiveRegion>>` plus `region_plans: &HashMap<RegionKey, RegionPlan>`, or a new `RegionMappingPlanProjection` struct that bundles those two). The wrapper in `slicer-runtime/src/builtins/region_mapping_producer.rs` does the `&ExecutionPlan` → projection unpack inline (~10 LOC); the kernel itself becomes a pure IR-in/IR-out fn.

Outcomes:

1. `slicer-core` deepens by one more algorithm (~410 LOC moved); the wrapper retains ~80 LOC of glue.
2. `slicer-core` does NOT gain a `slicer-scheduler` dep — the decoupling works.
3. Tests for `execute_region_mapping` run in `slicer-core` without `slicer-runtime` or `slicer-scheduler` in scope.

## Grouped Task IDs

- **TASK-237** (new) — Decouple `region_mapping` from `ExecutionPlan`; move kernel to `slicer-core`. Final D-phase task; completes "Architecture Deepening Phase II".

## In Scope

- Define the projection type in `slicer-core/src/algos/region_mapping.rs`:
  ```rust
  pub struct RegionMappingPlanProjection<'a> {
      pub module_region_index: &'a std::collections::HashMap<(u32, slicer_ir::ModuleId), Vec<slicer_ir::ActiveRegion>>,
      pub region_plans: &'a std::collections::HashMap<slicer_ir::RegionKey, slicer_ir::RegionPlan>,
  }
  ```
  Exact field set determined by dispatch #1 (whatever `execute_region_mapping_inner` reads today). Either a struct or a tuple is acceptable; whichever the implementer chooses, it lives in `slicer-core` and does NOT import any scheduler / runtime / wasm-host type.
- Move the algorithm body (currently `execute_region_mapping_inner` + helpers) from `crates/slicer-runtime/src/region_mapping.rs` into `crates/slicer-core/src/algos/region_mapping.rs` (matching the P84 `algos/` subtree layout). Public entry: `pub fn execute_region_mapping(layer_plan: &LayerPlanIR, projection: &RegionMappingPlanProjection<'_>, paint_regions: Option<&PaintRegionIR>, paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>, objects: &[ObjectMesh]) -> Result<RegionMapIR, RegionMappingError>`.
- Move `RegionMappingError` and any other error/helper types the kernel uses into `slicer-core/src/algos/region_mapping.rs` (if they don't reference runtime types).
- Create `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`:
  - Declares `pub static REGION_MAPPING_PRODUCER: BuiltinProducer = ...` with identical metadata as before P87.
  - The wrapper body (≤ 100 LOC):
    1. Receives `&ExecutionPlan` from the host scheduler.
    2. Builds a `RegionMappingPlanProjection` from `plan.module_region_index` and `plan.region_plans` (zero copy — just borrows).
    3. Calls `slicer_core::algos::region_mapping::execute_region_mapping(layer_plan, &projection, paint_regions, configs, objects)`.
    4. Commits the returned `RegionMapIR` via `Blackboard::replace_region_map_ir` (or whatever `commit_region_mapping_builtin` does today). Per ADR-0001, commit stays in-stage in the wrapper.
- Update `crates/slicer-runtime/src/builtins/mod.rs` to declare `pub mod region_mapping_producer;` + re-export `REGION_MAPPING_PRODUCER`.
- Update `crates/slicer-runtime/src/lib.rs`:
  - Drop `pub mod region_mapping;`.
  - Drop or rewrite the `pub use region_mapping::{execute_region_mapping, ...};` re-export. Rewrite to `pub use slicer_core::algos::region_mapping::execute_region_mapping;` if any external consumer relies on the runtime-path; drop otherwise.
  - `runtime_builtins()` references `&builtins::region_mapping_producer::REGION_MAPPING_PRODUCER as &dyn Producer` in the same position (3rd entry, after MESH_ANALYSIS_PRODUCER) as before P87.
- Delete `crates/slicer-runtime/src/region_mapping.rs`.
- Migrate tests under `crates/slicer-runtime/tests/` whose SUT is `execute_region_mapping_inner` (or `execute_region_mapping` directly) into `crates/slicer-core/tests/`. Imports rewrite to `slicer_core::algos::region_mapping::*`. The wrapper-level `commit_region_mapping_builtin` tests stay in `slicer-runtime/tests/` and rewire their `use crate::region_mapping::*` to `use slicer_runtime::builtins::region_mapping_producer::*`.

## Out of Scope

- `crates/slicer-test/`, `crates/slicer-sdk/` — concurrent work.
- WIT contract changes. None.
- Touching the other six P84-moved algorithms or the `*_producer.rs` files. The new `region_mapping_producer.rs` follows the P84 pattern exactly.
- New abstractions beyond the `RegionMappingPlanProjection` struct (which is purely a sig-decoupling artifact, not a new architectural seam).
- Adding `slicer-scheduler` as a dep of `slicer-core` (explicitly rejected — see `design.md`).
- Touching `gcode_emit` or `slicer-gcode` (P86's territory).

## Authoritative Docs

- `docs/02_ir_schemas.md` — `RegionMapIR`, `RegionPlan`, `ActiveRegion`, `RegionKey`, `LayerPlanIR`. The kernel's I/O.
- `docs/04_host_scheduler.md` — `PrePass::RegionMapping` stage placement; `runtime_builtins()` order.
- `docs/adr/0001-prepass-builtins-commit-in-stage.md` — preserved.
- `docs/adr/0006-compiled-module-static-live-split.md` (P85 close) — confirms `ExecutionPlan`'s scheduler-side home, which the wrapper's projection-unpack reads.

## Acceptance Summary

The acceptance contract is enumerated in `packet.spec.md` (AC-1..AC-9, AC-N1..AC-N2). Measurable refinements:

- **AC-4 — Signature shape**: the new `execute_region_mapping` parameter list must contain `&LayerPlanIR`, a projection type (struct or tuple), `Option<&PaintRegionIR>`, `&BTreeMap<PaintSemantic, ResolvedConfig>`, `&[ObjectMesh]`. No more, no less (modulo lifetime annotations). The implementation log records the verbatim signature before and after.
- **AC-7 — Byte-identical g-code**: the SHA carries from P86 closure. The algorithm body is unchanged; only its caller-side input shape changes. Any SHA divergence is a regression in the projection-unpack.
- **AC-8 — Per-algorithm test**: at minimum one new test in `crates/slicer-core/tests/algo_region_mapping_tdd.rs` constructs a small two-object fixture and asserts `RegionMapIR` shape. Imports zero `slicer_runtime::*` or `slicer_scheduler::*` types.

## Verification Commands

| ID | Command | Delegation hint |
|---|---|---|
| AC-1 | `test ! -f crates/slicer-runtime/src/region_mapping.rs && find crates/slicer-core/src -name 'region_mapping*' -type f \| head -1 \| grep -q .` | FACT pass/fail |
| AC-2 | `grep -qE 'pub static REGION_MAPPING_PRODUCER' crates/slicer-runtime/src/builtins/region_mapping_producer.rs && [ $(wc -l < crates/slicer-runtime/src/builtins/region_mapping_producer.rs) -le 120 ]` | FACT pass/fail |
| AC-3 | `! grep -qE '^pub mod region_mapping;' crates/slicer-runtime/src/lib.rs && grep -qE 'REGION_MAPPING_PRODUCER' crates/slicer-runtime/src/lib.rs` | FACT pass/fail |
| AC-4 | `grep -E 'pub fn execute_region_mapping' crates/slicer-core/src/algos/region_mapping.rs \| head -1 \| grep -qE 'LayerPlanIR' && ! grep -qE 'ExecutionPlan\|CompiledStage\|CompiledModuleStatic\|Blackboard' crates/slicer-core/src/algos/region_mapping.rs` | FACT pass/fail |
| AC-5 | `[ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq 8 ]` | FACT pass/fail |
| AC-6 | `! grep -qE '^slicer-(scheduler\|runtime\|wasm-host) *=' crates/slicer-core/Cargo.toml` | FACT pass/fail |
| AC-7 | `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p87.gcode && sha256sum /tmp/benchy-p87.gcode` | SNIPPET (SHA) |
| AC-8 | `cargo test -p slicer-core` | FACT pass/fail + count |
| AC-9 | `cargo test -p slicer-core -p slicer-runtime -p pnp-cli` | FACT pass/fail + counts |
| AC-N1 | `! rg -e '\b(ExecutionPlan\|CompiledStage\|CompiledModuleStatic\|Blackboard\|BuiltinProducer)\b' crates/slicer-core/src/` | FACT empty/non-empty |
| AC-N2 | `! cargo tree -p slicer-core 2>&1 \| grep -qE '\b(slicer-scheduler\|slicer-wasm-host)\b'` | FACT pass/fail |
| gate-1 | `cargo build --workspace` | FACT pass/fail |
| gate-2 | `cargo clippy --workspace --all-targets -- -D warnings` | FACT pass/fail |
| gate-3 | `cargo xtask build-guests --check` | FACT pass/fail |

## Step Completion Expectations

- The `RegionMappingPlanProjection` type MUST be defined in `slicer-core` (not `slicer-runtime` or `slicer-scheduler`) so the kernel signature stays within `slicer-core`'s namespace.
- The wrapper's `&ExecutionPlan` → projection unpack MUST be zero-copy (only borrows). Any clone introduced changes algorithm performance characteristics.
- The `commit_*_builtin` body in the wrapper preserves the existing `Blackboard::replace_*` semantics exactly.
- Guest rebuild not expected; if `--check` reports STALE, investigate (no guest-feeding path was edited).

## Packet-Specific Context Discipline

- `region_mapping.rs` is 628 LOC. NEVER load in full. Identify the `execute_region_mapping_inner` body (~410 LOC of the file is the pure kernel per the deep-dive) via grep + line-range reads.
- `OrcaSlicerDocumented/` is irrelevant — no parity surface.
