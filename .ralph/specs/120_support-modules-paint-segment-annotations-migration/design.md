# Design: support-modules-paint-segment-annotations-migration

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-core/src/paint_policy.rs` (NEW) — `SupportPaintPolicy` enum + `support_eligibility` function + sub-helpers (`polygon_intersects_segment_annotation`, `annotation_area_in_region`).
  - `crates/slicer-core/src/lib.rs` — re-export the new module.
  - `modules/core-modules/tree-support/src/lib.rs` — delete local `fn support_paint_policy`; replace call site with `slicer_core::paint_policy::support_eligibility(&region)`.
  - `modules/core-modules/traditional-support/src/lib.rs` — same change.
  - `modules/core-modules/support-planner/src/lib.rs` — replace `collect_paint_enforcer_contacts` + `collect_paint_blocker_polygons` with new functions sourcing from per-region `segment_annotations`. Update the planner's contact-gathering loop in `plan_for_object` to iterate over `SliceRegionView`s (or whatever per-region input shape is available to the prepass).
- Neighboring tests/fixtures:
  - `crates/slicer-core/tests/paint_policy.rs` (NEW) — AC-1 through AC-5.
  - `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` — extend with AC-10 + AC-N1 + AC-N2.
- OrcaSlicer comparison surface: not consulted by this packet. The new IR shape is Pinch 'n Print's post-P95 design (D14), not an Orca port.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.

- The shared helper sits in `slicer-core` because both host-side prepass code paths AND guest-side layer modules need it. Guests link `slicer-core` already (per the existing dependency graph for `polygon_ops`, etc.). The new module fits the existing pattern.
- `support_eligibility` must be `#[inline]`-able and free of host-only side effects (no `log` calls, no I/O) — guest modules call it inside their layer hot path.
- Polygon intersection uses `slicer_core::polygon_ops::intersection` (or `intersection_ex` if the post-P95 ExPolygon-aware variant landed). The implementer confirms the exact function name via Step 1 dispatch.
- The precedence "Blocker wins" is encoded in the function body, not in the call site. Callers receive a single tri-state result.

## Code Change Surface

- Selected approach: single shared helper in `slicer-core`; three modules consume it; manifest updates aligned with helper usage.
- Exact functions/structs/tests to change:
  - `slicer_core::paint_policy::SupportPaintPolicy` (new enum, three variants).
  - `slicer_core::paint_policy::support_eligibility` (new fn).
  - `tree_support::run_support` (call site change; deletion of `support_paint_policy` local fn).
  - `traditional_support::run_support` (same).
  - `support_planner::collect_paint_enforcer_contacts` and `collect_paint_blocker_polygons` (deleted; replaced with new region-iterating functions, likely `collect_segment_annotations_for_object(&[SliceRegionView]) -> EnforcerContactSet`).
  - `support_planner::plan_for_object` (call-site update to consume the new function's output).
  - Three module manifests (`tree-support.toml`, `traditional-support.toml`, `support-planner.toml`) — `[ir-access].reads` change.
  - New test files + test additions per ACs.
- Rejected alternatives:
  - **Inline the helper in each module instead of moving to `slicer-core`** — rejected: duplication is the gap this packet closes.
  - **Make the helper return `bool` instead of `SupportPaintPolicy`** — rejected: the three-state semantics (Blocked / Enforced / DefaultEligible) is meaningful for downstream code; collapsing to a bool loses the distinction between "no paint" and "blocker says no".
  - **Use centroid-in-paint-region as a fast path before falling back to polygon intersection** — rejected: the centroid bug is exactly what this packet is fixing. Fast paths that re-introduce it are forbidden.

## Files in Scope (read + edit)

The packet edits 4 source files + 3 manifests + 3 new/extended test files (10 total). The count is justified because the migration is structurally three-prong (helper extraction + per-module call-site swap + manifest update) and removing any prong leaves the workspace in a broken half-migrated state.

- `crates/slicer-core/src/paint_policy.rs` — role: shared helper module; expected change: file created.
- `crates/slicer-core/src/lib.rs` — role: re-export; expected change: one line added.
- `crates/slicer-core/tests/paint_policy.rs` — role: AC-1 through AC-5; expected change: file created.
- `modules/core-modules/tree-support/src/lib.rs` — role: helper removal + call-site swap; expected change: ≈30 lines deleted, ≈3 lines added.
- `modules/core-modules/traditional-support/src/lib.rs` — role: same; expected change: same magnitude.
- `modules/core-modules/support-planner/src/lib.rs` — role: contact extraction migration; expected change: two function bodies rewritten (≈60 lines), call site in `plan_for_object` updated.
- `modules/core-modules/tree-support/tree-support.toml` — role: manifest reads update; expected change: 1 line edit.
- `modules/core-modules/traditional-support/traditional-support.toml` — role: same.
- `modules/core-modules/support-planner/support-planner.toml` — role: same.
- `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` — role: AC-10, AC-N1, AC-N2 integration cases; expected change: three test functions added.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C2 — directly.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §D14 — directly.
- `docs/01_system_architecture.md` §"Support Stage Paint Precedence" — directly.
- `docs/02_ir_schemas.md` §"SliceIR" — range-read the `SlicedRegion` + `segment_annotations` definitions.
- `crates/slicer-sdk/src/views.rs::SliceRegionView::segment_annotations` — accessor only.
- `crates/slicer-core/src/polygon_ops.rs` — read `intersection` (line 93-108 area) + `area_ex` (delegate if exact line unknown) signatures only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted.
- `crates/slicer-core/src/algos/paint_segmentation/**` — paint kernel produced by P95; consumer side only.
- `target/`, `Cargo.lock`, generated code — never load.
- Other paint consumers (`fuzzy-skin`, `seam-placer`) — out of scope; do not browse for consistency.
- The full body of `crates/slicer-sdk/src/views.rs` outside the accessor — range-read.
- The full body of `support-planner/src/lib.rs` outside lines around 729 and 755 and the planner's `plan_for_object` site — range-read.

## Expected Sub-Agent Dispatches

- "Locate `SliceRegionView::segment_annotations` accessor in `crates/slicer-sdk/src/views.rs`; return LOCATIONS + SNIPPETS ≤ 20 lines showing the signature." — purpose: confirm helper input type.
- "Confirm whether `crates/slicer-core/src/polygon_ops.rs` defines `intersection_ex` (ExPolygon-aware) or only `intersection` (flat-polygon). Return FACT (which) + file:line." — purpose: choose the right helper for `support_eligibility`.
- "Return current state of `fn support_paint_policy` in `modules/core-modules/tree-support/src/lib.rs` post-P95; SNIPPETS ≤ 30 lines showing the current body (likely a stub returning `DefaultEligible`)." — purpose: confirm Step 1 baseline.
- "Return current state of `collect_paint_enforcer_contacts` + `collect_paint_blocker_polygons` in `modules/core-modules/support-planner/src/lib.rs`; SNIPPETS ≤ 60 lines combined." — purpose: confirm migration target.
- "Return current `[ir-access].reads` values for `tree-support.toml`, `traditional-support.toml`, `support-planner.toml`; FACT (per-manifest list)." — purpose: confirm manifest baseline.
- "Run `cargo test -p slicer-core --test paint_policy`; return FACT (per-test pass/fail); SNIPPETS ≤ 20 lines on failure." — purpose: AC-1 through AC-5 gate.
- "Run `cargo test -p slicer-runtime --test live_layer_support_tdd`; return FACT (per-test pass/fail); SNIPPETS ≤ 30 lines on failure." — purpose: AC-10 / N1 / N2 gate.
- "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list>`). NEVER paste rebuild log." — purpose: WASM gate post manifest changes.

## Data and Contract Notes

- IR contracts touched: none structurally — `SliceRegion.segment_annotations` already exists post-P95. The packet consumes the existing contract.
- WIT boundary considerations: none (the existing prepass + layer interfaces already plumb the segment_annotations into the SDK views).
- Determinism: `support_eligibility` is pure; polygon intersection is deterministic.
- Helper threading model: callable from both host (planner prepass) and guest (layer modules) contexts. Stateless.

## Locked Assumptions and Invariants

- Blocker-wins-over-enforcer precedence per `docs/01_system_architecture.md` is fixed. The helper encodes the order explicitly; downstream callers do not override it.
- "Non-trivial intersection" means area > some small epsilon (suggested: 1e-6 mm² in unscaled mm, i.e. roughly one polygon-op unit²). The implementer picks a defensible epsilon in Step 2 and documents it; AC tests assert against it.
- `support_eligibility` does NOT consult `SurfaceClassificationIR.needs_support`. That fallback is the caller's responsibility (kept exactly as today). The helper returns `DefaultEligible` when there's no paint, and the caller falls back to `needs_support` exactly like today.
- The helper does NOT emit diagnostics. Misuse (e.g., region with no segment_annotations key at all) returns `DefaultEligible`.

## Risks and Tradeoffs

- **Risk**: polygon intersection on every region × every layer adds work the old centroid hit didn't. **Mitigation**: typical region count per layer is modest (single-digit to low-double-digit on real models); Clipper2 intersection is fast at this scale. If profiling later shows hot-spot behavior, a bbox-overlap fast path can be added inside the helper without changing the signature.
- **Risk**: segment_annotations may not be populated for layers where the paint kernel decided there's no paint coverage. The helper must handle empty / missing keys gracefully (return `DefaultEligible`, NOT panic). AC-N2 explicitly tests this.
- **Risk**: the integration tests in `live_layer_support_tdd.rs` require fixtures with painted enforcer/blocker regions. The cube_4color fixtures (P95-era) carry tool/material paint, not support paint. The implementer may need to author a small new fixture (`cube_with_support_enforcer.3mf` or equivalent). **Mitigation**: if the existing `bridge_support_enforcers.3mf` already covers this (P67 referenced), reuse it; otherwise document a fixture-authoring sub-step in Step 7.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 4 — `support-planner` extraction rewrite).
- Highest-risk dispatch: `cargo build --workspace` after the manifest changes — return FACT pass/fail; on fail SNIPPETS ≤ 30 lines with FIRST error.

## Open Questions

- `[FWD]` The exact `[ir-access].reads` key to declare in the three manifests depends on whether segment_annotations is keyed under `SliceIR` or has its own IR name post-P95. Step 1 discovery dispatches the LOCATIONS lookup; the answer becomes a packet-author note before Step 5 (manifest update). Forward-looking because resolution is local to the implementer and does not change the packet shape.
- `[FWD]` The integration-test fixture for AC-10 / AC-N1: reuse `bridge_support_enforcers.3mf` (if it carries the right paint) vs. author a small new fixture. Decision in Step 7 of the implementation plan.
