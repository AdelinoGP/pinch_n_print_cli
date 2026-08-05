---
status: superseded
packet: live-seam-placement-and-consumption
task_ids:
  - TASK-120c
  - TASK-151
superseded_by: 14-rev1_live-seam-placement-and-consumption
---

# 14_live-seam-placement-and-consumption

## Goal

Restore seam placement on real wall-loop seam candidates and teach the live path-optimization surface to consume `PerimeterRegion.resolved_seam` so it replays seam-started wall loops instead of remaining a comment-only slot filler.

## Problem Statement

Seam candidates exist upstream, and `seam-placer` has unit coverage, but the live Workstream 3 path still lacks two linked behaviors: a real committed `resolved_seam` on production wall loops and a path-optimization stage that does something useful with that seam output. Right now the default path-optimization module emits only a marker comment. This packet narrows the gap to the seam slice only: make real wall-loop seam candidates land in `resolved_seam`, then replay seam-started wall loops on the path-optimization surface without pulling in the broader travel-ordering and retraction work.

## Architecture Constraints

- The packet must keep seam placement in `Layer::WallPostProcess` and seam consumption in `Layer::PathOptimization`; it must not collapse the stages.
- Selected approach: expand `path-optimization-default` just enough to replay seam-started wall loops from `resolved_seam`, while leaving broader travel-ordering work to later packets.
- Because TASK-151 explicitly calls out `path-optimization-default`, the seam-consumption fix must live on that module surface, not only on a host-side pre-sort helper.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `PerimeterIR.regions[*].seam_candidates`
  - `PerimeterIR.regions[*].resolved_seam`
  - replayed wall-loop `GCodeCommand::Move` output on the path-optimization surface
- WIT boundary considerations:
  - the packet may require a narrow acceptance change on the path-optimization commit surface so replayed wall-loop moves are no longer treated as unsupported overrides
- Determinism or scheduler constraints:
  - replayed wall loops must be deterministic for repeated identical inputs

## Locked Assumptions and Invariants

- Seam placement stays upstream of travel/retraction policy.
- Packet `11` owns final text formatting, so this packet stops at replayed commands and layer-stage output.

## Risks and Tradeoffs

- Risk: move replay support can accidentally widen the path-optimization surface too much. Mitigation: limit the packet to seam-started wall-loop replay only.
- Risk: seam commitment may pass in module tests but fail in host dispatch. Mitigation: keep the host `resolved_seam` test as a required acceptance gate.
