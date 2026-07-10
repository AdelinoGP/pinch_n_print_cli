---
status: implemented
packet: live-top-bottom-surface-fill
task_ids:
  - TASK-120a
---

# 12_live-top-bottom-surface-fill

## Goal

Restore live top and bottom surface fill generation on the real Benchy path by ensuring top-surface, bottom-surface, and bridge-sensitive fill regions reach the canonical infill generator and survive into `LayerCollectionIR.ordered_entities` with the exact `ExtrusionRole` variants the GCode emit path expects.

## Problem Statement

The real Benchy path still misses top and bottom surface fill even though the host owns the slice, surface-classification, and infill-stage plumbing required to produce it. The coherent slice here is not generic infill feature work. It is the specific live-path gap between classified top/bottom surfaces and the canonical `rectilinear-infill` generator so that real `TopSolidInfill`, `BottomSolidInfill`, and `BridgeInfill` paths reach `LayerCollectionIR`.

## Architecture Constraints

- The packet stays on the canonical Benchy-path generator: `rectilinear-infill`.
- Restored fill must use the exact `ExtrusionRole` variants already declared in `slicer-ir`; do not invent new role names.
- Host integration must prove those roles survive into `LayerCollectionIR.ordered_entities`, because packet `11` consumes that surface.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `SliceRegionView.infill_areas`
  - `InfillIR.regions[*].solid_infill`
  - `ExtrusionRole::{TopSolidInfill, BottomSolidInfill, BridgeInfill, SparseInfill}`
  - `LayerCollectionIR.ordered_entities[*].role`
- WIT boundary considerations:
  - no world change is expected; the packet stays inside existing layer-world infill inputs and outputs
- Determinism or scheduler constraints:
  - output ordering must remain deterministic so repeated runs preserve the same role sequence

## Locked Assumptions and Invariants

- `rectilinear-infill` is the canonical infill generator for the live Benchy path.
- Packet `11` consumes these roles for final text emission, so this packet must preserve exact `ExtrusionRole` values.

## Risks and Tradeoffs

- Risk: surface classification may already exist but not reach the module. Mitigation: add the host integration test before broad code changes.
- Risk: bridge fill can regress into sparse fill silently. Mitigation: direct `BridgeInfill` assertions.
