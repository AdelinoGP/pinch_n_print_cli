# Task Map: 43-rev1_macro-prepass-segmentation-output-drain

This packet reopens and supersedes Packet 43. The same `docs/07_implementation_status.md` backlog ids are in scope; the difference is the corrected scaffolding approach (two macro sibling crates instead of one extended crate) and the added regression-defense layer for previously-demoted tests.

## docs/07 → Packet Steps

| docs/07 Task ID | docs/07 Status (current) | Packet Steps | Closure Trigger |
| --- | --- | --- | --- |
| `TASK-130` ("Finish the `#[slicer_module]` prepass segmentation bridge for macro-authored modules. Covers DEV-025.") | `[~]` in-progress | Steps 1, 2, 9, 10, 11 | All 18 ACs (16 positive + 2 negative) green, DEV-025 mismatch 3 closed in `docs/DEVIATION_LOG.md`, single-stage rule documented in `docs/05_module_sdk.md`. |
| `TASK-130a` ("Drain `PaintSegmentationOutput` back through WIT `push-paint-region`...") | `[ ]` not started in docs/07 (drain landed in commit `46aed61` but backlog hasn't been updated) | Steps 3, 4, 5, 8, 10 | Round-trip ACs (AC-5/6/7) green via macro-emitted bytes from `sdk-prepass-paintseg-guest`. |
| `TASK-130b` ("Add end-to-end macro-path regression tests proving `MeshSegmentation` and `PaintSegmentation` round-trip real data through WIT.") | `[ ]` not started | Steps 1, 3, 4, 5, 6, 7, 8, 10 | All round-trip tests load `#[slicer_module]`-authored sibling guests and pass; `macro_all_worlds_roundtrip_tdd` registry extended to cover the two new siblings. |

## Packet Steps → docs/07 Tasks (reverse map)

| Packet Step | Task IDs Touched | Notes |
| --- | --- | --- |
| 1 — Activation gate | TASK-130, TASK-130a, TASK-130b | Discovery only. |
| 2 — Revert sdk-prepass-guest | TASK-130 | Restores macro coverage for previously-demoted MeshAnalysis tests. Not a TASK-130a/130b motion (those are about paint/mesh seg). |
| 2.5 — Macro paint_seg_arm scope fix | TASK-130a | Bounded two-hunk edit in `build_prepass_world_glue` added in 2026-05-08 packet revision; total churn < 20 lines: line 1317 inline-WIT + Rust `use self::slicer::world_prepass::geometry::{Polygon, Point2};` in segmentation_helpers (mirroring finalization-world pattern at lib.rs:998). Closes the latent compilation bug in commit 46aed61's paint_seg_arm that blocked the original packet 43 path. The WIT-line fix alone is necessary but not sufficient under wit-bindgen 0.24. Paint_seg_arm quote-block at 1814-1829 stays byte-identical. |
| 2.6 — Host layer-idx alignment | TASK-130b | Bounded host edit in `wit_host.rs` (alias to s32; explicit u32 retention for four non-paint records; negative-rejection in push_paint_region validator) and `dispatch.rs:harvest_paint_segmentation_ir` (cast i32→u32 at IR boundary). Closes the host-vs-canonical drift that blocked AC-5/6/7. PaintRegionIR contract unchanged. Added in 2026-05-08 packet revision. |
| 3 — Author paintseg sibling | TASK-130a, TASK-130b | New macro guest for paint segmentation. |
| 4 — Author meshseg sibling | TASK-130a, TASK-130b | New macro guest for mesh segmentation. |
| 5 — Wire into build script | TASK-130a, TASK-130b | Build-system plumbing. |
| 6 — Retarget paint round-trip TDD | TASK-130b | Tests now exercise macro arm via sibling. |
| 7 — Retarget mesh round-trip TDD | TASK-130b | Tests now exercise macro arm via sibling. |
| 8 — Extend freshness + macro-all-worlds registries | TASK-130a, TASK-130b | Macro-arm proof loop now catches future deviation on the two new guests. |
| 9 — docs/05 single-stage section | TASK-130 (housekeeping) | Prevents future packets from repeating the original 43's planning mistake. |
| 10 — Close TASK-130 cluster + DEV-025 mismatch 3 | TASK-130, TASK-130a, TASK-130b | Status flips. |
| 11 — Mark packet 43 superseded | TASK-130 (housekeeping) | Cross-packet mutation rule compliance. |
| 12 — Acceptance ceremony | all | Re-runs all pipe-suffixed AC commands. |

## Aggregate

- 3 backlog task ids closed (TASK-130, TASK-130a, TASK-130b).
- 1 deviation row closed (DEV-025 mismatch 3 — the last open mismatch in DEV-025).
- 1 doc cross-reference updated (docs/14_deviation_audit_history.md).
- 1 prior packet marked superseded (Packet 43).

## Why this packet exists as a separate slice (vs amending Packet 43 in place)

Packet 43's `design.md` carries an embedded incorrect assumption ("a single guest exposes multiple fixtures…one guest .wasm covers both round-trip TDDs") that drove the rejected-alternatives section to dismiss the two-crate approach on scaffolding-economy grounds. Editing Packet 43's `design.md` in place would erase the historical record of why the original approach failed, hiding the planning lesson. A fresh `43-rev1` directory follows the existing project convention (`01-rev1`, `02-rev1`, `12-rev1`, `14-rev1`, `23-rev1`, `36-rev1`, `38-rev1`) and lets future spec authors see both:

1. The original assumption (in Packet 43's `design.md`).
2. The corrective approach + Locked Assumption #1 in this packet's `design.md` (single-stage-per-impl macro constraint).
3. The audit trail (`status: superseded` on 43; `supersedes:` on 43-rev1).
