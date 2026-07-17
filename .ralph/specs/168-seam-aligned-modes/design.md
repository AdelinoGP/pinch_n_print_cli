# Design: 168-seam-aligned-modes

## Controlling Code Paths

- Primary code path: `PrePass::SeamPlanning` dispatch (`crates/slicer-wasm-host/src/dispatch.rs:742-831`) → `SeamPlannerDefault::run_seam_planning` (`modules/core-modules/seam-planner-default/src/lib.rs:54-274`) → blackboard `SeamPlanIR` → host injection into `PerimeterRegionView.resolved_seam` (`dispatch.rs:1394-1410` at guest push; commit-time backfill `crates/slicer-runtime/src/layer_executor.rs:1830-1847`, ADR-0020) → `SeamPlacer::run_wall_postprocess` (`modules/core-modules/seam-placer/src/lib.rs:201-279`).
- Neighboring tests/fixtures: `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs` (SDK-level `PerimeterRegionViewBuilder` + `seam_candidate` helpers), `modules/core-modules/seam-planner-default/tests/seam_planner_tdd.rs`, `crates/slicer-runtime/tests/contract/` (WIT drift + prepass harvest suites).
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

- **Per-layer modules cannot chain across layers.** Guest instances are re-created per call (fresh `wasmtime::Store`, `dispatch.rs:315-386`) and layers run under `par_iter` (`layer_executor.rs:200-215`) with per-layer arenas and a read-only blackboard. All cross-layer machinery MUST live in the `PrePass::SeamPlanning` stage; `seam-placer` only consumes.
- **WIT version policy:** adding the required `layer-plan` parameter to `run-seam-planning` is a type change to an existing export → major world-version bump (docs/11 rules; DEV-084 is the on-record precedent from packet 130).
<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Caveat to the unit rule for this packet: the seam data path (`MeshObjectView.vertices`, `Point3WithWidth`, `SeamPlanEntry.chosen_position`, seam-placer tolerances like the `0.001` match at `seam-placer/src/lib.rs:114-117`) is **f32 millimetres**, not integer units. Orca constants expressed in scaled coords must be converted to mm; constants already in mm (angles, weights) pass through unchanged. State the unit in a comment beside every ported constant.

## Code Change Surface

- Selected approach: aligned machinery as a whole-object pass in `seam-planner-default`, consuming a new `layer-plan` WIT parameter for real layer z's; `seam-placer` gains `Aligned` / `AlignedBack` variants whose selection path inverts today's preference — it takes the host-injected `resolved_seam` (the planner's chained choice) and snaps it to the nearest `seam_candidates()` position (fallback: nearest wall-loop vertex) before `find_seam_location`/rotation.
- Exact functions, traits, manifests, tests, and fixtures:
  - `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` — `export run-seam-planning: func(objects, layer-plan: layer-plan-view, output, config)`; world version major bump. The `layer-plan-view` type already exists in this world (used by `run-support-geometry`).
  - `crates/slicer-sdk/src/traits.rs::PrepassModule::run_seam_planning` — add `layer_plan: &LayerPlanView` (match the existing `run_support_geometry` parameter type spelling).
  - `crates/slicer-macros` guest shim for world-prepass — marshal the new parameter (mirror the run-support-geometry arm).
  - `crates/slicer-wasm-host/src/dispatch.rs` prepass arm (`742-831` range) — push the layer-plan view into the seam-planning call (blackboard layer plan is already available to the prepass driver for support-geometry).
  - `modules/core-modules/seam-planner-default/src/lib.rs` — mode enum (`Nearest`/`Rear`/`Random`/`Aligned`/`AlignedBack`) replacing the raw `mode: String`; `run_seam_planning` dispatches: existing corner-MVP behavior for nearest/rear/random (unchanged output), new aligned driver for the two new modes using real layer z's from `layer_plan`.
  - `modules/core-modules/seam-planner-default/src/comparator.rs` (new, attribution header) — `SeamCandidate` internal struct (visibility, overhang, embedded_distance, local_ccw_angle, central_enforcer), `compute_angle_penalty`, `SeamComparator` with `is_first_better`/`is_first_not_much_worse`-equivalent predicates and the `spAligned`/`spAlignedBack`/`spRear` branches, `pick_seam_point`, `pick_nearest_seam_point_index`, `pick_random_seam_point` — all ported from canonical `SeamPlacer.cpp` (cite by function name only).
  - `modules/core-modules/seam-planner-default/src/visibility.rs` (new, attribution header) — mesh AABB raycast visibility ported from `raycast_visibility` / `calculate_candidates_visibility`, plus `calculate_overhangs_and_layer_embedding` equivalents computed from per-layer contours; deterministic fixed sample set (no RNG) — record the sampling deviation in `D-168-SEAM-PREPASS-SOURCE` if the count differs from canonical.
  - `modules/core-modules/seam-planner-default/src/align.rs` (new, attribution header) — `find_next_seam_in_layer`, `find_seam_string`, `align_seam_points` driver, and the least-squares cubic B-spline fit ported from `Curves.hpp::fit_cubic_bspline`.
  - `modules/core-modules/seam-planner-default/src/contours.rs` (new) — per-layer polygon extraction by z-plane sectioning of `MeshObjectView` triangles (PNP-side substitute for Orca's per-layer `Perimeter` polygons; this is the D-168 deviation source).
  - `modules/core-modules/seam-planner-default/seam-planner-default.toml` and `modules/core-modules/seam-placer/seam-placer.toml` — `values = ["nearest", "rear", "random", "aligned", "aligned_back"]`.
  - `modules/core-modules/seam-placer/src/lib.rs` — `SeamMode::{Aligned, AlignedBack}`, `seam_mode()` strings, `on_print_start` parse arms, aligned branch in `run_wall_postprocess` seam-target closure: `resolved_seam()` first, snap to nearest candidate via 2D XY distance, then existing `find_seam_location` + `rotate_wall_loop`.
  - Tests: `modules/core-modules/seam-placer/tests/seam_aligned_mode_tdd.rs` (new), `modules/core-modules/seam-planner-default/tests/seam_aligned_planning_tdd.rs` (new; 20-layer square-prism triangle-mesh fixture + layer-plan builder).
- Rejected alternatives and reasons:
  - Per-object anchor accumulator in `seam-placer` — rejected by the approved plan (quality: no chaining/smoothing) and impossible anyway (parallel layers, no cross-layer state).
  - Host-builtin aligned pass (native, in `slicer-runtime`) — rejected: TASK-159 deliberately created the `SeamPlanning` prepass + `SeamPlanIR` channel for exactly this; a builtin would bypass the module architecture and ADR-0024's producer-trait seam design.
  - Deriving layer z's from `layer_height` config instead of the WIT parameter — rejected: breaks under adaptive layer heights and duplicates `LayerPlanIR` truth; the manifest already declares `reads = ["LayerPlanIR"]` that the signature never honored.
  - Extending `SeamPlanIR` with seam-string metadata — rejected: `chosen_candidate` per `(layer, object, region)` fully encodes the outcome; no IR schema bump needed.

## Files in Scope (read + edit)

Primary (justified above 3: the WIT parameter change mechanically touches four crates; each edit is small and template-driven by the `run-support-geometry` precedent):

- `modules/core-modules/seam-planner-default/src/lib.rs` (+ new `comparator.rs`, `visibility.rs`, `align.rs`, `contours.rs`) - role: aligned machinery home; expected change: mode dispatch + port modules.
- `modules/core-modules/seam-placer/src/lib.rs` - role: consumption; expected change: enum + parse + snap branch.
- `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` - role: contract; expected change: one parameter + version bump.
- `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros` (world-prepass shim), `crates/slicer-wasm-host/src/dispatch.rs` - role: signature plumbing; expected change: mirror run-support-geometry's layer-plan handling.
- Both module TOML manifests - expected change: enum values line.

## Read-Only Context

- `crates/slicer-runtime/src/layer_executor.rs` - lines `1810-1900` only - purpose: confirm injection/backfill semantics stay untouched.
- `crates/slicer-wasm-host/src/dispatch.rs` - lines `742-831` and `1380-1414` only - purpose: prepass arm shape and region push injection.
- `crates/slicer-ir/src/slice_ir.rs` - lines `1040-1096` and `1600-1945` only - purpose: `ScoredSeamCandidate`/`SeamPlanEntry`/`SeamPlanIR` and `Point3WithWidth`/`SeamReason`/`SeamCandidate`/`SeamPosition` shapes.
- `crates/slicer-sdk/src/views.rs` - lines `600-660` only - purpose: `PerimeterRegionView` accessor signatures.
- `modules/core-modules/seam-placer/tests/seam_placer_dispatch_tdd.rs` - purpose: fixture-builder idioms to reuse.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `crates/slicer-core`, perimeter modules (`arachne-perimeters`, `classic-perimeters`), `crates/slicer-gcode` - delegate symbol lookups; do not browse
- `docs/03_wit_and_manifest.md`, `docs/02_ir_schemas.md`, `docs/07_implementation_status.md` - delegate; ranged edits only via dispatch

## Expected Sub-Agent Dispatches

- Question: verbatim behavior of `align_seam_points` (string seeding order, `seam_align_score_tolerance`-style constants, smoothing weights, `spRear` branch) plus `find_seam_string` / `find_next_seam_in_layer` chaining rules; scope: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`; return: `SUMMARY` (+ ≤3 SNIPPETS ≤30 lines); purpose: Steps 4-5.
- Question: `SeamComparator` predicate bodies and `compute_angle_penalty` formula incl. all constants with units; scope: same file; return: `SNIPPETS`; purpose: Step 3.
- Question: `fit_cubic_bspline` signature and algorithm outline; scope: `OrcaSlicerDocumented/src/libslic3r/Geometry/Curves.hpp`; return: `SUMMARY`; purpose: Step 5.
- Question: exact `layer-plan-view` WIT record fields and how `run-support-geometry`'s dispatch arm builds it; scope: `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit` + `crates/slicer-wasm-host/src/dispatch.rs`; return: `LOCATIONS`; purpose: Step 1.
- Question: does `docs/02_ir_schemas.md` describe the `run-seam-planning` signature (needs edit) or only `SeamPlanIR` (no edit)?; scope: `docs/02_ir_schemas.md`; return: `FACT`; purpose: doc-impact confirmation.

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

- Chaining over prepass mesh-derived contours instead of Orca's final perimeter polygons (Orca runs `SeamPlacer` after perimeter generation) — geometric offsets up to one wall inset; mitigated by the seam-placer snap step; recorded as `D-168-SEAM-PREPASS-SOURCE`.
- Visibility raycasting in a guest over large meshes is O(candidates × triangles) without an AABB tree; port a simple BVH or cap candidate counts; benchmark risk flagged for the acceptance ceremony (prepass runs once, not per layer).
- The WIT parameter change rebuilds every guest and can break the WIT-drift contract suite; Step 1 runs that suite explicitly.
- B-spline fit numerics (f32 vs Orca's float with Eigen): assert smoothing ACs with mm-scale tolerances (0.5 mm), not exact values.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 5, align + smoothing port)
- Highest-risk dispatch and required return format: `align_seam_points`/`find_seam_string` behavior extraction from `SeamPlacer.cpp` — `SUMMARY` + ≤3 SNIPPETS, strictly capped.

## Open Questions

- `[FWD]` Snap radius in seam-placer aligned mode: unlimited nearest-candidate (Orca `place_seam` behavior is nearest perimeter point) vs a capped radius. Default to unlimited nearest among `seam_candidates()`; implementer may cap if fixtures show pathological jumps — record the constant either way.
- `[FWD]` Visibility sample count/quality: canonical raycast counts may be too slow in WASM; implementer picks a deterministic reduced budget and records it in the D-168 deviation row.
- `[FWD]` Whether `docs/02_ir_schemas.md` documents the seam-planning WIT signature (then it needs the same one-line edit as docs/03) — resolve via the FACT dispatch listed above.
