# Common-stage per-op cost decomposition: where does the 9.8–15.7x go?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: resolved (2026-11-09; Phase-1 decomposition — **the per-op wrapper is
  not the lever; the sweep is 98.2–99.9 % of every common stage, operand size
  is the gap, and the planner's 2.19x parallelism is the largest measured
  scheduling lever**. See ## Answer)
Assignee: 2026-11-09 OpenCode wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 131 is the first unblocked, unclaimed
  AFK frontier ticket in order (130 is the human's grilling).
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

**Resolved 2026-11-09 as a Phase-1 per-op decomposition.** The per-op wrapper
is **not** the lever. On every common prepass stage the Clipper2 sweep holds
**98.2–99.9 %** of the measured primitive thread-time; path conversion, WIT
marshalling, PolyTree rebuild and allocation together hold **0.1–1.8 %**. The
9.8–15.7x phase gap is bought by **operand size inside the sweep**, and the
largest measured scheduling lever is the planner's **2.19x** effective
parallelism.

Four findings:

1. **The first gate branch fails by two orders of magnitude.** On base
   classic-off the whole non-sweep total across the five common stages is
   **543 ms of 280,657 ms** of primitive thread-time (**0.19 %**); on base
   classic-on it is **1,075 ms of 517,928 ms** (**0.21 %**); benchy
   classic-off **0.42 %** and benchy classic-on **0.56 %**. The largest single
   non-sweep bucket is the planner's path conversion (**383.3 ms**), the
   largest PolyTree rebuild is `OverhangAnnotation`'s (**97.7 ms**), and the
   largest WIT marshal is base-on `SupportGeometry`'s (**31.7 ms**). As a share
   of each stage's required contribution to its cell's cut, the overhead is
   **0.9–2.6 %**.
2. **The gap is operand volume.** `ShellClassification` runs **19.5 clipper
   calls per layer** (inside canonical's 15–25 range) at **4,608 points per
   call** — **44,463,545 points**, **89,825 per layer** — and the planner runs
   **290 calls per layer** at **2,040 points per call** — **293,219,629
   points**, **592,363 per layer**. t70 measured the Rust Clipper2 at parity
   with the C++ twin, so the 7.0x / 6.7x stage ratios cannot be kernel
   throughput or wrapper cost: they are the size of what each op is fed.
3. **S5 and S6 are now priced; only S6 is exact.** S5 (internal-bridge offsets
   on a candidate-bbox operand): whole-layer **22,964 ms** → windowed
   **1,937 ms** of thread-time on 130 base-off layers (**16.06x / 8.81x**),
   wall-equivalent **2,520 ms** (**7.4x** floor), but **not exact** —
   **115/130** positive and **36/42** negative layers match, max **0.71 /
   0.59 mm²**, net **−1.27 / +1.18 mm²**. S6 (occupancy memo): the `+0.35 mm`
   family is **864 calls over 433 distinct payloads**, and the **431**
   duplicate calls cost **14,227.7 ms** of thread-time (**6,504 ms** wall at
   2.19x, **6.4x** floor); the paired `−0.6 / −0.8 mm` collision family is
   **1,350 calls over 1,350 distinct payloads** — **zero** memo hits.
4. **The planner is the scheduling outlier.** **122,657.6 ms** of sweep
   thread-time delivered in **56,289 ms** of wall (**2.19x**; benchy-on
   **1.68x**), against **8.34–11.80x** on the shell, overhang, seam and slice
   stages. At 4x the planner would be **30,664 ms**, a **25,625 ms** cut =
   **27.7 %** of base-on's required prepass cut — larger than S5 and S6
   combined on that cell, and output-neutral in principle.

**Go/no-go: NO-GO for a per-op rewrite; GO (narrowed) for operand locality and
planner scheduling.** The ticket's stop rule does not fire: the overhead *is*
mostly the sweep, but the operands *can* be localised — S6 exactly, S5 with a
measured bounded delta — and the planner's parallelism is a separate,
unmeasured-until-now wall lever.

**Gate verdict.** The first branch (overhead ≥ the stage's share of the cut)
fails on every stage; the second branch (a named shape's ceiling ≥ the stage's
share) fails literally too — S5 is **17.7 %** of the shell stage's share, S6
**12.8 %** of the planner's share, and the planner-parallelism ceiling is
**50.6 %** of it at 4x. The literal per-stage gate fires, but the route-level
program is not killed, exactly as in [t129](129-prepass-work-set-per-op-parity.md).

**Graduated:** [S6 occupancy memo](132-planner-occupancy-memo.md) (exact,
AFK), [S5 windowed internal-bridge offsets](133-internal-bridge-windowed-offsets.md)
(bounded delta, AFK), and [planner effective parallelism](134-planner-effective-parallelism.md)
(AFK). The destination's 58–79 % cut still needs the sweep's total operand
volume reduced; none of these three shapes reaches a cell's cut on its own.

**Ledger corrections.** (1) t129's suspect list is corrected: conversion,
allocation, rebuild and marshalling are 0.1–1.8 % of primitive time and cannot
be a lever; only operand size inside the sweep survives. (2) t113/t129's S6
ceiling is **2.3x overstated** — the cacheable part is **14.2 s**, not 32.7 s,
because a memo cannot remove the first call per payload, and the sibling
collision family has no duplicates at all. (3) t126's stage walls reproduce
within +4.1 % on the prepass; shares here are taken within this take.

**Evidence:** [`evidence/t131-per-op-decomposition/`](../evidence/t131-per-op-decomposition/FINDINGS.md)
(`verify.py` 150/150 checks, prose re-derived; `--self-test` rejects 11/11
negative controls; four cells × (`probe`, `probe+s5`), every G-code hash equal
to the t123 candidate-ordinary reference). No production edit, no G-code
reference move; the probe is preserved as `probe.patch` and reverted,
`cargo xtask build-guests --check` exits 0, and the rebuilt ordinary binary
reproduces `90638bf3…` on benchy classic-off.

**Disclosures.** One probe pass per cell, ordinary arm, single pass; the probe
adds two clock reads per primitive and its stage walls run 3–13 % above t126's,
so they are used only for within-take shares and no wall claim is taken from
them. Thread-time is accumulated across rayon workers; the thread/wall column
is effective parallelism, not utilisation. Allocation is not isolated: the
input vectors are inside `conv_in`, the output vectors inside `rebuild`, the
engine's internal allocation inside `sweep`. Canonical has no per-op
measurement, so its op counts are source-derived at the `v2.5.0` pin. The S5
window is a probe construct, not a proposed patch, and its non-equivalence is
a property of the operands. The S6 census keys on payload content (a lower
bound for a canonical `{radius, layer}` key) and the two-calls-per-payload
structure holds while a layer carries one region.
