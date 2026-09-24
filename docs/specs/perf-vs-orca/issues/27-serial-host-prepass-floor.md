# Serial host floor: prepass built-ins and slice-outside wall

Type: task
Status: open
Blocked by: 24

## Question

[Gap budget per cell](12-gap-budget-per-cell.md) shows the serial terms bind
every measured cell: with all per-layer work free, the serial prepass alone
still loses benchy ~2.0–2.5x and base supports-on ~15–20x, and the
slice-outside wall R measures 2.3x (5.5x post-repair) of Orca's whole budget on
base — none of it attributed or ticketed before this ticket. Attribute and
reduce this floor. Base supports-off has no matched-job phase split at all;
producing it is the first deliverable.

Work, in order:

1. **Slice-outside wall first (cheap; unblocks the rest).** Re-derive same-run
   process wall − `run_slice_with_collector`'s `wallclock_ms`
   (`crates/slicer-runtime/src/run.rs`) per cell class (the existing figures —
   51.2 s pre-repair / 122.2 s post-repair on base classic-on — come from the
   `dev174-repair` single runs), and attribute it: model ingest (STL parse of
   2.4M triangles), module discovery/instantiation (24 guests), G-code write
   (21–54 MB), process startup/teardown.
2. **`commit_shell_classification_builtin`
   (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`) residual** —
   `unsupported_span_areas`, the `deep_infill_clip_area` offset, the `retain`
   erosion (`evidence/PERF-HANDOFF.md` §9.7), and `apply_opening`'s round-join
   arc-tolerance-0 anomaly (§9.1; every other round-join morphological pass
   uses `MORPH_ROUND_ARC_TOLERANCE_MM`).
3. **`commit_support_analysis_builtin`
   (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`)** —
   37.8–39.1 s base / 1.9 s benchy (2026-09-05 vintage; re-derive at the
   matched job).
4. **`compute_xy_footprint` (`crates/slicer-core/src/algos/mesh_analysis.rs`)** —
   the surviving exact Clipper union over overhang facets: 18.1 s on 326,546
   facets on base (~60% of `host:mesh_analysis`; 186x benchy's 0.097 s on
   14.8x facets — superlinear). Open question from `evidence/PERF-HANDOFF.md`
   §6 lead 2: does any consumer need the exact union, or only bbox/coverage?
5. **`commit_overhang_annotation_builtin`
   (`crates/slicer-runtime/src/builtins/overhang_annotation_producer.rs`)** —
   11.9–17.6 s base / 0.8 s benchy.

`host:slice`'s wall is back in scope as of
[host:slice closing_ex span contradiction](24-host-slice-closing-span-contradiction.md)'s
resolution (2026-09-23) — the exclusion that deferred it here is spent.

Constraints: output preserved exactly (any delta disclosed per the fairness
contract); substage splits inherit `fold_marks`' verdict from ticket 24 (the
blocking edge); all measurement follows the recipe traps (§3.2/§3.3/§3.5/§3.6 —
uninstrumented wall only, repeats on base, starvation exclusion).

Deliverable: the matched-job serial-floor split (P and R per cell class) plus
ranked reduction candidates; then paired ordinary + accelerated A/B per the
standing metric for each accepted candidate. Keep/drop to the human; no
auto-commit.
