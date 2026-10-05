# Serial host floor: prepass built-ins and slice-outside wall

Type: task
Status: open
Assignee: unassigned — last take: ses_f14ba59b7ffeteaZ0tDfSNEjpc (human route decision); earlier takes: ses_f1507d458ffetnZ0plQLQ4yUmf, ses_f1b6563ccffeO4IE3vsBNXl6QL and ses_f1994dd16ffeSxSBJeKjScCYRW
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

**Update 2026-09-24** — [Tree-planner substage
attribution](22-tree-planner-substage-attribution.md) closed with the stage's
line item measured at the matched job: `PrePass::SupportGeometry` is
**295.0 s** of the same capture's **551.8 s instrumented base prepass**;
`com.core.tree-support-planner` contributes 270.7 s (91.8% of that stage).
On benchy the stage is 11.42 s of its 24.93 s prepass and the planner is
10.36 s (90.7% of the stage). **97.2% base / 92.6% benchy instead refers to
the emit pass's share of the per-object planner.** Of that emit pass, 46.8%
is per-region carve (147,993 calls on base, not the stage's 171,636 clips),
27.8% union+simplify and 23.4% model-occupancy inflate. The stage-wide
`clip_polygons` bracket recorded 171,636 core boolean calls / 194.0 s;
the collision/avoidance ladders are 4.27 s of the 269.6 s per-object planner
(1.6%). The bbox-gate candidate and its measured representation caveat are in
that ticket's `## Answer`; its acceptance slot is the timing chain's next.
These are attribution figures, **not** an update to the separate 455.7 s
post-repair capture or an uninstrumented wall claim. The other `PrePass`
built-ins above keep their own lines.

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

## Answer

**Work item 1 resolved 2026-09-27 — R is measured same-run and is 15–36x
smaller than the budget assumed; the serial floor is P plus the elapsed tail,
not R.** Work items 2–5 (prepass built-in attribution) remain open.

Evidence: `evidence/t27-serial-floor/FINDINGS.md` (method, per-cell table,
elapsed-tail anatomy), `results/run-phase-splits.csv` (70 measured runs),
`results/CELL-MEDIANS.md`, `results/compute_phase_splits.py`. All numbers are
uninstrumented same-run (process wall/CPU from the committed scoreboard CSVs;
`elapsed_ms` and phase walls from the same runs' default progress-event
captures) — no instrumented figure enters a wall claim (§3.2).

**The slice-outside wall R that ticket 12 budgeted was a trap artifact.** The
51.2 s / 122.2 s figures came from subtracting *instrumented* stage-phase sums
from *uninstrumented* process wall in the `dev174-repair` single runs — §3.2
mixing. Same-run measurement of the same captures: **R = 1.85 s
pre-repair / 3.40 s post-repair** (ordinary; accelerated 5.06 s), i.e.
**c-min = 0.15–0.23x** on the worst cell, not 2.3x–5.5x. R scales with
input+output size (benchy ≈ 0.2 s; base 1.6–3.4 s) and is a non-term on
every cell.

**Elapsed-tail anatomy** (the gap between phase-sum and `elapsed_ms`, which
ticket 12 never attributed), same-run:

- **Pre-validation head** (~1.1–1.5 s in every capture): module discovery +
  24-guest WASM compile (~0.77 s, the t15 measured fixed term) + config/plan
  prep. Sits between `t0` and `phase_start(validation)`; confirmed by
  slice_id-creation → validation-start timestamp deltas.
- **Prepass → per_layer handoff** (~0.75 s on benchy; ~1 ms elsewhere).
- **Post-postpass tail**: pre-repair base supports-on carries **32.0–37.6 s
  of `module_error` diagnostic replays** — `emit_host_support_diagnostics`
  re-emits all 172,181 DEV-174 "body rejected" events at slice end (a
  pre-repair-only measurement artifact, gone post-repair). Post-repair, the
  tail is `estimate_print` + the `layer_count` G-code scan: 6.8 s accelerated
  vs **16.9 s ordinary on byte-identical 54.5 MB outputs** — the
  ordinary/accelerated 10 s difference is unexplained by static analysis and
  needs a same-run probe before any candidate is formed.

**Route consequences.** R retires as a budget term. The binding serial terms
are unchanged in kind (P: prepass 295–517 s base supports-on; per_layer), but
ticket 12's budgets table needs its c-min column corrected (every value
10–35x too high; this *weakens* the serial-bind conclusion only in R's
favour). The `estimate_print` tail (6.8–16.9 s on base supports-on) is real
serial cost inside `elapsed_ms` that no phase bracket sees — in scope for
this ticket's split, small against P.

**Next take (work items 2–5).** The prepass built-ins keep their own lines;
`commit_shell_classification_builtin`'s `apply_opening` arc-tolerance-0
anomaly (§9.1) and `compute_xy_footprint`'s exact union (§6 lead 2) are the
ranked leads, to be attributed at the matched job with the same-run phase
method this take established. A same-run probe patch
(`PERF-T27-PROBE`) bracketing the tail segments (diagnostics replay, layer
scan, `estimate_print`) resolves the 6.8-vs-16.9 s ordinary/accelerated
question first.

**Work items 2–5 executed 2026-09-27 — `PERF-T27-PROBE` attributed all four
stages at the matched job; `PrePass::Slice` (the #2 serial block) was missing
from this ticket's list and is adopted into it.** Evidence:
`evidence/t27-serial-floor/FINDINGS-SUBSTAGE.md` (+ `run_probe.ps1`,
`.probe.txt` files). Four uninstrumented probe runs (benchy/base × classic
off/on), guests fresh, probe lines outside the JSONL contract; `_wall`
brackets are calling-thread wall, `_sum` brackets accumulate across rayon
threads (§3.3 class, ranking only). Findings, ranked:

1. **`PrePass::Slice` promoted into this ticket** — 47–56 s wall of
   per-layer parallel map on base (2.2 s benchy), second only to
   SupportGeometry; internals still unbracketed (probe iteration 2 brackets
   the per-layer body: classify / bridge-assembly / closing-radius).
2. **ShellClassification's serial phase B** (internal-bridge gate commit
   loop) is the largest confirmed serial block outside Slice: 25–30 s wall
   per base cell; its cost is per-qualified-polygon intersections against
   `deep_infill_clip_area` / `internal_unsupported_area` /
   `expansion_area`. `apply_opening` is the largest worker term (74–93 s
   accumulated / 990 calls) but its wall is bounded by pass walls (≤8.5 s);
   its Round-join arc-tolerance-0 vs canonical `jtMiter` parity question
   stands, small prize.
3. **Overhang footprint exact union** (`compute_xy_footprint`, 17–18 s
   serial inside MeshAnalysis's 22–24 s): the consumer audit holds —
   `region_needs_support` reads it only via `expolygon_bbox`,
   `derive_needs_support` via one bbox-gated `intersection_ex`, bridge
   assembly intersects, visual-debug draws. No consumer needs the exact
   union for itself; a bbox/coverage replacement needs an output-equivalence
   test (needs_support flips) before any A/B.
4. **SupportAnalysis**: the serial sweep (24.7 s base-on) is the biggest
   serial block in the stage; the contact-detect pass is
   parallel-saturated (171.7 s accumulated / 14.7 s wall ≈ 11.7x). Its
   `derive_needs_support` re-derives what MeshAnalysis already measured
   (`region_needs_support`, negligible) — dedup candidate,
   representation-neutral.
5. **OverhangAnnotation**: interior is heavily parallel
   (`partition_into_bands` = 97.7% of accumulated fuel base-off, ~12x sum
   over wall); needs a wall bracket around `annotate_overhangs` before any
   candidate graduates. Below-fold for now.

The probe code is temporary and **stays out of any commit**: on resolution
of this take the two `perf_t27_probe` modules and their call-site brackets
are reverted (`crates/slicer-runtime/src/perf_t27_probe.rs`,
`crates/slicer-core/src/perf_t27_probe.rs`, and the bracket edits in
`prepass.rs` / `slice_postprocess_prepass.rs` / `prepass_slice_producer.rs`
/ `support_analysis_producer.rs` / `mesh_analysis.rs` /
`overhang_annotation.rs` / `run.rs`); the evidence (probe text + FINDINGS files)
stays.

**Probe iteration 2 executed 2026-09-27/28 — three open questions closed.**
Evidence: `evidence/t27-serial-floor/FINDINGS-SUBSTAGE-2.md` (+
`run_probe2.ps1`, `.probe.txt` files). Findings:

1. **ShellClassification phase-B is two full-layer Miter offsets** —
   `offset(deep_infill_area, +1.5×spacing)` (12.1 s/121 calls) plus the
   `−(4.5×spacing)` erosion (9.3 s/36 calls) are ~90% of the 25 s serial
   commit loop; the angles step I suspected is only 2 s. A per-layer memo
   shared across sibling regions is the identical-output-by-construction
   candidate (up to ~20 s serial wall per base cell; paired A/B before any
   keep/drop).
2. **`PrePass::Slice` fuel ranked**: `assemble_flat_bridge_areas` is ~70%
   of the stage's fuel (410–454 s accumulated vs the 47–56 s wall ≈ 10–12x
   parallel); bridge-assembly second; classify/closing small. Parallel
   stage — wall prize bounded by the 47–56 s wall.
3. **The 6.8-vs-16.9 s tail mystery resolves**: `estimate_print` on the
   final IR is ~0.1 s, not 16.9 s. The unbracketed remainder (7.53 s
   base-on here, 0.07–0.23 s elsewhere) is the **pipeline-boundary
   teardown** — `blackboard`/`layer_irs`/plan/runners/wasm handles dropped
   at `run_pipeline_core` scope exit between the postpass
   `phase_complete` event and the first tail bracket. Real critical-path
   wall, invisible to phase brackets, vintage/content-dependent; the
   accelerated 6.8 s sits in range. Below-fold candidate (early-release or
   parallel teardown; returns to the human).
4. **OverhangAnnotation wall** resolved: 14.2 s base-off / 12.2 s base-on
   vs 138–225 s accumulated → 11–16x parallel; below-fold confirmed.

**Premise audit 2026-09-28 (ranked candidates 1, 2 and 4).** The
[audit](../evidence/t27-serial-floor/PREMISE-AUDIT.md) supersedes the
*candidate interpretations*, not the measured brackets, above:

- **Drop the sibling-region phase-B memo for the matched job.** The planner
  supplies one base region per object before this stage; a debug skip census
  on benchy-on/base-on found only `region=0` (226/484 skip events; never
  more than one per object/layer). The deep-infill input depends on each
  timeline's geometry and previously committed bridge polygons; `(object,
  layer, spacing)` alone is not a sound cache key even in a multi-region
  future. The 121/36 base-on offset calls are real work, not duplicate calls
  across siblings. No candidate A/B is justified for this memo.
- **Footprint bbox replacement is unsafe as stated.**
  `SliceRegionView::derive_needs_support` uses exact `intersection_ex`, not a
  bbox predicate; SupportAnalysis and WASM marshal consumers use it. Bridge
  assembly reads *bridge* footprints, not this overhang field. A narrow test
  now covers disjoint triangles with overlapping bounding boxes. An exact-
  coverage alternative requires eligibility/representation proof before A/B.
- **Flat-bridge reduction remains unproven.** The stage already short-circuits
  empty inputs; its hot path uses a closing-derived mask and a fractional
  enclosure discriminator. Raw-slice intersection in place of its `support`
  difference is not representation-identical by construction, and a bbox
  cannot distinguish enclosure. Compare both positive and free-edge cases
  and production-layer IR before trying a paired wall/CPU A/B. No measured
  keep/drop recommendation is claimed for this candidate yet.

**Flat-bridge experiment 2026-09-28/29 — the tested `refilled`-mask removal is
DROP, not a stage verdict.** A mixed positive/free-edge narrow test passed,
but the real-mesh differential probe found a Benchy layer with 105 candidate
bridge polygons versus 104 legacy polygons (`SliceIR.bridge_areas` differs).
Two exploratory ordinary Benchy/classic-off pairs predated that finding;
median wall was 38.159 s legacy / 38.494 s candidate, process CPU 136.164 s /
132.070 s, load-qualified, not an acceptance A/B. **No accelerated or base A/B
was run after the output gate failed.** The temporary candidate and probe were
removed; details and the canonical `apply_opening` reassessment are in
[`FLAT-BRIDGE-EXPERIMENT.md`](../evidence/t27-serial-floor/FLAT-BRIDGE-EXPERIMENT.md).
The `apply_opening` lead is a geometry-parity investigation (canonical Miter/3,
width/10, versus current Round/0, width/2), not a representation-preserving
performance drop-in. The ticket remains open for a new safe call reduction.

## Comments

**2026-09-29 — human accepted the phase-B sibling-region memo DROP for the
matched jobs and chose to park this candidate search.** The matched jobs have
no eligible siblings, and the proposed memo key does not prove identical
geometry in a multi-region job. This accepts the candidate verdict, not a
claim that phase B is cheap: its measured offsets remain serial work. The
tested flat-bridge mask elimination is independently dropped for changed real-
layer `SliceIR.bridge_areas`; the `apply_opening` parity lead is separate and
was not changed. Next timing-chain work here requires a new output-safe,
measurably useful reduction with a real-layer IR oracle before paired A/B.
Meanwhile prioritize [Classic output-volume surplus](28-classic-output-volume-surplus.md)
for its disclosure-only attribution; that ticket is already open and does not
claim a serial-floor speedup. No candidate was committed in this take.
