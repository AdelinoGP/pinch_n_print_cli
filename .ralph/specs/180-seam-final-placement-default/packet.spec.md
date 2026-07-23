---
status: draft
packet: 180-seam-final-placement-default
task_ids:
  - TASK-293
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 180-seam-final-placement-default

## Goal

Project canonical aligned seam targets onto continuous final wall geometry, preserve wall-loop feature flags and width profiles through rotation and point insertion, report degraded fallback via non-fatal module errors when no plan reaches a region, and make `aligned` the default `seam_mode` matching OrcaSlicer.

## Scope Boundaries

This packet consumes packets 178 and 179's variant-aware canonical seam target and implements the final wall projection, degraded fallback diagnostics, and default-mode change. It does not change the prepass WIT input, the active-region identity contract, or the canonical scoring/visibility/spline algorithm. It modifies `seam-placer` wall mutation, both module manifests' default config, and the SDK view plumbing for continuous projection.

## Prerequisites and Blockers

- Depends on: `TASK-291` (packet 178) and `TASK-292` (packet 179).
- Unblocks: none (terminal packet in this batch).
- Activation blockers: none known; packet remains draft until preflight and guest freshness gates pass.

## Acceptance Criteria

- **AC-1. Given** an aligned seam target from `SeamPlanIR` that does not coincide with any existing wall-loop vertex, **when** `run_wall_postprocess` projects it onto the nearest point of the final wall geometry, **then** the emitted wall loop's `path.points[0]` is the projected point (possibly inserted by splitting the nearest segment), `feature_flags` and `width_profile.widths` remain parallel to `path.points` with the inserted flag/width interpolated, and the loop is still closed. | `cargo test -p seam-placer --test seam_continuous_projection_tdd -- projects_onto_nearest_segment_point 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-2. Given** an active region with no matching `SeamPlanIR` entry in aligned mode, **when** `run_wall_postprocess` runs, **then** the module emits a non-fatal `ModuleError` (with `fatal: false`) carrying a message identifying the `(layer, object, region_id, variant_chain)` key, applies canonical local candidate selection as a degraded fallback, preserves all wall loops, and the slice continues with degraded status. | `cargo test -p seam-placer --test seam_degraded_fallback_tdd -- missing_plan_emits_non_fatal_and_preserves_walls 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-3. Given** both module manifests (`seam-placer.toml` and `seam-planner-default.toml`), **when** grepped, **then** `[config.schema.seam_mode].default` is `"aligned"` in both files and `"aligned"` remains in the `values` list. | `grep -q 'default = "aligned"' modules/core-modules/seam-placer/seam-placer.toml && grep -q 'default = "aligned"' modules/core-modules/seam-planner-default/seam-planner-default.toml && grep -q '"aligned"' modules/core-modules/seam-placer/seam-placer.toml && grep -q '"aligned"' modules/core-modules/seam-planner-default/seam-planner-default.toml && echo PASS`
- **AC-4. Given** a multi-region end-to-end slice with `seam_mode = "aligned"` (the new default), **when** the full pipeline runs, **then** every active region's final wall loop starts at a seam point within 0.05 mm XY of the planner's projected target, every region's walls are preserved, and no region silently emits pristine unrotated walls in aligned mode. | `cargo test -p slicer-runtime --test e2e -- seam_aligned_default_e2e 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-5. Given** a `nearearest`/`rear`/`random` mode slice, **when** the existing seam-placer suites run, **then** they all pass without regression from the continuous projection or default-mode changes. | `cargo test -p seam-placer 2>&1 | tee target/test-output.log | grep '^test result'`

## Negative Test Cases

- **AC-N1. Given** a wall loop with no points (degenerate), **when** continuous projection is attempted, **then** the module emits a non-fatal error and preserves the empty loop without inserting a phantom point or panicking. | `cargo test -p seam-placer --test seam_continuous_projection_tdd -- empty_wall_loop_is_non_fatal 2>&1 | tee target/test-output.log | grep '^test result'`
- **AC-N2. Given** an unknown `seam_mode` value, **when** `on_print_start` runs, **then** it returns `Err(ModuleError)` whose message contains exactly `unknown seam_mode: <value>` (the rejection path is preserved after the default change). | `cargo test -p seam-placer --test seam_aligned_mode_tdd -- unknown_mode_still_rejected 2>&1 | tee target/test-output.log | grep '^test result'`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask build-guests --check`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated document-map for normative architecture docs.
- `docs/01_system_architecture.md` - delegated seam-first contract and stage I/O locations.
- `docs/02_ir_schemas.md` - delegated `PerimeterIR`, `WallLoop`, `SeamPosition`, and `SeamCandidate` locations.
- `docs/03_wit_and_manifest.md` - delegated `perimeter-region-view` and claim contract locations.
- `docs/05_module_sdk.md` - delegated seam-candidate convention and wall-preservation behavior.
- `docs/15_config_keys_reference.md` - direct `seam_mode` config key and default values.
- `docs/adr/0046-aligned-seam-in-seam-planning-prepass.md` - accepted prepass placement decision.
- `docs/DEVIATION_LOG.md` - `D-168-SEAM-PREPASS-SOURCE` predecessor deviation; this packet closes the source-geometry gap via continuous projection.

## Doc Impact Statement (Required)

- `docs/01_system_architecture.md` seam-first contract and final projection - `rg -q 'continuous.*projection|seam-first.*aligned' docs/01_system_architecture.md`
- `docs/05_module_sdk.md` seam-candidate convention and degraded fallback - `rg -q 'degraded.*seam|non-fatal.*seam|continuous.*projection' docs/05_module_sdk.md`
- `docs/15_config_keys_reference.md` `seam_mode` default change - `rg -q 'default.*aligned|seam_mode.*aligned' docs/15_config_keys_reference.md`
- `docs/DEVIATION_LOG.md` closure of `D-168-SEAM-PREPASS-SOURCE` source-geometry gap - `rg -q 'D-168-SEAM-PREPASS-SOURCE' docs/DEVIATION_LOG.md`
- `docs/DEVIATION_LOG.md` new row `D-283-ADR-0046-AMENDED` amending ADR-0046 default from `nearest` to `aligned` - `rg -q 'D-283-ADR-0046-AMENDED' docs/DEVIATION_LOG.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — canonical `place_seam` final perimeter placement and nearest-point projection behavior that PNP must match through continuous wall projection.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — canonical `SeamPosition` and `Perimeter` final placement fields.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
