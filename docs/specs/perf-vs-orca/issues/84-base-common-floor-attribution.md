# Base common-floor attribution: SeamPlanning and OverhangAnnotation

Type: task
Label: wayfinder:task
Status: resolved (2026-10-24; attribution take — the seam inset offsets own
92.4/93.2 % of `PrePass::SeamPlanning` and graduate as
[ticket 85](85-seam-inset-offset-batching.md); the overhang stage is engine-bound
and its only operand-side shape measures below the floor; no production edit,
probe reverted — see ## Answer)
Assignee: 2026-10-24 OpenCode wayfinder session — human chose the "Base
common-floor attribution" route from the map's route options (no ticket named).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the map's fog — the two base common-floor stages t73 measured
without a reduction (map's "Still fog" note under the base refresh)

## Question

Where do the two un-attributed base common-floor stages spend their wall —
`PrePass::SeamPlanning` and `PrePass::OverhangAnnotation` — and which named
reduction shapes survive a measured ceiling check against the map's floor?

## Measured basis

t73's base refresh
([`perf-refresh-base-2026-10-18`](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md))
measured the common prepass floor at 104–115 s on every base cell. Two of its
five stages have never been decomposed:

| stage | classic-off | classic-on | arachne-off | arachne-on |
| --- | ---: | ---: | ---: | ---: |
| `PrePass::OverhangAnnotation` | 10.59 s | 9.48 s | 9.84 s | 9.27 s |
| `PrePass::SeamPlanning` | 8.62 s | 8.88 s | 8.68 s | 10.85 s |

Neither stage is a guest-only cost:

- `PrePass::OverhangAnnotation` is a host built-in —
  `commit_overhang_annotation_builtin`
  (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`). It
  gathers every object's per-layer `SliceIR` polygons into owned
  `layer_footprints`, calls the parallel `annotate_overhangs`
  (`crates/slicer-core/src/algos/overhang_annotation.rs`, whose
  `partition_into_bands` runs three round offsets + three intersections + four
  differences per overhang layer), merges the kernel maps, then deep-clones
  `SurfaceClassificationIR` and replaces it.
- `PrePass::SeamPlanning` is a guest module stage. The host projects
  `SliceIR` regions into a WIT view (`project_seam_planning_view`,
  `crates/slicer-wasm-host/src/marshal/in_.rs`), `seam-planner-default`'s
  `run_region_planning_entries` (`modules/core-modules/seam-planner-default/src/lib.rs`)
  allocates one `ScoredSeamCandidate` per boundary vertex, runs one
  `host::offset_polygons` + `project_point_onto_inset_boundary` per region, and
  pushes every candidate back through `push_seam_plan`; the host then harvests
  the entries into `SeamPlanIR` (`harvest_seam_plan_ir_from`). The candidate
  list is carried in the IR but no production consumer reads
  `SeamPlanEntry.scored_candidates` outside the marshal boundary (the dispatch
  path uses `resolved_seam` / `chosen_position`), so its share is a prime
  measurement target rather than an assumed dead cost.

## Pre-declared gate

This is an **attribution take**: no candidate is implemented, no default or
production source changes, and the probe is env-gated and reverted before the
take closes.

A reduction shape graduates to a candidate ticket only if its measured
occupancy-normalized ceiling is **≥ 0.80 % of cell wall** on at least one base
cell — the same floor t83 used. The ceiling is computed from the measured term
share of the stage × the measured stage wall ÷ the cell wall, and the shape
must be representable without changing committed IR semantics (a shape that
changes `SeamPlanIR` contents or support/overhang geometry is parity-scoped and
goes to the human, not into a candidate ticket).

## Answer

**Resolved 2026-10-24 as an attribution take — one shape graduates, two are
measured dead. No candidate was implemented, no production source changed, and
the probe was reverted before the take closed.**

Measured at HEAD `ce8f3992` on the two base classic cells (ordinary, 12 rayon
threads, 24/24 external modules, guest freshness exit 0). Both arms produce
byte-identical output equal to the t81 references: off `292df501…` /
21,023,739 B, on `22200671…` / 42,507,912 B.

| stage | stage wall (off / on) | share of cell wall | owner |
| --- | --- | --- | --- |
| `PrePass::OverhangAnnotation` | 10.01 s / 10.30 s | 10.78 % / 4.94 % | **99.7 %** parallel kernel `annotate_overhangs`; **85.4 %** of its work is the canonical three round offsets per band layer (1,455 calls), at **11.86x** measured parallelism |
| `PrePass::SeamPlanning` | 9.14 s / 8.97 s | 9.84 % / 4.30 % | **92.4 / 93.2 %** one serial native `offset_polygons` per region (495 calls, 8,195 / 8,177 ms) |

The seam stage's candidate list — the obvious suspect — is **not** the wall
problem: 2,553,550 candidates cost 94.6 ms generation + 16.6 ms scoring +
58.1 ms harvest (0.29 % / 0.13 % of cell wall). The overhang stage's host-side
copy churn (the t74/t58 pattern) does **not** repeat: gather + merge + clone +
commit is 0.18 % of the stage.

**Graduated: [85](85-seam-inset-offset-batching.md)** — batch the 495 per-region
inset offsets through the existing `offset_polygons_batch`. Ceiling **8.83 %**
(off) / **3.92 %** (on) of cell wall if every offset vanished; **5.43 % /
2.41 %** at t78's measured 2.6x effective speedup for the same `map_batch`
mechanism. Both clear the pre-declared 0.80 % floor.

**Dropped, with measured ceilings:** the overhang bbox-window operand retains
**97.84 %** of the previous layer's points (2,540,125 counted, 2,485,164 inside
`current` bbox + 0.7 mm), a **0.189 % / 0.080 %** ceiling; the seam candidate
list is **0.288 % / 0.127 %**. The overhang stage is engine-bound: no
representation-safe shape exists, so making its offsets cheaper is engine work
(t70 territory), not a shape ticket.

Full method, tables, disclosures (probe3 load drift; profile wall inflation) and
the reproduce recipe: [`evidence/t84-common-floor/`](../evidence/t84-common-floor/FINDINGS.md).
`verify.py --self-test` exits 0 checkout-only and rejects 8/8 negative controls.

Frontier now: [85](85-seam-inset-offset-batching.md) (open, unclaimed); the
`tree_support_branch_distance` observation from t83 remains un-graduated in the
fog. The take order is the human's. Standing paired acceptance and KEEP/DROP
rules unchanged.
