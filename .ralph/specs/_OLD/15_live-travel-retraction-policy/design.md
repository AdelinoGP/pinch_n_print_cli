# Design: live-travel-retraction-policy

## Controlling Code Paths

- Primary module path: `modules/core-modules/path-optimization-default/src/lib.rs`.
- Host queue and commit path: `crates/slicer-host/src/wit_host.rs`, `crates/slicer-sdk/src/postpass_builders.rs`, and `crates/slicer-host/src/dispatch.rs`.
- Neighboring tests or fixtures: existing `dispatch_tdd.rs` path-optimization queue tests plus new `travel_policy_tdd.rs` and `live_travel_policy_tdd.rs`.
- OrcaSlicer comparison surface: `GCode.cpp`, `RetractWhenCrossingPerimeters.hpp`, and `AvoidCrossingPerimeters.cpp`.

## Architecture Constraints

- Selected approach: retract/no-retract policy lives on `path-optimization-default` because the decision depends on per-layer path adjacency, not final text formatting.
- `DefaultGCodeEmitter` remains out-of-scope for policy; packet `11` serializes whatever commands this packet decides.
- The packet uses the existing and newly-added deferred queues: `deferred_retracts`, `deferred_z_hops`, and `deferred_travel_moves`; it must not invent any further travel-policy channels.
- OrcaSlicer canonical travel sequence (verified in `GCode.cpp:retract()` + `GCodeWriter::travel_to_xyz()`): **Retract → ZHop(up) → Travel Move → Unretract → ZHop(down handled by serializer)**. The module emits commands in this order.

## Code Change Surface

- Selected approach:
  - add focused module tests for external-travel retract and internal-travel suppression
  - add host integration tests for `z_hops` and deterministic travel decisions
  - wire the chosen policy through the existing path-optimization output queue
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs`
  - `crates/slicer-host/src/blackboard.rs` — `DeferredTravelMove` type + `LayerArena` queue
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/live_travel_policy_tdd.rs`
- `crates/slicer-host/src/wit_host.rs` — **no change needed** (deviation from original design.md list): `GcodeCommandCollected::Retract/Unretract` were already present in the WIT bindings; no world widening was required.
- `crates/slicer-host/src/gcode_emit.rs` — **modified but not originally listed** (deviation): the new `LayerCollectionIR.retracts` and `LayerCollectionIR.travel_moves` fields required a serializer consumer. `gcode_emit.rs` was updated to build per-entity lookup maps and emit the canonical Retract→ZHop(up)→Travel→ZHop(down)→Unretract sequence. This is serialization-only and does not move any retract/no-retract decision into the emitter; packet 11 still governs text formatting.
- Rejected alternatives that were considered and why they were not chosen:
  - moving travel policy into `gcode_emit.rs`: rejected because it would collapse decision-making into a text serializer surface
  - bundling generic nearest-neighbor ordering into this packet: rejected because packet `18` already owns that broader slice

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

## Open Questions

- None. The ownership decision is locked by this packet.

## Closed Deviations

- **ZHop anchor normalization**: The original design assumed the module would supply a valid global `after_entity_index` to `push_z_hop`. In practice the module only had access to a per-region wall-loop count, causing a mismatch with the global anchor used for Retract/Move commands. Fixed by normalizing ZHop's `after_entity_index` to `anchor` in the dispatch, identical to how Retract and Move are handled.
- **AC-3 split test strategy**: The spec says "when the live path-optimization stage runs" for the z-hop criterion. The host-level test (`live_travel_policy_tdd.rs`) tests the dispatch commit path by injecting pre-formed commands rather than invoking the WASM module with `travel_z_hop=0.2`. The module-level test `external_travel_with_z_hop_emits_z_hop_and_retract_pair` covers the config→ZHop emission path; the host test covers the dispatch→deferred-queue routing. Both are needed because native host tests cannot cross the WASM boundary without the full WASM toolchain.