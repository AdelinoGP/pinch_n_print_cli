# Task Map: scope-resolution-module

This single-task crosswalk is emitted because TASK-566 spans the resolver, two production consumers, and one coordinated WIT major bump.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-566` | `Steps 1–5` | `docs/02_ir_schemas.md`, `docs/04_host_scheduler.md`, ADR-0068 | `slicer-config` resolution module; scheduler/core/runtime migration and tests | None; current-scope behavior is repository-owned | `M` | Replaces five scheduler resolvers plus `overlay_resolved`, exposes Z-grid/scope-stack queries, and converges both entry points. |
| `TASK-566` | `Steps 6–9` | `docs/03_wit_and_manifest.md`, `docs/11_operational_governance_and_acceptance_gate.md` | layer-planning WIT/schema/macro/SDK/host/guest surfaces | None | `M` | Adds the exact five-field record and performs the accepted `1.0.0` → `2.0.0` package bump once. |
| `TASK-566` | `Steps 10–11` | plan Resolution/Layer range/Guests sections; docs 02/03/04 | docs plus delegated freshness/closure gates | `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` functions `layer_height_profile_from_ranges`/`layer_height_profile` | `M` | Locks later-starting `layer_height` precedence and conflicting non-layer-height load error for row 9 without claiming a current Rust range implementation. |

Costs match `implementation-plan.md`; no row or step is L.
