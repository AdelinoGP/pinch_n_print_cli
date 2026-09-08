---
status: implemented
packet: 221-tree-support-family
task_ids:
  - TASK-332
---

# 221-tree-support-family

## Goal
Split the existing tree planner into `tree-support-planner` and make its paired `tree-support` renderer emit distributed, collision-safe structural support bodies and printable anchored events.

## Problem Statement
The live `support-planner` is tree-specific (`modules/core-modules/support-planner/support-planner.toml:2-17`) but is globally selected even for traditional support; family selection is now per-region via the `support_family` canonical key (`support_type` aliases resolve `normal*`/`classic*` → `traditional`, `tree*`/`hybrid*` → `tree`). The live tree renderer is `Layer::Support`, and the renderer must emit attributed `SupportIR` v2.0.0 (per body/role: family_id, body_id, demand_ids, object/region, role incl. raft+ironing, printable paths) rather than flat support output (`modules/core-modules/tree-support/tree-support.toml:9-18`). This packet gives that algorithm an explicit family boundary and makes geometry structural before rendering.

## Architecture Constraints
- `SupportPlanIR` is structural; no `ExtrusionPath3D` branch path is emitted.
- ADR-0009's single-writer rule, that `support-planner` keeps sole ownership of `SupportPlanIR`, is preserved by hosting that ownership in the host aggregator: it is the sole writer of the aggregated `SupportPlanIR`, while each family planner emits only family-scoped plan entries for host aggregation. The named-owner change is recorded as an explicit amendment in ADR-0059 (which supersedes ADR-0009's single-writer clause); this packet conforms to ADR-0059 and introduces no second writer.
- Tree planner and renderer selection is atomic and uses `support-family:tree`; `tree*` and `hybrid*` aliases resolve to it.
- Invalid complete bodies are dropped, never clipped or replaced by filler.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
