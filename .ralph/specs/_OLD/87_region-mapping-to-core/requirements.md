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
  **Verified field shapes** (from `crates/slicer-scheduler/src/execution_plan.rs:286-289`): `region_plans: Arc<HashMap<RegionKey, RegionPlan>>` (note the `Arc` indirection) and `module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>>` (bare). The projection's `&'a HashMap<...>` shape for `region_plans` is intentional — the wrapper does the Arc deref via `&*plan.region_plans` (or `plan.region_plans.as_ref()`). Step 1 dispatch #1 confirms whether `execute_region_mapping_inner` reads anything beyond these two fields; if so, extend the projection.
- Move the algorithm body from `crates/slicer-runtime/src/region_mapping.rs` into `crates/slicer-core/src/algos/region_mapping.rs` (matching the P84 `algos/` subtree layout — 6 algos already present at L1-L6 of `crates/slicer-core/src/algos/`). **Verified surface to move**: `pub fn execute_region_mapping` (L336, simple delegator passing `DEFAULT_REGION_MAP_CAP`), `pub fn execute_region_mapping_with_cap` (L365, the actual kernel taking a `usize` cap), `fn execute_region_mapping_inner` (L384, private helper), the `DEFAULT_REGION_MAP_CAP` constant (used by the simple wrapper), `pub enum RegionMappingError` (L68), and any private helpers reachable from the kernel. **Replace** `plan: &ExecutionPlan` with `projection: &RegionMappingPlanProjection<'_>` on every public sig. Other parameters preserved: `layer_plan: &LayerPlanIR`, `paint_regions: Option<&PaintRegionIR>`, `paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>`, `objects: &[ObjectMesh]`, plus `cap: usize` on the `_with_cap` variant. Return type unchanged: `Result<RegionMapIR, RegionMappingError>`.
- Move `RegionMappingError` and any other error/helper types the kernel uses into `slicer-core/src/algos/region_mapping.rs` (if they don't reference runtime types).
- Create `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`:
  - Declares `pub static REGION_MAPPING_PRODUCER: BuiltinProducer = ...` with identical metadata as before P87 (currently L30 of `region_mapping.rs`).
  - **Relocates `commit_region_mapping_builtin`** (currently L559 of `region_mapping.rs`, ~70 LOC body, takes `&ExecutionPlan`). This is the runtime glue — stays in the wrapper per ADR-0001. The body of `commit_region_mapping_builtin` is what does the `Blackboard` commit; do NOT split it into a separate function in slicer-core.
  - The wrapper file LOC ≤ 150 (commit body ~70 + producer static ~20 + projection unpack + imports + headroom). AC-2's 120 ceiling was tight; relaxed to 150.
  - Wrapper body shape:
    1. Receives `&ExecutionPlan` from the host scheduler (signature of `commit_region_mapping_builtin` is preserved).
    2. Builds a `RegionMappingPlanProjection { module_region_index: &plan.module_region_index, region_plans: &*plan.region_plans }` — the `Arc` deref on `region_plans` is essential (zero copy, just borrows).
    3. Calls `slicer_core::algos::region_mapping::execute_region_mapping_with_cap(layer_plan, &projection, paint_regions, configs, objects, DEFAULT_REGION_MAP_CAP)` (or `execute_region_mapping` if cap-default suffices).
    4. Commits the returned `RegionMapIR` via the existing `Blackboard` method (verified via Step 1 dispatch #3).
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
| AC-4 | `grep -A 8 -E 'pub fn execute_region_mapping\b' crates/slicer-core/src/algos/region_mapping.rs \| head -9 \| grep -qE 'LayerPlanIR' && ! grep -qE '\b(ExecutionPlan\|CompiledStage\|CompiledModuleStatic\|Blackboard)\b' crates/slicer-core/src/algos/region_mapping.rs` | FACT pass/fail |
| AC-5 | `[ $(grep -cE '_PRODUCER as &dyn Producer' crates/slicer-runtime/src/lib.rs) -eq 8 ]` | FACT pass/fail |
| AC-6 | `! grep -qE '^slicer-(scheduler|runtime|wasm-host) *=' crates/slicer-core/Cargo.toml` | FACT pass/fail |
| AC-7 | `cargo run --bin pnp_cli --release -- slice --model resources/benchy.stl --module-dir modules/core-modules --output /tmp/benchy-p87.gcode && sha256sum /tmp/benchy-p87.gcode` (must equal carried-forward baseline `89a329ad3a4c1b7febca839edfca8b6302e562d8d2a390ee144252fd54e65a2b`) | SNIPPET (SHA) |
| AC-8 | `cargo test -p slicer-core` (slicer-core needs `--features host-algos` if algos test targets gate on it — verify per Step 6's test layout) | FACT pass/fail + count |
| AC-9 | `cargo test --features slicer-core/host-algos --features slicer-sdk/test -p slicer-core -p slicer-runtime -p pnp-cli` (flags mandatory per P85 closure — bare form masks regressions) | FACT pass/fail + counts |
| AC-N1 | `! rg -e 'use [^;]*\b(ExecutionPlan\|CompiledStage\|CompiledModuleStatic\|Blackboard\|BuiltinProducer)\b' crates/slicer-core/src/ && ! rg -e ': *&(mut )?(ExecutionPlan\|CompiledStage\|CompiledModuleStatic\|Blackboard\|BuiltinProducer)\b' crates/slicer-core/src/` | FACT pass/fail |
| AC-N2 | `! cargo tree -p slicer-core 2>&1 \| grep -qE '\b(slicer-scheduler|slicer-wasm-host)\b'` | FACT pass/fail |
| AC-N3 | `for line in $(grep -nE '^pub use slicer_core::algos::region_mapping::' crates/slicer-runtime/src/lib.rs \| cut -d: -f1); do prev=$((line-1)); next=$((line+1)); (sed -n "${prev}p" crates/slicer-runtime/src/lib.rs \| grep -qE '^// kept:') \|\| (sed -n "${next}p" crates/slicer-runtime/src/lib.rs \| grep -qE '^// kept:') \|\| exit 1; done` | FACT pass/fail |
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
