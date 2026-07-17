# Requirements: 168-seam-aligned-modes

## Packet Metadata

- Grouped task IDs: `TASK-274` (new; minted by this packet's `task-map.md` into the `docs/07` crosswalk)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer's default `seam_position` is `spAligned`, but PNP's `seam_mode` accepts only `nearest` / `rear` / `random` (`SeamMode` enum, `modules/core-modules/seam-placer/src/lib.rs:31-39`; config read at `lib.rs:185-196`, with `_ => SeamMode::Nearest` fallback). The fork has no PNP value to map Orca's default `spAligned` onto: omitting the key silently falls back to `nearest` (`_ => SeamMode::Nearest`, `lib.rs:195`), while passing `"aligned"` today fails the slice fatally (`ModuleError::fatal(1, "unknown seam_mode: ...")`, `lib.rs:192`). Either way every default-settings slice ends up on `nearest` — a per-slice visible quality regression with no user-facing signal (fork ships no gap warnings). The approved plan (`docs/specs/fork-gaps-wave1-plan.md`, Packet 16) decided a **full Orca-parity port** of canonical `SeamPlacer`'s aligned path over a simple per-object-anchor accumulator; `aligned_back` reuses the same machinery with rear-biased seeding.

Architecture constraint discovered during grounding: per-layer modules are re-instantiated per call and layers execute in parallel (`crates/slicer-runtime/src/layer_executor.rs:200-215`, `crates/slicer-wasm-host/src/dispatch.rs:315-386`), so cross-layer seam-string chaining is impossible inside `seam-placer`. The whole-object channel already exists: `seam-planner-default` (`PrePass::SeamPlanning`, TASK-159) writes `SeamPlanIR` to the blackboard, and the host injects each entry's `chosen_candidate` into the matching region's `resolved_seam` before `seam-placer` runs (`crates/slicer-wasm-host/src/dispatch.rs:1394-1410`; commit-time backfill `crates/slicer-runtime/src/layer_executor.rs:1830-1847`, ADR-0020). The aligned machinery therefore lands in the prepass module; `seam-placer` consumes it.

## In Scope

- Extend `SeamMode` (`modules/core-modules/seam-placer/src/lib.rs`) with `Aligned` and `AlignedBack`; accept config strings `"aligned"` / `"aligned_back"`; keep the unknown-string rejection (`ModuleError::fatal(1, "unknown seam_mode: ...")`).
- Extend `[config.schema.seam_mode].values` in both `seam-placer.toml` and `seam-planner-default.toml` with `"aligned"`, `"aligned_back"`; default remains `"nearest"`.
- WIT change: add `layer-plan: layer-plan-view` parameter to `export run-seam-planning` in `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` (precedent: `run-support-geometry` in the same world), with the matching `slicer-sdk` `PrepassModule::run_seam_planning` trait change, `slicer-macros` guest-shim update, and `crates/slicer-wasm-host/src/dispatch.rs` prepass-arm update. Major world-version bump per `docs/11` policy (DEV-084 precedent: adding a required parameter is a type change).
- Port into `seam-planner-default` (new module-local source files with attribution headers): per-layer candidate construction from mesh-derived layer contours; `compute_angle_penalty`; visibility scoring ported from `raycast_visibility` / `calculate_candidates_visibility`; overhang/embedding penalties from `calculate_overhangs_and_layer_embedding`; `SeamComparator` with `spAligned` / `spAlignedBack` / `spRear` branch behavior; `pick_seam_point` / `pick_nearest_seam_point_index` / `pick_random_seam_point`; `find_next_seam_in_layer`; `find_seam_string`; `align_seam_points` driver with the least-squares cubic B-spline smoothing (`fit_cubic_bspline` port).
- Replace `run_seam_planning`'s MVP layer enumeration (hardcoded `layer_height = 0.2`, `clamp(1, 100)` at `modules/core-modules/seam-planner-default/src/lib.rs:216-226`) with real layer indices/z from the new layer-plan parameter.
- `seam-placer` consumption path for `Aligned` / `AlignedBack`: prefer the host-injected `resolved_seam` (the planner's chained choice) over local candidate re-selection, snapping it to the nearest `seam_candidates()` position (fallback: nearest wall-loop vertex when the candidate list is empty) before rotation. This also closes the known exact-match gap noted at `lib.rs:210-214` for the aligned modes.
- Tests: new `seam_aligned_mode_tdd.rs` (seam-placer) and `seam_aligned_planning_tdd.rs` (seam-planner-default) fixtures per the ACs; keep all existing suites green.
- Docs: `docs/03_wit_and_manifest.md` signature update, `docs/15_config_keys_reference.md` value list, `docs/DEVIATION_LOG.md` row `D-168-SEAM-PREPASS-SOURCE`, new `docs/adr/0046-aligned-seam-in-seam-planning-prepass.md`.
- Guest WASM rebuild for all affected guests (WIT + SDK + macros changes invalidate every guest).

## Out of Scope

- Changing the shipped default `seam_mode` away from `"nearest"` (fork supplies its config explicitly; a default flip is a separate policy decision).
- Any `SeamPlanIR` / `SeamPlanEntry` schema change — the existing `region_key` + `chosen_candidate` + `scored_candidates` shape suffices (`crates/slicer-ir/src/slice_ir.rs:1066-1086`).
- Perimeter modules (`arachne-perimeters`, `classic-perimeters`) and their `generate_sharp_corner_seam_candidates` producer path.
- Seam painting / enforcer-blocker paint semantics beyond preserving the existing `central_enforcer`-free behavior (PNP paint seam regions are handled upstream; comparator enforcer branches are ported but fed no enforcers).
- `nearest` / `rear` / `random` behavior changes in either module.
- Packet 170's sibling-wall audit (queue row 5).

## Authoritative Docs

- `docs/03_wit_and_manifest.md` — 1870 lines; delegate SUMMARY of the world-prepass section (signature + version policy anchors).
- `docs/11_operational_governance_and_acceptance_gate.md` — read only the WIT version policy rules range.
- `docs/08_coordinate_system.md` — direct read (porting checklist).
- `docs/ORCASLICER_ATTRIBUTION.md` — direct read (header text).
- `docs/02_ir_schemas.md` — 1811 lines; delegate a FACT check that `SeamPlanIR` docs need no edit (schema unchanged).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — the aligned path: `align_seam_points`, `find_seam_string`, `find_next_seam_in_layer`, `pick_seam_point` / `pick_nearest_seam_point_index` / `pick_random_seam_point`, `compute_angle_penalty`, `raycast_visibility`, `calculate_candidates_visibility`, `calculate_overhangs_and_layer_embedding`, and the `SeamComparator` `spAligned` / `spAlignedBack` / `spRear` branches (including the `spAlignedBack` front/back visibility adjustment and `central_enforcer` handling).
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — `Perimeter` and `SeamCandidate` struct fields (visibility, overhang, embedded_distance, local_ccw_angle, central_enforcer) borrowed for the planner's internal candidate representation.
- `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp` — `fit_cubic_bspline` least-squares curve fit used by `align_seam_points`'s smoothing step (ported as a module-local helper).

## Acceptance Summary

- Positive: `AC-1` through `AC-7`. Refinement on AC-4/AC-5: the prism fixture must construct the mesh via triangle/vertex arrays fed to the SDK prepass test builders (not host slicing), and the layer-plan view must carry 20 entries with z = (i+1) * 0.2 mm so entry `global_layer_index` values are asserted exactly 0..=19.
- Negative: `AC-N1`, `AC-N2`.
- Cross-packet impact: packet 170 re-tests `seam-placer` after this packet's edits; packets 166/167 are disjoint crates. The WIT parameter addition rebuilds every guest — any concurrently active packet touching guests must re-run its own freshness check after this packet merges.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only the gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p seam-placer --test seam_aligned_mode_tdd 2>&1 \| grep '^test result'` | AC-1, AC-6, AC-N1 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p seam-placer 2>&1 \| grep '^test result'` | AC-N2 whole-module regression | FACT pass/fail |
| `cargo test -p seam-planner-default --test seam_aligned_planning_tdd 2>&1 \| grep '^test result'` | AC-4, AC-5 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p seam-planner-default 2>&1 \| grep '^test result'` | planner regression (existing `seam_planner_tdd.rs`) | FACT pass/fail |
| AC-2 grep chain (see `packet.spec.md`) | manifest enum values | FACT PASS/absent |
| AC-3 awk/grep (see `packet.spec.md`) | WIT signature | FACT PASS/absent |
| AC-7 grep (see `packet.spec.md`) | attribution headers | FACT PASS/absent |
| `cargo xtask build-guests --check` | guest freshness after WIT/SDK/module edits | FACT clean/STALE list |
| `cargo test -p slicer-runtime --test contract 2>&1 \| grep '^test result'` | WIT drift / dispatch contract suites after signature change | FACT pass/fail |
| `cargo check --workspace --all-targets` | compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| Doc greps from the Doc Impact Statement | doc edits landed | FACT PASS/absent |

## Step Completion Expectations

- The WIT/SDK/dispatch signature change (Step 1) must land and compile before any planner-port step, because every later planner step's tests construct the new `run_seam_planning` signature.
- `cargo xtask build-guests` (not just `--check`) must run after Step 1 and again at packet close: the WIT and SDK edits invalidate every guest, and stale guests will fail unrelated-looking host-integration tests.
- The seam-placer consumption step must not begin until the planner emits aligned entries, so its fixture mirrors real injected coordinates.

## Context Discipline Notes

- `OrcaSlicerDocumented/` reads are delegation-only (see snippet above); `SeamPlacer.cpp` is thousands of lines — never open it directly.
- `crates/slicer-wasm-host/src/dispatch.rs` and `crates/slicer-runtime/src/layer_executor.rs` are large; open only the ranges named in `design.md`.
- `docs/03_wit_and_manifest.md` (1870 lines) and `docs/02_ir_schemas.md` (1811 lines): delegate; never full-read.
