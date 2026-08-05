---
status: implemented
packet: 178-seam-region-aware-planning
task_ids:
  - TASK-294
supersedes: ../168-seam-aligned-modes/packet.spec.md
---

# 178-seam-region-aware-planning

## Goal

Make `PrePass::SeamPlanning` consume real per-active-region `SliceIR` geometry and preserve the full `RegionKey` identity through WIT, harvest, blackboard injection, and per-layer seam placement.

## Problem Statement

Packet 168 implemented aligned planning over mesh-derived contours before
region mapping and assigned `perimeter_idx.to_string()` as `region_id`. That
works for the single-region prism fixture but cannot address PNP active regions,
painted variants, or their `variant_chain` identities. The follow-up must replace
that identity/source mismatch without moving cross-layer alignment into a
parallel per-layer module or a host builtin.

## Architecture Constraints

- Cross-layer alignment remains in `PrePass::SeamPlanning`; per-layer `seam-placer` remains a consumer and final-geometry adapter.
- The new prepass input is a read-only projection of committed `SliceIR`/region data; it must not create a second mutable blackboard channel.
- `RegionKey` identity is `(global_layer_index, object_id, region_id, variant_chain)` everywhere. A numeric region ID is not sufficient when variants coexist.
- The existing `SeamPlanIR` duplicate-key validation remains authoritative; malformed identity is a contract error, not a best-effort drop.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: `SeamPlanIR` already owns full `RegionKey` in Rust but current WIT harvest reconstructs an empty `variant_chain`; this packet closes that loss. Perimeter regions must expose the same identity before host injection can be exact. The `SeamPlanEntry` adds a `variant_chain` field; the additive minor bump of `CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION` 1.0.0 → 1.1.0 must be made in the same step that touches `SeamPlanEntry` (no AC hardcodes the literal — the field-addition is the assertion, the version follows per `docs/11` policy).
- WIT boundary: adding required fields/parameters to `slicer:world-prepass` and shared `ir-handles` is a major world change and rebuilds all prepass guests. The `seam-plan-entry` WIT record gains a `variant-chain: list<tuple<string, paint-value>>` field and a new per-region input record/resource is added. World version bumps 2.0.0 → 3.0.0 (AC-1 pins the literal).
- Determinism/scheduler constraints: projection ordering is ascending `(layer, object, region, variant_chain)`; phase routing must ensure SliceIR and region data exist before dispatch; no map iteration may choose plan order.

## Locked Assumptions and Invariants

- No aligned plan is keyed by contour ordinal.
- No active-region plan is broadcast to another variant.
- Inactive regions produce no plan entry.
- `variant_chain` order and values survive guest -> host -> IR -> layer lookup unchanged.
- Existing wall-preservation behavior remains unchanged in this packet.

## Risks and Tradeoffs

- Adding variant identity to perimeter IR may require a minor schema bump and broad struct-literal fallout.
- Moving seam planning to the late prepass phase changes timing but not the module claim; the scheduler must still run it before any layer stage.
- Per-region SliceIR polygons are closer to canonical source than mesh contours but remain upstream of final inset walls; packet 3 owns the final projection mitigation.
