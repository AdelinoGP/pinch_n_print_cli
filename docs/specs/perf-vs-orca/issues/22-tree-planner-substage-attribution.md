# Tree-planner substage attribution

Type: task
Status: resolved
Blocked by: 12, 24
Assignee: wayfinder session (ses_f29984bdaffeT04YbEh81JtnVy), 2026-09-24 (evidence-correction resolution)

## Question

Split `com.core.tree-support-planner`'s wall time — the largest measured serial
module (92.9–96.6 s on base, 3.3 s on benchy, per the 2026-09-05 refresh) —
into cache/batched-host work, guest work, and surrounding host
validation/commit.

Prerequisite inside this ticket: add the missing instrumentation to the batched
host services — `offset_polygons_batch`, `clip_polygons_batch`, and
`simplify_polygon_batch` record nothing today (only the two mesh-query batches
push `batch_calls` audit entries). ADR-0049 measured the collision-cache build
at 98.0% of planner runtime (603 independent `offset_polygons` calls) — but
whether that is still the cost **at HEAD is unmeasured**, and the batched
services' own share has never had a timing hook.

Attribution runs only (no wall claims); instrumentation kept greppable and
removed or waived per the probe discipline before any commit.

Deliverable: the substage split with the batched-service share quantified, and
a recommendation for/against an algorithmic candidate — feeding [Gap budget per
cell](12-gap-budget-per-cell.md) for the prepass-dominated cells (base especially).

## Answer

Resolved 2026-09-24 (AFK agent session) — attribution only; no optimization
implemented or authorized. Evidence:
[`evidence/t22-planner-substage/FINDINGS.md`](../evidence/t22-planner-substage/FINDINGS.md),
raw probe lines in its [RAW-LINES.md](../evidence/t22-planner-substage/RAW-LINES.md),
the extraction tool [`t22_split.py`](../evidence/t22-planner-substage/t22_split.py),
and the full probe diff [`probe.patch`](../evidence/t22-planner-substage/probe.patch).
All probe code was `PERF-T22-PROBE`-tagged, removed, and the guest rebuilt
(freshness exit 0); `rg PERF-T22-PROBE crates modules` is empty.

**The premise is answered, and it is not the cache.** ADR-0049's collision-cache
lead does not survive to the matched job. On base at the matched job
(`pnp-classic-supports-on.json`, tree(auto)) the planner's own single dispatch
is **269.6 s per-object / 270.7 s module wall**, and its split is:

| term | base | benchy |
| --- | ---: | ---: |
| `TreeVolumes::new` (outline stack + below-union) | 0.52 s | 0.043 s |
| contact seeding | 2.18 s | 0.32 s |
| **top-down drop/move/MST loop** (incl. every lazy collision + avoidance ladder build — ADR-0049's "cache") | **5.25 s** | 0.41 s |
| — collision ladders | 0.76 s / 49 builds | 0.14 s / 27 |
| — avoidance ladders | 3.51 s / 38 builds | 0.28 s / 16 |
| F-14 drain + erase | 0.002 s | 0.0003 s |
| `smooth_nodes` | 0.12 s | 0.018 s |
| **emit pass (`draw_circles`)** | **262.0 s (97.2% of the planner)** | **9.48 s (92.6%)** |
| template stamp + 239c/239d interpolation | 0.033 s | 0.005 s |
| push entries | 0.00002 s | 0.000008 s |

The collision/avoidance cache is 4.27 s of 269.6 s — **1.6%**, not 98%. The
emit pass owns the planner, and its own sub-split closes at 99.4–99.7%:

| emit sub-term | base | benchy |
| --- | ---: | ---: |
| model-occupancy inflation (host offset per (layer, region)) | 61.4 s (23.4%) | 2.27 s (23.9%) |
| **per-region carve** (one host `clip_polygons` per drawn region) | **122.8 s (46.8%)** | **4.25 s (44.8%)** |
| `union_expolys` + simplify of the carved regions | 72.8 s (27.8%) | 2.43 s (25.6%) |
| post-simplify set-wide difference | 2.35 s (0.9%) | 0.27 s (2.8%) |
| per-node role classification | 1.50 s (0.6%) | 0.09 s (1.0%) |
| per-node ellipse construction | 0.55 s (0.2%) | 0.12 s (1.3%) |

**Host side of the stage (the batched-service share is now measured).** The
stage's host work is 194.0 s of `clip_polygons` over 171,636 calls (base) /
6.63 s over 29,000 (benchy), plus 65.1 s of singular `offset_polygons` over
23,535 calls, 0.51 s of batched offsets over 50 batches, **0 of
`clip_polygons_batch`, 0 of `simplify_polygon_batch`**, and a 23.8 s
`validate_entry` aggregation (every exact-Z query a miss: 0 hits / 431 misses;
21.1 s of `cross_section_at_z`). The batched host services are real but small
here — the planner's batch adoption covers only the lazy ladders — and the
clip path never adopted the batch form at all.

**Recommendation, for the gap budget: investigate the emit-pass carve, not the
cache.** The in-guest bbox pre-test counts 11,018 (benchy) / 78,832 (base)
disjoint calls **inside `carve_emitted_regions`**: 50.7% / 53.3% of carve
calls, or 38.0% / 45.9% of all stage singular clips. The host-side disjoint
class (12,237 / 79,369 calls; 1.78 / 52.59 s of core boolean-call wall) also
includes clips outside the carve, so it is **not** a measured gate saving.
Its net wall effect needs a probe-free A/B. The temporary
`t22_probe_diff_identity_tdd.rs` test observes that one disjoint rectangle
`Difference` preserves area but changes the contour representation; it does
not identify the changed attribute or prove general set equivalence. Returning
the input verbatim would *not* reproduce that baseline representation. Verify
downstream representation-insensitivity or reproduce the normalized output
before claiming the gate behaviour-preserving. `union_expolys` + simplify
(72.8 s base) is the second-ranked sub-term.

**What this feeds.** [Serial host floor](27-serial-host-prepass-floor.md) gets
its `PrePass::SupportGeometry` line item: the stage is 295.0 s of the **same
capture's 551.8 s instrumented base prepass**, and 270.7 s (91.8%) of the
stage is this planner's dispatch. On benchy the stage is 11.42 s of the
24.93 s prepass and 10.36 s (90.7%) is the planner. The 97.2% base / 92.6%
benchy figures above instead mean **emit pass ÷ per-object planner**. [Gap budget per
cell](12-gap-budget-per-cell.md)'s base supports-on binding cell needs the
support path to move; this ticket names where. No wall claim is made from these
instrumented numbers (§3.2). The candidate's own paired uninstrumented A/B
(timing chain position 2's acceptance slot) is the next step and was **not**
taken here.

## Answer — provenance and traps

- Capture conditions: `cargo xtask build-guests --check` exit 0 before the
  definitive runs; `--module-dir modules/core-modules`; matched-job config;
  `--instrument-stderr` only (no `--profile`); one probe build, one guest
  rebuild.
- The planner has no *direct* `slicer-core` dependency (its `Cargo.toml`);
  on wasm32 the SDK's polygon ops use host imports. The controlled accelerated
  perimeter cfg does not select a different guest carve algorithm; this is
  not a measured mode-timing equivalence or a waiver of paired acceptance.
- Base host terms differed materially between two captures with identical
  host-probe coverage but different guest bracket sets
  (`clip` 146.8 s vs 194.0 s; `exact_z` 27.4 s vs 21.1 s) while call counts and
  class shares were stable (46.2% disjoint in both; in-guest carve counts
  byte-identical). Load is a possible cause (§3.6/§10.3), not isolated by
  these runs. The tables quote the final bracket-set capture; the drift is
  disclosed rather than averaged away.
- The originally committed evidence mixed `base-def` guest terms with
  `base-final`, and `benchy-def` host/context values with `benchy-final`.
  `RAW-LINES.md` and the tables now quote the actual final captures, with
  the earlier base capture explicitly separated for drift comparison.
- The committed `probe.patch` passes `git apply --check --whitespace=nowarn`
  against the probe-free tree. It includes the newly created host module and
  temporary test; use the committed patch rather than relying on a stash to
  recreate the whole probe.
- `host::now_us` in the guest probe is the host clock for the module call; the
  planner is dispatched once per slice, so guest brackets are serial wall on
  one thread. The guest figures and host figures are different scopes and are
  never summed.

## Answer — evidence-correction resolution

Reopened and resolved 2026-09-24 after checking the retained `benchy-final`,
`base-final`, and `base-def` JSONL captures with `t22_split.py`. The preserved
probe and the headline emit-pass-vs-cache conclusion hold, but the original
write-up mixed two captures, attributed stage-wide disjoint clip counts and
wall to the carve, and used emit/planner percentages as planner/stage shares.
`RAW-LINES.md`, `FINDINGS.md`, the serial-host-floor line item, the map gist,
and the downstream carve-gate question now use the corrected scopes. The
committed raw excerpt includes the same-capture module, stage, prepass, and
slice events so the denominators can be checked without local scratch files.

Verification: the retained final-capture probe and event lines match their
local JSONL files; `git apply --check --whitespace=nowarn` accepts the committed
probe patch; `cargo xtask build-guests --check` exited 0; the probe-free tree's
`tree-support-planner` suite passed 137 tests; workspace all-target clippy
and `check-literals` passed; `check-test-quality --report` still reports 12
findings outside the changed files. A fresh `cargo build --workspace` did not
finish before its command timeout and is **not** claimed as passed. No
uninstrumented A/B or new geometry parity test was run; those remain the
separate [Emit-pass carve gate: representation safety + paired A/B](32-emit-carve-gate-representation-and-ab.md)
ticket's work.
