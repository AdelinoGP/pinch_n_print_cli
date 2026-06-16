# Task Map: 73_support-geometry-normalization

This packet implements the `TASK-163c` WIT-boundary fix for `run-support-geometry` (the support-geometry / TASK-163 cluster): it adds `config: config-view` and a `result<_, module-error>` return to the export, fixing the empty-`ConfigView` injection and the swallowed planner `Result`. It is **distinct from `TASK-166`**, which threaded resolved config into `RegionMapIR`/`RegionPlan` at a different layer. It also depends on packet 72 (the canonical `world-prepass.wit` it edits). The mapping records which step is sufficient evidence.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-163c` (WIT-boundary config/error normalization for run-support-geometry) | Step 1, Step 2, Step 3 | `docs/03_wit_and_manifest.md`, `docs/04_host_scheduler.md` | `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-runtime/src/{dispatch.rs,wit_host.rs}` | none | M | Sufficient when `run-support-geometry` takes `config-view` end-to-end (AC-1) and the empty-`ConfigView` injection + error-swallow are removed (AC-3, AC-4), proven by config now reaching the guest (AC-2) and disabling it (AC-N1). |
| `TASK-163c` (behavior verification) | Step 4 | `docs/02_ir_schemas.md` | `crates/slicer-runtime/tests/support_geometry_config_normalization_tdd.rs` | none | M | Sufficient when AC-2 (raft honored), AC-N1 (`support_enabled=false` → 0 entries), AC-N2 (fatal → `DispatchError`) pass and the enabled path (AC-6) shows no regression. |

Aggregate context cost across rows: `M`. No cell is `L`. Prerequisite: packet 72 `implemented`. `requirements.md` §Problem Statement records the TASK-163c scoping (distinct from TASK-166's RegionMapIR-layer work).
