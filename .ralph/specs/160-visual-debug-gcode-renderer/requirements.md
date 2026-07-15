# Requirements: 160-visual-debug-gcode-renderer

## Packet Metadata

- Grouped task IDs: `TASK-270`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 157 (implemented, commit `3e33ca01`) and packet 158 (implemented) establish the opt-in visual-debug command, bundle contract, and typed-tap capture, but the standalone final-G-code source still cannot produce the final artifact a printer would receive: `crates/pnp-cli/src/visual_debug.rs:519-561`'s `VisualDebugSource::Gcode` arm is a verified placeholder that never opens the request's G-code file, never parses it, uses only the first requested layer for every synthesized image, and never writes PNG bytes (no PNG-encoding dependency exists in the crate). TASK-270 closes that rendering slice without depending on the intermediate renderer. It must make unsupported input visible as warnings, preserve unknown-role extrusion, iterate every requested/resolved layer, and fail rather than create misleading evidence.

## In Scope

- Parse serialized final G-code after `PostPass::TextPostProcess`.
- Support the documented Pinch 'n Print `G0`/`G1` X/Y/Z/E/F subset.
- Track `;LAYER_CHANGE`, `;Z:`, `;TYPE:`, extrusion mode markers, motion state, layer selection, and source line numbers.
- Emit final `filament_lines` PNGs and standalone `filled_areas` PNGs using the request's `gcode_line_width_mm`.
- Iterate every requested/resolved layer per tap/visualization combination; the current placeholder's `req.layers.first()`-only behavior must not survive.
- Preserve role-less extrusion as `unclassified` and record its warning.
- Record unsupported constructs as manifest warnings without approximation.
- Reuse packet 157's manifest, output lifecycle, overwrite, viewport, palette, and parser-version fields.
- Keep ordering and raster output deterministic.

## Out of Scope

- Typed intermediate taps or IR adapters.
- Scheduler dependency closure, scheduler rules, or module-visible access.
- Intermediate typed geometry rendering, synthetic stage diagrams, or a second renderer abstraction claimed by packet 159.
- Agent-facing skill or workflow documentation.
- Ordinary `pnp_cli slice` capture, allocation, serialization, rendering, or overhead work.
- Full OrcaSlicer G-code-preview parity or translated Orca source.
- Changes to WIT, IR schemas, module manifests, or scheduler contracts.

## Authoritative Docs

- `docs/specs/visual-pipeline-debug.md` - complete 288-line proposal (grew from 235 lines via packet 158's doc-impact update); this packet's ranges are 61-98 (Command And Request Contract, standalone mode), 119-125 and 186-195 (Bundle Contract intro and Visualization Types — skipping the packet-158-owned Typed Post-Stage Capture subsection at 126-185), 218-231 (Final G-code Path), 233-267 (Stage Tap Inventory, `final_gcode` row at 266), and 276-288 (Candidate Packets, packet 160's row at 283).
- `docs/19_visual_debug.md` - complete 95-line usage and inspection contract (grew from 58 lines via packet 158's doc-impact update); this packet's ranges are 16-33 (Request Shape), 35-43 (Reading A Bundle), and 82-87 (standalone-G-code width/warnings guidance).
- `docs/adr/0039-visual-debug-is-a-separate-opt-in-artifact-command.md` - complete 41-line accepted command and artifact-lifecycle decision.
- `docs/01_system_architecture.md` - lines 460-497 and 562-589, postpass and serialized G-code boundaries.
- `docs/11_operational_governance_and_acceptance_gate.md` - complete 179-line determinism, recoverability, and gate contract.
- `docs/07_implementation_status.md` - delegated line 223, TASK-270 ownership (`docs/07` reflowed after prior packets closed; TASK-270 is a single bullet, not a 239-242 range).
- `docs/specs/visual-pipeline-debug-plan.md` - complete packet queue and packet 160 dependency row.
- `.ralph/specs/157-visual-debug-request-bundle-contract/packet.spec.md` - dependency-owned request and manifest contract (`status: implemented`).
- `.ralph/specs/158-visual-debug-typed-tap-capture/packet.spec.md` - lines 21-25, confirms `crates/pnp-cli/src/visual_debug.rs` as the sole integration file (`status: implemented`).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCodeReader.hpp` and `OrcaSlicerDocumented/src/libslic3r/GCodeReader.cpp` — documented motion-token and line-state parsing locations to compare selectively; do not claim full Orca parity.
- `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.hpp` and `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.cpp` — documented extrusion-mode, role, layer, and warning-state locations to compare selectively.
- `OrcaSlicerDocumented/src/libvgcode/src/GCodeInputData.cpp` and `OrcaSlicerDocumented/src/slic3r/GUI/GCodeViewer.cpp` — documented preview geometry/render-input locations for bounded viewport and motion interpretation context only.

## Acceptance Summary

- Positive: `AC-1` through `AC-6` in `packet.spec.md`; these cover supported parsing, final PNG and manifest entries, unclassified preservation, requested line width, state/viewport determinism, warnings, and repeatability.
- Negative: `AC-N1` through `AC-N2` in `packet.spec.md`; these cover missing filled-area width and inputs with no supported renderable moves.
- Cross-packet impact: packet 157 supplies the command validation and bundle lifecycle; packet 158 supplies the manifest's typed-capture fields this packet leaves empty for the standalone G-code path; packet 161 owns broader contract coverage and ordinary-slice overhead verification.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p pnp-cli --all-targets --test visual_debug_gcode_renderer_tdd` | Prove supported parsing, PNG/manifest output, warnings, negative rejection, and determinism. | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets` | Compile the renderer and all test targets without changing unrelated contracts. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Enforce workspace lint cleanliness. | FACT pass/fail |

## Step Completion Expectations

The parser's source-order state must feed the renderer before manifest serialization; no PNG or manifest success may be exposed until all requested artifacts are written. The parser version, warning ordering, image ordering, and viewport calculation must be deterministic across runs.

## Context Discipline Notes

OrcaSlicerDocumented paths are location-only delegated references. Do not load implementation sources, generated output, or broad workspace code while implementing this packet. Cargo commands are delegated with bounded FACT results.
