# Design: 180-seam-final-placement-default

## Controlling Code Paths

- Primary code path: `SeamPlacer::run_wall_postprocess` -> `aligned_seam_target` -> `find_seam_location` -> `rotate_wall_loop` -> `push_reordered_wall_loop` in `modules/core-modules/seam-placer/src/lib.rs`; host injection via `push_perimeter_regions` and `backfill_resolved_seam` in `crates/slicer-wasm-host/src/dispatch.rs` and `crates/slicer-runtime/src/layer_executor.rs`.
- Neighboring tests/fixtures: existing `seam_placer_tdd.rs`, `seam_aligned_mode_tdd.rs`, and e2e fixtures under `crates/slicer-runtime/tests/e2e/`. New test files: `seam_continuous_projection_tdd.rs`, `seam_degraded_fallback_tdd.rs`, `seam_aligned_default_e2e.rs`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- Wall-preservation invariant: every region's walls must reach the output regardless of seam state. No step may drop, skip, or fail to emit a region's wall loop.
- `feature_flags` and `width_profile.widths` must stay parallel to `path.points` after point insertion. The inserted point's flag and width must be interpolated (linear, nearest-neighbor, or canonical-specific) such that the parallel invariant is maintained.
- `ModuleError::non_fatal` is the existing channel for degraded reporting. It is defined in `crates/slicer-sdk/src/error.rs` and surfaced through progress events in `crates/slicer-runtime/src/progress_events.rs`. The `fatal: false` field is carried through WIT `module-error`.
- The default change must not break existing nearest/rear/random tests. Those modes keep their existing vertex-based selection and are unaffected by continuous projection.
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Caveat for this packet: `SeamPosition.point` is f32 millimetres; wall loop points are f32 millimetres. The continuous projection operates in mm space. The 0.05 mm final seam tolerance in AC-4 is already in mm and passes through unchanged.

## Code Change Surface

- Selected approach: continuous projection — project the planner's target onto the nearest point of the final wall loop geometry, inserting a new point into the segment if needed, interpolating `feature_flags` and `width_profile`, and re-closing the loop. Replace the current vertex-only snap in `aligned_seam_target` with this continuous projection. Add degraded fallback: when no `SeamPlanIR` entry matches, emit `ModuleError::non_fatal` with the missing key, apply canonical local candidate selection, preserve walls. Change `default = "nearest"` to `default = "aligned"` in both module TOML manifests.
- Exact functions, traits, manifests, tests, and fixtures:
  - `modules/core-modules/seam-placer/src/lib.rs`: modify `aligned_seam_target` to perform continuous projection instead of vertex-only snap; add `project_onto_wall_segment` helper; add degraded fallback path in `run_wall_postprocess` when no plan entry matches.
  - `modules/core-modules/seam-placer/seam-placer.toml`: change `seam_mode` default to `"aligned"`.
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml`: change `seam_mode` default to `"aligned"`.
  - New test files: `seam_continuous_projection_tdd.rs`, `seam_degraded_fallback_tdd.rs`.
  - New e2e test: `seam_aligned_default_e2e.rs` (or hosted in an existing e2e file).
- Rejected alternatives and reasons:
  - Keep vertex-only snap: loses continuity; seam jumps to a different corner when the planner's target is on a segment interior.
  - Keep `nearest` as default: deviates from OrcaSlicer's `aligned` default; users must opt in to get canonical behavior.
  - Silent pristine walls on missing plan: hides degraded state from the user; non-fatal error is the canonical diagnostic channel.
  - Separate projection module: unnecessary indirection; the projection logic is small and specific to `aligned_seam_target`.

## Files in Scope (read + edit)

The three production/config files are the primary change surface; the three listed test files are required AC coverage and are kept in the packet rather than split into a follow-up.

- `modules/core-modules/seam-placer/src/lib.rs` - role: continuous projection + degraded fallback + existing rotation; expected change: replace vertex-only snap in `aligned_seam_target`, add `project_onto_wall_segment`, add degraded fallback in `run_wall_postprocess`.
- `modules/core-modules/seam-placer/seam-placer.toml` - role: module manifest; expected change: `default = "aligned"`.
- `modules/core-modules/seam-planner-default/seam-planner-default.toml` - role: module manifest; expected change: `default = "aligned"`.
- `modules/core-modules/seam-placer/tests/seam_continuous_projection_tdd.rs` - role: projection regression coverage; expected change: add inserted-point, parallel-metadata, closed-loop, and degenerate-loop tests.
- `modules/core-modules/seam-placer/tests/seam_degraded_fallback_tdd.rs` - role: degraded-path coverage; expected change: add missing-key, non-fatal-reporting, local-selection, and wall-preservation tests.
- `crates/slicer-runtime/tests/e2e/seam_aligned_default_e2e.rs` - role: end-to-end default coverage; expected change: add multi-region aligned-default and 0.05 mm final-placement assertions.

## Read-Only Context

- `modules/core-modules/seam-placer/src/lib.rs` - lines 121-183 and 245-353 only - existing `aligned_seam_target`, `find_seam_location`, `rotate_wall_loop`, `push_reordered_wall_loop`, and `run_wall_postprocess` structure.
- `crates/slicer-sdk/src/error.rs` - lines 1-40 only - `ModuleError` definition and `non_fatal` constructor.
- `crates/slicer-runtime/src/progress_events.rs` - lines 121-186 only - non-fatal error surfacing in progress events.
- `docs/01_system_architecture.md` - lines 986-998 only - seam-first contract and final projection stage.
- `docs/02_ir_schemas.md` - lines 959-1075 only - `WallLoop`, `PerimeterRegion`, `SeamPosition` schemas.
- `docs/15_config_keys_reference.md` - lines 166-226 only - `seam_mode` config key and default values.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load directly.
- `target/`, `Cargo.lock`, generated code, and vendored dependencies - never load.
- WIT/IR identity and host scheduling - packet 1.
- Canonical scoring, visibility, overhang, retry, and spline - packet 2.
- `crates/slicer-wasm-host/**` - delegate symbol lookups only; do not browse.

## Expected Sub-Agent Dispatches

- Question: canonical `place_seam` nearest-point projection behavior — does OrcaSlicer project onto the nearest segment point (continuous) or snap to the nearest vertex? What is the exact projection formula?; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` (≤200 words); purpose: Step 1 continuous projection implementation.
- Question: how does the existing non-fatal error progress event surface in the runtime? What is the exact `ModuleError::non_fatal` constructor signature and the progress event path?; scope: `crates/slicer-runtime/src/progress_events.rs`; return: `LOCATIONS` (file:line + 1-line context, ≤10 entries); purpose: Step 2 degraded fallback wiring.

## Data and Contract Notes

- IR/manifest contracts: `SeamPosition.point` is f32 mm; wall loop `path.points` are f32 mm. `feature_flags` and `width_profile.widths` must stay parallel to `path.points` after insertion. The inserted point's flag and width are interpolated from the segment's endpoints.
- WIT boundary: no WIT changes in this packet. `ModuleError::non_fatal` carries `fatal: false` through WIT `module-error` as defined in packet 1.
- Determinism/scheduler constraints: continuous projection is deterministic given the planner's target and wall geometry. No RNG or sampling is involved.

## Locked Assumptions and Invariants

- Wall preservation is unconditional: every region's walls reach the output regardless of seam state, missing plan, or degenerate geometry.
- Continuous projection applies to aligned modes only. Nearest, rear, and random modes keep their existing vertex-based selection and are not modified.
- The default change applies to both manifests simultaneously. A mismatch between the two manifests is a bug.
- The 0.05 mm final seam tolerance in AC-4 is a hard bound; the projected point must be within this distance of the planner's target.
- `ModuleError::non_fatal` is the only channel for degraded reporting; no silent pristine-wall emission is permitted.
- The default change to `aligned` amends ADR-0046's normative clause "the default remains `nearest`" (`docs/adr/0046-aligned-seam-in-seam-planning-prepass.md` L50) and the closing clause "nearest mode is untouched end-to-end; aligned / aligned_back are opt-in via seam_mode" (L97–98). This is recorded as deviation `D-283-ADR-0046-AMENDED` in `docs/DEVIATION_LOG.md`. The amendment is justified by the algorithmic canonical parity target: OrcaSlicer's default `seam_position` is `spAligned` (`docs/specs/fork-gaps-wave1-plan.md` L31), and the deviation row quotes both the contested clause and the canonical default to make the change auditable.

## Risks and Tradeoffs

- Point insertion changes wall loop cardinality, affecting downstream consumers that assert on vertex count. All existing tests that assert on vertex count must be reviewed and updated if they break.
- Default change may break existing e2e tests that assume `nearest` behavior. The e2e test suite must be run after the change to identify and fix regressions.
- Interpolation method for `feature_flags` and `width_profile` at the inserted point is left to the implementer (linear, nearest-neighbor, or canonical-specific). The wrong choice could produce non-canonical seam behavior, but the parallel invariant is the hard requirement.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M`
- Highest-risk dispatch and required return format: canonical `place_seam` nearest-point projection, `SUMMARY`.

## Open Questions

- `[FWD]` The implementer may choose the interpolation method for `feature_flags` and `width_profile` at the inserted point (linear, nearest-neighbor, or canonical-specific) provided parallelism is maintained.
