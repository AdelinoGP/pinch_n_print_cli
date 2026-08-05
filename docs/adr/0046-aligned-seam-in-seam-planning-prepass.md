# ADR-0046: Aligned seam modes live in the SeamPlanning prepass, not the per-layer seam placer

Status: accepted

Packet 168 ports OrcaSlicer's `aligned` / `aligned_back` seam modes (canonical
`SeamPlacer::place_seam` and the seam-string machinery around
`SeamPlacer.cpp`'s `pick_seam_option` / alignment pass, plus the fitting
utilities in `Curves.hpp`). This ADR records where that machinery lives and the
WIT contract change it required.

## Context

Aligned seam placement is inherently **cross-layer**: canonical OrcaSlicer
chains seam candidates across consecutive layers into "seam strings" and then
smooths each string with a least-squares B-spline fit (canonical
`Curves.hpp::fit_cubic_bspline`), so that seams form a continuous vertical line
instead of jumping per layer. `aligned_back` is the same pass with a rear bias
applied to candidate scoring.

PnP has two candidate homes for this logic:

1. The per-layer `seam-placer` module (`modules/core-modules/seam-placer`),
   Layer tier.
2. The `PrePass::SeamPlanning` stage module
   (`modules/core-modules/seam-planner-default`), which runs once per print
   before any layer work.

Option 1 is structurally impossible under this codebase's execution model:
per-layer modules are **re-instantiated per call and run in parallel across
layers** (ADR-0045 records that no state survives between calls — the module is
rebuilt per call, and packet 102 already ruled cross-call caching forbidden).
A per-layer module can never see two layers, so it can never chain anything.

The only sanctioned cross-layer conduit is the one ADR-0020 established:
`SeamPlanIR` produced by the SeamPlanning prepass, delivered to the per-layer
seam placer as a host-injected `resolved_seam` on each layer's input. There is
no other channel.

## Decision

- **All aligned machinery lives in `seam-planner-default`'s prepass** —
  `modules/core-modules/seam-planner-default/src/comparator.rs` (candidate
  scoring, ported from canonical `SeamPlacer.cpp`'s seam-comparator logic),
  `visibility.rs` (deterministic raycast visibility, reduced budget — see
  the seam-prepass source record in `docs/DEVIATION_LOG.md`), `align.rs`
  (seam-string chaining + least-squares spline smoothing, ported from canonical
  `SeamPlacer.cpp` + `Curves.hpp`), and `contours.rs` (PnP-original z-plane
  sectioning of `MeshObjectView` triangles into per-layer contours).
  `seam_mode` on `seam-planner-default` accepts `aligned` / `aligned_back`;
  the default was `nearest` initially; amended 2026-07-22 to `aligned` per packet 180 (see the ADR amendment in `docs/DEVIATION_LOG.md` and packet 180).

- **The WIT export gains a parameter.** The prepass needs real layer z values,
  so `run-seam-planning` (canonical WIT source
  `crates/slicer-schema/wit/deps/world-prepass/world-prepass.wit`) now takes
  `layer-plan: layer-plan-view` alongside `objects` / `output` / `config` —
  the same view `run-support-geometry` already consumes.

- **That is a major world-version bump: `slicer:world-prepass` 1.0.0 → 2.0.0.**
  `docs/11_operational_governance_and_acceptance_gate.md` classifies a type
  change to an existing export — which adding a required parameter is — as a
  major bump. DEV-084 (packet 130's `run-infill-postprocess` parameter, shipped
  as 1.1.0 and corrected to 2.0.0) is the precedent this follows.

- **Consumption side:** `seam-placer` (per-layer) reads the host-injected
  planner choice and **snaps it to the nearest of its own seam candidates**
  (unlimited snap radius, falling back to the nearest wall vertex when no
  candidate exists; pristine per-layer behaviour when no planner entry is
  injected). Snapping is what keeps the emitted seam on a real wall vertex even
  though the prepass computed it from mesh-derived contours rather than final
  perimeters (see the seam-prepass source record).

## Alternatives rejected

- **Per-object anchor accumulator inside `seam-placer`.** A static or
  blackboard-side accumulator that per-layer calls append to. Rejected: layer
  calls run in parallel with no ordering guarantee, so the accumulator would
  see layers out of order and nondeterministically; it also reintroduces
  exactly the cross-call state ADR-0045 and packet 102 forbid.
- **Host-builtin native alignment pass.** Run the chaining/smoothing in the
  host between prepass and layer dispatch. Rejected: it moves slicing policy
  out of the module system, bypassing the manifest/config surface and the
  ADR-0020 injection contract that already exists for precisely this data flow.
- **Deriving z from `layer_height` config instead of the layer plan.** Rejected:
  variable layer height, first-layer height, and catch-up layers make
  `z = i * layer_height` wrong in general; `layer-plan-view` carries the
  planned truth and was already exported to prepass modules for
  `run-support-geometry`.

## Consequences

- `slicer:world-prepass` majors to 2.0.0; all prepass guests rebuild
  (`cargo xtask build-guests`).
- The aligned path's inputs are mesh-derived contours, not final perimeters —
  a recorded deviation from canonical (which runs `SeamPlacer` after perimeter
  generation), mitigated by the seam-placer snap. Tracked as
  the seam-prepass source record.
- `nearest` mode is still untouched end-to-end and remains available via `seam_mode`; `aligned` and `aligned_back` are now the defaults (set in both `seam-placer.toml` and `seam-planner-default.toml`), matching OrcaSlicer's `spAligned` default. Users may still opt in to any other mode via `seam_mode`.

## Amendment — 2026-08-05 (packet 168)

The `run-seam-planning` export gained a **required** `layer-plan: layer-plan-view` parameter alongside `objects` / `output` / `config` — the same view `run-support-geometry` already consumes — so the prepass has real layer z values. Adding a required parameter to an existing export is a type change, which `docs/11_operational_governance_and_acceptance_gate.md` classifies as a major world-version bump (DEV-084, packet 130's `run-infill-postprocess` parameter, is the precedent). Packet 168 shipped the intermediate bump `slicer:world-prepass` 1.0.0 → 2.0.0; the packet 178 amendment below records the final version.

## Amendment — 2026-08-05 (packet 178)

`SeamPlanning` now consumes **per-active-region SliceIR geometry through a `seam-planning-view` input** instead of the mesh-derived contours `contours.rs` produced by z-plane sectioning of `MeshObjectView` triangles. Plans are keyed by the full active-region `RegionKey` `(global_layer_index, object_id, region_id, variant_chain)`; no contour ordinal is used as region identity, and inactive regions emit no `SeamPlanEntry`. The required SeamPlanning input change bumped the WIT world to **`slicer:world-prepass` 3.0.0** (2.0.0 → 3.0.0, pinned by packet 178's AC-1) — that is the final version, superseding this ADR's earlier 1.0.0 → 2.0.0 statements and the "majors to 2.0.0" consequence.

This amendment also resolves the audit finding that the "seam-prepass source record" referenced by this ADR (and by the doc-impact statements of packets 168/178/179/180) has **no row in `docs/DEVIATION_LOG.md`**: the retired `seam source-geometry deviation` label never had a surviving row. The mesh-derived source that record would have described no longer exists — packet 178's per-region SliceIR input supersedes it — so the missing record is retired as resolved rather than backfilled.

Retired by this amendment:

- The Decision bullet's "`contours.rs` (PnP-original z-plane sectioning of `MeshObjectView` triangles into per-layer contours)".
- The Consequences paragraph "The aligned path's inputs are mesh-derived contours, not final perimeters — a recorded deviation from canonical ... mitigated by the seam-placer snap. Tracked as the seam-prepass source record."

## Amendment — 2026-08-05 (packet 179)

Visibility sampling is restored to the **canonical budget**: 30000 samples with 25 rays per sample (`raycasting_visibility_samples_count = 30000`, `sqr_rays_per_sample_point = 5` → 25 rays), using deterministic per-object seeded **area-uniform sampling** — not the reduced budget packet 168 shipped, and not Halton low-discrepancy sampling. Retired by this amendment: the Decision bullet's "deterministic raycast visibility, **reduced budget** — see the seam-prepass source record in `docs/DEVIATION_LOG.md`" parenthetical (the reduced budget no longer exists, and the referenced record has no DEVIATION_LOG row; see the packet 178 amendment).

Additional packet 179 behavior:

- (a) **Aligned seam chaining retries short seam strings** from alternative starts spaced `1 + size/20` apart, retaining the longest qualifying string; only strings meeting the canonical minimum (`SEAM_ALIGN_MINIMUM_STRING_SEAMS`) are finalized.
- (b) **Active-region gap bridging** (PnP extension, documented as such, not claimed canonical): inactive layers emit no entries, the last real seam is retained as the continuity anchor, the resume search is bounded by canonical `seam_align_tolerable_dist_factor` × flow width, and a new string is created when no candidate qualifies.
- (c) The internal seam candidate model carries `layer_angle`, and seam-string weighting uses canonical `curling_influence` rather than a fixed 1.0 constant.
- (d) Weighted spline fitting uses `faer::linalg::solvers::ColPivQr` **unconditionally** — never normal equations or a local fallback; rank-deficient control points are zeroed and non-finite outputs sanitized.

## Amendment — 2026-08-05 (packet 180)

The consumption side is now **continuous projection onto final wall geometry**: `seam-placer` projects the planner's target onto the nearest point of the final wall loop, inserting a point into the segment when the target is not on a vertex, interpolating `feature_flags` and `width_profile` at the inserted point, and re-closing the loop. When no `SeamPlanIR` entry matches a region, the module emits a non-fatal `ModuleError` identifying the missing `(layer, object, region_id, variant_chain)` key and applies canonical local candidate selection as a **degraded fallback**, preserving all walls.

Retired by this amendment: the Decision bullet's "**snaps it to the nearest of its own seam candidates** (unlimited snap radius, falling back to the nearest wall vertex when no candidate exists...)" — the vertex-only snap was replaced by continuous projection. The `aligned` default change (amended 2026-07-22 per packet 180) is unaffected by this amendment.
