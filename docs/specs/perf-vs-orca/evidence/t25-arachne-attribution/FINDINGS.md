# Ticket 25 — Arachne graph-construction re-attribution at HEAD

**The measured answer: at HEAD the Arachne module's cost is NOT the host
pipeline the 2026-09-07 study measured as 75.3%.** The module's own fuel is
**86.4% two guest-side per-vertex spatial queries** — `signed_distance_to_boundary`
(48.3%) and `overhang_quartile` (38.1%) — while the whole host
`generate-arachne-walls` service (all 9 preprocess stages, graph construction,
beading, toolpaths, post-process) is only **~20% of module elapsed**; it burns
**no guest fuel at all** (host calls are unmetered, ADR-0055) and its guest-side
marshalling is 0.04% of module fuel. Inside the host
service, stage 1's triple offset is 41.6% and the boostvoronoi sweep 33.5% of
the service wall.

Attribution only: no fix, no commit, no acceptance retry. Fuel shares are
deterministic (ADR-0055) and exact; wall figures are instrumented and
indicative only (map §3.2).

## The question this answers

Ticket 25 asked for a re-attribution of Arachne's hot path at HEAD: the
2026-09-07 [Perimeter attribution study](../../issues/06-perimeter-attribution-study.md)
put 75.3% of the "enclosing pipeline interval" in preprocess + graph
construction and left a 364.932 ms remainder unexplained — but those shares
predate the committed
[Wall-flags annotation-free fast path](../../issues/07-wall-flags-annotation-fast-path.md).
The ticket names `preprocess_input_outline` / `run_nine_stage_pipeline`
(`crates/slicer-core/src/arachne/preprocess.rs`) and
`SkeletalTrapezoidationGraph::from_polygons`
(`crates/slicer-core/src/skeletal_trapezoidation/graph.rs`) as the targets.

## Method

- Frozen inputs: the supports-off Benchy Arachne job every recent take uses —
  corpus `.local-artifacts/perimeter-reference-preparation/t41-20260930T232255Z/corpus/supports-off-benchy/`
  and ordinary snapshot `t38-campaign-20261001T024827Z/ordinary/`.
- Host side: a temporary host-only probe (`T25-PROBE`) brackets the
  `generate_arachne_walls` service body, every `run_arachne_pipeline` stage and
  sub-stage, and the two clipper halves of stage 1. State is thread-local and
  armed only for one service invocation, so host geometry calls made by other
  modules on the same worker cannot leak into a flush line.
- Guest side: temporary `t25::*` user scopes in `arachne-perimeters` split the
  module's own fuel (guest fuel is the deterministic unit; `--profile`).
- Mode comparison: the same guest scopes built under
  `cargo xtask build-guests --accelerated`, run with the accelerated
  `target/dist-accelerated/developer` snapshot.
- Every capture's output was verified **byte-identical** to the frozen
  reference `7049a06d…` before its numbers were read — including both probe
  builds, both modes, and the `--profile-verbose` capture. The probe sources are
  preserved in `probe.patch` + the `*.rs.txt` copies and were built out of the
  working tree before any commit.

## Finding 1 — the boundary the old study could not see

Ticket 06 measured the **host pipeline interval** (4,739.9 ms accumulated over
240 calls) and could not see the guest's own work outside it. At HEAD, same-run
`module_complete` vs the probe's service bracket gives the split directly:

| Capture | module elapsed (sum) | host service (sum) | host share |
| --- | ---: | ---: | ---: |
| ordinary, `attrib4-inst` | 14,656 ms | 2,648.5 ms | 18.1% |
| ordinary, `attrib10-verify` | 15,033 ms | 2,645.2 ms | 17.6% |
| ordinary, guest-scope capture (`attrib8-final`) | 18,130 ms (wall) | 3,732.6 ms (wall) | 20.6% |
| accelerated (`attrib`) | 11,171 ms (wall) | 3,312.1 ms (wall) | 29.6% |

**The host pipeline is a fifth of the module, not three quarters.** The old
study's "75.3% of the enclosing interval" was true of that interval, and the
interval is not the module.

## Finding 2 — the module's cost is two guest per-vertex queries

Guest fuel split (ordinary, `attrib7-guest2` / `attrib8-final`; shares
reproduce to three significant figures across captures):

| scope | calls | fuel share | wall share (instrumented) |
| --- | ---: | ---: | ---: |
| `t25::signed_distance` (`PerimeterSpatialContext::signed_distance_to_boundary`) | 112,606 | **48.3%** | 35.6% |
| `t25::overhang_quartile` (`PerimeterSpatialContext::overhang_quartile`) | 3,289 | **38.1%** | 29.7% |
| `t25::build_walls` self (path/flags plumbing) | 240 | 5.3% | 4.5% |
| `polygon_ops::offset` (precise-outer-wall shrink) | 240 | 4.1% | 5.1% |
| `t25::region_loop` self | 240 | 3.8% | 2.9% |
| `t25::host_arachne_svc` (the whole host pipeline) | 240 | 0.04% | 20.6% |
| everything else (`seam`, `wall_flags`, `publish`, `setup`, `push`) | — | 0.2% | 0.3% |

**The two queries are 86.4% of the module's fuel.** They are not the
`build_wall_flags` reprojection (0.02%) and not the seam candidates (0.2%).

The tail is not concentrated: the top 12 of 239 layers carry 17.3% of guest
fuel, and 239 of 239 layers call both queries. This is a per-vertex,
every-layer cost, unlike the linker's tail.

Per-call shape from the same capture:

- `signed_distance`: ~547k fuel/call ordinary, 57 µs/call wall.
- `overhang_quartile`: ~14.8M fuel/call ordinary, 1.64 ms/call wall — **27x**
  the distance query per call, on 3,289 calls vs 112,606.

## Finding 3 — acceleration is necessary but does not close the query cost

Same guest scopes, accelerated build + snapshot (output byte-identical):

| term | ordinary | accelerated | ratio |
| --- | ---: | ---: | ---: |
| module total fuel | 127,640,422,555 | 73,652,660,684 | **1.73x** |
| `signed_distance` | 61,605,327,383 | 31,964,757,468 | 1.93x |
| `overhang_quartile` | 48,659,969,659 | 28,441,581,669 | 1.71x |
| per-call, signed_distance | 547,087 | 283,864 | 1.93x |
| per-call, overhang_quartile | 14,794,761 | 8,647,486 | 1.71x |
| two queries as module share | 86.4% | 82.0% | — |

This is exactly the map's ticket-18 shape re-measured on Arachne: the
accelerated indexed path cuts the same query work ~1.9x, and the queries still
dominate the module afterwards. Mode invariance holds on the host side
(service wall ratio 1.13 between modes; `clip`-class work untouched), matching
[infill-linker attribution](../../issues/21-infill-linker-attribution.md).

## Finding 4 — inside the host service (the ticket's named targets, re-measured)

Ordinary host service wall, 240 invocations, `attrib8-final`:

| stage | wall ms | share |
| --- | ---: | ---: |
| `preprocess_input_outline` total | 1,597.9 | **43.3%** |
| — stage 1 triple offset | 1,536.3 | 41.6% |
| — — `offset2_ex` (erode+dilate) | 1,108.2 | 30.0% |
| — — — pass 1 `inflate_paths_64` | 530.1 | 14.4% |
| — — — pass 2 `execute_tree` | 571.3 | 15.5% |
| — — follow-up `offset` | 427.7 | 11.6% |
| — stages 2-9 combined | 61.6 | 1.7% |
| `from_polygons` total | 1,400.9 | **38.0%** |
| — `voronoi_from_segments` | 1,239.0 | 33.6% |
| — — boostvoronoi `Builder::build` | 1,235.7 | 33.5% |
| — — `Builder::build` (per-cell loop) | 55.7 | 1.5% |
| — `collapse_small_edges` | 70.8 | 1.9% |
| `connect_junctions` | 212.9 | 5.8% |
| `reorder_by_region_order` | 110.6 | 3.0% |
| `assign_bead_counts` | 62.1 | 1.7% |
| `generate_junctions` | 67.6 | 1.8% |
| `propagate_beadings_downward` | 57.2 | 1.6% |
| everything else named | <1.5% each | — |
| WIT in+out conversion | 1.2 | 0.03% |

The named-stage sum closes to 99.2% of the pipeline total — the 364.932 ms
remainder the old study could not explain is now covered. The old study's two
tied leads (preprocess 1,782.8 ms vs graph 1,785.3 ms) keep their shape: both
are real, preprocess is now slightly larger, and each is dominated by a single
sub-term (triple offset / boostvoronoi sweep).

## The one candidate

**`overhang_quartile`'s per-vertex query is the single largest reducible guest
term (38.1% of module fuel, ~1.64 ms/call ordinary), and the measured
reducible part is its `eps = 0.0` boundary-tolerance pre-pass.**

The evidence:

- `legacy_overhang_quartile` / `indexed_overhang_quartile`
  (`crates/slicer-core/src/perimeter_spatial.rs`) both classify points through
  `slicer_ir::point_in_polygon_winding(polygon, x, y, 0.0)`.
- That predicate runs two passes: a per-edge point-to-segment projection
  guarded by `dist2 <= eps²`, then the winding scan. At `eps = 0.0` the first
  pass can only fire for a point exactly on an edge, yet still pays a full
  projection per edge. **Measured: 62.8-66.3% of the predicate's own cost**
  across 64/512/4096-vertex rings, with identical inside/outside verdicts
  between the full predicate and a winding-only replica
  (`t25_probe_winding_pass_share_tdd`).
- The predicate's own doc-comment defines `0.0` as "strict containment
  (winding-number only)", so skipping the pass on that path is the contract the
  call sites already ask for.
- Cost is linear in band-polygon size (851 ns/call at 16 points, 3.07 µs at 64,
  9.6 µs at 256, 39.1 µs at 1024, 157.6 µs at 4096;
  `t25_probe_quartile_shape_tdd`), and the per-call cost tracks band complexity
  on real layers (layer 45: 59.4M fuel/call; layer 189: 2.0M fuel/call — 30x).

**Shape choice (design work for the candidate's own take).** Either narrow
`point_in_contour_winding`/`point_in_polygon_winding` so a `eps = 0.0` call
skips the boundary pass outright (the smaller change; needs a decision on
exactly-on-edge points, which the strict path currently admits via `dist2 <= 0`),
or keep the predicate whole and add an inclusive per-polygon bbox prefilter in
the quartile query so non-containing polygons never reach the O(N) scan
(value-preserving by construction; the saving is unmeasured because the
y-interval lookups may already reject most candidates). Both are inside the
38.1% term the attribution localizes; neither is implemented here.

**Ceiling arithmetic (fuel only, wall unmeasured).** The pre-pass removal alone
is bounded by the measured 63-66% of the predicate, itself 38.1% of module fuel
— a fuel ceiling of **~24-25% of module fuel**. The bbox shape's saving is a
bound, not a measurement (see Gaps). The map's measured fuel→wall transfer is
0-16%, so no wall win is implied.

**Not implemented here.** The candidate needs its own scope/authorization plus
the standing paired ordinary + accelerated A/B before any keep.

## Validation

- `python docs/specs/perf-vs-orca/evidence/t25-arachne-attribution/verify-t25.py`:
  exit 0; re-derives the 240-line capture, the stage shares, the guest split,
  the mode ratios, and every output hash from the raw captures.
- `python docs/specs/perf-vs-orca/evidence/t25-arachne-attribution/t25_split.py
  <capture>`: the reducer that produced `host-split.txt`.
- Probe-only tests: `t25_probe_quartile_shape_tdd` (linear-scaling evidence),
  `t25_probe_winding_pass_share_tdd` (the `eps=0` pass share). Both are
  probe-only and were removed with the rest of the probe.
- No Rust tests were run against production code: nothing in the production
  tree changed (probes stashed/removed); guests were rebuilt to production and
  `cargo xtask build-guests --check` exits 0 at the time of writing.

## Gaps

- Wall figures are instrumented (`--profile`) and indicative only; the
  deterministic result is the fuel split. Wall sums also include a
  non-trivial capture-to-capture spread (e.g. module wall 18,130 ms vs
  13,460 ms across identical builds), consistent with the map's §3.3 class.
- The bbox-hit share of `overhang_quartile`'s scans was NOT measured (the
  counting probe would have added O(N) work inside the function it measured
  and was removed before the final captures); the candidate's bbox saving is
  therefore a bound, not a measurement.
- The `eps=0` pass share was measured on synthetic rings in a probe test, not
  on production band polygons; production rings are similar closed contours but
  the exact share is not asserted.
- Layer 45's 30x per-call quartile cost is consistent with band-polygon size
  and vertex count, but the band polygon's own shape was not dumped; the
  scaling test supports the interpretation rather than proving it for that
  layer.
