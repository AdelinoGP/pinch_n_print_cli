---
status: implemented
packet: 222-traditional-support-family
task_ids:
  - TASK-333
---

# 222-traditional-support-family

## Goal
Add a `traditional-support-planner` that plans cross-layer contact, base, interface, obstacle, and termination geometry, and make `traditional-support` render only its paired structural polygons.

## Problem Statement
The live traditional renderer is `Layer::Support` but writes `SupportIR` without reading `SupportPlanIR` (`modules/core-modules/traditional-support/traditional-support.toml:9-18`). This packet introduces a genuine cross-layer traditional planner and removes per-layer eligibility/filler behavior from the renderer. Packet 220 already removed the fallback filler; this packet migrates the renderer to attributed `SupportIR` v2.0.0 (per body/role: family_id, body_id, demand_ids, object/region, role incl. raft+ironing, printable paths).

## Architecture Constraints
- Traditional planner owns contact detection, propagation, interfaces, obstacles, and termination; renderer owns no eligibility algorithm.
- `SupportPlanIR` contains structural polygons and roles, never nozzle-width paths.
- Selection is atomic through `support-family:traditional`; `normal*` and `classic*` aliases map to it.
- Invalid complete bodies are dropped, never clipped or replaced by fallback filler.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
