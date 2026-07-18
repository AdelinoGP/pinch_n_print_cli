---
status: implemented
packet: 108_perimeter-special-modes-and-seam
task_ids:
  - T-070
  - T-071
  - T-072
  - T-073
  - T-074b
  - T-074c
  - T-074d
  - T-077
  - T-080
  - T-081
  - T-082
  - T-083
  - T-P98-SEAM
  - T-090
  - T-091
  - T-092
backlog_source: docs/specs/perimeter-modules-orca-parity-roadmap.md
context_cost_estimate: M
---

# Packet Contract: 108_perimeter-special-modes-and-seam

## Goal

Land the Phase 7 wall-count overrides (`extra_perimeters` config bonus, narrow-island `smaller_perimeter` handling, `LoopType::NonPlanarShell` emission for regions in surface groups) and the Phase 8 seam-candidate quality work (sharp-corner threshold replacing every-vertex emission, painted `seam_enforcer`/`seam_blocker` consumption in candidate scoring + seam-placer selection).

## Scope Boundaries

Touches both perimeter modules' `lib.rs` + manifests, `slicer-core::perimeter_utils` (sharp-corner threshold + paint-seam consumption helpers), `seam-placer/src/lib.rs` (consume painted bias/exclusion), `docs/15_config_keys_reference.md`, and `docs/07_implementation_status.md` (D-98-SEAM-NO-CONSUMER supersession — the deviation is tracked there, not in `docs/DEVIATION_LOG.md`; this packet registers the closure entry in `docs/DEVIATION_LOG.md` under `D-108-SEAM-CONSUMED`). T-077 is now a real consumer of `region.overhang_areas()` — the upstream data flow lights up via P106 (overhang PrePass foundation) + P107 (overhang view-accessor + consumer refactor); this packet wires the perimeter-side consumption to actually add extra perimeters in overhang regions.

**FORWARD-DEP NOTE (S1 / fix-list item 7):** P104, P105, P106, and P107 are all `status: draft`. All data-flow items described as "available" from those packets (`SliceRegionView::surface_group()`, `SliceRegionView::overhang_areas()`, `OverhangRegion.xy_footprint`, `region.nonplanar_surface`, wall-sequence/spacing from P105) are forward-deps. None may be consumed by implementers until those packets reach `status: implemented`.

**DELETION SCOPE (T-090/T-091/T-092):** This packet also deletes the fake `arachne-perimeters` module (a 512-line iterative-inset approximation that is NOT real Arachne). Decision: the module is dropped outright; no renamed successor module will ship. P110 will create a fresh `arachne-perimeters/` skeleton for real Arachne later. Between P108 and P110 activation, `classic-perimeters` is the sole perimeter generator — by design.
- T-090 — delete `modules/core-modules/arachne-perimeters/` (directory + src/ + tests/ + manifest).
- T-091 — remove its workspace member entry from root `Cargo.toml`.
- T-092 — remove all doc/spec references to the fake `com.core.arachne-perimeters` / `arachne-perimeters` M1 module.

## Prerequisites and Blockers

- Depends on:
  - **P102** (foundations) — shared utils crate, multi-segment `MaterialBoundary`. `status: implemented` ✓
  - **P103** (polygon-op primitives) — used by perimeter_utils. `status: implemented` ✓
  - **P104** (propagation + surface rules) — `SliceRegionView::surface_group()` accessor (T-074b/c/d consume), `overhang_areas()` accessor (T-077 consumes). **`status: draft` — FORWARD-DEP.** `surface_group()` and `overhang_areas()` do NOT yet exist on `SliceRegionView` (verified: `crates/slicer-sdk/src/views.rs` has only `has_nonplanar()`). This packet's AC-3, AC-6, and AC-N1 are blocked until P104 lands.
  - **P105** (spacing + fill + MMU) — outer/inner widths, wall_sequence, ThinWall/GapFill emission (T-074d skips them for non-planar regions). **`status: draft` — FORWARD-DEP.** Spacing model, wall_sequence, and ThinWall/GapFill not yet available.
  - **P106** (overhang PrePass foundation) — populates `OverhangRegion.xy_footprint` at MeshAnalysis. **`status: draft` — FORWARD-DEP.** NOTE: the tree shows `xy_footprint: Vec<ExPolygon>` exists on `BridgeRegion` (line 581), NOT on `OverhangRegion`. The spec's claim that P106 populates `OverhangRegion.xy_footprint` must be reconciled with P106 before activation — the IR struct must be amended or the accessor renamed.
  - **P107** (overhang consumers + refactor) — confirms P104's `overhang_areas()` stub returns non-empty data. **`status: draft` — FORWARD-DEP.**
- Unblocks:
  - **P109 (M1 verification + closure)** — T-103 will close the deviations this packet registers (D-98-SEAM-NO-CONSUMER note in `docs/07_implementation_status.md` → superseded by `D-108-SEAM-CONSUMED` in `docs/DEVIATION_LOG.md`).
- Activation blockers: **P104, P105, P106, P107 must all be `status: implemented` before this packet activates.** The `xy_footprint` struct location (BridgeRegion vs OverhangRegion) must be reconciled with P106 before both specs are activated.

## Acceptance Criteria

- **AC-D1 (T-090). Given** the deletion of `modules/core-modules/arachne-perimeters/`, **when** deletion is complete, **then** (a) `! test -d modules/core-modules/arachne-perimeters/` returns true, (b) `rg 'arachne-perimeters' Cargo.toml` (root) returns zero hits, (c) `cargo build --workspace` passes. | `! test -d modules/core-modules/arachne-perimeters && ! rg -q 'arachne-perimeters' Cargo.toml && cargo build --workspace`
- **AC-D2 (T-092). Given** the stale-ref cleanup, **when** `rg` is run, **then** no doc or spec file references the fake `com.core.arachne-perimeters` or the old M1 `arachne-perimeters` module in a context implying it still exists as a live module. Historical/decision references (e.g., "P108 deleted the fake arachne-perimeters") are permitted. | `rg -rn 'com\.core\.arachne-perimeters' docs/ .ralph/specs/108_* 2>/dev/null | grep -viE 'deleted|p108|drop|removed|remove' | wc -l` returns 0. <!-- exclusion regex widened at packet close: original 'deleted\|P108\|drop\|removed' missed the packet's own present-tense "Remove" T-092 descriptions -->

- **AC-1. Given** a region with base `wall_count = 2` and config override `extra_perimeters = 2`, **when** `run_perimeters` runs, **then** `PerimeterRegion.walls` contains exactly 4 walls (`loop_number = wall_count + extra_perimeters - 1 = 3` zero-indexed, i.e. 4 walls). With `extra_perimeters = 0`, the count stays at 2. | `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-2. Given** a long narrow rectangular island (length 20 mm, width 0.6 mm) with `smaller_perimeter_threshold_mm = 0.8` and `smaller_perimeter_line_width = 0.3`, **when** `run_perimeters` runs, **then** the outer wall on that island uses `width = 0.3 mm` per vertex (not the default `outer_wall_line_width`), and a wider island in the same fixture uses the default width. | `cargo test -p slicer-runtime --test integration narrow_island_smaller_perimeter_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a `SlicedRegion` whose `nonplanar_surface` is `Some(SurfaceGroupId(7))` and the corresponding `SurfaceGroup.shell_count == 3`, **when** `run_perimeters` runs, **then** `PerimeterRegion.walls` contains exactly 3 walls all with `loop_type = LoopType::NonPlanarShell` (NOT `Outer` or `Inner`), `infill_areas` is empty, and no `ThinWall` or `GapFill` loops are emitted regardless of `detect_thin_wall` or `gap_infill_speed` config. | `cargo test -p slicer-runtime --test integration nonplanar_shell_emission_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** a square contour with 4 corners (90°) and an additional 30 redundant collinear points along each edge, **when** seam-candidate generation runs with `seam_candidate_angle_threshold_deg = 30.0`, **then** `PerimeterRegion.seam_candidates` contains exactly 4 entries (one per corner) — NOT 124 (every vertex) and NOT 0; corner positions match the 4 corner XYs within ±0.01 mm. | `cargo test -p slicer-core --test sharp_corner_seam_threshold_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** a `boundary_paint` region carrying `PaintSemantic::Custom("seam_enforcer")` over a flat (non-corner) wall segment AND a sharper corner candidate outside the enforced region, **when** `seam-placer` selects the seam, **then** the resolved seam falls inside the enforcer region (the enforcer's bias outweighs the sharper-corner geometric score). The helper `apply_seam_paint_bias` MUST match on the `Custom(s)` string — NOT on a `SeamEnforcer` named variant, which does not exist in `PaintSemantic`. Given a `PaintSemantic::Custom("seam_blocker")` region covering a wall corner, **then** that corner is **excluded** from `seam_candidates` entirely (filter, not deboost). | `cargo test -p slicer-runtime --test integration painted_seam_enforcer_blocker_tdd -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** an overhang-ramp fixture where `region.overhang_areas()` returns non-empty (P106's `xy_footprint` populated + P107's view-accessor lighting up P104's stub) and `extra_perimeters_on_overhangs = true`, **when** `run_perimeters` runs on a layer with overhang regions, **then** the wall_count inside `region.overhang_areas()` polygons is N+1 (one extra perimeter beyond the configured base `wall_count = N`), while wall_count outside those polygons stays at N. With `extra_perimeters_on_overhangs = false`, the wall count is N everywhere regardless of overhang membership. | `cargo test -p slicer-runtime --test integration extra_perimeters_on_overhangs_tdd -- --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** a non-planar region (`nonplanar_surface.is_some()`) and `detect_thin_wall = true`, **when** `run_perimeters` runs, **then** zero `WallLoop`s with `loop_type = LoopType::ThinWall` appear — the non-planar branch short-circuits thin-wall detection. | `cargo test -p slicer-runtime --test integration nonplanar_shell_emission_tdd nonplanar_skips_thin_wall_case -- --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. [SUPERSEDED by P109 — see D-109-SEAM-FATAL-CORRECTED in `docs/DEVIATION_LOG.md` and the Deviations entry below. The fatal-on-empty behavior this AC specified was reversed to graceful wall preservation; the test is now `blocker_exhausts_candidates_preserves_walls_no_seam` asserting Ok + walls preserved + no resolved seam.]** **Given** a fixture that passes the sharp-corner threshold but a `PaintSemantic::Custom("seam_blocker")` region covers the only sharp corner present, **when** seam-candidate generation runs, **then** `seam_candidates` is empty for that region (blocker excludes the corner; no fallback to next-sharpest because no other candidates exist), AND `seam-placer` returns `Err(ModuleError::fatal(…))` with a recognisable message (e.g. `"no seam candidates"`). **NOTE:** `SeamPlacerError::NoCandidates` does NOT exist in the tree — `modules/core-modules/seam-placer/src/lib.rs` returns `Result<(), ModuleError>` only. If the implementer wishes a typed error variant, they MUST define `SeamPlacerError` as net-new in that module and propagate it via `ModuleError::fatal`; otherwise assert via `ModuleError`. | `cargo test -p slicer-runtime --test integration painted_seam_enforcer_blocker_tdd blocker_exhausts_candidates_case -- --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd narrow_island_smaller_perimeter_tdd nonplanar_shell_emission_tdd painted_seam_enforcer_blocker_tdd extra_perimeters_on_overhangs_tdd && cargo test -p slicer-core --test sharp_corner_seam_threshold_tdd`

## Authoritative Docs

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — Phase 7 (T-070..T-077, including superseded T-074/T-075 + non-planar T-074b/c/d), Phase 8 (T-080..T-083), Inherited from P98 (T-P98-SEAM). Range-read those sub-tables.
- `docs/02_ir_schemas.md` — `LoopType::NonPlanarShell`, `SurfaceGroup`, `PaintSemantic` (delegate SUMMARY).
- `docs/05_module_sdk.md` — `SliceRegionView::surface_group()` accessor + `seam-placer`'s `seam_candidates()` consumer (delegate SUMMARY).
- `docs/15_config_keys_reference.md` — config-key registration format.
- `docs/07_implementation_status.md` — D-98-SEAM-NO-CONSUMER source (tracked here, not in DEVIATION_LOG.md); read for context.
- `docs/DEVIATION_LOG.md` — format reference; register net-new `D-108-SEAM-CONSUMED` at packet close.

## Doc Impact Statement (Required)

- `docs/15_config_keys_reference.md` — register `extra_perimeters` (int, default 0), `smaller_perimeter_line_width` (float, default 0.25), `smaller_perimeter_threshold_mm` (float, default 0.8), `narrow_loop_length_threshold_mm` (float, default 10.0), `seam_candidate_angle_threshold_deg` (float, default 30.0), `extra_perimeters_on_overhangs` (bool, default false) — `rg -q 'extra_perimeters' docs/15_config_keys_reference.md && rg -q 'smaller_perimeter_line_width' docs/15_config_keys_reference.md && rg -q 'seam_candidate_angle_threshold_deg' docs/15_config_keys_reference.md && rg -q 'extra_perimeters_on_overhangs' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` — register new closure entry `D-108-SEAM-CONSUMED` (supersedes the note in `docs/07_implementation_status.md` §"D-98-SEAM-NO-CONSUMER"; that ID lives only in the status doc, NOT in `DEVIATION_LOG.md`) — verify: `rg -q 'D-108-SEAM-CONSUMED' docs/DEVIATION_LOG.md`
- `docs/05_module_sdk.md` §"Seam-candidate generation" — document the sharp-corner threshold + paint-seam consumption convention — `rg -q 'seam_candidate_angle_threshold_deg' docs/05_module_sdk.md && rg -q 'seam_enforcer.*bias|seam_blocker.*exclude' docs/05_module_sdk.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1569` — `surface.extra_perimeters` bonus arithmetic (`loop_number = wall_loops + surface.extra_perimeters - 1`). Delegate a FACT.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1611-1628` — narrow-island `smaller_ext_perimeter_flow` handling. Delegate a SUMMARY ≤ 150 words.
- `OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp` non-planar shell branch (if present in source) OR `SkeletalTrapezoidation`'s shell_count consumer — Delegate a SUMMARY ≤ 100 words for the shell_count → wall_count override semantics.
- `OrcaSlicerDocumented/src/libslic3r/Feature/SeamPlacer/SeamPlacer.cpp` (or analogous) — sharp-corner candidate selection + painted seam consumption rules. Delegate a SUMMARY ≤ 200 words.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.

## Deviations

- [design.md §Code Change Surface, apply_seam_paint_bias] — Specified: `apply_seam_paint_bias(&mut Vec<SeamCandidate>, &PaintRegionLayerView)` in slicer-core | Implemented: data-driven signature taking enforcer/blocker polygon slices (perimeter_utils.rs:521); `PaintSemantic::Custom` string matching done module-side (classic-perimeters lib.rs:858-862) | Reason: slicer-core cannot depend on slicer-sdk, where PaintRegionLayerView lives.
- [design.md §Files in Scope] — Specified: slicer-sdk not in scope | Implemented: test-fixture setters `SliceRegionView::set_surface_group` (views.rs:260) and `set_overhang_areas` (views.rs:479) + builder wiring in test_support/fixtures.rs; #[doc(hidden)], not cfg-gated | Reason: no existing fixture could inject SurfaceGroup/overhang_areas required by AC-3/AC-N1/AC-6; mirrors existing set_sparse_infill_area precedent. Registered in D-108-SEAM-CONSUMED.
- [design.md §Files in Scope] — Specified: no slicer-runtime manifest changes | Implemented: `[dev-dependencies.seam-placer]` added to crates/slicer-runtime/Cargo.toml | Reason: painted_seam_enforcer_blocker_tdd exercises seam-placer directly; mirrors existing classic-perimeters dev-dep.
- [packet.spec.md AC-D2] — Specified: exclusion regex `grep -v 'deleted\|P108\|drop\|removed'` | Implemented: widened at close to `grep -viE 'deleted|p108|drop|removed|remove'` | Reason: original regex missed the packet's own present-tense "Remove" T-092 descriptions, making the literal command fail on satisfied reality.
- [packet.spec.md §Verification] — Specified: `cargo test -p slicer-runtime --test integration name1 name2 …` (multiple positional test names) | Implemented: names passed after `--` | Reason: cargo accepts only one positional TESTNAME; literal command is invalid syntax.
- **[AC-N2 — recorded retroactively 2026-07-02, packets-102–109 review]** — AC-N2's fatal-on-empty-candidates contract was REVERSED by P109: it directly contradicted the pre-existing HIGH-2 wall-preservation contract (`seam_placer_tdd::no_candidates_no_seam`, `region_without_seam_candidates_or_resolved_seam_preserves_walls`) — two tests with identical inputs asserting opposite outcomes. The fatal block and module-local `SeamPlacerError` were removed; a seam-info-less region now emits its walls pristine with no resolved seam, and the blocker test was renamed to `blocker_exhausts_candidates_preserves_walls_no_seam`. See D-109-SEAM-FATAL-CORRECTED in `docs/DEVIATION_LOG.md`. This spec was not amended at the time of the reversal; AC-N2 is annotated SUPERSEDED above.
- **[§Verification — recorded retroactively 2026-07-02, packets-102–109 review]** — The closure gate was structurally blind to the executor bucket: §Verification ran only the named integration tests plus one slicer-core test, so the 13 `cube_4color` executor tests broken by this packet's fatal-on-empty-seam path went undetected at close and were only caught (and fixed) by the follow-up commit `454964a7`. Lesson recorded for future packets: any packet touching seam/perimeter emission paths must include the executor bucket (`cargo test -p slicer-runtime --test executor`) in its closure commands.
