# t149 — `PrePass::MeshAnalysis`: the work set is a bridge detector nobody reads

**Headline.** `PrePass::MeshAnalysis`'s wall is **85.68 %** a bridge-clustering
product with **no production reader**: `find_bridge_clusters` builds a
`HashMap` of **7,383,702** directed half-edges over base's 2.46 M triangles, BFS
clusters them into **3,199** `BridgeRegion`s, and unions each cluster's
footprint — **4,563.2 ms** of the stage's **5,325.8 ms** on base classic-off.
The only production-read output is `OverhangRegion::xy_footprint`, which costs
**529.1 ms**. The t129 S2 shape ("whole-mesh facet union → per-layer overhang
polygons") is therefore **not** the lever: after the unread half is deleted, the
remaining stage is **762.6 ms** (classify + footprint + eligibility + IR
construction), and the per-layer port would have to add canonical's per-layer
diff on top. Deleting the unread half is a **16.18x-floor** tranche on base
classic-off — **24.96 %** of that cell's indicative required cut — graduated as
ticket 150.

## 1. Reader census

`classify_object` (`crates/slicer-core/src/algos/mesh_analysis.rs`) is serial per
object. Both matched fixtures are single-object jobs (base 2,461,234 triangles,
benchy 225,706), so the probe's per-object totals are the stage's totals.

| output | production reader | verdict |
| --- | --- | --- |
| `overhang_regions[].xy_footprint` | `SliceRegionView::derive_needs_support` (`crates/slicer-sdk/src/views.rs`), called from `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` and from all three marshal legs (`crates/slicer-wasm-host/src/marshal/in_.rs`, `.../native.rs`, `.../prepared.rs`) | **read** |
| `overhang_regions[].needs_support` | none. `derive_needs_support` ignores the producer's flag and re-intersects `xy_footprint` against the region's polygons; the flag is only copied into `PreparationOverhangRegion` (`crates/slicer-wasm-host/src/preparation_input.rs`, `crates/slicer-wasm-host/src/preparation_bindings.rs`) | dead |
| `overhang_regions[].{id, max_angle_deg, facet_indices}` | transport only | dead |
| `bridge_regions` | `estimated_surface_classification_bytes` (`crates/slicer-runtime/src/blackboard.rs`, instrumentation only), `crates/slicer-runtime/src/visual_debug_render.rs`, `crates/slicer-wasm-host/src/preparation_input.rs`'s `surface_objects`, and the `crates/slicer-wasm-host/test-guests/whole-print-input-path` **test guest** | no production reader |
| `surface_groups` | resolves `region.nonplanar_surface` (`crates/slicer-wasm-host/src/marshal/in_.rs`, `.../native.rs`, `.../prepared.rs`); the only production producer writes `nonplanar_surface: None` (`crates/slicer-core/src/algos/prepass_slice.rs`) | unread |
| `facet_classes` | bridge clustering in `crates/slicer-core/src/algos/mesh_analysis.rs`, the blackboard size estimate, the prep marshaller, visual debug | no reader once clustering goes |

Two independent checks fix `bridge_regions`' status. First, **no core module
declares the `surface_objects` preparation read**: the only occurrence in
`modules/core-modules` is `lightning-infill`'s
`unavailable_read!(surface_objects(...))`, i.e. a read that must never be
called; its manifest's `input_reads` list is `layer_plan`, `slices`,
`region_map`. Second, t123 already audited the field and kept it **for visual
debug and the whole-print-input guest observation surface**, recording that
"no `PrePass::Slice` consumer remains" ([t123 evidence](../t123-facet-retirement/FINDINGS.md)).

The consumed set is therefore exactly one field: `xy_footprint`.

## 2. Sub-phase price

Probe: `probe.patch` (non-landing) adds `Instant` deltas around four blocks of
`classify_object` and prints them per object. One run per cell, ordinary build,
`RAYON_NUM_THREADS=12`, no warmup.

| cell | cell ms | stage ms | classify | overhang footprint | eligibility | **bridge** | phase sum | unaccounted |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-off | 35,244 | 5,325.8 | 187.0 | 529.1 | 0.2 | **4,563.2** | 5,279.5 | 46.3 |
| base classic-on | 85,525 | 4,832.9 | 135.9 | 510.9 | 0.1 | **4,150.2** | 4,797.1 | 35.8 |
| benchy classic-off | 3,702 | 195.1 | 6.4 | 91.1 | 0.0 | **95.1** | 192.6 | 2.5 |

`classify_ms` is the whole facet loop — transform, normal, area, class — plus
building `all_facet_indices` and retaining `facet_classes`; `footprint_ms` is
the partitioned union of the overhang facets alone; `eligibility` is
`region_needs_support`'s bbox fast path, which never reaches the exact union on
these meshes. The 35–46 ms unaccounted is `ObjectSurfaceData` construction
(`surface_groups`'s all-facet move, `bridge_regions` collect) — **0.7–1.3 %** of
the stage.

A profiled run of the same base classic-off cell reconciles the probe against
the runtime's own accounting: probe total **4,914.1 ms**, `host:mesh_analysis`
stage wall **5,639.5 ms** (**1.1476x**, +725.4 ms). The gap is the profile
scope accounting plus the IR commit/blackboard hand-off; it is disclosed, not
allocated to a sub-phase.

Bridge clustering is not one op. `find_bridge_clusters` inserts **three
directed edges per triangle** into a `HashMap<(u32, u32), Vec<HalfEdgeEntry>>`
(3 x 2,461,234 = **7,383,702** insertions, derived), BFS-walks each cluster,
re-walks every cluster's edges to classify anchor edges, and then
`compute_bridge_metrics` unions each cluster's projected facets (`3,199`
clusters on base, `339` on benchy) and computes anchor runs, anchor width and
span. That the map and BFS dominate the 4.56 s is an **inference** from the two
measured unions, not a separate measurement: the bridge unions partition the
same 326,543 facets the 529 ms footprint union covers, into 3,199 much smaller
sets, so they cannot alone account for 4.56 s.

## 3. What this means for S2

t129 priced S2 as "whole-mesh facet union → per-layer overhang polygons",
ceiling **4,260 ms** = the whole stage. The decomposition says the ceiling was
attached to the wrong half:

- the **union** that the per-layer port would replace is **529.1 ms**;
- the **classification** that feeds it is **187.0 ms** (and the overhang facet
  scan is needed under either design);
- the **eligibility predicate** is **0.2 ms**;
- the rest — **4.56 s**, 85.68 % — is `bridge_regions`, which canonical does not
  compute at all and no production reader consumes.

So the per-layer-overhang-polygons port is a **work-set swap whose entire
ceiling is 762.6 ms** on base classic-off, before paying canonical's per-layer
diff. It is not route work. The S2 member of the sweep-volume family is
answered: **delete the unread half, keep the footprint**.

## 4. Route arithmetic

Same-run shares (the only ones this evidence supports):

| cell | bridge ms | % of cell | x 0.80 % floor | required cut (ratio of record) | bridge as % of that cut |
| --- | ---: | ---: | ---: | ---: | ---: |
| base classic-off | 4,563.2 | **12.95 %** | **16.18x** | 51.88 % (2.078x, t145) | **24.96 %** |
| base classic-on | 4,150.2 | **4.85 %** | **6.07x** | 70.20 % (3.356x, t148) | **6.91 %** |
| benchy classic-off | 95.1 | **2.57 %** | **3.21x** | 51.05 % (2.043x, t145) | **5.03 %** |

The cut columns are **indicative**: they apply a cited ratio of record to this
run's cell wall, and the cell wall is from a different window than the ratio
(see disclosures). What is measured in-window is the % of cell and the floor
multiple.

Composition on base classic-off, if the whole bridge half disappears and
nothing else moves: **35.244 s → 30.681 s**, i.e. the ratio of record moves
**2.078x → 2.387x** and the required cut from 51.88 % to 58.11 %. That is a
**12.95 %-of-cell** tranche — the same order as t143's 12.80 % on base
classic-on — not a destination step, but it is the largest single unread work
set measured since t142.

## 5. Disclosures

- **One run per cell, no warmup, no paired A/B.** The sub-phase times are
  `Instant` deltas, not wall claims; only the ratios inside a run are quoted.
- **Probe overhead.** The probe adds four `Instant::now()` calls and two
  `eprintln!`s per object; it is recorded as `probe.patch` and reverted.
- **The profiled run is faster than the non-profiled one** (33,913 ms vs
  35,244 ms) and its stage wall exceeds the probe total by 725.4 ms. Both are
  one sample; no stability claim is taken.
- **Cross-window cells.** t143 recorded base classic-on at 74.70 s; this run
  measures 85.53 s. The ratio-of-record columns therefore do not compose with
  these walls as a scoreboard; they are stated as indicative.
- **Overhang facet count.** 326,543 here against t129's 326,546. The three-facet
  difference is a recount difference on a byte-identical fixture; the measured
  number is this take's.
- **`needs_support` is dead but not removed.** The field is part of the
  preparation surface's shape; the candidate leaves it in place.
- **Visual debug and the observation test guest are the only visible output.**
  Deleting the computation empties `bridge_regions`, which both surfaces print;
  that is a diagnostic disclosure for the candidate, not a G-code change.

## 6. Candidate

Graduated as [t150](150-meshanalysis-bridge-retirement.md): stop computing
`bridge_regions`, keep the field empty, and delete the clustering machinery.
Its pre-declared gate is a 16-cell byte-identical G-code campaign — the reader
census says nothing can consume the product, so **any** output difference
falsifies this ticket rather than being absorbed by the candidate.

## Evidence index

- `census.json` — the probe rows, the cited ratios, and the profiled run.
- `probe.patch` — the non-landing probe applied to
  `crates/slicer-core/src/algos/mesh_analysis.rs`.
- `verify.py` — re-derives all 43 published numbers from `census.json`;
  `--self-test` rejects 4/4 negative controls.
- `REPRODUCE.md` — build, probe application, the three slice commands, and the
  verification step.
