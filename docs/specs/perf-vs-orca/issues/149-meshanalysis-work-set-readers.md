# MeshAnalysis work set: which outputs does production read, and what is the unread half worth?

Type: task / Phase-1 attribution
Label: wayfinder:task
Status: resolved (2026-11-13; Phase-1 attribution — the stage's read set is one
  field and **85.68 %** of its wall is a bridge detector with no production
  reader; the t129 S2 "per-layer overhang polygons" port is answered as not the
  lever and the deletion is graduated as [t150](150-meshanalysis-bridge-retirement.md))
Assignee: 2026-11-13 OpenCode wayfinder session — the human said "Continue the perf vs orca map, pick an AFK ticket"; **no AFK ticket was on the frontier** (146 is the human's HITL grilling; 1–148 are resolved), so the map's route arithmetic graduates the largest remaining unpriced shape of the **sweep-volume work-set family** from Not yet specified: S2, the whole-mesh facet union in `PrePass::MeshAnalysis` ([t129](../evidence/t129-prepass-work-set-parity/FINDINGS.md) §2.5, ceiling **4,260 ms** base classic-off, **12.5x** the floor, **15.3 %** of that cell's required cut).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Prepass work-set and per-op parity against canonical v2.5.0](129-prepass-work-set-per-op-parity.md)'s
  S2 row, which priced the stage wall as a ceiling and named the shape
  ("whole-mesh facet union → per-layer overhang polygons") but left the
  replacement unpriced: *"The clean ceiling is the stage wall (4,260 ms
  base-off), less canonical's per-layer overhang diff."*

## Question

`PrePass::MeshAnalysis` is **4,260 ms** on base classic-off (162 / 179 ms on
the two benchy cells) and produces a `SurfaceClassificationIR` whose parts may
not all have a production reader. Which of its outputs does the pipeline
actually read, and what is the priced ceiling of deleting the rest — before
anyone ports canonical's per-layer overhang polygons?

Decide and record, in order:

1. **Reader census, by symbol.** For each `ObjectSurfaceData` output
   (`facet_classes`, `surface_groups`, `bridge_regions`,
   `overhang_regions[].{needs_support, xy_footprint, max_angle_deg,
   facet_indices}`), name the production caller and the crate-qualified path,
   or establish that the only readers are the marshaller, the visual-debug
   renderer, and tests. The candidate consumer to check first is
   `SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`),
   which reads `overhang_regions[].xy_footprint` — not
   `OverhangRegion::needs_support`.
2. **Sub-phase price.** Decompose the stage's wall into the facet
   classification loop (2.46 M triangles, transform + normal + area + class),
   the overhang footprint union (326,546 projected triangles, t76's partition),
   `region_needs_support`'s bbox fast path, and `compute_bridge_metrics`
   (7.4 M directed half-edge insertions, BFS clusters, per-cluster union, anchor
   runs). The last is the suspected dominant term and the one whose product has
   no reader.
3. **Candidate shape.** If the unread half is the dominant term, state the
   smallest exact deletion: keep `overhang_regions[].xy_footprint` (the only
   production-read output), stop computing the rest, and say what happens to
   the IR fields (empty vs removed — a schema decision, and the visual-debug
   disclosure it implies).
4. **Route arithmetic.** Convert the ceiling into `%` of the cell's required
   cut on the post-t148 tree (base classic-off's ratio of record is **2.078x**,
   required cut **51.9 %**; the base classic-off cell was **74.70 s** at t143)
   and graduate the measured candidate only if it clears the standing 0.80 %
   floor by a wide multiple.

## Pre-declared gate

Phase-1 attribution: **no production change** is required to resolve this
ticket. Bars:

- the reader census names a concrete caller or establishes absence with the
  grep surface, never "seems unused";
- the sub-phase table sums to the stage wall within a disclosed tolerance, and
  the probe is a recorded, non-landing patch;
- the ceiling is quoted against the **current** reference set and cell wall
  (t148's active set; t143's 74.70 s base classic-off), never t129's
  pre-t142/t143 numbers;
- any candidate graduated carries its own pre-declared gate, not this ticket's.

## Answer

**The stage's read set is one field, and 85.68 % of its wall is a product no
production code reads.** `classify_object`'s only consumed output is
`OverhangRegion::xy_footprint`, read by `SliceRegionView::derive_needs_support`
(`crates/slicer-sdk/src/views.rs`) from `support_analysis_producer.rs` and all
three marshal legs. `bridge_regions` — `find_bridge_clusters` +
`compute_bridge_metrics` — is **4,563.2 ms of 5,325.8 ms** on base classic-off
(7,383,702 directed half-edge insertions, 3,199 BFS clusters, one footprint
union per cluster); its only readers are the blackboard size estimate, the
visual-debug renderer, the prep `surface_objects` read that no core module
declares, and the `whole-print-input-path` test guest. `overhang_regions[].needs_support`
is also dead — the SDK re-derives eligibility from `xy_footprint` — and
`surface_groups` resolves only `region.nonplanar_surface`, which production only
ever writes as `None`.

**Sub-phase, base classic-off:** classify 187.0, overhang footprint 529.1,
eligibility 0.2, bridge 4,563.2, phase sum 5,279.5, unaccounted 46.3 (0.87 %).
Base classic-on is 4,832.9 ms with bridge 4,150.2; benchy classic-off is 195.1 ms
with bridge 95.1. A profiled run reconciles the probe against the runtime's own
accounting — 4,914.1 ms probe vs 5,639.5 ms `host:mesh_analysis` (1.1476x).

**S2 is answered and is not the lever.** The per-layer-overhang-polygons port
would replace the 529.1 ms union; after deleting the unread half the remaining
stage is 762.6 ms, before paying canonical's per-layer diff. The 4.56 s is the
bridge detector.

**Route:** the bridge half is **12.95 %** of the base-off cell (35.24 s),
**16.18x** the 0.80 % floor and **24.96 %** of that cell's indicative required
cut; base-on **4.85 % / 6.07x / 6.91 %**; benchy-off **2.57 % / 3.21x / 5.03 %**.
Composition on base-off if the whole half disappears: **2.078x → 2.387x**, cut
**51.88 % → 58.11 %** — indicative, because the ratio of record and this run's
cell wall are from different windows. A tranche of t143's order, not a
destination step. Graduated [t150](150-meshanalysis-bridge-retirement.md), whose
gate is a 16-cell byte-identical campaign: the census says nothing can consume
the product, so any output difference falsifies this ticket.

Evidence: [`evidence/t149-meshanalysis-readers/`](../evidence/t149-meshanalysis-readers/FINDINGS.md)
(`verify.py` 43 claims, 4/4 negative controls; `probe.patch` is the non-landing
probe; production restored).
