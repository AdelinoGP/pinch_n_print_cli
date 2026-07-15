---
status: draft
packet: 159-visual-debug-intermediate-renderer
task_ids:
  - TASK-269
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Packet 158 is status active (grounded against implemented packet 157, commit 3e33ca01) but has no merged capture code yet; its renderer-owned capture export remains a forward contract that must be confirmed against packet 158's real implementation before this packet's implementation begins. Note also: slicer-runtime cannot import crates/pnp-cli's Manifest/ImageEntry types (pnp-cli depends on slicer-runtime, not the reverse), so this packet's renderer must be a pure function in slicer-runtime, with crates/pnp-cli/src/visual_debug.rs assembling the ImageEntry/Manifest.
---

# Packet Contract: 159-visual-debug-intermediate-renderer

## Goal

Render packet-158 typed, renderer-owned intermediate captures as deterministic PNGs with typed geometry, swept extrusion widths, stable overlays, one shared viewport, and the v1 fixed semantic palette without changing capture or command ownership.

## Scope Boundaries

This packet adds the intermediate visual-debug rasterization path for typed captures: geometry views, width sweeps, diagnostic overlays, shared viewport/legend metadata, and deterministic PNG encoding. It consumes packet 158's exported capture contract and packet 157's bundle/manifest seam. It does not own request parsing, bundle lifecycle, scheduler capture, final G-code rendering, agent documentation, or ordinary-slice instrumentation.

## Prerequisites and Blockers

- Depends on: packet `158-visual-debug-typed-tap-capture` (status `active`, no capture code merged yet); the `png` dependency decision and packet-158 renderer-owned capture export.
- Unblocks: packet `161` visual-debug agent surface and verification work; packet `160` remains an independent final G-code renderer path.
- Activation blockers: `[FWD]` packet 158 must publish the exact renderer-owned capture types, tap/layer ordering contract, and manifest image-entry extension seam; `[FWD]` the implementation must record the enabled `png` features and license review before activation.

## Acceptance Criteria

- **AC-1. Given** a packet-158 model-backed capture containing a documented polygon-bearing tap with `SliceIR.regions[].polygons` and a selected layer, **when** the intermediate renderer writes the bundle, **then** `manifest.json` contains exactly one image entry for that tap/layer/`filled_areas` view whose PNG exists, whose viewport is the model-wide viewport, whose legend version is the v1 legend version, and whose PNG dimensions are `1024 * resolution_scale` by `1024 * resolution_scale`. | `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd -- typed_polygon_render_records_contract_metadata --exact`
- **AC-2. Given** typed path captures containing `Point3WithWidth.width` values and a `filled_areas` request, **when** rendering runs, **then** each path is rasterized as its deterministic swept extrusion-width shape rather than a zero-width centerline or an inferred width, and the output differs from the corresponding `filament_lines` centerline view on a non-zero-width fixture. | `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd -- typed_width_sweep_is_rendered --exact`
- **AC-3. Given** a typed capture with documented stage-specific overlay fields such as seam coordinates, travel anchors, region/object identifiers, layer bounds, or execution annotations, **when** `diagnostic_overlay` is requested with a geometry view, **then** the PNG contains the stable labeled overlay and `manifest.json` records the same tap/layer/view association without changing the underlying geometry pixels outside the overlay operation. | `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd -- diagnostic_overlay_is_stable_and_composable --exact`
- **AC-4. Given** two selected typed taps or layers with different XY extents in one model-backed request, **when** both images are rendered at `resolution_scale: 2`, **then** every image entry records byte-identical viewport bounds, fixed margin, legend version, and raster dimensions `2048 x 2048`, and both renders use the fixed v1 semantic palette rather than request-supplied colors. | `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd -- shared_viewport_palette_and_scale_are_bundle_wide --exact`
- **AC-5. Given** the same packet-158 captures, request, and renderer version, **when** rendering is performed twice, **then** every PNG byte sequence and every image-entry field including `png_path`, viewport, legend version, dimensions, tap identity, layer index, and warnings is identical. | `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd -- intermediate_png_output_is_deterministic --exact`
- **AC-N1. Given** a typed capture with a missing or invalid geometry field required by its documented visualization, **when** rendering starts, **then** it fails with a typed renderer error naming the tap, layer, and missing field and does not report a successful bundle or leave a successful image entry for that capture. | `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd -- invalid_typed_geometry_fails_without_partial_success --exact`
- **AC-N2. Given** a request selecting `filled_areas` for a typed path whose capture has no usable `Point3WithWidth.width`, **when** rendering starts, **then** it rejects the view rather than inferring a bead width, and no successful PNG or manifest success result is produced. | `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd -- missing_typed_width_is_rejected --exact`
- **AC-N3. Given** a requested `resolution_scale` outside `1`, `2`, or `3`, **when** the renderer receives the already validated packet-157/158 request, **then** it returns a typed scale error and produces no PNG or successful image entry. | `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd -- unsupported_resolution_scale_fails_without_output --exact`

## Verification

- `cargo test -p pnp-cli --all-targets --test visual_debug_intermediate_renderer_tdd`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - direct read of the complete 235-line contract; renderer scope, visualization types, typed source fields, viewport, palette, resolution, and intermediate-path ownership.
- `docs/19_visual_debug.md` - direct read of the complete 58-line usage and bundle-reading contract.
- `docs/adr/0037-render-pngs-from-ir-stage-taps-not-gcode-only.md` - direct read of the complete 44-line typed post-stage rendering decision.
- `docs/01_system_architecture.md` - direct reads of lines 246-387, 460-635, and 638-665; typed IR ownership, stage outputs, postpass/lifetime boundaries, and memory model.
- `docs/11_operational_governance_and_acceptance_gate.md` - direct read of the complete 179-line determinism, recoverability, coupling, and acceptance-gate contract.
- `docs/07_implementation_status.md` - delegated task-location fact: TASK-269 is the intermediate renderer row at lines 240-241.
- `.ralph/specs/158-visual-debug-typed-tap-capture/packet.spec.md` - published prerequisite contract; it is draft, so its capture export remains `[FWD]` until implementation confirms it.

## Doc Impact Statement

- **`none`** - this packet implements the already documented intermediate renderer contract and changes no IR, WIT, scheduler, claim, manifest schema, host-service, or SDK contract. Any required manifest image-entry extension must be additive within packet 158's exported seam and documented by the implementation worker before closure.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
