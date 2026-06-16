# Requirements: 106_perimeter-special-modes-and-seam

## Packet Metadata

- Grouped task IDs:
  - `T-070` — Register `extra_perimeters` config key
  - `T-071` — Honour `extra_perimeters` config bonus
  - `T-072` — Register `smaller_perimeter_line_width`, `smaller_perimeter_threshold_mm`, `narrow_loop_length_threshold_mm`
  - `T-073` — Narrow-island handling with `smaller_ext_perimeter_flow`
  - `T-074b` — Detect non-planar regions via `nonplanar_surface.is_some()`; emit `LoopType::NonPlanarShell`
  - `T-074c` — Read `SurfaceGroup.shell_count`; override `wall_count` for non-planar regions
  - `T-074d` — Skip thin-wall, gap-fill, `infill_areas` for non-planar regions
  - `T-077` — Register `extra_perimeters_on_overhangs`; implement consumer (ships as no-op + deviation per current preconditions)
  - `T-080` — Replace every-vertex seam-candidate heuristic with sharp-corner threshold
  - `T-081` — Register `seam_candidate_angle_threshold_deg`
  - `T-082` — Audit seam-placer for dependency on dense candidate lists
  - `T-083` — Confirm/document interaction with seam-planner-default
  - `T-P98-SEAM` — Consume painted `seam_enforcer`/`seam_blocker` in seam-candidate generation + seam-placer selection
- Backlog source: `docs/specs/perimeter-modules-orca-parity-roadmap.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

After P105 lands the wall-emission geometry stack, three wall-count override mechanisms and the seam-candidate quality work remain. The override mechanisms are:

1. **`extra_perimeters` per-region config**: a normal per-region bonus that adds N walls beyond the configured base (`loop_number = wall_count + extra_perimeters - 1`). Currently the perimeter modules don't read this config; setting it has no effect.
2. **Narrow-island width handling**: long-narrow islands below a length threshold use a smaller extrusion width (`smaller_perimeter_line_width`) so the wall actually fits. Without this, narrow islands are skipped entirely when the wall_inset can't fit two full-width walls.
3. **Non-planar wall emission** (per D-11 closure in the roadmap): regions whose `nonplanar_surface` is set are part of a swept surface group; the perimeter module must emit `LoopType::NonPlanarShell` walls instead of `Outer`/`Inner`, honour `SurfaceGroup.shell_count` as the override for `wall_count`, and skip thin-wall/gap-fill/infill (because the surface group sweep is the only geometry).

The seam quality work has two halves:

1. **Sharp-corner threshold (T-080..T-083)**: current modules push **every wall vertex** as a seam candidate. For a 100-vertex polygon, that's 100 candidates per layer-region. Seam-placer's scoring runs over all of them. Replacing with an angle-threshold (only corners with turn-angle ≥ ~30°) reduces candidates ~25× on typical shapes.
2. **Painted seam consumption (T-P98-SEAM, inherited)**: P98 decoded `paint_seam` sub-facet strokes into `SeamEnforcer`/`SeamBlocker` semantics in `boundary_paint`, but no live module reads them (`D-98-SEAM-NO-CONSUMER`). This packet wires the consumer: enforcer regions bias seam-candidate selection toward enclosed vertices; blocker regions exclude enclosed vertices.

T-077 (`extra_perimeters_on_overhangs`) is included in scope but ships as a deferred no-op + registered deviation because its data-flow preconditions (P104 implementation + sibling roadmap Phase 3) are unmet. The config key is registered and the consumer code path is wired against `region.overhang_areas()`; when the accessor returns non-empty, T-077 starts working without further code change.

## In Scope

- Both perimeter modules' `lib.rs` (Phase 7 consumers): `extra_perimeters` bonus consumption, narrow-island detection + smaller-width emission, non-planar branch (`LoopType::NonPlanarShell` emission with `shell_count` from `SurfaceGroup`).
- Both perimeter modules' `lib.rs` (T-077 deferred consumer): read `region.overhang_areas()`; when non-empty, add extra perimeters within those areas. With current empty-accessor preconditions, this code path produces zero extras.
- `crates/slicer-helpers/src/perimeter_utils.rs`:
  - Extend `generate_seam_candidates` with `angle_threshold_deg: f32` parameter; emit only corners exceeding the threshold; rename to `generate_sharp_corner_seam_candidates` or version-2 alongside the existing (which both modules then call with the new threshold).
  - Add `apply_seam_paint_bias(&mut Vec<SeamCandidate>, &PaintRegionLayerView)` helper that biases enforcer-enclosed candidates and removes blocker-enclosed candidates.
- `modules/core-modules/seam-placer/src/lib.rs`: confirm candidate-list-density assumptions are robust to sparser input (T-082 audit); document or fix; call `apply_seam_paint_bias` before scoring.
- Both perimeter manifests + `docs/15_config_keys_reference.md`: register 6 new config keys (`extra_perimeters`, `smaller_perimeter_line_width`, `smaller_perimeter_threshold_mm`, `narrow_loop_length_threshold_mm`, `seam_candidate_angle_threshold_deg`, `extra_perimeters_on_overhangs`).
- `docs/05_module_sdk.md` §"Seam-candidate generation" — document the new convention.
- `docs/DEVIATION_LOG.md` — supersede `D-98-SEAM-NO-CONSUMER`; register `D-<packet>-OVERHANG-EXTRA-PERIMETERS-DEFERRED`.
- 6 new TDD files covering ACs.

## Out of Scope

- Spacing model / outer-inner widths / wall_sequence / thin-walls / gap-fill / MMU bisector mask — all P105.
- Per-vertex `is_bridge`, `overhang_quartile`, inner-wall material boundary — P104.
- Shared utils crate creation, IR widening — P102.
- Polygon-op primitives — P103.
- M1 verification harness, fixture recording, M1 close ceremony — P107.
- Real Arachne (Voronoi + SkeletalTrapezoidation + BeadingStrategy stack) — M2.
- The overhang-pipeline-restructuring sibling roadmap implementation that T-077 ultimately depends on — separate workstream.
- Rename of `arachne-perimeters` → `variable-width-perimeters` — separate workstream.

## Authoritative Docs

| Doc | Size | Read strategy |
| --- | --- | --- |
| `docs/specs/perimeter-modules-orca-parity-roadmap.md` | ~700 lines | Range-read Phase 7 + Phase 8 sub-tables + "Inherited from P98" section. |
| `docs/02_ir_schemas.md` | ~900 lines | Delegate SUMMARY for `LoopType::NonPlanarShell`, `SurfaceGroup`, `PaintSemantic`, `SeamCandidate`. |
| `docs/05_module_sdk.md` | ~500 lines | Delegate SUMMARY for `SliceRegionView::surface_group()`, `PaintRegionLayerView::get_regions`, `seam-placer` consumer contract. |
| `docs/15_config_keys_reference.md` | ~300 lines | Range-read §"Walls" + §"Seam". |
| `docs/DEVIATION_LOG.md` | varies | Range-read `D-98-SEAM-NO-CONSUMER` entry. |

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1569` — `surface.extra_perimeters` bonus formula. FACT.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1611-1628` — narrow-island `smaller_ext_perimeter_flow`. SUMMARY ≤ 150 words.
- `OrcaSlicerDocumented/src/libslic3r/Feature/SeamPlacer/SeamPlacer.cpp` — sharp-corner candidate selection + painted seam consumption. SUMMARY ≤ 200 words.

## Acceptance Summary

- Positive cases: `AC-1` (extra_perimeters bonus), `AC-2` (narrow-island smaller_perimeter), `AC-3` (non-planar shell emission with shell_count), `AC-4` (sharp-corner threshold reduces candidate count), `AC-5` (painted enforcer biases, painter blocker excludes), `AC-6` (T-077 deferred no-op + deviation logged).
- Negative cases: `AC-N1` (non-planar skips thin-wall even when config enabled), `AC-N2` (blocker exhausts candidates → graceful error).
- Refinements not captured in Given/When/Then:
  - Sharp-corner threshold is signed turn angle (concave + convex both count toward threshold — both are sharp in absolute value). Concave corners get a slight score bonus (visibility hides better) per the existing `generate_seam_candidates` convention.
  - `apply_seam_paint_bias` enforcer bias factor: multiply the candidate's score by `seam_enforcer_bias_factor` (default 0.1, lower = more preferred). Blocker exclusion: remove candidate entirely (do not just deboost).
- Cross-packet impact: depends on P102 + P104 + P105. Unblocks P107.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | Cross-crate compile | FACT pass/fail; SNIPPETS ≤ 20 lines on fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clippy gate | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd` | AC-1 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration narrow_island_smaller_perimeter_tdd` | AC-2 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration nonplanar_shell_emission_tdd` | AC-3 + AC-N1 | FACT pass/fail per case |
| `cargo test -p slicer-helpers --test sharp_corner_seam_threshold_tdd` | AC-4 | FACT pass/fail |
| `cargo test -p slicer-runtime --test integration painted_seam_enforcer_blocker_tdd` | AC-5 + AC-N2 | FACT pass/fail per case |
| `cargo test -p slicer-runtime --test integration extra_perimeters_on_overhangs_deferred_tdd` | AC-6 (no-op verification) | FACT pass/fail |
| `rg -q 'D-.*-SEAM-CONSUMED' docs/DEVIATION_LOG.md` | T-P98-SEAM deviation supersession | FACT pass/fail |
| `rg -q 'D-.*-OVERHANG-EXTRA-PERIMETERS-DEFERRED' docs/DEVIATION_LOG.md` | T-077 deviation registration | FACT pass/fail |
| `cargo xtask build-guests --check` | Guest WASM coherence | FACT clean / STALE list |

## Step Completion Expectations

- Cross-step invariant: existing `boundary_paint_tdd.rs`, `arachne_perimeters_tdd.rs`, `classic_perimeters_tdd.rs`, and `mmu_bisector_dedup_tdd.rs` (from P105) regression tests MUST stay green at every step. The new code paths add wall-count overrides and seam-candidate filtering — they must not regress existing single-color planar wall shapes.
- Step ordering rationale: wall-count overrides (Step 1-3) before seam work (Step 4-5) because the overrides change `walls` content, which seam-candidate generation reads. T-077 deferred consumer (Step 6) lands last because it depends on all prior wall-emission logic and only adds a no-op-by-default code path.
- Shared scratch state: none.

## Context Discipline Notes

- Both perimeter modules' `lib.rs` post-P105 will be ~800-1000 LOC each. Range-read `run_perimeters` only — do NOT load the whole file each step.
- `modules/core-modules/seam-placer/src/lib.rs` is smaller (≤ 300 LOC). Read full for the T-082 audit; edit narrowly.
- `crates/slicer-helpers/src/perimeter_utils.rs` post-P105 will carry `wall_sequence_reorder` + the new seam helpers; range-read by `rg -n 'fn generate_seam_candidates'` before editing.
- Likely temptation read: `seam-planner-default/src/lib.rs` for the T-083 documentation. **Skip** — T-083's deliverable is a one-paragraph documentation note in `docs/05_module_sdk.md` based on what seam-planner-default's manifest declares; reading its source is not required.
- Sub-agent return-format for the heaviest dispatch: SeamPlacer SUMMARY (≤ 200 words) — must describe the candidate-scoring convention and where painted-seam consumption fits in the priority order. Re-dispatch if return includes implementation pseudocode.
