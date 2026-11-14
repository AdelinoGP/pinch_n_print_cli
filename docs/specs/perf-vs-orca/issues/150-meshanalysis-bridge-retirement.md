# MeshAnalysis bridge retirement: stop computing a product no reader consumes

Type: task / measured deletion candidate
Label: wayfinder:task
Status: resolved (2026-11-13; measured deletion — the unread bridge half is
  gone, `bridge_regions` is an empty vector, the MeshAnalysis stage falls
  4,445 → 732 ms on base classic-off, all 16 output runs are byte-identical to
  the t148 active set, and the paired medians are −12.08 % / −4.06 % on the two
  base cells. See ## Answer.)
Assignee: 2026-11-13 OpenCode wayfinder session — the human said "Continue the perf vs orca map"; 150 was the first unblocked, unclaimed AFK frontier ticket in order (151 is the other)
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [MeshAnalysis work set: which outputs does production read](149-meshanalysis-work-set-readers.md) —
  the stage is **5,325.8 ms** on base classic-off and **4,563.2 ms (85.68 %)** of
  it is `find_bridge_clusters` + `compute_bridge_metrics`: 7,383,702 directed
  half-edge insertions, 3,199 BFS clusters, a per-cluster footprint union and
  anchor-width/span metrics. No production reader consumes `bridge_regions`;
  the only consumed output is `overhang_regions[].xy_footprint` (**529.1 ms**).
  See [`evidence/t149-meshanalysis-readers/`](../evidence/t149-meshanalysis-readers/FINDINGS.md)
  (`verify.py` 43 claims, 4/4 negative controls).

## Question

Delete the unread half of `classify_object`
(`crates/slicer-core/src/algos/mesh_analysis.rs`) and prove the slice is
unchanged, so that the measured **4.15–4.56 s** on the base cells stops being
paid.

Decide and land, in order:

1. **Stop producing `bridge_regions`.** Remove the `compute_bridge_metrics` call
   from `classify_object` and return `bridge_regions: Vec::new()`. Keep
   `SurfaceClassificationIR`'s types, field, and schema version unchanged — an
   empty vector is the same schema, as t142 did for
   `overhang_quartile_polygons`.
2. **Delete the now-unreachable machinery.** `find_bridge_clusters`,
   `build_half_edge_map`, `BridgeCluster`, `DirectedEdge`, `HalfEdgeEntry`,
   `edge_key`, `compute_bridge_metrics`, `is_bridge_candidate`,
   `build_anchor_runs`, `metrics_projection_axis_deg`, `compute_anchor_width_mm`,
   `compute_bridge_length_mm`. Keep `compute_xy_footprint` /
   `union_footprint`, `classify_facet`, `region_needs_support`, and the
   `OverhangRegion` construction.
3. **Keep `facet_classes` and `surface_groups`.** They are equally unread, but
   they cost memory (2.46 M `FacetClass` + a 2.46 M-entry all-facet index), not
   the measured wall; dropping them is a separate memory decision and would
   enlarge the diagnostic delta.
4. **Diagnostics.** `visual_debug_render.rs` prints `bridge_regions` and the
   `whole-print-input-path` test guest records `bridges=<n>`; both become empty.
   Update the test guest's expectation to `bridges=0` and record the visual-debug
   disclosure in the FINDINGS. t123 kept these fields for exactly those two
   surfaces, so this is a known, bounded consequence.
5. **Retire the tests that only exercise the deleted clusterer.** The
   `bridge_detector_tdd` cases and the `algo_mesh_analysis_tdd` bridge
   assertions lose their subject with the product; keep the footprint/union and
   overhang-classification tests. Per `docs/22_test_quality.md`, retire with a
   stated justification, never by deleting the assertion and keeping the name.
6. **Ceremony.** 16-cell output (8 cells x {baseline, candidate}) must be
   **byte-identical** to the active reference set, and the paired A/B must
   show the saving. Any output difference falsifies t149's reader census; it is
   not a number to absorb.

## Pre-declared gate

- **Identity, 8/8.** G-code byte-identical to the t148 active reference set
  (supports-off inherits t144's four hashes; supports-on is t148's four).
  A delta audit must show `bridge_regions` empty and every other IR field
  equal, with the schema version unchanged.
- **Wall, base cells.** Positive paired median wall improvement on base
  classic-off and base classic-on, at or above the standing 0.80 % floor
  (t149 measured 16.18x and 6.07x the floor respectively), 3 interleaved pairs
  per cell, process CPU not contradicting.
- **Wall, benchy cells.** The ceiling is 2.57 % of the cell and 95 ms; the
  medians must not regress and the measured saving is disclosed even if it
  lands inside the box's null band. A benchy cell failing to clear the floor is
  not a gate failure — its ceiling is 3.21x the floor but 95 ms of a 3.7 s cell
  is inside this box's own repeat spread — but it must be reported.
- **No CPU exception invented after the fact.** Measure the null band first
  (identical-arm control) and read the CPU clause against it, as t135 did.

## Route arithmetic (indicative)

At t149's same-run shares and the cited ratios of record, removing the whole
bridge half moves base classic-off from **2.078x to 2.387x** and its required
cut from **51.88 % to 58.11 %**; base classic-on keeps **6.91 %** of its 70.20 %
cut and the four benchy cells **~5 %** each. A tranche, not a destination step.

## Answer

**Resolved 2026-11-13 — measured deletion, banked.** The unread bridge half is
gone: `classify_object` returns `bridge_regions: Vec::new()` and the clusterer
(`find_bridge_clusters`, `build_half_edge_map`, `BridgeCluster`,
`DirectedEdge`, `HalfEdgeEntry`, `edge_key`, `compute_bridge_metrics`,
`is_bridge_candidate`, `build_anchor_runs`, `AnchorRun`,
`metrics_projection_axis_deg`, `compute_anchor_width_mm`,
`compute_bridge_length_mm`) is deleted. The IR type, field and schema version
are unchanged. Headline: the `PrePass::MeshAnalysis` stage falls **4,445 → 732
ms (−83.5 %)** on base classic-off and **4,616 → 582 ms (−87.4 %)** on base
classic-on; whole-slice paired medians are **−12.08 %** (15.09x the 0.80 %
floor) and **−4.06 %** (5.08x) on those two cells, and −1.77 … −9.37 % on the
other six. **All 16 output runs (8 cells x 2 arms) are byte-identical to the
t148 active reference set**, every A/B pair carries the same hash on both arms,
and all runs are `degraded: false` with zero `exact-Z occupancy` diagnostics.

Identity gate: **8/8 cells, 16/16 runs**, zero output difference — t149's
reader census holds. Paired gate: both base cells positive and above the floor;
the identical-arm null control (6 pairs, base classic-off) bounds a pair at
**±3.67 % wall / ±0.17…+0.50 % CPU**, so base classic-off's 12.08 % is 3.3x the
band and base classic-on's 4.06 % is only 1.1x it (disclosed; all six pairs are
candidate-faster). Benchy classic-off's −1.77 % is **inside** the null band and
is reported, not claimed, exactly as the ticket allowed. CPU is lower or flat
in all eight cells (−1.77 … −0.10 %), so the CPU clause is not contradicted.

Route: base classic-off **2.078x → 1.827x** (cut 51.88 % → 45.27 %), base
classic-on 3.356x → 3.220x, the other six −1.77 … −9.37 % of their ratios of
record — a tranche of t142's order, not a destination step.

Two corrections to the ticket's instructions, both verified against the tree:

- **Item 4's `bridges=0` was not delivered** because the whole-print-input
  guest is fed a **literal** `SurfaceClassificationIR` (`Product::Literal` in
  `crates/slicer-wasm-host/tests/contract/whole_print_input_driver.rs`,
  `literal_surface`), not mesh-analysis output; its synthetic bridge exercises
  the transport path and still passes. The visual-debug surface does go quiet,
  which is disclosed.
- `algo_mesh_analysis_tdd` has no bridge assertions to retire; the eight
  retired cases all live in `crates/slicer-runtime/tests/unit/bridge_detector_tdd.rs`
  (AC-1…AC-6 + NEG-3), whose remaining four cases drive the slice-level
  `bridge_areas` product and stay.

Evidence: [`evidence/t150-meshanalysis-retirement/`](../evidence/t150-meshanalysis-retirement/FINDINGS.md)
(`verify.py` 71 checks, 5/5 negative controls; `reduce_t150.py` regenerates
`census.json`; `run_t150.ps1` is the campaign driver).

