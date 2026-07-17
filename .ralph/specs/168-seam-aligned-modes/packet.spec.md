---
status: implemented
packet: 168-seam-aligned-modes
task_ids:
  - TASK-274
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 168-seam-aligned-modes

## Goal

Port OrcaSlicer SeamPlacer's aligned seam path (per-layer candidate scoring with visibility/angle penalties, cross-layer seam-string chaining, least-squares B-spline smoothing) into the `seam-planner-default` prepass module, and add `aligned` / `aligned_back` as accepted `seam_mode` values that the per-layer `seam-placer` module consumes by snapping the planner's chained choice to real wall geometry.

## Scope Boundaries

The whole-object aligned machinery lives in `seam-planner-default` (`PrePass::SeamPlanning`, writes `SeamPlanIR`) because per-layer modules are dispatched in parallel with no cross-layer state; `seam-placer` gains the two new mode variants plus a snap-to-nearest-candidate consumption path for them. One WIT signature extension (`run-seam-planning` gains a `layer-plan` parameter, precedent `run-support-geometry`) is in scope; no `SeamPlanIR` schema change, no changes to perimeter modules, host scheduler policy, or G-code emit.

## Prerequisites and Blockers

- Depends on: none (plan queue row 3; parallel-safe with packets 166/167).
- Unblocks: `.ralph/specs/170-seam-livepath-audit` (same module; queue row 5 runs after this packet).
- Activation blockers: none known; `[FWD]` questions in `design.md` are implementer-resolvable.

## Acceptance Criteria

- **AC-1. Given** a `ConfigView` with `seam_mode = "aligned"` (and separately `"aligned_back"`), **when** `SeamPlacer::on_print_start` runs, **then** it returns `Ok` and `seam_mode()` returns exactly `"aligned"` (resp. `"aligned_back"`). | `cargo test -p seam-placer --test seam_aligned_mode_tdd -- aligned_mode_parses 2>&1 | tail -5`
- **AC-2. Given** the two module manifests, **when** grepped, **then** `[config.schema.seam_mode].values` in both `modules/core-modules/seam-placer/seam-placer.toml` and `modules/core-modules/seam-planner-default/seam-planner-default.toml` contains both `"aligned"` and `"aligned_back"` (default stays `"nearest"`). | `grep -q '"aligned"' modules/core-modules/seam-placer/seam-placer.toml && grep -q '"aligned_back"' modules/core-modules/seam-placer/seam-placer.toml && grep -q '"aligned"' modules/core-modules/seam-planner-default/seam-planner-default.toml && grep -q '"aligned_back"' modules/core-modules/seam-planner-default/seam-planner-default.toml && grep -q 'default = "nearest"' modules/core-modules/seam-planner-default/seam-planner-default.toml && echo PASS`
- **AC-3. Given** the canonical WIT source, **when** inspected, **then** `export run-seam-planning` in `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` takes a `layer-plan: layer-plan-view` parameter and the world-prepass world version carries a major bump (type change to an existing export per `docs/11` policy and DEV-084). | `awk '/export run-seam-planning/,/;/' crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit | grep -q 'layer-plan' && echo PASS`
- **AC-4. Given** a synthetic 20-layer square-prism mesh (10 mm side) with a matching layer-plan view (uniform 0.2 mm layers) and `seam_mode = "aligned"`, **when** `run_seam_planning` runs, **then** `SeamPlanningOutput` contains exactly one `SeamPlanEntry` per `(global_layer_index, object_id, region_id)` triple for all 20 layers, every entry has non-empty `scored_candidates`, and the chained+smoothed `chosen_position` values all lie within 0.5 mm XY of one single prism corner (max pairwise XY spread across layers <= 0.5 mm). | `cargo test -p seam-planner-default --test seam_aligned_planning_tdd -- aligned_chain_locks_single_corner 2>&1 | tail -5`
- **AC-5. Given** the same prism fixture with `seam_mode = "aligned_back"`, **when** `run_seam_planning` runs, **then** every layer's `chosen_position.y` is within 0.5 mm of the prism's maximum Y (rear-biased seeding selects a rear corner on every layer). | `cargo test -p seam-planner-default --test seam_aligned_planning_tdd -- aligned_back_prefers_rear_corner 2>&1 | tail -5`
- **AC-6. Given** `seam_mode = "aligned"` and a region whose injected `resolved_seam` point lies 0.3 mm away from the nearest wall-loop vertex (deliberately off-vertex, mimicking the planner's mesh-derived coordinates) with a non-empty `seam_candidates` list, **when** `run_wall_postprocess` runs, **then** the output `resolved_seam().point` equals the nearest seam-candidate position (not the raw injected point), and the rotated wall loop's `path.points[0]` equals that snapped vertex. | `cargo test -p seam-placer --test seam_aligned_mode_tdd -- aligned_snaps_to_nearest_candidate 2>&1 | tail -5`
- **AC-7. Given** the three net-new files ported from OrcaSlicer (`comparator.rs`, `visibility.rs`, `align.rs`; `contours.rs` is PNP-original and `lib.rs` pre-exists), **when** grepped, **then** each begins with the standard porting header of `docs/ORCASLICER_ATTRIBUTION.md` including the line `Original C++ source path:`. | `cd F:/slicerProject/pinch_n_print && for f in comparator.rs visibility.rs align.rs; do test -f "modules/core-modules/seam-planner-default/src/$f" && grep -q 'Original C++ source path' "modules/core-modules/seam-planner-default/src/$f" || exit 1; done && echo PASS`

## Negative Test Cases

- **AC-N1. Given** a `ConfigView` with `seam_mode = "diagonal"`, **when** `SeamPlacer::on_print_start` runs, **then** it returns `Err(ModuleError)` whose message contains exactly `unknown seam_mode: diagonal` (the rejection path is preserved after the enum extension). | `cargo test -p seam-placer --test seam_aligned_mode_tdd -- unknown_mode_still_rejected 2>&1 | tail -5`
- **AC-N2. Given** the pre-existing `nearest` / `rear` / `random` behavior, **when** the existing seam-placer suites run unmodified, **then** they all pass (no regression from the enum extension or the aligned consumption path). | `cargo test -p seam-placer 2>&1 | grep '^test result'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — 1870 lines; delegate a SUMMARY of the world-prepass section only (run-seam-planning signature, world version policy).
- `docs/11_operational_governance_and_acceptance_gate.md` — read the WIT version-bump policy rules range only (type change to an existing export = major bump; see DEV-084).
- `docs/08_coordinate_system.md` — direct read; porting checklist for Orca constant conversion.
- `docs/ORCASLICER_ATTRIBUTION.md` — direct read; exact header text.

## Doc Impact Statement (Required)

- `docs/03_wit_and_manifest.md` section "world-prepass" — update `run-seam-planning` signature and world version - `rg -q 'layer-plan' docs/03_wit_and_manifest.md`
- `docs/15_config_keys_reference.md` `seam_mode` entry — add `aligned` / `aligned_back` values - `rg -q 'aligned_back' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` — new row `D-168-SEAM-PREPASS-SOURCE` recording that PnP chains seam strings over prepass mesh-derived contours instead of OrcaSlicer's final perimeter polygons - `rg -q 'D-168-SEAM-PREPASS-SOURCE' docs/DEVIATION_LOG.md`
- `docs/adr/0046-aligned-seam-in-seam-planning-prepass.md` — new ADR for the prepass placement decision and the WIT parameter addition - `rg -q 'run-seam-planning' docs/adr/0046-aligned-seam-in-seam-planning-prepass.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — the aligned path: `align_seam_points`, `find_seam_string`, `find_next_seam_in_layer`, `pick_seam_point` / `pick_nearest_seam_point_index` / `pick_random_seam_point`, `compute_angle_penalty`, `raycast_visibility`, `calculate_candidates_visibility`, `calculate_overhangs_and_layer_embedding`, and the `SeamComparator` `spAligned` / `spAlignedBack` / `spRear` branches (including the `spAlignedBack` front/back visibility adjustment and `central_enforcer` handling).
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — `Perimeter` and `SeamCandidate` struct fields (visibility, overhang, embedded_distance, local_ccw_angle, central_enforcer) borrowed for the planner's internal candidate representation.
- `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp` — `fit_cubic_bspline` least-squares curve fit used by `align_seam_points`'s smoothing step (ported as a module-local helper).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
