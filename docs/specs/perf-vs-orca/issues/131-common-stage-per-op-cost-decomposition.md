# Common-stage per-op cost decomposition: where does the 9.8–15.7x go?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: open (unclaimed)
Assignee: —
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Prepass work-set and per-op parity against canonical v2.5.0](129-prepass-work-set-per-op-parity.md)'s
  2026-11-08 go/no-go: the work set is not the lever, operand locality and
  per-op cost are; the next gate must price the per-op half.

## Question

Where does PnP's per-op cost go on the **common** prepass stages — the ones that
compute canonical's work set rather than extra work — and which operand-locality
shapes remove enough of it to make the prepass reach canonical cost?

The stages and their base classic-off / -on walls (t126, instrumented
attribution) are:

- `PrePass::ShellClassification` — 14,698 / 15,300 ms. Same classification
  algorithm as canonical, plus the `discover_horizontal_shells` cascade that
  canonical skips at `evstAll`; PnP omits canonical's
  `process_external_surfaces` wave expansion (`DEV-200`).
- `PrePass::Slice` — 2,110 / 2,117 ms. Same batched `slice_mesh_ex` shape.
- `PrePass::SupportGeometry` + `SupportAnalysis` + the planner — base-off
  **296 ms** (the host producer, negligible); base-on **68,882 ms** (55,798
  planner + 13,084 analysis). Same legacy `TreeSupport` path; the occupancy
  inflation is uncached at two sites (32.7 s / 864 calls, t113).
- The per-layer phase — 8,051 / 12,505 ms. The classic-perimeters host
  round-trip bucket is 97–99 % native Clipper ops (t68), and the benchy cells
  need the phase in the same program even at canonical prepass cost (t129).

The question is not "which stage can be deleted"; t129 answered that the clean
work-set deletions are 55.8 % of base-off's required cut and 17.5 % of base-on's.
The question is **why the same work costs 9.8–15.7x more**, and whether that cost
is removable within the map's representation-safe / parity rules.

## Why this ticket exists

[t129](129-prepass-work-set-per-op-parity.md) measured the decisive split:

- PnP's op count is **not** higher than canonical's in the shell cluster
  (≈15–25 vs ≈45–60 ops/layer), because PnP omits the external-surface wave
  expansion. Yet canonical's whole prepass is **2.103 / 10.445 s** against PnP's
  **33.0 / 102.8 s**.
- t70 measured the vendored Rust Clipper2 **at parity or faster** than the C++
  twin on a 295-case corpus (union / offset / intersection engine-time ratios
  0.559 / 0.587 / 0.650; difference 0.964).
- Therefore the gap is **above the kernel**: operand size and shape, path
  conversion, PolyTree rebuild, allocation, and parallel scheduling. Canonical
  bbox-clips every expensive intersection
  (`clip_clipper_polygons_with_subject_bbox`), caches collision / avoidance by
  `{radius, layer}`, and uses per-vertex `LinesDistancer` queries for overhang
  speed and seam quality. PnP's hot ops are whole-layer.

Without this decomposition, the next implementation ticket is scope without a
denominator: it knows the work set is not the lever, but not which per-op term
to attack first.

## Scope

1. **Instrument the wrapper boundary on the common stages.** For base
   classic-off and base classic-on (and a benchy control), split each hot call
   into:
   - `ExPolygon`/`Point2` → `Path64` conversion (and back through
     `expolygons_from_tree`);
   - the Clipper2 sweep itself (use the existing t70-style phase tags);
   - allocation / `Vec` growth / `Path64` churn;
   - parallel scheduling / rayon fan-out and merge;
   - the host/IR clone or dispatch around the call.

   A probe patch is allowed if reverted; no production edit, no G-code
   reference move, no guest change.
2. **Price the operand-locality shapes**, each as a separate ceiling:
   - **S5** — clip the internal-bridge whole-layer offsets
     (`deep_infill_area` ±1.5 / −4.5 × spacing) to the candidate surface bbox,
     following canonical `bridge_over_infill`'s
     `clip_clipper_polygons_with_subject_bbox` pattern;
   - **S6** — serve planner occupancy from a radius/layer collision cache
     following canonical `TreeSupportData::calculate_collision` /
     `calculate_avoidance`, replacing the two uncached
     `inflate_model_occupancy` sites (t113's 32.7 s / 864 calls);
   - **S7** — the localized `process_external_surfaces` wave expansion that PnP
     omits (`DEV-200`), priced only if it is needed for parity; it is a
     two-way swap, not a deletion.
3. **Separate op count from per-op cost** for every shape, as in t129: a shape
   that only reduces op count cannot explain the 15.7x, and a shape that only
   reduces operand size must show its own measured delta.

## Pre-declared gate (kill gate)

The decomposition graduates a per-op rewrite candidate only if, for at least one
common stage on at least one cell, the measured per-op overhead (conversion +
allocation + scheduling, excluding the Clipper sweep) is **at least the stage's
share of the cell's required cut**, or if a named operand-locality shape's
measured ceiling is **at least the stage's share**. If the per-op overhead is
mostly the Clipper sweep and the operands cannot be localized without changing
canonical-equivalent output, stop and return the finding to
[the destination ruling](128-destination-arithmetic-ruling.md) for re-aim or
park.

The map's **0.80 %** floor still governs any candidate; a shape below the floor
is not a route even if it is exact.

## Acceptance

- A per-op report for the common stages on base classic-off and base classic-on,
  with a benchy control, keeping op count and per-op cost separate.
- A ranked shape list, each shape carrying its measured ceiling, floor multiple,
  and the share of its cell's required cut it serves.
- A recorded go/no-go on the per-op rewrite, on this ticket and in the map's fog.
- No production edit, no G-code reference move, both guest-freshness checks
  untouched.

## Evidence starts

- [t129](../evidence/t129-prepass-work-set-parity/FINDINGS.md) — the work-set /
  per-op split and the route arithmetic.
- [t70](../evidence/t70-clipper-candidate/FINDINGS.md) — Rust/C++ Clipper2
  parity and the `nearbyint` fix.
- [t113](../evidence/t113-strong-planner-phase1/FINDINGS.md) — the planner
  occupancy split (32.7 s / 864 calls) and the failed batch (t125).
- [t75](../evidence/t75-shellclass-phase-b/FINDINGS.md) — the two whole-layer
  internal-bridge offsets and their phase-B share.
- [t68](68-classic-perimeters-host-round-trips.md) — the per-layer phase is
  97–99 % native Clipper ops.
- [t126](../evidence/t126-post-t123-phase-split/FINDINGS.md) — current stage
  walls.

## Answer

_To be recorded on resolution._
