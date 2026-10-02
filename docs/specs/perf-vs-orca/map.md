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
- **Evidence policy:** every claim's headline evidence lives in
  `docs/specs/perf-vs-orca/evidence/` (in-repo) and must be re-derivable from a
  plain checkout. A gitignored raw capture tree may be cited only as the
  fuller, optional source that the in-repo reduced set is derived from (and
  the reducer script must live in-repo); verification scripts must exit 0
  against the in-repo set without the raw tree. The only other gitignored
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
  superseded 2026-09-23; chain: 15 ✅ → 17 ✅ → 27's candidate → **22 ✅
  (attribution; produced the emit-pass carve candidate)** → **[emit-pass carve
  gate acceptance](issues/32-emit-carve-gate-representation-and-ab.md) ✅**
  → **21 ✅ (attribution, 2026-10-01; produced the
  → **[clip-universe hoist](issues/44-clip-universe-hoist.md) candidate, subsequently
  resolved as KEEP approved under the human-approved fresh-baseline gate)** → **25 ✅ (attribution, 2026-10-01; produced the
  [overhang-quartile predicate narrowing](issues/45-overhang-quartile-predicate-narrowing.md)
  candidate awaiting its own take)** → below-fold
  polish — 15, 17, 22
  closed 2026-09-23/24); attribution-only tickets are parallel-takeable, but no
  substage split starts before [host:slice closing_ex span
  contradiction](issues/24-host-slice-closing-span-contradiction.md) closes.
   Claim (`Status: claimed`) before any work.
   2026-09-29 route update: the human parked 27's candidate search after
   accepting the sibling-region memo DROP; attribution-only 28 is resolved.
   Its new [Benchy classic sparse-fill domain at middle layers](issues/35-benchy-classic-sparse-fill-domain.md)
   and [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md)
   are distinct frontier investigations, not already-accepted optimizations.
   2026-09-29 second update: 35 resolved as a **containment defect, not a fill
   domain** — the linker's clipped-to-nothing verdict is overridden by the
   empty-output preservation rule, printing 68% of classic Benchy's sparse
   output outside the part. Its fix is graduated to [InfillPostProcess
   empty-output protocol](issues/37-infillpostprocess-empty-output-protocol.md),
   which touches the linker/host contract rather than the fill geometry and
   needs the standing paired A/B before any keep. 36 stays open and parallel.
   2026-09-29 third update: **37 resolved and fixed** — a ran
   `Layer::InfillPostProcess` invocation now commits the empty replacement set,
   so the linker's clip verdict supersedes the raw envelope. Stage-local (only
   this stage's contract is a complete replacement set); `Layer::Infill`,
   `Layer::Support`, `Layer::SupportPostProcess` and `Layer::AnchoredEvents`
   keep their existing empty-output semantics. Verified on real G-code (60
   classic layers → zero sparse, worst overshoot 21.78 mm → 0.00 mm; base.stl
   byte-identical as the negative control) and paired ordinary + accelerated
   A/B measured **no wall win with corroborating CPU** — the case is
   containment, not speed.
   Recommended KEEP to the human; not auto-committed.
- **Recipe trap (new 2026-09-29):** a `Layer::InfillPostProcess` (or any stage)
   invocation that emits no paths is indistinguishable from a stage that did not
   run — the host preserves the prior `InfillIR`. Never read "the linker produced
   nothing" as "no linking was needed"; check the linker's own kept-length from
   `evidence/t35-fill-domain/probe.patch` before attributing output to the fill
   generator.
- **Recipe trap (narrowed 2026-09-29 by 37):** the trap above applies to
   *instrumented analysis of the pre-fix tree*. From the fix onward a ran
   `Layer::InfillPostProcess` that re-emits nothing commits the empty
   replacement set, so "no sparse output on this layer" is a real verdict on
   both engines. The distinction still matters for `Layer::Infill` (merge
   stage, unchanged) and for genuinely absent stages. See
   [InfillPostProcess empty-output protocol](issues/37-infillpostprocess-empty-output-protocol.md).
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
  ([Criterion bench refresh](issues/15-criterion-bench-refresh.md), 2026-09-23);
  a scratch harness in its own cargo workspace inherits only the *caret*
  requirement, so its lock floats to the newest compatible release and a
  version A/B silently measures the wrong version — pin the harness dep and
  assert the resolved version from the harness's own `Cargo.lock`
  ([clipper2 cost/output verdict](issues/17-clipper2-cost-output-verdict.md),
  2026-09-24); host-side stage walls differed materially between base captures
  with the same host probe but different guest bracket sets while call counts
  and class shares held (two ticket-22 captures: `clip` 146.8 s vs 194.0 s,
  in-guest carve counts byte-identical) — quote shares/counts as the stable
  signal and disclose wall drift rather than averaging it away; external load
  is plausible but its cause was not isolated
  ([Tree-planner substage
  attribution](issues/22-tree-planner-substage-attribution.md), 2026-09-24).
- Measured keep/drop recommendations return to the human; candidates are never
  auto-committed.
- **Production-mode route (2026-09-30):** pursue conditional accelerated
  adoption; ordinary stays the actual default pending the existing acceptance
  gate. See [Accelerated perimeter-spatial adoption decision](issues/13-accelerated-adoption-decision.md)
   and its [acceptance campaign](issues/38-accelerated-adoption-acceptance-campaign.md).
  **Adoption preflight (2026-09-30):** both mode-specific guest freshness checks
  passed, but the real acceptance CSV reader loses non-fatal counts and the
  existing references predate correctness repairs. Campaign timing has not
  started; its prerequisite blocking edges carry the new frontier. Evidence:
   [Accelerated adoption campaign preflight](evidence/t38-adoption-preflight/PREFLIGHT.md).
   **Runner prerequisite repaired:** see
   [Acceptance runner CSV status preservation](issues/39-acceptance-runner-status-roundtrip.md).
   **References now prepared/frozen:** see
   [Current-job adoption reference preparation policy](issues/40-acceptance-reference-refresh-policy.md).
   The historical corpus remains intact; use the frozen manifest's new corpus
   root explicitly after its identity gate. No campaign timing has started;
    full adoption gates remain pending.
    **Resumption blocker (2026-09-30):** the manifest-named frozen ordinary
    snapshot and current-job corpus roots are now absent; the read-only identity
    gate failed before comparison or timing. Historical corpus, manifest and
    build-input hashes still match. The campaign is blocked on
    [Missing frozen adoption inputs recovery decision](issues/41-missing-frozen-adoption-inputs-recovery.md);
    do not silently recreate the missing payloads. Evidence:
    [Frozen-input difference audit](evidence/t38-adoption-campaign/freeze-difference.json).
    **Recovery authorized:** the human chose a new, separate ordinary-only
    preparation with the unchanged six-cell job and checks. Durable payloads
    will live under `.local-artifacts/perimeter-reference-preparation/`, outside
    `target/`; previous evidence and the historical corpus remain preserved.
    **Replacement technically prepared/frozen:** all six ordinary references
    passed the unchanged checks and main-session identity verification outside
    `target/`. Evidence:
    [Durable preparation findings](evidence/t41-reference-repreparation/FINDINGS.md).
    **Recovery accepted:** the human accepted the validated durable set and
    the recorded preparation-only preflight restart exception, then requested
    continuation. The
    [acceptance campaign](issues/38-accelerated-adoption-acceptance-campaign.md)
     resumes under the unchanged gates; no automatic retry, commit or default
     switch is authorized.
     **Campaign completed (2026-10-01): inconclusive.** Classic supports-off
     Benchy passed; Arachne wall ranges overlapped and the prescribed stop left
     four cells unrun. No retry or default switch occurred. The human route is
      [Accelerated adoption route after inconclusive campaign](issues/42-accelerated-adoption-inconclusive-route.md).
      **Investigation selected:** saved Arachne evidence localizes the finishing
       tail to layers 1 and 2. The subsequently authorized two-slice diagnostic
       pair identified infill-linker dispatch as the owner, not Arachne perimeter
       dispatch. No acceptance retry or fix ran. Next lead: low-layer linker
       subcost attribution, requiring separate authorization. See
       [Arachne critical-tail module attribution](issues/43-arachne-critical-tail-module-attribution.md).
      **Next lead taken and answered (2026-10-01):** the human authorized
       [infill-linker attribution](issues/21-infill-linker-attribution.md) as
       the next map step. Result: the tail is ~99.95% guest execution, and the
       linker's cost is 97.65% the **per-path re-clip** — 64.84% of all linker
       fuel is the repeated clip-universe pre-inflate, 32.65% the clipper
       execute; connectivity is 2.01%. The split is numerically mode-invariant
       (linker is the #1 accelerated fuel consumer at 59.1%). One candidate:
       hoist the clip-universe preparation out of the per-path loop (fuel
       ceiling ≈ 61.4% of linker ≈ 28.1% of slice fuel as a uniform-count
       estimate; 63.7% of linker layer-aware; both estimates under a same-cost
       assumption), needing its own scope plus the standing paired A/B. No
       fix, commit or acceptance retry occurred; ordinary remains production
       and the adoption gate stays inconclusive. Evidence:
       [linker subcost findings](evidence/t21-linker-subcost/FINDINGS.md).
      **Next chain link taken and answered (2026-10-01):**
       [Arachne graph-construction attribution](issues/25-arachne-graph-attribution.md)
       resolved with an attribution that **reverses the old study's focus
       boundary**: the module's cost is 86.4% two guest per-vertex queries
       (`signed_distance_to_boundary` 48.3%, `overhang_quartile` 38.1%), while
       the whole host pipeline is ~18% of module elapsed; the host internals
       still split into the old two leads (triple offset 41.6%, boostvoronoi
       sweep 33.5%) and the 364.932 ms remainder is closed. One candidate (the
       quartile predicate's `eps = 0.0` boundary pre-pass, fuel ceiling ≈
       21–27% of module fuel as a cross-domain bound) awaits its own scope plus
       the paired A/B. No fix, commit or acceptance retry; ordinary remains
       production. Evidence:
       [Arachne re-attribution findings](evidence/t25-arachne-attribution/FINDINGS.md).

## Decisions so far

<!-- one line per closed ticket; the ticket holds the detail -->

- [Clip-universe preparation hoist](issues/44-clip-universe-hoist.md): **KEEP approved (2026-10-02)** — immutable per-invocation preparation preserves the tested fresh-baseline output and passes the standing paired ordinary + accelerated wall/CPU gate; no adoption retry or default-mode switch, with pre-existing frozen-job drift disclosed separately in [Frozen-job output drift boundary](issues/46-frozen-job-output-drift-boundary.md).

- [Arachne graph-construction attribution](issues/25-arachne-graph-attribution.md): **the module's cost is two guest per-vertex spatial queries — 86.4% of module fuel (`signed_distance_to_boundary` 48.3%, `overhang_quartile` 38.1%) — not the host pipeline, which is only ~18% of module elapsed** (the 75.3% ticket 06 measured was of the enclosing pipeline interval, not the module); inside the host service preprocess is 43.3% (stage-1 triple offset 41.6%) and graph construction 38.0% (boostvoronoi sweep 33.5%), closing the old 364.932 ms remainder; acceleration cuts the module 1.73x but the two queries still hold 82.0% accelerated; one candidate — the `overhang_quartile` predicate's `eps = 0.0` boundary pre-pass measures 56.5–71.2% of the predicate across four preserved synthetic-ring runs, fuel ceiling ≈ 21–27% of module fuel as a cross-domain bound — awaits its own scope + paired A/B, with no fix, commit or acceptance retry.

- [infill-linker attribution](issues/21-infill-linker-attribution.md): the linker's cost is 97.65% the per-path re-clip — 64.84% of linker fuel is the per-call clip-universe pre-inflate, 32.65% the clipper execute — and the layers 1–2 tail is ~99.95% guest execution (host prep/marshalling ~1.7 ms); one candidate (hoist the universe preparation per invocation, fuel ceiling ≈ 61.4% of linker ≈ 28.1% of slice by the uniform-count estimate, 63.7% of linker layer-aware, both estimates under a same-cost assumption) awaits its own scope + paired A/B, with the split mode-invariant and the linker the #1 accelerated fuel consumer at 59.1%.

- [Arachne critical-tail module attribution](issues/43-arachne-critical-tail-module-attribution.md): authorized diagnostic pair completed with clean byte-identical output and frozen inputs; infill-linker dispatch occupies 90.01–96.69% of layers 1/2 (its host-vs-guest subcosts were subsequently split by [infill-linker attribution](issues/21-infill-linker-attribution.md): ~99.95% guest); no acceptance retry or production change.

- [Accelerated adoption route after inconclusive campaign](issues/42-accelerated-adoption-inconclusive-route.md): human chose saved-evidence-first Arachne investigation, then authorized the diagnostic pair; ordinary remains production pending unchanged acceptance gates.

- [Accelerated production adoption acceptance campaign](issues/38-accelerated-adoption-acceptance-campaign.md): controlled snapshots and one real campaign completed; adoption remains inconclusive after Arachne wall overlap, with four cells unrun; route returned to the human, ordinary unchanged.

- [Missing frozen adoption inputs recovery decision](issues/41-missing-frozen-adoption-inputs-recovery.md): human accepted the new validated ordinary-only frozen set outside `target/` and the recorded preparation-only preflight restart exception; campaign continuation authorized under unchanged gates.

- [Current-job adoption reference preparation policy](issues/40-acceptance-reference-refresh-policy.md): human-authorized ordinary-only references prepared and frozen for the unchanged six-cell job, with clean-output and targeted repair checks; historical corpus preserved, full adoption acceptance still pending.

- [Acceptance runner CSV status preservation](issues/39-acceptance-runner-status-roundtrip.md): real CSV non-fatal counts and validated generator markers now survive the runner gate; regression checks pass without relaxing adoption gates.

- [Accelerated perimeter-spatial adoption decision](issues/13-accelerated-adoption-decision.md): human confirmed conditional accelerated adoption under the existing exactness and strict CPU/wall campaign gate; ordinary remains the default pending acceptance.

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
- [clipper2 cost/output verdict](issues/17-clipper2-cost-output-verdict.md): **stay on 1.1.0** — the 1.0.3→1.1.0 bump is measured cost-neutral and output-identical (367 fixtures through the full entry-point surface, incl. 24 real benchy layers; `diff -rq` exit 0, tree SHA equal, both sides self-reproducing), with compiled bodies 230/240 bit-identical as the load-independent backstop; the geometry modules are byte-identical and `check_split_owner`'s reach is unchanged, so DEV-173's posture is unaffected; evidence in `evidence/t17-clipper2-ab/`. The harness trap (a scratch workspace's lock floats the caret req to the newest release — it silently measured 1.2.0 while labelled 1.1.0) is recorded for future dependency A/Bs, and it surfaced a real onward finding: **1.1.0→1.2.0 is not output-neutral** (2/367 fixtures lose one collinear vertex; 1.2.0's `clean_collinear` changed) — a future 1.2.0 bump needs its own geometry A/B.
- [Tree-planner substage attribution](issues/22-tree-planner-substage-attribution.md): ADR-0049's 98%-collision-cache lead **does not survive to the matched job** — the emit pass is 97.2% base / 92.6% benchy of the per-object planner, with per-region carve the largest sub-term (46.8% of base emit wall), followed by union+simplify (27.8%) and model-occupancy inflate (23.4%); the ladders are 4.27 s of 269.6 s (1.6%). Batched host offsets run (50 batches on base), but batched clips/simplify do not. The carve alone makes 147,993 clips on base; the whole stage makes 171,636 / 194.0 s of core boolean calls. A bbox gate could avoid 78,832 base / 11,018 benchy disjoint *carve* calls as set operations, but its net wall and output equivalence are unmeasured; the probe showed one disjoint rectangle's contour differs from a verbatim input. Evidence in `evidence/t22-planner-substage/`.
- [Emit-pass carve gate: representation safety + paired A/B](issues/32-emit-carve-gate-representation-and-ab.md): the naive bbox skip changes representation and a reachable lone-disc simplification result; guarded winding and lone-region repair preserve the tested production chain exactly. Independently measured ordinary + accelerated pairs favor the candidate on process CPU in every completed pair and on paired median wall in every cell, but wall is load-qualified (cpu/wall below quiet reference). **Human accepted KEEP; committed with this ticket.** Evidence in `evidence/t32-carve-gate/`.
- [Accelerated residual query attribution](issues/18-accelerated-residual-query-attribution.md): the hot queries' ~1.9× residual is **the exact winding predicate loop after candidate reduction** — `inside_scan`+`quartile_scan` are ~89% of base's accelerated query fuel (collect is 24–55× cheaper), same shape under Arachne (79–81% of module) — while the u128-envelope suspect is falsified (f64 envelopes; the u128 bridge precheck is ~0.03% with its unsafe fallback never firing) and Linear fallbacks fire often but cost 0.07–1.96%; per-query scan fuel grows 3.5× benchy→base while collect stays flat, locating the superlinearity in the scan. Next candidates (2-D per-polygon bbox records, pre-converted vertex cache + short-circuits, descending quartile first-hit) stay below-fold — fuel→wall transfer 0–16%, no gap-closer. Evidence in `evidence/t18-accelerated-residual/`.
- [Integrated-parity oracle experiment](issues/23-integrated-parity-oracle.md): native region eligibility and per-region config precedence were repaired and narrowly tested, but integrated timings remain disqualified as a module-work oracle pending the [Integrated/external matched-output oracle gate](issues/33-integrated-external-matched-output-oracle-gate.md) against real full-slice outputs.
- [Config appendix must reflect resolved settings](issues/26-config-appendix-resolved-settings.md): one-to-one Orca-named appendix aliases now disclose PNP's effective wall count, infill direction, and wall-path precision instead of static padding; sparse-density padding now matches its documented default. Remaining padding-only disclosure semantics are the separate [Config padding disclosure boundary and viewer-key gate](issues/34-config-padding-disclosure-boundary.md).
- [Serial host floor](issues/27-serial-host-prepass-floor.md) — **work item 1** (2026-09-27; items 2–5 still open): slice-outside wall R measured same-run is **0.15–0.23x** of Orca's budget on the worst cell (benchy ≈ 0.2 s, base 1.6–3.4 s) — ticket 12's 2.3x/5.5x R came from subtracting instrumented phase sums from uninstrumented process wall (§3.2), so every c-min in its budgets table is 10–35x too high and R retires as a budget term; the elapsed tail is attributed (pre-validation ~1.1–1.5 s incl. the ~0.77 s module-compile fixed term; prepass→per_layer handoff ~0.75 s; post-postpass = 32–38 s of pre-repair DEV-174 diagnostic replays — an artifact, not route work — plus `estimate_print`+scan at 6.8 s accelerated vs 16.9 s ordinary on identical 54.5 MB output, ordinary/accelerated delta unexplained, probe next). Evidence in `evidence/t27-serial-floor/`.
- [Serial host floor](issues/27-serial-host-prepass-floor.md) — **work items 2–5** (2026-09-27, `PERF-T27-PROBE`, 4 uninstrumented runs, guests fresh): all four prepass built-ins attributed at the matched job and **`PrePass::Slice` (47–56 s wall on base, #2 serial block) adopted into the ticket**; ShellClassification's serial gate-commit loop (25–30 s) is the largest confirmed serial block outside Slice; the overhang footprint exact union (17–18 s serial) has an exact-intersection support-eligibility consumer (bbox replacement unsafe; premise audit below); SupportAnalysis's serial sweep is 24.7 s and its detect pass is parallel-saturated (11.7x); OverhangAnnotation's interior is parallel (needs a wall bracket before candidates graduate). Probe code stays out of commits. Evidence in `evidence/t27-serial-floor/FINDINGS-SUBSTAGE.md`.
- [Serial host floor](issues/27-serial-host-prepass-floor.md) — **probe iteration 2 and premise audit** (2026-09-28): phase-B's two Miter offsets are measured work, but the proposed sibling-region memo has zero eligible siblings in the matched single-region jobs and its key does not prove identical input; drop that candidate, not the cost. `PrePass::Slice` fuel is ~70% `assemble_flat_bridge_areas` (parallel-saturated), but a safe reduction needs representation analysis. **The 6.8-vs-16.9 s tail mystery resolves to the pipeline-boundary teardown** (`estimate_print` itself is ~0.1 s; teardown 7.5–17 s vintage-dependent, below-fold); OverhangAnnotation wall-bracketed at 12.2–14.2 s (11–16x parallel, below-fold). Evidence in `evidence/t27-serial-floor/FINDINGS-SUBSTAGE-2.md` and `evidence/t27-serial-floor/PREMISE-AUDIT.md`.
- [Serial host floor](issues/27-serial-host-prepass-floor.md) — **human route decision** (2026-09-29): accepted DROP of the phase-B sibling-region memo for the matched jobs and parked further serial-floor candidate search until an output-safe, measurably useful reduction has a real-layer IR oracle. The failed flat-bridge mask elimination remains dropped; `apply_opening` parity is separate. Prioritize the already-open [Classic output-volume surplus](issues/28-classic-output-volume-surplus.md) attribution next. Serial host floor stays open and unclaimed; no candidate committed in this take.
- [Classic output-volume surplus](issues/28-classic-output-volume-surplus.md): fresh ordinary matched output census attributes 90.6% of Benchy's classic-vs-Arachne byte excess to actual sparse XY paths, not gap fill; base's classic excess is mostly wall bytes and sparse is smaller than Arachne. Historical Orca captures and a middle Benchy layer expose a fill-domain discrepancy, not a proven safe deletion. PNP also repeats Z/F modal tokens on every XY+E move; its speed impact is unmeasured. Follow-ups: [Benchy classic sparse-fill domain at middle layers](issues/35-benchy-classic-sparse-fill-domain.md) and [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md). Evidence: `evidence/t28-output-volume/FINDINGS.md`.
- [Benchy classic sparse-fill domain at middle layers](issues/35-benchy-classic-sparse-fill-domain.md): **not a fill-domain question — a commit-protocol containment hole.** The linker clips the raw gyroid waves to nothing on the burst layers (probe: 1,741 mm in, 0 paths kept at classic L104), the host reads that empty output as "committed nothing", and the preserved prior `InfillIR` prints the raw, unclipped envelope — 54,324 of 79,940 printed sparse mm (68.0%) and 3,054,585 of 4,486,477 sparse bytes across 60 classic layers, with 88.7% of the L104 revived path outside the part cross-section and 1,741 mm printed against a 44.2 mm absolute bound from the 17.7 mm² sparse claim. Arachne 16 layers / 20.9%; base has zero all-empty layers and classic prints fewer sparse mm than Arachne there, which explains the ticket-28 fixture divergence. No fix made — the protocol change is graduated to [InfillPostProcess empty-output protocol](issues/37-infillpostprocess-empty-output-protocol.md). Evidence: `evidence/t35-fill-domain/FINDINGS.md`.
- [InfillPostProcess empty-output protocol](issues/37-infillpostprocess-empty-output-protocol.md): **fixed — a ran `Layer::InfillPostProcess` that re-emits nothing commits the empty replacement set, so the linker's clip verdict supersedes the raw envelope.** The fix is stage-local (only this stage's contract is replace-with-complete-re-emit; `Layer::Infill` keeps merge semantics and `Layer::SupportPostProcess` stays additive) and lands at the producer on both legs (`deconstruct_layer_ctx`, `commit_native_layer_response`) plus the test mirror, all sharing `empty_infill_replacement`. `RoleBoundaries::for_role`'s `Some(empty)`-vs-`None` containment distinction is untouched (`modules/` diff empty). Real-G-code verification: the 60 classic burst layers go to zero sparse output and every other layer is byte-identical (classic 106,442 → 34,307 segments, 79,939.9 → 25,616.2 mm; Arachne 47,659 → 37,606), worst overshoot past the layer's own wall bbox 21.78 mm → 0.00 mm, and base.stl is byte-identical as the negative control (225,004 segments both sides, 0 differing layers). Paired ordinary + accelerated A/B (6 repeats/arm/cell, quiet machine, no starvation exclusions) measures **no wall win with corroborating CPU** in any of four benchy supports-off batches: paired median wall −0.17/−0.13/−0.03/−0.01 s, and the one directional cell (classic-off ordinary, 6/6) has flat CPU (+0.04 s, 3/6) — the case is containment, not speed, so this is a correctness keep, not a route-performance step. Contract recorded in ADR-0028 §Amendment 2026-09-29, `docs/02_ir_schemas.md` IR 8, the `run_infill_postprocess` trait doc, and DEV-196. **KEEP recommended to the human; not auto-committed.** Evidence: `evidence/t37-empty-commit-protocol/`.

## Not yet specified

- **Painted-wall performance work.** No representative painted workload exists,
  so the painted path of `build_wall_flags` (nearest-original reprojection
  reuse) is not even selectable as a candidate. Needs a fixture/scenario before
  it can graduate into a ticket.
- **New candidates from the structural splits.** [Gap budget per cell](issues/12-gap-budget-per-cell.md)'s
  route tickets (24/22/27/28 and the re-scoped 18) may surface further
  candidates inside the serial floor, the query path, or the marshalling
  residual; graduate each as it appears. [Arachne graph-construction
  attribution](issues/25-arachne-graph-attribution.md)'s split (2026-10-01)
  re-derived the module's shape: the two per-vertex queries are 86.4% of guest
  fuel and its candidate graduated the now-open
  [overhang-quartile predicate narrowing](issues/45-overhang-quartile-predicate-narrowing.md)
  (fuel ceiling ≈ 21–27% of module fuel as a cross-domain bound), which needs
  its own scope plus the standing paired A/B; the paired
  `signed_distance_to_boundary` term (48.3% of
  guest fuel) is the larger but not-yet-candidate-shaped half and stays in this
  fog. [infill-linker
  attribution](issues/21-infill-linker-attribution.md)'s split graduated the
  now-resolved [clip-universe preparation hoist](issues/44-clip-universe-hoist.md)
  (2026-10-01); its measured mode-invariance also re-ranks the *remaining*
  linker work below the hoist — the per-polyline clipper execute (32.65% of
  linker fuel) and the ~2.4% that is not the re-clip (connectivity 2.01% plus
  a <0.4% residual) stay in this fog until a take has a reason to touch that
  shape. [Tree-planner substage
  attribution](issues/22-tree-planner-substage-attribution.md) graduated the
   now-resolved [Emit-pass carve gate: representation safety + paired
  A/B](issues/32-emit-carve-gate-representation-and-ab.md). Its sibling
  sub-term (`union_expolys` + simplify, 72.8 s base) remains in this fog:
  investigate whether a representation-safe call reduction exists before
  graduating a concrete candidate. [Serial host
  floor](issues/27-serial-host-prepass-floor.md)'s probe batches (items 1–5 +
  iteration 2) surfaced three graduate-when-probed questions resolved on
  2026-09-28 (phase-B = two costly offsets, sibling memo dropped for the matched job by human decision on 2026-09-29;
  estimate_print tail = pipeline teardown, below-fold; OverhangAnnotation
  wall = parallel) and one still-open (flat-bridge reduction inside
   `PrePass::Slice`, representation analysis first; the tested `refilled`-mask
   elimination failed real-layer IR equivalence and was dropped — see
   `evidence/t27-serial-floor/FLAT-BRIDGE-EXPERIMENT.md`). The smaller
   `apply_opening` lead is a separate geometry-parity question, not a
   representation-preserving replacement. Below-fold polish (`offset2_ex` call
  reduction, consume-only-when-read, `split_top_surfaces` secondary work, the
  `emit_walls` memory-shape idea, and the ticket-18 query-internals candidates
  — 2-D per-polygon bbox records, pre-converted winding vertex cache +
  short-circuits, descending quartile first-hit) stays parked — the park
  decision is in
  [Gap budget per cell](issues/12-gap-budget-per-cell.md), and ticket 18's
  measured fuel attribution keeps its candidates in the same below-fold class —
  until a cell lands
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
