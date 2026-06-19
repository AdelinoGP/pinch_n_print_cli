# Requirements: support-modules-paint-segment-annotations-migration

## Packet Metadata

- Grouped task IDs:
  - `TASK-261` — Migrate `support_paint_policy` to `SlicedRegion.segment_annotations` post-P95 (C2 from `docs/specs/support-modules-orca-port.md`)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

Before P95, the three support modules read `PaintRegionIR` and called `slicer_core::paint_region::point_in_paint_region(...)` at the polygon centroid to gate support emission. P95 deleted `PaintRegionIR` and `paint_region.rs`; per its plan, the modules were stubbed to return `SupportPaintPolicy::DefaultEligible` unconditionally, breaking paint-driven enforcer/blocker behavior for the duration of this packet's runway.

Two correctness gaps existed in the pre-P95 code that this migration fixes simultaneously:

1. **Centroid hits the wrong polygon.** An L-shaped region's centroid often lies in the notch, outside the polygon. Centroid-in-paint-region tests gave the wrong answer for any non-convex shape — the eligibility verdict didn't match the region's actual coverage by the enforcer/blocker.
2. **Per-module duplication.** `tree-support` and `traditional-support` shipped byte-for-byte copies of `support_paint_policy`. Future bugs would have to be fixed in two places.

This packet:

- Extracts a single `support_eligibility` helper to `slicer_core::paint_policy` that takes a `SliceRegionView` and reads its `segment_annotations` (post-P95 IR shape per D14 of the paint roadmap).
- Replaces centroid-hit with polygon-intersection (any non-trivial area overlap between the enforcer/blocker annotation and the region polygon counts as a hit).
- Removes the per-module helper from `tree-support` and `traditional-support`; both call the shared helper.
- Updates `support-planner`'s enforcer-contact and blocker-polygon extraction to read from `SliceRegionView.segment_annotations` instead of `MeshObjectView.paint_layers.facet_values` (which is now fed by the corrected paint kernel, but the planner's per-facet path is no longer needed since segment_annotations carries the polygon-level data directly).
- Updates the three module manifests' `[ir-access].reads` to drop `PaintRegionIR` and declare the post-P95 source.

## In Scope

- Create `crates/slicer-core/src/paint_policy.rs` with `pub enum SupportPaintPolicy` and `pub fn support_eligibility(region: &SliceRegionView) -> SupportPaintPolicy`.
- The helper resolves precedence per `docs/01_system_architecture.md` §"Support Stage Paint Precedence":
  1. `PaintSemantic::SupportBlocker` annotation intersecting the region polygon → `Blocked`.
  2. Else `PaintSemantic::SupportEnforcer` intersecting the region polygon → `Enforced`.
  3. Else → `DefaultEligible`.
- "Intersecting" means the intersection has non-zero area (`area > epsilon` after rounding) — NOT a centroid hit.
- Author `crates/slicer-core/tests/paint_policy.rs` with five tests (AC-1 grep is paired with this file; AC-2 through AC-5 are the polygon-coverage cases).
- Delete `fn support_paint_policy` from `tree-support` and `traditional-support`; both import and call `slicer_core::paint_policy::support_eligibility`.
- Replace `support-planner::collect_paint_enforcer_contacts` and `collect_paint_blocker_polygons` (currently reading `MeshObjectView.paint_layers.facet_values`) with new functions that source from `SliceRegionView.segment_annotations` per-region. The function signatures change; their callers in `plan_for_object` are updated accordingly.
- Update `tree-support.toml`, `traditional-support.toml`, `support-planner.toml` `[ir-access].reads`: drop `"PaintRegionIR"`; declare the correct post-P95 source for `segment_annotations` (likely `"SliceIR"` because segment_annotations is inlined into `SliceRegion` per the paint roadmap D8 + D14 — confirm exact key via discovery dispatch).
- Author `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` AC-10 + AC-N1 + AC-N2 enforcer/blocker integration cases (additions to the existing test file).
- Update `docs/05_module_sdk.md` per Doc Impact Statement.

## Out of Scope

- Changes to the paint kernel itself (`paint_segmentation` and friends are P95/P96 territory).
- Migration of OTHER modules' (e.g., `fuzzy-skin`, `seam-placer`) paint consumption — this packet is support-specific.
- Adding new paint semantics (e.g., `SupportRoofEnforcer`) — out of spec.
- Reworking `tree-support`'s `support_plan_segments_for` consumption path — that's the planner→layer contract, separate concern.
- Geometric performance optimizations on the polygon intersection — Clipper2-backed `intersection_ex` is fast enough for the per-region cardinalities expected.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C2 — directly.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §D14 — directly.
- `docs/01_system_architecture.md` §"Support Stage Paint Precedence" — directly.
- `docs/02_ir_schemas.md` §"SliceIR" — range-read the `SlicedRegion` definition.
- `docs/05_module_sdk.md` — range-read for the "Shared helpers" section the Doc Impact Statement adds to.
- `crates/slicer-sdk/src/views.rs::SliceRegionView::segment_annotations` — read the accessor signature only.
- `crates/slicer-core/src/polygon_ops.rs` — read `intersection`, `area_ex` (or equivalent) signatures — the helper needs these.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-10` from `packet.spec.md`.
  - AC-1 through AC-5 gate the new shared helper.
  - AC-6, AC-7, AC-8 gate the per-module removal of the local helper + adoption of the shared one.
  - AC-9 gates the manifest changes.
  - AC-10 gates the live integration path (enforcer forces support against classification).
- Negative cases: AC-N1 (blocker suppresses support), AC-N2 (no paint + no classification = no support).
- Cross-packet impact: future Block C work that reads `segment_annotations` for new semantics reuses the helper pattern (new function in `slicer_core::paint_policy`).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Guest WASM current before integration tests. | FACT pass/fail |
| `cargo build --workspace` | Workspace compiles after src + manifest changes. | FACT pass/fail |
| `cargo test -p slicer-core --test paint_policy 2>&1 \| tee target/test-output.log` | AC-1 through AC-5 unit tests. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p slicer-runtime --test live_layer_support_tdd 2>&1 \| tee target/test-output.log` | AC-10 + AC-N1 + AC-N2 integration. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `! rg -q 'fn support_paint_policy' modules/core-modules/tree-support/src/lib.rs` | AC-6 helper removed. | FACT pass/fail |
| `! rg -q 'fn support_paint_policy' modules/core-modules/traditional-support/src/lib.rs` | AC-7 helper removed. | FACT pass/fail |
| `! rg -q 'paint_layers\.facet_values' modules/core-modules/support-planner/src/lib.rs` | AC-8 planner no longer reads facet_values. | FACT pass/fail |
| `rg -q 'segment_annotations' modules/core-modules/support-planner/src/lib.rs` | AC-8 planner reads new shape. | FACT pass/fail |
| `for m in tree-support traditional-support support-planner; do ! rg -q 'PaintRegionIR' modules/core-modules/$m/$m.toml \|\| { echo "$m"; exit 1; }; done` | AC-9 manifests cleaned. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint gate. | FACT pass/fail |

## Step Completion Expectations

- The migration of `support-planner`'s contact extraction is functionally larger than the tree/traditional helper swap — it changes the data flow from per-facet (mesh-level) to per-region polygon (slice-level). The new code consumes `SliceRegionView.segment_annotations`, which already exists per layer + per region. The implementer MUST NOT introduce a temporary parallel path; the old `collect_paint_enforcer_contacts` and `collect_paint_blocker_polygons` are deleted as part of Step 4, not deprecated.
- The `[ir-access].reads` manifest change in Step 5 triggers `cargo xtask build-guests`. The implementer must run that BEFORE running the integration tests in AC-10/N1/N2 (which require fresh guest WASM).
- AC-1 specifies the function signature `support_eligibility(region: &SliceRegionView) -> SupportPaintPolicy`. If during Step 2 the implementer discovers that `SliceRegionView` is not the right input type (e.g., the segment_annotations accessor returns a different ownership pattern than the helper needs), they document the signature deviation in `design.md` §Open Questions and proceed with the closest possible shape; the deviation must not silently widen the helper's input surface.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `crates/slicer-sdk/src/views.rs` — read only `SliceRegionView::segment_annotations` accessor. Delegate LOCATIONS if not at expected line.
  - `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` — existing tests; range-read the setup pattern for the new AC-10 / N1 / N2 cases. Do NOT read the full file.
  - `modules/core-modules/support-planner/src/lib.rs` — 1,000+ lines; range-read around `collect_paint_enforcer_contacts` (currently line 729 area) and `collect_paint_blocker_polygons` (currently line 755 area) only.
  - `crates/slicer-core/src/polygon_ops.rs` — read only the signatures the helper needs (intersection, area).
- Likely temptation reads (skip these):
  - The paint kernel itself (`crates/slicer-core/src/algos/paint_segmentation/`) — produced by P95; the consumer side (this packet) only needs the contract surface, not the implementation.
  - All other consumers of `SliceRegionView.segment_annotations` (e.g., fuzzy-skin) — out of scope; do not browse them for "consistency."
  - `OrcaSlicerDocumented/**` — the migration is project-internal.
- Sub-agent return-format hints for heaviest dispatches:
  - `cargo build --workspace` post-change — FACT pass/fail; SNIPPETS ≤ 30 lines on FIRST error.
  - `cargo xtask build-guests --check` — FACT (`up to date` or `STALE: <list>`); NEVER paste rebuild log.
  - LOCATIONS for `SliceRegionView::segment_annotations` — file:line + 1-line context, ≤ 5 entries.
