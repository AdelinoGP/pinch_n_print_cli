# MeshAnalysis exact footprint union reduction

Type: task
Label: wayfinder:task
Status: resolved (2026-10-21; human-approved KEEP — candidate committed `465f2fd3`; proof 16/16; wall -4.46 % ordinary base / -5.21 % accelerated base; CPU -0.14 % / -0.93 %)
Assignee: 2026-10-19 OpenCode wayfinder session — resumed on the human's
"continue"; first frontier ticket in order
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: 73

## Question

Can `compute_xy_footprint`'s exact union over the projected facets be made
cheaper while preserving the downstream footprint representation exactly?

The fresh base captures measure `PrePass::MeshAnalysis` at **19.4–22.2 s** on
every base cell, and `host:mesh_analysis`'s only scope is
`polygon_ops::clip_polygons` at **19.2–20.1 s accumulated** — a serial union
(concurrency ≈ 1x) ([findings](../evidence/perf-refresh-base-2026-10-18/FINDINGS.md)).
Ticket 27 localized it to `compute_xy_footprint`
(`crates/slicer-core/src/algos/mesh_analysis.rs`): the exact Clipper union over
326,546 projected facets, ~75 % of the stage. Its proposed bbox/coverage
replacement is **unsafe as stated** — `region_needs_support`
(`crates/slicer-core/src/algos/mesh_analysis.rs`) uses the footprint through an
exact intersection, and the premise audit verified that consumer. Bbox-only
substitution is out of scope for this ticket.

Deliverables:

1. Re-derive the union's current cost and input shape at HEAD (the t27 probe
   predates the t70 clipper keep; `clipper2-rust` is now vendored in-tree).
2. Investigate only exact, representation-preserving reductions: a spatial
   partition with a deterministic merge order, parallel union of disjoint
   tiles, or an exact input reduction that provably preserves the downstream
   footprint. Any change must survive the 16-cell output-identity matrix against
   the preserved references; a representation-equivalence argument alone is not
   sufficient.
3. If a candidate survives, run the standing paired ordinary + accelerated A/B
   on the pre-declared cells. Keep/drop is the human's.

Constraints: exactness first; no production edit before a measured owner; no
bbox substitution, no approximation; no default switch or auto-commit.
Profile/worker elapsed is not process CPU or uninstrumented wall.

## Pre-declaration (2026-10-20, before the timing campaign)

Attribution at HEAD (post-t75 `0691f050`, slicer sources = t75 keep `07b97cad`)
with a temporary Rust probe on `tmp/base.stl` and `tmp/3dbenchy.stl` (loaded
through `slicer_model_io::load_model`, `execute_mesh_analysis` at 12 rayon
threads):

| quantity | base | benchy |
| --- | ---: | ---: |
| total facets | 2,461,234 | 225,786 |
| overhang facets (union input) | 326,546 | 22,137 |
| one-shot `union_ex` | 15.0-15.9 s | 0.084 s |
| `execute_mesh_analysis` (whole stage) | 19.5-20.0 s | - |
| bottom-surface footprint union (22,954 facets) | 0.26-0.31 s | - |

Census of the base union input: 326,546 distinct projected triangles, **0
duplicate**, 0 zero-area, all wound the same way. Exact input-reduction
shapes were measured and refuted as insufficient: dropping the 19,263 triangles
(5.9 %) wholly contained in another triangle saves only ~1.5 s (15.3 → 13.7 s)
and still changes the serialized footprint (input reordering alone does, with
identical geometry).

The tested candidate is a **work-preserving spatial partition**: above
100,000 facets, projected centroids are Morton-ordered, split into 48
contiguous chunks (fixed count, not thread-derived), each chunk is unioned on
the rayon pool, and the partial results are merged by one final union. Below
the threshold the one-shot sweep is unchanged (benchy stays on the old path).
Prototype: **15.0-15.9 s → 0.39-0.52 s** for the base union; the production
`execute_mesh_analysis` measured **19.5-20.0 s → 4.8-5.9 s**.

Exactness boundary, stated before measuring: set union is associative, so the
filled region is the union of the whole input; the partition changes which
worker runs the sweep. It is **not bit-identical at the IR level**: the
partitioned result measured 993 vs 1,049 output polygons, 26,646 vs 26,570
vertices, and an area delta of -7,570 units² (4.2e-8 relative; 7.6e-5 mm² on a
1,803 mm² footprint) in sub-resolution slivers. Input reordering or dropping a
provably contained triangle also changes the serialized representation, so
byte-identical IR is not an invariant of the current engine. The gate is
therefore the byte-identical G-code matrix plus the paired A/B, as the ticket
requires.

Pre-check: ordinary base classic supports-on with the candidate measured
**byte-identical to the preserved t58 reference** `f60e5e9b…` (43,912,792 B),
one uninstrumented run at 12 threads (314.7 s process wall).

Pre-declared gate:

- **Exactness**: the full 16-cell matrix (both modes x both fixtures x both
  generators x both support states), baseline and candidate each byte-identical
  to each other and to the preserved t58 reference, clean and non-vacuous.
- **Timing cells (gate)**: `ordinary-base-classic-on` and
  `accelerated-base-classic-on`, 6 alternating pairs each; paired-median wall
  must fall (`< 0 %`) and paired-median process CPU must not exceed **+1.0 %**
  (the t75/t27 parallelization ceiling).
- **Control cells (measured, not gate)**: the two benchy classic-on cells, where
  the input is below the threshold and the stage is unchanged; expected
  byte-identical output and wall/CPU inside paired noise.
- Expected magnitude: ~15 s of serial sweep removed, i.e. **-3 % to -5 %** of
  base classic-on wall.

## Acceptance

- A current-head union cost and input census, with the parallel/reduction shape
  stated precisely enough to implement.
- Either a candidate passing the standing gate, or a measured no-safe-reduction
  verdict with the refuted shapes recorded.
- Durable reduced evidence and a checkout-only verifier, per the map's
  evidence policy.

## Resolution

**KEEP (2026-10-21).** The one-shot union over the base mesh's 326,546 overhang
facets is 15.0-15.9 s of the 19.5-20.0 s `PrePass::MeshAnalysis` stage. Exact
input reductions were refuted: 0 of 326,546 projected triangles are duplicates
or zero-area; dropping the 19,263 triangles (5.9 %) contained in another
triangle saves only ~1.5 s and still changes the serialized footprint; reversing
or sorting the input keeps the geometry but changes the serialized footprint.
The serialized footprint is therefore already input-order sensitive, so
byte-identical IR is not an invariant of the unchanged engine.

The kept candidate partitions the sweep above `PARALLEL_UNION_MIN_FACETS =
100_000`: projected centroids are Morton-ordered, split into a fixed 48
contiguous chunks (not thread-derived, so machine-independent), unioned on the
rayon pool, and the partials merged with one final union. Set union is
associative, so the filled region is the union of the whole input. Measured
production stage: **19.5-20.0 s -> 4.8-5.9 s**. The partition changes the
representation only by sub-resolution slivers (-7,570 units², 4.2e-8 relative
area) and polygon ordering; the gate is byte-identical G-code, not IR.

Proof: **16/16 PASS** - both modes x both fixtures x both generators x both
support states, baseline and candidate each byte-identical to the preserved t58
reference, clean and non-vacuous.

Standing paired gate: **6/6 retained pairs in every cell, every pair
byte-identical**. Ordinary base classic-on **-4.46 %** wall (307.37 -> 293.40 s,
paired median) with CPU **-0.14 %**; accelerated base classic-on **-5.21 %**
(271.26 -> 257.53 s) with CPU **-0.93 %**. Both pre-declared clauses pass with
CPU under the +1.0 % work-preserving ceiling. The below-threshold Benchy
controls are inert: +2.11 % / +0.71 % wall with overlapping min-max ranges and
CPU -0.26 % / -0.26 %.

Recovery: three timing campaigns were lost to host stalls / a requested pause
(4 measured pairs, 0 measured rows, 12 of 14 rows respectively); the partial
trees are preserved under `measure-interrupted-20261020`,
`measure-interrupted-20261021` and `measure-interrupted-20261021b`, none are
used, and a fourth detached campaign re-ran the cell whole and produced the
completed measurements. No output-bearing cell was retried and no gate-failing
cell was re-run.

Production commit: `465f2fd3`. The frozen candidate snapshots predate one
test-only edit (`union_footprint` delegating to the threshold-injectable
helper); the measured binary is behaviourally identical. Pre-commit gates:
workspace clippy `-D warnings` clean, `check-literals` 0 violations,
`check-test-quality --report` 0 findings, the new regression test passes,
`verify.py` exits 0 checkout-only with four negative controls rejected.

Evidence: [`evidence/t76-footprint-union/`](../evidence/t76-footprint-union/FINDINGS.md)
(`qualification.json`, `ab-reduction.json`, `candidate.patch`,
`probe_t76_union_probe_tdd.rs.txt`, `reduce_ab.py`, `verify.py`).

Closeout: the developer distribution was rebuilt at the kept HEAD
(`target/dist/developer`); its 48 module files are byte-identical to the
campaign's ordinary-candidate snapshot, both guest freshness gates exit 0
(ordinary and accelerated), and a fresh run of the staged binary on ordinary
base classic-on reproduces `f60e5e9b...` (43,912,792 B) at 322.9 s process wall.
