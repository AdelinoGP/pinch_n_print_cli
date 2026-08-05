---
status: implemented
packet: 87
task_ids: [TASK-237]
---

# 87_region-mapping-to-core

## Goal

Decouple `execute_region_mapping`'s public signature from `&ExecutionPlan` (which after P85 lives in `slicer-scheduler`), then move the pure-algorithm kernel (~410 LOC of the file's 628 LOC) from `crates/slicer-runtime/src/region_mapping.rs` into `crates/slicer-core/src/algos/region_mapping.rs`, leaving a thin `RegionMappingProducer` wrapper (~80 LOC, including the `BuiltinProducer` impl, the `&ExecutionPlan` → projection unpacking, and the `Blackboard` commit per ADR-0001) in `crates/slicer-runtime/src/builtins/region_mapping_producer.rs`.

## Problem Statement

`region_mapping.rs` was the only one of seven D-phase builtin candidates that did NOT split cleanly in P84. The deep-dive on D rated it 65 % pure / 35 % glue with a "MESSY" verdict because its public signature took `&ExecutionPlan` (a runtime type pre-P85; now a scheduler type). The kernel could not move into `slicer-core` without either (a) leaking a planning type into the geometry crate or (b) adding a `slicer-core → slicer-scheduler` back-edge to the dep graph. Both were rejected.

After P85, `ExecutionPlan` lives in `slicer-scheduler` and `slicer-runtime` can read what it needs without re-exporting the type. The fix for P87 is a small **signature refactor**: replace `&ExecutionPlan` in `execute_region_mapping`'s public sig with the smaller projection of plan data the kernel actually reads (the `module_region_index: &HashMap<(u32, ModuleId), Vec<ActiveRegion>>` plus `region_plans: &HashMap<RegionKey, RegionPlan>`, or a new `RegionMappingPlanProjection` struct that bundles those two). The wrapper in `slicer-runtime/src/builtins/region_mapping_producer.rs` does the `&ExecutionPlan` → projection unpack inline (~10 LOC); the kernel itself becomes a pure IR-in/IR-out fn.

Outcomes:

1. `slicer-core` deepens by one more algorithm (~410 LOC moved); the wrapper retains ~80 LOC of glue.
2. `slicer-core` does NOT gain a `slicer-scheduler` dep — the decoupling works.
3. Tests for `execute_region_mapping` run in `slicer-core` without `slicer-runtime` or `slicer-scheduler` in scope.

## Architecture Constraints

- ADR-0001 preserved: the wrapper holds the `BuiltinProducer` impl + the `Blackboard::replace_*` commit (the relocated `commit_region_mapping_builtin` body, currently L559 of `region_mapping.rs`, ~70 LOC).
- ADR-0002 / 0003 (preserved); ADR-0005 / 0006 (P83 — runner traits + export_for_stage_id); ADR-0007 (P85 — CompiledModule Static/Live split with HashMap-keyed pairing). ADR-0004 (Test support in slicer-sdk, P77) is unrelated to this packet's surface.
- `slicer-core` MUST NOT gain a `slicer-scheduler` dep. The `RegionMappingPlanProjection` type is the decoupling artifact.
- The exact field set on `RegionMappingPlanProjection` is determined by what `execute_region_mapping_inner` actually reads from `&ExecutionPlan` — dispatch #1 enumerates the reads. Two reads expected (`module_region_index`, `region_plans`) per the deep-dive; if more surface, add them to the projection.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

(`region_mapping` operates on `ActiveRegion` / `RegionPlan` / `LayerPlanIR` — all integer-unit IR — and its kernel does NOT perform mm↔unit conversion. The constraint is for completeness; the algorithm body is unaffected.)

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
