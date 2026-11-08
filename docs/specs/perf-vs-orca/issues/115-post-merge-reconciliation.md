# Post-merge reconciliation: re-baseline the perf keeps and audit master's semantic drift

Type: task
Label: wayfinder:task
Status: resolved (2026-11-04; **the merge is output-neutral on the matched job
— all eight cells grow by exactly the 34-byte line
`; prepared_plan_diagnostic = none` — but it regresses the per-layer phase
1.9x–6.0x with process CPU +17…+54 % in 12/12 pairs; no ratio of record may be
quoted against this tree, and the regression is graduated as
[ticket 121](121-post-merge-per-layer-dispatch-regression.md); see ## Answer**)
Assignee: 2026-11-04 OpenCode wayfinder session
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the 2026-11-03 merge `a5e9a403` — 52 master commits (lightning
migration, preparation capability, shared selection, variant chains) merged
into the branch's perf work.

## Question

After merging `origin/master`, which of the map's perf keeps and measurements
still hold, which outputs or disclosures moved, and what must be re-baselined
before the map quotes another ratio?

The merge is clean and green on the narrow gates, but "compiles and tests pass"
is not the same claim as "the pre-merge numbers still describe this tree".

## What the merge changed (measured, not assumed)

- **Structural keeps survive.** The merged tree still carries t78's
  `EMIT_BATCH_LAYERS = 4` / `CLIP_BATCH_MAX_REQUESTS = 256`, t95's
  `prefetch_cross_sections`, t90's `execute_layer_finalization_owned`, t102's
  mimalloc allocator, t81's `resolution` call sites, and the vendored
  clipper2-rust **1.1.0** (t31's decision). This is a source-level check only;
  no post-merge timing exists.
- **The plumbing changed.** Master brought variant-chain identity in
  `ExactZQueryService`, `ContactWork` (variant chain + exact address) in support
  analysis, `run_pipeline_core` / the preparation lifecycle in the pipeline, and
  shared-selection dispatch. The merge resolutions preserve the branch's work,
  but the code paths those keeps sit on are not the pre-merge paths.
- **One real output fix landed with the merge.** Classic paint attribution now
  reads the **original, pre-simplification** polygons
  (`classic-perimeters/src/lib.rs`): t81's `resolution` re-union can reorder ring
  vertices, which was misattributing tool indices near material boundaries. That
  is a correctness fix, but it changes painted-job output and has no fresh
  reference/parity audit yet.
- **Config disclosure moved.** The merged tests and oracle now treat
  `resolution` as its own key (distinct from `gcode_resolution`) with the
  canonical **1 µm** clamp; the manifest-default table and the config block were
  reconciled. This needs a disclosure/parity check, not just green tests.
- **No post-merge scoreboard exists.** The map's ratios of record (t103/t106)
  and phase splits (t104/t110) predate master's lightning migration and
  preparation lifecycle.

## Scope and method

1. **Semantic-drift audit.** Diff the merged tree's hot paths against the
   pre-merge branch for the keeps above, and list every behavior change the merge
   introduced (not only compile/test fixes). The paint-attribution fix is the
   named example; find the others.
2. **Paint/output audit.** For the painted-fixture jobs, compare pre-merge vs
   merged G-code: TYPE/feature counts, byte volume, tool attribution, and
   determinism across the ordinary/accelerated arms. Decide whether a reference
   re-record is required and disclose the delta. Do not quote a painted-job ratio
   until this lands.
3. **Post-merge scoreboard.** Re-run the paired ordinary/accelerated scoreboard
   on the merged tree (four cells minimum, all eight if the budget allows) on a
   quiet host, under the standing proof gate. Compare to t103/t106 and flag any
   cell that moved beyond its own spread. Do not assume the merge is
   performance-neutral.
4. **Ledger reconciliation.** Re-derive the ratios of record and the t110
   base-on caveat from the new measurements; update the map's Notes/Decisions
   only after they land.
5. **Rule on the differences.** The paint-attribution output change, the
   preparation-lifecycle cost on the measured cells, and any dependency/toolchain
   drift (master's lockfile versions vs the branch's pins).

## Stop rule

Measurement and audit only; no production edit beyond the already-committed
merge fixes. If a keep regressed, record it with its measured delta and graduate
a separate candidate ticket; do not silently re-tune. If the scoreboard cannot
run quiet, disclose the load and bound the result rather than quoting a noisy
median.

## Deliverable

A reconciliation report with the keep audit, the painted-output delta, a fresh
scoreboard (or a disclosed bound), the exact map/ticket text changes, and a
list of any new candidate tickets. Evidence under
`evidence/t115-post-merge/` (`verify.py` exits 0 from a plain checkout).

## Evidence starts

- Merge commit `a5e9a403` and its resolution commits.
- `crates/slicer-wasm-host/src/exact_z_query.rs`,
  `crates/slicer-runtime/src/builtins/support_analysis_producer.rs`,
  `crates/slicer-runtime/src/pipeline.rs`,
  `modules/core-modules/classic-perimeters/src/lib.rs`.
- [`evidence/t103-orca-rebaseline/`](../evidence/t103-orca-rebaseline/SCOREBOARD.md),
  [`evidence/t106-post-pin-scoreboard/`](../evidence/t106-post-pin-scoreboard/FINDINGS.md),
  [`evidence/t104-phase-split/`](../evidence/t104-phase-split/phase-split.json),
  [`evidence/t110-orca-phases/`](../evidence/t110-orca-phases/FINDINGS.md).
- [`evidence/matched-pair/run_scoreboard.ps1`](../evidence/matched-pair/run_scoreboard.ps1).

## Answer

Resolved 2026-11-04 at `9a7371aa` (merge `a5e9a403` + 13 branch commits;
`origin/master` has nothing newer). Full record:
[`evidence/t115-post-merge/FINDINGS.md`](../evidence/t115-post-merge/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 403 checks, 6/6 negative
controls rejected).

### 1. Output: the matched job is unchanged

All eight matched cells' ordinary output grows by **exactly +34 bytes** — the
one config line `; prepared_plan_diagnostic = none`. A direct line diff of the
four classic cells against the pre-merge t104 captures returns that single hunk
and nothing else: no move, no `;TYPE:`, no extrusion changes. The post-merge
reference set is recorded in `summary.json` `output_audit`
(`22200671… → 4b6d2843…` on base classic-on, `3672dc7e… → 39298f40…` on benchy
classic-off). **No toolpath re-record is needed; the hash set moves by the
disclosure line.**

The merge's classic paint-attribution fix *does* change painted output, as
expected: `cube_4color.3mf` loses 1.08 % of its bytes and **moves up to 8.19 %
of extrusion between tools** (T0 −8.19 %, T1 −3.18 %, T2 +1.73 %, T3 +5.06 %)
while total extrusion moves **−0.11 %**; `painted_seams.3mf` keeps its move count
and changes 9,608 lines of order/start vertex. Both arms are deterministic. The
map holds no painted-job ratio, so nothing it quotes depends on this; any future
painted claim must re-record first.

### 2. Performance: a real per-layer regression

Paired pre-merge vs merged A/B, four classic cells, three interleaved pairs,
**load 71 % at start** (browser + chat), so wall is a disclosed bound:

| cell | wall Δ (median) | CPU Δ (median) | per-layer ratio |
| --- | ---: | ---: | ---: |
| benchy classic-off | +19.26 % | **+24.11 %** | 2.23x |
| benchy classic-on | +4.73 % | **+16.96 %** | 1.91x |
| base classic-off | +42.29 % | **+53.57 %** | 6.02x |
| base classic-on | +13.85 % | **+34.88 %** | 5.08x |

CPU rises in **12/12 pairs**; the two negative wall pairs are single-run load
spikes inside the cell's spread. The prepass is unchanged (−0.3 % instrumented on
base classic-off); the whole cost is the per-layer phase, and it is **not guest
work**: call counts are identical while `fuzzy-skin` and `seam-placer` go from
162/211 ms to 56,068/56,226 ms, and ten more modules land at a near-uniform
~28.5 s accumulated. The per-call cost grows with layer index (55 ms at layer 0 →
141 ms at the last layers).

The accelerated dist build does **not** show it (2,318 ms per-layer on benchy
classic-off vs ordinary 4,807–5,860 ms, same output hash), which points at a
build-mode-sensitive host path. Limitation: the pre-merge accelerated snapshot
was overwritten by the merge's required rebuild, so that comparison is
"accelerated post-merge ≈ ordinary pre-merge", not a measured accelerated A/B.

### 3. Keeps: all structurally present

t78 (`EMIT_BATCH_LAYERS`, `CLIP_BATCH_MAX_REQUESTS`), t95
(`prefetch_cross_sections`), t90 (`execute_layer_finalization_owned`), t102
(mimalloc allocator), t101 (support-identity table) and t81 (`resolution` sites)
all survive the merge at source level. Their prices are **not** re-measured: the
per-layer regression dominates the phase they sit in and would confound the
brackets until it is fixed.

### 4. Ruling

1. **No ratio of record may be quoted against the merged tree.** t103/t106
   ratios and t104/t110 phase splits are pre-merge; t110's Orca side is fresh
   but its PnP side is t104's table.
2. **The post-merge Orca scoreboard is deferred, not dropped.** Running it now
   would bake a known regression into the ledger; it becomes the closing gate of
   [ticket 121](121-post-merge-per-layer-dispatch-regression.md).
3. **References move to post-merge** (matched toolpaths unchanged; painted
   fixtures need their own re-record).
4. **The merge's preparation-lifecycle cost on the measured cells is this
   regression**, not a separate term.
