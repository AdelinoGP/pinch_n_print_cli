# Requirements: 159-visual-debug-intermediate-renderer

## Packet Metadata

- Grouped task IDs: `TASK-269`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `implemented`
- Aggregate context cost: `M`
- Dependency: packet `158-visual-debug-typed-tap-capture` (status `implemented`, commit `68b10706`; renderer capture export is grounded against merged code — see `design.md` Open Questions)

## Problem Statement

Packet 158 supplies request-gated typed post-stage captures but intentionally stops before turning them into visual evidence. TASK-269 is the single renderer slice: consume those captures and produce comparable, deterministic intermediate PNGs without moving capture, scheduler, command, or final-artifact responsibilities into the renderer.

## In Scope

- Consume packet 158's exact renderer-owned typed capture export, preserving tap identity, layer index, source schema version, ordering, and warnings.
- Render `filament_lines` from typed path centerlines colored by semantic role.
- Render `filled_areas` from direct `ExPolygon` geometry where available.
- Render typed paths using swept extrusion-width shapes from `Point3WithWidth.width`; never infer width from unrelated values.
- Render stage-specific `diagnostic_overlay` content from documented fields, scoped to what packet 158 actually captures: seam coordinates and region/object identifiers on `Perimeter`/`Infill`/`Support` captures; travel anchors and execution annotations only on `LayerCollection` captures; layer bounds computed as a renderer-derived XY bounding box of the captured geometry (not a source field).
- Compose overlays with either geometry visualization without changing the selected geometry view's semantic meaning.
- Calculate one model-wide XY viewport with the documented fixed margin and reuse it for every selected image in the bundle.
- Implement the v1 fixed semantic palette and legend version for perimeter families, infill families, travel, support, support interface, and unclassified typed final extrusion where present.
- Enforce `resolution_scale` values `1`, `2`, and `3`, with dimensions `1024`, `2048`, and `3072` on each axis.
- Encode PNGs through a pure-Rust `png` path, recording enabled features and license review in implementation evidence.
- Produce deterministic pixel bytes, paths, ordering, manifest image metadata, and warnings for identical inputs.
- Add focused positive and negative renderer contract tests.

## Out of Scope

- CLI request parsing, request validation, source-mode selection, output-directory lifecycle, overwrite behavior, and base manifest ownership.
- Scheduler dependency closure, post-stage tap registration, host-hook timing, layer selection, capture retention, or any packet-158 capture implementation.
- Final serialized G-code parsing, G-code rendering, unsupported-command handling, or unclassified final G-code behavior owned by packet 160.
- WIT contracts, module manifests, IR schema changes, module-visible access, new modules, guest artifacts, or WASM build work.
- Agent skill or guide documentation, HTML galleries, pixel/perceptual bundle comparison, and ordinary-slice overhead instrumentation.
- OrcaSlicer parity or source translation.
- Request-supplied palette or legend overrides.

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - complete direct read; lines 112-163 and 180-213 control bundle metadata, visualizations, intermediate ownership, and typed source fields.
- `docs/19_visual_debug.md` - complete direct read; lines 18-50 control request-to-bundle expectations and inspection semantics.
- `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - complete direct read; typed post-stage evidence and synthetic-diagram boundary.
- `docs/01_system_architecture.md` - direct ranges 246-387, 460-635, and 638-665; IR shapes, ownership, and lifetime constraints.
- `docs/11_operational_governance_and_acceptance_gate.md` - complete direct read; determinism, recoverability, resource, and coupling gates.
- `docs/07_implementation_status.md` - delegated TASK-269 location at lines 240-241.
- `.ralph/specs/158-visual-debug-typed-tap-capture/packet.spec.md` - published prerequisite acceptance and renderer-owned capture handoff; draft status requires forward confirmation.
- `.ralph/specs/158-visual-debug-typed-tap-capture/requirements.md` - prerequisite scope and explicit exclusion of PNG/rasterization.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-5` prove typed polygon rendering, width sweeps, stable composable overlays, bundle-wide viewport/palette/scale, and byte determinism.
- Negative: `AC-N1` through `AC-N3` reject invalid typed geometry, missing width, and unsupported scale without successful partial output.
- Cross-packet impact: packet 157 remains the owner of request and bundle lifecycle; packet 158 remains the owner of capture and dependency closure; packet 159 consumes their exports and owns only rasterization and image-entry rendering metadata; packet 160 owns final G-code rendering.
- Forward contracts (all resolved 2026-07-15 against commit `68b10706`; see `design.md` Open Questions): `[FWD-158-1]` resolved — `StageCapture`/`CapturedIr` (`crates/slicer-runtime/src/layer_executor.rs:591-679`) exposes stable typed capture values with tap identity, layer index, schema version, and deterministic ordering; `warnings` is not carried (stays hardcoded empty). `[FWD-158-2]` resolved — `ImageEntry.png_path`/`typed_capture` (`crates/pnp-cli/src/visual_debug.rs:269-288`, populated at 437-454) is already the additive handoff; this packet fills `png_path` only. `[FWD-158-3]` resolved — no synthetic non-geometry capture kind exists; all four `CapturedIr` variants are geometry-bearing, so `diagnostic_overlay` is always composited over a geometry view, never a standalone diagram.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd` | Run all typed geometry, width sweep, overlay, viewport/palette, determinism, and negative renderer tests. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | Compile runtime, renderer, CLI integration, and all test targets. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Enforce the workspace quality gate. | FACT pass/fail |

## Step Completion Expectations

- Renderer inputs are immutable packet-158-owned values; no renderer borrow survives the render call and no renderer reaches into `LayerArena`.
- The same viewport, fixed margin, legend version, palette, raster dimensions, ordering, and PNG encoding rules apply across all images in a bundle.
- A renderer failure is fatal to visual evidence and cannot leave a successful partial bundle result.
- Renderer behavior is opt-in through the existing validated visual-debug path; ordinary `pnp_cli slice` remains untouched.
- Typed source fields remain explicit: direct polygons stay direct, `Point3WithWidth.width` drives sweeps, and overlays use documented fields only.

## Context Discipline Notes

- `docs/01_system_architecture.md` is large; read only the ranges listed in this packet and delegate any symbol lookup.
- Packet 158 is now `implemented` (commit `68b10706`); `[FWD-158-1]` through `[FWD-158-3]` are resolved against its actual implementation (see `design.md` Open Questions) — no further grounding dispatch is needed before Step 2.
- Grounded fact: packet 157's `Manifest`/`ImageEntry` types live in `crates/pnp-cli/src/visual_debug.rs`, and `slicer-runtime` cannot import them (dependency direction is `pnp-cli -> slicer-runtime`). This packet's renderer logic (rasterization, viewport, palette, PNG encoding) is expected to live in `slicer-runtime` as a pure function of typed capture data, while `crates/pnp-cli/src/visual_debug.rs` calls it and assembles the resulting `ImageEntry` values — mirroring packet 158's own pnp-cli/slicer-runtime split.
- Do not read packet 160 or implementation code broadly to infer the renderer seam; use bounded dispatches and return only the requested symbols or facts.
