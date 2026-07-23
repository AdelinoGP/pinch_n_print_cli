# Requirements: support-modules-paint-segment-annotations-migration

## Packet Metadata

- Grouped task IDs:
  - `TASK-285` (renumbered from source-plan `TASK-261`; `TASK-261` is now used by `docs/07_implementation_status.md` for infill-parity integration, packet 136). Source-plan ID recorded for audit.
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

The current `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView::paint_policy_for` (line 172) takes a single `ExPolygon`, computes its vertex-mean centroid via `expolygon_centroid` (line 220), and probes that single point against `SlicedRegion.segment_annotations[PaintSemantic::SupportBlocker | SupportEnforcer]`. P95 deleted `PaintRegionIR` and `paint_region.rs`; `paint_policy_for` was rewritten against the post-P95 `segment_annotations` field, but kept the centroid probe.

Two correctness gaps exist in the current code:

1. **Centroid hits the wrong polygon.** An L-shaped region's vertex-mean centroid often lies in the notch, outside the polygon. Centroid-in-paint-region tests gave the wrong answer for any non-convex shape — the eligibility verdict didn't match the region's actual coverage by the enforcer/blocker.
2. **Per-module helper duplication was never present** (the two modules always consumed the SDK's `paint_policy_for`); but the centroid bug IS in the SDK helper that BOTH modules consume, so a single fix at the SDK layer (or, per the spec, by extracting a shared helper to `slicer-core`) covers both modules.

This packet:

- Extracts a single `support_eligibility` helper to `crates/slicer-core/src/paint_policy.rs` that takes a `SliceRegionView` and reads its `segment_annotations` (post-P95 IR shape per D14 of the paint roadmap). The helper uses `slicer_core::polygon_ops::intersection` (or `intersection_ex` for ExPolygon-aware intersection) to compute area overlap with the enforcer/blocker annotations.
- Replaces the centroid probe with polygon-intersection: any non-trivial area overlap (threshold: `> 1e-6 mm²`) between the enforcer/blocker annotation and the region polygon counts as a hit.
- Refactors `crates/slicer-sdk/src/traits.rs::paint_policy_for` to be a thin compatibility wrapper: iterate the input `SliceIR.regions`, call `support_eligibility` per region, aggregate with blocker-wins precedence. This keeps the call sites in `tree-support` and `traditional-support` unchanged (the `SupportPaintPolicy` enum and the `match` arms don't change).
- Removes the now-unused `expolygon_centroid` and `regions_cover_point` helpers from `crates/slicer-sdk/src/traits.rs` (no other callers — confirmed via `rg -c`).
- Cleans the three module manifests' `[ir-access].reads` to drop `"PaintRegionIR"` (the IR is gone; the strings are dead).
- Cleans the dead shim in `crates/slicer-wasm-host/src/host.rs::HostPaintRegionLayerView` that still pushes `"PaintRegionIR"` into `runtime_reads` (lines 3060, 3084, 3094) and uses kebab-case semantic-name keys (`"support-enforcer"`, `"support-blocker"`, `"fuzzy-skin"`) that violate the snake_case config-key naming convention in `docs/01`.

## In Scope

- Create `crates/slicer-core/src/paint_policy.rs` with `pub enum SupportPaintPolicy` (re-exported from `slicer_sdk::traits` to keep the call-site match arms working) and `pub fn support_eligibility(region: &SliceRegionView) -> SupportPaintPolicy`.
- The helper resolves precedence per `docs/01_system_architecture.md` §"Support Stage Paint Precedence":
  1. `PaintSemantic::SupportBlocker` annotation intersecting the region polygon with non-trivial area → `Blocked`.
  2. Else `PaintSemantic::SupportEnforcer` annotation intersecting the region polygon with non-trivial area → `Enforced`.
  3. Else → `DefaultEligible`.
- "Intersecting with non-trivial area" means `polygon_ops::intersection(region.polygons, annotation_polys).area() > 1e-6 mm²` after `mm_to_units` rounding. NOT a centroid hit.
- Re-export `SupportPaintPolicy` from `crates/slicer-core` (under `slicer_core::paint_policy::SupportPaintPolicy`) AND keep the `slicer_sdk::traits::SupportPaintPolicy` alias so the `tree-support` / `traditional-support` match arms continue to compile without source edits.
- Author `crates/slicer-core/tests/paint_policy.rs` with five tests (AC-1 grep pairs with this file; AC-2 through AC-5 are the polygon-coverage cases + AC-N3 empty-annotations graceful return).
- Refactor `crates/slicer-sdk/src/traits.rs::paint_policy_for` to be a thin wrapper. The `match` arms in the two modules don't change.
- Delete `expolygon_centroid` and `regions_cover_point` from `crates/slicer-sdk/src/traits.rs` (the new helper subsumes their role; no other callers in the workspace per `rg -c`).
- Add `enforcer_works_when_centroid_outside_paint_region` to BOTH `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` and `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs`. The test uses an L-shaped expoly whose vertex-mean centroid lies outside the painted region; the test must FAIL against the pre-packet centroid-based helper (RED) and PASS after Step 3 lands (GREEN).
- Update `tree-support.toml`, `traditional-support.toml`, `support-planner.toml` `[ir-access].reads`: drop `"PaintRegionIR"`. The post-P95 source (`"SliceIR"` for tree/traditional; `"MeshIR"` for planner, since planner reads the per-facet mesh directly) is already declared.
- Clean `crates/slicer-wasm-host/src/host.rs::HostPaintRegionLayerView`: stop pushing `"PaintRegionIR"` into `runtime_reads`; switch the kebab-case semantic-name keys to snake_case (`"support_enforcer"`, `"support_blocker"`, `"fuzzy_skin"`).
- Update `docs/05_module_sdk.md` per Doc Impact Statement.

## Out of Scope

- Changes to the paint kernel itself (`paint_segmentation` and friends are P95/P96 territory, closed).
- Migration of OTHER modules' (e.g., `fuzzy-skin`, `seam-placer`) paint consumption — this packet is support-specific.
- Adding new paint semantics (e.g., `SupportRoofEnforcer`) — out of spec.
- Reworking `tree-support`'s `support_plan_segments_for` consumption path — that's the planner→layer contract, separate concern.
- Geometric performance optimizations on the polygon intersection — Clipper2-backed `intersection_ex` is fast enough for the per-region cardinalities expected.
- Deleting `crates/slicer-sdk/src/traits.rs::PaintRegionLayerView` itself — the wrapper around `paint_policy_for` is the compatibility seam for any future caller; only the centroid helpers are removed.

## Authoritative Docs

- `docs/specs/support-modules-orca-port.md` §C2 — directly.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §D14 — directly.
- `docs/01_system_architecture.md` §"Support Stage Paint Precedence" — directly.
- `docs/02_ir_schemas.md` §"SliceIR" — range-read the `SlicedRegion` definition (lines 1347-1401 of `crates/slicer-ir/src/slice_ir.rs`).
- `docs/05_module_sdk.md` — range-read for the "Shared helpers" section the Doc Impact Statement adds to.
- `crates/slicer-sdk/src/views.rs::SliceRegionView::segment_annotations` — read the accessor signature only (line 368).
- `crates/slicer-sdk/src/traits.rs::paint_policy_for` — read lines 172-240 (the function being replaced + its helpers).
- `crates/slicer-core/src/polygon_ops.rs` — read `intersection` (line 93-108 area) + `area_ex` (delegate if exact line unknown) signatures only.

## Acceptance Summary

- Positive cases: `AC-1` through `AC-10` from `packet.spec.md`.
  - AC-1 through AC-5 + AC-N3 gate the new shared helper.
  - AC-6 gates the SDK-side refactor (no more centroid logic; thin wrapper).
  - AC-7 confirms the per-module call sites are unchanged in shape.
  - AC-8 gates the per-module L-shape regression test in `enforcer_blocker_tdd.rs`.
  - AC-9 gates the manifest cleanup.
  - AC-10 gates the live integration path (enforcer forces support against classification).
- Negative cases: AC-N1 (blocker suppresses support), AC-N2 (no paint + no classification = no support), AC-N3 (empty annotations → DefaultEligible), AC-N4 (dead host shim cleaned; kebab-case keys replaced).
- Cross-packet impact: future Block C work that reads `segment_annotations` for new semantics reuses the helper pattern (new function in `slicer_core::paint_policy`).

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo xtask build-guests --check` | Guest WASM current before integration tests. | FACT pass/fail |
| `cargo build --workspace` | Workspace compiles after src + manifest changes. | FACT pass/fail |
| `cargo test -p slicer-core --test paint_policy 2>&1 \| tee target/test-output.log` | AC-1 through AC-5 + AC-N3 unit tests. | FACT pass/fail; SNIPPETS ≤ 20 lines on failure |
| `cargo test -p tree-support --test enforcer_blocker_tdd 2>&1 \| tee target/test-output.log` | AC-8 RED→GREEN + existing 8 tests. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `cargo test -p traditional-support --test enforcer_blocker_tdd 2>&1 \| tee target/test-output.log` | AC-8 mirror + existing 8 tests. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `cargo test -p slicer-runtime --test live_layer_support_tdd 2>&1 \| tee target/test-output.log` | AC-10 + AC-N1 + AC-N2 integration. | FACT pass/fail; SNIPPETS ≤ 30 lines on failure |
| `! rg -q 'expolygon_centroid\|regions_cover_point' crates/slicer-sdk/src/traits.rs` | AC-6 helpers deleted. | FACT pass/fail |
| `rg -q 'slicer_core::paint_policy::support_eligibility' crates/slicer-sdk/src/traits.rs` | AC-6 wrapper uses new helper. | FACT pass/fail |
| `! rg -q 'PaintRegionIR' modules/core-modules/tree-support/tree-support.toml modules/core-modules/traditional-support/traditional-support.toml modules/core-modules/support-planner/support-planner.toml` | AC-9 manifests cleaned. | FACT pass/fail |
| `! rg -q 'PaintRegionIR\|"support-enforcer"\|"support-blocker"\|"fuzzy-skin"' crates/slicer-wasm-host/src/host.rs` | AC-N4 host shim cleaned. | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Workspace lint gate. | FACT pass/fail |

## Step Completion Expectations

- The refactor of `crates/slicer-sdk/src/traits.rs::paint_policy_for` is FUNCTIONAL: the new body is a thin wrapper that iterates `SliceIR.regions`, calls `support_eligibility` per region, aggregates with blocker-wins precedence. The two consumer modules' `match` arms are unchanged because the enum and its variants are unchanged.
- The L-shape regression test (AC-8) is RED on the pre-packet code and GREEN after Step 3. The implementer MUST confirm RED on the unfixed code BEFORE merging the fix; a GREEN-on-first-try indicates the test isn't actually exercising the centroid bug.
- The `[ir-access].reads` manifest change in Step 5 does NOT require a `cargo xtask build-guests` rebuild IF the manifest is consumed only at the host level (not fed to bindgen). Confirm via Step 1 dispatch; if the manifest IS read by the build scripts, the implementer must run `cargo xtask build-guests --check` (and rebuild if STALE) before AC-10/AC-N1/AC-N2 to avoid stale-guest attribution.
- The `crates/slicer-wasm-host/src/host.rs` cleanup is the most likely silent-failure surface: the `HostPaintRegionLayerView` impl is host-side glue called by guest modules, so a stale `runtime_reads.push("PaintRegionIR")` would surface as a `runtime_reads` mismatch in dispatch contract tests, NOT in the unit tests for the new helper. Step 6 must include `cargo test -p slicer-wasm-host` in its verification.

## Context Discipline Notes

- Large files in the read-only path that MUST be ranged or delegated:
  - `crates/slicer-sdk/src/traits.rs` — read only lines 1-50, 165-245, 1200-1713. 1713 lines total; range-read tightly.
  - `crates/slicer-wasm-host/src/host.rs` — file is 3000+ lines; range-read the `HostPaintRegionLayerView` impl block at lines 3054-3120 only.
  - `crates/slicer-sdk/src/views.rs` — read only the `SliceRegionView::segment_annotations` accessor (line 368) + `needs_support` accessor (line 264) + the struct definition (lines 19-79).
  - `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` — 1406 lines; range-read the setup pattern for the three AC-10/AC-N1/AC-N2 tests (lines 200-380 cover all three). Do NOT read the full file.
  - `crates/slicer-core/src/polygon_ops.rs` — read only the signatures the helper needs (`intersection` line 93-108, `area_ex`).
- Likely temptation reads (skip these):
  - The paint kernel itself (`crates/slicer-core/src/algos/paint_segmentation/`) — produced by P95; the consumer side (this packet) only needs the contract surface, not the implementation.
  - All other consumers of `SliceRegionView.segment_annotations` (e.g., fuzzy-skin) — out of scope; do not browse them for "consistency."
  - `OrcaSlicerDocumented/**` — the migration is project-internal.
- Sub-agent return-format hints for heaviest dispatches:
  - `cargo build --workspace` post-change — FACT pass/fail; SNIPPETS ≤ 30 lines on FIRST error.
  - `cargo xtask build-guests --check` — FACT (`up to date` or `STALE: <list>`); NEVER paste rebuild log.
  - LOCATIONS for `SliceRegionView::segment_annotations` — file:line + 1-line context, ≤ 5 entries.
