# Task Map: support-modules-paint-segment-annotations-migration

| docs/07 task ID | Packet step | Primary docs | Expected code surface | OrcaSlicer refs | Context cost | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `TASK-285` (renumbered from source-plan `TASK-261`; collision: `TASK-261` is now infill-parity integration per `docs/07_implementation_status.md:226`) | Step 1, Step 2, Step 3, Step 4, Step 5, Step 6, Step 7, Step 8 | `docs/specs/support-modules-orca-port.md` §C2 + `docs/specs/paint-pipeline-orca-parity-roadmap.md` §D14 + `docs/01_system_architecture.md` §"Support Stage Paint Precedence" | `crates/slicer-core/src/paint_policy.rs` (NEW; `SupportPaintPolicy` re-export + `support_eligibility`); `crates/slicer-core/src/lib.rs` (one-line module export); `crates/slicer-sdk/src/traits.rs` (line 172 `paint_policy_for` refactored to thin wrapper; `expolygon_centroid` + `regions_cover_point` deleted at lines 220/238); `crates/slicer-wasm-host/src/host.rs` (`HostPaintRegionLayerView` lines 3054-3120: kebab→snake semantic-name keys; three `runtime_reads.push("PaintRegionIR")` deleted); three module manifests (`[ir-access].reads` drops `"PaintRegionIR"`); `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` + `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs` (one new L-shape regression test per file); `crates/slicer-core/tests/paint_policy.rs` (NEW; 5 unit tests); `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` (NO source change — the 3 integration tests at lines 200/361/236 already exist; this packet verifies they still pass); `docs/05_module_sdk.md` (one-paragraph Shared helpers entry). | none (project-internal migration, not an Orca port) | M | The geometric correctness fix is the load-bearing item. The two existing enforcer/blocker test fixtures in `enforcer_blocker_tdd.rs` use 10 mm squares inside 20 mm enclosing painted regions, so the centroid bug does NOT manifest — the new L-shape test is RED on the pre-packet code and GREEN after Step 3's SDK refactor. |

Aggregate context cost across rows: `M`. No row is L.

## Source-plan ID crosswalk

| Source plan | Renumbered to | Reason |
| --- | --- | --- |
| `TASK-261` (proposed by `docs/specs/support-modules-orca-port.md:506`) | `TASK-285` | `TASK-261` is now closed-tracked for infill-parity integration (`docs/07_implementation_status.md:226`, packet 136). |

The original TASK IDs 250-255 (Bucket B) and 260-266 (Bucket C) in the support spec are reused for unrelated work in the current `docs/07`. Bucket B was already renumbered to TASK-281/TASK-282 (B5/B6, both closed 2026-07-19) and to TASK-163b-diagnostic (B2/B4/B7, closed 2026-07-19). Bucket C1-C5 still need renumbering; this packet establishes TASK-285 as the first C-block renumber. Packets 121-124 (smooth-nodes, multi-neighbour-MST, to-buildplate, raft-plan) renumber to TASK-286..289 per their own task-map.md files.
