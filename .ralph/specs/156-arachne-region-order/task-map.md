# Task Map: 156-arachne-region-order

## Backlog Mapping

This audit-driven packet has no task ID. It closes G12 in
`docs/18_arachne_parity_audit.md` only when the complete production ordering
path is faithful to OrcaSlicer.

`docs/07_implementation_status.md` has no G12 or region-order row. Its
`TASK-156` is unrelated runtime-budget work and must not be mapped here.

## Step Mapping

| Step | G12 responsibility | Evidence |
| --- | --- | --- |
| 1 | Canonical RED tests | focused failing tests name each missing behavior |
| 2 | Core constraint/grid/walk port | core TDD suite |
| 3 | Finalized-line placement | G12 runtime fixture |
| 4a | WIT/SDK/core three-state declaration | `cargo check -p slicer-sdk` + `cargo check -p slicer-core` |
| 4b | WASM-host and module propagation | workspace check and guest freshness |
| 5 | Real WIT-boundary evidence | targeted runtime integration test |
| 6 | Module commitment | module sequence tests |
| 7 | Optimizer preservation | end-to-end sequence tests |
| 8 | Audit/WIT/deviation docs | Doc Impact greps |
| 9 | Architecture docs and ceremony | all AC commands and packet review |

## Ownership Map

- The perimeter module resolves `wall_sequence` and commits final wall order.
- WIT/SDK/host transports the resolved sequence without interpretation.
- Core creates a canonical proposed region order over finalized lines.
- Path optimization preserves committed sequence while optimizing permitted
  travel.

## Dependencies

- No active packet dependency. Existing G11/G15/G20 work is adjacent but out
  of scope.
- This packet now explicitly touches WIT and `path-optimization-default`; any
  prior statement that they were out of scope is superseded.
