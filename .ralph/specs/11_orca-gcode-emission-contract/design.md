# Design: orca-gcode-emission-contract

## Controlling Code Paths

- Primary code path: `crates/slicer-host/src/gcode_emit.rs` — `DefaultGCodeEmitter::emit_gcode()` and `DefaultGCodeSerializer::serialize_gcode()` are the narrowest host-owned surfaces that already decide emitted line order.
- Secondary code path: `crates/slicer-host/src/postpass.rs` — `execute_postpass()` is the whole-postpass integration surface the packet must guard.
- Supporting boundary surface: `crates/slicer-host/src/wit_host.rs` and `crates/slicer-sdk/src/postpass_builders.rs` — deferred comments, retract/unretract, tool changes, and Z hops all cross here before final emission.
- Neighboring tests or fixtures: `crates/slicer-host/tests/postpass_gcode_boundary_tdd.rs`, `crates/slicer-host/tests/benchy_end_to_end_tdd.rs`, and a new focused `crates/slicer-host/tests/gcode_emit_tdd.rs`.
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/GCode.cpp`, `GCodeWriter.cpp`, `GCodeProcessor.hpp`.

## Architecture Constraints

- The emitter must derive Orca-facing text from `LayerCollectionIR`, `GCodeIR`, and postpass commands. Modules must not own Orca-specific string formatting.
- The contract must be deterministic. The same layer/entity sequence must always emit the same headers, labels, and travel ordering.
- `;HEIGHT:` cannot come from guessed preset state hidden outside the layer stream. Selected approach: derive the current layer height from consecutive `LayerCollectionIR.z` deltas, falling back to the last non-zero delta for the terminal layer.
- Seam emission in this packet is preserve-only. If a wall loop already starts at the resolved seam point, the emitter must not disturb that ordering.

## Code Change Surface

- Selected approach:
  - add one canonical label/header helper in `gcode_emit.rs`
  - extend `DefaultGCodeEmitter` to insert layer headers and role-boundary markers before serializing moves
  - keep retract/unretract, tool-change, and Z-hop serialization on the existing `GCodeCommand` path rather than inventing a second emit surface
  - add focused text assertions in a new `gcode_emit_tdd.rs` and a whole-postpass regression in `postpass_gcode_emit_contract_tdd.rs`
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `crates/slicer-host/src/gcode_emit.rs`
  - `crates/slicer-host/src/postpass.rs`
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/tests/gcode_emit_tdd.rs`
  - `crates/slicer-host/tests/postpass_gcode_emit_contract_tdd.rs`

  Note: `crates/slicer-host/tests/postpass_gcode_boundary_tdd.rs` is a neighboring WASM-module boundary test and is not owned by this packet. `crates/slicer-sdk/src/postpass_builders.rs` does not exist; retract/travel/Z-hop cross the boundary via `GCodeCommand` variants on the existing `execute_postpass()` path.
- Rejected alternatives that were considered and why they were not chosen:
  - injecting Orca comment strings directly from feature modules: rejected because it would duplicate spelling/order rules across producers
  - widening `LayerCollectionIR` with Orca-specific text fragments: rejected because the text contract belongs to postpass, not IR schema
  - letting later Benchy tests define the contract implicitly: rejected because producer packets need one stable emitted-text target before they restore geometry

## Data and Contract Notes

- IR or manifest contracts touched:
  - `ExtrusionRole` to Orca label mapping
  - `LayerCollectionIR.z`, `ordered_entities`, `tool_changes`, `z_hops`, and `annotations`
  - `GCodeIR.commands` and `GCodeCommand::{Move, Retract, Unretract, ToolChange, Comment, Raw}`
- WIT boundary considerations:
  - no WIT schema change is required; the packet consumes the already-declared postpass command types
- Determinism or scheduler constraints:
  - role-boundary labels must only change when the contiguous role block changes
  - header emission must not depend on hash-map iteration or module discovery order

## Locked Assumptions and Invariants

- The host remains the only owner of final GCode text formatting.
- Travel/retract policy may evolve in later packets, but once a `GCodeCommand` sequence reaches this packet's surface, serialization order is owned here.
- Seam placement decisions remain upstream; this packet only preserves seam-started output.

## Risks and Tradeoffs

- Risk: layer-height derivation for the terminal layer can drift if the emitter guesses from zero context. Mitigation: codify a deterministic fallback and test it directly.
- Risk: role labels can fragment too often if inserted per entity. Mitigation: group only on contiguous role transitions.
- Risk: producer packets may try to bypass this contract with ad hoc comments. Mitigation: keep one canonical helper and require their tests to assert against this emitted path.

## Open Questions

- None. The packet has a single selected approach and a closed contract surface.