# Accelerated residual query attribution — matched Classic, supports off

Ticket: [Accelerated residual query attribution](../../issues/18-accelerated-residual-query-attribution.md).
The underlying unmodified matched-job config is
`../matched-pair/configs/pnp-classic-supports-off.json`; inputs are the
licence-encumbered `tmp/3dbenchy.stl` and `tmp/base.stl` (not committed).
`cargo xtask build-guests --accelerated --check` exited 0 before the capture.
The full `target/dist-accelerated/developer/` snapshot was used with its
`modules/` directory; `pnp_cli module diagnose` returned `pass: true` and all
loaded modules `external`. Captures used `--profile --instrument-stderr` for
**fuel attribution only**. Do not use their walls for acceptance.

## First split — probe and scratch provenance

Temporary `PERF-T18-PROBE` scopes are preserved as `probe.patch`; their
unmodified counterpart was captured before editing. Both versions used the
same matched config. `summary-stage1.json` is derived by `summarize.py` from
the local (gitignored) captures in `target/t18/`:

| local capture | purpose |
| --- | --- |
| `benchy-profile.jsonl` / `base-profile.jsonl` | pre-probe accelerated baseline |
| `benchy-probe.jsonl` / `base-probe.jsonl` | accelerated scopes in `PerimeterSpatialContext` |

`ScopeTotals::total_fuel` includes nested scopes. The script sums *inclusive*
totals only for the three disjoint public query scopes (`distance`, `quartile`,
`bridge`); their child leaves use `self_fuel` for non-overlapping accounting.
For each fixture, indexed leaves + fallback scans + reason overhead + bridge
precheck + parent wrappers equal inclusive query fuel **exactly** (script
assertion); fallback call classifications and query counts reconcile. No
`other_fallback` or unsafe bridge fallback fired in either matched run.

| Fuel (billion wasmtime instructions) | Benchy | base |
| --- | ---: | ---: |
| classic module, unmodified | 467.964 | 6,601.044 |
| classic module, probe | 468.025 | 6,601.451 |
| three query scopes (inclusive) | 325.111 | 4,081.419 |
| `indexed_inside` (tree candidates **plus exact winding predicate**) | 153.360 | 2,866.456 |
| `indexed_quartile` (tree candidates **plus exact winding predicate**) | 148.343 | 1,064.596 |
| `indexed_distance` (tree + nearest-edge exact evaluations) | 13.561 | 88.879 |
| `indexed_bridge` (tree + exact point-in-polygon) | 3.004 | 55.631 |
| explicit legacy fallback scan bodies | 6.370 | 2.934 |
| bridge arithmetic precheck | 0.114 | 0.971 |
| context construction (not part of public query scopes) | 5.057 | 32.949 |

Call counts: Benchy / base: public distance 617,996 / 3,408,985;
public quartile and bridge 628,675 / 3,416,107 each;
`IndexState::Linear` fallbacks 241,599 / 40,762, mostly bridge
(230,920 / 33,640); quartile fallbacks 10,679 / 7,122; **distance
fallbacks zero**. The `bridge_arithmetic_safe` precheck fired on all indexed
bridge queries (397,755 / 3,382,467); its *unsafe* fallback never fired.
The fallback *count* on Benchy is not a large fallback *fuel* share: the
scan bodies are 1.96% of the three query scopes; on base, 0.07%.
Per-call `indexed_inside` fuel rises from 248,158 (Benchy) to 840,853
(base), a measured **3.39× increase per query**, while indexed distance
rises only 1.19× per query. This is a source of the benchy→base
superlinearity; it does not by itself tell whether rstar candidate counts or
exact polygon scans grew.

Both variants completed non-degraded with zero nonfatal errors. G-code bytes
and TYPE section counts are in `summary-stage1.json`; Benchy's `Inner wall`
differs by three sections, while base TYPE counts agree and bytes differ
slightly. These coarse checks are disclosure, **not** exact parity. No wall
improvement or optimization has been claimed.

## Remaining discrimination

The first split proves that `indexed_inside` and `indexed_quartile` dominate
the accelerated query scope, **not** that rstar traversal dominates. Both
invoke `point_in_polygon_winding` on shortlisted whole polygons, which can
still scan their vertices. A second non-overlapping bracket set must split
tree candidate collection from exact polygon predicates before choosing a
query-optimization candidate. The `u128 envelope` premise is inapplicable to
these f64 Y-envelope paths; `u128` arithmetic is in
`bridge_arithmetic_safe` (`crates/slicer-core/src/perimeter_spatial.rs`),
whose measured precheck is a small part of both query totals.

## Stage 2 — tree traversal vs exact winding predicate (the decisive split)

The stage-1 `indexed_inside` / `indexed_quartile` figures lump rstar
candidate collection with the exact `point_in_polygon_winding` scan. Stage 2
adds non-overlapping leaf scopes (`probe.patch`; stage-1 probe kept as
`probe-stage1.patch`), each a sibling nested inside its stage-1 parent:

| Stage-2 leaf | Wraps |
| --- | --- |
| `inside_collect` / `quartile_collect` / `bridge_collect` | `locate_in_envelope_intersecting` + `BTreeSet` candidate build |
| `inside_scan` / `quartile_scan` / `bridge_scan` | the exact-predicate loop over candidate polygons |
| `distance_seed` | `nearest_neighbor` |
| `distance_range` | envelope range query + per-edge `edge_distance_sq` |

Captures: `target/t18/{benchy,base}-{probe,arachne}-stage2.jsonl` (scratch,
gitignored; digests in `stage2.json`). The same reconciliation identity holds
— for each run, query-scope inclusive fuel equals the sum of every leaf and
parent self-fuel exactly (residual 0 in all four).

**Result (fuel, accelerated, matched supports-off; classic = `probe-stage2`,
Arachne = `arachne-stage2`):**

| Fuel (billion) | benchy classic | base classic | benchy arachne | base arachne |
| --- | ---: | ---: | ---: | ---: |
| module total | 467.50 | 6,598.34 | 51.87 | 471.59 |
| three query scopes (inclusive) | 324.93 | 4,080.31 | 41.20 | 381.89 |
| `inside_scan` (winding over candidates) | **147.13** | **2,814.51** | 18.58 | 267.17 |
| `quartile_scan` | 141.48 | 1,004.66 | 18.07 | 89.74 |
| `inside_collect` + `quartile_collect` | 12.79 | 110.17 | 1.61 | 10.27 |
| `distance_seed` + `distance_range` | 13.24 | 87.14 | 1.71 | 8.84 |
| `bridge_scan` + `bridge_collect` | 2.91 | 54.80 | 0.40 | 5.02 |
| context construction | 4.72 | 30.95 | 4.70 | 30.71 |

**Reading.** The residual is **not** rstar traversal and **not** u128
envelope math: candidate collection is 3.9–5.5% of the inside+quartile
scan+collect fuel on classic (4.4–4.5% Arachne). The dominant cost is the
exact winding predicate loop itself — `inside_scan` is 24–55× its collect
cost — i.e. the "1.9× not O(log n)" question resolves to *tree pruning is
working and cheap; the per-candidate exact polygon scan is the residual*.
Per-query `inside_collect` fuel is near-constant (9.8k–15.0k instructions),
while `inside_scan` per query grows 3.5× benchy→base (237,602 → 825,589
classic) — the scan, not the tree, carries the measured benchy→base
superlinearity.

Arachne runs confirm the same shape at smaller absolute scale (query scopes
79–81% of the arachne module): the residual's location is not a
generator-specific artifact of the query path.

All four stage-2 runs completed `degraded: false`, zero non-fatal errors.
G-code bytes retained in `stage2.json` (benchy classic 7,447,418 vs stage-1
probe 7,443,967; base classic 21,936,463 vs 21,936,043 — within the known
same-binary noise band, §3.6).

## Candidate (for the ticket's recommendation, not authorized)

Ranked, semantics-preserving candidates from the measured split plus the
delegated code audit (query paths in `PerimeterSpatialContext`,
`crates/slicer-core/src/perimeter_spatial.rs`):

1. **2-D per-polygon bbox records for the inside/quartile trees** —
   `BoundaryYRecord`/`QuartileYRecord` are X-degenerate by construction
   (`y_interval_envelope`), so candidate sets reduce only by Y. A per-polygon
   2-D bbox record (the `BridgeRecord` pattern) is an exact superset filter
   and adds the missing X pruning. Largest measured target:
   `inside_scan`+`quartile_scan` are 89% (base classic) of the query scope.
2. **Pre-converted vertex cache + early exits** — the winding predicate
   performs ~6n+4m `units_to_mm` divisions per candidate per query
   (`slicer_ir::point_in_polygon_winding`); a per-context converted-coordinate
   cache is bit-exact. `indexed_inside`/`indexed_bridge` also fail to
   short-circuit where their legacy oracles (`.any()`) do — restore parity.
3. **Descending first-hit for `indexed_overhang_quartile`** — max over bands
   is achieved at the first hit in descending `(band, polygon)` order.

These are attribution-scoped candidates only: fuel→wall transfer is measured
0–16% elsewhere; none is a gap-closer against G = 6.28–26.24× per
[Gap budget per cell](../../issues/12-gap-budget-per-cell.md). The
falsified u128 suspect and the small fallback/precheck terms need no further
work.
