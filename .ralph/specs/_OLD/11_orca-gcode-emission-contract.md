---
status: implemented
packet: orca-gcode-emission-contract
task_ids:
  - TASK-119
  - TASK-119a
  - TASK-119b
  - TASK-119c
---

# 11_orca-gcode-emission-contract

## Goal

Define and implement one canonical OrcaSlicer-compatible GCode emission contract on the live host postpass path, including layer-change comments, role-to-` ;TYPE:` labeling, and the emitted serialization rules for fill, support, seam-started wall loops, retract/unretract, and travel moves when those entities or decisions are present on the final path.

## Problem Statement

The live host emit path currently converts `LayerCollectionIR` into mostly raw `G1` moves plus pass-through comments. That is not enough for OrcaSlicer-compatible preview and visualization semantics. The missing gap is larger than comment headers alone: the host must own one exact emit contract for layer-change comments, `;TYPE:` role boundaries, seam-started wall-loop preservation, and travel/retraction serialization when fill, support, seam, or travel decisions reach the final postpass path.

This packet owns the emitted-text contract only. It does not restore the upstream feature producers. Those producer packets remain separate so the repo can validate emit behavior against synthetic fixtures now, then layer the real fill/support/seam/travel generation work on top of the same contract later.

## Architecture Constraints

- The emitter must derive Orca-facing text from `LayerCollectionIR`, `GCodeIR`, and postpass commands. Modules must not own Orca-specific string formatting.
- The contract must be deterministic. The same layer/entity sequence must always emit the same headers, labels, and travel ordering.
- `;HEIGHT:` cannot come from guessed preset state hidden outside the layer stream. Selected approach: derive the current layer height from consecutive `LayerCollectionIR.z` deltas, falling back to the last non-zero delta for the terminal layer.
- Seam emission in this packet is preserve-only. If a wall loop already starts at the resolved seam point, the emitter must not disturb that ordering.

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
