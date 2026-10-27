# Per-surface bridge boolean locality in ShellClassification

Type: task
Label: wayfinder:task
Status: open (graduated 2026-10-27 from a merged-upstream-PR screening; unclaimed)
Assignee: none
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: OrcaSlicer PR #15985, "Faster slicing of layers cut into many pieces" (merged 2026-10-21)

## Question

Can the per-surface / per-candidate whole-layer clipper work in
`PrePass::ShellClassification` be restricted to each surface's
bounding-box neighbourhood — and the layer-constant fill-boundary offset
hoisted out of the candidate loop — with byte-identical output, and does the
resulting wall win clear the map's 0.80 % floor on the four base classic cells?

**Screen first.** The known terms sit below the floor; the unmeasured
per-surface term may not. Phase 1 sizes it, Phase 2 builds the candidate only
if it does.

## Measured premise

The merged upstream PR's applicable mechanisms are "restrict whole-layer work
to the neighbourhood that needs it" and "fill boundary expanded once per
spacing". Its other halves do not map: PnP is Rust (moves already move), the
per-layer phase is already parallel across 495 layers at 12 threads, and the
dominant per-layer module insets a whole region once per wall
(`emit_walls`, `modules/core-modules/classic-perimeters/src/lib.rs`) rather than
looping pieces.

The per-piece shape in PnP is the internal-bridge qualification in
`compute_internal_bridge_layer`
(`crates/slicer-runtime/src/slice_postprocess_prepass.rs`):

- for every surface in `region.internal_solid_fill`, one
  `offset(surface, spacing)` plus
  `intersection(expanded_candidate, deep_infill_clip_area)` against the whole
  layer's clip set, then `qualify_internal_bridge_surface(surface, unsupported)`
  (a whole-`unsupported` intersection), then a retain
  `intersection(polygon, internal_unsupported_area)` per qualified polygon;
- `internal_bridge_angles`
  (`crates/slicer-core/src/algos/bridge_over_infill.rs`) loops the candidates
  and re-offsets the layer-wide `total_fill_area` by `1.3 × spacing` inside
  `boundary_anchor_polylines` for every candidate, although that half of the
  anchor set is layer-constant.

The [t75 attribution](../evidence/t75-shellclass-phase-b/FINDINGS.md) measured
the neighbouring terms on base classic supports-off before its candidate:
`internal_bridge_angles` **2.4 s** and the clip term **1.5 s** of a 30.7 s
serial phase-B commit, with 121 qualified entries, 7,861 deep-infill polygons
and 10 angle calls. t75's KEEP (`07b97cad`) moved the independent per-layer work
into the parallel qualification phase, so the work survives and its wall share
is divided by the phase concurrency.

The current base classic supports-off cell wall is **73.17 s**
([t87 scoreboard](../evidence/scoreboard-refresh-20261025/SCOREBOARD.md)). The
floor is therefore **0.585 s**. The measured CPU/wall ratio on that cell is
**9.65–9.74**, so 1 s of wall ≈ 9.7 s of accumulated work. The t75-named
clip+angles terms alone are 3.9 s accumulated ≈ 0.40 s ≈ **0.55 %** of the
current cell — below the floor if that is the whole target. The *unmeasured*
terms are the per-layer `region.internal_solid_fill` count and the share of the
`deep_infill_clip_area` intersections; Phase 1 must size them before any
candidate is built.

Disclosure: the t75 split is pre-t75 and pre-t81, and the current per-stage
split is unmeasured. The 0.55 % is arithmetic from those brackets and the
current CPU/wall ratio, not a measurement of the candidate.

## Phase 1 — screening (mandatory)

1. Instrument an ordinary base classic supports-off slice with a
   reverted-before-close probe that records, per layer,
   `region.internal_solid_fill.len()`, the number of `deep_infill_clip_area`
   polygons, and the accumulated time of the per-surface
   `offset`/`intersection` calls and of `boundary_anchor_polylines`'s two
   offsets.
2. Convert the accumulated shares to wall with the run's measured CPU/wall
   ratio and compare against 0.585 s.
3. If the ceiling is below the floor, close as **measured-dead** with the probe
   reverted; no candidate.
4. If above, proceed to Phase 2.

## Phase 2 — candidate (conditional)

1. Bbox-gate every per-surface/per-candidate clip: pass only the polygons of
   `deep_infill_clip_area` / `unsupported` / `internal_unsupported_area` whose
   bbox meets the subject's bbox. Conservative, so the set result is unchanged.
2. Hoist the layer-constant `offset(total_fill_area, 1.3 × spacing)` out of the
   `boundary_anchor_polylines` candidate loop; keep the `limiting_area` half per
   candidate.
3. If a layer's `internal_solid_fill` count exceeds the many-piece threshold
   (Orca uses 128), group the surfaces into tiles and run one intersection per
   tile against the clip polygons near it, instead of one whole-layer call per
   surface. Tiles must be cut with a margin so the final set is unchanged.
4. No geometry function changes; emitted G-code must stay byte-identical to the
   post-t81 reference set.

## Pre-declared floor

**0.80 % of the cell wall** (the map's established screening floor). Phase 1
measures the ceiling; Phase 2's paired campaign measures the actual win.

## Proof gate

- Phase 1 probe reverted; both guest freshness gates exit 0.
- Phase 2 only: 16-cell output-identity matrix against the post-t81 reference
  set, both modes.
- Phase 2 only: an equivalence test on a multi-surface fixture with a non-empty
  unsupported set — the bbox-gated/hoisted path returns the same polygons as
  the current per-surface reference.
- Phase 2 only: standing paired ordinary + accelerated gate on the four base
  classic cells (both modes), 6 alternating pairs per cell, 12 threads, zero
  starvation exclusions, every pair byte-identical, process CPU ≤ baseline
  + 3.0 %.
- Both guest freshness gates exit 0 (host-only candidate).

## Deliverables

1. The Phase 1 screening table and a measured ceiling against the floor.
2. If above the floor, the candidate plus the equivalence test.
3. The paired campaign results and a measured recommendation to the human. No
   auto-commit.
