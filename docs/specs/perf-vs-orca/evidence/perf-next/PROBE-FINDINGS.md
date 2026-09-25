# Probe capture findings — attribution only

Captures: `tmp/perf-next/probe/arachne-stage-probe.jsonl` (Arachne host wall
brackets, `PNP_STAGE_PROBE=1`, two validated runs in `probe-runs.csv`) and
`tmp/perf-next/probe/classic-scope-probe.jsonl` (Classic guest fuel scopes,
`--profile`, artifacts present, no CSV row by design). Provenance in
`tmp/perf-next/probe/provenance.json`; probe diffs preserved as
`classic-probe-diff.patch` / `arachne-probe-diff.patch`. Probe-run wall/CPU are
attribution-only and must not be compared with baseline rows.

## Reconciliation trap established first

The Arachne pipeline runs **concurrently on 12 rayon workers** (240
`pipeline_preprocess` calls across 240 layers, sometimes several per layer for
multi-region layers). Bracket sums therefore **exceed** the enclosing per_layer
phase wall (17.608 s of `pipeline_preprocess` sums vs an 11.023 s per_layer
phase wall): the sums double-count concurrent wall. Per-call values are
contended service time, not exclusive service time. Child-vs-parent ratios
**inside one bracket tree** are the meaningful measure; absolute
"unattributed parent time" figures are **not** wall-truth.

## Arachne: parents vs children (inside one call tree, per-call)

| parent | per-call p50 | children sum share | leading child |
| --- | ---: | ---: | --- |
| `pipeline_preprocess` | 70.8 ms | **5.9%** | `stage1_triple_offset` (94.6% of children) |
| `pipeline_graph_construction` | 53.2 ms | **6.9%** | `graph_voronoi` (87.2% of children) |

Both named parents are dominated by time **outside** their instrumented
children. Inside preprocessing, `stage1_triple_offset` — the net-zero
shrink/grow/shrink Clipper offsets in `triple_offset`
(`crates/slicer-core/src/arachne/preprocess.rs`) — is 94.6% of the instrumented
children (p50 3.0 ms/call); stages 2–9 together are noise (p50 ≤ 0.05 ms each).
Inside graph construction, `graph_voronoi` (`voronoi::voronoi_from_segments`,
`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`) is 87.2% of children
(p50 2.2 ms/call); `collapse_small_edges`, `builder_build`, and the rest are
small.

**What the residual means:** the ≥93% of each parent call that is not in any
child bracket is either (a) Clipper2/boostvoronoi work that happens inside
library internals those children call (most likely for `stage1_triple_offset`'s
three offset passes, which are themselves inside the child — note the child
*is* the offset call, so its 3.0 ms p50 is real and the parent residual is
something else), or (b) scheduling/descheduling wall inside the parent brackets
under 12-worker contention. The parent p50 (70.8 ms) vs child p50 (3.0 ms) gap
on the *same* 240 calls cannot be explained by the stage-9 union or warning; it
predominantly reflects concurrent-wait wall captured by `Instant::now()` around
calls that contend. The honest statement: **preprocessing and graph
construction dominate Arachne host wall (52.8% + 39.6% of bracketed total,
consistent with the old 75.3% finding), their internal Clipper/Voronoi
children are secondary, and the remaining parent time is contention-dominated
wall that fuel/worker-elapsed units do not measure.**

Per-call medians (contended wall, attribution only): preprocess 70.8 ms,
graph construction 53.2 ms, `stage1_triple_offset` 3.0 ms, `graph_voronoi`
2.2 ms, `generate_toolpaths` 0.5 ms, beading propagation 0.4 ms,
`reorder_by_region_order` 0.3 ms. The "unattributed 364.932 ms remainder" from
the old study is now bracketed: per-layer 240 calls show the setup/post stages
(`params_setup` 0.0 ms, `reorder` 0.3 ms p50) are individually tiny; the old
remainder is consistent with this capture's un-bracketed parent residual.

## Arachne candidate implications (to discuss, not yet chosen)

1. **`stage1_triple_offset` is the only concrete preprocessing target**: it is
   a two-call Clipper sequence (`offset2_ex(-eps,+2eps)` then `offset(-eps)`)
   on every region of every layer. Any candidate here (e.g. merging passes or
   reducing redundant offsets) touches canonical sub-epsilon feature handling —
   high semantic risk, must be parity-tested against
   `crates/slicer-core/tests/preprocess_golden.rs`.
2. **`graph_voronoi` is the only concrete graph target**: the boostvoronoi
   construction itself. Optimizing it means changing the Voronoi computation
   (e.g. algorithmic or batching), not reordering — topology/ties are
   canonical.
3. The **retention lead** (`run_nine_stage_pipeline` holding all nine stage
   Vecs) is real but its fuel/retention cost is small: stage outputs 2–9 are
   microseconds each; only `stage1_triple_offset`'s output is substantial.
   Dropping intermediates earlier would cut peak memory, not measured CPU/wall.
4. The dominant remaining Arachne time is inside the two parent brackets but
   outside their children — likely contention-visible wall, not attributable to
   an optimization candidate without a different measurement method (e.g.
   single-worker runs, or perf/Sampler-based profiling). **No Arachne
   optimization candidate is confirmed by this capture alone.**

## Classic: guest fuel scopes (deterministic fuel, exact)

Whole-slice guest fuel 1,134,567,142,838 (matches the no-probe capture within
0.0001% — probes burn no fuel). `com.core.classic-perimeters` total
1,012,845,348,475, self 500,004,913.

| scope | calls | self_fuel | % of module total |
| --- | ---: | ---: | ---: |
| `build_ring_wall` | 2107 | 791,683,438,203 | **78.16%** |
| `polygon_ops::offset2_ex` | 960 | 172,131,239,137 | 17.00% |
| `bridge_flag_loop` | 2107 | 47,843,474,807 | **4.72%** |
| `emit_walls` | 240 | 452,199,658 | 0.04% |
| `insets` | 240 | 200,433,328 | 0.02% |
| `inset_push` | 694 | 14,367,049 | 0.001% |
| `gap_fill` | 240 | 11,980,474 | 0.001% |
| `run_perimeters` | 240 | 5,989,871 | 0.001% |
| `push_wall_loop` | 2107 | 1,574,204 | 0.000% |
| `wall_assembly` | 240 | 646,831 | 0.000% |

**This overturns the retention hypothesis.** The handoff's lead 1 ("retain
only the first inset… rather than cloning/storing every inset") targeted
`all_wall_polygons` retention, but `inset_push` (the clone) measures
**14,367,049 fuel = 0.001%** of the module. The `insets` loop including all
cloning measures 0.02%. Clone cost is noise; retention restructuring cannot
produce a measurable CPU or wall win.

**The actual dominant cost is `build_ring_wall` (78.16% of classic-module
fuel, 2107 calls).** Its internals: `bridge_flag_loop` is only 4.72%; the rest
is `build_ring_wall` self — chiefly `expolygon_to_path3d`
(`crates/slicer-core/src/perimeter_utils.rs`), which runs for every vertex of
every ring and inside it, per vertex:
1. `overhang_bands` winding-number scan (band polygons per vertex),
2. `signed_distance_to_boundary` — a full **O(boundary edges)** nearest-edge
   scan over `prev_layer_boundary` for every vertex (line 469, non-empty
   boundary path),
3. then `build_wall_flags` (fast path committed), and
4. the per-vertex `point_in_any_polygon(bridge_areas)` scan in
   `bridge_flag_loop` (4.72%).

`offset2_ex` (inset generation) remains the second measured block at 17.0%,
consistent across both captures.

## Classic candidate implications

1. **`expolygon_to_path3d` vertex loop is the measured dominant cost** —
   specifically `signed_distance_to_boundary`'s brute-force nearest-edge scan
   per vertex plus the overhang-band winding scan. Both are per-vertex ×
   per-band-polygon / per-boundary-edge, with no spatial index. A candidate
   would add a spatial acceleration structure (grid/R-tree over boundary edges
   and/or band polygons) while preserving exact results (same signed distance,
   same quartile tie behavior). This is now the highest-confidence Classic
   candidate — but it is a geometry-data-structure change, needing exact-output
   tests before/after.
2. **`offset2_ex` host migration** stays the fuel-lead alternative (17.0%),
   with the same contract caveat as before (no approved WIT contract; guest
   fuel would move to host wall, not disappear).
3. **Inset retention/cloning: drop as a candidate** — measured at 0.001–0.02%
   of module fuel.

## Validation status

Gates at capture time: `cargo check --workspace --all-targets` pass, clippy
`-D warnings` pass, `check-literals` 0 violations, guests rebuilt, freshness
exit 0, release binary rebuilt. Arachne bracket run validated
(expected/validated arachne, ok, 0 fatal/non-fatal); Classic profile capture
produced a valid G-code + JSONL (profile mode writes no CSV row by design).
No production behavior changed; all probe code is `[PERF-PROBE]`-tagged and
uncommitted. No KEEP/DROP decision is made by this document.