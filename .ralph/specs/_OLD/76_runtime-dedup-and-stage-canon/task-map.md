# Task Map: 76_runtime-dedup-and-stage-canon

Bridge from packet ACs back to `docs/07_implementation_status.md` backlog task
IDs. Packet 76 spans three task IDs (two implemented, one deferred), so this
file is required per the spec-packet-generator convention.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-220` | 3a (wildcard matcher), 3b (config map), 3c (transform), 3d (stage-order canon) — covers AC-3a / AC-3b / AC-3c / AC-3d | none (refactor) | `crates/slicer-runtime/src/{execution_plan,manifest,validation,dag_cli,gcode_emit,dispatch,mesh_analysis,paint_segmentation,model_loader,prepass_slice}.rs`, new `crates/slicer-runtime/src/stage_order.rs`, `crates/slicer-ir/src/resolved_config.rs`, `crates/slicer-core/src/lib.rs`, new `crates/slicer-runtime/tests/unit/stage_canon_seam_support_tdd.rs` | none | `M` | Four dedup invariants + the SeamPlanning/SupportGeometry/PaintRegionAnnotation startup-validation bug fix. Sufficient evidence: each AC's grep gate + targeted test suite green; net diff is dedup (no new abstractions). |
| `TASK-221` | 1a (region single-pass), 1b (pipeline core) — covers AC-1a / AC-1b | none (refactor) | `crates/slicer-runtime/src/region_mapping.rs`, `crates/slicer-runtime/src/pipeline.rs`, `crates/slicer-runtime/tests/integration/run_pipeline_with_instrumentation_tdd.rs` | none | `S` | Region mapping single-pass via host-config Option threading; pipeline-core extraction with `run_pipeline_with_events` deliberately kept standalone (CONFIG_BLOCK behavioural diff, locked by `pipeline_tdd`). Sufficient evidence: `e2e` + `pipeline_tdd` + `dispatch_tdd` green. |
| `TASK-222` | Candidate 2 — DEFERRED, NOT RECOMMENDED — covered by AC-2 (no-op) | new `docs/adr/0003-macro-per-world-wit-conversions.md` | _none in this packet_ | none | `S` (deferred, doc-only) | Macro WIT↔IR conversion dedup deferred and reassessed as not worth implementing — headline benefit (unit-testable conversions) impossible under per-world `wit_bindgen` (ADR-0003); duplication is compiler-guarded and low-churn. ADR-0003 captures the durable insight. Full rationale in `implementation-plan.md` §TASK-222. |

Aggregate context cost: `M` (TASK-220 alone is the largest at `M`; TASK-221 is
`S`; TASK-222 is doc-only). No step is rated `L`.
