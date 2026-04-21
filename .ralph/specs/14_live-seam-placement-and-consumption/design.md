# Design: live-seam-placement-and-consumption

## Controlling Code Paths

- Seam commitment path: `modules/core-modules/seam-placer/src/lib.rs`, `crates/slicer-host/src/dispatch.rs`, and `crates/slicer-host/src/wit_host.rs`.
- Seam consumption path: `modules/core-modules/path-optimization-default/src/lib.rs` plus the host commit surface in `crates/slicer-host/src/dispatch.rs` and `crates/slicer-host/src/layer_executor.rs`.
- Neighboring tests or fixtures: `modules/core-modules/seam-placer/tests/seam_placer_tdd.rs`, `crates/slicer-host/tests/dispatch_tdd.rs`, plus new `live_seam_path_tdd.rs` and `seam_consumption_tdd.rs`.
- OrcaSlicer comparison surface: `OrcaSlicerDocumented/src/libslic3r/GCode/SeamPlacer.cpp`.

## Architecture Constraints

- The packet must keep seam placement in `Layer::WallPostProcess` and seam consumption in `Layer::PathOptimization`; it must not collapse the stages.
- Selected approach: expand `path-optimization-default` just enough to replay seam-started wall loops from `resolved_seam`, while leaving broader travel-ordering work to later packets.
- Because TASK-151 explicitly calls out `path-optimization-default`, the seam-consumption fix must live on that module surface, not only on a host-side pre-sort helper.

## Code Change Surface

- Selected approach:
  - add one host integration test for live `resolved_seam` commitment
  - add one new module-level seam-consumption test surface for replayed wall-loop moves
  - widen the path-optimization commit surface narrowly so replayed wall-loop `Move` commands are accepted for this seam slice
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - `modules/core-modules/seam-placer/src/lib.rs`
  - `modules/core-modules/path-optimization-default/src/lib.rs`
  - `crates/slicer-host/src/dispatch.rs`
  - `crates/slicer-host/src/layer_executor.rs`
  - `crates/slicer-host/src/wit_host.rs`
  - `crates/slicer-host/tests/live_seam_path_tdd.rs`
  - `modules/core-modules/path-optimization-default/tests/seam_consumption_tdd.rs`
- Rejected alternatives that were considered and why they were not chosen:
  - host-side seam normalization before path optimization without touching the module: rejected because TASK-151 explicitly requires the module to consume seam output
  - bundling generic travel ordering into this packet: rejected because it would reopen the broader DEV-023 slice too early

## Data and Contract Notes

- IR or manifest contracts touched:
  - `PerimeterIR.regions[*].seam_candidates`
  - `PerimeterIR.regions[*].resolved_seam`
  - replayed wall-loop `GCodeCommand::Move` output on the path-optimization surface
- WIT boundary considerations:
  - the packet may require a narrow acceptance change on the path-optimization commit surface so replayed wall-loop moves are no longer treated as unsupported overrides
- Determinism or scheduler constraints:
  - replayed wall loops must be deterministic for repeated identical inputs

## Locked Assumptions and Invariants

- Seam placement stays upstream of travel/retraction policy.
- Packet `11` owns final text formatting, so this packet stops at replayed commands and layer-stage output.

## Risks and Tradeoffs

- Risk: move replay support can accidentally widen the path-optimization surface too much. Mitigation: limit the packet to seam-started wall-loop replay only.
- Risk: seam commitment may pass in module tests but fail in host dispatch. Mitigation: keep the host `resolved_seam` test as a required acceptance gate.

## Open Questions

- None. The packet chooses the `path-optimization-default` replay approach explicitly.