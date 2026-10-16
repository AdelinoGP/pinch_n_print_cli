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

**Current selection scope (2026-10-16, on the t67 take):** the frontier was
re-derived on resume — open:
[67](issues/67-region-preparation-derivation-sharing.md) first and
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold — and
the session took the first frontier ticket in order:
[67](issues/67-region-preparation-derivation-sharing.md) — **resolved
(2026-10-16) as measured GATE NOT MET with a human-approved KEEP**: the wall
(destination) clause passed in all four pre-declared classic supports-on
cells (paired medians −0.11 %, −4.26 %, −0.50 %, −0.28 %; 6/6 retained pairs,
zero starvation exclusions; 16/16 qualification byte-identical to the
preserved t58 references) with the ordinary base-classic-on win
CPU-corroborated (−0.60 %), while the CPU corroboration clause missed in
accelerated-benchy by +0.19 % (+0.31 s on a 118 s run) — the human kept the
candidate over that sub-noise miss and it is committed (`f4c542a3`) with its
tests. Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) is the only open child
(audited, parked below-fold), awaiting the human's route decision. The
standing paired acceptance and human KEEP/DROP rules are unchanged; new
questions require the human's route decision.

**Current selection scope (2026-10-14, on the t60 take):** the frontier was
re-derived on resume — open:
[60](issues/60-host-native-attribution.md) first and
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold — and
the session took the first frontier ticket in order:
[60](issues/60-host-native-attribution.md) — **resolved (2026-10-14) with the
`host:native` bucket fully attributed: residual zero and exact scope-call
reconciliation in all four classic cells** (owner families: the two
region-preparation derivations, the `region_partition` commit hook,
classic-perimeters/linker guest host-service calls, the prepass seam-planner,
and on supports-on the planner/renderer/batch/aggregation support machine;
outputs byte-identical to the t58 references; probe reverted, no production
edit). One candidate graduated:
[67](issues/67-region-preparation-derivation-sharing.md). Frontier now:
[67](issues/67-region-preparation-derivation-sharing.md) (open, awaiting the
human's route decision) first, [20](issues/20-consume-only-when-read-annotations.md)
parked below-fold. The standing paired acceptance and human KEEP/DROP rules are
unchanged; new questions require the human's route decision.

**Previous selection scope (2026-10-14, on the t59 take):** the frontier was
re-derived on resume — open:
[59](issues/59-prep-slice-flat-bridge-closing-reduction.md),
[60](issues/60-host-native-attribution.md), and
[20](issues/20-consume-only-when-read-annotations.md) (parked below-fold) —
and the session took the first frontier ticket in order:
[59](issues/59-prep-slice-flat-bridge-closing-reduction.md) — **resolved
(2026-10-14) as verdict (b): no representation-safe reduction** (the enclosure
closing's per-component 10 % gate is decided by miter-offset artifacts from
~72–74 degenerate sliver paths in `support`; a set-preserving local-closing
candidate flips verdicts on 38.5 % of benchy and 68.2 % of base call layers;
far-guards have zero hits; the refuted shape's ceiling is ≈12 % of the
closing's accumulated thread-time). No production edit, no timing campaign,
nothing graduated. Frontier now: [60](issues/60-host-native-attribution.md)
first, [20](issues/20-consume-only-when-read-annotations.md) parked
below-fold. The standing paired acceptance and human KEEP/DROP rules are
unchanged; new questions require the human's route decision.

**Previous selection scope (2026-10-14, after the t66 take):** the human
selected [66](issues/66-paint-policy-dead-walk-reduction.md) from the open
frontier; the take resolved as a **measured GATE NOT MET — human-approved
DROP** (annotations-empty fast path; 16/16 identity qualification; benchy
wall flat inside paired noise while base wall and all-cell CPU improved;
candidate retired to the committed re-appliable patch, no production edit).
Next: re-derive the open frontier on resume. The standing paired acceptance
and human KEEP/DROP rules are unchanged; new questions require the human's
route decision.

**Previous selection scope (2026-10-14, after the t64 take):** on
resumption the human said "continue" without naming a ticket, so the session
took the first frontier ticket in order —
[64](issues/64-tree-support-renderer-attribution.md) — **resolved as an
attribution take (2026-10-14)**: the probed renderer's fuel is owned by a
**dead paint-policy scan** (`paint_policy_for` 79.7%: 0 covers / 0
eligibility checks in 836 queries — walk without a covering region, exact
reduction shapes identified) with skeleton containment second at 15.9%;
`scan_fill_region` — the ticket's named suspect — is measured **0.43%
inclusive, no-useful-reduction**; wall relevance is bounded (renderer
accumulated worker 6.79–8.35 s over 12-thread phases of 8.2–8.7 s =
6.5–8.5% occupancy, 0–2 ms on the last-finishing layer). The reduction
graduated to
[66](issues/66-paint-policy-dead-walk-reduction.md) (open, awaiting human
route/KEEP-DROP later; ticket creation is not execution approval). The
existing paired-mode acceptance and human KEEP/DROP rules are unchanged; new
questions require the human's route decision.

**Previous selection scope (2026-10-13, after the t63 take):** the dispatch-granularity mechanism is
established at worker level, the ticket's measured opportunity is shown
stale at post-t65 HEAD (deficit 42–50% → 2.4–7.0%; lone-open tail gone),
the generic experiment is recommended NOT opened, and the fresh scheduling
baseline the t65 route note required has now been taken (seven probe runs;
see the ticket's Answer and
[evidence](evidence/t63-lowlayer-dispatch/FINDINGS.md)).

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
   candidate **resolved as human-approved DROP/inconclusive (2026-10-05;
   output-identical bbox alternative fails the ordinary-wall clause; candidate
   source and active artifacts restored)**; its gate prerequisite
  [Frozen-job output drift boundary](issues/46-frozen-job-output-drift-boundary.md)
  **is resolved (2026-10-05: intended behavior; current refs `5a61a106…` and `e1088b3d…`+alias;
  frozen `7049a06d…` / t44 `1b71f83d…` retire as this job's timing baselines)** — post-ticket-47 the fan-command reference
  boundary (see Decisions: ticket 47) also applies to any fresh reference
  45 derives)** → below-fold
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
   **2026-10-03 update: 36 claimed and resolved; DROP approved by the human.**
   The human selected the
   [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md)
   as this session's take from the open frontier. Resolution:
   **DROP/inconclusive, approved by the human 2026-10-03.** The modal Z/F
   candidate is proven semantics-preserving by an independent interpreter
   (`f49b5ec5…` / `c237fb01…` normalized move-stream SHA, both cells, both
   modes) and removes 27.0–27.1% of G-code bytes, but it fails the pre-declared
   CPU-corroboration gate in 3 of 4 batches (wall −0.3…−1.7%, inside the job's
   own spread; CPU flat-to-higher), and a ~1% effect cannot matter against this
   destination's 6.28–26.24x gap. The candidate stays uncommitted and is
   preserved as a re-appliable patch; a byte/bandwidth-bound consumer would be
   a separate decision. A second audit session re-derived every figure from the
   raw captures and replayed both arms byte-for-byte (see the ticket's audit
   section), corrected one `FINDINGS.md` gist sentence on the CPU deltas,
   purged the take's candidate-built binaries from the standing
   `target/dist-accelerated/developer/` and `target/debug/` dirs (rebuilt from
   HEAD), and disclosed the t47-caused `empty_end_gcode_emits_no_block` red test
   on the ticket.
   See [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md).
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
  2026-09-23; the semantics are now stated at the definition sites, the
  rendered table, and `17_agent_debugging.md` — see [profile wall-column
  semantics](issues/30-profile-wall-column-semantics.md), 2026-10-07);
  `--module-dir modules/core-modules` is mandatory
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
  **Prepared-cache experiment completed; DROP approved (2026-10-06):**
  [Prepared winding-coordinate cache](issues/51-prepared-winding-coordinate-cache-experiment.md)
  implemented the exact cache and passed exactness/output-identity, but the
  initial campaign was inconclusive and the one explicitly authorized repeat
  failed accelerated wall under the unchanged both-modes gate (ordinary passed).
  The human approved **DROP and restore**: candidate source/tests retired to
  `HEAD`, the preserved patch remains re-appliable, rebuilt guests are
  byte-identical to archived baselines, and both freshness checks are green.
  No commit, adoption retry, one-sided exception or new ticket; see
   [findings](evidence/t51-winding-cache/FINDINGS.md).
   **Current route (2026-10-07):** the human confirmed the
   [Scanbeam TopX projection-cache experiment scope](issues/56-scanbeam-topx-cache-experiment-scope.md)
   after the completed large-body-union diagnostic. Its
   [execution child](issues/57-scanbeam-topx-cache-experiment.md): the isolated
   shared-engine candidate passed exactness and the entire fresh 16-cell output
   matrix byte-for-byte in both modes, but the frozen paired timing campaign
   **stopped at its first cell — `ordinary-benchy-classic-on` — with the
   candidate slower on both wall and process CPU in 6 of 6 retained pairs**
   (wall +2.14%, CPU +2.78%, no starvation exclusions). Because the both-modes
   gate requires every timed cell to pass, the remaining three cells were not
   run by rule, and the measured recommendation is **DROP — human accepted
   2026-10-07** (no commit, adoption or retry; the standing authorization was
   confirmed to cover the externally interrupted-cell rerun). Two external
   orchestration stops
   (a self-inflicted command timeout and a harness session eviction) were
   recovered by re-proving completed cells from raw bytes and rerunning only
   untouched cells; no gate-failing cell was ever rerun. Production restored,
   both freshness checks green; see
   [findings](evidence/t57-topx-cache/FINDINGS.md). Earlier
   below-fold/serial-floor parking remains in force.
   **Next route (2026-10-07):** after t57's DROP the human selected the small
   non-timing ticket family. This session took the first in number order,
   [mesh_ops fixture limits](issues/29-mesh-ops-fixture-limits.md), and
   resolved it (fixture strategy recorded, both criterion leaves replaced,
   guards + negative control, fresh baselines). Tickets 30/31/34 remain open
   frontier; no route-performance work was authorized or performed by that take.
   **Same route, continued (2026-10-07):** this session then took the next in
   number order, [profile wall-column semantics](issues/30-profile-wall-column-semantics.md),
   and resolved it — the accumulated-thread-time unit is now stated at the type
   docs, module docs, rendered table, ADR-0055, `09_progress_events.md`, and
   `17_agent_debugging.md`, with the rendered note pinned by a test.
   **Same route, third take (2026-10-07):** the session then took
   [clipper2 1.2.0 geometry delta](issues/31-clipper2-1-2-0-geometry-delta.md) and
   resolved it — **stay on 1.1.0, no bump**. The 2-of-367 fixture delta was
   reproduced and attributed to `clean_collinear`, which is a
   **canonical-convergence fix** the repo's bundled canonical Clipper2 1.5.2
   already has, so 1.1.0 is the diverging side; DEV-173's posture is unchanged
   and the bump buys nothing this map needs. `PolyFace64` is a 1.1.0 addition
   (premise corrected) and remains unused. Ticket 34 was then the last open
   frontier ticket in this family; no timing run, candidate, adoption or commit
   was part of the take.
   **Same route, fourth take (2026-10-07) — the family is now complete:** the
   session took that last one,
   [Config padding disclosure boundary and viewer-key gate](issues/34-config-padding-disclosure-boundary.md),
   and resolved it. The premise was **corrected before acting**: `ORCA_CONFIG_PADDING`
   is inert in every production path (the registry projection reaches 233 pairs
   while the padding loop breaks at 96, so it never fires; the table only serves
   the test/legacy callers). The audit nevertheless found **two real wire-format
   defects in the block itself** and fixed both: booleans were rendered
   `true`/`false` where OrcaSlicer's wire literals are `1`/`0` (23 production
   lines; the substitution branch in `ConfigBase::set_deserialize_raw` —
   `enum_looks_like_true_value` is true only for `enabled`/`on` — loaded every
   rendered `true` as `false`, e.g. `detect_overhang_wall` and
   `support_remove_small_overhang`), and
   four enum values used PnP-internal spellings
   (`wall_sequence`, `wall_generator`, `wall_direction`, and the `seam_mode` alias
   rendering `rear` for Orca's `back`), which Orca's strict map lookup answers by
   substituting the option's own default. Two more keys PnP models as `bool` but
   Orca declares as `coEnum` (`dont_filter_internal_bridges`,
   `enable_extra_bridge_layer`) went through the same translator. The table itself
   was audited row by row: 10 values were corrected (7 to the pinned upstream
   default, 3 more in enum domain), 4 rows fabricated keys the viewer's
   `GCodeProcessor` consumes (contract-forbidden; swapped for neutral keys), 3 rows
   named keys OrcaSlicer cannot load
   (`support_material`, `extra_perimeters` removed; `wipe_tower_width` retained as
   a `handle_legacy` rename), and 4 rows named keys absent from OrcaSlicer entirely
   and were replaced with real cosmetic keys. A freshly generated production block
   now shows **0 strict wire violations** against the pinned source. No timing,
   geometry, adoption or default change; no guest touched; both freshness gates and
   all required code gates pass. The residual — the padding-only fallback block
   still cannot satisfy Orca's `>= 80` loadable-pair gate on its own because the
   remainder are PnP's own key names, which no table edit can fix — is recorded on
   the ticket rather than silently patched.
   **Next route (2026-10-08):** with the small non-timing family complete, the
   human reopened [offset2_ex call reduction](issues/19-offset2-ex-call-reduction.md) —
   its below-fold park is lifted for that candidate's take; its blockers 12/17
   are resolved; acceptance is the standing paired ordinary + accelerated A/B
   (timing chain position 4, keep/drop to the human, no auto-commit).
   **Take complete (2026-10-08):** the reopen's premise was re-derived at HEAD
   and **is stale** — the ticket's 720-call `min_width_top` headline site does
   not execute on the matched classic job (0 calls; 13 layers / 0.037% of module
   fuel with its gate forced on), and the two other named shapes are dead or not
   output-preserving. The surviving candidate is a shape the ticket did not
   name: the `classify_narrow_island` opening-emptiness short-circuit (two-pass
   `opening_ex` → shared single-erosion predicate; site calls 720 → 480, module
   fuel 750.03 B → 732.43 B ≈ −2.35% at the probe, output byte-identical). The
   standing paired A/B **passes both modes** — ordinary −0.65% median wall /
   −2.08% median CPU, accelerated −1.47% / −1.04%, six retained pairs per mode,
   no starvation exclusions, all 32 outputs byte-identical to `34886ccb…`.
   Measured **KEEP approved by the human and committed with this ticket
   (2026-10-08)**; both freshness gates are green. A
   provenance constraint was measured: any text change to `polygon_ops.rs`
   shifts the accelerated guest's artifact hash (name-section `.llvm.<hash>`
   suffixes) even for a comment, so the shipped source keeps the measured bytes
   and takes a manifest-level lint allow instead of rewriting the expression.
   See [findings](evidence/t19-offset2-call-graph/RESULTS.md) and the ticket's
   Answer for the full evidence set.
   **Second take (2026-10-08, routing claim + premise audit):** the session took
   the first numbered open child, [consume-only-when-read
   annotations](issues/20-consume-only-when-read-annotations.md), as a routing
   claim only — t19's lift was candidate-scoped and the below-fold park in
   [Gap budget per cell](issues/12-gap-budget-per-cell.md) still binds, so no
   implementation, experiment or timing started. The claim resolved into a
   read-only premise audit whose verdict **keeps the premise and sharpens it**:
   the only production site making a decision on the two per-vertex stamps is
   `OverhangClassifierDefault::run_finalization`
   (`modules/core-modules/overhang-classifier-default/src/lib.rs`), and it
   early-returns — twice — before reading any stamp whenever
   `enable_overhang_speed` is false or all four `overhang_1_4_speed` …
   `overhang_4_4_speed` bands are 0; none of the four matched configs authors
   those keys (manifest defaults `true` / `0.0`), so **on every matched cell the
   dominant measured query cost is stamped and never read**. The surviving
   candidate shape is therefore a conditional stamp gated on the consumer's own
   liveness predicate (value-identical when live) — a fuel→wall question the
   standing paired A/B still decides. Route returned to the human: t20 take vs
   [serial host floor](issues/27-serial-host-prepass-floor.md). See the ticket's
   Comments for the full consumer/gate audit.
   **Closure (2026-10-09):** the human-selected
   [serial host floor](issues/27-serial-host-prepass-floor.md) take completed —
   probe-3 attributed 99.8% of `commit_support_analysis_builtin`'s serial sweep
   to one predicate (`SliceRegionView::derive_needs_support`'s per-region
   `intersection_ex` against the object-global overhang footprint: 24.19 s of
   the 38.20 s base supports-on stage wall) and moved it to an order-preserving
   indexed rayon pre-pass (dual-compute 0 mismatches on 495 base + 240 benchy
   regions; 8.7x on the predicate). The standing four-cell paired A/B measured
   base median wall −18.198 s ordinary / −19.380 s accelerated at ≈+0.5% median
   process CPU (benchy −1.105 s / −0.943 s, the only cell passing the strict
   wall-and-CPU clause; base was accepted as a serialization fix), all 48
   outputs byte-identical, and the **human KEPT the candidate 2026-10-09**;
   committed with the ticket. The ticket resolves with this keep; the bbox
   filter was refuted and the `from_ir` clone hypothesis measured dead.
   Frontier: [consume-only-when-read annotations](issues/20-consume-only-when-read-annotations.md)
   is now the only open child — premise audited, unblocked — awaiting the
   human's route decision; the below-fold park in
   [Gap budget per cell](issues/12-gap-budget-per-cell.md) is unchanged, and
   the structural route it conditions on is now fully resolved (24/22/27/28 +
   the re-scoped 18), with no cell inside its ~1.5x release bound. The
   remaining serial-floor leads stay in the fog above. This closure session
   started no candidate, experiment or timing run.
   **Refresh (2026-10-09):** a re-run attribution session executed the fresh
   classic-cell pass (four matched classic cells at HEAD, profiled +
   instrumented; the 2026-10-09 t27 A/B base captures re-reduced; evidence in
   `evidence/perf-refresh-2026-10-09/`, `verify.py` exit 0). Headline finding:
   `PostPass::LayerFinalization` on base classic supports-on measures
   100.2–121.3 s in **all 24** probe-free 2026-10-09 A/B runs — never
   attributed — and decomposes to the four finalization modules' per-module
   whole-print `layers.clone()` + drop (stage 110.1 s = module brackets 62.7 s
   + gaps 47.4 s; module guest fuel ≤ 1.4 B of 13,090 B) plus a further pre-emit
   `to_vec`+reconcile copy (≈11.1 s between stage brackets); base supports-off
   is the 2.0 s control. Also refreshed: `host:slice`'s `closing_ex` remains the
   largest host scope on base (384–389 s accumulated; the flat-bridge enclosure
   in `assemble_flat_bridge_areas`), `host:native` carries 197.2 s accumulated
   on base supports-off with no named owner, and guest fuel remains
   `classic-perimeters` self (the ticket-20 target). **Three new tickets were
   brought to the table for the human's selection — none parked, none claimed:
   [58](issues/58-layerfinalization-whole-print-copying.md),
   [59](issues/59-prep-slice-flat-bridge-closing-reduction.md),
   [60](issues/60-host-native-attribution.md).** The below-fold park is
   unchanged; no candidate, experiment or timing campaign ran in the refresh.
   **Take completed (2026-10-10 → 2026-10-12, ticket 58):** the human selected
   [58](issues/58-layerfinalization-whole-print-copying.md); the take
   implemented the clone-once finalization stage + owned pre-emit capture
   candidate (three runtime files, uncommitted), qualified a fresh 16-cell
   output-identity matrix (ordinary cells reproduced the preserved production
   references; base-arachne-on fresh ref `a1bc8110…`), and **PASSED the
   standing paired gate in all four pre-declared cells** with 6/6 retained
   pairs and zero starvation exclusions: base-classic-on wall medians
   **610.0 → 539.5 s (−10.48 %) ordinary and 601.0 → 547.9 s (−8.76 %)
   accelerated**, CPU corroborating (−2.33 % / −2.45 %); benchy-classic-on
   −1.21 % / −1.44 %. Attribution (instrumented single runs,
   corroboration-only): the LayerFinalization stage dropped
   **113.2 → 70.4 s (−37.7 %)**, mechanism exactly as the ticket hypothesized —
   the three inter-bracket commit-drop gaps (8.0/13.9/13.1 s) became **0 ms**
   (one 2.7 s upfront clone, one 10.1 s stage-commit tail); the postpass phase
   went 132.3 → 87.4 s. The campaign ran under disclosed background load
   (Opera churn; one other-checkout test binary for a window), survived three
   interruptions (two opencode server restarts → partial cells deleted and
   re-run per the t57 recovery discipline; one harness 600 s-timeout
   self-stop → per-run timeout raised to 1800 s, infrastructure-only and
   disclosed), and closed with the standing snapshots restored and both
   freshness gates green. **Recommendation to the human: KEEP — human KEPT
   it 2026-10-12; no auto-commit was made before the ruling — the
   candidate and evidence are committed (code `e7159687`, docs
   `e4e93dea`).**
   Evidence: `evidence/t58-finalization-copies/FINDINGS.md` (+
   `qualification.json`, `ab-reduction.json`).
   **Tracker catch-up (2026-10-12, same session):** the previously
   uncommitted t29/t30 source changes and the t29/t31/t45/t49-t57 tracker
   and evidence sets are now committed (byte-pinned in `.gitattributes`;
   every checkout-only evidence verifier exits 0); t58's tracker text was
   reconciled to the committed KEEP. The frontier is unchanged and still
   awaits the human's selection:
   [20](issues/20-consume-only-when-read-annotations.md) (audited, parked
   by the below-fold park),
   [59](issues/59-prep-slice-flat-bridge-closing-reduction.md),
   [60](issues/60-host-native-attribution.md).
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
       **Oracle qualification attempt (2026-10-02):** the
       [Integrated/external matched-output oracle gate](issues/33-integrated-external-matched-output-oracle-gate.md)
       FAILED — integrated dispatch output-diverges (native
       `Layer::InfillPostProcess` views miss the partitioned-`SliceIR`
       enrichment the WASM leg performs; the linker falls back to the union
       boundary and prints paths the external leg clips away), reproducing
       perf-split Finding 3's TYPE deltas on a fresh tree. Integrated timings
       stay disqualified and the integrated edition gains no timing evidence.
       The enabling `raft-default` integration-row repair (packet-240b
       registry drift; `xtask dist --edition integrated` was unbuildable) is
       committed (`4a8330a6`); the named dispatch fix was resolved, verified
       and KEEP-approved under
       [Native postprocess-view enrichment repair](issues/47-native-postprocess-view-enrichment.md)
       (2026-10-02 → 2026-10-03; supersedes this FAILED verdict — the gate now
       PASSES from proof, so integrated timings are usable as an oracle on the
       examined cell).

## Decisions so far

<!-- one line per closed ticket; the ticket holds the detail -->

- [Region-preparation derivation sharing across region projections](issues/67-region-preparation-derivation-sharing.md): **resolved (2026-10-16; measured GATE NOT MET on one CPU-corroboration clause — human-approved KEEP, committed `f4c542a3` with tests)** — `LayerArena` serves the perimeter and infill region projections from one prepared-region derivation when the perimeter source projection onto the regions is the identity (modifier sub-regions that restore base geometry keep the two derivations separate; no second cache, no clone). Qualification: 16 proof cells byte-identical (`build-qualification.py` pass 16/16, `verify-references.py` 16/16 against the preserved t58 references). Standing paired gate on the four pre-declared classic supports-on cells (56 timed runs, 6/6 pairs everywhere, zero starvation exclusions): paired-median wall Δ −0.11 % ordinary-benchy / **−4.26 % ordinary-base** / −0.50 % accelerated-benchy / −0.28 % accelerated-base; CPU Δ −0.44 % / −0.60 % / **+0.19 %** / −0.11 %. Read plainly: the only delta outside run noise is ordinary base-classic-on (−4.26 % paired and −4.12 % unpaired, 4 of 6 pairs by 2–9 %, CPU-corroborated ≈ −14 s, consistent with t60's attributed duplicate derivation); ordinary-benchy and accelerated-base flip sign between paired and unpaired medians (±0.3 %, inside noise) and the benchy cells were pre-declared at/below the 0.1–0.5 % floor; the single clause miss is +0.31 s of CPU on the 118 s accelerated-benchy slice. Evidence: `evidence/t67-region-prep-sharing/FINDINGS.md` (+ `qualification.json`, `ab-reduction.json`).

- [host:native attribution: name the per-layer native geometry bucket](issues/60-host-native-attribution.md): **resolved (2026-10-14; bucket fully attributed — residual `host:native` = 0 with exact per-scope call reconciliation in all four classic cells; probe reverted, no production edit)** — four fresh ordinary profiled captures (outputs byte-identical to the t58 references; clean) carry the temporary `T60-PROBE` host instrument (thread-local sink owner + worker-thread owner for rayon batch items; `probe.patch`, reverted). Owners: the two region-preparation derivations (`ensure_prepared_regions` / `ensure_prepared_perimeter_source_regions`; 1.48+1.47 s benchy-off, 40.53+40.18 s base-off accumulated, once per layer, 661/1,926 clips each), `region_partition`'s `sync_perimeter_infill_areas_into_slice` 3×/layer (1.26 s benchy-off / 10.04 s base-off; 6,120/13,320 clips), guest-requested host services from per-layer dispatches (classic-perimeters 5.91/89.26 s; infill-linker 0.53/3.10 s; supports-on tree-support 3.79/40.29 s), the prepass seam-planner (0.95/13.67 s) and — supports-on — the support machine now named rather than suspected: tree-support-planner 6.49/160.33 s, its rayon batch-offset items 0.25/2.74 s (24,255 items = ticket 22's independent count), and support-plan aggregation clips 0.22/2.09 s. The supports-off headline bucket (benchy 11.60 s / base 176.78 s accumulated; base-off baseline was 197.2 s) is now a named split, not an unknown. One candidate graduated:
  [Region-preparation derivation sharing across region projections](issues/67-region-preparation-derivation-sharing.md) (the two derivations are identical work over identical region sets on the matched jobs; ≈80.7 s accumulated base-off); named but un-graduated leads (classic-perimeters host round trips; `region_partition` dedup) are in the map's fog. Evidence: `evidence/t60-host-native/` (`FINDINGS.md`, `summary.json`, `captures/`, `probe.patch`, `run-t60.ps1`, `verify.py` exits 0 from the committed set alone with six negative controls rejected).

- [PrePass::Slice flat-bridge closing reduction](issues/59-prep-slice-flat-bridge-closing-reduction.md): **resolved (2026-10-14; verdict (b) — no representation-safe reduction; no production edit, no timing campaign)** — the closing is consumed only as the per-component ≥10 % gate; on real layers that gate is decided by miter-offset artifacts from ~72–74 degenerate sliver paths in `difference`'s output (drilled: benchy 161 raw 325.3 mm² vs 215.4 normalized on the same point set; base 8 raw 31.5 mm² vs 72.1 normalized — on base the raw closing is smaller than its own input), so a set-preserving local-closing candidate flips verdicts on 92/239 benchy (38.5 %) and 320/469 base (68.2 %) call layers — refuted; far-guards have zero hits (0 of 14,951 components); baseline gate margins reach 4e-5; the refuted shape's measured ceiling is ≈12 % of the closing's accumulated thread-time. Evidence: `evidence/t59-flat-bridge-closing/` (`FINDINGS.md`, `probe.patch`, `reduced/`, `verify.py` exits 0).

- [Support paint-policy dead-walk reduction](issues/66-paint-policy-dead-walk-reduction.md): **resolved (2026-10-14; measured GATE NOT MET on the wall clause — human-approved DROP, candidate retired, no production edit)** — the annotations-empty fast path (`slicer_core::paint_policy::has_support_paint` plus the early return in `PaintRegionLayerView::paint_policy_for`) passed the full 16-cell output-identity qualification (baseline == candidate == preserved t58 references; `build-qualification.py` 16/16 and `verify-references.py` 16/16, all clean), then the standing paired gate on the four pre-declared classic supports-on cells (6/6 pairs everywhere, zero starvation exclusions, 56/56 runs byte-identical): **CPU improved in all four cells (−0.56…−1.22 %)** and **base wall improved (−0.4977 % ordinary / −2.1098 % accelerated)**, but **benchy wall sat inside paired noise (+0.0017 % / +0.1272 %)**, below the pre-declared wall < 0 % clause — the dead walk is a real ~1 %-class CPU cost whose wall transfer is base-only. Candidate retired to `HEAD` with the re-appliable artifacts committed (`evidence/t66-paint-walk/candidate.patch` sha `8ccfbeb1…` + the new-test-file copy, both byte-identical to the as-measured state); no adoption retry; re-entry needs a new pre-declaration. Interruption recoveries disclosed (one harness eviction kill mid-proof, two further evictions survived by the detached relaunch; background load; zero exclusions). Evidence: `evidence/t66-paint-walk/FINDINGS.md`.

- [Tree-support renderer self-fuel and scan-fill attribution](issues/64-tree-support-renderer-attribution.md): **resolved (2026-10-14; attribution take, probe reverted, no production edit, no timing campaign)** — four fresh profiled captures (outputs byte-identical to the t58 references, all four cells); the renderer's 43.510 B dispatch fuel splits: **`paint_policy_for` 34.697 B (79.7%)** — per-query counters pin a **dead walk** (1 rebuilt region / ~428 vertices / ~1,285 rings / ~893K edge tests × ~46.5 instr each; **0 covers, 0 eligibility checks in 836 queries** → `DefaultEligible` every time), two exact reduction shapes graduated to [Support paint-policy dead-walk reduction](issues/66-paint-policy-dead-walk-reduction.md); **`skeleton_wall_count` 6.921 B (15.9%)** (bbox-scan walks 377.7 M of it; ranked second); **`scan_fill_region` 0.187 B (0.43%) inclusive — the ticket's named suspect measured no-useful-reduction** (4,443 rows / 2.8 M edge walks / 2.008 conversions per edge; role splits exact); wall relevance bounded: renderer accumulated worker 6.79–8.35 s over 12-thread phases = **6.5–8.5% machine occupancy**, 0–2 ms on the last-finishing layer, freeing the dead walk's share bounded ≈0.45–0.55 s of those phases' walls (attribution-only bounds, never a wall claim). Supports-off controls match the t61 baselines with the probe silent (0 counts). Ordinary mode only (disclosed); the graduated experiment carries the both-modes paired gate. Evidence: `evidence/t64-tree-support-renderer/` (`FINDINGS.md`, `probe.patch`, `verify.py`).

- [Independent low-layer dispatch and Arachne finishing-tail reduction](issues/63-low-layer-dispatch-granularity.md): **resolved (2026-10-13; diagnosis/design, no production edit, no timing campaign)** — the granularity mechanism is confirmed with worker-level evidence (a reverted env-gated probe, `evidence/t63-lowlayer-dispatch/probe.patch`): at 12 threads layers 0/1/2 are rayon's initiator left spine on worker 0 in all five fresh runs (0,1,2,3 at six threads), module-pool waits are zero at ≤12 threads (every per-layer-tier manifest is `layer-parallel-safe`; waits measured only at 24 threads, max 42 ms), and the 24-thread counterfactual collapses the serialization entirely (layer 1 stolen at ≈0 ms, layer 2 ending mid-field — same job/binary, only steal width). **The premise's measured opportunity is stale at HEAD**: the per-layer-phase scheduling deficit collapsed from 1,079–3,022 ms (12.7–49.9%) across the six pre-t65 t61 captures to 161–969 ms (2.4–7.0%) post-t65 with the lone-open tail gone (0–61 ms), because committed t65 cut the low-layer linker chunk the spine held (layer-1 linker dispatch 3,191 → 1,228–1,662 ms; layers 0–2 accumulated 4.50–6.23 → 1.39–1.80 s, ≈3–4×). Recommendation: do **not** open the output-identical dispatch-granularity experiment now (measured ceiling ≈0.12–2.3% of Benchy slice wall, below the map's ~1%-effect bar); the generic candidate shape (`with_max_len(1)`-class single-item leaves; bounded-chunk designs measured the wrong direction) and the re-open condition (a fresh scheduling baseline re-developing a 40%+-class deficit) are recorded on the ticket. Fresh scheduling baseline for 63's family: taken (the probe runs); outputs byte-identical to the t58 references; loader/provenance checks pass. Evidence: `evidence/t63-lowlayer-dispatch/` (`FINDINGS.md`, probe patch, `verify.py`).

- [Infill-linker batch-execute experiment](issues/65-infill-linker-batch-execute-experiment.md): **resolved (2026-10-13); human-approved KEEP — committed with this ticket** — one batched `Clipper64` + execute per `link_paths_without_offset` invocation with re-derived per-subject attribution (`PreparedPolylineClip::clip_batched`, `crates/slicer-core/src/polygon_ops.rs`; per-subject `clip` repair for ambiguous junction-kiss subjects, whole-call fallback reserved for unattributable segments). In-guest replay-oracle probe: 0 mismatches over 12,022–19,290 subjects per cell, both modes, outputs byte-identical to the preserved references; 10 differential tests. Qualification 16/16 byte-identical, then the standing paired gate on the 8 pre-declared arachne cells (6/6 pairs everywhere, zero starvation exclusions): **benchy arachne wall −20.01/−21.08 % (off) and −12.24/−10.84 % (on), CPU corroborating; base cells a wash** (−0.07/−0.52/−1.65 %; ordinary-base-arachne-on +0.43 % median loss, ranges overlapping) — GATE NOT MET on the letter, the failing cell explicitly accepted in the human's ruling. Production artifacts already in the candidate state; both freshness gates green. Evidence: `evidence/t65-linker-batch/FINDINGS.md`.

- [Infill-linker post-hoist residual and exact clip-work reduction](issues/62-infill-linker-post-hoist-residual.md): **resolved (2026-10-13)** — the t44 hoist confirmed working (prepare now 2.60% of linker fuel, once per invocation); the post-hoist cost is the **per-path engine round trip: `execute` 85.96% + `engine_add` 4.42% of linker fuel** across 12,022 clip calls / 393 invocations, with the prepared universe re-fed **57.3× per preparation** (classic 49.5×) and per-call cost universe-dominated (fit −2.8M + 7,990·U, R² 0.869). Qualification: 8/8 cells byte-identical both modes before the probe; all six probe captures output-neutral; cross-mode scope agreement ≤5.4e-4 relative. Candidate (batch one engine + one execute per invocation with re-derived per-subject attribution, exactness conditions measured and documented) **graduated to** [Infill-linker batch-execute experiment](issues/65-infill-linker-batch-execute-experiment.md); same-cost ceiling ≈12.58% of current linker fuel, slice share 44.28% → ≈5.57% (estimates, not measurements). Attribution/representation only; production restored, freshness green. Evidence: `evidence/t62-linker-post-hoist/FINDINGS.md`.

- [Benchy current bottleneck refresh after clone-once finalization](issues/61-benchy-current-bottleneck-refresh.md): fresh output-identical Benchy attribution graduates the post-hoist linker residual, low-layer dispatch granularity and support-renderer questions; existing leads retained, no optimization or speed verdict.

- [LayerFinalization whole-print copying](issues/58-layerfinalization-whole-print-copying.md): **resolved (2026-10-12); human-approved KEEP — committed with this ticket (`e7159687`, docs `e4e93dea`)** — the clone-once finalization stage + owned pre-emit capture candidate (three runtime files) passed the fresh 16-cell output-identity qualification (ordinary cells reproduce the preserved production references; base-arachne-on fresh ref `a1bc8110…`), then the standing paired gate **in all four pre-declared cells, 6/6 pairs each, zero starvation exclusions**: base-classic-on wall medians **−10.48 % ordinary (610.0 → 539.5 s) / −8.76 % accelerated (601.0 → 547.9 s)**, CPU corroborating (−2.33 % / −2.45 %); benchy-classic-on −1.21 % / −1.44 % with byte-identical outputs everywhere. Attribution: LayerFinalization stage 113.2 → 70.4 s (−37.7 %), inter-bracket drop gaps → 0 ms, one 2.7 s upfront clone + 10.1 s commit tail; postpass phase 132.3 → 87.4 s. Campaign context disclosed (background load; three interruption recoveries incl. a harness-timeout raise to 1800 s, infrastructure-only); snapshots restored, freshness gates green. Evidence: `evidence/t58-finalization-copies/FINDINGS.md`.

- [Serial host floor](issues/27-serial-host-prepass-floor.md): **resolved (2026-10-09)** — the post-park premise search found the new safe call reduction the ticket waited for: 99.8% of `commit_support_analysis_builtin`'s serial sweep (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) was one predicate — `SliceRegionView::derive_needs_support`'s per-region exact `intersection_ex` against the object-global overhang footprint (`crates/slicer-sdk/src/views.rs`), 24.19 s of the 38.20 s base supports-on stage wall — now moved to an order-preserving indexed rayon pre-pass (dual-compute 495/495 base + 240/240 benchy regions, 0 mismatches; 8.7x on the predicate). Standing paired A/B: base median wall **−18.198 s ordinary / −19.380 s accelerated** (6/6 and 5/6 pairs) at ≈+0.5% median process CPU; benchy −1.105 s / −0.943 s. All 48 outputs byte-identical; the strict wall-and-CPU clause passes on ordinary/benchy only and the **human KEPT the base wall win 2026-10-09** (≈3.4–3.7% of arm-median wall; accepted as a serialization fix, not a scoreboard claim), committed with the ticket. The probe-3 bbox-index filter alternative was refuted (~45% pruning, ~12% saving) and the `from_ir` deep-clone hypothesis measured dead. Remaining leads (flat-bridge representation analysis, `apply_opening` parity, teardown early release) stay in the fog. Evidence: `evidence/t27-serial-floor/FINDINGS-PROBE3.md`.

- [offset2_ex call reduction](issues/19-offset2-ex-call-reduction.md): the reopen's premise was re-derived at HEAD and **is stale** — the ticket's 720-call `min_width_top` headline site does not execute on the matched classic job (0 calls; 13 layers / 0.037% of module fuel with its gate forced on), and its other two named shapes are dead or not output-preserving. The surviving candidate is a shape the ticket did not name: the `classify_narrow_island` opening-emptiness short-circuit (two-pass `opening_ex` → shared single-erosion predicate; site calls 720 → 480, attributed module fuel 750.03 B → 732.43 B ≈ −2.35%; output byte-identical). The standing paired A/B **passes both modes** — ordinary −0.65% median wall / −2.08% median CPU, accelerated −1.47% / −1.04%, six retained pairs per mode, no starvation exclusions, all 32 outputs byte-identical to `34886ccb…` — with the effect small and not a gap-closer. Measured **KEEP approved by the human and committed with this ticket (2026-10-08)**; both freshness gates green. A provenance constraint was measured: any text change to `polygon_ops.rs` shifts the accelerated guest's artifact hash (name-section `.llvm.<hash>` suffixes) even for a comment, so the shipped source keeps the measured bytes and takes a manifest-level lint allow instead of rewriting the expression. Evidence: [findings](evidence/t19-offset2-call-graph/RESULTS.md) and the ticket's Answer.

- [Large body-union exact corpus and native-internals investigation](issues/55-large-body-union-corpus-and-internals.md): largest/hole-bearing full-role replay and paired-mode output identity pass; native sweep counters establish unchanged-edge `TopX` recomputation and recommend a separately scoped projection cache, while exact polygon deduplication is falsified for the selected corpus. No candidate or speedup claim; sources/artifacts restored and both freshness gates pass.

- [Large body-union investigation scope](issues/54-large-body-union-investigation-scope.md): human authorized immediate largest/hole-bearing real-input capture, exact full-role replay and native-union internal attribution on the unchanged paired-mode matched jobs; production candidate and speed campaign remain separate approvals.

- [Tree-planner union/simplify current-job attribution](issues/53-tree-planner-union-simplify-current-job-attribution.md): measured all four cells with byte-identical clean baseline/probe outputs; simplification never executes, native body union dominates, and no expensive removable call was established. Recommendation: park simplify/four-role batching; a separate large-body-union scope must address largest/hole-bearing replay gaps. Production sources/artifacts restored and both freshness gates pass.

- [Tree-planner union/simplify investigation scope](issues/52-tree-planner-union-simplify-investigation-scope.md): human authorized fresh Benchy/base classic supports-on attribution in both modes, preserving the exact current chain and qualifying temporary probes against fresh outputs; diagnostic execution is separate, with no candidate or speed campaign authorized.

- [Prepared winding-coordinate cache: exactness + standing paired A/B](issues/51-prepared-winding-coordinate-cache-experiment.md): **DROP approved by the human (2026-10-06)** — the exact per-context contour-coordinate cache passed seven exactness tests in both modes, the precision negative control, byte-identical current outputs, and all code gates, but the standing both-modes wall/CPU gate failed on accelerated wall: the initial campaign was inconclusive (2/6 ordinary, 1/6 accelerated pairs retained) and the one human-authorized repeat retained 6/6 in both modes with ordinary passing (−1.35% wall / −2.08% CPU medians) and accelerated failing (+1.35% wall / −1.16% CPU, ranges overlapping). Candidate source/tests retired to `HEAD`, re-appliable patch and both campaigns preserved, rebuilt guests byte-identical to archived baselines, both freshness gates green. No commit, adoption retry, one-sided exception, further retry or new ticket.

- [Prepared winding-coordinate cache scope](issues/50-prepared-winding-coordinate-cache-scope.md): human authorized one eager per-context contour-coordinate cache experiment for signed-distance sign on the same supports-off Benchy Arachne job, with independent exactness and the unchanged paired-mode end-to-end wall/CPU gate; execution graduated separately, no candidate or speed claim yet.

- [Signed-distance sign-versus-distance attribution](issues/49-signed-distance-sign-versus-distance-attribution.md): fresh paired-mode probes localize the accelerated residual to exact winding classification; output checks, non-overlapping fuel accounting and production restoration pass. An exact prepared winding-coordinate cache is recommended for a separate scope decision, not implemented or speed-approved.

- [Signed-distance investigation scope](issues/48-signed-distance-investigation-scope.md): human authorized fresh supports-off Benchy Arachne sign-versus-distance attribution in ordinary and accelerated modes; diagnosis only, with current-output checks. No candidate implementation, speed campaign, commit, adoption retry, or default change authorized.

- [Overhang-quartile query: predicate narrowing + standing paired A/B](issues/45-overhang-quartile-predicate-narrowing.md): **DROP approved by the human (2026-10-05)** — blanket boundary-pass removal changes exact-edge verdicts; the output-identical inclusive-bbox alternative fails the standing paired-mode speed gate on ordinary wall. Candidate code/tests retired together into a re-applicable patch, evidence preserved, active distributions restored; no commit, adoption retry or default switch.

- [Frozen-job output drift boundary](issues/46-frozen-job-output-drift-boundary.md): **intended behavior, not a defect (2026-10-05)** — the frozen→current drift decomposes into exactly two documented intent programs: t44-era→current is identical move stream (92,519 moves) + the t47 fan repair (+277 lines) − the `support_overhang_angle` alias line (30 B), with `current + alias == 11e7d38a…` (t44 job) and `== e1088b3d…` (classic-off) byte-for-byte; frozen→t44-era is the config-scope-resolution resolved-view program (start block / fill 45° / wall F3000 / registry CONFIG_BLOCK 102→240 keys). All eras re-derive their own references (`7049a06d…`, `1b71f83d…`), all complete clean, zero containment violations. **Boundary: frozen `7049a06d…` and t44 `1b71f83d…` retire as timing baselines for this job; current refs are `5a61a106…` (t44 job) and `e1088b3d…`+alias (classic-off); fan-bearing jobs need post-t47 refs.** Ticket 45's gate prerequisite is satisfied; not map-wide authorization.

- [Native postprocess-view enrichment repair](issues/47-native-postprocess-view-enrichment.md): **KEEP approved by the human (2026-10-03); t33 oracle gate re-run from proof PASSES (benchy-classic-off, byte-identical arms `e1088b3d…` across proof + 3 ABBA pairs; frozen t44-job corroborates `11e7d38a…`)** — three native-dispatch projection gaps fixed: (1) the scoped `Layer::InfillPostProcess` partitioned-`SliceIR` view enrichment (`native_infill_postprocess_regions`), (2) the ordered-entities snapshot the native path-optimization/anchored-events arms never received (travel reordering silently skipped), and (3) the finalization WIT annotation channel whose absence made the EXTERNAL leg silently drop part-cooling's `M106`/`M107` fan commands on every job (package `slicer:finalization-layer-finalization` 1.0.0→1.1.0; disclosed intentional output change on BOTH legs — pre-repair external baselines like t44's `1b71f83d…` are outdated for fan-bearing jobs; post-repair references: classic-off `e1088b3d…`, t44-job `11e7d38a…`). Integrated timings are usable as an oracle on the examined cell, bounded to it; timing campaigns/adoption/default changes still need their own authorization.

- [Integrated/external matched-output oracle gate](issues/33-integrated-external-matched-output-oracle-gate.md): **gate FAILED (2026-10-02)** — integrated dispatch is not output-equivalent; the native `Layer::InfillPostProcess` request omits the partitioned-`SliceIR` view enrichment the WASM leg performs (`push_infill_postprocess_regions`), the infill linker silently falls back to the union boundary, and the native leg prints paths the external leg clips away (+572.5 mm sparse, +698.7 mm top surface etc. on the t44 job), reproducing Finding 3's TYPE deltas; integrated timings stay disqualified. Enabling repair: `raft-default` got its missing 24th integration row (packet-240b registry drift; `xtask dist --edition integrated` was unbuildable before it). Named fix scoped and now taken as [Native postprocess-view enrichment repair](issues/47-native-postprocess-view-enrichment.md) — **superseded there: the gate now PASSES from proof (2026-10-03, see that ticket's answer)**.

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
- [Modal G-code Z/F token redundancy and speed gate](issues/36-modal-gcode-token-redundancy.md): **DROP approved by the human (2026-10-03)** — an audit session re-derived every figure from the raw captures with its own census and per-move effective-Z/F walk (not the take's verifier), replayed both arms from `source-worktree.diff` byte-for-byte (baseline `e1088b3d…`, candidate `132f9501…` / `8099ce5c…`; `--reduce` regenerates `evidence.json` exactly), and found one `FINDINGS.md` gist sentence wrong on the CPU deltas (only classic-off accelerated has a positive paired median CPU delta; corrected in place). The candidate modal `Z`/`F` serializer (`ModalZfState` in `DefaultGCodeSerializer::serialize_gcode`; non-comment `Raw` and `ToolChange` invalidation; bare no-op lines for neutral moves) is proven **semantics-preserving by an independent interpreter** (`verify-t36.py`, normalized move-stream SHA identical per cell/mode: classic-off `f49b5ec5…`, arachne-off `c237fb01…`; E tokens and line counts identical; `--falsify` detects sabotage) and removes **27.10% / 26.98% of bytes** (4,332,881→3,158,453; 4,126,888→3,013,383; Z tokens 105,070→520 / 99,716→521, F 105,549→1,039 / 100,195→955), but it **fails the pre-declared CPU-corroboration gate in 3 of 4 batches** (wall −0.3…−1.7%, inside the job's own 0.38–1.41 s spread; CPU flat-to-higher; only arachne-off ordinary passes both clauses) and a ~1% effect cannot matter against the destination's 6.28–26.24x gap. **No commit, no production change; the candidate stays uncommitted** — if a byte/bandwidth-bound consumer matters later the implementation and oracle are ready (a separate decision). The take's candidate-built binaries were purged from `target/dist-accelerated/developer/` and `target/debug/` (rebuilt from HEAD; both emitted the candidate byte stream until then) and the ticket discloses the t47-caused `empty_end_gcode_emits_no_block` red test as a non-caused pre-existing failure. Evidence: `evidence/t36-modal-zf/`.

- [Scanbeam TopX projection-cache experiment scope](issues/56-scanbeam-topx-cache-experiment-scope.md): human confirmed one conservative exact-version shared host/guest trial, with independent engine tests, full-matrix output qualification and paired-mode timing bounded to the measured classic supports-on jobs; temporary integration only, execution handed off separately without implementation or speed approval.

- [Scanbeam TopX projection-cache experiment](issues/57-scanbeam-topx-cache-experiment.md): **DROP accepted by the human (2026-10-07)** — the isolated shared-engine projection cache passed independent native/actual-WASM exactness and the full fresh 16-cell output matrix byte-for-byte in both modes, but the frozen paired campaign stopped by rule at its first cell with the candidate slower on both wall (+2.14%) and process CPU (+2.78%) in 6 of 6 retained pairs; candidate preserved, production restored, both freshness gates exit 0, no commit or adoption.

- [mesh_ops fixture limits](issues/29-mesh-ops-fixture-limits.md): **resolved (2026-10-07)** — the two fixture-limited criterion leaves now measure successful operations: `repair/wedge` on tracked `resources/regression_wedge.stl` (real damage: 12 reoriented, 56 open edges closed) and `decimate/calicat_default` on tracked `resources/calicat.stl` (default error budget reaches 876→438); `cube.step` untouched, in-bench guards plus a companion test with a `cube.step` negative control make the no-op shape falsifiable, fresh `base == new` baselines in `evidence/t29-mesh-ops-fixtures/`, pre-ticket leaves (2.947 µs / 3.745 µs) superseded and not comparable. Bench-trustworthiness only; moves no cell.

- [clipper2 1.2.0 geometry delta](issues/31-clipper2-1-2-0-geometry-delta.md): **stay on 1.1.0; the bump is parked, no manifest change (2026-10-07)** — ticket 17's "2 of 367 fixtures differ" is reproduced exactly (both a single deleted, (near-)collinear spur vertex: `benchy__layer01_opening` cross 0, `benchy__layer07_closing` cross −3) and attributed to `clean_collinear`, but the delta is a **canonical-convergence fix, not a divergence**: the repo's bundled canonical Clipper2 1.5.2 matches **1.2.0**'s `startOp = op2; continue` restart, 1.1.0's is the non-canonical one, and upstream's 1.2.0 note names the fix (zero-width spurs in union output; `preserve_collinear` defaults on and this repo never clears it on `Clipper64`). DEV-173's posture does **not** improve: `check_split_owner`, `recursive_check_owners`, `is_valid_owner`, `fix_self_intersects` and the poly-creation helpers are byte-identical across the bump. The rest of the feature-off delta is plumbing plus additive API; `using_z` is off by default so it is no cost argument, and `PolyFace64`/`poly_tree_to_faces64` is a **1.1.0** addition (corrected), unused here, and does not expose the island-in-hole nesting `expolygons_from_tree` needs. Matched-cell disclosure not run (nothing moved). No bump, pin, vendoring, timing run or commit; `slicer-core` compiles unmodified against 1.2.0. Evidence: `evidence/t31-clipper2-120/` (`verify.py` exits 0 against the in-repo set).

- [profile wall-column semantics](issues/30-profile-wall-column-semantics.md): **resolved (2026-10-07, docs/comments only)** — the `--profile` wall columns are now documented as accumulated thread-time, never wall: each value sums one span per completed activation and `flush_native` adds concurrent worker activations, so a run-wide total can exceed the run's own wall-clock by the mean concurrency and is never comparable to `module_complete`/phase walls or process CPU. Semantics recorded in the `profiling_report.rs` module doc (with ticket 24's `closing_ex` ≈34 s vs 3,889 ms exemplar) and type docs (`ScopeTotals`, `ProfileScopeRow`, `ProfileModuleRow`, `ProfileSummary::wall_total_ns`, `ProfileUnit::WallNs`, `flush_native`), `slicer-wasm-host`'s `ScopeTotals` docs, the rendered wall heading and closing note, ADR-0055, `docs/09_progress_events.md`, and `docs/17_agent_debugging.md`; the rendered note is pinned by `summary_always_states_the_observer_effect_contract`. No behavior change, timing run, candidate or commit.

- [Config padding disclosure boundary and viewer-key gate](issues/34-config-padding-disclosure-boundary.md): **resolved (2026-10-07, disclosure/correctness only)** — premise corrected: `ORCA_CONFIG_PADDING` never fires on any production path (the registry projection reaches 233 pairs; the loop breaks at 96), so its values are unreachable there and only the test/legacy callers consume the table. The audit nonetheless found two **live** wire-format defects in the emitted block: booleans rendered as `true`/`false` where OrcaSlicer's wire literals are `1`/`0` (23 production lines; `ConfigBase::set_deserialize_raw`'s substitution branch — `enum_looks_like_true_value` is true only for `enabled`/`on` — loaded every rendered `true` as `false`, e.g. `detect_overhang_wall` and `support_remove_small_overhang`), and four enum values carrying PnP-internal spellings (`wall_sequence` `InnerOuter`, `wall_generator` `Classic`, `wall_direction` `counter_clockwise`, and the `seam_mode` alias rendering `rear`) that Orca's strict `enum_keys_map` lookup answers by replacing the option with its own default; two keys PnP models as `bool` but Orca declares `coEnum` (`dont_filter_internal_bridges`, `enable_extra_bridge_layer`) needed the same treatment. Fixed in one documented `translate_appendix_enum` table applied only at the appendix (module `ConfigView`s unchanged). Table audited row by row: 10 values were corrected (7 to the pinned upstream default, 3 more in enum domain), 3 rows named keys OrcaSlicer cannot load (`support_material`/`extra_perimeters` removed as non-Orca keys; `wipe_tower_width` kept — `handle_legacy` renames it), and 4 rows named keys absent from OrcaSlicer entirely, replaced with real cosmetic keys. A freshly generated production block now measures **0 strict wire violations**; the edited table compares **0 default mismatches** against the pinned `PrintConfig.cpp`. Residual: the padding-only fallback block still cannot satisfy Orca's `>= 80` loadable-pair gate because the remaining unloadable lines are PnP's own key names (registry-projected, not table rows) — recorded, not patched. No timing, geometry, adoption, default or guest change; all required gates pass.

## Not yet specified

- **Fan-command reference boundary (graduated from ticket 47, 2026-10-03;
  boundary settled by ticket 46, 2026-10-05).**
  The finalization annotation repair intentionally changed BOTH legs' output:
  external now emits part-cooling's `M106`/`M107` fan commands. Every
  pre-repair external reference (t44's accepted `1b71f83d…`, the t30-era
  matched-pair captures) is outdated for fan-bearing jobs. Ticket 46's
  frozen-job drift question is **resolved**: current refs `5a61a106…` (t44
  job) / `e1088b3d…`+alias (classic-off); frozen `7049a06d…` and t44
  `1b71f83d…` retire as this job's timing baselines. Any future keep/drop gate
  on a job with fan commands must use post-repair references or strip
  `M106`/`M107` before comparing (the ticket-46 evidence set carries the
  reconstruction: current + `; support_overhang_angle = 30` == the t47 refs).
  For ticket 45 (next chain link), the fresh-baseline gate may proceed against
  the current references; it is not map-wide authorization.
- **Renderer skeleton-containment reduction (second-ranked by t64, 2026-10-14).**
  [Tree-support renderer attribution](issues/64-tree-support-renderer-attribution.md)
  ranked `skeleton_wall_count`'s containment second at 6.921 B (15.9% of
  renderer fuel), of which the per-point 4× bbox min/max scans in
  `point_in_expolygon` walk 377,682,312 ring lengths per supports-on Benchy
  slice. A per-expoly precomputed bounding box (or per-entry skeleton memo) is
  the cheap shape, but its ceiling (~2–4 B ≈ 5–9% of renderer fuel) and the
  renderer's small wall occupancy do not yet justify a separate experiment; it
  graduates only with its own measured reason, independently of ticket 66's
  paint-walk take.
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
   fuel and its candidate graduated the now-resolved
  [overhang-quartile predicate narrowing](issues/45-overhang-quartile-predicate-narrowing.md)
   (2026-10-05: human-approved DROP of the bbox alternative; blanket boundary
   removal is not output-preserving). That take did not measure fuel or turn
    the synthetic pre-pass bound into a slice-wall saving. Additional query
     candidate shapes beyond the graduated signed-distance investigation and
      its completed exact prepared winding-coordinate cache experiment
      (human-approved DROP, 2026-10-06) remain
     unspecified; distance-query alternatives need their own reason and scope,
     not implicit permission from that split.    [infill-linker
   attribution](issues/21-infill-linker-attribution.md)'s split graduated the
   now-resolved [clip-universe preparation hoist](issues/44-clip-universe-hoist.md)
    (2026-10-01). The post-hoist linker residual graduated to
   [Infill-linker post-hoist residual and exact clip-work reduction](issues/62-infill-linker-post-hoist-residual.md)
   and is **resolved (2026-10-13)**: the current split is measured —
   per-path `execute` 85.96% + `engine_add` 4.42% of linker fuel, connect
   4.73%, prepare 2.60%, metadata 1.25% — and its candidate graduated to
   [Infill-linker batch-execute experiment](issues/65-infill-linker-batch-execute-experiment.md).
   Post-batch linker alternatives (the connect/prepare/metadata residual and
   any subject-side costs the experiment exposes) await that experiment's
   outcome rather than inheriting any assumed split. [Tree-planner substage
  attribution](issues/22-tree-planner-substage-attribution.md) graduated the
   now-resolved [Emit-pass carve gate: representation safety + paired
   A/B](issues/32-emit-carve-gate-representation-and-ab.md). Its sibling
   union/simplify lead graduated to
   [Tree-planner union/simplify investigation scope](issues/52-tree-planner-union-simplify-investigation-scope.md)
   and its now-completed diagnostic after the human selected that route
   on 2026-10-06. The clarified large-body-union question graduated to
   [Large body-union investigation scope](issues/54-large-body-union-investigation-scope.md)
   and its completed exact-corpus/native-internals diagnostic. The now-concrete
    projection-cache question graduated to
    [Scanbeam TopX projection-cache experiment scope](issues/56-scanbeam-topx-cache-experiment-scope.md)
     and its [completed execution child](issues/57-scanbeam-topx-cache-experiment.md):
     candidate exactness and the full fresh 16-cell output matrix passed
     byte-for-byte in both modes, but paired timing measured the candidate
     **slower in 6 of 6 retained pairs** on the first cell (wall +2.14%, CPU
     +2.78%), so the campaign stopped by rule and the **DROP was accepted by
     the human on 2026-10-07**. The isolated candidate is preserved, not
     adopted. Permanent dependency adoption and alternative algorithmic shapes
     remain unspecified.
   [Serial host
  floor](issues/27-serial-host-prepass-floor.md)'s probe batches (items 1–5 +
  iteration 2) surfaced three graduate-when-probed questions resolved on
  2026-09-28 (phase-B = two costly offsets, sibling memo dropped for the matched job by human decision on 2026-09-29;
  estimate_print tail = pipeline teardown, below-fold; OverhangAnnotation
  wall = parallel) and one since graduated: the flat-bridge reduction inside
   `PrePass::Slice` graduated to
   [ticket 59](issues/59-prep-slice-flat-bridge-closing-reduction.md)
   (representation analysis first — the tested `refilled`-mask elimination
   failed real-layer IR equivalence and was dropped; fresh 2026-10-09
   anchors: `closing_ex` 384–389 s accumulated on base cells) and **is now
   resolved as a no-safe-reduction verdict (2026-10-14; see Decisions — the
   gate is artifact-bound and irreducible)**. The smaller
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
- **Named leads from the t60 host:native attribution (un-graduated,
  2026-10-14).** [host:native attribution](issues/60-host-native-attribution.md)
  names two reducible shapes beyond the now-resolved (KEEP 2026-10-16)
  [Region-preparation derivation sharing across region projections](issues/67-region-preparation-derivation-sharing.md):
  the **classic-perimeters per-layer host-service round trips** (8
  `offset_polygons` + 3 `clip_polygons` calls per layer; 89.26 s accumulated
  base-off and 85.83 s base-on vs 5.91–6.31 s benchy — the largest
  supports-off owner, whose reducible call sites need their own analysis) and
  the **`region_partition` 3×-per-layer re-run** (10.04 s accumulated base-off;
  dedup needs semantic analysis of what the post-process commit can change).
  Both graduate only on the human's route decision; the standing paired A/B
  governs any candidate.
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
- **Making the resolved-config-less fallback block satisfy Orca's viewer gate.**
  Ticket 34's residual — the test/legacy path's `CONFIG_BLOCK` cannot reach 80
  *loadable* pairs because the remainder are PnP's own key names — is past this
  destination: every measured production path passes a registry projection (233
  pairs, 161 loadable), so no cell's disclosure depends on it. Whether those
  entry points should synthesize a viewer block at all is a product decision, not
  route work; it returns only if the destination is redrawn.
