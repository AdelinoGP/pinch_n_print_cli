# Requirements: 105_classic-spacing-fill-mmu

## Packet Metadata

- Grouped task IDs:
  - `T-050` — Port minimal `Flow::new_from_width_height` math (width → spacing) to `slicer-core::flow`
  - `T-051` — Distinct `outer_wall_line_width` + `inner_wall_line_width` (replace single `line_width`)
  - `T-052` — Implement `ext_perimeter_spacing2` (outer↔first-inner) vs `perimeter_spacing` (inner↔inner) arithmetic
  - `T-053` — Register + implement `precise_outer_wall` mode (gated on `wall_sequence == InnerOuter`)
  - `T-054` — Register `wall_sequence` enum in perimeter manifests; deregister from `path-optimization-default` per ADR-0011
  - `T-054b` — Implement `OuterInner` and `InnerOuter` modes in `wall_sequence_reorder` (in `slicer-core::perimeter_utils`)
  - `T-054c` — Implement `InnerOuterInner` sandwich mode (per-outer-contour grouping via in-module wall tree)
  - `T-060` — Register `detect_thin_wall` config key
  - `T-061` — Implement thin-wall detection cascade (`offset2_ex` + `opening_ex` + `medial_axis`)
  - `T-062` — Emit ThinWall geometry as `WallLoop { loop_type: ThinWall, role: ThinWall, is_thin_wall: true }`
  - `T-062b` — Add `LoopType::GapFill` + `ExtrusionRole::GapFill` variants; ensure `#[non_exhaustive]`; add match arms in downstream role-switching consumers
  - `T-063` — Implement gap collection per-inset (`diff_ex(offset(last, -0.5d), offset(offsets, 0.5d+safety))`)
  - `T-064` — Run `medial_axis` over collected gaps; filter by `filter_out_gap_fill`; emit as `WallLoop { loop_type: GapFill, role: GapFill }`
  - `T-065` — Register `gap_infill_speed` + `filter_out_gap_fill` config keys
  - `T-P96-A0` — OrcaSlicer-source investigation: produce `docs/specs/orca-mmu-perimeter-investigation.md` one-pager citing line-numbered MMU per-color paths + bisector tie-break rule
  - `T-P96-B` — Revert `external_contour` consumption in `classic-perimeters` and `arachne-perimeters`
  - `T-P96-C0` — Resurrect `SlicedRegion.bisector_edge_skip_mask: Vec<bool>` (flat per-edge, indexed against `SlicedRegion.polygons` per ADR-0013); host computes the mask at paint-segmentation commit using the tie-break rule from T-P96-A0
  - `T-P96-C1` — Classic-perimeters consumes mask: skip edges where `bisector_edge_skip_mask[i][j] == true` during outer-wall per-cell trace
  - `T-P96-C2` — Variable-width-perimeters consumes the mask (same per-cell trace logic — algorithmic equivalence with classic at the current iterative-inset approximation)
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`classic-perimeters` currently emits walls with a single configurable `line_width` (not distinguishing outer from inner), with a constant inter-wall spacing that ignores OrcaSlicer's `ext_perimeter_spacing2 vs perimeter_spacing` distinction, no thin-wall detection, no gap-fill, no `wall_sequence` modes, and an MMU dedup mechanism (`external_contour` from P96) that union-traces the model perimeter once per painted object — diverging from OrcaSlicer's per-color outer-wall fragmentation. The four defects compound: incorrect spacing on multi-width prints, missing thin features, gap-filled by infill or left as voids, single-color MMU wall regardless of paint, and an unparsable single sequence of walls per region (no sandwich mode, no inner-first option). (`variable-width-perimeters` never ships per D-110-DROP-VARIABLE-WIDTH; the fake-Arachne module is deleted under P108.)

This packet lands the entire wall-emission geometry stack in one coordinated change because the four workstreams touch the same `lib.rs` files (the perimeter modules), the same IR (`SlicedRegion`, `LoopType`, `ExtrusionRole`), and the same host-side surface (`paint_segmentation`). Splitting would require three sequential touches of the same files, each with its own compile-cycle and AC churn. The MMU foundation (T-P96-A0/B/C0/C1/C2) folds in because T-P96-C1/C2 modify the same per-cell wall-trace loop that the wall_sequence + thin-wall + gap-fill code paths rewrite — coupling at the LOC level, not just at the conceptual level. T-P96-A0 lands first as a doc-only investigation step so the tie-break rule for C0 is grounded in OrcaSlicer source rather than guessed.

## In Scope

- New `crates/slicer-core/src/flow.rs` exporting `pub fn line_width_to_spacing(width: f32, layer_height: f32, nozzle_diameter: f32) -> f32` and the related `flow_to_width` round-trip. Port the minimal subset of OrcaSlicer's `Flow::new_from_width_height` that the perimeter modules need.
- Extension to `crates/slicer-core/src/perimeter_utils.rs`: `pub fn wall_sequence_reorder(walls: &mut Vec<WallLoop>, mode: WallSequence, wall_tree: &[PolygonTreeNode])` implementing all three modes.
- Config-key registrations in both perimeter manifests + `docs/15_config_keys_reference.md`: `outer_wall_line_width`, `inner_wall_line_width`, `precise_outer_wall`, `wall_sequence`, `detect_thin_wall`, `gap_infill_speed`, `filter_out_gap_fill`.
- Deregister `wall_sequence` from `modules/core-modules/path-optimization-default/path-optimization-default.toml` (ADR-0011 migration).
- New IR variants in `crates/slicer-ir/src/slice_ir.rs`: `LoopType::GapFill`, `ExtrusionRole::GapFill`. Both enums declared `#[non_exhaustive]`.
- Resurrect `SlicedRegion.bisector_edge_skip_mask: Vec<bool>` IR field (flat per-edge mask, indexed against `SlicedRegion.polygons` per ADR-0013; accessed via `edge_offset_for_polygon(region, poly_idx) + edge_j`); bump `CURRENT_SLICE_IR_SCHEMA_VERSION` from its live value to `4.4.0`.
- WIT mirrors in `crates/slicer-schema/wit/deps/ir-types.wit` for the new IR additions: `gap-fill` arm on `wall-loop-type`; `bisector-edge-skip-mask: func() -> list<bool>` accessor on the `slice-region-view` resource. `gap-fill` arm on `extrusion-role` in `crates/slicer-schema/wit/deps/types.wit`.
- Host-side bisector-mask computation in `crates/slicer-core/src/algos/paint_segmentation/` — populates `bisector_edge_skip_mask` deterministically per the tie-break rule named in T-P96-A0.
- `classic-perimeters/src/lib.rs` consumes the spacing model, runs thin-wall detection, runs gap-fill emission, applies `wall_sequence_reorder`, skips bisector-masked edges during per-cell outer-wall trace, removes `external_contour` consumption.
- ~~`arachne-perimeters/src/lib.rs` (or `variable-width-perimeters` post-rename) mirrors classic~~ — **dropped**: fake-Arachne module deleted under P108 (D-110-DROP-VARIABLE-WIDTH); real Arachne is created fresh by P110+P112.
- Downstream role-switching consumers gain a `GapFill` match arm: `modules/core-modules/part-cooling/src/lib.rs`, `modules/core-modules/machine-gcode-emit/src/lib.rs` (if it dispatches by role), the host GCodeEmit role priority table.
- New one-pager `docs/specs/orca-mmu-perimeter-investigation.md` from T-P96-A0.
- 6 new TDD files covering AC-1 through AC-6 + 3 negative cases.
- All Doc Impact Statement edits.

## Out of Scope

- `extra_perimeters` consumer (T-070/T-071) and `extra_perimeters_on_overhangs` (T-077) — Phase 7 work, lands in P106.
- Narrow-island `smaller_perimeter_line_width` handling (T-072/T-073) — Phase 7, P106.
- Non-planar wall emission (T-074b/c/d) — Phase 7, P106. The `surface_group` accessor it consumes lands in P104, not here.
- Seam-candidate quality (T-080/T-081/T-082/T-083) — Phase 8, P106.
- M1 verification harness, fixture recording, deviation closure (Phase 9) — P107.
- T-P96-A (reshape AC-22b assertion), T-P96-C3 (parity verification), T-P96-D (delete `external_contour` IR field), T-P96-F (re-baseline SHA + deviation entry) — Phase 9 cleanup, P107.
- T-P96-E (real Arachne MMU at boundary level) — M2 work.
- ~~Rename of `arachne-perimeters` → `variable-width-perimeters`~~ — **cancelled** (D-110-DROP-VARIABLE-WIDTH); deletion is P108's scope.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~700 lines | Range-read §"Phase 5", §"Phase 6", §"Inherited from P96 — AC-22b reshape obligation". |
| `docs/adr/0011-perimeter-module-owns-wall-sequencing.md` | ~50 lines | Read full. |
| `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md` | ~80 lines | Read full. |
| `docs/02_ir_schemas.md` | ~900 lines | Delegate SUMMARY for `LoopType`, `ExtrusionRole`, `SlicedRegion`, schema-version contract. Range-read around the modified definitions. |
| `docs/03_wit_and_manifest.md` | ~400 lines | Range-read §"WIT/Type Changes Checklist" (~30 lines). |
| `docs/01_system_architecture.md` | ~250 lines | Read §"Crate Boundaries" full. |
| `docs/15_config_keys_reference.md` | ~300 lines | Range-read §"Walls" and §"Quality". |
| `docs/DEVIATION_LOG.md` | varies | Range-read the most recent N entries (`D-96-AC22-*` rows) to align format. |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1501-1506,1644` — `ext_perimeter_spacing2`/`perimeter_spacing` + `precise_outer_wall` gating. SUMMARY ≤ 150 words.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1596-1609` — thin-wall cascade. SUMMARY ≤ 150 words.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1665-1670,1930-1958` — gap collection + emission. SUMMARY ≤ 150 words.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1801-1913` — `wall_sequence` reorder including `InnerOuterInner` sandwich. SUMMARY ≤ 200 words.
- `OrcaSlicerDocumented/src/libslic3r/Flow.cpp` — `Flow::new_from_width_height` math. SUMMARY ≤ 100 words.
- `OrcaSlicerDocumented/src/libslic3r/MultiMaterialSegmentation.cpp` and `PerimeterGenerator.cpp` per-color branches — MMU per-color outer-wall fragmentation + bisector tie-break rule. SUMMARY ≤ 200 words. **This is the deliverable of T-P96-A0** — the implementer dispatches this SUMMARY and writes `docs/specs/orca-mmu-perimeter-investigation.md` from it.

## Acceptance Summary

- Positive cases: `AC-1` (outer/inner widths + spacing arithmetic), `AC-2` (three wall_sequence modes), `AC-3` (ThinWall emission), `AC-4` (GapFill emission), `AC-5` (bisector_edge_skip_mask field + host populator + schema bump to 4.4.0), `AC-6` (MMU per-color fragmentation end-to-end).
- Negative cases: `AC-N1` (thin-wall config off → no ThinWall), `AC-N2` (no gaps → no GapFill, no panic), `AC-N3` (single-color → mask all-false, unchanged baseline).
- Refinements not captured in Given/When/Then:
  - `wall_sequence_reorder`'s `InnerOuterInner` per-outer-contour grouping uses the in-module wall tree built during generation; the tree is discarded before commit (per ADR-0011 — IR stays flat).
  - `bisector_edge_skip_mask` is a flat `Vec<bool>`; use `edge_offset_for_polygon(region, poly_idx) + edge_j` to index edge `(polygons[poly_idx].contour.points[edge_j], polygons[poly_idx].contour.points[(edge_j+1) % len])`. Offset helper `pub fn edge_offset_for_polygon(region: &SlicedRegion, poly_idx: usize) -> usize` is created in `crates/slicer-core/src/perimeter_utils.rs` by this packet (NET-NEW; consumed by P109). Documented in `slice_ir.rs` doc-comment.
  - `external_contour` IR field is **NOT** deleted by this packet — that's T-P96-D, deferred to P107. Both modules' code paths just stop consuming it.
- Cross-packet impact: depends on P102 + P103 (must ship first). Unblocks P106 (special modes + seam) and P107 (verification + closure).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile after IR + WIT + host additions | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace clippy gate | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration outer_inner_width_and_spacing_tdd` | AC-1 | FACT pass/fail |
| `cargo test -p slicer-core --test wall_sequence_reorder_tdd` | AC-2 (all 3 modes) | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration thin_wall_emission_tdd` | AC-3 + AC-N1 | FACT pass/fail per case |
| `cargo test -p slicer-runtime --test integration gap_fill_emission_tdd` | AC-4 + AC-N2 | FACT pass/fail per case |
| `cargo test -p slicer-core --test paint_segmentation_bisector_mask_tdd` | AC-5 host populator + symmetry | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration mmu_bisector_dedup_tdd` | AC-6 + AC-N3 end-to-end | FACT pass/fail per case |
| `cargo xtask build-guests --check` | Guest WASM coherence after WIT change | FACT clean / STALE list |
| `rg -q 'pub bisector_edge_skip_mask: Vec<bool>' crates/slicer-ir/src/slice_ir.rs` | AC-5 field present (flat Vec<bool>, ADR-0013 conformant) | FACT pass/fail |
| `! rg -q '\.external_contour\(\)' modules/core-modules/classic-perimeters/src/lib.rs modules/core-modules/arachne-perimeters/src/lib.rs` | AC-6 revert complete | FACT pass/fail |
| `rg -q 'tie-break' docs/specs/orca-mmu-perimeter-investigation.md` | T-P96-A0 deliverable (one-pager states the bisector tie-break rule) | FACT pass/fail |

## Step Completion Expectations

- Cross-step invariant: existing `boundary_paint_tdd.rs` and `arachne_perimeters_tdd.rs` regression tests in both perimeter modules MUST stay green at every step. The thin-wall and gap-fill code paths add new `walls` entries — they must not regress existing single-color extrusion shapes.
- Step ordering rationale: T-P96-A0 investigation first (Step 1) because its tie-break finding is consumed by T-P96-C0's host populator. IR additions (Step 2) before host populator (Step 3) because the populator writes the new field. Module-side spacing model (Step 4) before module-side wall_sequence (Step 5) because the sandwich reorder operates on the tree the spacing model builds. Thin-walls + gap-fill (Step 6) before MMU module consumption (Step 7) because both Step 6 and Step 7 modify the per-cell trace loop, and Step 7's bisector skip layer goes outermost; doing them in the opposite order requires re-touching Step 6's edits.
- Shared scratch state: none.

## Context Discipline Notes

- This packet has 8 implementation steps (7 source + 1 doc-impact landing) and ~19 tasks. Per-step file edit count is held to ≤ 3 throughout. The implementer must keep each step independently committable — do NOT batch two steps' edits into one commit even if "they're related".
- `crates/slicer-ir/src/slice_ir.rs` is ~1700 lines — range-read by `rg -n 'LoopType\|ExtrusionRole\|SlicedRegion\|CURRENT_SLICE_IR_SCHEMA_VERSION'` then ±40 lines.
- `crates/slicer-core/src/algos/paint_segmentation/` is a directory — `wc -l` each file before reading; the target file for bisector mask computation is `bisector_ownership.rs` (already owns `populate_external_contours`); range-read by `rg -n 'bisector|external_contour|populate'` in that directory.
- Both perimeter modules' `lib.rs` files post-P102/P103/P104 state will be ~600-800 LOC each. Range-read `run_perimeters` body and the per-cell wall-trace loop only. Loading the whole file each step is forbidden.
- Likely temptation read: the existing `arachne-perimeters/src/lib.rs` ray-cast logic. Skip — that logic was promoted to `slicer_core::geometry` in P103.
- Sub-agent return-format for the heaviest dispatch: OrcaSlicer `wall_sequence` SUMMARY (≤ 200 words) is the longest contract; the sandwich mode is structurally complex and the SUMMARY MUST describe the per-outer-contour grouping and the inset-index reordering rule without code. Re-dispatch if the return includes implementation pseudocode.
