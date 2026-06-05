# Packet 87 — Design

## Controlling Code Paths

```
BEFORE (in slicer-runtime):
  pub fn execute_region_mapping(
      layer_plan: &LayerPlanIR,
      plan: &ExecutionPlan,       ← couples to scheduler crate
      paint_regions: Option<&PaintRegionIR>,
      paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
      objects: &[ObjectMesh],
  ) -> Result<RegionMapIR, RegionMappingError>;

AFTER (in slicer-core):
  pub struct RegionMappingPlanProjection<'a> {
      pub module_region_index: &'a HashMap<(u32, ModuleId), Vec<ActiveRegion>>,
      pub region_plans:        &'a HashMap<RegionKey, RegionPlan>,
  }

  pub fn execute_region_mapping(
      layer_plan: &LayerPlanIR,
      projection: &RegionMappingPlanProjection<'_>,   ← only IR types
      paint_regions: Option<&PaintRegionIR>,
      paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
      objects: &[ObjectMesh],
  ) -> Result<RegionMapIR, RegionMappingError>;

WRAPPER (in slicer-runtime/src/builtins/region_mapping_producer.rs):
  pub static REGION_MAPPING_PRODUCER: BuiltinProducer = ...;
  // The relocated commit_region_mapping_builtin body (currently L559 of region_mapping.rs):
  pub fn commit_region_mapping_builtin(bb: &mut Blackboard, plan: &ExecutionPlan, ...) -> Result<(), _> {
      // NOTE: region_plans is Arc<HashMap<...>> (slicer-scheduler/src/execution_plan.rs:286).
      // module_region_index is a bare HashMap (L289). The Arc deref is `&*plan.region_plans`
      // (or `plan.region_plans.as_ref()`).
      let projection = RegionMappingPlanProjection {
          module_region_index: &plan.module_region_index,
          region_plans:        &*plan.region_plans,
      };
      let region_map = slicer_core::algos::region_mapping::execute_region_mapping_with_cap(
          &bb.layer_plan(), &projection, bb.paint_regions(), &bb.paint_semantic_configs(), &bb.objects(), DEFAULT_REGION_MAP_CAP
      )?;
      bb.replace_region_map_ir(region_map);   // ADR-0001: commit in-stage
      Ok(())
  }
```

**Verified facts (Step 1 dispatches in implementation-plan):** `execute_region_mapping` (L336, simple delegator) → calls `execute_region_mapping_with_cap` (L365, actual kernel) → calls `execute_region_mapping_inner` (L384, private helper). The `DEFAULT_REGION_MAP_CAP` constant lives near the kernel and moves with it. `ExecutionPlan` fields: `region_plans: Arc<HashMap<RegionKey, RegionPlan>>` (L286 — note the **Arc indirection**), `module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>>` (L289). The projection struct's `region_plans: &'a HashMap<...>` matches the kernel's needs; the wrapper does the Arc deref.

OrcaSlicer comparison surface: none.

## Architecture Constraints

- ADR-0001 preserved: the wrapper holds the `BuiltinProducer` impl + the `Blackboard::replace_*` commit (the relocated `commit_region_mapping_builtin` body, currently L559 of `region_mapping.rs`, ~70 LOC).
- ADR-0002 / 0003 (preserved); ADR-0005 / 0006 (P83 — runner traits + export_for_stage_id); ADR-0007 (P85 — CompiledModule Static/Live split with HashMap-keyed pairing). ADR-0004 (Test support in slicer-sdk, P77) is unrelated to this packet's surface.
- `slicer-core` MUST NOT gain a `slicer-scheduler` dep. The `RegionMappingPlanProjection` type is the decoupling artifact.
- The exact field set on `RegionMappingPlanProjection` is determined by what `execute_region_mapping_inner` actually reads from `&ExecutionPlan` — dispatch #1 enumerates the reads. Two reads expected (`module_region_index`, `region_plans`) per the deep-dive; if more surface, add them to the projection.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

(`region_mapping` operates on `ActiveRegion` / `RegionPlan` / `LayerPlanIR` — all integer-unit IR — and its kernel does NOT perform mm↔unit conversion. The constraint is for completeness; the algorithm body is unaffected.)

## Selected Approach

Signature decouple via projection struct + kernel move + thin wrapper. The smallest possible change that satisfies AC-6 (no `slicer-scheduler` dep on `slicer-core`).

Rejected alternatives:

- **Add `slicer-scheduler` as a `slicer-core` dep**. Rejected: `slicer-core` is upstream of `slicer-scheduler` in the documented dep graph (the planning crate imports IR types from `slicer-ir`, not from `slicer-core`). Adding a back-edge would be a layering inversion.
- **Move `RegionMapIR` itself to `slicer-scheduler` and have `slicer-core` not own this algorithm**. Rejected: `RegionMapIR` is a pure IR type that already lives in `slicer-ir`; moving it would cascade. Also, the algorithm is geometry-shaped (per-region polygon stamping), which fits `slicer-core`'s purpose better than the planning crate.
- **Pass the projection as a tuple `(&map1, &map2, ...)` instead of a named struct**. Acceptable but ugly. The named struct is one extra type for one extra crate boundary; worth the clarity.

## Code Change Surface

| File | Action | Notes |
|---|---|---|
| `crates/slicer-core/src/algos/region_mapping.rs` | **CREATE (from move)** | Holds: `RegionMappingPlanProjection<'a>` struct, BOTH `pub fn execute_region_mapping` (the cap-default delegator at current L336) AND `pub fn execute_region_mapping_with_cap` (the actual kernel at current L365), `execute_region_mapping_inner` (private, L384), `DEFAULT_REGION_MAP_CAP` constant, `RegionMappingError` enum (L68), any private helpers used only by the kernel. Estimated LOC: ~470 (file is 628 total minus the wrapper-side ~150 LOC = `commit_region_mapping_builtin` ~70 LOC at L559 + producer static ~20 LOC at L30 + projection-unpack glue + imports). Confirm actual count post-move. |
| `crates/slicer-core/src/algos/mod.rs` | **EDIT** | Add `pub mod region_mapping;` + selective `pub use region_mapping::{execute_region_mapping, RegionMappingPlanProjection, RegionMappingError};`. |
| `crates/slicer-core/tests/algo_region_mapping_tdd.rs` | **CREATE** | Per-AC-8: two-object two-region fixture, asserts `RegionMapIR` shape. |
| `crates/slicer-runtime/src/region_mapping.rs` | **DELETE** | Entire file. |
| `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` | **CREATE** | The wrapper: `pub static REGION_MAPPING_PRODUCER`, the body that unpacks `&ExecutionPlan` → `RegionMappingPlanProjection`, calls `slicer_core::*`, commits to `Blackboard`. ≤ 100 LOC. |
| `crates/slicer-runtime/src/builtins/mod.rs` | **EDIT** | Add `pub mod region_mapping_producer;` + re-export `REGION_MAPPING_PRODUCER`. |
| `crates/slicer-runtime/src/lib.rs` | **EDIT** | Drop `pub mod region_mapping;`. Rewrite/drop the `pub use region_mapping::*;` re-exports. `runtime_builtins()` references `builtins::region_mapping_producer::REGION_MAPPING_PRODUCER`. |
| `crates/slicer-runtime/tests/**` | **EDIT or MOVE** | Tests whose SUT is `execute_region_mapping_inner` (or `execute_region_mapping` directly) → move to `crates/slicer-core/tests/algo_region_mapping_tdd.rs` (or sibling file). Tests of `commit_region_mapping_builtin` → stay in runtime; rewire imports to `slicer_runtime::builtins::region_mapping_producer::*`. |

Primary edit target ≤ 3 files: `crates/slicer-core/src/algos/region_mapping.rs` (new), `crates/slicer-runtime/src/builtins/region_mapping_producer.rs` (new), `crates/slicer-runtime/src/lib.rs` (edits to the 9 lines that referenced the old module). All other edits are mechanical follow-on.

## Files in Scope (read+edit)

The 8 files in the table above plus the conditional test files from dispatch #2.

## Read-Only Context

| File | Why | Hint |
|---|---|---|
| `crates/slicer-runtime/src/region_mapping.rs` | Identify the public sig, the `_inner` boundary, and what fields of `&ExecutionPlan` are read. | NEVER load full 628 LOC. Grep for `pub fn`, `execute_region_mapping_inner`, `plan.<field>`. Line-range ±50 reads. |
| `crates/slicer-scheduler/src/execution_plan.rs` (post-P85) | Confirm `ExecutionPlan`'s field set (`module_region_index`, `region_plans`, etc.) for the projection. | Lines around the struct definition. |
| `crates/slicer-runtime/src/lib.rs` | Find the `pub mod region_mapping;` line, the re-export block, the `runtime_builtins()` entry. | Grep. |
| `crates/slicer-runtime/src/blackboard.rs` | Find the `replace_region_map_ir` method (or equivalent) for the wrapper's commit. | Grep. |

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted.
- `target/**`, `Cargo.lock` — never loaded.
- `crates/slicer-test/**`, `crates/slicer-sdk/**` — concurrent work.
- All files moved in P81–P86. Their post-move locations apply.
- `crates/slicer-runtime/src/{layer_executor,prepass,postpass,layer_finalization,run,pipeline,blackboard}.rs` — read only their `use` lines if grep surfaces them.
- `crates/slicer-runtime/src/builtins/` other files (P84/P86 producers) — already in place; do not touch.

## Expected Sub-Agent Dispatches

| # | Question | Scope | Return format |
|---|---|---|---|
| 1 | What fields of `&ExecutionPlan` does `execute_region_mapping_inner` (and any helpers it calls) actually read? Search for `plan.X`, `plan.X()`, or pattern-binds in `crates/slicer-runtime/src/region_mapping.rs`. | The single file | LOCATIONS (file:line + field name, ≤ 10 entries) |
| 2 | Which test files under `crates/slicer-runtime/tests/` reference `region_mapping::*`, `execute_region_mapping`, or `commit_region_mapping_builtin`? | `crates/slicer-runtime/tests/` | LOCATIONS (≤ 15 entries) |
| 3 | What does `crates/slicer-runtime/src/blackboard.rs` expose for committing the region map? (`replace_region_map_ir`, `set_region_map_ir`, etc.) | Grep blackboard.rs | FACT (1-line method name + signature) |
| 4 | After move, `cargo build --workspace`. | repo root | FACT pass/fail |
| 5 | After move, `cargo test -p slicer-core -p slicer-runtime -p pnp-cli`. | repo root | FACT pass/fail + counts |
| 6 | Post-packet g-code SHA. | repo root | FACT `<hex>` |

## Data and Contract Notes

- `RegionMappingPlanProjection<'a>` is a borrow-only type — no `Clone`, no owned data. The wrapper constructs it inline.
- `RegionMappingError` moves to `slicer-core` with the kernel. Any caller that wrapped it into `RuntimeError::Builtin(RegionMappingError(_))` in the runtime side adjusts via `From<RegionMappingError>` impl.
- The kernel's behavior is preserved exactly — only the input shape changes.

## Locked Assumptions and Invariants

- ADR-0001 preserved: in-stage commit in the wrapper.
- Byte-identical g-code: AC-7 SHA = P86 closure SHA. The signature refactor is pure code motion; algorithm behavior is unchanged.
- `slicer-core` ↛ `slicer-scheduler` (no back-edge). AC-N2 verifies.
- 8 builtins in `runtime_builtins()`, same order as `docs/04_host_scheduler.md` documents.

## Risks and Tradeoffs

- **Risk: `execute_region_mapping_inner` reads more `&ExecutionPlan` fields than the deep-dive identified.** Mitigation: dispatch #1 enumerates them; add each to `RegionMappingPlanProjection`.
- **Risk: `RegionMappingError` references a runtime type.** Mitigation: read the enum definition; if any variant uses a runtime type, refactor to use an IR-only equivalent.
- **Tradeoff: `RegionMappingPlanProjection` is a new public type** in `slicer-core`. Minor surface bloat; acceptable.

## Context Cost Estimate

- Aggregate: **M.** No L step. Total step count: 7.
- Largest single step: step 3 (kernel move + projection definition + wrapper creation), rated M.

## Open Questions

`None — change is reversible via reverting moves; the projection struct is the only new type and is private to slicer-core (or pub-but-low-stakes).`
