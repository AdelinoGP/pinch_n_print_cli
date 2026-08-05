---
status: implemented
packet: 139_lightning-layer-generator
task_ids:
  - TASK-264
---

# 139_lightning-layer-generator

## Goal

Port the lightning orchestration — `Lightning::Layer` (`generateNewTrees`,
`reconnectRoots`, `convertToLines`) and `Generator` (`generateInitialInternalOverhangs`
+ the two top-down all-layers passes of `generateTrees`, `getTreesForLayer`) — into
`crates/slicer-core/src/algos/lightning/`, wire the packet-137 producer so
`PrePass::LightningTreeGen` commits real per-layer tree segments into
`LightningTreeIR`, and add the per-region refinement that closes
`lightning per-object collapse` (`region_id: RegionId` on `LightningTreeEntry`,
per-region HashMap keying in the host dispatch, `region_id` honored by the SDK
accessor).

## Problem Statement

With the seam (137) and primitives (138) in place, the generator itself is still missing:
the 137 producer commits empty trees, so a lightning-configured print gets no benefit
from the new architecture. The orchestration is where OrcaSlicer's cross-layer semantics
live — the top-down overhang seeding and the two-pass tree growth — and it is the reason
the whole PrePass architecture exists (ADR-0029). This packet makes the seam real.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The two-pass structure is load-bearing (ADR-0029): outlines pass THEN growth pass,
  both top-down over all layers — do not fuse or reorder the passes.
- The 137 skip promise stays: no lightning holder → no generator construction at all.
- Deterministic output (AC-4) — inherits 138's no-hash-iteration rule; the per-layer
  segment ordering frozen at close is 140's input contract.

## Data and Contract Notes

- IR: extends the 137 `LightningTreeIR` with one new field — `region_id:
  RegionId` on `LightningTreeEntry` (mirroring `SupportPlanEntry.region_id:
  RegionId` at `:1129`; `RegionId` is a `pub type RegionId = u64;` alias
  at `slice_ir.rs:36`, so the WIT-boundary plumbing — `region_id.to_string()`
  at `dispatch.rs:1353` — is identical to the support-plan keying). No
  schema-version bump (the additive field is backward-compatible at the IR
  level; existing 137 test fixtures that used `region_id = 0` still parse).
- Determinism: layer iteration strictly top-down by index; per-layer tree iteration
  in creation order; the new per-region keying is `region_id`-integer-sorted
  (matches `SupportPlanEntry.region_id` access pattern at `slice_ir.rs:1129`).

## Locked Assumptions and Invariants

- Two-pass top-down structure preserved (ADR-0029).
- 138 primitive APIs frozen (deviations recorded, tests co-updated).
- Skip promise (no holder → no work) preserved.
- Faithful port: constants ÷ 100, cited; behavioral divergence → `DEVIATION_LOG`.

## Risks and Tradeoffs

- Generator constructor inputs are density-coupled in Orca — the parameterization
  decided in 138 must line up; a mismatch surfaces at Step 1's constants FACT and is
  resolved by adjusting the producer's parameter sourcing (config keys read host-side),
  recorded.
- Synthetic multi-layer fixtures must be small enough to hand-verify continuity — 3-5
  layers, single overhang; resist realistic-model tests here (that is 140's pipeline
  smoke).
- Memory: whole-print `LightningTreeIR`; the compact 2-point storage decision (137)
  bounds it; no further mitigation this packet.
