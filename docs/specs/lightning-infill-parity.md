# Lightning Infill — Full OrcaSlicer Parity via PrePass Tree Generation

## Context

Companion to `docs/specs/infill-parity-rectilinear-gyroid-linker.md`. The 2026-07-01 grilling
expanded the infill-parity roadmap to full OrcaSlicer lightning parity (user decision),
replacing the 512-LOC single-layer stub in `modules/core-modules/lightning-infill/` with the
canonical cross-layer algorithm. Architecture: ADR-0029 — a host-side
`PrePass::LightningTreeGen` producer builds `LightningTreeIR`; the per-layer module samples it
and emits raw polylines for the infill-linker (ADR-0025). Closes DEV-081.

Packet mapping: `137_lightning-prepass-contract` (Phase L1), `138_lightning-distancefield-treenode`
(Phase L2), `139_lightning-layer-generator` (Phase L3), `140_lightning-module-rewrite` (Phase L4).

## Authoritative references

### ADRs
- `docs/adr/0029-lightning-prepass-tree-generator.md` — the architecture decision (read first).
- `docs/adr/0025-infill-linker-as-raw-emit-post-pass.md` (+ amendment) — raw-emit contract.

### OrcaSlicer canonical sources (3,317 LOC total; delegate all reads)
- `OrcaSlicerDocumented/src/libslic3r/Fill/FillLightning.hpp` (128) / `.cpp` (151) —
  `build_generator` (per-object factory, cpp:145), `Filler::_fill_surface_single` (per-layer
  sampling + the fill call shape).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/DistanceField.hpp` (219) / `.cpp` (225) —
  unsupported-cell tracking grid; seeds tree growth.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/TreeNode.hpp` (471) / `.cpp` (629) —
  tree node graph: `propagateToNextLayer`, straightening, rerooting, pruning.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Layer.hpp` (171) / `.cpp` (587) —
  per-layer tree set: `generateNewTrees`, `reconnectRoots`, `convertToLines`.
- `OrcaSlicerDocumented/src/libslic3r/Fill/Lightning/Generator.cpp` (475) / `.hpp` (261) —
  orchestration: `generateInitialInternalOverhangs` + `generateTrees` (cpp:189-190,342; two
  full top-down all-layers passes), `getTreesForLayer`.

### PnP codebase
- `modules/core-modules/lightning-infill/src/lib.rs` (512) — current stub; self-link at
  `build_branches` (lib.rs:234, branch construction lib.rs:265). Tests:
  `tests/lightning_infill_tdd.rs` (323), `tests/slicer_module_binding_tdd.rs`.
- `crates/slicer-core/src/algos/support_geometry.rs:93` — the PrePass host-producer pattern.
- `crates/slicer-ir/src/slice_ir.rs:1046` — `SupportPlanIR` (IR precedent).
- `crates/slicer-scheduler/src/execution_plan.rs:19-41` — `STAGE_ORDER` (gains
  `PrePass::LightningTreeGen`).

## Phase L1 — PrePass stage + `LightningTreeIR` + WIT read-view (packet 137)

- Add `PrePass::LightningTreeGen` to `STAGE_ORDER` after the stages producing sparse-infill
  outlines and before `Layer::Infill` dispatch; host producer skeleton in
  `crates/slicer-core/src/algos/lightning/mod.rs` (returns empty IR until L3 wires the
  generator).
- `LightningTreeIR` in `crates/slicer-ir/src/slice_ir.rs`: schema-versioned; per object, per
  layer: tree-edge segments (integer units, 2-point) + locators sufficient for per-layer
  sampling. Document in `docs/02_ir_schemas.md`.
- WIT read-view so the lightning module can read its layer's trees at `Layer::Infill`
  (pattern: how support modules read `SupportPlanIR` views). Guest rebuild ceremony per
  CLAUDE.md WIT checklist.
- Producer skip: zero work when no region's `sparse_fill_holder` is `lightning-infill`.

## Phase L2 — `DistanceField` + `TreeNode` port (packet 138)

- Port `DistanceField` and `TreeNode` into `crates/slicer-core/src/algos/lightning/`
  (host-side; OrcaSlicer attribution header per `docs/ORCASLICER_ATTRIBUTION.md`).
- TDD per structure: cell seeding/consumption for `DistanceField`; propagate/straighten/prune
  invariants for `TreeNode`. Divide OrcaSlicer distance constants by 100
  (`docs/08_coordinate_system.md`).

## Phase L3 — `Lightning::Layer` + `Generator` port + producer wiring (packet 139)

- Port `Layer` (`generateNewTrees`, `reconnectRoots`, `convertToLines`) and `Generator`
  (`generateInitialInternalOverhangs`, `generateTrees`, `getTreesForLayer`).
- Wire the L1 producer to run the generator per object and commit `LightningTreeIR`.
- Cross-layer behavior tests: overhang seeding from layer N+1 outlines; tree continuity
  between adjacent layers; determinism (two runs → identical IR).

## Phase L4 — Module rewrite: sample + raw emit (packet 140)

- Rewrite `modules/core-modules/lightning-infill/src/lib.rs`: read the layer's trees from the
  L1 view, emit raw branch polylines (`ExtrusionPath3D`, SparseInfill role, speed factor from
  config). Delete `build_branches` self-linking and the grid sampler. No clipping (linker
  re-clips), no chaining (linker connects).
- Close DEV-081 in `docs/DEVIATION_LOG.md`.
- Contained golden re-bless for lightning-bearing fixtures (the roadmap's main bless happened
  at packet 136).

## Validation (per phase; packet ACs carry the narrow commands)

```bash
cargo build -p slicer-core && cargo test -p slicer-core lightning   # L2/L3
cargo xtask build-guests --check                                     # L1/L4 (WIT/module edits)
cargo test -p lightning-infill                                       # L4
cargo test -p slicer-runtime --test contract                         # L1 view contract
```

## Risks

- **Port size.** 3,317 LOC C++ is the largest single port in the infill roadmap; L2/L3 split
  keeps each packet's steps ≤ M. TreeNode's rerooting/straightening logic is the subtle core —
  TDD it against small hand-computed trees.
- **Memory.** Whole-print tree storage; keep 2-point integer segments (see ADR-0029).
- **Determinism.** OrcaSlicer uses stable iteration; preserve ordering when porting hash-based
  containers (use BTree/sorted structures).
