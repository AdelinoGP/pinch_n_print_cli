---
status: implemented
packet: wit-boundary-gaps-postpass
task_ids:
  - TASK-129
  - TASK-129a
  - TASK-129b
  - TASK-129c
---

# 07_wit-boundary-gaps-postpass

## Goal

Close the remaining non-segmentation WIT-boundary gaps on live execution paths. Covers DEV-006. Specifically: widen the postpass WIT surface so full `GCodeCommand` payloads and explicit `Unretract` writes cross the boundary, widen the finalization WIT surface so completed layers expose `ordered_entities` and `z_hops`, and add live-path regression coverage for the existing layer-world builder-to-commit path.

## Problem Statement

DEV-006 identifies that postpass GCode command content and executable WIT-boundary coverage still have live gaps. The three sub-tasks target distinct boundary surfaces:

- TASK-129a: `dispatch_postpass_gcode_call` in `crates/slicer-host/src/dispatch.rs` currently passes an empty `&[]` slice into the WIT call instead of the real command list, and the postpass WIT surface does not currently expose payload-bearing command input or explicit `Unretract` write support. This task widens `world-postpass.wit`, the shared `gcode-output-builder`, and the host/macro/SDK mirrors so all eight `GCodeCommand` variants can cross the live boundary with exact payloads.

- TASK-129b: Layer-world modules do not have a `LayerCollectionIR` read resource; they write through builder surfaces and the host commits those outputs into `LayerCollectionIR`. This task adds live-path regression coverage for that existing builder-to-arena-to-commit path so entity fields, tool changes, and z-hops are proven to survive the production layer-world boundary.

- TASK-129c: Finalization-world modules consume `Vec<LayerCollectionIR>` through `world-finalization.wit`, but the current `layer-collection-view` only exposes metadata (`layer-index`, `z`, `entity-count`, `tool-changes`). This task widens the finalization WIT surface and its mirrors so `ordered_entities` and `z_hops` cross the live boundary with exact field preservation.

## Architecture Constraints

- WIT boundary rules from `docs/03_wit_and_manifest.md`: modules never receive more data than declared; access control enforced per-call
- GCodeIR is owned by the host and passed into postpass modules as a mutable reference; the `gcode-output-builder` is the only write surface modules may use
- The postpass dispatch currently passes `&[]` (empty slice) for the command list instead of the actual `gcode_ir.commands`, and the postpass WIT surface omits both full command payload input and explicit `Unretract` output support
- Layer-world modules write via builders (`perimeter-output-builder`, `infill-output-builder`, etc.); there is no `LayerCollectionIR` read resource in `world-layer.wit`, so TASK-129b must validate the existing builder-to-arena-to-commit path rather than inventing a new read surface
- Finalization-world modules read `Vec<LayerCollectionIR>`, but the current `layer-collection-view` only exposes metadata (`layer-index`, `z`, `entity-count`, `tool-changes`); `ordered_entities` and `z_hops` require WIT widening before live-path deep-copy can be fully verified
- DEV-014 remains in force: canonical WIT files, host inline WIT, macro inline WIT, and hand-written guest WIT must be updated together and guarded by drift tests in the same implementation slice

## Data and Contract Notes

- IR or manifest contracts touched:
  - `GCodeIR.commands: Vec<GCodeCommand>` — all 8 variants must cross the widened postpass boundary with exact payloads
  - `gcode-output-builder` — must expose `push-unretract` so guest output can express the full IR command set
  - `LayerCollectionIR` — layer-world coverage targets the committed host IR after builder output drains through the production commit path
  - `Vec<LayerCollectionIR>` — finalization WIT widening must expose `ordered_entities` and `z_hops` in addition to metadata-level fields
- WIT boundary considerations:
  - Postpass boundary: `gcode-output-builder` resource is pushed into the store before the call; the builder collects pushes from the guest and commits them back to the host GCodeIR after the call. The widened command input must be mirrored in canonical WIT, host bindgen, macro glue, and any hand-written guest WIT copies.
  - Layer-world boundary: validate the existing builder outputs and host commit path rather than inventing a new read surface.
  - Finalization-world boundary: `world-finalization.wit` exposes `layer-collection-view`; widening that resource must preserve all completed-layer data and remain mirrored across canonical and inline WIT definitions.
- Determinism or scheduler constraints: The boundary operations must be deterministic; order/content preservation is required

## Locked Assumptions and Invariants

- The `gcode-output-builder` is the only channel through which a postpass module may emit GCode commands; direct mutation of `GCodeIR.commands` by the guest is not permitted
- Empty command lists are valid — no contract violation should be raised for an empty list
- The postpass implementation slice must update canonical WIT, host inline WIT, macro inline WIT, and hand-written guest WIT together; partial updates are not acceptable
- TASK-129b must not add a new layer-world read resource; the live commit path is the authoritative surface under test
- All entity fields in committed `LayerCollectionIR` are preserved bit-for-bit through the layer-world boundary — no normalization, rounding, or truncation
- All completed-layer fields exposed through widened finalization WIT are preserved bit-for-bit through the finalization-world boundary

## Risks and Tradeoffs

- Risk: Widening duplicated WIT surfaces can reintroduce DEV-014 drift if canonical, host, macro, and guest copies do not change together
- Mitigation: Extend `wit_drift_detection_tdd` in the same slice and run it before boundary-runtime tests
- Risk: Changing `dispatch_postpass_gcode_call` to pass a non-empty payload-bearing command list may expose latent guest bugs or bindgen mismatches
- Mitigation: Start with focused postpass boundary and SDK tests before running broader host tests
- Risk: Widened finalization inputs increase bindgen and macro deep-copy complexity
- Mitigation: Reuse the existing witness-based finalization pattern with minimal 2-3 layer fixtures that isolate field-preservation assertions
