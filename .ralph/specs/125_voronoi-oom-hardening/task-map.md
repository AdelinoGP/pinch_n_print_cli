# Task Map: 125_voronoi-oom-hardening

No `docs/07` `TASK-###` ids back this packet (`task_ids: []`) — it is a diagnose-driven bug-fix slice
expanded in place. This crosswalk maps each Part to its governing docs, OrcaSlicer refs, and ACs so a
reviewer can navigate without re-deriving scope.

| Part | Concern | Governing docs | OrcaSlicer ref | ACs | Primary surface |
|---|---|---|---|---|---|
| A | `region_id`↔tool split | `docs/02` §IR 10; `docs/03` (ordered-entity-view, print-entity-view, finalization push) | — | AC-1…AC-6 | `slice_ir.rs`, `layer_executor.rs`, two WIT worlds, host/SDK/macro, emit, 4 guests |
| B | D14 fuzzy routing | `docs/03` (slice-region-view.variant-chain); D14 (`95_paint-segmentation` closure-log) | — | AC-7, AC-8 | `paint_segmentation/mod.rs`, `ir-types.wit`, `perimeter_utils.rs`, arachne/classic guests |
| C | per-tool config (`tool_config:`) | `docs/02` §Config Key Namespaces (precedence) | `PrintApply.cpp`, `PrintObject.cpp`, `Flow.cpp`/`PerimeterGenerator.cpp` | AC-9…AC-12 | `config_resolution.rs`, `emit.rs`, `region_mapping.rs`, `run.rs`/`prepass.rs` |
| D | boostvoronoi input guard | `OOM_FINDINGS` (historical) | — | AC-13 | `voronoi_graph.rs` |
| E | fpv-panic containment | — | — | AC-14 | `voronoi_graph.rs` |
| safety net | retained guards | — | — | AC-N1, AC-N2 | `emit.rs`, executor allocator tripwire |

## Supersession

- This packet supersedes its own `HANDOFF.md` (the historical "defer the split" note). The deferred
  split is implemented here; `HANDOFF.md` carries a superseded banner pointing back to this packet.
- The original `packet.spec.md` Doc Impact `none` + "separate refactor" scoping is falsified and
  recorded as deviation `D-125-TOOL-IDENTITY-SPLIT` in `docs/DEVIATION_LOG.md`.
