# Gap budget per cell

Type: task
Status: resolved
Blocked by: 11

## Question

Convert the scoreboard gap into per-cell budgets: for each of the 8 matrix
cells, how much wall must move — and from which stage — for PNP to overtake
Orca?

Work:

- Combine [Matched-pair rig and first scoreboard](11-matched-pair-rig-and-scoreboard.md)'s
  per-cell gaps with existing attribution (the phase walls of
  `docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` §5, the fuel splits of tickets 08/09) into an Amdahl
  budget per stage and generator: which stages must shrink by what factor per
  cell, and which cells bind (the hardest cell decides the route).
- Re-rank the optimization tickets (19–25) against the finish line: promote,
  demote, or kill candidates below irrelevance for the binding cells, and state
  the new take order (the timing chain may be re-wired accordingly).
- State where acceleration already pays (ticket 09's −35.8% fuel) versus where
  the remaining gap actually lives per cell.

Analysis only — no timing runs. Deliverable: the budgeted, ranked route.

## Answer

Resolved 2026-09-23 — analysis only, no timing runs. Derived from the
ticket-11 scoreboard (`evidence/matched-pair/SCOREBOARD.md`, the only
same-vintage cross-cell measurements), the tickets 08/09 fuel splits
(`evidence/perf-emit-walls/FINDINGS.md`), the phase splits in
`evidence/PERF-HANDOFF.md` §5 + the `perf-emit-walls` stage confirmation +
`evidence/dev174-repair/EVIDENCE.md` (incl. its `results/dev174-*.csv`
same-run walls), and the `perf-split` / `perf-refresh` / `perf-next` /
`perf-perimeters-study` FINDINGS. Cross-vintage figures are shape evidence
only and are never summed (§3.3/§10.3 traps).

**Model.** W = P (serial prepass) + L (parallel per-layer) + Q (postpass) +
R (slice-outside: process startup, model ingest, module load, G-code write —
`run_slice_with_collector`'s `wallclock_ms` (`crates/slicer-runtime/src/run.rs`)
spans validation→gcode-text only, so R = process wall − `slice_complete`
`elapsed_ms`). A cell is won when P/a + (L+Q)/b + R/c < T (Orca wall); corner
factors bound every mix: **a-min = P/T** (all else free), **b-min = (L+Q)/T**,
**c-min = R/T**. CPU must corroborate and PNP's cpu/wall (2.1–7.7) already
meets or exceeds Orca's (2.1–3.3) — there is no parallelism slack to hide
behind; CPU gaps are 6.6–50x.

**Composition anchors.** Benchy, measured (2026-09-22 stage confirmation,
classic supports-off: P 7.893 / L 15.914 / Q 0.474 s instrumented) deflated
onto the uninstrumented 20.83 s median. Base supports-on, measured at the
matched job (`dev174-repair`): pre-repair P 349.5 / L 107.9 / Q 25.1 s
(instrumented) against internal `elapsed_ms` 487.4 s and **same-run** process
wall 538.61 s — self-consistent at instrumentation inflation ≈1.0, with
**R = 51.2 s measured**. Post-repair single run: P 455.7 / L 152.8 / Q 52.2 s,
`elapsed_ms` 669.5 s, process wall 791.71 s (R 122.2 s) — n=1, drift-flagged
(the dev174 evidence itself disclaims whole-slice wall claims). Benchy R is
unmeasured (estimated small). **Base supports-off has no matched-job phase
split at all** — its per-term corners are unmeasured and are ticket 27's first
deliverable.

**Per-cell budgets** (ordinary-mode walls; `G` = required speedup, parenthesized
= best accelerated-mode G; corner factors as anchored above):

| Cell | T | W ord | G (acc best) | CPU gap | a-min (P) | b-min (L+Q) | c-min (R) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic off | 2.91 | 20.83 | 7.16x (7.16x) | 13.7x | ~2.0–2.4x | ~4.1–4.8x | ~0.5–1x (est) |
| benchy arachne off | 3.07 | 20.49 | 6.68x (6.28x) | 7.0x | ~2.0–2.4x | ~3.9–4.5x | ~0.5–1x (est) |
| benchy classic on | 3.74 | 34.92 | 9.33x (9.01x) | 10.7x | ~2.5–3.3x | ~5.5–6.3x | ~0.5–0.8x (est) |
| benchy arachne on | 3.83 | 31.92 | 8.34x (8.21x) | 6.6x | ~2.5–3.1x | ~4.8–5.3x | ~0.5–0.8x (est) |
| base classic off | 18.31 | 251.34 | 13.73x (11.62x) | 50.0x | unmeasured | unmeasured | unmeasured |
| base arachne off | 17.79 | 161.55 | 9.08x (7.63x) | 24.9x | unmeasured | unmeasured | unmeasured |
| base classic on | 22.02 | 577.96 | 26.24x (23.68x) | 37.8x | ~15–16x (→~20x post-repair) | ~5.8–6.2x (→~9x) | 2.3x (measured; 5.5x post-repair) |
| base arachne on | 21.62 | 460.71 | 21.31x (21.08x) | 22.8x | unmeasured (n=1 post-repair rows disagree under drift) | unmeasured | ~2.3x (est) |

**Verdict: the route is bound by the serial terms, and no old candidate reaches
any cell.**

- Even at zero cost for everything tickets 18–25 address (all of L), the serial
  terms alone lose every measured cell: benchy 2.0–3.3x on P, base-on 15–20x on
  P, plus R at 2.3x (5.5x post-repair) on base. Even at zero P, per-layer alone
  loses benchy 3.9–6.3x and base-on 5.8–9.3x. Every measured cell binds on at
  least two terms; the most favorable corner anywhere is 2.0x over budget.
- Base supports-off's split is unmeasured, but its G (9.08–13.73x) and CPU gaps
  (24.9–50x — the worst work-density deficits on the board) are far beyond any
  single-term candidate class.
- **Binding cell: base classic supports-on** — worst corners everywhere, and the
  DEV-174 repair makes it worse: restoring 151 skipped support layers grew the
  single-run internal elapsed 487→670 s and the process wall 538.6→791.7 s
  (n=1, drift-flagged), i.e. the honest gap is ≈ 27–36x classic / ≈ 19–22x
  arachne pending a scoreboard revision. A degraded slice is not Orca's job, so
  the pre-repair numbers cannot be pursued.
- Candidate ceilings: 19/20/21/25 are each ≤~1.15–1.3x on one term of some
  cells (fuel-proportional × the measured 0–16% fuel→wall transfer); acceleration's
  measured ceiling is 1.19x. Against G = 6.28–26.24x (worse post-repair), all
  are below-fold as gap-closers.

**Work volume vs speed.** Output-normalized CPU (CPU-s per output MB): benchy
arachne-off is the clean pair — outputs equal within 0.5% and PNP burns 16.1
vs Orca's 2.31 (**6.98x**); base arachne-off (+9% bytes): 48.5 vs 2.12
(**22.9x**). The work-density deficit triples benchy→base, so superlinear work
is real (suspects: per-vertex query scans over growing boundary complexity;
`compute_xy_footprint`'s exact union (`crates/slicer-core/src/algos/mesh_analysis.rs`)
— 18.1 s on 326,546 facets = 186x benchy's 0.097 s on 14.8x facets;
region×module marshalling; support machinery). Classic cells additionally carry
an output-volume surplus on benchy: 7.44 MB vs PNP arachne 4.71 MB (+58%) and
vs Orca classic 3.89 MB (+91%), while base surplus is only +6–10%. Per-tool
section counts (ticket-11 CSVs' `type_counts_json`) show classic emitting 110
`Gap infill` sections where arachne emits none at ~2.8x fewer wall sections —
suggestive but not byte-attributed (cross-tool counts are not comparable; the
measured G-code was scratch and is not retained) → ticket 28.

**Where acceleration pays.** acc/ord 0.841–1.011 = 1.00–1.19x wall: best on
base supports-off (~15%), ~0–6% on benchy, ~1–10% on base-on (pre-repair).
Against G = 6.28–26.24x it is not a gap-closer — the −35.8% guest-fuel cut lands
only in L and measured fuel→wall transfer is 0–16%. The remaining gap lives in
all three structural terms per the table plus the work-density deficit
everywhere. Ticket 13's adoption decision remains a production-mode question
this budget does not answer; no adoption outcome substitutes for the route.

**Re-ranked route (take order).**

Promoted:

1. **[host:slice closing_ex span contradiction](24-host-slice-closing-span-contradiction.md)** —
   first. Smallest ticket; adjudicates `fold_marks`
   (`crates/slicer-wasm-host/src/profiling.rs`) thread handling, which every
   substage split (22/25/27) inherits; may also promote a real ~22 s serial
   `host:slice` cost into P.
2. **[Tree-planner substage attribution](22-tree-planner-substage-attribution.md)** —
   the binding cells are support cells and the planner is the largest measured
   serial module (93–211 s band on base); its batched-service instrumentation
   prerequisite feeds 27 as well.

New route tickets (surfaced by this budget; created):

3. **Serial host floor** (27) — the un-ticketed deficits that make every benchy
   cell unreachable: the prepass built-ins (`commit_support_analysis_builtin`,
   `compute_xy_footprint`'s union, `commit_overhang_annotation_builtin`,
   `commit_shell_classification_builtin`'s residual incl. `apply_opening`'s
   arc-tolerance-0 anomaly) plus slice-outside wall R (its same-run
   re-derivation across cells is 27's first measurement).
4. **Classic output-volume surplus at matched settings** (28) — work reduction
   at identical intended geometry; parallel-takeable.

Kept as scoped attribution, below the structural work:

5. **[Accelerated residual query attribution](18-accelerated-residual-query-attribution.md)** —
   re-scoped: must also answer the benchy→base superlinearity of per-vertex
   query work (6.98x→22.9x CPU/byte on the output-matched arachne pair).
6. **[infill-linker attribution](21-infill-linker-attribution.md)** — cheap split
   (15.3% of acc-mode total guest fuel; 53.5–75.9 s summed worker on base); its
   single candidate joins the chain late.
7. **[Arachne graph-construction attribution](25-arachne-graph-attribution.md)** —
   the four arachne cells need it eventually, but its ceiling (subset of L)
   cannot reach b-min 3.9–5.3x alone.

Demoted below-fold (below irrelevance as gap-closers today; endgame polish only
if a cell lands within ~1.5x of Orca or the structural tickets come back short):

- **[offset2_ex call reduction](19-offset2-ex-call-reduction.md)** — ceiling
  ≤~1.1–1.15x on classic-cell L (23.7% of acc-mode classic fuel × measured
  0–16% fuel→wall transfer); zero on arachne cells and on P/R. Its "Timing
  chain position 4" is superseded.
- **[Consume-only-when-read annotations](20-consume-only-when-read-annotations.md)** —
  subset of the same ceiling ("position 5" superseded).

Enabler, not route progress:
**[Integrated-parity oracle experiment](23-integrated-parity-oracle.md)** —
cheapens every later measurement session if usable; parallel; moves no cell.

**Timing chain re-wired:** 15 (criterion bench refresh) → 17 (clipper2
cost/output verdict — the chosen version underlies all polygon-op cost,
prepass included) → acceptance slots for 27's then 22's candidate → 21's →
25's → below-fold polish (19, 20). Attribution-only tickets (18, 22, 24, 25,
26, 28) stay parallel-takeable, but no substage split starts before 24 closes
(wired on 22 and 27).
