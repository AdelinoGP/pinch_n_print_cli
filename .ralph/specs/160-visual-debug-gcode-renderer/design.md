# Design: 160-visual-debug-gcode-renderer

## Controlling Code Paths

- Primary code path: the packet 157 `pnp_cli visual-debug` standalone-G-code branch, extended with a final-G-code parser and PNG renderer after request validation and before bundle commit.
- Neighboring tests/fixtures: `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`, with deterministic inline G-code fixtures covering markers, modes, roles, unsupported commands, and no-renderable input.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Final rendering consumes serialized text after `PostPass::TextPostProcess`, not `GCodeIR`, so the artifact represents the printer-facing final G-code.
- This is an opt-in artifact path only. No `slice` flag, typed tap, scheduler edge, module invocation, ordinary-slice allocation, or intermediate-renderer dependency may be introduced.
- Unsupported commands are warnings with source line numbers, never guessed geometry; a bundle succeeds only when its requested PNGs and manifest are complete.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add a narrow parser/state model for the documented PnP subset, convert supported moves into renderer-owned segments, compute one deterministic XY viewport, rasterize requested final views, then pass complete entries and warnings to packet 157's bundle writer.
- Exact functions, traits, manifests, tests, and fixtures: the packet 157 visual-debug command/request and manifest integration in `crates/pnp-cli/src/main.rs` and `crates/pnp-cli/src/lib.rs`; the new focused final-G-code parser/renderer module `crates/pnp-cli/src/visual_debug_gcode.rs`; `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`; and the existing pnp-cli manifest/dependency surface only if the selected PNG encoder requires it.
- Rejected alternatives and reasons: parsing `GCodeIR` is rejected because final text post-processing can change the artifact; extending `pnp_cli slice` is rejected by ADR-0039; inferring filled-area width from E is rejected by the visual-debug contract; importing Orca preview code is rejected because v1 is a documented PnP subset, not full parity.

## Files in Scope (read + edit)

- `crates/pnp-cli/src/main.rs` - existing command dispatch and packet 157 visual-debug entry point; expected change: route the standalone `final_gcode` request to the renderer.
- `crates/pnp-cli/src/lib.rs` - pnp-cli library/module ownership and shared visual-debug wiring; expected change: expose the focused parser/renderer integration without changing ordinary slice behavior.
- `crates/pnp-cli/src/visual_debug_gcode.rs` - new focused final-G-code parser/state model, segment projection, viewport calculation, and PNG rendering; expected change: implement the selected parser/renderer design without owning request validation or bundle lifecycle.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - renderer contract tests and deterministic inline fixtures; expected change: add positive and negative assertions named by this packet's ACs.

## Read-Only Context

- `docs/specs/visual-pipeline-debug.md` - lines 61-178 and 195-213 - request, bundle, coordinate, visualization, and final-G-code source contract.
- `docs/19_visual_debug.md` - lines 16-50 - command usage, bundle inspection, width requirement, warnings, and failure semantics.
- `docs/01_system_architecture.md` - lines 460-497 and 562-589 - postpass serialization ownership.
- `.ralph/specs/157-visual-debug-request-bundle-contract/packet.spec.md` - lines 27-65 - dependency-owned request, manifest, lifecycle, and test contract.
- `OrcaSlicerDocumented/...` - delegate locations only; never load directly.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/`, `crates/slicer-ir/`, `crates/slicer-scheduler/`, WIT files, and module manifests - no contract or scheduler changes in this packet.
- Typed tap and intermediate-renderer implementation surfaces - no reads or edits unless packet 157 integration requires a symbol lookup delegated by name.
- Agent skill files and ordinary `slice` performance/instrumentation paths - no reads or edits.

## Expected Sub-Agent Dispatches

- Question: identify the exact packet 157 standalone-G-code request, manifest-entry, bundle-writer, and command-dispatch symbols to extend; scope: `crates/pnp-cli/src/**` and packet 157 artifacts; return: `LOCATIONS`; purpose: prevent guessed integration names.
- Question: verify bounded Orca locations for motion parsing, extrusion mode, role/layer metadata, and preview geometry context; scope: `OrcaSlicerDocumented/src/libslic3r/GCodeReader.*`, `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.*`, `OrcaSlicerDocumented/src/libvgcode/src/GCodeInputData.cpp`, `OrcaSlicerDocumented/src/slic3r/GUI/GCodeViewer.cpp`; return: `LOCATIONS`; purpose: constrained parity reference.
- Question: run each targeted test and workspace gate and return only pass/fail or bounded failure snippets; scope: repository commands; return: `FACT`; purpose: falsify the packet criteria without loading large output.

## Data and Contract Notes

- IR/manifest contracts: no IR changes; reuse packet 157 image-entry fields and add the documented G-code parser version, source line warnings, unclassified role, viewport, and PNG paths through its existing manifest model.
- WIT boundary: none.
- Determinism/scheduler constraints: source-order parsing, stable warning/image ordering, fixed palette/legend, shared model-wide viewport, and no scheduler interaction.

## Locked Assumptions and Invariants

- Standalone G-code is the only source mode for this renderer and `final_gcode` is the only tap it owns.
- `filled_areas` requires explicit `gcode_line_width_mm`; E values never determine physical bead width.
- Role-less extrusion is retained as `unclassified`; unsupported constructs are not approximated.
- Successful output contains the complete requested manifest and PNG set, never a partial success.

## Risks and Tradeoffs

- The documented subset intentionally omits full printer macro and preview semantics; warnings make that limitation observable instead of silently fabricating geometry.
- A new pure-Rust PNG dependency may be required; implementation must record its enabled features and license review without expanding this packet into unrelated rendering work.
- Existing packet 157 symbols may not yet be implemented; unresolved integration names are a worker discovery dispatch, not permission to broaden scope.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: packet 157 integration symbol lookup; `LOCATIONS` with at most 20 file:line entries.

## Open Questions

- [FWD] Packet 157 must provide the request, bundle-lifecycle, and manifest/image-entry exports and acceptance evidence specified in `packet.spec.md` before this packet's integration is implemented; no replacement contract may be invented here.
