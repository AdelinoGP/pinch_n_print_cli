---
status: implemented
packet: 138_lightning-distancefield-treenode
task_ids:
  - TASK-263
---

# 138_lightning-distancefield-treenode

## Goal

Port the two lightning primitives into the packet-137 `algos/lightning/` home:
`DistanceField` (unsupported-cell grid: seeding, nearest-unsupported queries, radius-consuming
updates; from `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.{hpp,cpp}`) and
`TreeNode` (tree graph: attachment, `propagateToNextLayer`, straightening, rerooting, pruning;
from `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.{hpp,cpp}`), TDD'd against
hand-computed small cases with all length constants divided by 100 per the PnP coordinate
system.

## Problem Statement

Full lightning parity (roadmap decision, 2026-07-01 grilling) requires the canonical
generator, and the generator is built from two primitives the workspace lacks: the
unsupported-cell `DistanceField` that decides **where** trees must grow, and the `TreeNode`
graph that decides **how** they grow, straighten, reroot, and prune across layers. These
are the subtlest 750 lines of the 2,175-LOC OrcaSlicer port (the plan's "3,317 LOC" figure
overstates the source files; verified: `DistanceField.{hpp,cpp}` = 383, `TreeNode.{hpp,cpp}`
= 750, `Layer.{hpp,cpp}` = 540, `Generator.{hpp,cpp}` = 423, `FillLightning.{hpp,cpp}` =
79). Landing them alone, with hand-computed unit cases, keeps packet 139's orchestration
port mechanical instead of monolithic, and the 750-line TreeNode is split across
attachment/propagate/straighten/reroot/prune sections so no single dispatch is ever a
whole-file dump.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Determinism: NO hash-container iteration anywhere in the primitives (Orca's own port
  uses ordered structures; PnP mirrors that) — 139's whole-print determinism test
  depends on it.
- Attribution headers mandatory (ported code; see `docs/ORCASLICER_ATTRIBUTION.md`).

## Data and Contract Notes

- IR/WIT/manifest: none touched.
- Public API freeze at packet close: `DistanceField::{new, unsupported_point/next, update}`,
  and the `tree_node` graph operations used by 139 — signature changes after close are
  139-recorded deviations with 138 tests co-updated in the same step.
- Resolution: `DistanceField` takes `supporting_radius` as a constructor parameter;
  `m_cell_size = supporting_radius / 6` is derived internally from Orca's
  `radius_per_cell_size = 6`. There is no density-derived resolution in 138.

## Deviations

- 138 ships `propagate_to_next_layer` with the realign step stubbed. The `next_outlines` and
  `outline_locator_resolution` parameters are accepted for API stability but unused; 139's
  `Layer` will fill in the real outline-snap. AC-2 tests the prune+straighten path only; the
  realign path is not exercised until 139.

## Locked Assumptions and Invariants

- Faithful port: behavioral divergence from the Orca primitives requires a
  `DEVIATION_LOG` entry — there is no "improvement" license here (NaN guards and safety
  checks excepted, following the gyroid precedent).
- All distance constants ÷ 100, cited by canonical Orca function name in test comments.
- Deterministic iteration everywhere (no HashMap/HashSet in any hot loop).

## Risks and Tradeoffs

- TreeNode ownership mapping is the port's hardest translation; `Rc<RefCell<Node>>` is the
  selected mapping because the graph has no back-edges.
- Hand-computed test cases can encode a misreading of the C++ — mitigation: each
  behavioral test cites the section dispatch (date + section) its expectation came from,
  making the chain auditable.
- Grid resolution is derived internally from the `supporting_radius` constructor parameter;
  138 does not add density-derived resolution.
