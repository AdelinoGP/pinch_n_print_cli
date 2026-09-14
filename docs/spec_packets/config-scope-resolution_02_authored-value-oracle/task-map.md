# Task Map: authored-value-oracle

This single-task packet emits a task map because the approved batch requires all five packet artifacts and TASK-563 crosses registry, model-input, scheduler-dedup, runtime-plan, and executor-test seams.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-563` | Steps 1-3 | config-scope-resolution plan row 2; ADR-0067; ADR-0068; `docs/04_host_scheduler.md`; `docs/22_test_quality.md` | Runtime executor oracle + aggregator registration + net-new `slicer-config` dev-dependency | None — no source port | `M` | Proves exact registry-typed authored values at each manifest-derived live owner across the arachne/classic dedup matrix; intentionally red until packet 03. |
