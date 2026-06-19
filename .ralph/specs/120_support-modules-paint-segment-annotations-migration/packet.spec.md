---
status: draft
packet: 120
task_ids:
  - TASK-261
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-modules-paint-segment-annotations-migration

## Goal

Restore paint-driven `SupportEnforcer` / `SupportBlocker` eligibility behavior in `tree-support`, `traditional-support`, and `support-planner` against the post-P95 IR shape — replace centroid-in-`PaintRegionIR` (the deleted IR) with polygon-intersection against `SlicedRegion.segment_annotations[PaintSemantic::SupportEnforcer | SupportBlocker]`, extract the shared eligibility helper to `slicer_core::paint_policy`, and fix the geometric-correctness bug where an L-shaped enforcer/blocker was previously gated by a single centroid hit.

## Scope Boundaries

Touches `support_paint_policy` (and equivalents) in three support modules + a new shared helper in `slicer-core` + their manifest `[ir-access].reads`. No WIT change, no IR shape change (this packet *consumes* the IR shape P95 + D14 already established). The behavior change is functional: `SupportEnforcer` / `SupportBlocker` paint regions, currently no-op since P95's stub (per the plan's D1), now drive support eligibility correctly. The migration also fixes the original centroid-in-polygon geometric correctness bug as a same-edit improvement (an L-shaped enforcer over an L-shaped region was previously gated by a centroid that may have landed in a hole).

## Prerequisites and Blockers

- Depends on: P95 + P96 + P97 implemented (all confirmed in the codebase survey). `SlicedRegion.segment_annotations[PaintSemantic::SupportEnforcer | SupportBlocker]` must be populated by the paint kernel.
- Unblocks: any future Block C support work that wants enforcer/blocker-aware behavior (e.g., contact placement in `support-planner` adjusting density based on enforcer regions). None of those are explicitly scheduled in this spec block.
- Activation blockers: the implementer must confirm P95's stub for `support_paint_policy` is in place (probably returns `SupportPaintPolicy::DefaultEligible` unconditionally). If P95 instead left the modules non-compiling, the implementer's Step 1 dispatch will surface this; resolution is to land a compile fix first as a sub-step of this packet.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-core/src/paint_policy.rs` (NEW file), **when** parsed, **then** the file defines `pub enum SupportPaintPolicy { Blocked, Enforced, DefaultEligible }` and `pub fn support_eligibility(region: &SliceRegionView) -> SupportPaintPolicy` that returns `Blocked` if the region's `segment_annotations[PaintSemantic::SupportBlocker]` intersects the region polygon non-trivially, `Enforced` if `[PaintSemantic::SupportEnforcer]` does so AND blocker is absent, `DefaultEligible` otherwise. | `rg -q 'pub enum SupportPaintPolicy' crates/slicer-core/src/paint_policy.rs && rg -q 'pub fn support_eligibility' crates/slicer-core/src/paint_policy.rs && cargo test -p slicer-core --test paint_policy 2>&1 | tee target/test-output.log`
- **AC-2. Given** a `SliceRegionView` whose `segment_annotations[SupportBlocker]` covers ≥ 50% of the region polygon area, **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::Blocked`. | `cargo test -p slicer-core --test paint_policy -- blocker_majority_returns_blocked --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a `SliceRegionView` whose `segment_annotations[SupportEnforcer]` covers ≥ 50% of the region polygon area and has no blocker annotation, **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::Enforced`. | `cargo test -p slicer-core --test paint_policy -- enforcer_majority_returns_enforced --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** a `SliceRegionView` with both blocker (covering 30% of region area) and enforcer (covering 40% of region area), **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::Blocked` — blocker wins per the precedence in `docs/01_system_architecture.md` Support Stage Paint Precedence. | `cargo test -p slicer-core --test paint_policy -- blocker_wins_over_enforcer --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** an L-shaped `SliceRegionView` whose centroid lies in the L's notch (outside the polygon) but whose `segment_annotations[SupportEnforcer]` covers the L's vertical arm, **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::Enforced` (NOT `DefaultEligible` — the old centroid-based logic would have been wrong because the centroid is outside the polygon). | `cargo test -p slicer-core --test paint_policy -- enforcer_works_for_l_shape_with_centroid_outside --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** `modules/core-modules/tree-support/src/lib.rs`, **when** searched, **then** the file does NOT define a local `fn support_paint_policy` (the helper has been moved to `slicer-core`), the file imports `slicer_core::paint_policy::SupportPaintPolicy` and `slicer_core::paint_policy::support_eligibility`, and the `run_support` entry point calls the imported helper. | `! rg -q 'fn support_paint_policy' modules/core-modules/tree-support/src/lib.rs && rg -q 'use slicer_core::paint_policy::' modules/core-modules/tree-support/src/lib.rs && rg -q 'support_eligibility\(' modules/core-modules/tree-support/src/lib.rs`
- **AC-7. Given** `modules/core-modules/traditional-support/src/lib.rs`, **when** searched, **then** the file does NOT define a local `fn support_paint_policy`, imports the shared helper, and the `run_support` entry calls it. | `! rg -q 'fn support_paint_policy' modules/core-modules/traditional-support/src/lib.rs && rg -q 'use slicer_core::paint_policy::' modules/core-modules/traditional-support/src/lib.rs && rg -q 'support_eligibility\(' modules/core-modules/traditional-support/src/lib.rs`
- **AC-8. Given** `modules/core-modules/support-planner/src/lib.rs`, **when** searched, **then** `collect_paint_enforcer_contacts` and `collect_paint_blocker_polygons` read from `SlicedRegion.segment_annotations` (via a `SliceRegionView`-equivalent input) rather than from `MeshObjectView.paint_layers.facet_values`. The functions are renamed (or replaced) to reflect the new IR shape — e.g. `collect_enforcer_contacts_from_segment_annotations`. | `! rg -q 'paint_layers\.facet_values' modules/core-modules/support-planner/src/lib.rs && rg -q 'segment_annotations' modules/core-modules/support-planner/src/lib.rs`
- **AC-9. Given** all three module manifests (`tree-support.toml`, `traditional-support.toml`, `support-planner.toml`), **when** searched, **then** the `[ir-access].reads` list contains `"SliceIR"` (or `"SegmentAnnotations"` if a separate IR was committed for them post-P95 — confirm via discovery dispatch) and does NOT contain `"PaintRegionIR"`. | `for m in tree-support traditional-support support-planner; do ! rg -q 'PaintRegionIR' modules/core-modules/$m/$m.toml || { echo "$m still declares PaintRegionIR"; exit 1; }; done`
- **AC-10. Given** an integration fixture with one painted support-enforcer region and `tree-support` loaded, **when** the pipeline runs, **then** support paths are emitted in regions covered by the enforcer even where `SurfaceClassificationIR.needs_support` is `false`. | `cargo test -p slicer-runtime --test live_layer_support_tdd -- enforcer_forces_support_against_classification --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** an integration fixture with one painted support-blocker region overlapping a region where `SurfaceClassificationIR.needs_support` is `true`, **when** the pipeline runs, **then** NO support paths are emitted in the blocker-covered area. | `cargo test -p slicer-runtime --test live_layer_support_tdd -- blocker_suppresses_support_against_classification --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** a fixture with no support enforcer/blocker paint and `SurfaceClassificationIR.needs_support = false` for all regions, **when** the pipeline runs, **then** zero support paths are emitted (paint policy returns `DefaultEligible` AND classification gates the emission to nothing). | `cargo test -p slicer-runtime --test live_layer_support_tdd -- no_paint_no_classification_no_support --nocapture 2>&1 | tee target/test-output.log`

## Verification

- `cargo xtask build-guests --check`
- `cargo test -p slicer-core --test paint_policy 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test live_layer_support_tdd 2>&1 | tee target/test-output.log`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C2 — read directly (≈30 lines). The exact behavior specification.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §D14 — read directly. Documents the modifier-volume support routing to `segment_annotations`.
- `docs/01_system_architecture.md` §"Support Stage Paint Precedence" — read directly (≈30 lines). The blocker-wins-over-enforcer precedence.
- `docs/02_ir_schemas.md` §"SliceIR" + §"SegmentAnnotations" (if a separate section exists post-P95) — range-read for the field paths the new helper consumes.
- `crates/slicer-sdk/src/views.rs::SliceRegionView` — read the `segment_annotations()` accessor signature + return type only.

## Doc Impact Statement (Required)

- `docs/05_module_sdk.md` §"Shared helpers" — add a one-paragraph entry documenting `slicer_core::paint_policy::support_eligibility` as the canonical support-eligibility entry point. Verification: `rg -q 'slicer_core::paint_policy::support_eligibility' docs/05_module_sdk.md`.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- stop reading at 60% context and hand off at 85%

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation.
