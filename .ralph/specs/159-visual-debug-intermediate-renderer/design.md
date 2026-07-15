# Design: 159-visual-debug-intermediate-renderer

## Controlling Code Paths

- Primary code path: packet-158 renderer-owned typed capture handoff into the runtime/CLI visual-debug intermediate renderer, viewport planner, fixed palette, overlay compositor, and PNG encoder.
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/visual_debug_intermediate_renderer_tdd.rs` and the smallest deterministic typed-capture fixtures exported by packet 158.
- Final G-code comparison: explicitly out of scope; packet 160 owns the independent serialized final-G-code renderer.

## Architecture Constraints

- The renderer consumes typed, post-stage, post-host-hook, renderer-owned values only. It does not create scheduler edges, invoke modules, read Blackboard state, or retain `LayerArena` data.
- The renderer must preserve the documented source distinction: direct `ExPolygon` areas render directly; typed paths use `Point3WithWidth.width` for filled sweeps; diagnostic diagrams use documented trace fields rather than fabricated model geometry.
- Every image in a bundle uses one model-wide XY viewport, documented fixed margin, fixed v1 semantic palette, fixed legend version, and the requested raster scale. Palette values are implementation constants, not request options.
- PNG output is deterministic in pixel traversal, primitive ordering, alpha handling, compression configuration, path naming, and manifest image-entry ordering. A failure cannot be reported as a successful partial bundle.
- `png` is pure Rust; the implementation must record enabled features and license review before closure.
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add a renderer-owned intermediate pipeline that normalizes packet-158 capture records into render primitives, computes the bundle viewport once, applies fixed semantic colors and legend metadata, composites optional overlays, and writes deterministic PNGs through the pure-Rust encoder.
- Exact functions, traits, manifests, tests, and fixtures: packet 158's exported typed capture and image-entry handoff; the runtime visual-debug renderer module; the owning runtime manifest/bundle integration seam; `crates/slicer-runtime/tests/visual_debug_intermediate_renderer_tdd.rs`; and a focused deterministic typed-capture fixture. Exact symbol names are `[FWD]` until packet 158's export dispatch confirms them.
- Rejected alternatives and reasons: rendering from serialized G-code is packet 160 and cannot localize intermediate defects; a debug WASM module violates ADR-0037 ownership; per-image viewports prevent stage comparison; inferred bead width loses typed geometry semantics; external image tools violate release-time portability and determinism.

## Files in Scope (read + edit)

- `crates/slicer-runtime/src/` - role: intermediate renderer, viewport/palette/overlay composition, PNG encoding, and packet-158 handoff; expected change: add the renderer without changing scheduler capture.
- `crates/slicer-runtime/tests/visual_debug_intermediate_renderer_tdd.rs` - role: focused renderer contract coverage; expected change: add positive, determinism, and failure-path assertions.
- Packet-158-owned visual-debug integration source identified by `[FWD-158-2]` - role: additive image-entry handoff; expected change: only the fields required to attach renderer-produced PNG metadata and paths, if packet 158 does not already export them.

The two concrete runtime/test surfaces and the packet-158 seam are the only edit categories; implementation must not broaden them without a new packet.

## Read-Only Context

- `docs/specs/visual-pipeline-debug.md` - lines 112-163 and 180-213 only - bundle image contract, visualization semantics, intermediate ownership, and exact tap source fields.
- `docs/19_visual_debug.md` - lines 18-50 only - request scale, shared viewport, filled-area semantics, warnings, and failure behavior.
- `docs/01_system_architecture.md` - lines 246-387, 460-635, and 638-665 only - typed IR outputs, postpass ownership, data ownership, and arena lifetime.
- `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - complete 44-line decision record - typed tap rationale and synthetic-diagram boundary.
- `docs/11_operational_governance_and_acceptance_gate.md` - complete 179-line governance contract - determinism, recoverability, resource, and coupling obligations.
- `.ralph/specs/158-visual-debug-typed-tap-capture/**` - bounded export/contract lookup only - exact renderer-owned capture and manifest handoff names.

## Out-of-Bounds Files

- `crates/pnp-cli/` command parsing, request validation, source-mode selection, output lifecycle, overwrite behavior, and base manifest semantics - packet 157 owns them; only an existing renderer dispatch seam may be consumed.
- Scheduler/executor capture paths and layer-retention logic - packet 158 owns them.
- Final G-code parser/renderer surfaces - packet 160 owns them.
- `crates/slicer-schema/wit/`, module manifests, IR schema definitions, `modules/`, guest artifacts, and WASM build inputs - no contract or guest changes are permitted.
- `.claude/skills/`, `docs/17_agent_debugging.md`, HTML report code, and agent-facing documentation - later packet or existing tooling owns them.
- `target/`, `Cargo.lock`, generated code, and vendored dependencies - never load or edit.
- `OrcaSlicerDocumented/` - no parity scope applies; do not load.

## Expected Sub-Agent Dispatches

- Question: What exact public packet-158 capture records, tap/layer ordering, and renderer handoff symbols are available? Scope: `.ralph/specs/158-visual-debug-typed-tap-capture/**`; return: `LOCATIONS` at most 20 entries; purpose: resolve `[FWD-158-1]` and `[FWD-158-2]` without inventing APIs.
- Question: Which existing runtime module owns visual-debug bundle image entries and the narrowest renderer invocation seam? Scope: `crates/slicer-runtime/src/**`; return: `LOCATIONS` at most 20 entries; purpose: avoid moving packet-157/158 ownership.
- Question: Which exact typed fields and fixture constructors are available for polygon, `Point3WithWidth`, overlay, and synthetic diagram inputs? Scope: packet-158 export plus targeted `crates/slicer-ir/src/**` and `crates/slicer-runtime/tests/**`; return: `SNIPPETS` at most 3 snippets, 30 lines each; purpose: build compile-pinned renderer tests against real types.
- Question: Which `png` crate version/features and license evidence are acceptable in the workspace? Scope: manifests and dependency policy only; return: `FACT` in 5 lines or fewer; purpose: close the pure-Rust encoder decision without browsing lockfiles.

## Data and Contract Notes

- IR/manifest contracts: the renderer consumes packet-158 typed captures and records image path, source mode, tap, layer/Z where applicable, visualization, viewport, legend version, source schema version, and warnings in the existing manifest image-entry seam.
- WIT boundary: unchanged. No capture or render data crosses into a module and no module receives a new access capability.
- Determinism/scheduler constraints: scheduler timing and capture ordering are inherited from packet 158; renderer ordering, viewport, palette, raster scale, PNG bytes, and manifest fields must be stable for identical inputs.
- `[FWD-158-1]` Capture records must preserve all documented source fields needed by this packet, including `Point3WithWidth.width` and overlay fields, in renderer-owned values.
- `[FWD-158-2]` The packet-158 handoff must permit image entries to be appended without duplicating request parsing, bundle lifecycle, or base manifest semantics.
- `[FWD-158-3]` Non-geometry typed captures must expose trace-relevant fields sufficient for a stage-specific synthetic diagram or be rejected as unsupported rather than fabricated.

## Locked Assumptions and Invariants

- Packet 158 remains draft and is not treated as implemented; all three handoff statements above are forward contracts to verify before activation.
- The v1 palette, legend version, fixed margin, and `1024 x 1024` base raster are implementation-owned constants derived from the documented contract, not user-configurable request fields.
- A successful render has no dangling source borrows and no successful partial image set.
- Ordinary slice execution has no renderer allocation or invocation because this packet only consumes the existing opt-in visual-debug path.

## Risks and Tradeoffs

- Packet 158 may expose a capture shape that cannot carry one documented source field; the correct response is a draft blocker or scope change, not field guessing.
- Rasterization of integer geometry and width sweeps can introduce edge ambiguity; deterministic tie-breaking and pixel-center rules must be tested on fixture boundaries.
- Overlay text can vary by font backend; use a deterministic in-process glyph/label representation or a documented fixed raster strategy, never an OS font dependency.
- PNG compression settings affect byte determinism; pin encoder settings and test repeated byte output.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: packet-158 export/handoff lookup; `LOCATIONS` at most 20 entries.

## Open Questions

- [FWD] What exact packet-158 type names and file own the renderer-owned capture payload, selected tap/layer order, and image-entry handoff? Resolve with the bounded packet-158 export dispatch before activation.
- [FWD] Does packet 158 already provide a synthetic-diagram payload for non-geometry taps, or must this packet define a renderer-local projection from its documented fields? Resolve with the bounded capture-field dispatch; reject unsupported absence rather than fabricate geometry.
- [FWD] Which existing runtime crate/module should own the pure-Rust `png` dependency and its feature/license record? Resolve with the bounded dependency dispatch before editing manifests.
