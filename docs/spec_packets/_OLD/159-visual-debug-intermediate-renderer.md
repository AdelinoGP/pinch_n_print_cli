---
status: implemented
packet: 159-visual-debug-intermediate-renderer
task_ids:
  - TASK-269
---

# 159-visual-debug-intermediate-renderer

## Goal

Render packet-158 typed, renderer-owned intermediate captures as deterministic PNGs with typed geometry, swept extrusion widths, stable overlays, one shared viewport, and the v1 fixed semantic palette without changing capture or command ownership.

## Problem Statement

Packet 158 supplies request-gated typed post-stage captures but intentionally stops before turning them into visual evidence. TASK-269 is the single renderer slice: consume those captures and produce comparable, deterministic intermediate PNGs without moving capture, scheduler, command, or final-artifact responsibilities into the renderer.

## Architecture Constraints

- The renderer consumes typed, post-stage, post-host-hook, renderer-owned values only. It does not create scheduler edges, invoke modules, read Blackboard state, or retain `LayerArena` data.
- The renderer must preserve the documented source distinction: direct `ExPolygon` areas render directly; typed paths use `Point3WithWidth.width` for filled sweeps; diagnostic diagrams use documented trace fields rather than fabricated model geometry.
- Every image in a bundle uses one model-wide XY viewport, documented fixed margin, fixed v1 semantic palette, fixed legend version, and the requested raster scale. Palette values are implementation constants, not request options.
- PNG output is deterministic in pixel traversal, primitive ordering, alpha handling, compression configuration, path naming, and manifest image-entry ordering. A failure cannot be reported as a successful partial bundle.
- `png` is pure Rust; the implementation must record enabled features and license review before closure.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Data and Contract Notes

- IR/manifest contracts: the renderer consumes packet-158 typed captures and records image path, source mode, tap, layer/Z where applicable, visualization, viewport, legend version, source schema version, and warnings in the existing manifest image-entry seam.
- WIT boundary: unchanged. No capture or render data crosses into a module and no module receives a new access capability.
- Determinism/scheduler constraints: scheduler timing and capture ordering are inherited from packet 158; renderer ordering, viewport, palette, raster scale, PNG bytes, and manifest fields must be stable for identical inputs.
- `[FWD-158-1]` Capture records must preserve all documented source fields needed by this packet, including `Point3WithWidth.width` and overlay fields, in renderer-owned values.
- `[FWD-158-2]` The packet-158 handoff must permit image entries to be appended without duplicating request parsing, bundle lifecycle, or base manifest semantics.
- `[FWD-158-3]` Non-geometry typed captures must expose trace-relevant fields sufficient for a stage-specific synthetic diagram or be rejected as unsupported rather than fabricated.

## Locked Assumptions and Invariants

- Packet 158 is `implemented` (commit `68b10706`) and its capture code is merged; the three FWD handoff statements above are grounded against real types, not the spec text (see Open Questions).
- The v1 palette, legend version, fixed margin, and `1024 x 1024` base raster are implementation-owned constants derived from the documented contract, not user-configurable request fields.
- A successful render has no dangling source borrows and no successful partial image set.
- Ordinary slice execution has no renderer allocation or invocation because this packet only consumes the existing opt-in visual-debug path.

## Risks and Tradeoffs

- Packet 158 may expose a capture shape that cannot carry one documented source field; the correct response is a draft blocker or scope change, not field guessing.
- Rasterization of integer geometry and width sweeps can introduce edge ambiguity; deterministic tie-breaking and pixel-center rules must be tested on fixture boundaries.
- Overlay text can vary by font backend; use a deterministic in-process glyph/label representation or a documented fixed raster strategy, never an OS font dependency.
- PNG compression settings affect byte determinism; pin encoder settings and test repeated byte output.
