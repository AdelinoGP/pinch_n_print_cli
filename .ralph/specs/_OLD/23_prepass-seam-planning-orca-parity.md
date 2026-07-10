---
status: superseded
packet: 23_prepass-seam-planning-orca-parity
task_ids:
  - TASK-159
superseded_by: 23-rev1_prepass-seam-planning-orca-parity
---

# 23_prepass-seam-planning-orca-parity

## Goal

Introduce an explicit `PrePass::SeamPlanning` contract that computes Orca-inspired seam choices once from global mesh and layer-plan context, stores them in a new `SeamPlanIR`, and feeds those chosen seams into `Layer::PerimetersPostProcess` as an apply-only surface. This packet is intentionally separate from packet `22`: packet `22` restores the current live layer-stage seam contract, while packet `23` adds the larger scheduler/IR/WIT slice required for deeper Orca parity.

## Problem Statement

Packet `22` restores the current layer-stage seam contract, but it does not close the larger Orca parity gap: seam selection still happens too late and with too little global context. OrcaSlicer’s seam initialization logic scores candidate points against global mesh and planning context before the live wall-application phase. Pinch 'n Print currently has no prepass seam-planning stage, no blackboard slot for planned seams, and no mechanism to inject a precomputed seam choice into `Layer::PerimetersPostProcess`.

This packet defines that missing architecture slice:

1. add `PrePass::SeamPlanning` to the canonical stage order
2. add `SeamPlanIR` as a host-owned prepass artifact keyed by `RegionKey`
3. extend `world-prepass` / SDK / macro glue so a prepass module can emit planned seams
4. add `seam-planner-default` as a claim-free prepass core module
5. keep `seam-placer` as the apply-stage module, but feed it chosen seams from `SeamPlanIR` instead of rescoring live

## Architecture Constraints

- The new stage must slot into the fixed prepass order without breaking existing `required_slots(...)` rules
- `SeamPlanIR` must be write-once on the blackboard like the other prepass artifacts
- The packet must preserve the existing `seam-placer` claim semantics from `docs/01_system_architecture.md`; the new planner module is therefore claim-free in this design
- The layer-stage WIT world stays stable for packet `23`; planned seams are injected into the existing `PerimeterRegionView.resolved_seam` surface rather than adding a second layer-world handle

## Data and Contract Notes

- `SeamPlanIR.entries[*].region_key.{global_layer_index, object_id, region_id}` is the stable join key between prepass planning and the layer apply stage
- `SeamPlanIR.entries[*].chosen_candidate` uses the existing `SeamPosition` shape so the layer stage can reuse current seam application code
- `SeamPlanIR.entries[*].scored_candidates` keeps the scored candidate list for debugging, evidence, and deterministic regression checks
- Layer-stage injection happens before the guest sees `PerimeterRegionView`, so the apply module still reads `resolved_seam()` through the existing WIT surface

## Risks and Tradeoffs

- Adding a new prepass stage widens scheduler, manifest, blackboard, and SDK surfaces in one packet; this is why the packet remains draft until explicitly scheduled
- The planner module is claim-free by design, which preserves current `seam-placer` claim stability but means claim policy for prepass seam planning is intentionally deferred
- Using the existing prepass host-services may be slower than a future host-precomputed seam-visibility map, but it keeps the packet inside current host-service capabilities

## Locked Assumptions and Invariants

- packet `22` lands first and becomes the apply-stage baseline
- `SeamPlanIR` is write-once on the blackboard and keyed by deterministic `RegionKey`
- the layer-world WIT surface remains stable; prepass planning is injected into the existing `resolved_seam` view
