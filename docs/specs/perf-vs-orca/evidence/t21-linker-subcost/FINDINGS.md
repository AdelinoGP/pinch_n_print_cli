# Ticket 21 — linker subcost attribution (linker owns the tail; the re-clip owns the linker)

**The measured answer:** `com.core.infill-linker`'s cost is **97.65% the
per-path re-clip** (`link_paths_without_offset`'s `clip_polylines` loop) —
**64.84% of all linker fuel is the repeated clip-universe pre-inflate** and
32.65% the clipper execute. Connectivity (`connect_infill`) is 2.01%, overlap
offset 0.03%, graph build 0.02%. The Arachne low-layer tail T43 measured is
**~99.95% guest execution** with host preparation/marshalling ~0.04%. One
candidate follows: hoist the clip-universe preparation out of the per-path
loop.

Attribution only: no fix, no commit, no acceptance retry. Fuel figures are
deterministic (`--profile`, ADR-0055) and ratios exact; wall figures are
instrumented and indicative only (map §3.2).

## The question this answers

Ticket 43's authorized diagnostic pair attributed 90.01–96.69% of the frozen
supports-off Benchy Arachne job's layers 1–2 to `com.core.infill-linker`
dispatch, but its bracket (`on_module_start` → `on_module_end` in
`crates/slicer-runtime/src/layer_executor.rs`) encloses host region-view
preparation, dispatch/marshalling **and** guest execution, so it could not say
which. This study splits that boundary and then splits the guest work.

## Provenance and integrity

- Frozen inputs: ordinary snapshot
  `t38-campaign-20261001T024827Z/ordinary/`, corpus
  `t41-20260930T232255Z/corpus/supports-off-benchy/`.
- Snapshot revision `2ebc372a`; `git diff --stat 2ebc372a..HEAD -- crates
  modules resources xtask` touches only
  `resources/perimeter-acceptance/run-acceptance.ps1` and a new
  `test-status-roundtrip.ps1` — the linker, `slicer-core`, and the host are
  unchanged between the snapshot and HEAD.
- `cargo xtask build-guests --check` exit 0 before the first run; the frozen
  reference identity gate (`verify-freeze.py`) passed.
- All four measured outputs are byte-identical to the frozen reference
  (`7049a06d…`): the baseline profiling run, both probe runs, and the
  accelerated probe run (re-derived by `verify-t21.py` from the in-repo
  reduced captures at `captures/`, and re-runnable against the durable raw
  tree with `--capture-root`).
- The probe replaced **one** guest (`infill-linker.wasm`) in a complete module
  dir; the other 23 modules stayed bit-for-bit frozen. Probe sources are
  preserved in `probe.patch` and were stashed out of the working tree, not
  committed.

## Capture 1 — no rebuild, frozen everything

`--profile --profile-verbose` on the frozen ordinary snapshot:

- Linker fuel **109,636,786,250 = 45.8% of the slice's 239,432,389,779**
  (second after `com.core.arachne-perimeters`, 127.6B = 53.3%).
- Global layers 0, 1, 2 (zero-based, as the trace reports them) carry
  **95.38B = 87.0%**
  of all linker fuel on 3 of 240 dispatches. Layer 1: 53.87B / 4,126 ms;
  layer 2: 40.05B / 2,715 ms; layer 0: 1.46B / 135 ms.
- The marked primitives account for only **0.17%** (194 `clip_polygons`
  calls); **99.83% is unmarked guest module self-code**.

## Capture 2 — probe guest; the boundary and the split

Guest-only probe (`T21-PROBE`): fuel scopes around each linker sub-operation,
per-layer counters, and a `now_us` export-call wall bracket. Output
byte-identical again.

### Host prep/marshalling vs guest execution

`fn_us` (export entry → exit via `now_us`) against the `module_complete`
elapsed:

| layer | module elapsed | in-export | host side |
|---:|---:|---:|---:|
| 0 | 140 ms | 138.8 ms | 1.2 ms (0.89%) |
| 1 | 4,155 ms | 4,153.3 ms | 1.7 ms (0.04%) |
| 2 | 2,808 ms | 2,807.8 ms | 0.2 ms (0.01%) |

**The tail is guest execution; host preparation/marshalling is negligible.**
This closes the boundary ticket 43 left unmeasured, for these three layers.

### The sub-operation split (fuel, all 240 layers)

| scope | calls | self fuel | share | layers 0–2 share |
|---|---:|---:|---:|---:|
| `t21::path_reclip` (whole per-path clip loop) | 212 | **107,062,264,520** | **97.65%** | 98.44% |
| `t21::connect_infill` (`modules/core-modules/infill-linker/src/connect.rs`) | 212 | 2,203,531,440 | 2.01% | 1.33% |
| `t21::overlap_offset` (`ExPolygonWithOffset`) | 185 | 29,352,317 | 0.03% | 0.01% |
| `t21::graph_build` (`BoundaryInfillGraph::new`) | 212 | 21,979,325 | 0.02% | 0.01% |
| `t21::boundary_prep` (per-role boundary + locked-footprint diff) | 185 | 2,960,323 | 0.00% | 0.00% |
| `t21::records_build` (region records) | 240 | 4,597,461 | 0.00% | 0.00% |
| `t21::output_emit` | 240 | 495,065 | 0.00% | 0.00% |
| `t21::majority_owner` | 0 (single-region job) | 0 | 0.00% | 0.00% |

`t21::path_reclip` encloses `link_paths_without_offset`
(`modules/core-modules/infill-linker/src/orchestrate.rs`), which loops every
selected path calling `clip_to_offset_boundary`
(`modules/core-modules/infill-linker/src/offset.rs`) → `clip_polylines`
(`crates/slicer-core/src/polygon_ops.rs`); it is the linker. Note
`majority_owner` is 0 calls here: the matched job's groups all take
`link_region_group`, not `link_union_group` (the owner-assignment term only
exists on the union path).

### Workload shape (counters)

| layer | clip calls | in pts/call | clip-universe pts/call | out polylines | rings | boundary pts |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 428 | 2 | ~5,665 | 365 | 843 | 34,771 |
| 2 | 317 | 2 | ~4,661 | 294 | 66 | 9,366 |
| 0 | 84 | 2 | ~1,204 | 84 | 4 | 1,204 |
| 3–239 median | 8 | 2 | ~187 | 8 | 1 | 160 |

Every call clips one 2-point polyline (9,040 input points over 4,520 calls)
against a clip universe of up to ~5,700 points.

## Capture 3 — inside the re-clip

Three temporary core scopes inside
`slicer_core::polygon_ops::clip_polylines`
(`crates/slicer-core/src/polygon_ops.rs`) split the call into its clip-universe
pre-inflate and its clipper execute. Linker total fuel 109,694,763,754:

| term | fuel | share of linker |
|---|---:|---:|
| `t21::clip_polylines_inflate` (flatten + `inflate_paths_64` universe) | 71,125,426,244 | **64.84%** |
| `t21::clip_polylines_execute` (clipper add + execute) | 35,815,011,999 | **32.65%** |
| `t21::clip_polylines_total` self (subject prep, glue) | 159,101,340 | 0.15% |
| `t21::connect_infill` | 2,203,526,742 | 2.01% |
| residual | 391,697,429 | 0.36% |

Tail dominance holds inside the split: 85.8% of the inflate and 91.6% of the
execute are on layers 0–2. Per call ≈ 15.7M fuel inflate + 7.9M fuel execute;
per boundary point ≈ 12.6k fuel inflate + 6.3k fuel execute.

## Capture 4 — accelerated mode

The same probe on the accelerated snapshot (linker wasm `c519aeb6…`, differs
from the frozen accelerated `7a8a5c01…` only by the probe): output
byte-identical, and the linker sub-split is **numerically the same to within
1.1e-7 relative**:

| term | ordinary | accelerated |
|---|---:|---:|
| linker total fuel | 109,694,763,754 | 109,694,775,462 |
| `inflate` | 71,125,426,244 | 71,125,426,244 |
| `execute` | 35,815,011,999 | 35,815,011,999 |
| `connect_infill` | 2,203,526,742 | 2,203,526,742 |
| slice total fuel | 239,490,363,178 | 185,503,596,541 |

In accelerated mode the linker **is the #1 fuel consumer (59.1%)**, ahead of
`com.core.arachne-perimeters` (39.7%). Acceleration cuts the perimeter module,
not this work — the linker's clip work is untouched by the accelerated cfg
(which only selects `perimeter_spatial`'s indexed dispatch). This settles the
"does accelerated mode reduce that particular work?" question ticket 43
raised: **no.**

## The one candidate

**Hoist the clip-universe preparation out of the per-path loop.** Within one
`link_paths_without_offset` invocation every `(owner, path)` clips against the
*same* `boundary`, yet `clip_polylines` flattens and pre-inflates that
boundary on every call (the map's recipe note: the pre-inflate exists to make
on-edge spans strictly interior, guaranteeing AC-5).

Ceiling arithmetic: 4,520 clip calls across 212 `path_reclip` invocations plus
27 raw-boundary fallbacks. A per-invocation prepare needs at most 212 + 27 =
239 preparations.

- **Uniform-count estimate:** 1 − 239/4520 = 94.71% of the inflate term = 61.4%
  of linker fuel ≈ 28.1% of slice fuel.
- **Layer-aware estimate:** per-call inflate is heterogeneous (median 2.3M
  fuel/call on layers > 2 vs 81.0M on layer 1), so the saving is re-derived
  per layer as Σ `inflate_layer × (1 − preparations_layer / calls_layer)`,
  with `preparations_layer` = `path_reclip` invocations + fallbacks for that
  layer. That gives **98.24% of the inflate term = 63.70% of linker fuel**,
  because the expensive layers already have the fewest preparations-per-call.

The two numbers are close and both are **assumption-based estimates, not
measurements**: they assume the deduplicated preparation costs the same as one
current per-call preparation, which the fuel split does not prove (the universe
cost tracks boundary size, and the surviving preparations may carry the largest
universes). `verify-t21.py` re-derives the layer-aware figure from the
counters. Wall transfer is unmeasured; this map has measured 0–16% fuel→wall
transfer, and the candidate's own paired ordinary + accelerated A/B is
required before any keep.

Exactness: value-identical by construction — the prepared universe is the same
deterministic `inflate_paths_64` result the current code recomputes per call,
and `clip_polylines`'s AC-6 already guarantees independent per-polyline
clipping. The other three production call sites keep the current one-shot API:
`convert_to_lines` (`crates/slicer-core/src/algos/lightning/layer.rs`), the
tree-sampling loop in `generate_lightning_trees`
(`crates/slicer-core/src/algos/lightning/mod.rs`), and
`floating_edges_of_gated_area`
(`crates/slicer-core/src/algos/prepass_slice.rs`).

**Not implemented here.** It needs its own scope/authorization, then the
standing paired A/B; a host-side API extension or an in-guest precomputed
universe are both live shapes and the choice belongs to that take.

## Recommendation

**No keep/drop recommendation is possible from this ticket.** The ticket's
acceptance line requires a paired-mode A/B before a keep/drop, and this take
authorized attribution only (no fix, no A/B). The recommendation is handed to
the candidate's own take — [Clip-universe preparation hoist: implementation +
standing paired A/B](../../issues/44-clip-universe-hoist.md) — whose gates are
an exactness proof plus the standing paired measurement. That is a deliberate
scope boundary, not an omitted deliverable.

## Validation

- `python docs/specs/perf-vs-orca/evidence/t21-linker-subcost/verify-t21.py`:
  exit 0 against the in-repo reduced captures (`captures/`); re-derives the
  output hashes, the capture-1 shares, the capture-3 split and tail share, the
  layer-aware hoist saving, the cross-mode invariance, the accelerated ranking,
  and the L1 counters/wall bracket. `--capture-root
  .local-artifacts/perimeter-reference-preparation/t21-linker-subcost-run1`
  runs the same checks against the full durable raw tree.
- `python docs/specs/perf-vs-orca/evidence/t21-linker-subcost/reduce_captures.py`:
  regenerates `captures/` (probe lines + `profile_summary` +
  `module_complete` for `com.core.infill-linker`) and `output-hashes.json`
  from the raw tree.
- No Rust tests were run against production code: nothing in the production
  tree changed (probe stashed; `probe.patch` preserved).
- Guests were rebuilt to production after the probe; `cargo xtask
  build-guests --check` exits 0.

## Gaps

- Wall figures are instrumented and indicative only; the fuel split is the
  measured, deterministic result.
- `module_complete`'s `wasm_peak_kb` on layers 1–2 (8,320 / 4,992 KiB vs a
  1,280 KiB median) shows the tail also allocates disproportionately, but
  allocation was not instrumented; no claim is made about it.
- The probe's own marks inflate wall (measured: capture 3's in-export wall is
  5,304 ms on L1 / 3,232 ms on L2 versus capture 2's 4,153 / 2,808 ms — the
  three extra core sub-scope marks add six host calls per clip call); they do
  not perturb fuel (ADR-0055).
- The saving figures are bounds under a same-cost assumption, as above; the
  candidate's wall effect, and any host-vs-guest API choice, are unmeasured
  until the separate take.
