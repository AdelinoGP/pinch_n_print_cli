# Requirements — Packet 126: MMU Painted-Cube OrcaSlicer Parity

## Problem Statement

PNP's slice of the all-faces-painted MMU fixture `resources/cube_4color.3mf` diverges visibly from OrcaSlicer (classic perimeters). The root cause, established in this diagnose session, is a **chain of workarounds over one broken port**:

1. PNP ported OrcaSlicer's faithful colour decomposition — the leftmost-arc walk `extract_colored_segments` over the `MMU_Graph` — but the port is **broken**: `get_next_arc` ignores the seed colour (walks cross colour boundaries) and `segments_to_expolygons_by_color` emits each walk under *every* distinct colour it touches instead of its seed colour. With those bugs the walk floods colours across the whole part.
2. To avoid that, a prior implementer **bypassed** the walk with a boostvoronoi per-cell shortcut (`cells_to_expolygons_by_color`). That shortcut does not tile completely: it leaves thin unassigned wedges at multi-cell junctions (e.g. between two circles on a detail face) — the visible cross-colour gaps.
3. To mask those holes' symptom at vertical-face boundaries, **corner-displacement + foreign-edge-shrink post-passes** shoved each colour's shared-corner vertices ~0.5 mm inward — which is exactly the ~0.3–0.5 mm cross-colour gap users reported, and which **gamed the confinement TDD tests** (they passed only because the geometry was distorted to dodge an over-strict "any vertex near the face plane = bleed" predicate).

OrcaSlicer has none of this: its arc walk consumes each arc exactly once and tiles the painted area completely with one colour per walk (`MultiMaterialSegmentation.cpp:494-566`). The parity fix is to make PNP's walk faithful and retire both workarounds. Alongside the decomposition fix, several painted-face colour gaps remain (G1–G3, G7, G8), and several colour/flow/infill fixes already landed this session (G4/G5/G6, RC1/RC2/RC3, corner-displacement removal + confinement-test correction) and must be verified and documented for review.

## Task Mapping

- `TASK-245`, `TASK-246` (closed, packet 95) — paint-segmentation OrcaSlicer-parity port phases 1–7. This packet completes the Phase 4f (`extract_colored_segments`) port that packet 95 left non-faithful and that packet 96 bypassed.
- `DEV-009` (open) — live-path output quality bar (top/bottom fill, colour parity).
- Roadmap `P110` (draft, Voronoi/SKT) — related but separate; this packet is the colour-decomposition slice, not the SKT/arachne work.
- Untracked: RC5 volumetric flow and the filament-palette fix have no `docs/07` entry; this packet records them (see Doc Impact Statement).

## In Scope

- Fix `get_next_arc` (colour filter + Orca leftmost-angle convention) and `segments_to_expolygons_by_color` (seed-colour assignment) in `crates/slicer-core/src/algos/paint_segmentation/extract_segments.rs` and `mod.rs`.
- Switch `execute_paint_segmentation`'s active decomposition from `cells_to_expolygons_by_color` to the faithful `extract_colored_segments` walk; verify `from_colored_lines` builds BORDER/NON_BORDER arcs as Orca's `build_graph` expects; port the CCW + self-intersection repair (Orca 547-563) as needed for clean closure.
- Keep the corner-displacement removal + confinement-test predicate corrections (already in working tree) and the 4-colour-square regression guard.
- Remove the now-dead `cells_to_expolygons_by_color` path (or gate it off) once the walk is the live path.
- G1: eliminate the top/bottom diagonal-seam colour sliver (`top_bottom.rs` projection union/opening, Phase-7 precedence in `mod.rs`).
- G2: eliminate spurious inner walls on detail faces (`painted_line_collection.rs` / `colorize.rs`).
- G3: first-layer/bottom-shell perimeter colour inheritance from the bottom surface.
- G7: default face colour reads object base extruder (3MF `<metadata key="extruder">`), not hardcoded tool 0.
- G8: skip the tool-0 default projection for subdivided horizontal facets that carry strokes.
- Verify and document the landed fixes: G4 (internal solid infill), G5 (volumetric flow), G6 (shell inset), RC1 (palette), RC2 (default tool), RC3 (side colours).

## Out of Scope

- `arachne-perimeters` (WIP — bimodal wall widths). All verification uses the curated classic-only module dir.
- SKT / variable-width perimeters / `P110` roadmap.
- Support, skirt/brim, wipe-tower, ironing stage behaviour beyond what the cube fixtures already exercise.
- Any fixture other than `cube_4color.3mf` and `cube_fuzzyPainted.3mf`.
- WIT/IR schema changes (G4's `InternalSolidInfill` variant already exists and is marshalled).

## Authoritative Docs

- `docs/01_system_architecture.md` (large — delegate SUMMARY for PrePass ordering / claim system).
- `docs/02_ir_schemas.md` (large — read only `SlicedRegion`, `ExtrusionRole`, `ResolvedConfig` sections via line range).
- `docs/08_coordinate_system.md` (small — read directly).
- `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` (small — read directly; the governing decision).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:390-456` — `get_next_arc` colour filter + leftmost-angle convention.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:494-566` — `extract_colored_segments` seed-colour assignment + CCW/self-intersection repair.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp:1714` — `build_graph` BORDER/NON_BORDER arc construction.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1599-1629` — per-colour independent `offset_ex` (ADR-0013 baseline).
- `OrcaSlicerDocumented/src/libslic3r/GCode.cpp` — flow formula (G5 verify-only).
- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` + `Fill/` — top/bottom solid classification (G4 verify-only); bottom-shell perimeter colouring (G3 reference).

## Acceptance Summary

Authoritative criteria are in `packet.spec.md` (AC-1…AC-4, AC-G1/G2/G3/G7/G8, AC-V-G4/G5/G6/RC1/RC3, AC-N1/N2/N3). Measurable refinements not captured in the Given/When/Then:

- AC-1 completeness threshold is 1% painted-coverage rel-diff per layer (the existing `PNP_PAINTSEG_CELL_TILING_DEBUG` metric); the pre-fix baseline is 124 layers >1% (with corner-displacement) → 21 layers (without) → target 0.
- AC-4 gap threshold: adjacent-colour outer-wall centreline spacing on the left face must be ≈ one line width (0.40–0.45mm), never the ~1.0mm empty band observed pre-fix at z=4.0 and z=18.0.
- AC-G2 reference counts: pre-S2 inner-wall segments ≈ 7090, post-S2 ≈ 4698, Orca ≈ 3239; target ≤ 3600.
- AC-V-G5 expected flow: E/mm ≈ `width*layer_height/filament_area` ≈ 0.033 across roles (pre-fix 0.55/0.72, ~16–22× high).

## Verification Commands

| ID | Command | Delegation hint |
|----|---------|-----------------|
| build | `cargo build --bin pnp_cli --release` | sub-agent → FACT ok/err + first error line |
| curated-dir | build `scratchpad/modules-classic` = all `modules/core-modules/*` except `arachne-perimeters`, copying only each module's `*.toml`(non-Cargo) + `*.wasm` | one-time setup; sub-agent → FACT "N modules, arachne excluded" |
| AC-1 | `PNP_PAINTSEG_CELL_TILING_DEBUG=1 ./target/release/pnp_cli slice --model resources/cube_4color.3mf --no-default-module-paths --module-dir scratchpad/modules-classic --output target/p126.gcode 2>&1 \| grep -c "cell-tiling"` | FACT: integer (expect 0) |
| AC-2 | `cargo test -p slicer-runtime --test executor -- cube_4color_back_face_uniform cube_4color_right_face_uniform cube_fuzzy_painted_left_face cube_fuzzy_painted_bottom_face 2>&1 \| tee target/test-output.log \| grep "test result"` | FACT: pass/fail counts |
| AC-3 | `cargo test -p slicer-core --lib --features host-algos -- extract 2>&1 \| grep "test result"` | FACT: pass/fail |
| AC-4 | `cargo test -p slicer-core --lib --features host-algos -- cube_4color_left_face_circles_tile_without_gap 2>&1 \| grep "test result"` | FACT: pass/fail (new test) |
| AC-G1 | `cargo test -p slicer-runtime --test executor -- cube_4color_top_face cube_4color_bottom_face 2>&1 \| grep "test result"` | FACT: pass/fail |
| AC-G2 | `grep -c ";TYPE:Inner wall" target/p126.gcode` | FACT: integer (≤3600) |
| AC-G3 | `cargo test -p slicer-runtime --test executor -- cube_4color_first_layer_perimeter_colour_matches_bottom_face 2>&1 \| grep "test result"` | FACT: pass/fail (new test) |
| AC-G7 | `cargo test -p slicer-core --lib --features host-algos -- default_face_colour_uses_object_base_extruder 2>&1 \| grep "test result"` | FACT: pass/fail (new test) |
| AC-G8 | `cargo test -p slicer-core --lib --features host-algos -- subdivided_horizontal_face_skips_default_tool0_projection 2>&1 \| grep "test result"` | FACT: pass/fail (new test) |
| AC-V-G4 | `grep -c ";TYPE:Internal solid infill" target/p126.gcode` | FACT: integer (>0) |
| AC-V-G5 | `cargo test -p slicer-gcode -- emit_e_uses_volumetric_flow_formula 2>&1 \| grep "test result"` | FACT: pass/fail |
| AC-V-G6 | `cargo test -p slicer-core --lib --features host-algos -- propagate_top_bottom 2>&1 \| grep "test result"` | FACT: pass/fail |
| AC-V-RC1 | `grep -c "#FF9B00;#02BF06;#1800F2;#EC0006" target/p126.gcode` | FACT: integer (>0) |
| AC-V-RC3 | `grep -oE "^T[0-3]$" target/p126.gcode \| sort -u \| wc -l` | FACT: integer (==4) |
| AC-N3 | `cargo test -p slicer-core --lib --features host-algos -- cells_to_expolygons_four_color_square 2>&1 \| grep "test result"` | FACT: pass/fail |
| gate | `cargo clippy --workspace --all-targets -- -D warnings` | sub-agent → FACT clean/first-warning |

## Step Completion Expectations (cross-step invariants)

- After every step that edits `extract_segments.rs`, `mod.rs`, `slice_ir.rs`, `slicer-macros`, `slicer-sdk`, or any `modules/core-modules/*/src`, run `cargo xtask build-guests --check` and rebuild if `STALE:` before attributing any guest/dispatch/module test failure to the change (G4 already shipped this path; later steps that touch infill modules re-trigger it).
- The corner-displacement post-passes (Step 9/10 in `voronoi_graph.rs`) must remain removed across all steps; AC-N3 guards against reintroduction.
- All slicing verification uses the curated classic-only module dir; a run that loads `arachne-perimeters` is invalid evidence (bimodal wall widths confound every wall-coverage AC).

## Context Discipline Notes (packet-specific)

- The arc-walk debugging this session showed that incremental guessing on the angle convention does not converge — Step 3 must validate `from_colored_lines` graph construction against Orca's `build_graph` (delegated SUMMARY) *before* tweaking `get_next_arc`, and validate each sub-change against the `extract_*` unit tests + the four confinement tests, not the full slice.
- Do not re-derive the fix history from the source; the diagnosis is captured here and in `scratchpad/wip_extract_segments_arcwalk.rs` (preserved WIP) + `git stash@{0}`.
