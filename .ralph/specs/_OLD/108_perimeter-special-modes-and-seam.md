---
status: implemented
packet: 108_perimeter-special-modes-and-seam
task_ids:
  - T-070
  - T-071
  - T-072
  - T-073
  - T-074b
  - T-074c
  - T-074d
  - T-077
  - T-080
  - T-081
  - T-082
  - T-083
  - T-P98-SEAM
  - T-090
  - T-091
  - T-092
---

# 108_perimeter-special-modes-and-seam

## Goal

Land the Phase 7 wall-count overrides (`extra_perimeters` config bonus, narrow-island `smaller_perimeter` handling, `LoopType::NonPlanarShell` emission for regions in surface groups) and the Phase 8 seam-candidate quality work (sharp-corner threshold replacing every-vertex emission, painted `seam_enforcer`/`seam_blocker` consumption in candidate scoring + seam-placer selection).

## Problem Statement

**Deletion (T-090/T-091/T-092):** The existing `modules/core-modules/arachne-perimeters/` is a 512-line iterative-inset approximation that is NOT real Arachne. The decision is to DROP it outright (it is dead code; no successor module named after it will ship). P108 deletes the directory, removes the workspace member entry, and scrubs stale references. P110 will CREATE a fresh `arachne-perimeters/` skeleton for real Arachne in a later packet. Between P108 and P110 activation, `classic-perimeters` is the sole perimeter generator — by design.

After P105 lands the wall-emission geometry stack, three wall-count override mechanisms and the seam-candidate quality work remain. The override mechanisms are:

1. **`extra_perimeters` per-region config**: a normal per-region bonus that adds N walls beyond the configured base (`loop_number = wall_count + extra_perimeters - 1`). Currently the perimeter modules don't read this config; setting it has no effect.
2. **Narrow-island width handling**: long-narrow islands below a length threshold use a smaller extrusion width (`smaller_perimeter_line_width`) so the wall actually fits. Without this, narrow islands are skipped entirely when the wall_inset can't fit two full-width walls.
3. **Non-planar wall emission** (per D-11 closure in the roadmap): regions whose `nonplanar_surface` is set are part of a swept surface group; the perimeter module must emit `LoopType::NonPlanarShell` walls instead of `Outer`/`Inner`, honour `SurfaceGroup.shell_count` as the override for `wall_count`, and skip thin-wall/gap-fill/infill (because the surface group sweep is the only geometry).

The seam quality work has two halves:

1. **Sharp-corner threshold (T-080..T-083)**: current modules push **every wall vertex** as a seam candidate. For a 100-vertex polygon, that's 100 candidates per layer-region. Seam-placer's scoring runs over all of them. Replacing with an angle-threshold (only corners with turn-angle ≥ ~30°) reduces candidates ~25× on typical shapes.
2. **Painted seam consumption (T-P98-SEAM, inherited)**: P98 decoded `paint_seam` sub-facet strokes into `SeamEnforcer`/`SeamBlocker` semantics in `boundary_paint`, but no live module reads them (`D-98-SEAM-NO-CONSUMER`). This packet wires the consumer: enforcer regions bias seam-candidate selection toward enclosed vertices; blocker regions exclude enclosed vertices.

T-077 (`extra_perimeters_on_overhangs`) is a real consumer in this packet — its data-flow preconditions (P104 stub accessor + P106 PrePass-side `xy_footprint` population + P107 view-accessor confirmation) all ship before this packet runs. The config key is registered and the consumer code path adds one extra perimeter inside `region.overhang_areas()` polygons when enabled.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.

- ADR-0011 + ADR-0013 invariants from P105 carry forward unchanged. This packet adds no new ADRs.
- Per-layer config rule: all 6 new config keys are read via `_config.get*` per `run_perimeters` call.
- Non-planar branch invariant (per D-11 closure): when `region.nonplanar_surface.is_some()`, the perimeter module emits `shell_count` walls of `LoopType::NonPlanarShell` and produces empty `infill_areas`; the downstream `non-planar-walls` module (sibling roadmap, not in this packet's scope) does the Z modulation. The perimeter module does NOT compute or write per-vertex non-planar Z here.
- `LoopType::NonPlanarShell` already exists in the IR (no schema bump in this packet). The variant is used for the first time by emission code in this packet.
- T-077 real-consumer invariant: when `region.overhang_areas()` returns non-empty (post-P106+P107 data flow), the `extra_perimeters_on_overhangs` consumer adds one extra wall inside the overhang polygons; outside, wall count is unaffected. The code path also handles empty input gracefully (e.g., layers with no overhang) — zero extras, no panic. **FORWARD-DEP:** `SliceRegionView::overhang_areas()` does NOT yet exist (verified: `crates/slicer-sdk/src/views.rs` only has `has_nonplanar()`); it is produced by draft P104. This AC is blocked until P104 is `status: implemented`. Additionally, the spec's reference to `OverhangRegion.xy_footprint` (populated by P106) is a FORWARD-DEP conflict: the tree shows `xy_footprint: Vec<ExPolygon>` on `BridgeRegion` (slice_ir.rs:581), NOT on `OverhangRegion`; `OverhangRegion` has no such field. Reconcile with P106 before activation.
- Seam-candidate sparseness invariant: `seam-placer` MUST tolerate `seam_candidates.len() == 0` (returns `Err(ModuleError::fatal(…))` with a recognisable message per AC-N2 — `SeamPlacerError::NoCandidates` does NOT currently exist; it may be defined as net-new or the module can inline the error via `ModuleError::fatal`). If T-082 audit finds the current `seam-placer` panics or silently produces a degenerate seam on empty input, that's a Step 4 fix.

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
