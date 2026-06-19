# Design: 108_perimeter-special-modes-and-seam

## Controlling Code Paths

- Primary code path: in `run_perimeters` (both modules), three early-stage branches read upstream data and adjust the wall-emission loop — `extra_perimeters` bumps `loop_number`, narrow-island detection swaps in `smaller_perimeter_line_width`, non-planar detection short-circuits the loop entirely (emit `shell_count` `NonPlanarShell` walls, skip thin/gap/infill). After wall emission, `slicer_core::perimeter_utils::generate_sharp_corner_seam_candidates` produces the sparse candidate list using the angle threshold; `apply_seam_paint_bias` biases enforcer-enclosed entries (matching `PaintSemantic::Custom(s)` where `s == "seam_enforcer"`) and removes blocker-enclosed entries (matching `Custom(s)` where `s == "seam_blocker"`); the result lands in `PerimeterRegion.seam_candidates`. `seam-placer` reads from there as it does today (no architectural change), but now scores over a sparser list whose enforcer/blocker semantics are pre-applied. **`PaintSemantic::SeamEnforcer` and `SeamBlocker` are NOT named variants — do not match on them.**
- Neighboring tests / fixtures: 6 new TDD files. Existing regression tests from P102/P103/P104/P105 must stay green.
- OrcaSlicer comparison surface: see `requirements.md` §OrcaSlicer Reference Obligations (delegate; never load).

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- ADR-0011 + ADR-0013 invariants from P105 carry forward unchanged. This packet adds no new ADRs.
- Per-layer config rule: all 6 new config keys are read via `_config.get*` per `run_perimeters` call.
- Non-planar branch invariant (per D-11 closure): when `region.nonplanar_surface.is_some()`, the perimeter module emits `shell_count` walls of `LoopType::NonPlanarShell` and produces empty `infill_areas`; the downstream `non-planar-walls` module (sibling roadmap, not in this packet's scope) does the Z modulation. The perimeter module does NOT compute or write per-vertex non-planar Z here.
- `LoopType::NonPlanarShell` already exists in the IR (no schema bump in this packet). The variant is used for the first time by emission code in this packet.
- T-077 real-consumer invariant: when `region.overhang_areas()` returns non-empty (post-P106+P107 data flow), the `extra_perimeters_on_overhangs` consumer adds one extra wall inside the overhang polygons; outside, wall count is unaffected. The code path also handles empty input gracefully (e.g., layers with no overhang) — zero extras, no panic. **FORWARD-DEP:** `SliceRegionView::overhang_areas()` does NOT yet exist (verified: `crates/slicer-sdk/src/views.rs` only has `has_nonplanar()`); it is produced by draft P104. This AC is blocked until P104 is `status: implemented`. Additionally, the spec's reference to `OverhangRegion.xy_footprint` (populated by P106) is a FORWARD-DEP conflict: the tree shows `xy_footprint: Vec<ExPolygon>` on `BridgeRegion` (slice_ir.rs:581), NOT on `OverhangRegion`; `OverhangRegion` has no such field. Reconcile with P106 before activation.
- Seam-candidate sparseness invariant: `seam-placer` MUST tolerate `seam_candidates.len() == 0` (returns `Err(ModuleError::fatal(…))` with a recognisable message per AC-N2 — `SeamPlacerError::NoCandidates` does NOT currently exist; it may be defined as net-new or the module can inline the error via `ModuleError::fatal`). If T-082 audit finds the current `seam-placer` panics or silently produces a degenerate seam on empty input, that's a Step 4 fix.

## Code Change Surface

- Selected approach: each Phase 7 override is a discrete branch at the head of `run_perimeters`'s wall-emission loop; the three overrides are mutually exclusive (a region is non-planar, OR narrow, OR neither — non-planar takes precedence). Seam-candidate quality is two helper additions in `slicer-core::perimeter_utils` plus a `seam-placer` integration point for paint-bias application. T-077 is the narrowest of the seven changes — a one-branch addition that reads `overhang_areas()` and adds extras only inside those areas; post-P106+P107 the upstream data flow is live, so the consumer's non-empty path is exercised by AC-6.
- Exact functions, traits, manifests, tests, or fixtures expected to change:
  - **DELETION (T-090/T-091/T-092):** `modules/core-modules/arachne-perimeters/` — entire directory deleted. Root `Cargo.toml` member entry removed. Stale doc/spec refs scrubbed.
  - `modules/core-modules/classic-perimeters/src/lib.rs` — Phase 7 + Phase 8 consumer additions (~120 LOC delta). NOTE: `arachne-perimeters/` is DELETED in Step 0; all consumer additions land in classic-perimeters only.
  - `crates/slicer-core/src/perimeter_utils.rs` — add `pub fn generate_sharp_corner_seam_candidates(contour, z, angle_threshold_deg) -> Vec<SeamCandidate>`; add `pub fn apply_seam_paint_bias(candidates: &mut Vec<SeamCandidate>, paint: &PaintRegionLayerView)` — consume via `paint.semantics_on_layer()` + match on `PaintSemantic::Custom(s)` strings `"seam_enforcer"`/`"seam_blocker"` (NOT named variants); keep the existing `generate_seam_candidates` for back-compat (deprecated; callers migrate). Note: `SeamCandidate` here is the **local** type (`position: Point3, score: f32`), not `slicer_ir::SeamCandidate` (`position: Point3WithWidth, score, reason: SeamReason`). The bridge to `push_seam_candidate(pos: Point3, score)` (SDK builder) is type-compatible; callers convert to IR type when needed.
  - `modules/core-modules/seam-placer/src/lib.rs` — call `apply_seam_paint_bias` before scoring; return `Err(ModuleError::fatal(…))` on empty input (AC-N2 fix if audit finds a regression). If a typed `SeamPlacerError` is defined as net-new, wire it through `ModuleError::fatal`.
  - `modules/core-modules/classic-perimeters/classic-perimeters.toml` — register 6 config keys. (arachne-perimeters manifest is gone.)
  - `docs/15_config_keys_reference.md` — register the 6 keys.
  - `docs/05_module_sdk.md` — document seam-candidate generation convention.
  - `docs/DEVIATION_LOG.md` — supersede + register two deviations.
  - 6 new TDD files.
- Rejected alternatives that were considered and why they were not chosen:
  - Replace `generate_seam_candidates` in-place with the thresholded version: rejected because the existing function has callers elsewhere (audit-confirmed); a versioned addition + migration is safer.
  - Apply paint-seam bias inside `seam-placer` only (not in perimeter generation): rejected because the perimeter module needs to know about blocker regions to *exclude* candidates (blocker = no candidate, not zero-score candidate); placing the exclusion at perimeter time is cleaner.
  - Add a new `LoopType::ExtraPerimeter` variant for the bonus walls: rejected — they are still topologically inner walls (`LoopType::Inner`); the count change is the only semantic difference, not the type.

## Files in Scope (read + edit)

- `modules/core-modules/classic-perimeters/src/lib.rs` — primary consumer (seam + special-modes work).
- **DELETION (T-090): `modules/core-modules/arachne-perimeters/`** — entire directory deleted (src/, manifest, Cargo.toml, wit-guest/, tests/). Do NOT read or edit individual files; the operation is `rm -rf modules/core-modules/arachne-perimeters/`.
- **DELETION (T-091): root `Cargo.toml`** — remove the `"modules/core-modules/arachne-perimeters"` workspace member line.
- **DELETION (T-092): doc/spec stale refs** — `rg -rn 'arachne-perimeters\|com\.core\.arachne-perimeters' docs/ .ralph/specs/` to enumerate then scrub. Leave historical/decision context lines intact.
- `crates/slicer-core/src/perimeter_utils.rs` — two new helpers (seam work).
- `modules/core-modules/seam-placer/src/lib.rs` — integration + AC-N2 robustness.
- `modules/core-modules/classic-perimeters/classic-perimeters.toml` — 6 config keys (only classic; arachne-perimeters is deleted this packet).
- `docs/15_config_keys_reference.md`, `docs/05_module_sdk.md`, `docs/DEVIATION_LOG.md` — per Doc Impact Statement.
- 6 new TDD files.

## Read-Only Context

- `docs/specs/perimeter-modules-orca-parity-roadmap.md` — range-read Phase 7 + Phase 8 sub-tables + "Inherited from P98" section.
- `docs/specs/overhang-pipeline-restructuring.md` — read full — purpose: understand the now-shipped upstream data flow T-077 consumes.
- `docs/02_ir_schemas.md` — delegate SUMMARY for `LoopType`, `SurfaceGroup`, `PaintSemantic` (no `SeamEnforcer`/`SeamBlocker` variants; seam paint uses `Custom("seam_enforcer")`/`Custom("seam_blocker")`), `SeamCandidate` (IR: `Point3WithWidth`; local perimeter_utils: `Point3`).
- `docs/05_module_sdk.md` — delegate SUMMARY for `SliceRegionView::surface_group()` (FORWARD-DEP on draft P104 — does NOT yet exist on `SliceRegionView`) and `PaintRegionLayerView` real API (`semantics_on_layer()` + `paint_policy_for()` — `get_regions` does NOT exist).
- `docs/07_implementation_status.md` — read the `D-98-SEAM-NO-CONSUMER` note (it lives here, NOT in `docs/DEVIATION_LOG.md`) and any recent OVERHANG-related entries.

## Out-of-Bounds Files

- `OrcaSlicerDocumented/**` — delegate.
- `target/`, `Cargo.lock`, generated bindgen output — never load.
- Vendored deps — never load.
- `seam-planner-default/src/lib.rs` — out of scope for T-083; the deliverable is a doc note based on the manifest, not its source.
- All other `modules/core-modules/*/src/lib.rs` except the two perimeter modules + `seam-placer` — out of scope.
- All `crates/slicer-ir/`, `crates/slicer-schema/wit/`, `crates/slicer-core/algos/` — no IR/WIT changes in this packet (P105 closed those).
- All other crates not in §Files in Scope.

## Expected Sub-Agent Dispatches

- "Summarize OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1611-1628 for narrow-island `smaller_ext_perimeter_flow`; return SUMMARY ≤ 150 words." — Step 2.
- "Summarize OrcaSlicerDocumented/src/libslic3r/Feature/SeamPlacer/SeamPlacer.cpp for sharp-corner candidate selection + painted seam consumption; return SUMMARY ≤ 200 words, no code." — Step 4.
- "FACT: confirm OrcaSlicerDocumented/src/libslic3r/PerimeterGenerator.cpp:1569 carries `loop_number = wall_loops + surface.extra_perimeters - 1`; return single-line FACT." — Step 1.
- "Find call sites of `generate_seam_candidates` (legacy) across the workspace; return LOCATIONS ≤ 10 entries." — Step 4 migration scope.
- "Run `cargo test -p slicer-runtime --test integration extra_perimeters_config_tdd narrow_island_smaller_perimeter_tdd nonplanar_shell_emission_tdd painted_seam_enforcer_blocker_tdd extra_perimeters_on_overhangs_tdd && cargo test -p slicer-core --test sharp_corner_seam_threshold_tdd`; return FACT pass/fail per test." — packet close.

## Data and Contract Notes

- IR or manifest contracts touched: 6 new config keys in two manifests + central registry. No IR change. `LoopType::NonPlanarShell` first-use (already in IR per P102's existing variant).
- WIT boundary considerations: no WIT change in this packet.
- Determinism or scheduler constraints: sharp-corner threshold is a fixed numeric comparison; `apply_seam_paint_bias` operates on a Vec and is deterministic over the same `PaintRegionLayerView`. No scheduler change.
- T-082 audit deliverable: a paragraph in `docs/05_module_sdk.md` documenting `seam-placer`'s tolerance for sparse candidate lists; if the audit finds a regression on empty input, AC-N2's empty-candidate error path is the fix — return `Err(ModuleError::fatal(…))` with a recognisable message, or define `SeamPlacerError` as net-new and wire it through `ModuleError`.
- T-083 documentation deliverable: a paragraph in `docs/05_module_sdk.md` confirming that `seam-planner-default` (PrePass) and the perimeter modules + `seam-placer` (per-layer) operate independently — the PrePass output does NOT directly feed perimeter-time candidate generation.

## Locked Assumptions and Invariants

- Non-planar branch is the highest-precedence override: if `region.nonplanar_surface.is_some()`, the non-planar branch fires regardless of other config (no thin-wall, no gap-fill, no `extra_perimeters` bonus, no narrow-island handling).
- `extra_perimeters` and narrow-island handling are independent and additive within the planar branch (a narrow island with `extra_perimeters = 2` gets 2 extra walls AT the smaller width).
- Sharp-corner threshold uses **absolute** turn angle (degrees from straight). Default 30° matches OrcaSlicer's documented seam-placer convention.
- `apply_seam_paint_bias` enforcer bias factor: `score *= 0.1` (lower is more preferred). Blocker exclusion is a list-filter, not a score deboost.
- T-077 consumer code path is wired and tested for **both** non-empty (overhang → N+1 walls) and empty (flat → N walls) inputs on the same AC-6 fixture, because the P106+P107 data flow is a hard predecessor of this packet.
- `perimeter_utils` consumed from `slicer-core` per docs/13 §Out of Scope. Part of roadmap-wide correction `D-ROADMAP-CRATE-PLACEMENT`.

## Risks and Tradeoffs

- T-077 consumes data from two upstream packets (P106 + P107). Risk: if P106 or P107 ships incomplete (e.g., `xy_footprint` populated but accessor pre-filter buggy), T-077's overhang region check would silently produce wrong wall counts. Mitigation: AC-6 directly tests both "non-empty overhang → N+1 walls" and "empty overhang → N walls" paths on the same fixture, catching either failure mode.
- Sharp-corner threshold's default (30°) may cut some users' historical "every-vertex seam" expectations. Mitigation: register the config so users can lower it; document the default in the seam-placer SDK doc.
- `apply_seam_paint_bias` runs over all candidates after threshold filtering. Performance is O(candidates × paint_regions). With the candidate density reduced by ~25× (T-080), this stays well under budget. If a fixture surfaces a perf regression, switch to AABB pre-filtering of paint regions.
- The T-082 + T-083 deliverables are documentation, not code — risk that they get cut. Mitigation: the Doc Impact Statement greps fail if the sections aren't present, blocking packet close.

## Context Cost Estimate

- Aggregate (sum across all steps): `M`
- Largest single step: `M` (Step 2 — non-planar branch implementation across both modules + new TDD; or Step 4 — seam helpers + audit + paint-bias + new test).
- Highest-risk dispatch: SeamPlacer SUMMARY (≤ 200 words). Re-dispatch if pseudocode appears.

## Open Questions

- `[FWD]` `seam_enforcer_bias_factor` exact value: roadmap doesn't specify. Default `0.1` chosen for "10× preferred"; if a fixture shows enforcer regions losing to extremely sharp corners outside, raise the factor (lower the number). Configurable via a future packet if needed.
- `[FWD]` T-077 test fixture: overhang-ramp mesh shared with P106 + P107 is the natural fixture; if the regression fixture conventions differ, follow existing patterns.
- `[FWD]` T-083 seam-planner interaction: if the implementer finds during the audit that seam-planner-default DOES feed perimeter-time candidate generation (contrary to current assumption), revise the doc note + add an integration test. Otherwise the one-paragraph note is sufficient.
