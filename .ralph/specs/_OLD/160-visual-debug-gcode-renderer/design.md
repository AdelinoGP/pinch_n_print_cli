# Design: 160-visual-debug-gcode-renderer

## Controlling Code Paths

- Primary code path: `run_visual_debug`'s `VisualDebugSource::Gcode { path, .. }` match arm (`crates/pnp-cli/src/visual_debug.rs:519-561`) — a verified placeholder today (never opens `path`, never parses, uses only `req.layers.first()` for every image, writes no PNG bytes). This packet replaces that arm's body with a real parser/renderer call, still inside `run_visual_debug`, still before the existing atomic `manifest.json` commit at `visual_debug.rs:604-620`.
- Neighboring tests/fixtures: `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs`, with deterministic inline G-code fixtures covering markers, modes, roles, unsupported commands, and no-renderable input.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Final rendering consumes serialized text after `PostPass::TextPostProcess`, not `GCodeIR`, so the artifact represents the printer-facing final G-code.
- This is an opt-in artifact path only. No `slice` flag, typed tap, scheduler edge, module invocation, ordinary-slice allocation, or intermediate-renderer dependency may be introduced.
- Unsupported commands are warnings with source line numbers, never guessed geometry; a bundle succeeds only when its requested PNGs and manifest are complete.
- `Viewport { width, height }` (`visual_debug.rs:264-267`) is pixel dimensions derived from `resolution_scale` only, already computed identically for both source arms before dispatch (`visual_debug.rs:499-503`); this packet must not change its shape. The "one model-wide XY viewport" requirement (AC-4) is an internal mm-space bounding-box computation (parsed geometry + documented margin) used only to project consistently into that already-shared pixel canvas.
- No PNG-encoding dependency exists in `crates/pnp-cli/Cargo.toml` today, and no code path in the crate writes PNG bytes yet — this packet is the first to add both. PNG files must be fully written before the existing atomic `manifest.json` temp-then-rename commit (`visual_debug.rs:604-620`), so a failed run never leaves a successful-looking `manifest.json` next to partial/missing images.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

## Code Change Surface

- Selected approach: add a narrow parser/state model for the documented PnP subset in a new private submodule, convert supported moves into renderer-owned segments per requested layer, compute one deterministic XY bounding box internally, rasterize requested final views with a new PNG dependency, then replace the placeholder `Gcode` arm body (`visual_debug.rs:519-561`) so it returns real `ImageEntry` values (looped over requested layers, not just `req.layers.first()`) and writes real PNG bytes before the existing atomic manifest commit.
- Exact functions, traits, manifests, tests, and fixtures: the existing `run_visual_debug` `Gcode` match arm in `crates/pnp-cli/src/visual_debug.rs:519-561` (edit in place); a new private submodule `crates/pnp-cli/src/visual_debug_gcode.rs` declared with `mod visual_debug_gcode;` inside `visual_debug.rs` (no `lib.rs` change — the module is private to the crate's binary/lib target the same way `visual_debug.rs` already is) holding the parser/state model, segment projection, bounding-box computation, and PNG rasterization; `crates/pnp-cli/Cargo.toml` to add the new PNG-encoding dependency; `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` for the new AC tests.
- Rejected alternatives and reasons: parsing `GCodeIR` is rejected because final text post-processing can change the artifact; extending `pnp_cli slice` is rejected by ADR-0039; inferring filled-area width from E is rejected by the visual-debug contract; importing Orca preview code is rejected because v1 is a documented PnP subset, not full parity; editing `main.rs`/`lib.rs` is rejected because the verified integration seam is entirely inside `visual_debug.rs` and neither file needs a symbol added.

## Files in Scope (read + edit)

- `crates/pnp-cli/src/visual_debug.rs` - verified integration seam; expected change: replace the placeholder `VisualDebugSource::Gcode` arm body (lines 519-561) to call the new parser/renderer module, loop over all requested/resolved layers (not just the first), and write real PNG files before the existing atomic manifest commit; add `mod visual_debug_gcode;`.
- `crates/pnp-cli/src/visual_debug_gcode.rs` - new private submodule: final-G-code parser/state model, segment projection, bounding-box computation, and PNG rendering; expected change: implement the selected parser/renderer design without owning request validation or bundle lifecycle (both stay in `visual_debug.rs`).
- `crates/pnp-cli/Cargo.toml` - expected change: add the selected pure-Rust PNG-encoding dependency (none exists in this crate today); record enabled features per `docs/specs/visual-pipeline-debug.md` lines 207-211.
- `crates/pnp-cli/tests/visual_debug_gcode_renderer_tdd.rs` - renderer contract tests and deterministic inline fixtures; expected change: add positive and negative assertions named by this packet's ACs.

`crates/pnp-cli/src/main.rs` and `crates/pnp-cli/src/lib.rs` are verified to need no changes (see `packet.spec.md` "Grounded Packet 157/158 Integration Facts") and are therefore not in scope.

## Read-Only Context

- `docs/specs/visual-pipeline-debug.md` - lines 61-98, 119-125, 186-195, 218-231, 233-267, and 276-288 - request, bundle, coordinate, visualization, final-G-code source contract, and packet queue ordering.
- `docs/19_visual_debug.md` - lines 16-33, 35-43, and 82-87 - command usage, bundle inspection, width requirement, warnings, and failure semantics.
- `docs/01_system_architecture.md` - lines 460-497 and 562-589 - postpass serialization ownership.
- `crates/pnp-cli/src/visual_debug.rs` - lines 16-48 (request/source types), 220-288 (manifest/image-entry types), 480-621 (`run_visual_debug` lifecycle, dispatch match, atomic commit) - the verified integration seam; treat as ground truth over any packet prose if the two ever disagree.
- `.ralph/specs/157-visual-debug-request-bundle-contract/packet.spec.md` - lines 27-65 - dependency-owned request, manifest, lifecycle, and test contract.
- `.ralph/specs/158-visual-debug-typed-tap-capture/packet.spec.md` - lines 21-25 - confirms `visual_debug.rs` as sole integration file and the typed-capture fields this packet leaves untouched.
- `OrcaSlicerDocumented/...` - delegate locations only; never load directly.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load.
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load.
- `modules/`, `crates/slicer-ir/`, `crates/slicer-scheduler/`, WIT files, and module manifests - no contract or scheduler changes in this packet.
- `crates/pnp-cli/src/main.rs`, `crates/pnp-cli/src/lib.rs` - verified to need no changes; do not edit.
- Typed tap and intermediate-renderer implementation surfaces (`run_model_source`, `visual_debug.rs:342-478`) - no reads or edits.
- Agent skill files and ordinary `slice` performance/instrumentation paths - no reads or edits.

## Expected Sub-Agent Dispatches

- Question: verify bounded Orca locations for motion parsing, extrusion mode, role/layer metadata, and preview geometry context; scope: `OrcaSlicerDocumented/src/libslic3r/GCodeReader.*`, `OrcaSlicerDocumented/src/libslic3r/GCode/GCodeProcessor.*`, `OrcaSlicerDocumented/src/libvgcode/src/GCodeInputData.cpp`, `OrcaSlicerDocumented/src/slic3r/GUI/GCodeViewer.cpp`; return: `LOCATIONS`; purpose: constrained parity reference.
- Question: identify the smallest pure-Rust PNG-encoding crate already vetted/used elsewhere in the workspace (if any) versus adding `png` fresh, and its license/feature footprint; scope: `Cargo.lock`-derived `cargo metadata --format-version=1 --no-deps` summary, not the lockfile itself; return: `FACT`; purpose: avoid guessing a dependency choice.
- Question: run each targeted test and workspace gate and return only pass/fail or bounded failure snippets; scope: repository commands; return: `FACT`; purpose: falsify the packet criteria without loading large output.

## Data and Contract Notes

- IR/manifest contracts: no IR or schema changes; `Manifest`/`ImageEntry` already carry every field this packet populates (`gcode_parser_version`, `png_path`, `viewport`, `legend_version`, `warnings`) — this packet only fills them with real values in place of the placeholder's synthesized ones.
- WIT boundary: none.
- Determinism/scheduler constraints: source-order parsing, stable warning/image ordering, fixed palette/legend, one internally-computed model-wide XY bounding box reused for every emitted PNG, and no scheduler interaction.

## Locked Assumptions and Invariants

- Standalone G-code is the only source mode for this renderer and `final_gcode` is the only tap it owns; `TapSelector` is untagged so no new enum variant is needed for the tap name.
- `filled_areas` requires explicit `gcode_line_width_mm`; E values never determine physical bead width. This requirement is already validated by packet 157 (`visual_debug.rs:197-215`); AC-N1's new test confirms existing behavior rather than exercising new validation logic.
- Role-less extrusion is retained as `unclassified`; unsupported constructs are not approximated.
- Successful output contains the complete requested manifest and PNG set, never a partial success; PNG files are written before the existing atomic `manifest.json` rename.
- The placeholder's `req.layers.first()`-only behavior must not survive: every requested/resolved layer needs its own rendered entry per requested tap/visualization combination.

## Risks and Tradeoffs

- The documented subset intentionally omits full printer macro and preview semantics; warnings make that limitation observable instead of silently fabricating geometry.
- A new pure-Rust PNG dependency is required (verified: none exists in `crates/pnp-cli/Cargo.toml` today); implementation must record its enabled features and license review without expanding this packet into unrelated rendering work.
- The placeholder's per-layer behavior (`req.layers.first()` only) must be restructured into a layer loop; this is a real, verified gap, not a hypothetical one — underestimating it risks an AC-1 regression where only one layer's images are ever produced.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: PNG-crate selection fact-check; `FACT` with the chosen crate name and feature/license summary.

## Open Questions

- None blocking. Packets 157 and 158 are `status: implemented` and their exports are grounded above with `file:line` citations. The only remaining decision is the PNG-encoding crate choice, which is a bounded Step 2 implementation dispatch (see Expected Sub-Agent Dispatches), not a `[FWD]`/`[BLOCK]`.
