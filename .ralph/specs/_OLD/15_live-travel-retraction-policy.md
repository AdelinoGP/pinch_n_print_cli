---
status: implemented
packet: live-travel-retraction-policy
task_ids:
  - TASK-120d
  - TASK-120d1
  - TASK-120d2
---

# 15_live-travel-retraction-policy

## Goal

Make `path-optimization-default` the canonical decision surface for live travel, retract/no-retract, and Z-hop policy on the Benchy path, while keeping packet `11` responsible only for how those decisions serialize to final GCode text.

## Problem Statement

The Workstream 3 travel slice is blocked by ambiguity about ownership. This packet resolves that ambiguity explicitly: retraction policy belongs on `path-optimization-default`, not on `DefaultGCodeEmitter`. The emitter serializes commands; it does not decide whether a move should retract. With that ownership fixed, this packet restores external-travel retract decisions, internal-travel suppression, Z-hop planning, and deterministic host integration coverage.

## Architecture Constraints

- Selected approach: retract/no-retract policy lives on `path-optimization-default` because the decision depends on per-layer path adjacency, not final text formatting.
- `DefaultGCodeEmitter` remains out-of-scope for policy; packet `11` serializes whatever commands this packet decides.
- The packet uses the existing and newly-added deferred queues: `deferred_retracts`, `deferred_z_hops`, and `deferred_travel_moves`; it must not invent any further travel-policy channels.
- OrcaSlicer canonical travel sequence (verified in `GCode.cpp:retract()` + `GCodeWriter::travel_to_xyz()`): **Retract → ZHop(up) → Travel Move → Unretract → ZHop(down handled by serializer)**. The module emits commands in this order.

## Data and Contract Notes

- IR or manifest contracts touched:
  - `GCodeCommand::{Move, Retract, Unretract}` — emitted by the module; routed by the dispatch
  - `LayerCollectionIR.z_hops` — fed from `LayerArena.deferred_z_hops`
  - `LayerArena.deferred_retracts` (`DeferredRetract`) — new in this packet
  - `LayerArena.deferred_travel_moves` (`DeferredTravelMove`) — new in this packet
  - config keys: `retract_length`, `retract_speed`, `travel_z_hop` added to the module TOML schema
- WIT boundary considerations:
  - no world widening was needed; the packet uses existing `push_move`, `push_retract`, `push_unretract`, `push_z_hop` builder calls
- Determinism or scheduler constraints:
  - repeated identical inputs must produce identical retract and Z-hop decisions

## Locked Assumptions and Invariants

- Policy ownership is fixed in this packet: `path-optimization-default` decides, packet `11` serializes.
- Packet `20` may reconcile those decisions with finalization geometry later, but it must not move policy ownership again.

## Risks and Tradeoffs

- Risk: internal-versus-external travel classification may need more geometry context than the module currently receives. Mitigation: keep the first implementation and tests on small deterministic fixtures.
- Risk: Z-hop may drift away from retract decisions. Mitigation: assert matched pairing on the host path.
