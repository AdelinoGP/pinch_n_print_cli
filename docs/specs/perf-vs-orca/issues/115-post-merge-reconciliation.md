# Post-merge reconciliation: re-baseline the perf keeps and audit master's semantic drift

Type: task
Label: wayfinder:task
Status: open
Assignee: (unclaimed)
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
