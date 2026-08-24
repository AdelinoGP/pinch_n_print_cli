# Task Map: 239-support-independent-layer-z

Crosswalk for the `TASK-399`..`TASK-408` slice. Registration in
`docs/07_implementation_status.md` is deferred to the packet-owned closure step
(implementation-plan.md Step 10); rows below are the expected registration content.

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-399` | Step 1 | `docs/specs/support-parity-gap-register.md` (G-02) | none (discovery) | - | S | Live re-verification of both blockers; LOCATIONS inventory is the deliverable |
| `TASK-400` | Step 2 | plan §6 invariants 6/8/9/12/13 | `crates/slicer-runtime/tests/integration/offgrid_routing_tdd.rs` (new), `integration/main.rs` | - | S | Red-first: AC-2/AC-N2 fail on today's drop behavior |
| `TASK-401` | Step 3 | `docs/08_coordinate_system.md` (via constraint) | `crates/slicer-runtime/src/layer_executor.rs` (`is_same_z_entity` → total partition) | - | M | Invariant 6 preserved; substrate suites stay green |
| `TASK-402` | Step 4 | plan §7 E4/E5/E8 | `crates/slicer-runtime/src/pipeline.rs`, offgrid_routing_tdd.rs | - | M | First production call of `execute_per_layer_with_committed_anchored_events`; support-only row synthesis; AC-1/3/4 + AC-N3 |
| `TASK-403` | Step 5 | plan §7 E1/E7 | verdict record only in `docs/07_implementation_status.md` | `GCode.cpp::_extrude` (delegated) | M | Measure-first gate; verdict MISSCALE_FIXED / CONSISTENT with three measured numbers |
| `TASK-404` | Step 6 | plan §7 E3 | `crates/slicer-gcode/src/emit.rs` (fix branch only) + verdict test | `GCode.cpp::_extrude` (delegated, fix branch) | M | Branch on TASK-403; CONSISTENT means zero emitter edits |
| `TASK-405` | Step 7 | plan §8 human gate | none in-tree (`tmp/p239-*` artifacts) | - | S | Freshness gate precedes slicing; FRESH token captured |
| `TASK-406` | Step 8 | plan §9 reference regeneration | packet.spec.md checklist notes only | references existence-checked, never generated | S | REFS-PRESENT or REFS-ABSENT-GATE-OPEN; T11: no legacy figures |
| `TASK-407` | Step 9 | `.agents/doc-index.md` (fallback only) | none planned | - | M | Reconciliation: integration binary, slicer-gcode, check/clippy/literals gates |
| `TASK-408` | Step 10 | `docs/07_implementation_status.md` conventions | registration edits only | - | S | Packet-owned closure; status stays draft until gate sign-off |

Copy costs from `implementation-plan.md`. No row is L; aggregate is M. Dependency note:
activation requires `238c-support-renderer-flow-interfaces` to reach `implemented`
(FORWARD DEPENDENCY — 238c is generated as `draft`). This packet shares no file surface
with 240-support-raft or 241-support-agg-rasterizer.
