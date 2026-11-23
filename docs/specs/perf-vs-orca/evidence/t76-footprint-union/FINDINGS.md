# Ticket 76 - MeshAnalysis exact footprint union

Take: 2026-10-19/20, first frontier ticket in order after ticket 75's closeout.
Attribution and a candidate prototype first; the candidate then faces the
standing 16-cell output-identity gate and the paired A/B. Keep/drop is the
human's.

## Question

Can `compute_xy_footprint`'s exact union over the projected facets be made
cheaper while preserving the downstream footprint representation exactly?

The t73 base refresh measured `PrePass::MeshAnalysis` at 19.4-22.2 s on every
base cell, with `host:mesh_analysis`'s only scope (`polygon_ops::clip_polygons`)
at 19.2-20.1 s accumulated and a concurrency of ~1x. The ticket's bbox
replacement is out of scope because `region_needs_support` and the SDK's
`SliceRegionView::derive_needs_support` consume the footprint through exact
intersections.

## Attribution at HEAD

Post-t75 HEAD `0691f050` (slicer sources = the t75 keep `07b97cad`). A
temporary Rust probe (`probe_t76_union_probe_tdd.rs.txt`, preserved as source;
removed before any commit) loaded the user-supplied fixtures through
`slicer_model_io::load_model`, ran `execute_mesh_analysis`, replicated
`compute_xy_footprint`'s projection and union, and measured the input census
and exact-reduction shapes at 12 rayon threads.

| quantity | base | benchy |
| --- | ---: | ---: |
| total facets | 2,461,234 | 225,786 |
| overhang facets (overhang-union input) | 326,546 | 22,137 |
| bottom-surface facets (bottom-union input) | 22,954 | - |
| one-shot `union_ex` (overhang) | 15.0-15.9 s | 0.084 s |
| `execute_mesh_analysis` (whole stage) | 19.5-20.0 s | - |
| bottom-surface footprint | 0.26-0.31 s | - |

The base overhang union is **15.0-15.9 s of a 19.5-20.0 s stage**, and the
replica reproduces the production IR footprint exactly
(`same_region == true`, 1,049 polygons / 26,570 vertices /
180,317,211,903 units²). The bottom-surface union is small (22,954 facets,
0.26-0.31 s) because it is below the partition threshold.

Input census: **326,546 distinct projected triangles, 0 duplicates, 0
zero-area**, all wound the same way (signed area negative). Exact input
reductions were measured and rejected as too small:

| shape | result |
| --- | --- |
| dedupe identical projected triangles | 0 inputs removed, 15.4 s (no-op) |
| drop triangles wholly inside another (convexity; 5.9 % = 19,263 facets) | 13.6-14.2 s, saves only ~1.5 s; geometry equal, serialized footprint still changes |
| reorder the same inputs (reversed / xy-sorted) | same geometry, different serialized footprint |

The reorder control is the important one: the engine's serialized footprint is
already input-order sensitive, so byte-identical IR is not an invariant of the
current path and cannot be the ticket's gate.

## Candidate (uncommitted)

`candidate.patch` (one production file,
`crates/slicer-core/src/algos/mesh_analysis.rs`):

1. `compute_xy_footprint` keeps building the per-facet projected triangles,
   then delegates to `union_footprint`.
2. `union_footprint` runs the unchanged one-shot `union_ex` below
   `PARALLEL_UNION_MIN_FACETS = 100_000` (3D Benchy's 22,137 overhang facets
   stay on the old path).
3. Above the threshold it Morton-orders the projected centroids (offset,
   24-bit interleave, ties broken by facet index), splits the order into
   `PARALLEL_UNION_CHUNKS = 48` contiguous chunks (a fixed count, **not**
   thread-derived, so the partition is machine-independent), unions each chunk
   on the rayon pool, and merges the partial results with one final union.

Set union is associative, so the filled region is the union of the whole input;
the partition changes which worker runs the sweep, not the set being unioned.
The fixed chunk count keeps the result independent of the machine's thread
count.

Measured on the prototype (same probe, 12 threads):

| k (chunks) | chunk unions | final merge | total | output polygons |
| ---: | ---: | ---: | ---: | ---: |
| 12 | 0.71-0.76 s | 0.11-0.14 s | 0.82-0.90 s | 992 |
| 24 | 0.58-0.75 s | 0.13 s | 0.71-0.88 s | 993 |
| 48 | 0.31-0.37 s | 0.15 s | 0.45-0.52 s | 993 |
| 96 | 0.21-0.22 s | 0.18 s | 0.39-0.43 s | 993 |

The production candidate (48 chunks, proper non-negative Morton key) measured
`execute_mesh_analysis` **19.5-20.0 s -> 4.8-5.9 s** and a base footprint of
993 polygons / 26,646 vertices / 180,317,204,333 units².

The frozen snapshots predate one later, test-only edit: `union_footprint` now
delegates to `union_footprint_partitioned(tris, PARALLEL_UNION_MIN_FACETS)`
so the regression test can inject a low threshold. The delegation uses the same
constant and the same body, so the measured binary is behaviourally the current
source; `candidate.patch` is regenerated from the final tree.

## Exactness boundary (stated before qualification)

- **Geometry**: the partitioned region differs from the one-shot region only by
  sub-resolution slivers. Measured delta: -7,570 units² (1 unit = 100 nm, so
  7.6e-5 mm²) on a 1,803 mm² footprint = **4.2e-8 relative**. The one-shot
  result is a superset: the partition drops slivers, it does not add area.
- **Representation**: polygon count/order/vertex set differ. As the reorder
  control shows, this is already true of the unchanged engine under any input
  reordering, so it is not specific to the partition.
- **Consumers**: `region_needs_support` uses bbox tests (fast path) and the
  exact union only on the rare all-disjoint slow path;
  `SliceRegionView::derive_needs_support` does an exact `intersection_ex`;
  `assemble_flat_bridge_areas` intersects the distinct bottom-surface
  footprint; `visual_debug_render` draws the footprint. The 16-cell
  output-identity matrix is the required proof, not the equivalence argument.

Pre-check: ordinary base classic supports-on with the candidate measured
**byte-identical to the preserved t58 reference** `f60e5e9b…` (43,912,792 B) in
one uninstrumented run at 12 threads (314.7 s process wall).

## Qualification

**16/16 PASS.** Every cell's baseline and candidate outputs are byte-identical
to each other **and to the preserved t58 reference** for that cell, every run is
clean (`status=ok`, `degraded=false`, 0 fatal / 0 non-fatal) and non-vacuous.
The matrix covers both modes x both fixtures x both generators x both support
states. See `qualification.json`.

Snapshot provenance (gitignored tree): ordinary candidate `F427003E…`,
ordinary baseline `7BAB13CF…`, accelerated candidate `75581F37…`, accelerated
baseline `F0BED7D3…`. Within each mode the module set is byte-identical between
the baseline and candidate snapshots (the change is inside `host-algos`, which
no guest compiles); the ordinary and accelerated module sets differ from each
other by construction.

Recovery disclosure: the first proof campaign completed 15 cells, then its
candidate run for `accelerated-base-arachne-on` stopped in `postpass` without
writing output (machine stall overnight). The partial cell directory was
deleted and the cell was re-run whole; the interrupted run is not part of the
evidence. No output-bearing cell was retried.

## Standing paired gate

6 alternating pairs per arm, 2 warmups, uninstrumented `GetProcessTimes`,
12 rayon threads. Paired-median deltas (candidate vs baseline, per-pair ratio
of medians over 6 retained pairs; no pair excluded):

| cell | role | baseline wall median | candidate wall median | wall delta | CPU delta |
| --- | --- | ---: | ---: | ---: | ---: |
| `ordinary-base-classic-on` | gate | 307.37 s (305.72-310.23) | 293.40 s (292.09-296.25) | **-4.46 %** | -0.14 % |
| `accelerated-base-classic-on` | gate | 271.26 s (270.86-272.31) | 257.53 s (256.01-260.84) | **-5.21 %** | -0.93 % |
| `ordinary-benchy-classic-on` | control | 21.09 s (20.45-21.49) | 21.25 s (20.47-22.24) | +2.11 % | -0.26 % |
| `accelerated-benchy-classic-on` | control | 16.39 s (16.31-16.53) | 16.53 s (16.31-16.69) | +0.71 % | -0.26 % |

Both pre-declared gate clauses pass in both gate cells: wall falls and paired
process CPU stays under the +1.0 % ceiling (it falls slightly, consistent with
the serial sweep being split rather than duplicated). Every pair's G-code SHA
is identical. The benchy controls are inert by construction (below the
100,000-facet threshold) and their wall deltas (+2.1 %, +0.7 %) sit inside the
overlapping min-max ranges, i.e. paired noise; CPU is flat.

Recovery disclosure: the first timing campaign completed the ordinary-Benchy
control cell whole and 4 measured pairs of `ordinary-base-classic-on`, then its
`baseline-m4` run was killed ~2 s in by a host stall. The 11-row partial cell is
preserved as
`measure-interrupted-20261020/ordinary-base-classic-on` and is **not** part of
the result. A replacement campaign was stopped for a user-requested pause after
its two `ordinary-base-classic-on` warmups (zero measured rows); that tree is
preserved as `measure-interrupted-20261021/ordinary-base-classic-on`. A third
attempt reached 12 of 14 rows and lost its `candidate-m6` run 2 s in; that tree
is preserved as `measure-interrupted-20261021b/ordinary-base-classic-on`. The
completed measurements come from a fourth campaign, launched detached (Windows
scheduled task) so harness restarts could not kill it, which re-ran the cell
whole. No output-bearing cell was retried and no gate-failing cell was re-run.

## Evidence and verification

- `probe_t76_union_probe_tdd.rs.txt` - temporary attribution/census probe
  (removed from the tree before any commit).
- `candidate.patch` - the production candidate.
- `qualification.json` - 16-cell output-identity gate.
- `ab-reduction.json` - paired ordinary + accelerated reduction.
- `verify.py` - checkout-only verifier (exit 0 against this directory alone,
  negative controls rejected).
- `run-cell.ps1`, `run-all-proof.ps1`, `run-all-measure.ps1`,
  `build-qualification.py`, `reduce_ab.py` - the measurement harness, adapted
  from the t75 protocol only in root paths.

Raw captures and frozen snapshots live in the gitignored
`.local-artifacts/perimeter-reference-preparation/t76-meshanalysis-union/`
tree; the in-repo reduced set is what `verify.py` reads.

## Caveats

- Instrumented probe values are attribution only; acceptance numbers are the
  uninstrumented `GetProcessTimes` measurements in `ab-reduction.json`.
- The base footprint sliver delta is disclosed above; the gate is byte-identical
  G-code.
- The timing cells are the two base classic supports-on cells; the benchy
  classic supports-on cells are measured controls because the candidate is
  inert below the 100,000-facet threshold.

## Disposition

**KEEP (2026-10-21, human-approved).** Both pre-declared gate clauses pass in
both base cells (wall -4.46 % / -5.21 %, CPU -0.14 % / -0.93 %), the 16-cell
proof is 16/16 with every output byte-identical to the preserved reference, and
the below-threshold controls are inert. Committed as `465f2fd3`; the ticket and
the map carry the resolution.
