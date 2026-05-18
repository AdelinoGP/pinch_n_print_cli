# Design: 14-rev1_live-seam-placement-and-consumption

## Controlling Code Paths

- **Seam commitment path:** `modules/core-modules/seam-placer/src/lib.rs` → `PerimeterOutputBuilder::push_reordered_wall_loop` → `PerimeterOutputCollected` → `convert_perimeter_output` → `PerimeterIR`
- **WIT boundary:** `crates/slicer-host/src/wit_host.rs` — `PerimeterOutputBuilder` host implementation, `perimeter_region_to_data`
- **Commit surface:** `crates/slicer-host/src/dispatch.rs` — `commit_layer_outputs`
- **PathOptimization path:** `modules/core-modules/path-optimization-default/src/lib.rs` — reverts to comment-only (marker emission)
- **Neighboring tests:** `crates/slicer-host/tests/live_seam_path_tdd.rs`, `modules/core-modules/seam-placer/tests/seam_placer_tdd.rs`
- **OrcaSlicer reference:** `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp` — rotation logic

## Architecture Constraints

- `Layer::PerimetersPostProcess` is the stage that owns wall loop geometry modification. Seam placement writes `resolved_seam` (a reference) AND rotates the wall loop geometry so `path.points[0]` is the seam point.
- `Layer::PathOptimization` must NOT call `push-move` — that method is rejected at that stage per docs/03 contract. The original packet `14` violated this. This revision fixes it.
- `PerimeterIR` is the canonical store of seam-first wall loops. No replay happens at PathOptimization.
- The `feature_flags` and `width_profile` on `WallLoop` must remain parallel to `path.points` after rotation — host validation enforces this at the WIT boundary.

## Code Change Surface

**Selected approach:** Add `push-reordered-wall-loop` to `perimeter-output-builder` WIT resource. Seam-placer rotates wall loop points and calls this method. PerimeterIR is committed with seam-first geometry. PathOptimization reverts to comment-only.

**Exact functions, traits, manifests, tests, or fixtures expected to change:**

1. `wit/deps/ir-types.wit` — add `push-reordered-wall-loop` to `perimeter-output-builder`
2. `crates/slicer-host/src/wit_host.rs` — implement `push_reordered_wall_loop` in `HostPerimeterOutputBuilder`, add `rotated_wall_loops: Vec<WallLoop>` to `PerimeterOutputCollected`
3. `crates/slicer-host/src/wit_host.rs` — update `perimeter_region_to_data` to map rotated wall loop geometry from `PerimeterOutputCollected.rotated_wall_loops`
4. `crates/slicer-host/src/dispatch.rs` — handle `push-reordered-wall-loop` result in `commit_layer_outputs` for `Layer::WallPostProcess`
5. `crates/slicer-host/src/convert.rs` — update `convert_perimeter_output` to use rotated wall loops when present, fall back to original wall loops when not
6. `crates/slicer-sdk/src/postpass_builders.rs` — add `push_reordered_wall_loop` to SDK's `PerimeterOutputBuilder` wrapper
7. `crates/slicer-macros/src/lib.rs` — update `build_layer_world_glue` to handle `push-reordered-wall-loop` binding
8. `modules/core-modules/seam-placer/src/lib.rs` — implement wall loop rotation and call `push_reordered_wall_loop`
9. `modules/core-modules/seam-placer/seam-placer.toml` — update `writes` to include `PerimeterIR.regions` (for rotated geometry) in addition to `PerimeterIR.resolved-seam`
10. `modules/core-modules/path-optimization-default/src/lib.rs` — revert to comment-only output
11. `crates/slicer-host/tests/live_seam_path_tdd.rs` — add new tests for seam rotation behavior
12. `modules/core-modules/path-optimization-default/tests/seam_consumption_tdd.rs` — update tests for no-move output

**Rejected alternatives that were considered and why they were not chosen:**

- **Option A (replay at PathOptimization via `push-move`):** Violates docs/03 `push-move` rejection at `Layer::PathOptimization`. E coordinate was using local width, not cumulative amount. Moves committed as Raw annotations, not proper `ordered_entities`. Superseded by this packet.
- **Host-side pre-normalization before PathOptimization:** Would decouple the seam-first guarantee from the module system. Correct semantics but breaks the module authorship model — seam-placer must own the geometry rotation.
- **Separate `seam-first wall loop` IR type:** Would require adding a new IR type and updating all downstream consumers (Infill, PathOptimization, GCodeEmit). The rotation happens in-place on the existing `WallLoop.path` — no new IR type needed.

## Data and Contract Notes

**IR contracts touched:**
- `PerimeterIR.regions[*].walls[*].path.points` — rotated so `points[0]` is seam vertex
- `PerimeterIR.regions[*].walls[*].feature_flags` — re-indexed to match rotated points
- `PerimeterIR.regions[*].walls[*].width_profile.widths` — must have same cardinality as rotated `points`
- `PerimeterIR.regions[*].resolved_seam` — still written as `Some(SeamPosition)` for diagnostic reads

**WIT boundary considerations:**
- `push-reordered-wall-loop` receives `rotated-wall-loop: wall-loop-view` — the host validates that `feature-flags.len() == path.points.len()` before accepting
- The seam position (`pos: point3-with-width`) and wall index are passed separately so the host can validate the Z envelope and write the `SeamPosition` reference

**Determinism or scheduler constraints:**
- Wall loop rotation must be deterministic: the same `resolved_seam` applied to the same wall loop geometry must produce byte-identical rotated `path.points` across repeated runs
- The rotation algorithm: given `seam_point` and `wall_loop.points`, find `seam_idx` in points (by coordinate match), then emit `points[seam_idx], points[seam_idx+1], ..., points[end], points[0], ..., points[seam_idx-1]`

## Locked Assumptions and Invariants

- **Seam-first invariant:** After `Layer::PerimetersPostProcess` completes, `PerimeterIR.regions[R].walls[W].path.points[0]` is the first vertex of the seam-started wall loop. Downstream stages (Infill, PathOptimization, GCodeEmit) can assume this without re-checking.
- **Parallel cardinality invariant:** `WallLoop.path.points.len() == WallLoop.feature_flags.len() == WallLoop.width_profile.widths.len()`. This is enforced at the WIT boundary on `push-reordered-wall-loop`.
- **Loop closure invariant:** The last emitted point of a rotated wall loop must join cleanly to `points[0]`. The original wall loop is closed; rotation preserves the closed geometry.

## Risks and Tradeoffs

- **Risk:** Rotating wall loop geometry changes the semantic of what `seam-placer` writes. The manifest currently claims `writes = ["PerimeterIR.resolved-seam"]`. After this packet, it must claim write access to the wall loop fields it rotates.
  - **Mitigation:** Update `seam-placer.toml` `writes` to include `PerimeterIR.regions.walls` (the full wall path and feature_flags).
- **Risk:** If downstream code (GCodeEmit, Infill) reads `PerimeterIR.walls[].path` and assumes original ordering, rotating the wall loop would break that code.
  - **Mitigation:** The seam-first invariant is the documented contract. All downstream stages must respect it. If any downstream consumer is reading wall geometry without understanding the seam-first invariant, that is a bug in the consumer.
- **Risk:** Removing `push-move` support at PathOptimization breaks the original packet `14` tests that verified `Move` emission.
  - **Mitigation:** Those tests are now superseded. This packet provides correct tests for the Option B approach.

## Open Questions

- **Q1 (MUST RESOLVE BEFORE ACTIVATION):** `docs/02_ir_schemas.md` currently describes `SeamPosition { point, wall_index }` as a reference annotation, not as a directive that the wall loop must be reordered. The packet implementation requires updating this documentation to explicitly state: "After `Layer::PerimetersPostProcess` completes, `WallLoop.path.points[0]` is the seam-first vertex. The `resolved_seam` field stores a diagnostic reference to the seam position, but the canonical seam-first geometry is stored in the wall loop's path points."
  - **Owner:** This packet must update `docs/02_ir_schemas.md` in Step 1 before the packet can become `active`.
- **Q2:** Does `width_profile.widths` need to be rotated together with `path.points`? Or does it stay aligned with the original vertex indices? The original design has `width_profile.widths` parallel to `path.points`. After rotation, the widths must remain parallel to the rotated points — so yes, they must be rotated.
- **Q3:** Does `feature_flags` need to be rotated? Yes — the fuzzy skin and other per-vertex flags must follow the rotated points so segment i in the rotated loop still has the correct flags.

## Locked Assumptions

- Seam rotation is deterministic — the same input always produces the same rotated output
- The rotation preserves the closed nature of the wall loop (last point joins to first)
- `path-optimization-default` reverts to comment-only output, not move replay