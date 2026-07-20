---
status: draft
packet: 120
task_ids:
  - TASK-285
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: support-modules-paint-segment-annotations-migration

## Goal

Replace the centroid-based paint-eligibility logic in `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for` (which currently takes a single `ExPolygon`, computes its vertex-mean centroid, and probes that point against the post-P95 `SlicedRegion.segment_annotations[PaintSemantic::SupportEnforcer | SupportBlocker]`) with a polygon-intersection-based eligibility check that takes a `SliceRegionView` and verifies non-trivial area overlap between the region's polygon and the painted annotation. The fix is geometric correctness — an L-shaped enforcer overlapping the L's vertical arm was previously gated by a centroid that lay in the L's notch (outside the polygon) and produced `DefaultEligible` instead of `Enforced`. Extract the fixed helper to a new `crates/slicer-core/src/paint_policy` module so both host-side (`tree-support`, `traditional-support`) consumers share one implementation. Clean the three module manifests' stale `PaintRegionIR` reads (the IR was deleted by packet 95; the strings are dead).

## Scope Boundaries

Touches `crates/slicer-sdk/src/traits.rs` (replace the centroid-based body of `paint_policy_for` with a call into the new shared helper), `crates/slicer-core/src/paint_policy.rs` (NEW), `crates/slicer-core/src/lib.rs` (re-export), the three module manifests (drop `"PaintRegionIR"` from `[ir-access].reads`), and `crates/slicer-wasm-host/src/host.rs` (the `HostPaintRegionLayerView` shim still pushes `"PaintRegionIR"` into `runtime_reads` at lines 3060/3084/3094 — that is dead; remove). No IR shape change. No WIT change. The new `support_eligibility` helper takes a `SliceRegionView`; `paint_policy_for` becomes a thin compatibility wrapper that iterates the regions of the input expoly's parent `SliceIR` and aggregates the per-region result (Blocked wins, then Enforced, then DefaultEligible — same precedence as today).

The centroid-based test fixtures in `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` and `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs` use a 10 mm square expoly inside a 20 mm enclosing painted square; the expoly centroid `(0,0)` falls inside the painted region so the centroid probe currently passes those tests. **Those tests do NOT exercise the bug.** This packet adds one new test per module (L-shaped enforcer case) that exposes the centroid regression; the existing 8 tests per file continue to pass.

## Prerequisites and Blockers

- Depends on: P95 + P96 + P97 (TASK-245, TASK-246) implemented and `SlicedRegion.segment_annotations` populated by the paint kernel. Confirmed in the codebase survey (`crates/slicer-ir/src/slice_ir.rs:1365`; `crates/slicer-sdk/src/views.rs:368` accessor; kernel wired through `crates/slicer-sdk/src/traits.rs:185,192`).
- Unblocks: any future Block C support work that consumes enforcer/blocker annotations via the new helper.
- Activation blockers: none beyond P95. The current `paint_policy_for` already returns `DefaultEligible` for the no-annotation case; the packet's replacement preserves that path.

## Acceptance Criteria

- **AC-1. Given** `crates/slicer-core/src/paint_policy.rs` (NEW file), **when** parsed, **then** the file defines `pub enum SupportPaintPolicy { Blocked, Enforced, DefaultEligible }` and `pub fn support_eligibility(region: &SliceRegionView) -> SupportPaintPolicy` that returns `Blocked` if the region's `segment_annotations[PaintSemantic::SupportBlocker]` intersects the region polygon with non-trivial area, `Enforced` if `[PaintSemantic::SupportEnforcer]` does so AND blocker is absent, `DefaultEligible` otherwise. The "non-trivial area" threshold is `> 1e-6 mm²` (≈ one polygon-op unit² after `mm_to_units` rounding). | `rg -q 'pub enum SupportPaintPolicy' crates/slicer-core/src/paint_policy.rs && rg -q 'pub fn support_eligibility' crates/slicer-core/src/paint_policy.rs && cargo test -p slicer-core --test paint_policy 2>&1 | tee target/test-output.log`
- **AC-2. Given** a `SliceRegionView` whose `segment_annotations[SupportBlocker]` covers ≥ 50% of the region polygon area, **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::Blocked`. | `cargo test -p slicer-core --test paint_policy -- blocker_majority_returns_blocked --nocapture 2>&1 | tee target/test-output.log`
- **AC-3. Given** a `SliceRegionView` whose `segment_annotations[SupportEnforcer]` covers ≥ 50% of the region polygon area and has no blocker annotation, **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::Enforced`. | `cargo test -p slicer-core --test paint_policy -- enforcer_majority_returns_enforced --nocapture 2>&1 | tee target/test-output.log`
- **AC-4. Given** a `SliceRegionView` with both blocker (covering 30% of region area) and enforcer (covering 40% of region area), **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::Blocked` — blocker wins per the precedence in `docs/01_system_architecture.md` Support Stage Paint Precedence. | `cargo test -p slicer-core --test paint_policy -- blocker_wins_over_enforcer --nocapture 2>&1 | tee target/test-output.log`
- **AC-5. Given** an L-shaped `SliceRegionView` whose vertex-mean centroid lies in the L's notch (outside the polygon) but whose `segment_annotations[SupportEnforcer]` covers the L's vertical arm, **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::Enforced` (NOT `DefaultEligible` — the old centroid-based `paint_policy_for` would have returned `DefaultEligible` because the centroid lies outside the painted region). | `cargo test -p slicer-core --test paint_policy -- enforcer_works_for_l_shape_with_centroid_outside_polygon --nocapture 2>&1 | tee target/test-output.log`
- **AC-6. Given** `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for`, **when** parsed, **then** the function body is a thin wrapper that iterates over the input `SliceIR.regions` whose polygon covers `expoly`, calls `slicer_core::paint_policy::support_eligibility` per region, and aggregates via the same blocker-wins precedence. The `expolygon_centroid` and `regions_cover_point` helpers are deleted (no other callers — confirm via `rg -c 'expolygon_centroid\|regions_cover_point' crates/`). | `! rg -q 'expolygon_centroid' crates/slicer-sdk/src/traits.rs && ! rg -q 'regions_cover_point' crates/slicer-sdk/src/traits.rs && rg -q 'slicer_core::paint_policy::support_eligibility' crates/slicer-sdk/src/traits.rs`
- **AC-7. Given** `modules/core-modules/tree-support/src/lib.rs` and `modules/core-modules/traditional-support/src/lib.rs`, **when** parsed, **then** the `match paint.paint_policy_for(expoly) { ... }` call sites are unchanged in shape (still consume `SupportPaintPolicy::{Blocked, Enforced, DefaultEligible}`) — the helper's signature and the enum are unchanged; only the implementation moved. | `rg -q 'paint\.paint_policy_for' modules/core-modules/tree-support/src/lib.rs && rg -q 'paint\.paint_policy_for' modules/core-modules/traditional-support/src/lib.rs && rg -q 'SupportPaintPolicy::Blocked' modules/core-modules/tree-support/src/lib.rs && rg -q 'SupportPaintPolicy::Blocked' modules/core-modules/traditional-support/src/lib.rs`
- **AC-8. Given** `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` and `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs`, **when** searched, **then** a NEW test function `enforcer_works_when_centroid_outside_paint_region` exists in each file using an L-shaped expoly whose vertex-mean centroid lies outside the painted region but whose body overlaps the enforcer. The test must FAIL against the pre-packet centroid-based `paint_policy_for` (RED on the old logic) and PASS after Step 3 lands. The existing 8 tests in each file continue to PASS. | `rg -q 'fn enforcer_works_when_centroid_outside_paint_region' modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs && rg -q 'fn enforcer_works_when_centroid_outside_paint_region' modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs && cargo test -p tree-support --test enforcer_blocker_tdd 2>&1 | tee target/test-output.log && cargo test -p traditional-support --test enforcer_blocker_tdd 2>&1 | tee target/test-output.log`
- **AC-9. Given** all three module manifests (`tree-support.toml`, `traditional-support.toml`, `support-planner.toml`), **when** searched, **then** the `[ir-access].reads` list does NOT contain the string `"PaintRegionIR"`. The post-P95 source `"SliceIR"` is already present in `tree-support` and `traditional-support`; `support-planner` continues to read `"MeshIR"` (it operates on the per-facet mesh, not on slice regions — leave it). | `for m in tree-support traditional-support support-planner; do ! rg -q 'PaintRegionIR' modules/core-modules/$m/$m.toml || { echo "$m still declares PaintRegionIR"; exit 1; }; done`
- **AC-10. Given** the existing live integration test `enforcer_forces_live_support_commit_even_when_needs_support_is_false` in `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs`, **when** run, **then** it PASSES against the new helper. The test was authored under the centroid-based `paint_policy_for` and uses the `bridge_support_enforcers.3mf` fixture; the new helper must continue to return `Enforced` for that fixture's regions. | `cargo test -p slicer-runtime --test live_layer_support_tdd -- enforcer_forces_live_support_commit_even_when_needs_support_is_false --nocapture 2>&1 | tee target/test-output.log`

## Negative Test Cases

- **AC-N1. Given** the existing live integration test `blocker_overrides_needs_support_true_at_commit_level`, **when** run, **then** it PASSES — the new helper preserves blocker-wins precedence. | `cargo test -p slicer-runtime --test live_layer_support_tdd -- blocker_overrides_needs_support_true_at_commit_level --nocapture 2>&1 | tee target/test-output.log`
- **AC-N2. Given** the existing live integration test `disabled_or_ineligible_support_stage_commits_empty_support_ir`, **when** run, **then** it PASSES — the new helper returns `DefaultEligible` when no `segment_annotations` key is present, and the caller's `needs_support() = false` gate short-circuits to zero support paths. | `cargo test -p slicer-runtime --test live_layer_support_tdd -- disabled_or_ineligible_support_stage_commits_empty_support_ir --nocapture 2>&1 | tee target/test-output.log`
- **AC-N3. Given** a `SliceRegionView` whose `segment_annotations` map is empty (no `SupportBlocker` key, no `SupportEnforcer` key), **when** `support_eligibility(&region)` is called, **then** the return value is `SupportPaintPolicy::DefaultEligible` (graceful empty handling, no panic). | `cargo test -p slicer-core --test paint_policy -- empty_segment_annotations_returns_default_eligible --nocapture 2>&1 | tee target/test-output.log`
- **AC-N4. Given** `crates/slicer-wasm-host/src/host.rs::HostPaintRegionLayerView`, **when** searched, **then** no method body pushes the string `"PaintRegionIR"` into `self.runtime_reads` (the host-side shim that remained after packet 95 is now dead; clean it). The kebab-case semantic-name keys at lines 3063-3066 (`"support-enforcer"`, `"support-blocker"`) are replaced with snake_case (`"support_enforcer"`, `"support_blocker"`) per the `docs/01` config-key naming convention. | `! rg -q 'PaintRegionIR' crates/slicer-wasm-host/src/host.rs && ! rg -q '"support-enforcer"\|"support-blocker"\|"fuzzy-skin"' crates/slicer-wasm-host/src/host.rs`

## Verification

- `cargo xtask build-guests --check`
- `cargo build --workspace`
- `cargo test -p slicer-core --test paint_policy 2>&1 | tee target/test-output.log`
- `cargo test -p tree-support --test enforcer_blocker_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p traditional-support --test enforcer_blocker_tdd 2>&1 | tee target/test-output.log`
- `cargo test -p slicer-runtime --test live_layer_support_tdd 2>&1 | tee target/test-output.log`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C2 — read directly (≈30 lines). The exact behavior specification.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §D14 — read directly. Documents the modifier-volume support routing to `segment_annotations`.
- `docs/01_system_architecture.md` §"Support Stage Paint Precedence" — read directly (≈30 lines). The blocker-wins-over-enforcer precedence.
- `docs/02_ir_schemas.md` §"SliceIR" — range-read the `SlicedRegion` definition (lines 1347-1401 in `crates/slicer-ir/src/slice_ir.rs`).
- `crates/slicer-sdk/src/views.rs::SliceRegionView` — read the `segment_annotations()` accessor (line 368) + `needs_support()` accessor (line 264) only.
- `crates/slicer-sdk/src/traits.rs` lines 172-240 — the `paint_policy_for` body and its centroid helpers being replaced.

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
