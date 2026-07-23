# Requirements: 180-seam-final-placement-default

## Packet Metadata

- Grouped task IDs: `TASK-293`
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Packet 168's seam-placer snaps the planner's fitted point to the nearest existing wall vertex only; canonical OrcaSlicer projects onto the nearest point of the final perimeter, including segment interpolation. This means PNP's smoothed seam can jump to a different corner and lose continuity. Also, the current default is `nearest` while Orca's is `aligned`, and missing plans silently emit pristine walls instead of reporting degraded success.

## In Scope

- Continuous wall projection: project the planner's target onto the nearest point of the final wall loop geometry, inserting a new point into the segment when the target is not on a vertex, interpolating `feature_flags` and `width_profile` at the inserted point, and re-closing the loop.
- Degraded fallback via existing `ModuleError::non_fatal` channel: identify the missing `(layer, object, region_id, variant_chain)` key, apply canonical local candidate selection, preserve all walls, and continue the slice with degraded status.
- Default mode change: set `default = "aligned"` in both `seam-placer.toml` and `seam-planner-default.toml` manifests.
- End-to-end and regression tests: multi-region aligned default e2e test, continuous projection TDD, degraded fallback TDD, empty-wall-loop negative test, unknown-mode rejection test, and existing nearest/rear/random regression pass.

## Out of Scope

- WIT/IR identity changes, active-region key changes, or schema version bumps — packet 178 owns those.
- Canonical comparator, visibility, overhang, alternative-start retry, or B-spline solver changes — packet 179 owns those.
- Changes to OrcaSlicer source or direct final-perimeter generation.
- Host-native alignment policy or a second cross-layer state channel.

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated seam-first contract and stage I/O locations.
- `docs/02_ir_schemas.md` - delegated `PerimeterIR`, `WallLoop`, `SeamPosition`, and `SeamCandidate` locations.
- `docs/05_module_sdk.md` - delegated seam-candidate convention and wall-preservation behavior.
- `docs/15_config_keys_reference.md` - direct `seam_mode` config key and default values (lines 166-226).
- `docs/adr/0046-aligned-seam-in-seam-planning-prepass.md` - accepted prepass placement decision.
- `docs/DEVIATION_LOG.md` - `D-168-SEAM-PREPASS-SOURCE` predecessor deviation; this packet closes the source-geometry gap via continuous projection.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — canonical `place_seam` final perimeter placement and nearest-point projection behavior that PNP must match through continuous wall projection.
- `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.hpp` — canonical `SeamPosition` and `Perimeter` final placement fields.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (continuous projection), `AC-2` (degraded fallback), `AC-3` (default mode), `AC-4` (multi-region e2e), `AC-5` (no regression).
- Negative: `AC-N1` (empty wall loop), `AC-N2` (unknown mode rejection).
- **FORWARD-DEP**: `TASK-291` (`178-seam-region-aware-planning`) and `TASK-292` (`179-seam-canonical-algorithm-fidelity`) are both `status: implemented`. Unit tests for continuous projection and degraded fallback can be authored in isolation; the e2e test requires the full pipeline that 178 + 179 deliver.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p seam-placer --test seam_continuous_projection_tdd -- projects_onto_nearest_segment_point 2>&1 \| tee target/test-output.log \| grep '^test result'` | AC-1: continuous projection inserts point on segment | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p seam-placer --test seam_degraded_fallback_tdd -- missing_plan_emits_non_fatal_and_preserves_walls 2>&1 \| tee target/test-output.log \| grep '^test result'` | AC-2: missing plan non-fatal + walls preserved | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `grep -q 'default = "aligned"' modules/core-modules/seam-placer/seam-placer.toml && grep -q 'default = "aligned"' modules/core-modules/seam-planner-default/seam-planner-default.toml && grep -q '"aligned"' modules/core-modules/seam-placer/seam-placer.toml && grep -q '"aligned"' modules/core-modules/seam-planner-default/seam-planner-default.toml && echo PASS` | AC-3: default is aligned in both manifests | FACT pass/fail |
| `cargo test -p slicer-runtime --test e2e -- seam_aligned_default_e2e 2>&1 \| tee target/test-output.log \| grep '^test result'` | AC-4: multi-region e2e with aligned default | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p seam-placer 2>&1 \| tee target/test-output.log \| grep '^test result'` | AC-5: no regression in existing suites | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo xtask build-guests --check` | Guest artifact freshness | FACT pass/fail |

## Step Completion Expectations

Wall preservation is the invariant shared by every step. No step may drop or fail a region's walls, regardless of seam state, missing plan, or degenerate geometry. The continuous projection step must not change the cardinality or content of `feature_flags` or `width_profile.widths` relative to `path.points` after insertion. The degraded fallback step must not silently skip wall emission. The default change step must not break existing nearest/rear/random tests.

## Context Discipline Notes

Existing seam-placer tests and e2e fixtures must be read through bounded ranges or delegated. `docs/01_system_architecture.md` and `docs/02_ir_schemas.md` must be read through bounded ranges or delegated summaries only. The `OrcaSlicerDocumented/` directory must never be loaded directly.
