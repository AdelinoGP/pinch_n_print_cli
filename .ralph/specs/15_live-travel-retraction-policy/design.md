# Design: live-travel-retraction-policy

## Controlling Code Paths

- Primary module path: `modules/core-modules/path-optimization-default/src/lib.rs`.
- Host queue and commit path: `crates/slicer-host/src/wit_host.rs`, `crates/slicer-sdk/src/postpass_builders.rs`, and `crates/slicer-host/src/dispatch.rs`.
- Neighboring tests or fixtures: existing `dispatch_tdd.rs` path-optimization queue tests plus new `travel_policy_tdd.rs` and `live_travel_policy_tdd.rs`.
- OrcaSlicer comparison surface: `GCode.cpp`, `RetractWhenCrossingPerimeters.hpp`, and `AvoidCrossingPerimeters.cpp`.

## Architecture Constraints

- Selected approach: retract/no-retract policy lives on `path-optimization-default` because the decision depends on per-layer path adjacency, not final text formatting.
- `DefaultGCodeEmitter` remains out-of-scope for policy; packet `11` serializes whatever commands this packet decides.
- The packet can use the existing deferred retract/unretract and Z-hop queues; it must not invent a second travel-policy channel.

## Code Change Surface

- Selected approach:
  - add focused module tests for external-travel retract and internal-travel suppression
  - add host integration tests for `z_hops` and deterministic travel decisions
  - wire the chosen policy through the existing path-optimization output queue
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - `modules/core-modules/path-optimization-default/tests/travel_policy_tdd.rs`
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/tests/live_travel_policy_tdd.rs`
- Rejected alternatives that were considered and why they were not chosen:
  - moving travel policy into `gcode_emit.rs`: rejected because it would collapse decision-making into a text serializer surface
  - bundling generic nearest-neighbor ordering into this packet: rejected because packet `18` already owns that broader slice

## Data and Contract Notes

- IR or manifest contracts touched:
  - `GCodeCommand::{Move, Retract, Unretract}`
  - `LayerCollectionIR.z_hops`
  - existing path-optimization config keys for travel behavior, plus any explicit retract-related config this packet introduces
- WIT boundary considerations:
  - no world widening is expected; the packet uses existing postpass builder calls and deferred queue surfaces
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