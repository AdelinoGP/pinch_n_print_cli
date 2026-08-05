---
status: implemented
packet: 140_lightning-module-rewrite
task_ids:
  - TASK-265
---

# 140_lightning-module-rewrite

## Goal

Rewrite `modules/core-modules/lightning-infill` as a per-layer sampler: read the layer's
tree segments from the packet-137 / packet-139 `LightningTreeIR` view (accessed via
the `PaintRegionLayerView` SDK accessor `lightning_tree_segments_for(object_id,
region_id)`, which 139 upgrades to per-region keying), emit them as raw
`ExtrusionPath3D` polylines with `ExtrusionRole::SparseInfill` and the config-derived
`speed_factor`, delete the single-layer stub (the `build_branches` function at
`lib.rs:234` and the inline grid-sampling machinery in `run_infill`/`fill_expolygon`),
**port the full `getBestGroundingLocation` grounding search into
`crates/slicer-core/src/algos/lightning/layer.rs` (closing the 139 Step-2
`lightning grounding-search stub` stub — `wall_supporting_radius` becomes a
load-bearing parameter)**, close lightning raw-emit deviation, run the contained lightning re-bless +
roadmap-close workspace ceremony, and **close the infill paint-view contract
deviation** by extending the WIT `run-infill` signature with a
`paint: paint-region-layer-view` argument, bumping `slicer:world-layer@2.2.0` →
`@2.3.0`, threading the paint view through the SDK trait + macro glue + host
dispatch + the four `run_infill`-implementing core modules, and adding a real
`Layer::Infill` test-guest that calls `lightning-tree-segments` through the WIT
boundary.

## Problem Statement

Everything behind the seam is real (137–139: stage, IR, primitives, generator), yet
lightning prints still come from the 512-LOC single-layer stub — grid samples joined to
the nearest boundary, self-linked in violation of ADR-0025 (lightning raw-emit deviation), with none of the
canonical cross-layer tree behavior. The stub is also the roadmap's last self-linking
module: until it emits raw, the linker's "one place linking happens" invariant has a
standing exception that paths cannot even be detected (no module identity on paths).
This packet deletes the stub, samples the committed trees, and closes lightning raw-emit deviation —
completing both the lightning-parity sub-roadmap and Architecture A's uniformity.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- ADR-0029 module-sampler contract: NO generation, NO clipping, NO chaining in the
  module — sample and emit raw; the linker (133) clips and connects.
- **Generation-sampling boundary (revised):** 140 is "the lightning packet"
  and owns both sides of the per-layer seam. The generation side
  (`crates/slicer-core/src/algos/lightning/`) gets the full grounding
  search (Step 0) so the sampling side samples higher-quality trees.
  Cross-layer `Generator`, `DistanceField`, and the producer stay frozen
  for this packet — only `Layer::generate_new_trees` (and the
  `getBestGroundingLocation` helper it delegates to) and the
  `Node`-level surface used by the grounding search are in scope.
  Any change to other 138/139 surface is a recorded deviation.
- Raw-emit uniformity (ADR-0025 + Amendment): this packet removes the roadmap's last
  self-linking exception; nothing may reintroduce path connection here.
- WIT version-bump semantics: `slicer:world-layer@2.2.0` → `@2.3.0` follows the
  2.0.0→2.1.0→2.2.0 chain and is consistent with packet 130's DEV-084 precedent
  (a minor→major correction for an additive export-arg change). The version bump
  is purely advisory under ADR-0044 — no test mechanically detects a missed
  version — so the packet must rely on the doc-update checklist and the explicit
  `wit_drift_detection_tdd` string assertion, not the package version itself.
- The `cargo xtask test --workspace` ceremony only via `--summary` dispatch (CLAUDE.md).

## Data and Contract Notes

- IR/WIT: the view is unchanged from 139's per-region keying; this packet's
  WIT-extension is a transport change (the `run-infill` signature now takes
  a paint view that carries the view methods including
  `lightning-tree-segments`). The IR is unchanged from 139. If the view proves
  insufficient (e.g. missing per-tree grouping the sampler needs), that is a
  137/139-contract deviation — minor bump routed through a recorded deviation,
  not an inline hack.
- Emission: mm at `ExtrusionPath3D` (`f32` `points: Vec<Point3WithWidth>`) from
  integer-unit IR segments via `slicer_ir::units_to_mm(...)` (the one mm↔unit boundary
  in the packet). z derived from the dispatching `layer_index` and the layer Z table.
- Determinism: emission order = IR segment order (frozen by 139).
- WIT version: 2.2.0 → 2.3.0. The bump is purely advisory under ADR-0044;
  re-baseline `wit_drift_detection_tdd` for the new package version AND the new
  `run-infill` signature, do not rely on the package version itself.

## Locked Assumptions and Invariants

- Manifest stays `holds = ["claim:sparse-fill"]`.
- No generation/clipping/chaining in the module — sampler only.
- Empty trees → empty emission, slice completes (AC-N2) — no stub fallback exists.
- Non-lightning output byte-identical (AC-N1).
- lightning raw-emit deviation closes here or the packet does not close.

## Risks and Tradeoffs

- The old test suite encodes stub semantics wholesale — rewriting it risks losing
  genuine invariants (module binding, role tagging, origin discipline); those
  specific tests are kept/adapted, and each deletion names the stub behavior it
  encoded.
- Linked-lightning visual quality is new territory (no OrcaSlicer golden to compare
  linked output against, since Orca links differently) — the bless justification
  leans on AC-1 (sampling fidelity) + AC-3 (pipeline integrity) + the HTML-report
  visual note.
- The roadmap-close ceremony may surface cross-packet debt; triage per the fence,
  record honestly (the packet-126 lesson: never flip closure before the ceremony).
