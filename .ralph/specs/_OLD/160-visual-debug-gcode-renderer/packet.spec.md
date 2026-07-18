---
status: implemented
packet: 160-visual-debug-gcode-renderer
task_ids:
  - TASK-270
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Packet 157 is implemented (commit 3e33ca01) and packet 158 is implemented; both live in crates/pnp-cli/src/visual_debug.rs. That file already has a placeholder `VisualDebugSource::Gcode` arm (visual_debug.rs:519-561) that builds `ImageEntry` values without opening the G-code file, parsing it, iterating requested layers, or writing PNG bytes — no PNG-encoding dependency exists in crates/pnp-cli/Cargo.toml today. This packet replaces that placeholder with a real parser/renderer; `main.rs` and `lib.rs` need no changes.
---

# Packet Contract: 160-visual-debug-gcode-renderer

## Goal

Implement the standalone final-G-code visual-debug path that parses the documented Pinch 'n Print G0/G1 subset and writes deterministic final-view PNG artifacts plus manifest warnings.

## Scope Boundaries

This packet owns parsing serialized final G-code after `PostPass::TextPostProcess`, tracking the documented layer, Z, type, motion, and extrusion-mode markers, projecting supported moves into the existing visual-debug bundle contract, and emitting final PNG output. It preserves extrusion whose role is not classified as `unclassified` and reports unsupported constructs with source line numbers. It does not add typed taps, scheduler dependency closure, an intermediate renderer, the agent skill, or ordinary slice instrumentation.

## Prerequisites and Blockers

- Depends on: packet 157 (`status: implemented`, commit `3e33ca01`, TASK-267) and packet 158 (`status: implemented`, TASK-268); both live entirely in `crates/pnp-cli/src/visual_debug.rs`.
- Unblocks: packet 161 visual-debug agent verification.
- Activation blockers: none remaining. The integration seam is grounded below; the only open item is a bounded implementation-time PNG-crate selection (Step 2), not a packet-level blocker.

### Grounded Packet 157/158 Integration Facts

- **Request shape (verified, `crates/pnp-cli/src/visual_debug.rs:16-48`):** `VisualDebugRequest` already has `source: VisualDebugSource`, an untagged-tag enum with `kind = "model"` and `kind = "gcode"` variants (`Gcode { path, .. }`); `layers: Vec<LayerSelector>`, `taps: Vec<TapSelector>` (`TapSelector` is untagged — `"final_gcode"` needs no new enum variant, just a free-form tap name string), `visualizations: Vec<VisualizationSpec>`, `resolution_scale: u32`, and `gcode_line_width_mm: Option<f64>` (already present and already validated as required for a Gcode-source `filled_areas` request at `visual_debug.rs:197-215`). This packet consumes these fields as-is and must not add new request fields or re-validate what `validate_request` already enforces.
- **Manifest/image-entry shape (verified, `visual_debug.rs:220-288`):** `Manifest` and `ImageEntry` already carry `gcode_parser_version: Option<String>` alongside `ir_schema_version`, plus `source`, `tap`, `layer_index`, `layer_z`, `visualization`, `png_path`, `viewport`, `legend_version`, `warnings`, and `typed_capture`. No manifest schema change is needed; this packet only populates existing fields with real values instead of the current placeholder's synthesized ones.
- **Integration seam (verified, `visual_debug.rs:480-621`, `run_visual_debug`):** the `VisualDebugSource::Gcode { path, .. }` match arm at `visual_debug.rs:519-561` is a **placeholder**, not real behavior — it never opens the file at `path`, never parses G-code, only uses `req.layers.first()` for every synthesized `ImageEntry` (so it cannot yet satisfy "one PNG per selected rendered layer"), and never writes PNG bytes. This packet replaces that arm's body. `main.rs` and `lib.rs` require no changes: `main.rs`'s `Cmd::VisualDebug` clap surface (`main.rs:85-93`) is generic (`--request`/`--output`/`--overwrite`) and dispatches to `visual_debug::run_cli` (`main.rs:437-447`) with no source-kind-specific logic; `lib.rs` already declares `pub mod visual_debug;` and needs no new top-level module entry (the new parser/renderer module can be declared privately inside `visual_debug.rs`).
- **No PNG-writing facility exists yet (verified):** `crates/pnp-cli/Cargo.toml` has no `png`/`image` dependency, and the only artifact ever atomically committed today is `manifest.json` (temp-file-then-rename at `visual_debug.rs:604-620`). This packet is the first to add a real PNG-encoding dependency and file-write step; the existing manifest atomic-commit ordering (PNGs written before `manifest.json` is renamed into place) is the invariant this packet must preserve so a failed run never leaves a successful-looking `manifest.json` behind.
- **`Viewport` is pixel dimensions only (verified, `visual_debug.rs:264-267, 499-503`):** `Viewport { width, height }` is derived solely from `resolution_scale` and is already computed identically before the source dispatch match for both `Model` and `Gcode` arms — it is not a geometric mm-space bounding box and this packet does not change its shape. "One model-wide XY viewport for all emitted PNGs" (AC-4) means this packet must internally compute a model-wide XY bounding box (mm-space, plus documented fixed margin) from all parsed supported moves across the rendered layers, and use that one bounding box to project geometry consistently into the already-shared pixel canvas — not a manifest or `Viewport` struct change.

## Acceptance Criteria

- **AC-1. Given** a valid standalone request selecting `taps: ["final_gcode"]`, `visualizations: ["filament_lines"]`, `resolution_scale: 1`, and a final G-code file containing supported `G0`/`G1` X/Y/Z/E/F moves with `;LAYER_CHANGE`, `;Z:`, and `;TYPE:` markers, **when** `pnp_cli visual-debug --request request.json --output bundle-dir` runs, **then** it succeeds with `manifest.json` and one PNG for each selected rendered layer, and each image entry records `source.kind: "gcode"`, `tap: "final_gcode"`, the layer index and Z, PNG path, shared viewport, legend version, G-code parser version, and warnings. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- ac1_supported_final_gcode_produces_manifest_and_pngs --exact`
- **AC-2. Given** supported extrusion moves with recognized `;TYPE:` role boundaries and moves with no active recognized role, **when** the final renderer parses them, **then** recognized moves use their semantic role and every role-less extrusion move is retained in the output geometry with role `unclassified` and the manifest contains an unclassified warning rather than dropping or guessing the move. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- ac2_preserves_unclassified_extrusion --exact`
- **AC-3. Given** a supported standalone request selecting `visualizations: ["filled_areas"]` and supplying `gcode_line_width_mm`, **when** rendering runs, **then** each extrusion segment is rasterized using that requested physical width and the renderer does not derive bead width from E values. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- ac3_filled_areas_use_requested_line_width --exact`
- **AC-4. Given** a final G-code file containing supported absolute or relative extrusion-mode markers and travel moves, **when** it is parsed, **then** XY/Z state, E state, travel segments, extrusion segments, layer boundaries, and role boundaries are applied deterministically in source order and the output viewport is one model-wide XY viewport for all emitted PNGs. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- ac4_tracks_motion_state_layers_roles_and_shared_viewport --exact`
- **AC-5. Given** an input containing a raw macro or command outside the documented PnP `G0`/`G1` subset, **when** rendering runs, **then** the command remains non-approximated and `manifest.json` records a warning containing the unsupported source line number, while supported moves still render when the bundle can be completed. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- ac5_records_unsupported_construct_line_warning --exact`
- **AC-6. Given** the same valid final G-code request and bytes on two clean output directories, **when** the command runs twice, **then** both manifests and all PNG bytes are identical, including warning ordering and image-entry ordering. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- ac6_final_gcode_render_is_deterministic --exact`

## Negative Test Cases

- **AC-N1. Given** a standalone `filled_areas` request without `gcode_line_width_mm`, **when** validation runs, **then** it rejects the request before parsing or PNG creation and reports that the explicit line width is required. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- ac_n1_rejects_filled_areas_without_line_width --exact`
- **AC-N2. Given** a G-code file containing only unsupported motion constructs and no supported renderable moves, **when** rendering runs, **then** the command fails without reporting a successful partial bundle and preserves the unsupported line-number diagnostics. | `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd -- ac_n2_rejects_input_with_no_supported_renderable_moves --exact`

## Verification

- `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - direct read of the complete 288-line proposal (grew from 235 lines when packet 158 added its Typed Post-Stage Capture subsection); this packet's authoritative ranges are lines 61-98 (Command And Request Contract, standalone mode, `gcode_line_width_mm`), 119-125 and 186-195 (Bundle Contract intro and Visualization Types, explicitly skipping the packet-158-owned "Typed Post-Stage Capture" subsection at 126-185), 218-231 (Final G-code Path), 233-267 (Stage Tap Inventory, `final_gcode` row at line 266), and 276-288 (Candidate Packets, "Final G-code renderer" row at line 283).
- `docs/19_visual_debug.md` - direct read of the complete 95-line usage contract (grew from 58 lines when packet 158 added its Model-Backed Typed Captures subsection); this packet's authoritative ranges are lines 16-33 (Request Shape) and 35-43 (Reading A Bundle), plus the standalone-G-code `gcode_line_width_mm`/`unclassified`/warnings guidance embedded at lines 82-87 inside the otherwise packet-158-owned "Model-Backed Typed Captures" section.
- `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` - direct read of the complete 41-line accepted decision; separate command, opt-in artifact lifecycle, and no partial evidence.
- `docs/01_system_architecture.md` - direct read of lines 460-497 and 562-589; postpass serialization and final text source boundary.
- `docs/11_operational_governance_and_acceptance_gate.md` - direct read of the complete 179-line governance contract; determinism, recoverability, and acceptance evidence obligations.
- `docs/07_implementation_status.md` - delegated bounded lookup for TASK-270 at lines 239-242; backlog ownership and dependency context.
- `docs/specs/visual-pipeline-debug-plan.md` - direct read of the complete 15-line packet queue; packet 160 dependency ordering.
- `.ralph/specs/157-visual-debug-request-bundle-contract/packet.spec.md` - direct read of the packet contract (`status: implemented`); dependency-owned request, manifest, and lifecycle fields.
- `.ralph/specs/158-visual-debug-typed-tap-capture/packet.spec.md` - direct read of the packet contract (`status: implemented`); confirms the typed-tap capture fields (`executed_stage_ids`, `layer_expansions`, `executed_layer_indices`) this packet must leave empty/unused for the standalone G-code path, and confirms `crates/pnp-cli/src/visual_debug.rs` as the sole integration file.

## Doc Impact Statement (Required)

- **`none`** - this packet implements the existing final-rendering contract and changes no IR, WIT, scheduler, claim, manifest schema ownership, host-service, or SDK contract.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCodeReader.hpp` and `OrcaSlicerDocumented/src/libslic3r/GCodeReader.cpp` — documented motion-token and line-state parsing locations to compare selectively; do not claim full Orca parity.
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.hpp` and `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` — documented extrusion-mode, role, layer, and warning-state locations to compare selectively.
- `OrcaSlicerDocumented/src/libvgcode/src/GCodeInputData.cpp` and `OrcaSlicerDocumented/src/slic3r/GUI/GCodeViewer.cpp` — documented preview geometry/render-input locations for bounded viewport and motion interpretation context only.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
