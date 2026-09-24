# Wayfinder map: Slice performance vs OrcaSlicer

Label: wayfinder:map
Effort: `docs/specs/perf-vs-orca/` (local-markdown tracker; wayfinder maps live
under `docs/specs/` in this repo, per `docs/specs/orca-feature-gap`)
Charted 2026-09-22 from `docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` and the mode-comparative addendum
of the 2026-09-22 session (grilling decisions Q1–Q8).

## Destination

PNP beats OrcaSlicer on matched settings: lower median uninstrumented
wall-clock (process CPU corroborating, never contradicting) in every cell of
the {benchy, base} × {classic, arachne} × {supports off, on} matrix, under the
matched-pair protocol with outputs disclosed. 8 of 8 cells won closes the map.

## Notes

Domain: slice throughput on Windows dev hardware. Canonical evidence base:
`docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md` (measurement recipe and traps) and
`docs/specs/perf-vs-orca/evidence/perf-emit-walls/FINDINGS.md` (current attribution, incl. its addendum).
Every number in this tracker is a ledger fact — re-derive at the point of use.

- **Ledger caveat:** `docs/specs/perf-vs-orca/evidence/PERF-HANDOFF.md`'s DEV ids drift. Its "DEV-166" is
  the `check_split_owner` unbounded recursion — ledger row **DEV-173**; its
  "DEV-167" is the tree-support routing-cell defect — ledger row **DEV-174**.
  Re-derive ids from `docs/DEVIATION_LOG.md`, never from the handoff.
- **Evidence policy:** every file this map cites lives in
  `docs/specs/perf-vs-orca/evidence/` (in-repo). The only gitignored
  references allowed are the heavy model fixtures `tmp/3dbenchy.stl` and
  `tmp/base.stl` — user-supplied and licence-encumbered, they must never be
  committed.

Skills to consult: `diagnosing-bugs` for regressions, `debug-pipeline` and
`visual-debug` per the repo `AGENTS.md`; the repo test discipline (narrow runs,
`cargo xtask test`, guest freshness) governs any code change.

Standing decisions for this effort (2026-09-22):

- **Execution is carried into this map**: tickets are worked as measurement /
  implementation experiment sessions — deliberate override of wayfinder's
  "plan, don't do" default.
- **Verdict metric**: median uninstrumented wall-clock; process CPU must
  corroborate (never contradict). cpu/wall ratio quoted per sample, starved
  samples excluded (§10.3 trap).
- **Fairness contract**: matched settings define the job (nozzle, layer height,
  wall and infill counts, wall generator, supports). Every comparison discloses
  output stats (bytes, TYPE counts, degraded flag); a known work-skipping
  defect disqualifies that cell.
- **Win matrix**: all 8 cells decide, so support-correctness defects are route
  work — a degraded slice is not Orca's job.
- **Measurement mode**: paired ordinary + accelerated runs are the default
  keep/drop gate; the human may accept a one-sided win case by case.
- **Timing discipline**: timing/acceptance tickets chain in the take order set
  by [Gap budget per cell](issues/12-gap-budget-per-cell.md) (number order
  superseded 2026-09-23; chain: 15 → 17 → 27's candidate → 22's candidate →
  21's → 25's → below-fold polish); attribution-only tickets are
  parallel-takeable, but no substage split starts before
  [host:slice closing_ex span contradiction](issues/24-host-slice-closing-span-contradiction.md)
  closes. Claim (`Status: claimed`) before any work.
- **Recipe traps** (each has produced or nearly produced a wrong conclusion):
  instrumented runs never mixed into wall claims (§3.2); accumulated worker
  elapsed is not CPU (§3.3); the native `--profile` table's wall columns are
  accumulated per-thread spans (`flush_native` sums concurrent worker spans),
  never wall — do not compare them to `module_complete`/phase walls
  ([host:slice closing_ex span contradiction](issues/24-host-slice-closing-span-contradiction.md),
  2026-09-23); `--module-dir modules/core-modules` is mandatory
  (§3.4); `cargo xtask build-guests --check` must exit 0 first (§3.5); base.stl
  wall varies ±13% so take repeats (§3.6); accelerated runs use the complete
  `cargo xtask dist --accelerated` snapshot at `target/dist-accelerated/developer/`
  — the bare artifacts dir silently loads integrated modules, and dual
  `--module-dir` does not shadow (2026-09-22 recipe addendum); criterion's
  console `time:` is the regression *slope* in Linear sampling mode while its
  `change:` line is mean-based — read `target/criterion/**/estimates.json` (or
  `evidence/t15-criterion-refresh/baselines.json`) and compare like-for-like
  ([Criterion bench refresh](issues/15-criterion-bench-refresh.md), 2026-09-23);
  a PNP single-slice subprocess pays ~0.77 s of module compile *before* the
  `validation` phase event, so phase walls sum to ~10% of
  `slice_complete.elapsed_ms` — never read phase-sum as run wall
  ([Criterion bench refresh](issues/15-criterion-bench-refresh.md), 2026-09-23).
- Measured keep/drop recommendations return to the human; candidates are never
  auto-committed.

## Decisions so far

<!-- one line per closed ticket; the ticket holds the detail -->

- [Baseline and measurement recipe](issues/01-baseline-and-measurement-recipe.md): the three-fixture PNP-vs-Orca baseline is captured but configuration-unmatched (no speed multiple is defensible from it), and the six measurement traps plus reproduction commands are the standing protocol.
- [shell_classification guard ordering](issues/02-shell-classification-guard-ordering.md): the stage's cost was two full-layer `offset` calls computed before the early-outs that discard them; guards hoisted, erosion made lazy, bridge gate parallelized — benchy stage 17.8 s → 2.96 s, landed.
- [Allocator A/B closes the contention hypothesis](issues/03-allocator-ab-contention-hypothesis.md): mimalloc/snmalloc show no wall win and +5–23% peak memory; the inflated-CPU reading was external machine load in summed worker elapsed. Do not repeat without new contention evidence.
- [Profile-first refresh and dispatch split](issues/04-profile-refresh-and-dispatch-split.md): prepass dominates and tree-support-planner is the largest serial module; the user chose the perimeter/dispatch split, which attributed ~97% of clustered per-layer dispatch cost to host-side marshalling.
- [Prepared-region arena cache](issues/05-prepared-region-arena-cache.md): host region preparation memoized in `LayerArena` (landed `bf11d055`) — ~66% of preparation calls served from cache, process CPU −3.8% benchy / −5.8% base, guest work untouched. KEEP.
- [Perimeter attribution study](issues/06-perimeter-attribution-study.md): classic fuel 85.8% wall assembly / 8.0% gap fill; Arachne interval 75.3% preprocess/graph construction — attribution only, and pre-ticket-07 (re-attribute before trusting at HEAD).
- [Wall-flags annotation-free fast path](issues/07-wall-flags-annotation-fast-path.md): accepted and committed on repeatable process-CPU savings (wall benefit never proven; acceptance revised for this candidate only); follow-up audit also caught and repaired the quiet harness's Classic-labeled-Arachne bug.
- [emit_walls premise falsified and classic re-attribution](issues/08-emit-walls-premise-falsified.md): the inset clone/store is 0.0014% of module fuel (lead closed); classic's real cost is the per-vertex spatial queries (69.8% of guest fuel, ordinary mode) with `offset2_ex` at 15.2% — leads re-ranked.
- [Accelerated-mode pair](issues/09-accelerated-mode-pair.md): acceleration cuts total guest fuel −35.8% but the hot queries only ~1.9× (necessary, likely not sufficient); ranking rescales (classic self 60.4%, `offset2_ex` 23.7%, infill-linker 15.3%), and the accelerated-snapshot recipe traps are now recorded.
- [Unreachable batch-query dead end](issues/10-unreachable-batch-queries-dead-end.md): spatial-indexing `raycast_z_down_batch` / `surface_normal_at_batch` would optimize an unreached path (no production callers). Do not pursue.
- [clipper2 1.1.0 upstream evidence](issues/16-clipper2-1-1-0-upstream-evidence.md): 1.1.0 is purely additive (PolyFace64 face extraction) — polygon-op cost and output for identical inputs unchanged, and `check_split_owner`'s unbounded recursion is byte-identical with unchanged reach in both versions.
- [Matched-pair rig and first scoreboard](issues/11-matched-pair-rig-and-scoreboard.md): the matched job (0.4/0.20 mm, 2 walls, 20% gyroid, tree(auto) supports, per-cell generator) is rigged with per-run output-evidence validation and measured on all 8 cells — Orca wins every cell (median wall 6.3–26.2x, process CPU corroborating), accelerated mode buys 0–16% wall, and base supports-on is tainted by DEV-174 (`degraded=true`, 172,181 non-fatals); scoreboard in `evidence/matched-pair/SCOREBOARD.md`.
- [Support-correctness repair](issues/14-support-correctness-repair.md): the DEV-174 dropped-band defect is fixed and kept — the complete-body extent gate measured identity aggregates as one body against the deleted routing cell's 104.86 mm cap; now measured per body cross-section against a 419.43 mm plate bound (ADR-0059 Ruling 3 / D-287). base.stl supports-on runs clean (`degraded=false`, `non_fatal=0`) in both generators and both PNP modes, band layers 108–258 restored (151→0 zero-Support layers), so the four tainted cells are untainted for future scoreboard revisions; evidence in `evidence/dev174-repair/`.
- [Gap budget per cell](issues/12-gap-budget-per-cell.md): the serial terms bind every cell — with per-layer work free the prepass floor alone still loses benchy ~2–2.5x and base supports-on ~15–20x, a measured slice-outside wall costs 2.3x (5.5x post-repair) of Orca's budget on base, and the CPU work-density deficit is 6.98x→22.9x per output byte — so no old candidate closes a 6.28–26.24x gap. Route re-ranked: 24 → 22 → new [Serial host floor](issues/27-serial-host-prepass-floor.md) and [Classic output-volume surplus](issues/28-classic-output-volume-surplus.md), tickets 19/20 demoted below-fold, acceleration confirmed at 1.00–1.19x wall.
- [host:slice closing_ex span contradiction](issues/24-host-slice-closing-span-contradiction.md): the contradiction is a units mismatch — `module_complete`'s ~3.2 s is real wall while the profile table's ~22 s `closing_ex` spans are accumulated per-thread spans (`fold_marks` is thread-correct; the aggregation sums concurrent worker spans) — so the lead retires with no ~22 s cost to promote, and the 22/27 substage splits inherit "work-share yes, wall claims no"; same-run capture in `evidence/t24-span-contradiction/SAME-RUN.md`.
- [Criterion bench refresh](issues/15-criterion-bench-refresh.md): all 7 benches now have on-disk baselines (82 leaves, `base == new`) and are trustworthy — `gate_evidence` 885.9 ms vs the 10 s bound, `shell_classification` ms-scale with no short-circuit; two fixture limits recorded not fixed (`repair/cube` scans a clean 12-tri mesh, `decimate/cube_default` is rejected by the default `max_error = 0.01`); and the criterion console's `time:` is the regression *slope* in Linear mode, not the mean (up to +8.61% off) — cost A/Bs must compare like-for-like. Evidence in `evidence/t15-criterion-refresh/`, unblocking [clipper2 cost/output verdict](issues/17-clipper2-cost-output-verdict.md).

## Not yet specified

- **Painted-wall performance work.** No representative painted workload exists,
  so the painted path of `build_wall_flags` (nearest-original reprojection
  reuse) is not even selectable as a candidate. Needs a fixture/scenario before
  it can graduate into a ticket.
- **New candidates from the structural splits.** [Gap budget per cell](issues/12-gap-budget-per-cell.md)'s
  route tickets (24/22/27/28 and the re-scoped 18) may surface further
  candidates inside the serial floor, the query path, or the marshalling
  residual; graduate each as it appears. Below-fold polish (`offset2_ex` call
  reduction, consume-only-when-read, `split_top_surfaces` secondary work, the
  `emit_walls` memory-shape idea) stays parked — the park decision is in
  [Gap budget per cell](issues/12-gap-budget-per-cell.md) — until a cell lands
  within ~1.5x of Orca or the structural tickets come back short.
- **Orca-side attribution.** What OrcaSlicer spends its time on per cell is
  unknown (GUI-subsystem binary, little introspection). Worth scoping only if
  the gap narrows to specific stages where "why is Orca faster here" becomes the
  blocking question.
- **Occupancy gate still measures the identity aggregate.**
  [Support-correctness repair](issues/14-support-correctness-repair.md)
  un-widened only the extent bound; `validate_entry`'s exact-Z occupancy check
  still drops a whole identity entry when any one body cross-section overlaps
  model occupancy — the same class of aggregate mis-measurement, latent (zero
  occupancy rejections in the measured failure data). Becomes a ticket the day
  it fires on a matched cell.

## Out of scope

- **Memory instrumentation / peak-RSS bounds (DEV-026).** The destination's
  verdict is speed (wall, CPU corroborating); memory is not judged
  (2026-09-22 scoping decision). Returns only if the destination is redrawn.
- **OrcaSlicer geometry-parity work.** The repo's parity program runs as spec
  packets; this map treats output as disclosed evidence only (the fairness
  contract), not as a parity target.
