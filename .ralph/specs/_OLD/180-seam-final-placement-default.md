---
status: implemented
packet: 180-seam-final-placement-default
task_ids:
  - TASK-293
---

# 180-seam-final-placement-default

## Goal

Project canonical aligned seam targets onto continuous final wall geometry, preserve wall-loop feature flags and width profiles through rotation and point insertion, report degraded fallback via non-fatal module errors when no plan reaches a region, and make `aligned` the default `seam_mode` matching OrcaSlicer.

## Problem Statement

Packet 168's seam-placer snaps the planner's fitted point to the nearest existing wall vertex only; canonical OrcaSlicer projects onto the nearest point of the final perimeter, including segment interpolation. This means PNP's smoothed seam can jump to a different corner and lose continuity. Also, the current default is `nearest` while Orca's is `aligned`, and missing plans silently emit pristine walls instead of reporting degraded success.

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

## Data and Contract Notes

- IR/manifest contracts: `SeamPosition.point` is f32 mm; wall loop `path.points` are f32 mm. `feature_flags` and `width_profile.widths` must stay parallel to `path.points` after insertion. The inserted point's flag and width are interpolated from the segment's endpoints.
- WIT boundary: no WIT changes in this packet. `ModuleError::non_fatal` carries `fatal: false` through WIT `module-error` as defined in packet 178.
- Determinism/scheduler constraints: continuous projection is deterministic given the planner's target and wall geometry. No RNG or sampling is involved.

## Locked Assumptions and Invariants

- Wall preservation is unconditional: every region's walls reach the output regardless of seam state, missing plan, or degenerate geometry.
- Continuous projection applies to aligned modes only. Nearest, rear, and random modes keep their existing vertex-based selection and are not modified.
- The default change applies to both manifests simultaneously. A mismatch between the two manifests is a bug.
- The 0.05 mm final seam tolerance in AC-4 is a hard bound; the projected point must be within this distance of the planner's target.
- `ModuleError::non_fatal` is the only channel for degraded reporting; no silent pristine-wall emission is permitted.
- The default change to `aligned` amends ADR-0046's normative clause "the default remains `nearest`" (`docs/adr/0046-aligned-seam-in-seam-planning-prepass.md` L50) and the closing clause "nearest mode is untouched end-to-end; aligned / aligned_back are opt-in via seam_mode" (L97–98). This is recorded as the `ADR-0046 amendment` in `docs/DEVIATION_LOG.md`. The amendment is justified by the algorithmic canonical parity target: OrcaSlicer's default `seam_position` is `spAligned` (`docs/specs/fork-gaps-wave1-plan.md` L31), and the deviation row quotes both the contested clause and the canonical default to make the change auditable.
- **Scope-bound (packet 178 owns):** `PerimeterRegionView` does not expose the `variant_chain` accessor (only `SliceRegionView` does, at `crates/slicer-sdk/src/views.rs:372-377`). Packet 180's design §"Code Change Surface" forbids WIT changes, so the degraded-fallback error message identifies the missing region by `(layer, object, region_id, variant_chain=[])` — the variant chain component is rendered as the literal string `[]` because it is not in the API surface. The packet-authoring AC-2 language ("carrying a message identifying the (layer, object, region_id, variant_chain) key") is partially evidenced: the message contains the first three components; the fourth is the empty literal. A future packet that adds `variant_chain` to `PerimeterRegionView` can re-word the error to identify the actual chain.
- **Scope-bound (packet 178 owns):** The `ModuleError::non_fatal` value is created in `modules/core-modules/seam-placer/src/lib.rs` and reaches the in-process Rust unit-test path. The propagation of `fatal: false` through the WIT `module-error` record (defined in packet 178) is not exercised by packet 180's tests because the seam-placer's externally-visible WIT surface is unchanged and packet 180's tests instantiate the module in-process via `SeamPlacer::on_print_start` + `run_wall_postprocess`. The dispatch path in `crates/slicer-wasm-host/src/dispatch.rs` currently maps any `ModuleError` to `LayerStageError::FatalModule` regardless of the `fatal` field, which is a known host limitation tracked separately. The unit-test contract for AC-2 is that the function returns `Err(ModuleError { fatal: false, code: 6, ... })` and the output builder still contains all walls; both are verified.

## Risks and Tradeoffs

- Point insertion changes wall loop cardinality, affecting downstream consumers that assert on vertex count. All existing tests that assert on vertex count must be reviewed and updated if they break.
- Default change may break existing e2e tests that assume `nearest` behavior. The e2e test suite must be run after the change to identify and fix regressions.
- Interpolation method for `feature_flags` and `width_profile` at the inserted point is left to the implementer (linear, nearest-neighbor, or canonical-specific). The wrong choice could produce non-canonical seam behavior, but the parallel invariant is the hard requirement.
