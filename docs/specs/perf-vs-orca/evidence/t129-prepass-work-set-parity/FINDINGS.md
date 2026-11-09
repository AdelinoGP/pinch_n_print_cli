# t129 — Prepass work set and per-op parity against canonical v2.5.0

Route: [Prepass work-set and per-op parity against canonical v2.5.0](../../issues/129-prepass-work-set-per-op-parity.md).
Take: 2026-11-08, Phase-1 source decomposition at the t128 pin. No production
edit, no candidate, no G-code reference move. Stage walls are t126's
instrumented brackets (attribution only, never a wall claim); ratios of record
are t124's uninstrumented paired medians. Canonical is cited by **file +
function** at `v2.5.0` = `40ecc66f78ed72dc368e0c1ebe40d77f00cc8a19`.

The machine-readable census is [`census.json`](census.json); the source
citations are [`canonical_citations.json`](canonical_citations.json);
[`verify.py`](verify.py) re-derives every number below from the census and
checks it against this file.

## Verdict

**The work set is not the lever. Canonical does _more_ per-layer Clipper work
than PnP in the shell/external-surface cluster, yet its whole pre-G-code phase
is 15.7x / 9.8x cheaper on base classic-off / -on. The gap is per-op cost and
operand locality, not op count.**

Three findings follow, and together they re-scope the t128 route:

1. **The clean PnP-local work-set deletions are a real but partial tranche.**
   Replacing the overhang quartile bands, the whole-mesh facet union, and the
   seam inset removes **15.5 s** of base classic-off's 42.15 s cell = **55.8 %
   of the 27.8 s cut** the cell needs. On base classic-on the same deletions are
   **1.6 s of the 92.0 s cut = 17.5 %**; on the benchy cells **32.1 % / 14.9 %**.
   A work-set port alone cannot reach the destination.
2. **Canonical's prepass is not free, and "prepass at canonical cost" is not the
   same counterfactual as "zero prepass".** Substituting canonical's measured
   prepass (2.103 / 10.445 / 0.0505 / 0.5498 s) for PnP's, while holding PnP's
   other phases, wins **base classic-off only** (0.787x); it **loses** base
   classic-on (1.012x, a tie), benchy classic-off (1.053x) and benchy classic-on
   (1.075x). Full geometry parity wins all four (0.335x / 0.661x / 0.550x /
   0.704x). The destination is a **full engine-parity program**, and the benchy
   per-layer phase is part of it, not a last mile.
3. **The kernel is not the gap either.** t70 measured the vendored Rust Clipper2
   at parity or faster than the C++ twin on a 295-case corpus. The 15.7x is
   therefore above the kernel: it is the operand size and shape PnP feeds the
   kernel, plus the host plumbing around it. Canonical bbox-clips its expensive
   intersections (`clip_clipper_polygons_with_subject_bbox`), caches collision /
   avoidance by radius and layer, and computes overhang speed and seam quality
   as per-vertex distance queries. PnP's hot ops are whole-layer polygon
   operations — 3 overhang-band offsets + ~10 clips per layer, a 326,546-facet
   union, whole-layer `deep_infill_area` offsets, 495 per-region seam insets —
   and its occupancy inflation is uncached at two planner sites.

**Go/no-go: GO for the engine rewrite, but re-scope the first implementation
ticket from "work-set port" to "operand locality / per-op cost", with the three
clean deletions as its measured first tranche.** The literal single-stage gate
in the ticket fires — no single stage's ceiling reaches its cell's required
prepass fraction (the largest, overhang bands, is 23.0 % of the base-off cell
against 84 %) — but the route-level program is not killed: the decomposition
finds both extra work and a >15x per-op gap, so PnP does **not** already compute
canonical's work set at canonical's per-op cost. The next gate must price the
per-op half, not another work-set shape.

## 1. The stage census

t126's instrumented brackets on the four classic cells (ms; one pass per cell,
ordinary arm). The seven stages reconcile with the prepass phase to within
0.23–1.76 %; the residual is uninstrumented sub-stages and bracket overhead.

| stage | benchy off | benchy on | base off | base on |
| --- | ---: | ---: | ---: | ---: |
| `PrePass::SupportGeometry` | 39 | 2,842 | 296 | 55,798 |
| `PrePass::ShellClassification` | 1,633 | 1,668 | 14,698 | 15,300 |
| `PrePass::OverhangAnnotation` | 617 | 596 | 9,689 | 10,309 |
| `PrePass::SupportAnalysis` | 0 | 1,024 | 0 | 13,084 |
| `PrePass::MeshAnalysis` | 162 | 179 | 4,260 | 4,278 |
| `PrePass::Slice` | 205 | 232 | 2,110 | 2,117 |
| `PrePass::SeamPlanning` | 171 | 191 | 1,718 | 1,631 |
| **sum** | **2,827** | **6,732** | **32,771** | **102,517** |
| prepass phase (share × cell) | 2,878 | 6,790 | 33,003 | 102,752 |

The cell and ratio context (t126 / t124): `slice_complete` **4,634 / 8,887 /
42,150 / 117,970 ms**; ratio of record **2.70 / 3.61 / 2.95 / 4.63x**; cut to
beat **63 / 72 / 66 / 78 %**; prepass fraction needed **101 / 95 / 84 / 90 %**.
The 0.80 % floor is **46 / 80 / 340 / 1,010 ms**.

## 2. The three-column decomposition

Each row separates the canonical work set, the PnP work set, and the
op-count-versus-per-op verdict. Canonical citations resolve at the pin; the
machine-readable list is [`canonical_citations.json`](canonical_citations.json).

### 2.1 Slicing — `PrePass::Slice`

| column | content |
| --- | --- |
| canonical work set | `PrintObject::slice` (`PrintObjectSlice.cpp`) drives `slice_volumes` → `slice_mesh_ex` (`TriangleMeshSlicer.cpp`): **one triangle pass over all Z planes** (each triangle binary-searches its own span), loop chaining parallel over layers, polygon assembly parallel over layers. `resolution` simplifies to 0.0025 mm when configured. |
| PnP work set | `batch_slice_objects_by_layer` (`crates/slicer-core/src/algos/prepass_slice.rs`): **one batched `slice_mesh_ex` per object**, then `classify_unsupported_bridge_areas` per layer (`opening_ex(diff(current_raw, safety_offset(previous_raw)), 0.1 × external_perimeter_width)`) and region mapping. Same one-walk shape. |
| op count vs per-op cost | **Comparable work set, comparable op count.** PnP's Slice is 2,110 ms on base-off; canonical's slicing is part of its 2,103 ms whole prepass. The gap here is implementation cost, not work. **Not a work-set route.** |

### 2.2 Shell and solid classification — `PrePass::ShellClassification`

| column | content |
| --- | --- |
| canonical work set | `PrintObject::detect_surfaces_type` (`PrintObject.cpp`): per layer **3 diffs + 2 openings** (top, bottom, internal), plus a crack block only when top and bottom overlap. `PrintObject::discover_vertical_shells` is the active vertical-shell pass at the default `ensure_vertical_shell_thickness = evstAll` (per-layer parallel cache: 3 offsets + 3 unions). `PrintObject::discover_horizontal_shells` **early-outs at `evstAll`** and does not run. `PrintObject::bridge_over_infill` adds ~7 Clipper ops per layer and ~15 per candidate, with bbox-clipped intersections (`clip_clipper_polygons_with_subject_bbox`) and a cached `boundary_by_spacing`. `LayerRegion::process_external_surfaces` (live `#if 1` branch) adds 3 zone unions + bridge/bottom/top wave expansion + trims — **30–40 Clipper ops per layer** and the largest per-layer op consumer in the pre-G-code phase. |
| PnP work set | `slice_postprocess_prepass::compute_region_updates` (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`): Pass 1 = **2 diffs + ≤4 offsets** per layer; Pass 2 = the `discover_horizontal_shells` shrinking-shadow cascade (seeds × depth × intersection + offset + union); one internal-solid diff. `gate_internal_bridge_sites` then runs the `bridge_over_infill` qualification: **2 whole-layer Miter offsets** on `deep_infill_area` (+1.5 × spacing and −4.5 × spacing), per-surface qualification offsets/intersections, and the angles step. **`process_external_surfaces` is not ported** (DEV-200). |
| op count vs per-op cost | **PnP's op count is _lower_ than canonical's** (≈15–25/layer vs ≈45–60/layer), and PnP omits canonical's largest per-layer consumer. Yet PnP's stage is **14,698 ms** on base-off against canonical's whole 2,103 ms prepass — **at least 7.0x** the entire phase. **Per-op / operand cost.** The two whole-layer offsets were 78–81 % of the **serial phase-B** wall before t75 parallelised that phase (t75); canonical bbox-clips its downstream intersections and caches the per-spacing boundary. A pure work-set port is a **swap** — it would delete Pass 2 but add `discover_vertical_shells` and the wave expansion — not a saving. |

### 2.3 Overhang annotation — `PrePass::OverhangAnnotation`

| column | content |
| --- | --- |
| canonical work set | Slice phase: `PrintObject::detect_overhangs_for_lift` — **1 diff + 1 offset + 1 offset2** per layer, parallel over layers, producing one binary `Layer::loverhangs` set consumed only by `GCode::needs_retraction`. Overhang **speed** is not a slice-phase polygon construction: `precompute_overhang_layer` (`GCode/ExtrusionProcessor.hpp`) builds one `LinesDistancer<Linef3>` from the lower layer's `lslices` and queries **one signed distance per perimeter/bridge vertex** during G-code emission; band membership is a numeric comparison against `{0.1w, 0.25w, 0.5w, 0.75w, 0.87w, 1.0w}`, with linear interpolation. **Canonical has no 4-band polygon partition and no per-layer overhang-speed clips.** |
| PnP work set | `annotate_overhangs` (`crates/slicer-core/src/algos/overhang_annotation.rs`): per band layer **3 round offsets** at `line_width × {0.5, 1.0, 1.5}` plus ~**9.8 clips** to partition the overhang into 4 quartile bands; 1,455 offsets and 4,859 clips over 485 band layers on base-off (t84). The bands feed `overhang_quartile` per wall/infill vertex. |
| op count vs per-op cost | **PnP runs ~3x canonical's op count and does the work in the prepass instead of at emission.** 85.4 % of the stage is the three whole-layer offsets (t84). The clean shape is to replace the band polygons with canonical's per-vertex distance field at emission; the ceiling is the stage wall (9,689 ms base-off), less the replacement cost. **This is a genuine work-set + operand-locality shape.** |

### 2.4 Seam planning — `PrePass::SeamPlanning`

| column | content |
| --- | --- |
| canonical work set | `SeamPlacer::init` (`GCode/SeamPlacer.cpp`), at G-code export: candidates are the printed external-perimeter loop points (`collect_points`); per-vertex `overhang` / `unsupported_dist` / `embedded_distance` from `LinesDistancer<Linef>`; per-layer parallel candidate/visibility/pick; sequential whole-object B-spline alignment; `place_seam` per loop at emission. **No `offset` call exists in `SeamPlacer.cpp`.** |
| PnP work set | `seam-planner-default` (`modules/core-modules/seam-planner-default/src/lib.rs`): per region, `inset_delta_mm = −0.5 × width`, `offset_polygons_batch`, `project_point_onto_inset_boundary`. That is **one serial native offset per region** — 495 calls on base-off, 92.4 % of the stage (t84). |
| op count vs per-op cost | **Canonical has zero inset offsets; PnP has one per region per layer.** The clean ceiling is 92.4 % of 1,718 ms = **1,587 ms** base-off. The replacement is canonical's loop projection, whose cost is inside Orca's 1,349 ms emit phase. **A genuine work-set deletion, but small.** |

### 2.5 Mesh analysis — `PrePass::MeshAnalysis`

| column | content |
| --- | --- |
| canonical work set | No whole-mesh analysis. Overhang geometry is per layer: `detect_overhangs_for_lift` on supports-off; `TreeSupport::detect_overhangs` on supports-on (`diff` + `offset` + cantilever/sharp-tail probes). The support-necessity probe is `PrintObject::is_support_necessary`, and it only runs when supports are disabled. |
| PnP work set | `execute_mesh_analysis` / `classify_object` (`crates/slicer-core/src/algos/mesh_analysis.rs`): classifies **2.46 M facet normals**, builds a **half-edge adjacency map** (7.4 M edges), BFS-clusters bridge facets, and unions **326,546 projected overhang triangles** into an `xy_footprint`. t76 partitioned the union from 15–16 s to 0.45–0.52 s; the stage is still **4,260 ms** on base-off. |
| op count vs per-op cost | **Extra work canonical does not do.** `facet_classes`, `bridge_regions` and `surface_groups` have no production reader outside `needs_support` and visual debug; the footprint is a coarse support-eligibility predicate. The clean ceiling is the stage wall (4,260 ms base-off), less canonical's per-layer overhang diff. **A genuine work-set deletion.** |

### 2.6 Support geometry and analysis — `PrePass::SupportGeometry` + `SupportAnalysis`

| column | content |
| --- | --- |
| canonical work set | `TreeSupport::generate` (`Support/TreeSupport.cpp`) for `tree_strong` (the organic generator is a different style and returns early): `detect_overhangs` (3–5 full-layer booleans per layer), `generate_contact_points`, `drop_nodes` with a radius/layer-keyed `calculate_collision` / `calculate_avoidance` cache, `smooth_nodes`, `draw_circles` (~10–14 layer-level booleans + per-node circles), `generate_toolpaths`. `PrintObjectSupportMaterial` is not executed. |
| PnP work set | Host `PrePass::SupportGeometry` is a **coarse outline producer** (296 ms base-off; 9 ms of the 91,902 ms pre-t123 bracket on base-on, t107) — negligible. The planner module is a port of the same legacy `TreeSupport` path: occupancy inflation, role carve, drop loop, contacts. `SupportAnalysis` adds per-layer contact detection, `needs_support` eligibility, and model occupancy. |
| op count vs per-op cost | **The planner is the one stage where PnP may genuinely out-count canonical.** `inflate_model_occupancy` is one uncached `host::offset_polygons` per call, split between the per-layer emit check (16.40 s / 433 calls) and the per-origin role carve (16.37 s / 431 calls) — **32.7 s, 56.2 % of the planner on base-on** (t113). Canonical's `calculate_collision` / `calculate_avoidance` are cached by `{radius, layer, recursions}`. The clip core is also 31 % (18.0 s) against canonical's whole 8.34 s support increment. **This is the largest supports-on shape, and it is a caching/work-set shape before it is a per-op one.** t125 batched the offsets without reducing their count and the wall gate failed 6/8; a cache is a different candidate. |

### 2.7 Summary row per stage

| stage | canonical ops/layer | PnP ops/layer | verdict |
| --- | --- | --- | --- |
| Slice | 1 mesh walk + 3 parallel passes | 1 mesh walk + 3 classification ops | same work; per-op cost |
| ShellClassification | ≈45–60 (incl. wave expansion) | ≈15–25 | **PnP counts less, costs ≥7x more**; work-set swap, not deletion |
| OverhangAnnotation | 3 ops (lift) + per-vertex distance at emit | 3 offsets + ~9.8 clips | **PnP counts ~3x more**; work-set + locality |
| SeamPlanning | 0 offsets | 1 offset/region | **extra work**; small |
| MeshAnalysis | 0 whole-mesh ops | 2.46 M facets + 326 k union | **extra work** |
| SupportGeometry / Analysis / planner | legacy TreeSupport, cached collision/avoidance | legacy TreeSupport, uncached occupancy at 2 sites | **caching shape**; largest supports-on term |
| (Canonical only) `process_external_surfaces` | 30–40 | **0 (DEV-200)** | PnP omits canonical's largest per-layer consumer |

## 3. The decisive split: op count versus per-op cost

The ticket's decision rule is: if per-op cost is comparable and PnP's op count
is higher, the route is a work-set port; if the op count is comparable and
per-op cost is 5–10x, the route is the geometry kernel; if both are comparable,
no rewrite pays.

The measured shape is a fourth case:

- **Op count is not higher.** In the shell/external-surface cluster canonical
  runs 45–60 ops per layer and PnP runs 15–25, because PnP does not port
  `discover_vertical_shells` or the `process_external_surfaces` wave expansion
  (`DEV-200`). PnP's shell stage is still 7.0x canonical's **whole** prepass.
- **Per-op cost is several times the whole-phase ratio on the same-work
  stages.** Canonical's whole prepass is 2,103 ms (base-off) / 10,445 ms
  (base-on) against PnP's 33,003 / 102,752 ms — **15.7x / 9.8x** at the phase
  level. The per-stage lower bounds are direct and more conservative:
  PnP's `ShellClassification` (14,698 ms) is **7.0x canonical's entire
  prepass** (2,103 ms), and the PnP planner (55,798 ms) is **6.7x canonical's
  entire supports-on increment** (8,344 ms). Those ratios cannot be explained
  by op count, because PnP runs fewer shell ops and the planner ports the same
  legacy `TreeSupport` path. Both engines parallelise the same axes
  (layers/regions), so the direction holds.
- **The kernel is at parity.** t70's Rust/C++ twin measured the Clipper2 engine
  at parity or faster after the `nearbyint` fix (union/offset/intersection
  engine-time ratios 0.559 / 0.587 / 0.650; difference 0.964). The 6.7–15.7x is
  **above** the engine.

What is above the engine is the operand:

- Canonical clips every expensive intersection to a **subject bbox**
  (`clip_clipper_polygons_with_subject_bbox`) in `bridge_over_infill` and in
  `propagate_wave_from_boundary`; the wavefront is bounded to ≤5 steps of 0.1 mm
  and each step is clipped back to the seed's inflated bbox.
- Canonical's collision/avoidance is **cached by radius and layer**
  (`TreeSupportData::calculate_collision` / `calculate_avoidance`), so many
  nodes share one offset.
- Canonical's overhang-speed and seam-quality work is **per-vertex distance**
  against a `LinesDistancer`, not a whole-layer polygon partition.
- PnP's hot ops are whole-layer: `deep_infill_area` offsets, overhang band
  offsets, the 326 k-facet union, the per-region seam inset, and uncached model
  occupancy. The prepass has no `clip_clipper_polygons_with_subject_bbox`
  equivalent at these sites.

The actionable version of the ticket's second branch is therefore not "make
Clipper faster" — it already is at parity — but **"make the operands local."**
That is a different implementation program from a work-set port, and it is the
program the measured gap actually supports.

## 4. Named shapes and ceilings

Ceilings are the measured stage walls (t126/t84/t113), i.e. the upper bound if
the whole shape were free. The replacement cost is not subtracted; each row
carries the caveat. Floor multiples use the cell's 0.80 % floor from
`census.json`. "Share of required cut" is `ceiling / (cut_to_beat × cell)`.

| # | shape | kind | base-off ceiling | ×floor | % of required cut | base-on ceiling | ×floor | % of required cut |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| S1 | overhang quartile bands → per-vertex distance | work set + locality | **9,689 ms** | 28.5x | **34.8 %** | 10,309 ms | 10.2x | 11.2 % |
| S2 | whole-mesh facet union → per-layer overhang polygons | work set | **4,260 ms** | 12.5x | 15.3 % | 4,278 ms | 4.2x | 4.6 % |
| S3 | seam inset offset → canonical loop projection | work set | **1,587 ms** | 4.7x | 5.7 % | 1,507 ms | 1.5x | 1.6 % |
| S4 | Shell Pass 2 (horizontal shadow) → vertical-shell path | work set (swap) | unpriced (non-gate shell work 10,136 ms pre-t123) | — | — | unpriced | — | — |
| S5 | internal-bridge offsets clipped to candidate bbox | locality | unpriced (two offsets 21,423 ms pre-t75) | — | — | unpriced | — | — |
| S6 | occupancy from radius/layer cache | caching | — | — | — | **32,712 ms** | 32.4x | 35.6 % |

Shares of the **prepass** (the ticket's other denominator; order benchy-off /
benchy-on / base-off / base-on): S1 **21.4 / 8.8 / 29.4 / 10.0 %**, S2 **5.6 /
2.6 / 12.9 / 4.2 %**, S3 **5.5 / 2.6 / 4.8 / 1.5 %**; S6 is **31.8 %** of
base-on's prepass. No single shape reaches its cell's required prepass fraction
(101 / 95 / 84 / 90 %), which is why the single-stage gate fires.

The clean, measured work-set tranche (S1 + S2 + S3, with S3 at its measured
92.4 % share) is:

| cell | clean tranche | % of cell | % of prepass | % of required cut |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | 937 ms | 20.2 % | 32.6 % | **32.1 %** |
| benchy classic-on | 952 ms | 10.7 % | 14.0 % | **14.9 %** |
| base classic-off | 15,536 ms | 36.9 % | 47.1 % | **55.8 %** |
| base classic-on | 16,094 ms | 13.6 % | 15.7 % | **17.5 %** |

S4 and S5 are deliberately left unpriced: S4 is a two-way work-set swap
(canonical runs `discover_vertical_shells` and the wave expansion instead), and
S5 is a locality shape whose ceiling is not separable from the t75 parallel
gate without a fresh probe. S6 is the largest supports-on shape and is a cache
candidate, not a deletion: canonical's collision/avoidance cache is the
counterpart, and t125's batch (which did not reduce the call count) is not a
refutation of a cache.

## 5. Route arithmetic: prepass-only versus full geometry parity

The t126 counterfactual used **zero prepass**. Canonical's prepass is not zero:
t110 measured it at **50.5 / 549.8 / 2,103.0 / 10,444.8 ms** (benchy-off /
benchy-on / base-off / base-on). Substituting the canonical prepass, and holding
PnP's other phases at today's cost, is the honest "prepass rewrite lands first"
counterfactual:

| cell | PnP today | prepass at canonical | + per-layer | + postpass | + outside | result | Orca wall (t124) | multiple |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 4,634 | 50.5 | 1,450.4 | 134.4 | 171.5 | 1,806.8 | 1,716.3 | **1.053x** |
| benchy classic-on | 8,887 | 549.8 | 1,644.1 | 373.3 | 80.0 | 2,647.1 | 2,461.8 | **1.075x** |
| base classic-off | 42,150 | 2,103.0 | 8,050.7 | 1,011.6 | 84.3 | 11,249.6 | 14,288.1 | **0.787x** |
| base classic-on | 117,970 | 10,444.8 | 12,504.8 | 2,713.3 | 118.0 | 25,780.9 | 25,479.5 | **1.012x** |

**Prepass-only parity wins base classic-off only.** It ties base classic-on
(1.2 % over), and loses both benchy cells. This is the correction to the t128
route order: the benchy per-layer phase is not a "last mile" after the prepass
— with the prepass at canonical cost it is **1,450 ms of a 1,716 ms Orca wall**
on benchy-off and **1,644 ms of a 2,462 ms wall** on benchy-on. It must be in the
same program.

Full **geometry** parity — PnP's prepass + per-layer phase both at canonical's
whole slicing cost, keeping PnP's postpass and fast import — is the target that
wins every cell:

| cell | canonical slicing | + PnP postpass | + outside | result | Orca wall | multiple |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 637.8 | 134.4 | 171.5 | 943.6 | 1,716.3 | **0.550x** |
| benchy classic-on | 1,280.1 | 373.3 | 80.0 | 1,733.3 | 2,461.8 | **0.704x** |
| base classic-off | 3,684.5 | 1,011.6 | 84.3 | 4,780.4 | 14,288.1 | **0.335x** |
| base classic-on | 14,010.3 | 2,713.3 | 118.0 | 16,841.6 | 25,479.5 | **0.661x** |

The margin comes from PnP's import advantage (up to 10.1 s on base against
Orca's 10.13 s startup+import), not from the slicing engine. That is a real
advantage the destination may keep, and it is why full parity is comfortably
ahead even though the slicing engine is 19.5x behind.

The base classic-on cell is the tightest: full slicing parity gives 0.661x, but
prepass-only parity gives 1.012x. Its planner + analysis + shell + slice =
86.3 s of the 102.8 s prepass; a cache on the 32.7 s occupancy term and a
per-op reduction on the 18.0 s clip core are the carrying shapes, not the
work-set deletions.

## 6. Go/no-go

**GO for the engine rewrite; NO-GO for a work-set-only rewrite.**

- The decomposition does **not** show PnP already computing canonical's work set
  at canonical's per-op cost: PnP runs *fewer* shell ops and *more* overhang/mesh
  ops, and its per-op cost is at least 9.8–15.7x. The ticket's kill condition is
  therefore not met.
- It also does **not** show that the work-set differences are the lever. The
  clean work-set tranche is 55.8 % of base-off's required cut and 17.5 % of
  base-on's; a work-set port alone cannot reach 8/8.
- The measured lever is **operand locality and host geometry cost**: replace the
  whole-layer overhang-band offsets with canonical's per-vertex distance field;
  delete the whole-mesh facet union and seam inset; bbox-clip the internal-bridge
  offsets; serve planner occupancy from a radius/layer collision cache. The
  first three are the measured tranche; the last two are the supports-on route.
- The route order must change: the **benchy per-layer phase joins the first
  program**, not a later "last mile". Prepass-only parity loses both benchy
  cells.

**Recommended next ticket:** a Phase-1 **per-op cost decomposition of the common
prepass stages** — instrument the PnP side at the wrapper boundary (path
conversion, offset sweep, PolyTree rebuild, allocation, parallel scheduling) and
price the operand-locality shapes (S5, S6) with their own probes. It should not
re-cut the work-set census; that is answered here.

## 7. Disclosures

- **Stage walls are instrumented attribution, not wall claims.** They are t126's
  one instrumented pass per cell on the ordinary arm. The ratios of record are
  t124's uninstrumented paired medians; the census does not mix the two without
  saying so.
- **Canonical per-stage walls do not exist on this rig.** t110 measured only the
  phase boundary (prepass 2,103 / 10,445 ms) and showed that the CLI log, ETW and
  `--pipe` channels cannot resolve substages. Every canonical per-stage op count
  in this report is **source-derived**, not measured. The per-op ratio is a
  phase-level bound, not a per-stage measurement.
- **The canonical binary is not byte-proven identical to the source pin.** The
  rig runs the installed 2.5.0 binary (files dated 2026-11-06); the source is the
  `v2.5.0` tag (2026-11-07). The pin is disclosed in `canonical_citations.json`
  and in the map. No op-level comparison is quoted here as a wall claim.
- **The PnP-side per-op numbers are pre-t123 where they come from t75/t84/t76/t113
  probes.** They are used for shares and per-call prices, not as current wall
  claims. The current stage walls are t126's.
- **S4 and S5 are unpriced.** The non-gate shell work and the two whole-layer
  offsets are measured as brackets, but the Pass 2 subset and the bbox-locality
  saving are not separately isolated. They are listed as route shapes with their
  measured upper context, not as ceilings.
- **No production edit, no candidate, no G-code reference move, no guest rebuild.**
  Both freshness gates were untouched by this take (no guest or host source
  changed). The evidence is docs-only.
- **The `verify.py` self-test** re-derives the census arithmetic and rejects
  negative controls (tampered stage sums, tampered shares, tampered route
  multiples). It does not resolve the canonical citations against the external
  checkout; `--orca-check` does that when `D:\slicerProject\Orca(pnp_gui)` is
  present at the pin.
