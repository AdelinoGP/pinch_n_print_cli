# Task Map: 31_support-planner-orca-algorithmic-parity → docs/07

This packet closes the five algorithmic v1 limitations of `support-planner` left after packet `30_support-planner-prepass-wit-plumbing` plumbed the layer plan and region segmentation through the prepass WIT. It is additive — packets 26, 28, and 30 stay `implemented` and are not superseded.

## Task ID Mapping

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Notes |
| --- | --- | --- | --- | --- | --- |
| `TASK-163` | Step 1 | `docs/01`–`docs/05`, `docs/08`, `docs/09` | none | `TreeSupport.cpp` 720–800, 1460–1700, 1913, 2625–2860; `TreeSupport.hpp`; `TreeModelVolumes.cpp` | Read-only discovery of OrcaSlicer reference + packet-30 projector pattern. |
| `TASK-163` | Step 2 | `docs/04` | `.ralph/specs/31_support-planner-orca-algorithmic-parity/design.md` | none | Resolve open question Q3. |
| `TASK-163` | Step 3 | `docs/02` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-ir/src/lib.rs` | none | New `SlicePreviewIR` type. |
| `TASK-163` | Step 4 | `docs/01`, `docs/04` | `crates/slicer-host/src/prepass.rs`, `crates/slicer-host/src/blackboard.rs`, optional `modules/core-modules/slice-preview/*` | none | New `PrePass::SlicePreview` stage (built-in or user, per Q3). |
| `TASK-163` | Step 5 | `docs/03` | `wit/world-prepass.wit` | none | New WIT records + extended export signature. |
| `TASK-163` | Step 6 | `docs/03`, `docs/05` | `crates/slicer-sdk/src/prepass_types.rs`, `crates/slicer-sdk/src/prelude.rs`, `crates/slicer-sdk/src/traits.rs`, `crates/slicer-macros/src/lib.rs`, `crates/slicer-host/src/wit_host.rs` | none | SDK + macro + projector + dispatcher. |
| `TASK-163` | Step 7 | `docs/04` | `crates/slicer-host/src/prepass.rs` | none | `required_slots` extension. |
| `TASK-163` | Step 8 | `docs/05` | `.ralph/specs/31_support-planner-orca-algorithmic-parity/design.md` | none | Resolve open question Q4. |
| `TASK-163` | Step 9 | `docs/02`, `docs/04` | `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` (new) | `TreeSupport.cpp` 720–800, 2625–2860 | Failing TDD tests covering the new ACs. |
| `TASK-163` | Step 10 | `docs/05`, `docs/08` | `modules/core-modules/support-planner/src/lib.rs`, `crates/slicer-helpers/src/geometry.rs` | `TreeSupport.cpp` 720–800, 2625–2860; `TreeModelVolumes.cpp` | Avoidance, collision, wall-count move, radius tapering. |
| `TASK-163` | Step 11 | `docs/02` | `crates/slicer-ir/src/slice_ir.rs`, `crates/slicer-host/src/dispatch.rs`, `modules/core-modules/tree-support/src/lib.rs`, `modules/core-modules/support-planner/src/lib.rs` | `TreeSupport.cpp` 1460–1700, 1913 | Resolve Q2; raft + interface densification. |
| `TASK-163` | Step 12 | `docs/03` | `modules/core-modules/support-planner/support-planner.toml` | none | Config schema rewrite. |
| `TASK-163` | Step 13 | `docs/03`, `docs/05` | `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` | none | Config-validation negatives. |
| `TASK-163` | Step 14 | `docs/02`, `docs/12` | `resources/golden/benchy_tree_support_orca_branch_count.txt` (new), `resources/golden/benchy_tree_support_orca_endpoints.txt` (new), `crates/slicer-host/tests/prepass_support_generation_orca_parity_tdd.rs` | `TreeSupport.cpp` overall | Resolve Q5; Benchy OrcaSlicer parity check. |
| `TASK-163` | Step 15 | `docs/05` | `modules/core-modules/support-planner/src/lib.rs` | none | Remove v1 module-level doc bullets for limits 3–7. |
| `TASK-163` | Step 16 | `docs/03` | every `modules/core-modules/*/wit-guest/`, every `*.wasm` | none | Cascade rebuild. |
| `TASK-163` | Step 17 | `docs/07` | `docs/07_implementation_status.md` | none | Backlog row. |
| `TASK-163` | Step 18 | `docs/11`, `docs/12` | none | none | Packet completion gate. |

## Superseding Relationship

Packet 31 does **not** supersede any prior packet.

- Packet `28_tree-support-multi-layer-propagation` (status: `implemented`). Documented all seven v1 limitations. Packets 30 + 31 close them additively.
- Packet `30_support-planner-prepass-wit-plumbing` (status will be `implemented` before packet 31 activates). Closed limits 1 + 2. Packet 31 picks up the remaining five.
- Packet `26_live-support-module-evidence` (status: `implemented`). Unchanged — the grid-MST fallback path is independent of all v2 algorithmic work.

## docs/07 Reconciliation Note

`TASK-163` is a new task added by this packet. Draft line for `docs/07_implementation_status.md` Workstream 3:

```
- [ ] TASK-163 Close the five algorithmic v1 limitations of `support-planner` (avoidance/collision cache, radius tapering, raft + interface layers, wall-count-aware move scaling, OrcaSlicer config keys) by introducing `PrePass::SlicePreview` + `SlicePreviewIR` and consuming the per-layer outlines through a new `slice-preview-view` on `run-support-generation`. Continues TASK-120 acceptance evidence. Wired by packet `31_support-planner-orca-algorithmic-parity`.
```

`TASK-120` (Phase H acceptance with tree supports) and `TASK-120b` (live support generation evidence) are not closed by this packet but receive substantial new evidence:

- After Step 14 lands, the Benchy OrcaSlicer parity check provides the strongest piece of TASK-120 evidence to date. A short closure-block addendum may optionally be appended to `TASK-120` — optional, since TASK-120's text remains broader than this packet's deliverable.

`TASK-161` (packet 28) and `TASK-162` (packet 30) are unchanged.

## Parallelism Note

Packet 31 is serial by construction. It must run after packet 30 closes. No parallel tracks. Steps 4, 8, 11, and 14 each block on an open question (Q3, Q4, Q2, Q5 respectively); these must be resolved sequentially before the corresponding step starts.
