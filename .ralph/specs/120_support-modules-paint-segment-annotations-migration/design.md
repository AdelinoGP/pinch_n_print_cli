# Design: support-modules-paint-segment-annotations-migration

## Controlling Code Paths

- Primary code paths:
  - `crates/slicer-core/src/paint_policy.rs` (NEW) — `SupportPaintPolicy` enum (re-export of `slicer_sdk::traits::SupportPaintPolicy`) + `support_eligibility` function + sub-helpers (`polygon_intersects_segment_annotation`, `annotation_area_in_region`).
  - `crates/slicer-core/src/lib.rs` — re-export the new module.
  - `crates/slicer-sdk/src/traits.rs` (line 172) — `paint_policy_for` body becomes a thin wrapper that iterates `SliceIR.regions`, calls `support_eligibility`, and aggregates with blocker-wins precedence. The `expolygon_centroid` and `regions_cover_point` helpers (lines 220, 238) are deleted.
  - `crates/slicer-wasm-host/src/host.rs` (lines 3054-3120) — `HostPaintRegionLayerView` impl: stop pushing `"PaintRegionIR"` into `runtime_reads`; replace kebab-case semantic-name keys with snake_case.
  - The two support modules' `match paint.paint_policy_for(expoly) { ... }` call sites at `modules/core-modules/tree-support/src/lib.rs:176` and `modules/core-modules/traditional-support/src/lib.rs:155` are **unchanged in shape** (the enum and variants don't change; only the SDK helper body changes).
- Neighboring tests/fixtures:
  - `crates/slicer-core/tests/paint_policy.rs` (NEW) — AC-1 through AC-5 + AC-N3.
  - `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` (EXTEND) — add `enforcer_works_when_centroid_outside_paint_region` (AC-8). 8 existing tests pass unchanged.
  - `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs` (EXTEND) — mirror of AC-8.
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
- The `SupportPaintPolicy` enum is re-exported from `slicer-core` so `crates/slicer-sdk/src/traits.rs` keeps its `pub use` alias; the two consumer modules' `match` arms continue to compile without source edits. This is a deliberate decision: the public ABI of `slicer-sdk` is unchanged; only the implementation moves.

## Code Change Surface

- Selected approach: extract helper to `slicer-core`; refactor `slicer-sdk::traits::paint_policy_for` to a thin wrapper; clean dead manifest strings; add L-shape regression test; clean host shim.
- Exact functions/structs/tests to change:
  - `slicer_core::paint_policy::SupportPaintPolicy` (re-export of the existing `slicer_sdk::traits::SupportPaintPolicy`).
  - `slicer_core::paint_policy::support_eligibility` (new fn).
  - `slicer_sdk::traits::PaintRegionLayerView::paint_policy_for` (refactored to wrapper; deletes `expolygon_centroid` and `regions_cover_point`).
  - `slicer_wasm_host::host::HostPaintRegionLayerView` (kebab-case → snake_case; remove dead `runtime_reads.push("PaintRegionIR")`).
  - Three module manifests (`tree-support.toml`, `traditional-support.toml`, `support-planner.toml`) — `[ir-access].reads` drops `"PaintRegionIR"`.
  - Two test files (extend `enforcer_blocker_tdd.rs` in both modules with the L-shape regression test).
  - New test file `crates/slicer-core/tests/paint_policy.rs`.
  - Two prepass test fixtures (`prepass_support_geometry_tdd.rs` and `prepass_support_geometry_layer_plan_tdd.rs` under `crates/slicer-runtime/tests/executor/`) — replace the dead `PaintRegionIR.per_layer` `ir_reads` string in the `loaded_support_planner_module` helper with `RegionMapIR.entries` + `SupportGeometryIR.entries` (the real post-P95 sources the planner reads via `MeshObjectView`).
  - `docs/05_module_sdk.md` — one paragraph.
- Rejected alternatives:
  - **Inline the helper in each module instead of moving to `slicer-core`** — rejected: the bug currently lives in the SDK helper, not in the modules. Fixing it in place at `slicer-sdk/src/traits.rs` (without extraction) is also viable but the spec commits to extraction to `slicer-core` because (a) the helper is geometrically pure and belongs in the geometry crate, (b) the `slicer-core::polygon_ops` dependency is already there.
  - **Make the helper return `bool` instead of `SupportPaintPolicy`** — rejected: the three-state semantics (Blocked / Enforced / DefaultEligible) is meaningful for downstream code; collapsing to a bool loses the distinction between "no paint" and "blocker says no".
  - **Use centroid-in-paint-region as a fast path before falling back to polygon intersection** — rejected: the centroid bug is exactly what this packet is fixing. Fast paths that re-introduce it are forbidden.
  - **Edit `tree-support` / `traditional-support` to call the new helper directly** (skip the SDK wrapper) — rejected: forces module source edits to change the `SupportPaintPolicy` import path from `slicer_sdk::traits` to `slicer_core::paint_policy`; the re-export pattern keeps the import path stable.

## Files in Scope (read + edit)

The packet edits 4 source files + 3 manifests + 3 new/extended test files (10 total). The count is justified because the migration is structurally three-prong (helper extraction + SDK refactor + manifest + host shim cleanup + per-module regression test) and removing any prong leaves the workspace in a broken half-migrated state.

- `crates/slicer-core/src/paint_policy.rs` — role: shared helper module; expected change: file created.
- `crates/slicer-core/src/lib.rs` — role: re-export; expected change: one line added.
- `crates/slicer-core/tests/paint_policy.rs` — role: AC-1 through AC-5 + AC-N3; expected change: file created.
- `crates/slicer-sdk/src/traits.rs` — role: refactor `paint_policy_for` to wrapper; delete centroid helpers; expected change: ≈60 lines net (≈80 removed, ≈20 added).
- `crates/slicer-wasm-host/src/host.rs` — role: `HostPaintRegionLayerView` kebab→snake; drop dead `runtime_reads.push`; expected change: ≈5 line edits in the impl block.
- `modules/core-modules/tree-support/tree-support.toml` — role: manifest reads update; expected change: 1 line edit.
- `modules/core-modules/traditional-support/traditional-support.toml` — role: same.
- `modules/core-modules/support-planner/support-planner.toml` — role: same.
- `modules/core-modules/tree-support/tests/enforcer_blocker_tdd.rs` — role: AC-8 L-shape regression; expected change: 1 test function added.
- `modules/core-modules/traditional-support/tests/enforcer_blocker_tdd.rs` — role: AC-8 mirror; expected change: 1 test function added.
- `crates/slicer-runtime/tests/executor/live_layer_support_tdd.rs` — role: AC-10/AC-N1/AC-N2 verification; expected change: NONE (the three tests already exist at lines 200, 361, 236 — packet verifies they still pass).
- `crates/slicer-runtime/tests/executor/prepass_support_geometry_tdd.rs` — role: pre-existing test fixture for `loaded_support_planner_module` (the `support-planner.wasm` mock loader); expected change: replace the dead `PaintRegionIR.per_layer` `ir_reads` string with `RegionMapIR.entries` and `SupportGeometryIR.entries` (the two real post-P95 sources the planner reads via `MeshObjectView`). Same justification as the manifest `PaintRegionIR` cleanup — the IR was deleted by packet 95; the literal string in this test fixture is dead.
- `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` — role: same as above; expected change: same `PaintRegionIR.per_layer` → `RegionMapIR.entries` + `SupportGeometryIR.entries` string replacement in the `loaded_support_planner_module` helper.

## Read-Only Context

- `docs/specs/support-modules-orca-port.md` §C2 — directly.
- `docs/specs/paint-pipeline-orca-parity-roadmap.md` §D14 — directly.
- `docs/01_system_architecture.md` §"Support Stage Paint Precedence" — directly.
- `docs/02_ir_schemas.md` §"SliceIR" — range-read the `SlicedRegion` + `segment_annotations` definitions.
- `crates/slicer-sdk/src/views.rs::SliceRegionView::segment_annotations` — accessor only (line 368).
- `crates/slicer-core/src/polygon_ops.rs` — read `intersection` (line 93-108 area) + `area_ex` (delegate if exact line unknown) signatures only.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — not consulted.
- `crates/slicer-core/src/algos/paint_segmentation/**` — paint kernel produced by P95; consumer side only.
- `target/`, `Cargo.lock`, generated code — never load.
- Other paint consumers (`fuzzy-skin`, `seam-placer`) — out of scope; do not browse for consistency.
- The full body of `crates/slicer-sdk/src/views.rs` outside the accessor — range-read.
- The full body of `support-planner/src/lib.rs` — out of scope for this packet. The planner already reads `PaintRegionIR` from its manifest and calls `collect_paint_enforcer_contacts` from the per-facet mesh; the planner's contact extraction is NOT being migrated in this packet. The planner's IR-availability in `[ir-access].reads` simply has `"PaintRegionIR"` dropped (the data still flows via the same `collect_paint_enforcer_contacts` path that the planner already uses; the path reads `MeshObjectView` directly, not the deleted IR).

## Expected Sub-Agent Dispatches

- "Locate `SliceRegionView::segment_annotations` accessor in `crates/slicer-sdk/src/views.rs`; return LOCATIONS + SNIPPETS ≤ 20 lines showing the signature." — purpose: confirm helper input type.
- "Confirm whether `crates/slicer-core/src/polygon_ops.rs` defines `intersection_ex` (ExPolygon-aware) or only `intersection` (flat-polygon). Return FACT (which) + file:line." — purpose: choose the right helper for `support_eligibility`.
- "Return current state of `fn paint_policy_for` in `crates/slicer-sdk/src/traits.rs`; SNIPPETS ≤ 30 lines showing the current body + the two helpers (`expolygon_centroid`, `regions_cover_point`) at lines 220 + 238." — purpose: confirm Step 3 baseline.
- "Return current state of `HostPaintRegionLayerView` impl in `crates/slicer-wasm-host/src/host.rs` (lines 3054-3120); SNIPPETS ≤ 70 lines." — purpose: confirm Step 6 baseline (the kebab-case keys + the `runtime_reads.push` lines).
- "Return current `[ir-access].reads` values for `tree-support.toml`, `traditional-support.toml`, `support-planner.toml`; FACT (per-manifest list)." — purpose: confirm manifest baseline.
- "Confirm `rg -c 'expolygon_centroid\|regions_cover_point' crates/` returns 0 callers outside the helpers themselves; return FACT." — purpose: confirm safe-to-delete.
- "Run `cargo test -p slicer-core --test paint_policy`; return FACT (per-test pass/fail); SNIPPETS ≤ 20 lines on failure." — purpose: AC-1 through AC-5 + AC-N3 gate.
- "Run `cargo test -p tree-support --test enforcer_blocker_tdd` and `cargo test -p traditional-support --test enforcer_blocker_tdd`; return FACT (per-test pass/fail); SNIPPETS ≤ 30 lines on failure." — purpose: AC-8 gate.
- "Run `cargo test -p slicer-runtime --test live_layer_support_tdd -- enforcer_forces_live_support_commit_even_when_needs_support_is_false blocker_overrides_needs_support_true_at_commit_level disabled_or_ineligible_support_stage_commits_empty_support_ir`; return FACT pass/fail per-test; SNIPPETS ≤ 30 lines on failure." — purpose: AC-10 / N1 / N2 gate.
- "Run `cargo test -p slicer-wasm-host`; return FACT pass/fail; SNIPPETS ≤ 30 lines on failure." — purpose: AC-N4 host shim gate (the dead `runtime_reads.push` removal + the kebab→snake key replacement).
- "Run `cargo xtask build-guests --check`; return FACT (`up to date` or `STALE: <list>`). NEVER paste rebuild log." — purpose: WASM gate.

## Data and Contract Notes

- IR contracts touched: none structurally — `SliceRegion.segment_annotations` already exists post-P95. The packet consumes the existing contract.
- WIT boundary considerations: the `HostPaintRegionLayerView` cleanup changes the kebab-case semantic-name keys to snake_case; this is a host-side string change. If any guest module currently looks up `regions_by_semantic` with a kebab-case key, the lookup would return empty. The implementer's Step 1 dispatch audits the guest side (`rg 'regions_by_semantic.get' crates/slicer-wasm-host/test-guests/` and `rg 'get_regions' crates/slicer-wasm-host/test-guests/`) to confirm whether any test-guest uses the kebab-case form. If so, the test-guest is updated in Step 6 alongside the host shim.
- Determinism: `support_eligibility` is pure; polygon intersection is deterministic.
- Helper threading model: callable from both host (planner prepass) and guest (layer modules) contexts. Stateless.

## Locked Assumptions and Invariants

- Blocker-wins-over-enforcer precedence per `docs/01_system_architecture.md` is fixed. The helper encodes the order explicitly; downstream callers do not override it.
- "Non-trivial intersection" means area > `1e-6 mm²` (the area epsilon in the workspace's polygon ops; confirm via Step 1 dispatch).
- `support_eligibility` does NOT consult `SurfaceClassificationIR.needs_support`. That fallback is the caller's responsibility (kept exactly as today). The helper returns `DefaultEligible` when there's no paint, and the caller falls back to `needs_support` exactly like today.
- The helper does NOT emit diagnostics. Misuse (e.g., region with no segment_annotations key at all) returns `DefaultEligible`.
- The `SupportPaintPolicy` enum is unchanged: three variants, same names, same variants. The re-export from `slicer-core` is a strict alias.

## Risks and Tradeoffs

- **Risk**: polygon intersection on every region × every layer adds work the old centroid hit didn't. **Mitigation**: typical region count per layer is modest (single-digit to low-double-digit on real models); Clipper2 intersection is fast at this scale. If profiling later shows hot-spot behavior, a bbox-overlap fast path can be added inside the helper without changing the signature.
- **Risk**: the L-shape regression test (AC-8) might be GREEN on the pre-packet code by accident if the implementer picks an L-shape whose vertex-mean centroid lies inside the painted region. **Mitigation**: Step 2 (RED tests) MUST author a fixture whose centroid is provably outside the painted region (the L's notch is in the corner of the painted square); confirm via the `expolygon_centroid` helper output before committing the test. The implementer writes a brief comment in the test explaining the centroid coordinate it expects.
- **Risk**: removing `"PaintRegionIR"` from the support-planner manifest could surface as a build error if some dispatch contract test still asserts the planner reads `PaintRegionIR`. **Mitigation**: Step 5 dispatch greps for any such assertion (`rg 'support-planner.*PaintRegionIR\|PaintRegionIR.*support-planner' crates/`); if any, the assertion is updated to use `"MeshIR"` (the actual post-P95 source for the planner's contact extraction path).
- **Risk**: the host shim cleanup (kebab→snake) is the most likely silent-failure surface. A test-guest that still uses kebab-case would return empty regions_by_semantic. **Mitigation**: Step 6 audit (see Expected Sub-Agent Dispatches) catches this before the runtime test passes by accident with empty annotations.
- **Tradeoff**: the `SupportPaintPolicy` re-export from `slicer-core` is a small import-path indirection. The benefit (call-site imports don't change) outweighs the cost (one extra `pub use`).

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 3 — `paint_policy_for` refactor + `expolygon_centroid` / `regions_cover_point` deletion).
- Highest-risk dispatch: `cargo build --workspace` after the manifest changes — return FACT pass/fail; on fail SNIPPETS ≤ 30 lines with FIRST error.

## Open Questions

- None. The collision with the source-plan TASK-261 has been resolved by renumbering to `TASK-285` (recorded in `requirements.md` §Packet Metadata and `task-map.md`).
