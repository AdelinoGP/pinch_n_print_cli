---
status: draft
packet: 160-visual-debug-gcode-renderer
task_ids:
  - TASK-270
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
copy_note: Packet 160 is the final-G-code renderer slice and depends on packet 157.
---

# Packet Contract: 160-visual-debug-gcode-renderer

## Goal

Implement the standalone final-G-code visual-debug path that parses the documented Pinch 'n Print G0/G1 subset and writes deterministic final-view PNG artifacts plus manifest warnings.

## Scope Boundaries

This packet owns parsing serialized final G-code after `PostPass::TextPostProcess`, tracking the documented layer, Z, type, motion, and extrusion-mode markers, projecting supported moves into the existing visual-debug bundle contract, and emitting final PNG output. It preserves extrusion whose role is not classified as `unclassified` and reports unsupported constructs with source line numbers. It does not add typed taps, scheduler dependency closure, an intermediate renderer, the agent skill, or ordinary slice instrumentation.

## Prerequisites and Blockers

- Depends on: packet 157 visual-debug request/bundle contract and ADR-0039.
- Unblocks: packet 161 visual-debug agent verification.
- Activation blockers: Independent preflight review and the packet 157 forward contracts below. Packet 157 is active, not assumed implemented; packet 160 must not begin integration until these contracts are verified.

### [FWD] Packet 157 Dependency Contracts

- **Request export:** packet 157 must expose the parsed and normalized standalone request shape for `source.kind: "gcode"`, including `source.path`, `layers`, `taps`, `visualizations`, `resolution_scale`, and optional `gcode_line_width_mm`. The renderer consumes this validated shape and must not reimplement request validation. Verification/acceptance condition: packet 157 AC-2, AC-4, and AC-N4 pass, including the packet 157 request-contract test binary.
- **Bundle export:** packet 157 must expose the successful bundle lifecycle handoff that accepts all requested PNG artifacts and their manifest entries, writes them under the output directory, and publishes no partial bundle on failure, including explicit overwrite handling. The renderer supplies complete artifacts and warnings before commit. Verification/acceptance condition: packet 157 AC-5, AC-N5, and AC-N6 pass, including the packet 157 lifecycle test binary.
- **Manifest export:** packet 157 must expose the versioned manifest and image-entry shape with `source.kind`, `tap`, layer index, applicable Z, visualization, PNG path, shared viewport, legend version, source parser-version slot, and warnings, plus deterministic serialization. The renderer fills the G-code parser-version slot and warning/image-entry data without changing manifest ownership. Verification/acceptance condition: packet 157 AC-3 and AC-4 pass, including the packet 157 manifest test binary.
- **Dependency gate:** packet 157's request, bundle, and manifest acceptance tests must pass and its exports must be available at the integration seam before packet 160 Step 2 starts; otherwise packet 160 remains blocked and does not guess replacement types or lifecycle behavior.

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

- `docs/specs/visual-pipeline-debug.md` - direct read of the complete 235-line proposal; final-G-code source, supported subset, render contract, warnings, and packet boundary.
- `docs/19_visual_debug.md` - direct read of the complete 58-line usage contract; standalone request, bundle inspection, unclassified extrusion, and failure behavior.
- `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` - direct read of the complete 41-line accepted decision; separate command, opt-in artifact lifecycle, and no partial evidence.
- `docs/01_system_architecture.md` - direct read of lines 460-497 and 562-589; postpass serialization and final text source boundary.
- `docs/11_operational_governance_and_acceptance_gate.md` - direct read of the complete 179-line governance contract; determinism, recoverability, and acceptance evidence obligations.
- `docs/07_implementation_status.md` - delegated bounded lookup for TASK-270 at lines 239-242; backlog ownership and dependency context.
- `docs/specs/visual-pipeline-debug-plan.md` - direct read of the complete 15-line packet queue; packet 160 dependency ordering.
- `.ralph/specs/157-visual-debug-request-bundle-contract/packet.spec.md` - direct read of the packet contract; dependency-owned request, manifest, and lifecycle fields.

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
