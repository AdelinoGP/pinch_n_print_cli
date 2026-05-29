# Task Map: 73_support-geometry-normalization

This packet continues `TASK-166`'s config-threading intent at a **different layer** than the task's own work (TASK-166 resolved config into `RegionMapIR`/`RegionPlan`; this fixes the `run-support-geometry` **WIT export**, which still injects an empty `ConfigView` and swallows the planner's `Result`). It also depends on packet 72 (the canonical `world-prepass.wit` it edits). The mapping records which step is sufficient evidence.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-166` (thread resolved config through to downstream stages) | Step 1, Step 2, Step 3 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `crates/slicer-schema/wit/world-prepass.wit`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-runtime/src/{dispatch.rs,wit_host.rs}` | none | M | Sufficient when `run-support-geometry` takes `config-view` end-to-end (AC-1) and the empty-`ConfigView` injection + error-swallow are removed (AC-3, AC-4), proven by config now reaching the guest (AC-2) and disabling it (AC-N1). |
| `TASK-166` (behavior verification) | Step 4 | `docs/02_ir_schemas.md` | `crates/slicer-runtime/tests/support_geometry_config_normalization_tdd.rs` | none | M | Sufficient when AC-2 (raft honored), AC-N1 (`support_enabled=false` → 0 entries), AC-N2 (fatal → `DispatchError`) pass and the enabled path (AC-6) shows no regression. |

Aggregate context cost across rows: `M`. No cell is `L`. Prerequisite: packet 72 `implemented`. `requirements.md` §Problem Statement records why this is distinct from TASK-166's RegionMapIR-layer work.
