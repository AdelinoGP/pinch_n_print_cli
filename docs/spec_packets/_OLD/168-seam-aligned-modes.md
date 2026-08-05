---
status: implemented
packet: 168-seam-aligned-modes
task_ids:
  - TASK-274
---

# 168-seam-aligned-modes

## Goal

Port OrcaSlicer SeamPlacer's aligned seam path (per-layer candidate scoring with visibility/angle penalties, cross-layer seam-string chaining, least-squares B-spline smoothing) into the `seam-planner-default` prepass module, and add `aligned` / `aligned_back` as accepted `seam_mode` values that the per-layer `seam-placer` module consumes by snapping the planner's chained choice to real wall geometry.

## Problem Statement

OrcaSlicer's default `seam_position` is `spAligned`, but PNP's `seam_mode` accepts only `nearest` / `rear` / `random` (`SeamMode` enum, `modules/core-modules/seam-placer/src/lib.rs:31-39`; config read at `lib.rs:185-196`, with `_ => SeamMode::Nearest` fallback). The fork has no PNP value to map Orca's default `spAligned` onto: omitting the key silently falls back to `nearest` (`_ => SeamMode::Nearest`, `lib.rs:195`), while passing `"aligned"` today fails the slice fatally (`ModuleError::fatal(1, "unknown seam_mode: ...")`, `lib.rs:192`). Either way every default-settings slice ends up on `nearest` — a per-slice visible quality regression with no user-facing signal (fork ships no gap warnings). The approved plan (`docs/specs/fork-gaps-wave1-plan.md`, Packet 16) decided a **full Orca-parity port** of canonical `SeamPlacer`'s aligned path over a simple per-object-anchor accumulator; `aligned_back` reuses the same machinery with rear-biased seeding.

Architecture constraint discovered during grounding: per-layer modules are re-instantiated per call and layers execute in parallel (`crates/slicer-runtime/src/layer_executor.rs:200-215`, `crates/slicer-wasm-host/src/dispatch.rs:315-386`), so cross-layer seam-string chaining is impossible inside `seam-placer`. The whole-object channel already exists: `seam-planner-default` (`PrePass::SeamPlanning`, TASK-159) writes `SeamPlanIR` to the blackboard, and the host injects each entry's `chosen_candidate` into the matching region's `resolved_seam` before `seam-placer` runs (`crates/slicer-wasm-host/src/dispatch.rs:1394-1410`; commit-time backfill `crates/slicer-runtime/src/layer_executor.rs:1830-1847`, ADR-0020). The aligned machinery therefore lands in the prepass module; `seam-placer` consumes it.

## Architecture Constraints

- **Per-layer modules cannot chain across layers.** Guest instances are re-created per call (fresh `wasmtime::Store`, `dispatch.rs:315-386`) and layers run under `par_iter` (`layer_executor.rs:200-215`) with per-layer arenas and a read-only blackboard. All cross-layer machinery MUST live in the `PrePass::SeamPlanning` stage; `seam-placer` only consumes.
- **WIT version policy:** adding the required `layer-plan` parameter to `run-seam-planning` is a type change to an existing export → major world-version bump (docs/11 rules; DEV-084 is the on-record precedent from packet 130).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Caveat to the unit rule for this packet: the seam data path (`MeshObjectView.vertices`, `Point3WithWidth`, `SeamPlanEntry.chosen_position`, seam-placer tolerances like the `0.001` match at `seam-placer/src/lib.rs:114-117`) is **f32 millimetres**, not integer units. Orca constants expressed in scaled coords must be converted to mm; constants already in mm (angles, weights) pass through unchanged. State the unit in a comment beside every ported constant.

## Data and Contract Notes

- IR/manifest contracts: `SeamPlanEntry` guest-side record fields are `global-layer-index, object-id, region-id, chosen-position, chosen-wall-index, scored-candidates` (world-prepass.wit:82-89); IR-side it is `region_key: RegionKey` + `chosen_candidate: SeamPosition` + `scored_candidates` (`slice_ir.rs:1066-1073`) — the marshal/harvest layer maps them; do not change either shape.
- WIT boundary: after editing, follow CLAUDE.md §WIT/Type Changes Checklist — search all `wit_host.rs`, `dispatch.rs`, `wit_guest` modules for `run-seam-planning`, and run `cargo build --tests`.
- Determinism/scheduler constraints: prepass runs once per print; all ported scoring must be deterministic (no `std::rand` — port `pick_random_seam_point`'s hash-based determinism or reuse the layer-index scheme; the existing planner already documents a HashMap-iteration determinism hazard at `lib.rs:160-171` — new code must sort before selection).
- `SeamReason { tag: String }` (SDK prepass-side) vs `SeamReason` enum (IR): aligned entries keep tag `"aligned"` so downstream reason-bonus scoring is unchanged.

## Locked Assumptions and Invariants

- The host injection path (dispatch-time `resolved_seam` seeding and commit-time backfill, ADR-0020) is the ONLY channel by which the planner's choice reaches `seam-placer`; this packet must not add a second channel.
- Default `seam_mode` remains `"nearest"` in both manifests.
- `nearest`/`rear`/`random` outputs are byte-identical before and after this packet (AC-N2).
- The wall-preservation invariant in `run_wall_postprocess` (every region's walls reach the output; HIGH-2, comments at `seam-placer/src/lib.rs:208-238`) is preserved by the aligned branch.

## Risks and Tradeoffs

- Chaining over prepass mesh-derived contours instead of Orca's final perimeter polygons (Orca runs `SeamPlacer` after perimeter generation) — geometric offsets up to one wall inset; mitigated by the seam-placer snap step; recorded as `seam source-geometry deviation`.
- Visibility raycasting in a guest over large meshes is O(candidates × triangles) without an AABB tree; port a simple BVH or cap candidate counts; benchmark risk flagged for the acceptance ceremony (prepass runs once, not per layer).
- The WIT parameter change rebuilds every guest and can break the WIT-drift contract suite; Step 1 runs that suite explicitly.
- B-spline fit numerics (f32 vs Orca's float with Eigen): assert smoothing ACs with mm-scale tolerances (0.5 mm), not exact values.
