# Design: 22_live-seam-contract-repair

## Controlling Code Paths

- Seam resolution and rotation currently live in `modules/core-modules/seam-placer/src/lib.rs::run_wall_postprocess`
- Host collection and conversion live in `crates/slicer-host/src/wit_host.rs::push_resolved_seam`, `push_reordered_wall_loop`, and `convert_perimeter_output`
- Layer-stage commit ownership lives in `crates/slicer-host/src/dispatch.rs::commit_layer_outputs`
- Marker emission lives in `modules/core-modules/path-optimization-default/src/lib.rs::run_path_optimization`
- Neighboring regressions already exist in `crates/slicer-host/tests/live_seam_path_tdd.rs` and `crates/slicer-host/tests/dispatch_tdd.rs`

## Architecture Constraints

- `Layer::PerimetersPostProcess` is still the owning stage for live seam application in this packet; no new prepass stage is introduced here
- `PerimeterIR.regions[*].seam_candidates` remains the canonical input to seam selection on the live path
- `rotated_wall_loops` continues to replace `wall_loops` in `convert_perimeter_output`; therefore the module must emit a full region-preserving wall-loop set rather than only the target wall
- `Layer::PathOptimization` remains comment-only for this seam slice; travel policy stays with packet `15`

## Selected Implementation Approach

Keep the current WIT surface and repair the current layer-stage contract in place.

1. `seam-placer` chooses a seam from `region.seam_candidates()` using its configured mode
2. Once chosen, `seam-placer` calls `push_resolved_seam(...)`
3. `seam-placer` re-emits every wall loop for the region via `push_reordered_wall_loop(...)`, rotating only the targeted `wall_index` and passing untouched sibling loops through in their original order
4. `convert_perimeter_output` applies the chosen seam only to the emitting origin bucket instead of broadcasting it to every bucket
5. `path-optimization-default` gates marker emission on `emit_layer_markers`

This is the narrowest approach because it avoids a WIT expansion, avoids a new IR type, and uses the existing replacement semantics of `rotated_wall_loops` instead of inventing partial wall-loop patch logic.

## Rejected Alternatives

- Extend `push-reordered-wall-loop` to carry and preserve per-wall indices in host-side storage.
  Rejected because packet `22` can avoid that complexity by having the module emit the full region wall-loop set.
- Add `PrePass::SeamPlanning` immediately.
  Rejected for this packet because it widens scope across IR, scheduler, SDK, and WIT surfaces. That work is split into packet `23`.
- Leave marker suppression for a later path-optimization packet.
  Rejected because `dispatch_tdd` already demonstrates the config contract is broken on the current live path.

## Explicit Code Change Surface

- `modules/core-modules/seam-placer/src/lib.rs`
  - `run_wall_postprocess`
  - candidate-selection helper(s) introduced for `seam_candidates`
- `crates/slicer-host/src/wit_host.rs`
  - `convert_perimeter_output`
  - `PerimeterOutputCollected` handling for origin-scoped seams
- `modules/core-modules/path-optimization-default/src/lib.rs`
  - `run_path_optimization`
- `crates/slicer-host/tests/live_seam_path_tdd.rs`
  - add candidate-selection, region-scoping, sibling-preservation, and determinism regressions
- `crates/slicer-host/tests/dispatch_tdd.rs`
  - keep `path_optimization_emit_layer_markers_false_suppresses_output` as the focused falsifier

## Data and Contract Notes

- `PerimeterIR.regions[*].seam_candidates[*].position.{x,y,z}` is the only allowed source of the chosen seam point in this packet
- `PerimeterIR.regions[*].resolved_seam.point.{x,y,z}` and `.wall_index` must match the chosen candidate exactly
- `PerimeterIR.regions[*].walls[*].path.points`, `feature_flags`, and `width_profile.widths` must stay parallel after rotation
- `LayerAnnotationKind::Comment` and `LayerAnnotationKind::Raw` must both be absent when `path_optimization_emit_layer_markers = false`

## Risks and Tradeoffs

- Re-emitting the full wall-loop set per region slightly increases module-side work, but it avoids widening the host WIT contract mid-slice
- Failing when a chosen seam point is absent from the target wall loop is stricter than silently preserving geometry; this is intentional to keep the contract falsifiable and prevent silent wrong seams
- The packet leaves PrePass seam planning unresolved on purpose; full Orca parity still requires packet `23`

## Open Questions

- No technical design blocker remains inside packet `22`; it stays `draft` only because packet `15` is currently `active`

## Locked Assumptions and Invariants

- `convert_perimeter_output` continues to treat `rotated_wall_loops` as canonical replacement geometry
- origin tags remain the authoritative key for mapping emitted wall loops and seams back to `PerimeterRegion`
- `Layer::PathOptimization` does not reopen move replay in this packet
