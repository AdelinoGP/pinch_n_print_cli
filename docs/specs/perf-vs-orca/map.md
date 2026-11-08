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

**Destination status (2026-11-07, [t124](issues/124-post-t123-orca-scoreboard.md)):
0 of 8 cells won.** PnP is **2.36x–4.65x** slower on the ordinary arm
(**2.40x–4.67x** accelerated) and needs a **58%–79%** wall cut to win,
**72%–86%** for a 1.5x margin.

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
- **The Orca baseline is RE-DERIVED (ticket 103, 2026-10-31 campaign).** The
  2026-10-27 OrcaSlicer **2.5.0** install (`v2.5.0-beta`) ships the vendor as one
  binary preset cache (`resources/profiles/BBL.opc`) and resolves `--load-settings`
  against the loaded preset bundle, so the rig's file-path form failed. The rig now
  loads a version-gated source-manifest vendor tree
  (`evidence/orca-rig-repair/prepare_orca_vendor.ps1` → `target/orca-vendor/`) and
  applies the matched keys as CLI overrides. The repair falsified the archived Orca
  side (its runs carried Orca's hardcoded filament defaults — density 0, max
  volumetric speed 2, first-layer temp 200), and [ticket 103](issues/103-orca-rebaseline-repaired-rig.md)
  has now re-measured **all eight cells on both arms today-vs-today**: **Orca wins
  all 8 at 3.52x–8.25x**, fresh Orca is **−24…−35 %** on benchy but **+6…+18 %** on
  base, and the t98 inference "every archived multiple is optimistic for PnP" is
  **falsified for the four base cells**. The 2.5 default flips (`enable_arc_fitting`,
  `elefant_foot_compensation` 0 → 0.15, `brim_width` 0 → 5) are **pinned back** in
  `New-OrcaOverrideArgs` and asserted per run, so no ratio may be quoted from before
  this take. Fresh Orca measurement is available again.

**Next-session handoff (e2e dev loop):** `docs/specs/perf-vs-orca/HANDOFF.md` —
the 755 s bucket's critical path (`module_plan` 373 s, `typed_projection`
345 s), ranked next moves, measurement protocol and traps.

**Lightning test-leg screen:** [ticket 120](issues/120-lightning-test-leg-concurrency.md)
parallelizes independent test legs with the existing Rayon pool. One warm,
shared-machine pair measured **798.61 s → 700.16 s** with the same 172 passed
and one ignored; active browser load means this is not an idle-box benchmark.
See its [evidence](evidence/t120-lightning-parallel-legs/FINDINGS.md) for the
conditions, correctness gates and limits. Ticket 119 remains open.

**Post-merge reconciliation (2026-11-04, ticket 115):** the merge `a5e9a403` is
**output-neutral on the matched job** — all 8 cells grow by exactly the 34-byte
line `; prepared_plan_diagnostic = none`, and a line diff on the four classic
cells confirms that is the only change — but it **regresses the per-layer
phase 1.9x–6.0x** (benchy 2,608 → 5,816 ms; base 15,341 → 92,418 ms) with
process CPU **+17…+54 % in 12/12 paired runs**; the prepass is unchanged
(70,662 → 70,463 ms, −0.3 %). Call counts are identical while small modules jump
to a near-uniform ~28.5 s accumulated (`fuzzy-skin` 162 → 56,068 ms), and the
per-call cost grows with layer index. The accelerated dist build does not show
it (2,318 ms per-layer on benchy classic-off vs ordinary 4,807–5,860 ms, same
hash), which points at a build-mode-sensitive host path. **No ratio of record
may be quoted from the pre-fix merged tree** — t103/t106 ratios and t104/t110
splits are pre-merge. The post-merge scoreboard is now recorded by
[ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md) at
**2.78x–5.11x ordinary** (accelerated 2.76x–5.10x) after the per-layer fix. See
[ticket 115](issues/115-post-merge-reconciliation.md)
and [`evidence/t115-post-merge/`](evidence/t115-post-merge/FINDINGS.md), and
[ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md).

Skills to consult: `diagnosing-bugs` for regressions, `debug-pipeline` and
`visual-debug` per the repo `AGENTS.md`; the repo test discipline (narrow runs,
`cargo xtask test`, guest freshness) governs any code change.

**Current selection scope (2026-11-07, twenty-ninth session — the occupancy
batch candidate):** the human explicitly selected
[Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md).
It was claimed and resolved as a measured **DROP**. The candidate batched only
the per-origin role-carve offsets inside the bounded emit flush; the per-layer
emit-check half stayed singular. **Output proof passed everywhere:** baseline
ordinary ≡ baseline accelerated ≡ candidate ordinary ≡ candidate accelerated ≡
the post-t123 set in 8/8 cells, with 32 output runs + 48 paired runs all clean
and zero exclusions. **The paired gate failed in 6/8 cells:** only benchy
arachne-on (+1.89 % wall, +0.5782 s CPU) and base arachne-on (+10.88 %,
+2.3125 s) passed. Base classic-on improved wall +6.13 % but CPU regressed
1.7656 s; benchy classic-on regressed wall −17.90 %. The candidate and both
WASMs were retired, the production diff is empty, and both freshness checks are
green again. Full planner tests: **148 passed**. Evidence:
[`evidence/t125-occupancy-offset-batching/`](evidence/t125-occupancy-offset-batching/FINDINGS.md)
(`verify.py` 442 checks, 10/10 negative controls rejected). **Frontier now: no
open child.** The per-layer/emit program remains the next stage's fog; the
unmeasured per-layer occupancy half is not an automatic rescue ticket.

**Previous selection scope (2026-11-07, twenty-eighth session — the Strong
planner carry ruling):** the human said "Continue the perf vs orca map" (no
ticket named). The frontier held only
[Strong planner carry: fold the occupancy batching into the Slice campaign, amend the gate, or park it?](issues/117-strong-planner-carry-ruling.md),
so it was claimed and resolved **with the human**. The original Slice-carry route
is stale: the landed Slice candidates are already in the baseline and cannot be
counted again.
The human **amended this representation-safe candidate's gate**: the 0.80 %
paired wall floor with CPU corroboration applies to the four supports-on cells;
all eight outputs must remain byte-identical; the four supports-off cells must
show no median wall or CPU regression. The **8/8 destination is unchanged**. The
candidate is graduated as
[Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md),
open and unclaimed. This was a ruling-only take: no production edit, build or
timing campaign. **Frontier now:**
[Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md)
is open and unclaimed. The per-layer/emit program remains the next stage's fog.

**Previous selection scope (2026-11-07, twenty-seventh session — the t124 take):**
the human said "Continue the perf vs orca map, pick up the next afk ticket" (no
ticket named). The frontier held **124 open and unclaimed** (117 is the human's
grilling), so 124 was claimed and **resolved as a measurement**. Headline: **the
post-t123 scoreboard is re-derived — PnP loses all eight cells at 2.36x–4.65x
ordinary / 2.40x–4.67x accelerated**, and the required cut to beat Orca is
**58%–79%** of today's PnP wall (**72%–86%** for a 1.5x margin). Every cell's
ratio improved against t121's pre-t123 **2.78x–5.11x**, but the move is a mix of
both arms (PnP ordinary medians **−25.7% … +0.7%**; Orca medians
**−12.3% … +11.5%**), so **no causal claim is taken from the cross-take
comparison** — t123's paired A/B remains the causal measurement. The base
supports-off PnP deltas (**−24.2% / −25.7%**) agree with t123's paired
**−22.2% / −20.9%**. **Proof:** 96 rows = 8 cells × 3 tools × (1 warmup + 3
measured), **zero starvation exclusions**, PnP ordinary ≡ accelerated bytes in
8/8, every PnP hash equal to the t123 candidate set, 64/64 PnP runs
`degraded=false` / `non_fatal=0`, 32/32 Orca runs pinned
(`enable_arc_fitting = disabled`, `elefant_foot_compensation = 0`,
`brim_width = 0`) with the repaired filament leg `1.24 / 12 / 220`. Both
freshness gates exit 0; the arms are the t123 candidate binaries
(`7737a74f…` / `a6ae2adc…`) and `git diff c598f098..HEAD` is docs-only.
**Disclosed:** the first batch was killed by the host mid-cell at 13:14 local
(79 of 96 runs); its `base classic-on` partial was discarded whole (named in
`discarded_partial_sources`) and re-taken in its own interleaved batch, and
`base arachne-on` followed in a second — each cell is internally paired, so no
ratio spans windows. See
[ticket 124](issues/124-post-t123-orca-scoreboard.md) and
[`evidence/t124-post-t123-scoreboard/`](evidence/t124-post-t123-scoreboard/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 10/10 negative controls
rejected). The standing adhesion asymmetry (PnP 8 mm brim vs pinned Orca 0) is
disclosed. **Frontier now: no open AFK child; 117 is the human's grilling.**
The per-layer/emit program remains the next stage and is unchanged by this
take.

**Previous selection scope (2026-11-06/07, twenty-sixth session — the t123 take):**
the human said "Continue the perf vs orca map, pick up the next afk ticket" (no
ticket named). The frontier held **123 open and unclaimed** (117 is the human's
grilling), so 123 was claimed and **resolved as a BUILD**. Headline: **retiring
the facet bridge path and classifying canonical's whole unsupported bottom set
is output-bounded and clears the 0.80 % floor in all eight cells.** The
PnP-local facet split — `classify_region_surfaces`' facet point-in-polygon scan,
`assemble_bridge_areas`' 1.0 mm `expansion_margin_mm` offset, the flat-family
special case, the postpass `gate_bridge_areas_by_unsupported_span`, and the
bottom-surface footprint cache — is replaced by
`classify_unsupported_bridge_areas` applying canonical
`PrintObject::detect_surfaces_type`'s
`opening_ex(diff(current_raw, safety_offset(previous_raw)), 0.1 x
external_perimeter_width)` to the region's whole unsupported set.
**16-cell output: met** — baseline ordinary ≡ baseline accelerated ≡ the
post-t116 set in all eight cells, candidate ordinary ≡ candidate accelerated,
candidate moves every cell; benchy grows **+873 / +832 / +1,281 / +1,159** bytes
and base shrinks **~20.3 KB** in every cell. **Delta audit: 0 structural
geometry lines in all eight cells** — every wall/support difference is
`E`-token-only on unchanged `G1` motion or a non-motion command (`M73`,
`M106`/`M107` fan, appendix); no coordinate, feedrate, travel, skirt, brim or
raft line moved. **Paired gate (3 interleaved pairs):** median wall **4.91 →
4.53 / 8.92 → 8.65 / 5.00 → 4.65 / 8.93 → 8.68 s** (benchy classic-off/on,
arachne-off/on) and **49.28 → 38.33 / 116.48 → 108.07 / 48.92 → 38.68 / 117.88
→ 108.52 s** (base), i.e. **−2.73 % … −22.23 %**, **3.4x–27.8x** the floor,
with process CPU **−1.0 … −110.2 s** in every cell; every cell is strictly
separated on wall and CPU. **Ledger:** the base win is mostly the removed facet
work (t122 measured the facet path at 27x the canonical classifier for a
1.04x set), and the benchy win is the same removal on a set that grows 1.85x;
the removed bottom-surface footprint is an extra saving the t122 screen did not
price. `DEV-200` records the unported wave expansion / zone trim; `DEV-039`
(the retired bounding-box fallback) is closed. See
[ticket 123](issues/123-facet-bridge-path-retirement.md) and
[`evidence/t123-facet-retirement/`](evidence/t123-facet-retirement/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 309 checks; `--self-test` rejects 11/11
negative controls). The now-stale scoreboard is graduated as
[124](issues/124-post-t123-orca-scoreboard.md): every t121 ratio predates this
take.
**Frontier now: 124 open and unclaimed; 117 is the human's grilling.** Standing
paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-06, twenty-fifth session — the t122 take):**
the human said "Continue the perf vs orca map" (no ticket named). The frontier
held **122 open and unclaimed** (117 is the human's grilling), so 122 was
claimed and **resolved**. Headline: **the bridge-expansion half is mis-staged
as a `PrePass::Slice` change and its cost consumes the classification win on
benchy; the facet-path retirement alone is a strong BUILD.** Canonical's
`LayerRegion::process_external_surfaces` expands bridges into `stInternalSolid`
/ `stInternal` / `stTop` after perimeters and fill surfaces exist; in PnP the
sparse zone and `perimeter.infill_areas` are not created until the
`Layer::Perimeters` commit partition, while `PrePass::Slice` has none of the
three zones. A faithful port is therefore a per-layer partition change, not a
prepass one. Measured (serial thread-time, probe): classification-only nets
**+1,122.9 ms** benchy / **+94,852.4 ms** base; the expansion costs **1,151.9 /
4,976.6 ms**, i.e. **102.6 %** of the benchy saving and 5.2 % of the base
saving. **Classification-only clears the 0.80 % floor in all eight cells at
12.5x–24.5x (benchy) and 89.7x–213.9x (base); the combined shape is negative in
all four benchy cells.** Delta is bounded on both fixtures: benchy added
618.44 / removed 18.45 mm², expansion reach ≤300.64 mm²; base added 564.17 /
removed 384.82 mm², reach ≤513.85 mm². **Ledger correction:** the current facet
path's pre-gate polygon sum is 14,232.05 mm² (benchy) / 419,106.22 mm² (base) —
at most 5.0 % / 1.1 % survives the gate, so on base the expensive facet path
produces an almost identical set (1.04x) at 27x the canonical classifier's
cost. The classification-only candidate is graduated as
[ticket 123](issues/123-facet-bridge-path-retirement.md). See
[ticket 122](issues/122-prep-slice-wave-expansion-facet-retirement.md),
[`evidence/t122-wave-expansion/`](evidence/t122-wave-expansion/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 14/14 negative controls rejected).
**Frontier now: 123 open and unclaimed; 117 is the human's grilling.** Standing
paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-05, twenty-fourth session — the t121 take):**
the human said "Continue" after the t119 resolution, then paused the Orca
campaign overnight and resumed it. **121 is resolved.** Headline: **the per-layer
transport was deriving prepared-region geometry for every module call, and for
`Layer::PerimetersPostProcess` it derived data the stage never consumes.** The
first is fixed by reusing the arena's `PreparedRegionData` when the
perimeter-source projection is the identity and every projected origin is
staged; the second by passing `derive_prepared = false` for that stage. **Four
classic cells, 3 interleaved pairs:** CPU **−27 % … −46 %** in **12/12 pairs**,
wall **−9.3 % … −37.5 %** in **11/12** (the exception is benchy-off pair 2 at
+0.66 % wall with −37.23 % CPU; the cell median is −14.29 %), and all four
G-code hashes identical pre/fix. **Per-layer bracket (base-off, instrumented):
7,265 pre-merge → 42,406 unfixed → 6,758 fixed**, below the pre-merge bracket.
Residual cost after the first fast path was isolated to `fuzzy-skin` and
`seam-placer` (the `PerimetersPostProcess` stage), which the stage-aware empty
view returned to their pre-merge aggregate. **Ledger correction:** the t115
`output_audit` hashes predate t116 (which moved every cell); the current
references are `35600ee4…` / `596f6da5…` / `6a6a3d9d…` / `8f46fe92…`, and the
fix is output-neutral against them. **Scoreboard (complete):** the
fresh post-merge Orca scoreboard is recorded — 8 cells × 3 tools × (1 warmup +
3 measured). PnP ordinary and accelerated G-code bytes are identical in **8/8**,
and PnP **loses every cell**: ordinary/Orca **2.78x–5.11x**, accelerated/Orca
**2.76x–5.10x** (best benchy arachne-off 2.78x, worst base classic-on 5.11x).
The destination's 8/8 is not met; the required cut is **64%–80%** of today's
PnP wall (76%–87% for a 1.5x margin). See
[ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md),
[`evidence/t121-per-layer-dispatch/`](evidence/t121-per-layer-dispatch/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 8/8 negative controls rejected) and the
[scoreboard](evidence/t121-per-layer-dispatch/SCOREBOARD.md)
(`verify_scoreboard.py --self-test` rejects 6/6).
**Frontier now: 122 open and unclaimed; 117 is the human's grilling.** Standing
paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-05, twenty-third session — the t119 take):**
the human said "continue the perf vs orca map, pick up the next afk ticket" (no
ticket named; **117** is the human's grilling, so **119** was the first AFK
frontier ticket in order). **119 is resolved.** Headline: **the lightning module
now observes each `(case, arm)` once — 41 → 32 active invocations — and the
serial module wall measured 688.89 s → 620.44 s (−9.94 %), with every
audit-listed assertion preserved.** A process-global `OnceLock<Corpus>` builds
five cases × two arms = 10 runs; `module_plan` re-checks both arms against the
recorded plan, `fresh_consumers` reuses the WASM arm for its emitted-segment
claims and keeps only its preparation + cancellation legs, and the legacy,
comparator, selection and retirement tests read the shared capture instead of
re-taking it. The runtime trace agrees entry by entry: 32
`lightning-invocation:` lines (`run_visual` 13, `run_visual_native` 5,
preparation 6, full slice 8). Audit fixes: the typed-projection scene-sharing
assertion now requests two layers (it was vacuous on one), the selection
test-local Boolean assertion is gone, `max_retained_after_disposal()` is
`Option<u64>` so cleanup cannot pass without a disposal, and the foreign-owner
block is a real cross-store `InvalidOwner` control; the zero-slice selection
predicate and store controls moved to a contract test. Disclosures: the box was
**not idle** (Opera/Stremio/Chrome, 43 % sampled), the arms are same-session
rather than interleaved, and a second after observation measured 611.64 s; the
saving is smaller than t114's pre-cache estimate because t118 had already made
each capture cheap. No full-bucket wall and no aggregate CPU claim. See
[ticket 119](issues/119-lightning-corpus-consolidation.md) and
[`evidence/t119-lightning-corpus/`](evidence/t119-lightning-corpus/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 7/7 negative controls rejected).
**Frontier now: 121 and 122 open and unclaimed; 117 is the human's grilling.**
Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-04/05, twenty-second session — the t116 take):**
the human said "continue the perf vs orca map, pick up the next afk ticket" (no
ticket named; **117** is the human's grilling, so **116** was the first AFK
frontier ticket in order), then paused the campaign overnight and resumed it.
**116 is resolved.** Headline: **the canonical flat-bridge rule is built, output-
verified and 8/8 above the floor.** The PnP-local enclosure closing
(`closing_ex(support, 12 mm, Miter)` + `≥ 10 %` refill) is replaced by canonical's
`opening_ex(flat_bridge, 0.1 × external_perimeter_width, Miter, 3.0)` over PnP's
flat family, with canonical's `ApplySafetyOffset` (1-unit lower inflate) and the
lower-layer guard (`lower_layer == nullptr` ⇒ `stBottom`); the
`flat_bridge_closing_join` key is retired end to end. **16-cell output: met** —
baseline ordinary ≡ baseline accelerated ≡ the t115 post-merge set, candidate
ordinary ≡ candidate accelerated, candidate moves every cell. **Delta stop rule:
not triggered** — `audit_delta.py` finds **0 structural geometry lines** in all
eight cells. **Paired gate: 8/8** at **11.1x–30.5x** the 0.80 % floor (median wall
−8.9 % … −24.4 %; CPU −16.6 … −288.9 s), narrowest base classic-on. Two ledger
corrections: the baseline check is against the **post-merge** set (the ticket's
post-t81 wording predates the merge), and after the key retirement **no declared
config key carries `wire-type: enum`**, so the scheduler manifest's enum
assertions retire too. Disclosures: the campaign was interrupted twice (session
recycle, then a requested overnight pause), so pairs 1–2 and pair 3 come from two
windows ~4 h apart; each pair is internally paired. No Orca arm — the post-merge
scoreboard stays [ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md)'s
closing measurement. See [ticket 116](issues/116-prep-slice-canonical-bridge-classification.md)
and [`evidence/t116-prep-slice-canonical-bridge/`](evidence/t116-prep-slice-canonical-bridge/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 316 checks; `--self-test` rejects 10/10
negative controls). **Frontier now: 119, 121 and 122 open and unclaimed; 117 is the
human's grilling.** Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-04, twenty-first session — the t115 take):**
the human said "continue the perf vs orca map" (no ticket named) and asked that
any uncommitted work be committed; the t120 take's six files were committed
first as `9a7371aa`. The frontier at HEAD `f70e8ba7` held four open children —
[115](issues/115-post-merge-reconciliation.md),
[116](issues/116-prep-slice-canonical-bridge-classification.md),
[117](issues/117-strong-planner-carry-ruling.md) (the human's grilling) and
[119](issues/119-lightning-corpus-consolidation.md) — so the first frontier
ticket in order, **115**, was claimed and resolved the same session. Headline:
**the merge is output-neutral and per-layer-expensive.** Every matched cell's
ordinary output grows by exactly **+34 bytes** (the added config line
`; prepared_plan_diagnostic = none`), and a direct diff on the four classic cells
returns that single hunk — no toolpath re-record is needed, only the hash set
moving (`22200671… → 4b6d2843…` on base classic-on). Painted fixtures do move:
`cube_4color.3mf` loses **1.08 %** of its bytes and redistributes up to
**8.19 %** of extrusion between tools while total extrusion moves **−0.11 %**,
consistent with the classic paint-attribution fix; the map quotes no painted
ratio, so nothing it says depends on it. The **performance** side is the real
finding: a paired pre-merge-vs-merged A/B (preserved t103 developer snapshot vs
the current ordinary build, 3 interleaved pairs, load 71 % disclosed) puts
**process CPU up in 12/12 pairs** (+24.1 / +17.0 / +53.6 / +34.9 % medians,
benchy-off / benchy-on / base-off / base-on) and the whole cost in the
**per-layer phase** (incrementally 2.23x / 1.91x / 6.02x / 5.08x; instrumented
base-off 7,265 → 42,886 ms = 5.90x with the prepass flat at −0.3 %). Module
brackets show the same 495 call counts with `fuzzy-skin` 162 → 56,068 ms and ten
more modules at a near-uniform ~28.5 s, and the per-call cost rises with layer
index (55 → 141 ms) — a cost proportional to committed state, not guest work.
The accelerated dist build is unaffected (2,318 ms per-layer on benchy-off vs
ordinary 4,807–5,860 ms, same hash), so a build-mode-sensitive host path is the
suspect. All structural keeps are present (t78/t81/t90/t95/t101/t102); their
prices were not re-measured because the regression dominates their phase. The
ruling: **no ratio may be quoted against the merged tree**, the post-merge Orca
scoreboard is deferred (not dropped) to the fix's closing gate, references move
to the post-merge set, and the regression is graduated as
[121](issues/121-post-merge-per-layer-dispatch-regression.md). Evidence:
[`evidence/t115-post-merge/`](evidence/t115-post-merge/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 403 checks, 6/6 negative
controls). Frontier now: **116, 119, 121** open and unclaimed; **117** is the
human's grilling. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-04, nineteenth session — the t114 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier at HEAD
`7eeff93e` held four open children — [114](issues/114-e2e-bucket-latency.md),
[115](issues/115-post-merge-reconciliation.md),
[116](issues/116-prep-slice-canonical-bridge-classification.md),
[117](issues/117-strong-planner-carry-ruling.md) — so the first frontier ticket in
order, **114**, was claimed and **resolved the same session as a Phase-1 screen**.
Headline: **the e2e bucket is 85 % fixed per-slice module load.** One debug slice
of `resources/calicat.stl` through the 24 core modules is **11.732 s** median; the
instrumented run puts **1.833 s** in the four pipeline phases and **10.011 s** in
module load + startup + emit. The loader builds a fresh `WasmEngine` and compiles
every on-disk component **serially inside the binding loop** in
`load_live_modules_for_plan_manifest_first`
(`crates/slicer-wasm-host/src/execution_plan_live.rs`) with **no cache**. Inventory
(serial, 178 tests covered): the non-whale modules sum to **854 s (14.2 min)**,
`lightning_migration_tdd` measured **1,411 s for 6 of its 10 tests** (~39 min
extrapolated) and `preparation_runtime_lifecycle_tdd` is the ticket's recorded
**14-of-16-in-30-min** module (~34 min extrapolated) — the two whales are **83.8 %**
of the **5,263 s (≈88 min)** extrapolated suite, whose measured/cited floor is
**4,065 s (≈68 min)**. The counter-example is `slice_end_to_end_tdd`: 42 tests in
**201 s** because **~10 `pnp_cli` runs** back all of them (23 `cached_run` call
sites over 3 keys + 7 uncached determinism runs). A reverted 24-thread
compile probe (same G-code, `d12cdfd1…`) removed only **2.431 s (−20.7 %)** — the
compile already saturates the cores per component, so **parallelising the loop is
not the lever**. **Pruning: the first pass's "none justified" was too strong.** A
five-agent adversarial audit (postscript in the ticket, tables in
[`evidence/t114-e2e-latency/AUDIT.md`](evidence/t114-e2e-latency/AUDIT.md)) found
decorative, fake, success-only and never-running tests plus **~11 min** of
repeated observations (423 s lightning + 120 s prep + ~95 s small-module merges
+ 12 s precision). Eight claim fixes/merges were adopted — a dead disk
support-blocker test revived, both test-local `scenario_2` fakes deleted (the one
real `PaintSemantic` order claim moved to `slicer-ir`), a lying override test
renamed, an empty-tolerant pipeline test tightened, and
`modifier_infill_boundary_anchoring` merged into its identical sibling
(**38.76 s → 26.78 s, −11.98 s**); the precision merge removed a slice but landed
inside noise (**86.66 → 86.71 s**, not claimed). The ~**237 pipeline starts**
still dominate: at 10.0 s each that is **~39.5 min of pure loading**, so
deduplicating all ~11 min still leaves ~77 min — the cache remains the carrying
fix. **Lane split: not viable** — a fast lane would have to exclude every
slice-running module, leaving ≈ 22 s of pure-unit/model-IO tests. **Concurrency:**
`libtest`'s 12 threads oversubscribe a box each test already saturates (one
`lightning` test process used **6.8 cores**); `mm_painted_fixture_t0_t1` measured
**23.25 s alone vs 200.622 s** under 12 concurrent processes, an **upper bound**
because a game held ~6.6 cores during that arm. `cargo nextest` 0.9.151 is **not
adopted** (process-per-test discards the in-process `slicer_cache`/`wasm_cache`
sharing). **Ledger correction:** the 40-minute baseline was **under-counted** —
nine `pnp_cli`-backed e2e tests fail in 0.1 s on the stale-binary gate under plain
`cargo test` (which does not build `pnp_cli`), so their 12–29 s each were absent
from the 93-passed observation; `cargo xtask test` owns that preflight
(`ensure_pnp_cli_fresh`). The carry decision was graduated as
[118](issues/118-compiled-component-cache.md) and **executed the same session**
after the human set a steep milestone: the process-global component cache,
wasmtime's validated disk cache, per-layer capture parallelism and corpus-case
parallelism bring the full bucket to **755.12 s (12.59 min), 172 passed / 1
ignored — under the 15-minute gate** (t114's ≈4.5 min model was optimistic; the
lightning captures do 38–52 s of real pipeline work each). Evidence:
[`evidence/t114-e2e-latency/`](evidence/t114-e2e-latency/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 29/29 checks; `--self-test` rejects 11/11
negative controls). Frontier now: **115, 116, 119** open and unclaimed, **117**
is the human's grilling; [119](issues/119-lightning-corpus-consolidation.md) owns
the remaining lightning corpus consolidation (**41 → 32** active invocations, all
assertions preserved) — the bucket already clears its gate without it, so it is
claim hygiene rather than latency. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-04, eighteenth session — the t113 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier at HEAD
`19a5edfe` held four open children — [113](issues/113-strong-planner-phase1.md)
(blocked by the already-resolved 112), [114](issues/114-e2e-bucket-latency.md),
[115](issues/115-post-merge-reconciliation.md),
[116](issues/116-prep-slice-canonical-bridge-classification.md) — so the first
frontier ticket in order, **113**, was claimed and **resolved in the same
session as a Phase-1 stop**. Headline: the planner's largest term is **model-occupancy
inflation**, not the clip core. At HEAD the `com.core.tree-support-planner`
module is **58,498 ms** of a **200,099 ms** base classic-on cell (**3,279 ms** of
**15,333 ms** benchy-on; **84,337 ms** of **416,375 ms** base arachne-on;
**4,133 ms** of **27,122 ms** benchy arachne-on). Its split is **56.2 % / 46.0 %
/ 51.4 % / 41.4 %** `inflate_model_occupancy`, **31.0 % / 28.6 % / 35.9 % /
32.4 %** clip core, **6.2 % / 10.8 % / 6.0 % / 11.1 %** drop loop, **2.4 % /
6.0 % / 2.2 % / 5.4 %** contacts (order: base classic, benchy classic, base
arachne, benchy arachne). The occupancy inflation is **one singular
`host::offset_polygons` per call**, split evenly between the per-layer and
per-origin sites; batching it through `offset_polygons_batch` is exact by
construction (same `slicer_core::polygon_ops::offset`, input-order results;
t65/t78 precedent), needs **no canonical-parity exception**, and clears the
**0.80 %** floor on all four supports-on cells at **12.58x / 7.44x / 7.97x /
4.76x** the floor under t78's measured **2.6x** clamp (18.73x / 11.08x / 11.87x
/ 7.09x ideal), with **zero** expected output delta (the probe is
byte-identical to the HEAD baseline on base classic-on `4b6d2843…`, benchy
classic-on `3b8fa86a…`, benchy arachne-on `d4644c14…`). **But the all-8-cell
gate is structural:** the whole `PrePass::SupportGeometry` stage on the
supports-off cells is **300 ms = 0.22 %** of base classic-off and **46 ms =
0.51 %** of benchy classic-off, below the floor even if deleted, and the planner
module alone is **141 / 23 ms = 0.13x / 0.32x**. No planner-only or
support-stage shape can clear 0.80 % on those cells. The stop rule fired: no
candidate ticket, no production edit. The carry decision — fold the batch into
[116](issues/116-prep-slice-canonical-bridge-classification.md)'s Slice
campaign, amend the planner gate to the four supports-on cells with a
no-regression bound, or park it — is graduated as
[117](issues/117-strong-planner-carry-ruling.md) (open, unclaimed, unblocked;
HITL grilling). **Ledger correction:** t88/t107's "no host batch shape in the
non-clip residual" is falsified — the avoidance ladder, offsets, MST and node
moves are **6.2 % / 10.8 %** of the planner, while the occupancy offset batch is
**56.2 % / 46.0 %**. The `87,824 ms` t104 figure is an instrumented bracket; its
composition (≈49.3 s occupancy, ≈27.2 s clip, ≈5.5 s drop) is the transferable
result, not its absolute. Evidence:
[`evidence/t113-strong-planner-phase1/`](evidence/t113-strong-planner-phase1/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).
Frontier now: **114, 115, 116, 117** open and unclaimed; 116 is takeable and 117
is the human's grilling. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-03, seventeenth session — the t112 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier at HEAD
`2dade816` held four open children — [112](issues/112-prep-slice-parity-phase1.md),
[113](issues/113-strong-planner-phase1.md) (blocked by 112),
[114](issues/114-e2e-bucket-latency.md), [115](issues/115-post-merge-reconciliation.md)
— so the first frontier ticket in order, **112**, was claimed and **resolved the
same day as build-P1**. Headline: **canonical OrcaSlicer has no enclosure closing.**
`PrintObject::detect_surfaces_type` labels *every* bottom area the layer below does
not cover as `stBottomBridge` — `opening_ex(diff_ex(slices,
safety_offset(lower_slices), Yes), external_perimeter_width / 10)` — and the only
canonical bridge closing is `merge_bridges`' `closing_ex` at **≈0.15–0.17 mm**
(0.55 × 0.65 × 1.05 × solid-infill spacing) on the *bridge ∪ wave-anchor* surface,
a hole filler. PnP's 12 mm `closing_ex(support)` + **≥ 10 %** refill gate is a
PnP-local discriminator with **no canonical counterpart and no `DEV` row**
(**~70–80×** canonical's radius, different operand, different consumer), so no
shape can preserve its verdicts *and* claim parity: the parity move is to **replace**
the gate. Measured at HEAD: the closing is **86.8 %** (benchy) / **70.8 %** (base) of
`PrePass::Slice`'s accumulated thread-time, and shape **P1** (canonical opening on
PnP's flat family, every surviving component kept) has a net ceiling of
**1,893 / 37,682 ms** = **15.3–24.7 % / 23.4–50.0 %** of the cell = **19.1x–62.5x**
the **0.80 %** floor in **all eight** cells, with the replacement opening
**146x / 853x** cheaper than the closing. Delta, measured on both fixtures and
audited per layer: **182 / 239** benchy and **437 / 469** base committed layers
change; net **+8.86 mm² (+2.0 %)** / **+115.90 mm² (+35.9 %)** of flat bridge area —
the gate rejected **68.4 / 187.2 mm²** that canonical calls bridges while the
0.04 mm opening removed **59.58 / 71.27 mm²** of sub-0.08 mm features. The first
layer also pays a 12 mm closing whose result `gate_bridge_areas_by_unsupported_span`
clears (**595.8 ms** benchy, 1.1 ms base). **P2** (full canonical classification,
retiring the facet-derived `assemble_bridge_areas`) is **deferred**: its delta is
unbounded until canonical's wave expansion (`propagate_waves_ex`) is ported, and
PnP's fixed 1.0 mm `expansion_margin_mm` is not it. **P3** (radius swap on the
existing gate) is **rejected** on correctness. The candidate is graduated as
[116](issues/116-prep-slice-canonical-bridge-classification.md) (open, unclaimed,
unblocked) under t111's 16-cell contract. Evidence:
[`evidence/t112-prep-slice-parity/`](evidence/t112-prep-slice-parity/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 9/9 negative controls rejected).
Frontier now: **113, 114, 115, 116** open and unclaimed — 113's blocker (112) is
resolved, so it is takeable. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-03, sixteenth session — the prepass-rewrite
ruling):** the human said "continue the perf vs orca map" (no ticket named). The
frontier re-derivation at HEAD `f4a73892` found **110/110 children resolved — no
open child, no claim** — so the fog routes were offered and the human selected
**rule on the prepass rewrite**, graduated as
[111](issues/111-prepass-rewrite-scope-ruling.md), claimed and **resolved the
same day**. The ruling **authorizes a scoped canonical-parity exception for the
prepass**: both carrying shapes are covered, **`PrePass::Slice` is the first
campaign** (41,472 / 44,558 ms; 51.2 % of base classic-off, 19.5 % of base
classic-on) and the Strong planner is second (87,824 ms; 38.4 % of base
classic-on), graduated as [113](issues/113-strong-planner-phase1.md) and blocked
by 112. Candidate acceptance needs a fresh **16-cell** reference set, arms
byte-identical within themselves, the baseline reproducing the post-t81 set, the
candidate-vs-old-reference G-code delta audited before timing, and the standing
paired wall/CPU gate at the **0.80 %** floor in **all 8 cells**. The first step
is a **Phase-1 parity analysis**, graduated as
[112](issues/112-prep-slice-parity-phase1.md) (open, unclaimed); no production
edit happens before it clears the floor with a bounded delta. The rewrite is
**stage 1** of a multi-stage engine program — deleting both largest prepass
stages still leaves base at **2.86x–4.00x** inside t104's instrumented
arithmetic, so the per-layer/emit program is the next stage's fog. One ledger
caveat is carried into the ticket and remains a fog note: t110's base-on
**9.49x / 16.3x / 19.6x / 13.7x** figures divide an **instrumented** PnP span by
Orca's quiet wall; t104 discloses that pass at **+43.6 %** over its t103 median
and the uninstrumented paired record is **6.02x** (t106), so those base-on
multiples are upper bounds until corrected.

**Post-merge directive (2026-11-03, after master merge `a5e9a403`):** the human
directed that the two archive-dependent `preparation_capability_pilot_tdd` tests
be deleted — done in `69ef50f3`, because the gitignored pre-edit archive
`target/preparation-pilot/plain-before.component.wasm` cannot exist on a fresh
checkout and the current guest cannot reproduce its recorded SHA; the remaining
nine pilot tests pass — and that two tickets be opened from the merge
validation. [114](issues/114-e2e-bucket-latency.md) owns the **e2e-bucket
execution latency**: the full `slicer-runtime` e2e suite exceeds **40 minutes**
and is the dev-loop long pole, so this continues t108/t109's work on the
**execution** side rather than the compile side. The human also ruled that
**cutting/deleting e2e tests is fully in scope** on 114; the ticket now carries
a pruning pass with a per-deletion coverage map rather than a no-deletion
constraint. [115](issues/115-post-merge-reconciliation.md)
owns **post-merge reconciliation**: re-baseline the perf keeps, audit master's
semantic drift, the classic paint-attribution output change, and the
config/disclosure deltas before the map quotes another ratio. Both are open and
unclaimed; the 16th session does not claim either.

**Previous selection scope (2026-11-03, fifteenth session — the t110 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `1e72d9e3` found **109/109 children resolved — no open child,
no claim** — so six fog routes were offered and the human selected **Orca-side
attribution**, graduated as [110](issues/110-orca-side-attribution.md), claimed and
**resolved the same day as measurement-only**. Headline: **no per-stage timing
channel works on this install** (the `--logfile` CLI log buffers — 27 stage markers
arrive in **3 bursts**, 18 of 27 together, and carries no timestamps at `--debug`
3/4/5; ETW is refused without elevation — "Failed to enable the policy to profile
system performance" / "NT Kernel Logger: Invalid flags (0x3ec)"; `--pipe` accepted
the flag (`Will use pipe …`) but never connected to a listening server in two
attempts), so the finest observable boundary is a **~15 ms directory tap** on
`plate_1.gcode.tmp`. What it measures reframes the map's headline: **Orca's base
cells are import-dominated — 73.3 % / 42.0 % of wall is startup + model import
(10.13 s on the 123 MB `base.stl` against PnP's ≤ 0.708 s, ≥ 14.3x)** — and its whole
slicing phase is only **0.64 / 1.28 / 3.68 / 14.01 s** (benchy-off / benchy-on /
base-off / base-on) against PnP's **7.37 / 11.78 / 80.22 / 228.32 s**, i.e.
**11.6x / 9.2x / 21.8x / 16.3x**. The ratios of record (5.86x / 9.49x / 4.34x / 4.96x
today) therefore **understate the engine gap by 1.7x–3.7x**: PnP's fast import has been
offsetting part of a much larger slicing deficit, and PnP's **per-layer phase alone**
(1.54 / 1.74 / 6.54 / 12.70 s) is larger than Orca's **entire slicing**
(0.64 / 1.28 / 3.68 / 14.01 s) in three of the four cells. The prepass is the carrying phase
on **both** engines (Orca 57.1 % / 74.6 % of its base slicing; PnP 89.5 % / 89.4 %),
so the destination's prepass-rewrite ruling is not a PnP phase-mix artefact; on the
t105-pinned supports-on job Orca's prepass grows **+8.34 s** where PnP's support
stages add **~113.9 s** (**13.7x** on the increment; **8.4x** for the planner alone
against Orca's whole supports-on prepass). The feature stack is nearly
free on Orca's side — 1 wall loop, 1 shell, 0 % infill and no support still costs
**13,639.6 ms** against the full cell's 13,814.5 ms — which kills the "Orca does less
prepass work" explanation. **Ledger correction:** t104's Table 3 compares against
"Orca's whole slice (2.03 / 2.29 / 15.28 / 19.49 s)", which are *process* walls that
are 66–73 % import on base; t104's shares and counterfactuals stand, only that
reading changes. Evidence:
[`evidence/t110-orca-phases/`](evidence/t110-orca-phases/FINDINGS.md) (`verify.py`
exits 0 checkout-only, 7/7 negative controls rejected). Frontier now: **no open
child**; the destination's prepass-rewrite decision is unchanged and still the
human's to rule on, now with the engine multiple measured. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-02, fourteenth session — the t109 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `59b1b21f` found **108/108 children resolved — no open
child, no claim** — so six fog routes were offered and the human selected **fix
the two harness defects** recorded by t108, graduated as
[109](issues/109-xtask-host-algos-injection-and-narrow-run-example.md), claimed
and **resolved the same day**. Headline: `xtask test`'s `--features
slicer-core/host-algos` injection is now conditional on cargo's selection
including `slicer-core`, so `cargo xtask test -p xtask --summary` exits **0**
with **193 passed** where it previously exited **101** with
`the package 'xtask' does not contain this feature: slicer-core/host-algos`;
the same error had blocked `slicer-config`, `slicer-ir`, `slicer-scheduler`,
`slicer-gcode` and `slicer-helpers`. The injection still fires for `-p
slicer-core`, for the workspace and for a `--manifest-path` naming
`slicer-core` (proof: the `slicer-core` arm compiles with the feature; a
`slicer-config` arm now reaches its tests). `AGENTS.md`'s narrow example now
names the module's real home — `cargo test -p slicer-scheduler --test
scheduler_contract core_module_ir_access_contract_tdd` — and runs **9 tests, 9
passed**, where the old command ran **0 passed; 314 filtered out**. The fix is
a pure `build_test_arguments` plus a selector parser covering
`-p`/`--package`, `--workspace`/`--all`, `--exclude`, `--manifest-path`,
attached forms and `name@version`, with five unit tests. **Two adjacent
findings recorded, not fixed:** `slicer-config`'s
`real_toml_manifest_census_rederives_expected_scope` is red **before and after**
(**293 vs 281**, reproduced with plain cargo; the fix only makes it reachable),
and `--no-run --summary` prints a build-may-have-failed line beside `VERDICT:
PASS`. Evidence: [`evidence/t109-harness-defects/`](evidence/t109-harness-defects/FINDINGS.md)
(`verify.py` exits 0 checkout-only, with cargo and `--full`; 5/5 negative
controls rejected). Frontier now: **no open child**; the destination's
prepass-rewrite decision is unchanged and still the human's to rule on, and the
dev-loop axis still has its five unpriced levers (cold-build `debug`,
cold-build linker, `cargo-nextest`, `sccache`, CI). Standing paired acceptance
and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-02, thirteenth session — the t108 take):** the
human said "continue the perf vs orca map, create and complete a ticket focused
on improving Testing and Compilation speed", quoting a discussion of Rust
iteration-speed practice (ra-jit/Blaze, mold/lld, sccache/rust-cache,
cargo-nextest, selective `opt-level`, CI caching, agent-loop discipline). That is
a **human-directed scope extension**: this map's destination is slice
throughput, and the dev loop's compile/test latency is now an axis of the effort
by explicit ruling. [108](issues/108-dev-loop-latency-preflight-concurrency.md)
was graduated, claimed and **resolved the same day as KEEP**. Headline: the three
`xtask test` preflights were strictly serial read-only scans; they now run
concurrently, and a representative invocation falls from a **median 18.771 s to
10.802 s** (**−7.969 s, −42.4 %**, four interleaved pairs with frozen harness
binaries; the slowest after sample 13.214 s is faster than the fastest before
sample 18.438 s; the pre-declared floor was **5 s**). Reporting order is
preserved, so a literals or test-quality failure still aborts before a guest
rebuild, and the freshness check keeps the single-toolchain-probe `Invocation`
on the calling thread. The inventory is the rest of the output: warm no-op
check/clippy 0.8–2.8 s, a one-line edit to `slicer-ir` / `slicer-core` 65.3 / 72.0 s,
the first codegen of the `contract` bucket **994 s** against **17.26 s**
incremental, relink-only 12.8 s, bucket execution 99.8 s, and a 89.7 MB contract
binary beside a 770 MB PDB. **Four premises falsified:** check/clippy *do* share
artifacts (1.7–2.4 s alternation, zero re-checks); feature sets do *not* thrash
for `slicer-runtime` (0.72 s no-op); the linker is *not* the incremental
bottleneck; and `debug = 0` is *slower* on the incremental path (28.24 vs
17.26 s). **Two adjacent harness defects recorded, not fixed:** the injected
`--features slicer-core/host-algos` makes `cargo xtask test -p <pkg>` fail for
every package that does not depend on `slicer-core` (A1), and `AGENTS.md`'s
narrow example filter `core_module_ir_access_contract_tdd` matches **no test**
(A2). **Blaze (ra-jit) was tried, not datasheeted:** #1527 ships no Windows
archive (packaging only, installer still unlanded), and #1591 builds here in
**57m34s** and starts a daemon, but needs two manual workarounds (`hard_link`
stand-ins read with `read_link` → `os error 4390` on every later start; a default
profile ACL refused by the store's `Access::Alone` check) and then refuses a
`serde` + `serde_json` binary at **241 std instances that need the LLVM backend**
(the author's LLVM build stops at `f16`/`f128` on Windows x64). Its LLVM rebuild
was abandoned after **92 minutes**. **On WSL2 it runs and loses to cargo on this repository:** against cargo 1.96.0 in Ubuntu 24.04 (12 CPUs), blaze's warm edit is **23-33 ms vs 248-779 ms** on a toy crate, but on `slicer-helpers` cold build is **193 s vs 54 s**, warm edit **12.4 s vs 0.95-1.13 s** (crate root), **11.9 s vs 1.4-3.3 s** (module) and **15.8 s vs 2.4-3.5 s** (function body), and `blaze test` is **312 s vs 49 s** (all 18 tests pass under both). **No blaze speedup is claimed for this repo.** Not priced and left as fog:
cold-build `debug` level, cold-build linker, `cargo-nextest`, `sccache`, and the
CI job-graph/caching half. Evidence:
[`evidence/t108-dev-loop-latency/`](evidence/t108-dev-loop-latency/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 33/33 claims, 12/12 negative controls).
Frontier now: **no open child**; the destination's prepass-rewrite decision is
unchanged and still the human's to rule on, and the dev-loop axis has its own
unpriced fog. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-11-02, twelfth session — the t107 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `9a1921ac` found **106/106 children resolved — no open
child, no claim** — so five fog routes were offered and the human selected
**screen `PrePass::SupportGeometry`**, graduated as
[107](issues/107-support-geometry-producer-decomposition.md), claimed, and
**resolved the same day as measured-dead — the premise named the wrong stage.**
Headline: `PrePass::SupportGeometry` is **two stages sharing one id**, and
ticket 104's reducer summed them. Re-pairing t104's own committed raw streams
gives a **host producer of 9 ms on base classic-on and 0 ms on benchy classic-on**
(**0.0068x** and **0.00x** the 0.80 % floor) against a **planner module stage of
91,893 ms = 40.13 %** and **3,260 ms = 26.17 %** of their cells — the tree
planner **87,824 ms (95.6 %)** + the traditional planner 300 ms on base, and
2,943 + 30 ms on benchy. The producer's whole job is a coarse outline set —
**577 polygons / 444,259 points over 240 layers (~7.1 MB)** on benchy-on, per
`support-preview` — so its sub-millisecond-to-9-ms bracket is real work, and the
sentinel intermediate entries sit inside the same 0–9 ms. **No shape on the
producer can clear the floor**, so the ticket's stop rule fired before any
candidate pricing: no candidate, no campaign, no production edit, no probe build
(a probe patch was authored and reverted unbuilt once the bracket split
answered the question). The take's real output is a **ledger correction**:
t104's 91,902 / 3,260 ms is essentially the **planner**, i.e. t88's 55.5 s term
(implied **64.5 s / 3.25 s** today) that the map already carries as fog — the
supports-on lever is unchanged and the producer was never part of it. Evidence:
[`evidence/t107-support-geometry-producer/`](evidence/t107-support-geometry-producer/FINDINGS.md)
(`verify.py` exits 0 checkout-only, 71/71 claims, 9/9 negative controls). Frontier
now: **no open child**; the destination's prepass-rewrite decision is unchanged
and still the human's to rule on, with the supports-on carrying shape now
correctly named as the planner. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-02, eleventh session — the t106 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `e2294453` found **105 of 106 children resolved** — ticket
106 open and unclaimed, exactly as the tenth-session note predicted — so it was
claimed and worked. Headline: **post-pin PnP ratios are 6.02x / 6.11x (base classic /
arachne) and 4.77x / 4.53x (benchy classic / arachne)**, every post-pin Orca
median is slower than ticket 103's pre-pin one (**+37.0 / +14.5 / +13.7 / +5.7 %**),
and the park still needs **66.9–75.5 %** of each cell removed. The t105 transfer
**agreed on two cells and missed on two** (base arachne-on **−17.95 %**, benchy
classic-on **+3.89 %** against a **0.093 s** tolerance); the pinned job is
**byte-identical to t105's `tree_strong` arm** on all four cells yet **1.9–9.6 %
faster today**, so the transfer's error is two cross-day drifts rather than the
pin, and it is retired as an absolute predictor. The take ran quiet (load
16.4 % → 3.3 %, no starvation exclusions) after an unrelated game process and a
**sibling checkout's** `cargo test` e2e binary had exited. Frontier now: **no
open child**; the destination's prepass-rewrite decision is unchanged and still
the human's to rule on. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-02, tenth session — the t105 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `81219a66` found **104/104 children resolved — no open
child, no claim** — so five routes were offered from t104's post-resolution
state (closing-chain rewrite shape; supports-on engine fairness; redraw the
destination; park the map; lift the park and harvest per-layer leads) and the
human selected **supports-on engine fairness**, graduated as
[105](issues/105-supports-on-engine-style-fairness.md). The take measured
Orca's organic vs `tree_strong` arm on the four supports-on cells (1 warmup + 4
interleaved paired rounds; the campaign was paused at the human's request and
resumed) and the human ruled **pin to `tree_strong`**. Headline: the pin costs
Orca **+39.6 / +44.4 %** wall on the two base cells and **+7.6 / +9.5 %** on the
two benchy cells, moves the four supports-on ratios by **−7.1…−30.8 %** (best
supported-on cell still ~4.3x, park still needs 65–73 % cuts), and changes the
job (**+23.5–35.3 %** bytes). The pin is implemented as the standing rig's
default (supports-on only, per-run appendix assertion); **t103's supports-on
Orca medians are pre-pin**, and the re-derivation is graduated as
[106](issues/106-post-pin-supports-on-scoreboard.md). Frontier now: **ticket 106
only, open and unclaimed**. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-11-02, ninth session — the t104 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `e61c5655` found **103/103 children resolved — no open
child, no claim** — so six fog routes were offered (prepass-vs-per-layer
attribution; supports-on fairness re-run; Orca-side attribution; `apply_opening`
round-join parity; `region_partition` dedup; re-aim the destination; adhesion
asymmetry) and the human selected **prepass-vs-per-layer attribution**, graduated
as [104](issues/104-prepass-vs-per-layer-attribution.md), claimed, and
**resolved the same day as measurement-only**. Headline: **the wall is the
prepass** — **68.5–89.5 %** of every classic cell, against **7.9–20.1 %** for the
per-layer pipeline — so the park's 57–82 % cut maps onto removing **77.8–93.9 %
of the prepass**; deleting the *entire* per-layer phase never reaches the park
(**3.02x / 4.51x / 4.66x / 7.60x** remain, benchy-off / base-off / benchy-on /
base-on) while deleting the entire prepass would (**0.52x / 0.87x / 1.19x /
1.25x**), and the **two biggest prepass stages together still leave 1.55x–3.34x**
(closest: base classic-off at **1.55x**, 3 % short). `PrePass::Slice` is the
single largest measured term — **41,472 ms = 51.2 %** of base classic-off
(28.4 / 18.1 / 19.5 % of the other three) — and on the supports-on cells
`PrePass::SupportGeometry` dominates (**40.1 %** of base classic-on, 26.2 % of
benchy classic-on; with `SupportAnalysis` ~50 % of that cell), the `DEV-156`
style-substitution site — **corrected 2026-11-02 by
[ticket 107](issues/107-support-geometry-producer-decomposition.md): that 40.1 %
is the planner module stage that shares the stage id, not the host producer
(9 ms); read this row as producer + planner.** **Ledger corrections:** t73's 2026-10-18 base-off split
(prepass 115.0 s / per-layer **94.3 s** of a 214.5 s instrumented wall) is
superseded — the per-layer phase is now **8.7 %** (7,016 ms of 80,924 ms), and
the change happened between t73 and t91 with **no per-KEEP decomposition
claimed** — while t91's 2026-10-27 base-on prepass share (149,723 / 168,319 =
**89.0 %**) re-measures at **89.4 %**, so the split's semantics are unchanged.
Every per-layer fog entry is now bounded by that 6.5–12.7 s phase. **Walls are
not claimed**: all four uninstrumented sets ran under external load (a sibling
checkout's integration-test binary, then `clippy-driver` + two browser
processes), inflation **1.32x–1.80x**, load falling 95.6 % → 16.8 % mid-batch; the
human ruled "proceed with noise disclosed"; ratios of record are t103's paired
quiet medians and shares are one instrumented pass per cell, clean and
byte-identical to the t87 references, with a benchy repeat moving no share by
more than **5.92 pp**. Arachne unmeasured. No candidate, no campaign, no
production edit; `verify.py --self-test` exits 0 checkout-only (70/70 claims,
9/9 negative controls). Frontier now: **no open child**; the destination has
become an explicit **prepass-rewrite decision** whose two named shapes (the slice
closing's consumer, and the support-engine style question) are the human's to
rule on. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-31 → 2026-11-01, eighth session — the ticket
103 take):** the human said "continue the perf vs orca map" (no ticket named). The
frontier re-derivation at HEAD `c135e3e7` found exactly one open child,
[103](issues/103-orca-rebaseline-repaired-rig.md), which was claimed and is
**resolved (2026-10-31 campaign / 2026-11-01 disposal)**. The rig now carries the
ruling's three pins in `New-OrcaOverrideArgs`
(`evidence/matched-pair/run_scoreboard.ps1`) and `Test-OrcaEvidence` asserts each
resolved value per run (`enable_arc_fitting = disabled`,
`elefant_foot_compensation = 0`, `brim_width = 0`) plus the filament leg
(`1.24 / 12 / 220`), rejecting the archived hardcoded-default signature
(`0 / 2 / 200`) outright — all **32 Orca runs** resolved the six values, so a
future install fails closed instead of drifting. **Both arms were rebuilt from
HEAD and measured today-vs-today** (ordinary `4cb96942…`, accelerated
`80897e62…`, 24/24 external modules, both freshness gates exit 0; vendor stamp
installed `2.8.0.16` == upstream `02.08.00.16`).

**Headline: Orca still wins all 8 cells at 3.52x–8.25x** — benchy classic-off
**3.78x/3.78x**, benchy arachne-off **3.52x/3.59x**, benchy classic-on
**5.43x/5.50x**, benchy arachne-on **4.78x/4.85x**, base classic-off
**4.93x/4.96x**, base arachne-off **4.72x/4.58x**, base classic-on
**8.25x/8.21x**, base arachne-on **7.00x/7.53x** (ordinary/accelerated). The
**park stays binding**: every cell needs a **57–82 %** wall cut to reach ~1.5x,
i.e. **72–102x** the map's whole 0.80 % floor; the release bound is **3.52x**
versus 2026-10-25's 2.71x, and the cause is the **Orca side** moving fast on
benchy (−24…−35 %), not a PnP regression (PnP benchy −2.9…+1.9 %; base
supports-on −6…−14 % from t102). **The archive's error was not
one-directional:** fresh Orca is +6…+18 % on base, so t98's one-cell inference
"every recorded multiple is optimistic for PnP" is **falsified for the four base
cells** (base classic-on 9.31x → 8.25x, base arachne-on 9.36x → 7.00x). No
mechanism is asserted for the base slowdown. The **adhesion asymmetry** is
measured beside the numbers: PnP resolves `brim_width = 8` in all 16 groups,
pinned-back Orca `0` in all 32 runs; its price is ~235 ms, but the job is not
identical and closing it is its own take with its own human decision.

Proof gate: PnP 16/16 `degraded=false`/`non_fatal=0`, ordinary ≡ accelerated
byte-for-byte in all 8 cells, and the PnP output is **exactly the t87 post-t81
reference set** in all eight cells (only Orca moved). Two starvation exclusions
are **named** (`base-arachne-on-orca-m3`, `base-arachne-on-pnp-ordinary-m3`);
three whole-cell re-runs were attempted and all were spoiled by external load (a
competing integration-test binary in another checkout, then sustained browser
CPU at 78–96 %); the human was asked and instructed the take to proceed with the
noise disclosed, and all three spoiled CSVs are preserved. That cell's
two-clean-round band is 6.95–7.05x / 7.23–7.53x and its verdict is unaffected.
Evidence: [`evidence/t103-orca-rebaseline/`](evidence/t103-orca-rebaseline/SCOREBOARD.md)
(`verify.py --self-test` exits 0 checkout-only, 13/13 negative controls
detected). Frontier now: **no open child remains**; the fog and the next route
are the human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-31, seventh session — the ticket 101 take):**
the human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `681c0daa` found exactly one open child,
[101](issues/101-support-identity-table-materialisation.md), which was claimed
and is **resolved (2026-10-31) as a Phase-1 screen: both exact families are below
the 0.80 % floor, so the ticket's own stop rule fires; no candidate, no campaign,
no production edit, recommended family 3 (keep)**. The shape census is one
deterministic census on all three table arms: **23,751 rows over 431 bodies**
(mean **55.1** paths per body), **34,662,046** demand ids, per-row list length p50
**1,322** / max **2,847**, **`mixed_bodies = 0`** (every row of a body carries a
byte-identical list), **418 distinct list contents**, and **428,726 retained ids =
1.24 %** of the blocks if one identity is shared per body — so the table is **one
body's demand list cloned once per emitted support path**, and the 1.2486 GB it
holds is 92 % of the collection's bytes. Both sides are measured **in situ**: the
allocation side by a per-layer `Instant` window around
`assemble_ordered_entities_with_support_identities`
(`crates/slicer-runtime/src/layer_executor.rs`) — table **413.3 / 433.8 /
452.7 ms** vs skip **21.1 / 17.3 ms** — and the free side by a **sampling-free**
`drop(layer_irs)` timing — table **432.4 / 494.1 / 434.4 ms** vs skip **2.614 /
2.890 ms**, with the census reproducing t100's **34,796,481 blocks /
1,358,397,544 bytes** exactly. **Ceiling 821.6 ms uncontended / 926.8 ms most
generous = 0.50–0.56 % of the cell = 0.62–0.70x the 1,318 ms floor**, and that
charges 100 % of the *parallel* assembly time to the critical path. The headline
is a **ledger correction to t100**: its 3.468 s free-side ceiling was a
**system-heap** artifact, and t102's mimalloc KEEP cut the same free
**3.526 s → 0.432–0.494 s (7.1–8.2x)** — the cause-level route had already been
banked by the allocator swap. The wall is **not claimed**: five runs on one
binary spread **150.195–154.793 s (3.0 %)**, 5.0x the whole ceiling, on a machine
the owner's browser was loading at 15–56 % of 12 cores. Consumers re-derived:
written only by the assembly, **read by no production path** (only
`structured_support_identity.rs` and `raft_geometry.rs` under
`crates/slicer-runtime/tests/integration/`), set empty by
`crates/slicer-macros/src/lib.rs` and `crates/slicer-runtime/src/anchored_rows.rs`,
absent from `crates/slicer-schema/wit/`, and proven unread end-to-end by **5/5
runs byte-identical to the post-t81 reference** with the table absent. Evidence:
[`evidence/t101-side-table/`](evidence/t101-side-table/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 15/15 negative
controls rejected). The resolution empties the frontier of children, so the
long-standing Orca re-baseline fog was graduated as
[103](issues/103-orca-rebaseline-repaired-rig.md) — **the only open child, and it
blocks any honest answer to "which cells are still lost"**. Its blocking ruling
was given the same day — **"pinned back"**: OrcaSlicer 2.5's version-flipped
defaults (`enable_arc_fitting`, `elefant_foot_compensation` 0 → 0.15,
`brim_width` 0 → 5) are pinned to the 2.4-era values, so the pins land in
`New-OrcaOverrideArgs` (`docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1`)
and are asserted per run — so 103 is **unblocked and unclaimed**. The ruling
surfaced one **standing adhesion asymmetry** that every future scoreboard must
disclose: PnP's matched cell prints an **8 mm brim** (its skirt-brim module
default; layer 1 opens with 26 concentric rectangles, 12,040 mm of first-layer
extrusion against 2,167 mm on layer 2) while pinned-back Orca prints none. Its
slice-time price is small (`com.core.skirt-brim` reports **235 ms** for the whole
print) but the job is not identical, and closing it means changing the PnP side
of every cell and re-deriving the reference byte set — a separate take with its
own human decision. The take order is the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-31, sixth session — the ticket 102 take):**
the human ruled the allocator lever in scope after ticket 100 ("the map is a
speed study… feel free to test it"), so
[102](issues/102-global-allocator-substitution.md) was graduated, claimed and is
**resolved (2026-10-31, human-approved KEEP — candidate committed)**: `pnp_cli`
installs `AccountingAllocator<mimalloc::MiMalloc>`, the packager slot
`docs/16_slicer_report.md` already prescribes. Proof **16/16** (both arms
byte-identical, 8/8 ordinary cells equal to the t91 references, 8/8 cross-mode,
distinct binaries, identical 24-module trees), and the standing four-cell gate
**PASSED**: wall **−6.19 / −5.64 / −5.54 / −4.80 %** (−10.19 / −9.42 / −9.04 /
−7.41 s) with process CPU **−1.74 / −2.18 / −1.54 / −1.88 %**,
**24/24 pairs byte-identical**, 6/6 pairs per cell, zero starvation exclusions,
23/24 pairs wall-negative (accelerated arachne-on #6 is **+0.18 %**; that cell's
median is −4.80 %). Pre-gate **−7.74 % / CPU −2.25 %** on 3/3 pairs. The effect
is **~5–6 %** — far larger than the free-only price (**1.35–1.66 %**, t100) —
because the allocator acts on every allocation in the process, but it removes
**no allocations**: the census still shows the same 34,796,481 blocks, each
freed more cheaply. The cause-level route stays
[101](issues/101-support-identity-table-materialisation.md) (open, unclaimed).
Disclosures: the pre-gate ran while a stray integration-test binary pinned ~8
cores (walls 227–266 s); the campaign started at load median **1 %** after that
process exited (`target/t102-load-precampaign.txt`); today's baseline walls are
152–156 s against t91's 164.803 / 166.931 s, so ratios are today-vs-today and the
floor (t91's larger walls) is conservative; the allocator is process-wide, so no
stage attribution is claimed; memory is not gated; one transient `cl.exe`
failure on the accelerated build passed on retry. Evidence:
[`evidence/t102-allocator/`](evidence/t102-allocator/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 12/12 negative
controls rejected). Frontier now: **ticket 101 only, open and unclaimed**; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-30, fifth session — the ticket 100 take):**
the human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `cc2c54e6` found **99/99 child tickets resolved — no open
child, no claim** — so the session offered six measured fog routes and the human
selected **allocation-volume arena / bulk-free**. The fog patch graduated as
[100](issues/100-allocation-volume-layer-collection.md) and is **resolved
(2026-10-30) as a Phase-1 screen — measured-dead as scoped, no candidate, no
campaign, probe reverted**. The census: the whole-print free on ordinary base
classic supports-on is **34,796,481 blocks / 1,358,397,544 bytes / 3,526,544 µs**
(**101.3 ns per block**), of which **34,662,668 = 99.6 %** are 8–15-byte strings
sampled as `__skirt_…` / `demand-N` — the `support_entity_identities` side table
that `assemble_ordered_entities_with_support_identities` clones **per support
path** (23,751 rows × ~1,457 demand ids). Geometry is **0.094 %** of the blocks
and **7.8 %** of the bytes. A bulk free of the same bytes costs
**44.5–63.8 ms**, so the ceiling is **3.468 s = 2.10 %** of the cell =
**2.63x** the 1,318 ms floor — but flattening a live collection costs
**73.3–115.1 ms** of copy **plus the same storm** (**+73 ms worse**), and an
at-creation arena aimed at the geometry would remove 0.094 % of the blocks, so
the family dies on **aim and reachability**, not on price. The allocator arm
(mimalloc) cuts the drop **3.527 → 1.302 s measured** (**1.35 %**, 1.69x the
floor; 1.66 % / 2.07x corrected for the probe's 0.506 s sampling overhead), but
the shipped allocator is a **packager** decision
(`docs/16_slicer_report.md` §"Global allocator contract") and it removes no
work. The census graduates
[101](issues/101-support-identity-table-materialisation.md) (open, unclaimed):
price the table's allocation side, audit its consumers, and rule between
share-per-body, prune/gate (the t20 pattern) and keep. Evidence:
[`evidence/t100-allocation-arena/`](evidence/t100-allocation-arena/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 13/13 negative
controls rejected). Frontier now: **ticket 101 only, open and unclaimed**; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-30, fourth session — the ticket 99 take):**
the human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `6c1a2334` again found **98/98 child tickets resolved**
(t98's resolution was still uncommitted; it was committed first as `1240999c`),
so the session offered six measured fog routes and the human selected
**split-and-union the clip dilation**, the last representation-safe shape t96
left open. The fog patch graduated as
[99](issues/99-clip-split-union-dilation.md) and is **resolved (2026-10-30) as
REJECTED on price**: the most favourable split (offset every `deep_infill_area`
polygon on its own, then union the pieces) is **0.8230x / 0.8252x** of the
production call — order-independent within **0.27 %**, both arms byte-identical
to `292df501…` — but that is **314.3 / 298.1 ms** wall-equivalent =
**0.43 / 0.41 %** of the 73.17 s cell, **0.51–0.54x the 585.4 ms floor**. Even the
union-free bound (offsets only, 0.8056x) is **0.47 %**. No candidate, no campaign,
probe reverted. The reusable attribution: the call is **~80 % per-path Miter
offset**, **~19 % batched internal union/cleanup**, **~2 %** an explicit union of
the already-rounded pieces — splitting **partitions the cleanup** rather than
skipping work. On top, the split arm is byte-identical on **1/130** calls and
set-equal on **2/130** (`5,329` differing polygons, deterministic in both
orders), so Clipper2's discretized dilation does not distribute over the union.
The only surviving shape is t96's parity one (`determine_bridging_angle`), which
is out of scope absent a human ruling. Evidence:
[`evidence/t99-clip-split-union/`](evidence/t99-clip-split-union/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-30, third session — the ticket 98 take):** the human
said "continue the perf vs orca map" (no ticket named). The frontier re-derivation at
HEAD `6c1a2334` again found **all 97 child tickets resolved — no open child, no
claim** — so the session offered six measured fog routes (allocation-volume arena;
split-and-union clip-area dilation; `region_partition` 3x-per-layer dedup; repair the
Orca rig; supports-on engine fairness ruling) and the human selected **repair the Orca
rig**. The fog patch graduated as [98](issues/98-orca-rig-repair.md) and is **resolved
(2026-10-30)** as a repair that also falsified the archived Orca side. The 2.5 install
resolves `--load-settings` through a vendor preset bundle, so the rig now loads a
version-gated source-manifest tree (pinned to the tag whose `BBL.json` version equals
the installed `BBL.opc` stamp, `02.08.00.16` == `2.8.0.16`) plus CLI overrides, and
pins `enable_arc_fitting = 0` because 2.5 flipped that default to enabled.
Measured on the screening cell (benchy classic-off, median of three): **1.978 s** wall
/ 6.50 s CPU / 4,000,709 bytes against the archive's 2.787 s / 9.27 s / 3,890,392 —
because the archived Orca applied its **hardcoded filament defaults** (density 0,
max volumetric speed 2, first-layer temp 200) while the repaired rig resolves the
vendor chain (1.24 / 12 / 220). The archived Orca side was therefore doing less work
than the matched job asks: **every archived Orca median needs re-derivation before a
ratio is quoted**, and PnP's recorded multiples are optimistic for PnP. No ratio was
requoted. Evidence: [`evidence/orca-rig-repair/`](evidence/orca-rig-repair/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 4 claims, 7/7 negative controls
rejected). Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-30, second session — the ticket 97 take):** the
human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `f48a0488` again found **all 96 child tickets resolved — no
open child, no claim** — so the session offered six measured fog routes (match
the 5 mm branch distance; base + benchy bottleneck refresh; `region_partition`
dedup; split-and-union clip dilation; allocation-volume arena; host-removal gate
ruling) and the human selected **match 5 mm branch distance**. The fog patch
graduated as [97](issues/97-match-branch-distance-fairness.md) and is **resolved
(2026-10-30) as REJECTED on fairness — the premise is false, no profile edit, no
campaign.** Orca's `support_style = default` on a tree family resolves to
**organic**, and the organic engine reads only
`tree_support_branch_distance_organic`, resolved to **1 mm**. The normal
`tree_support_branch_distance = 5 mm` that the deviations table compares is inert
on that path. PnP is already at 1 mm, so the t88 arm was a 5x work reduction, not
a match. The ticket surfaced the real supports-on fairness gap — Orca organic vs
PnP's **Strong** substitution (`DEV-156`) — and the infrastructure blocker that
the 2026-10-27 Orca install no longer accepts the rig's file-path
`--load-settings`. Evidence: [`evidence/t97-branch-distance/`](evidence/t97-branch-distance/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 5/5 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-30, on the ticket 96 take):** the human said
"continue the perf vs orca map" (no ticket named). The frontier re-derivation at
HEAD `0b25a8b9` found **all 95 child tickets resolved — no open child, no
claim** — so the session offered six measured fog routes (windowed clip-area
dilation; match `tree_support_branch_distance` 5 mm; arena layer collection;
planner residual vs canonical; scoreboard + increment refresh; apply-opening
join parity) and the human selected **windowed clip-area dilation**. The fog
patch graduated as [96](issues/96-windowed-clip-area-dilation.md) and is
**resolved (2026-10-30) as a Phase-1 screen — REJECTED on exactness, no
candidate, no campaign, probe reverted.** The price is real and above the floor:
the whole-layer `+1.5 × spacing` dilation is 11.67–11.81 s accumulated over 130
calls, and a candidate-box window (8 × spacing, operand 11 × spacing) removes
**91 %** of it — **10.65–10.78 s saved = 1.57–1.60 %** of the ordinary base
classic-off cell, 2.0x the floor, the two runs agreeing within 1.2 %. But the
windowed arm deterministically changes the internal-bridge **direction**
(`5fc8fa72…` / 21,023,889 bytes against `292df501…` / 21,023,739; first differing
block is an `Internal Bridge` at `;Z:21.4`, 8 → 10 moves), and at that layer the
production and windowed clip areas are set-equal **inside the window**
(`mismatch_polys = 0`). The failure is the anchor consumer:
`internal_bridge_angles`' `expansion_area ∩ clip_area` term feeds
`boundary_anchor_polylines`; with qualification and `area` unchanged, the nearest
anchor must have lain beyond the window (or the cut introduced a closer ring), so
the anchor reach is not bounded by the planned 3.3 × spacing and no finite
candidate-driven window is safe. This is the
mirror of t93, where the same operand's **erosion** was safe because its
consumers are bounded by `2 × spacing`. The wall screen was never reached (the
single uninstrumented window-arm wall, 76.557 s, is unpaired and not a claim).
Fog now carries the surviving shapes: keep the whole operand and make the offset
itself cheaper (split-and-union over disjoint polygons, unmeasured), or change
the angle consumer (parity work). Evidence:
[`evidence/t96-windowed-clip-area/`](evidence/t96-windowed-clip-area/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

**Previous selection scope (2026-10-29 → 2026-10-30, on the ticket 95 take):**
the human said "continue the perf vs orca map" (no ticket named). The frontier
re-derivation at HEAD `8f19a6a9` found **all 94 child tickets resolved — no open
child, no claim** — so the session offered six measured fog routes (batch the
exact-Z cross-sections; windowed clip-area dilation; arena layer-collection free;
rule on the four-cell gate; port support branch distance; re-measure the engine
gap) and the human selected **batch exact-Z cross-sections**. The fog patch
graduated as [95](issues/95-exact-z-cross-section-batching.md) and is
**resolved (2026-10-30, human-approved KEEP, candidate committed `afe9c361`)**.

The premise, re-verified against source before the offer: `validate_entry`
(`crates/slicer-wasm-host/src/support_aggregation.rs`) runs per entry in a
**serial** loop; each `ExactZQueryService::query` miss called
`cross_section_at_z`, which is `slice_mesh_ex(mesh, &[z])` — a **one-element**
Z array over the **2,461,234** triangles of `tmp/base.stl`'s world mesh. t91
measured **431 queries / 431 distinct `(object, region, z)` / 431 misses** — the
cache never hit, so every entry paid a full walk.

Phase 1 (probe, 3 slices, outputs byte-identical to the post-t81 references):
**431/431 planes equal polygon-for-polygon**, **6.42 / 6.72 / 6.84 s** of
single-Z walks collapsing to **0.99–1.05 s** in one batched `slice_mesh_ex` walk
= **5.37 / 5.73 / 5.83 s** saved = **3.26 / 3.48 / 3.49 %** of the cell,
**4.07–4.37x** the 0.80 % floor. The batch buys **6.4–6.9x**, not 431x, because
the batched walk pays two binary searches over 431 sorted planes and a straddle
test per triangle and is memory-latency-bound.

Candidate: per-`(object, z_units)` cross-section memo plus
`ExactZQueryService::prefetch_cross_sections`, called once per object from
`try_aggregate_support_plans_with_policy` before the serial loop; the
per-`(object, region, z)` result contract, the single-Z miss path and the
per-entry degradation are unchanged; host-only, no IR/guest/geometry change.
**Proof 16/16 PASS** (both arms byte-identical, 8/8 ordinary cells equal to the
post-t81 references, 8/8 cross-mode equal, 24 identical modules per mode,
four distinct host binaries). **Standing gate PASS in all four pre-declared base
supports-on cells**, 6/6 pairs each, zero starvation exclusions, 24/24 pairs
byte-identical: wall **−3.97 / −3.19 / −2.86 / −2.64 %** and CPU **−0.70 /
−0.70 / −0.90 / −0.15 %** (ordinary/accelerated × classic/arachne), wall deltas
**−6.17 / −5.68 / −5.43 / −5.49 s**, CPU sum **−27.2 s**. cpu/wall rises in every
cell (6.57→6.76, 6.67→6.87, 6.46→6.63, 6.39→6.61) — serial work removal
concentrating the same parallel work, the t93 artefact, not extra work. One
**disclosed** non-negative pair (accelerated classic-on pair 4, **+2.35 %**,
during a load event that slowed both arms ~10 s in that pair); the gated
statistic is the paired median. Pre-commit: workspace clippy clean,
`check-literals` 0, `check-test-quality --report` 0, contract 163/0, support
integration 22/0, both freshness gates exit 0. Evidence:
[`evidence/t95-exact-z-batch/`](evidence/t95-exact-z-batch/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 7/7 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-29, on the ticket 94 take):** the human said
"continue the perf vs orca map" (no ticket named). The frontier re-derivation
at HEAD `57234b81` found **all 93 child tickets resolved — no open child, no
claim** — so the session offered five measured above-floor leads from the fog.
The human selected the **remaining whole-print collection drop** and settled the
gate as **strict wall in all four cells** (not CPU-corroborated wall-neutral).
The fog patch graduated as
[94](issues/94-remaining-whole-print-drop.md) and is **resolved (2026-10-29) as
a Phase-1 screen — measured-dead, no candidate, no campaign, probe reverted.**
The split: the `host:gcode_emit` bracket is **93-94 % drop** — walk 176-253 ms,
free **3,242-3,558 ms**, bracket 3,415-3,811 ms — and the post-emission tail a
deferred free could hide inside is only **1,367-1,603 ms** (`machine-gcode-emit`
0.53-0.60 s + `GCodeSerialize` 0.72-0.92 s). **Parallel free is a wash**
(classic −0.321 s = +0.19 %, arachne +0.200 s = −0.12 %), so the free does not
scale on Windows' process heap; **layer-by-layer consumption is a structural
zero** (serial walk); and the **deferred-overlap absolute ceiling is the tail
itself — 0.97 % classic / 0.82 % arachne of the cell, 1.22x / 1.02x the 0.80 %
floor** — before any overlap loss, so no shape had margin for the strict
four-cell campaign. The screen leaves a precise structural lead in the fog: the
free is an **allocation-volume** cost (~5.3 GB peak working set, t79), not a
drop-strategy cost; only fewer/larger allocations (arena or bulk-free layer
collection) can move it. Evidence:
[`evidence/t94-remaining-drop/`](evidence/t94-remaining-drop/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-28 → 2026-10-29, on the ticket 93 take):**
the human said "continue the perf vs orca map" (no ticket named); the frontier
re-derivation found [93](issues/93-whole-layer-deep-infill-offsets.md) as the
only open child, and the session took it. It is **resolved (2026-10-29) as
measured GATE NOT MET — human-approved DROP, candidate reverted**, with the
re-appliable `candidate.patch` preserved. Phase 1 closed three of four shapes:
lazy dilation **0 of 130** calls skippable; cheaper predicate rejected (no
Miter-join equivalence); duplicate recompute **9** calls, **0** identical
operands, absolute ceiling **0.28 %**. The survivor, **windowed erosion**,
measured **9,244.1 / 9,311.3 ms** accumulated saving = **1,006.9 / 1,045.6 ms
wall-eq = 1.38 / 1.43 %** of the cell against the **0.585 s** floor, the two
runs agreeing within **3.70 %**. The candidate (erosion operand clipped to the
pre-retain qualified bbox grown by `14 × spacing`) passed **16/16** proof cells
byte-identical, **8/8** ordinary cells equal to the post-t81 references, **8/8**
cross-mode equal, 24 identical modules per mode. The paired campaign passed the
wall clause in both ordinary cells (**−3.31 % / −0.26 %** wall, **−1.19 % /
−0.48 %** CPU) and missed it in both accelerated cells (**+0.13 % / +0.62 %**
wall, **−0.81 % / −0.45 %** CPU), so the pre-declared four-cell wall clause is
**NOT MET** and the human dropped the candidate. The CPU fall matches the probe
(9.7 s vs 9.2 s) and the wall transfer is lifted by cpu/wall 9.65 → 9.84. One
campaign interruption is disclosed (server restart during ordinary classic-on;
that cell re-run whole, partial preserved); the campaign timed only the four
pre-declared base classic cells, so benchy/arachne gains are unmeasured.
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged. Evidence:
[`evidence/t93-deep-infill-offsets/`](evidence/t93-deep-infill-offsets/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 18/18 negative controls
rejected).

**Previous selection scope (2026-10-18, on the t73 take):** the frontier was
re-derived on resume — the only open child was
[consume-only-when-read annotations](issues/20-consume-only-when-read-annotations.md),
parked below-fold — and the human selected a **Base-cell bottleneck refresh**
from the route options. [73](issues/73-base-current-bottleneck-refresh.md) was
graduated, claimed and **resolved (2026-10-18) as a measurement-only take**:
eight fresh ordinary base captures (four fuel profiles + four
instrumentation-only corroborations) at HEAD `86c15182` (slicer sources = the
t70 keep `7127b4d6`), every output byte-identical to its t58 qualification
reference, guest freshness exit 0, 24/24 external modules. The base cells are
**prepass-dominated** — the common floor is 104–115 s on every cell and the
supports-on cells add 134–149 s of support stages — while the per-layer phase
is 94–117 s on classic and 16–23 s on Arachne. Three precise tickets were
graduated in recommendation order: [LayerFinalization supports-on whole-print
view cost](issues/74-layerfinalization-supports-on-view-cost.md) (four module
brackets of 45.0–48.5 s against a 0.30–1.39 s supports-off control, with
0.042 % of the slice's guest fuel), [ShellClassification phase-B serial commit
reduction](issues/75-shellclassification-phase-b-commit.md) (26.0–31.4 s on
every base cell; t27's serial commit is still 25–30 s), and [MeshAnalysis exact
footprint union reduction](issues/76-meshanalysis-footprint-union.md)
(19.4–22.2 s serial over 326,546 facets; bbox replacement unsafe, exact
reduction only). Ticket 20 received the fresh base context (per-layer
94.3/98.2 s; classic-perimeters 98.8/95.0 % of fuel; t18's 57.9 % base scan
share) with the below-fold park **unchanged** — the t72 best cell is still
3.30x. No candidate, timing campaign, production edit or production-source commit. Evidence:
[`evidence/perf-refresh-base-2026-10-18/`](evidence/perf-refresh-base-2026-10-18/FINDINGS.md)
(`verify.py` exits 0 checkout-only with four negative controls rejected;
`analyze.py` reproduces the tables). Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold and the
three new tickets unclaimed; the take order is the human's. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-18 → 2026-10-19, on the ticket 74 take):**
the human's "continue" named no ticket, so the session took the first frontier
ticket in order, [74](issues/74-layerfinalization-supports-on-view-cost.md);
it is now **resolved (2026-10-19, human-approved KEEP, candidate committed
`091e6bc9`)**. Attribution: **96.1 % of the 61,555 ms supports-on module sum is
whole-print copy/free churn** (input snapshot clone 12,112 ms, store drop
16,653 ms, merge rollback clone 13,446 ms, old-vector drop 16,940 ms; guest call
1,816 ms = 2.9 %), owned by the full `LayerCollectionIR` clone/free and its
IR-only `support_entity_identities` side table (23,751 rows supports-on vs 0
supports-off), not by module reads or the 1.96x output-byte ratio. The candidate
(Arc-shared input snapshot + no-op/append-only merge fast paths) measured
**61,555 → 2,775 ms (−95.5 %)** with supports-off unchanged (1,290 → 1,252 ms)
and outputs byte-identical to the t58 references. **Proof gate 16/16 PASS**
(baseline and candidate byte-identical, clean, non-vacuous in every cell;
`qualification.json`), with snapshots built from `67ef742b` and provenance
recorded. The paired ordinary + accelerated A/B ran on a quiet machine on
2026-10-19 and **PASSED**: 6/6 pairs in every cell, paired wall deltas −3.13 % /
−12.71 % / −3.89 % / −14.23 % and corroborating CPU deltas −0.91 % / −2.30 % /
−0.84 % / −2.97 % (ordinary/accelerated × benchy/base). Base supports-on wall
falls **46.0 s ordinary** and **47.5 s accelerated**, inside t73's 45.0–48.5 s
bracket attribution, and every pair's output is byte-identical. Frontier now:
[75](issues/75-shellclassification-phase-b-commit.md) and
[76](issues/76-meshanalysis-footprint-union.md) unclaimed;
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the take
order is the human's. The probe take's machine drift (baseline wall 571.9 s vs
t73's 408.7 s) is why its brackets are an internal split, not wall evidence.
Evidence:
[`evidence/t74-finalization-view/`](evidence/t74-finalization-view/FINDINGS.md)
(`verify.py` exits 0 checkout-only, three negative controls rejected).

**Current selection scope (2026-10-28, on the ticket 92 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation at
claim time found [92](issues/92-per-surface-bridge-boolean-locality.md) as the
only open child, and the session took it. Ticket 92 is **resolved
(2026-10-28) as a Phase-1 screen — measured-dead, no candidate, no production
edit, probe reverted.** One ordinary base classic supports-off slice, two probe
runs, outputs byte-identical to the post-t81 reference `292df501…` and clean.
The ticket's three Phase-2 shapes (per-surface bbox gate, hoistable `1.3 ×`
anchor offset, many-piece tiling) have an **absolute upper-bound ceiling of
0.341–0.371 % of the cell** against the **0.80 %** floor (2,429.6 / 2,303.2 ms
accumulated = 271.1 / 249.7 ms wall-equivalent); deleting the whole
`internal_bridge_angles` step as well still reaches only 0.40–0.44 %. Shape
counts refute tiling outright — 130 calls carry **196 surfaces** (mean 1.51,
**max 7**) against Orca's 128-piece threshold — and the per-surface
`offset`/`intersection`/qualification is **0.25 s accumulated** against
`internal_bridge_angles`' 2.58 s (t75's 1.5 s clip bracket does not transfer;
its 2.4 s angles bracket does).

The screen found a **different, larger term of a different shape**: the same
function's two **whole-layer offsets on the unioned deep sparse-infill area** —
`offset(deep_infill_area, +1.5 × spacing)` **12.5 / 11.4 s accumulated** over
the 130 calls (median 81.6 ms, max 222.4 ms) and
`offset(deep_infill_area, −4.5 × spacing)` **10.0 / 9.3 s** over the 14
qualified calls (max 428.3 ms) — together **2.24–2.51 s wall-equivalent =
3.06–3.43 % of the cell**, ~4x the floor and ~9x t92's own ceiling. Both are
whole-layer operands (the clip area is also the angles operand; the erosion is
the `retain` predicate), so consumer bbox-gating cannot remove them; they are
out of t92's question rather than a reason to keep it open. Graduated
[ticket 93](issues/93-whole-layer-deep-infill-offsets.md) to price the lazy
dilation, windowed erosion, cheaper-predicate and duplicate-recompute shapes
with its own Phase-1 screen. Disclosures: instrumented wall 79.0–83.0 s against
the uninstrumented 73.17 s cell (probe + `--profile` overhead, never a wall
claim), ~7–9 % inter-run drift on the bucket with the verdict unchanged.
Evidence:
[`evidence/t92-per-surface-bridge/`](evidence/t92-per-surface-bridge/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 15/15 negative controls
rejected). Frontier now: [93](issues/93-whole-layer-deep-infill-offsets.md)
only, open and unclaimed; the take order is the human's. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-27, on the ticket 91 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation at
claim time found [91](issues/91-exact-z-object-geometry-memo.md) and
[92](issues/92-per-surface-bridge-boolean-locality.md) open, and the session took
the first in order. Ticket 91 is **resolved (2026-10-27, human-approved KEEP,
candidate committed `f10cef5a`)** as a measured **GATE NOT MET on one cell's
strict CPU clause** plus a wall pass in all four: wall
**−5.52 / −3.84 / −2.68 / −4.95 %** (ordinary/accelerated × classic/arachne)
with process CPU **−1.22 / −0.66 / +0.08 / −0.32 %**, **24/24 pairs
byte-identical**, one SHA per arm, zero starvation exclusions, 16/16 proof cells
byte-identical and 8/8 ordinary cells equal to the derived post-t81 references.
The **+0.08 pp** is `accelerated-base-classic-on` — 0.8 s of ~1,053 s process
CPU, inside that cell's own repeat spread — and it is why the ticket's
pre-declared `CPU ≤ baseline` clause is NOT MET; the human accepted the
sub-noise miss and kept the candidate. One memo pass: the exact-Z service builds
the world mesh, the plate and the constant top-Z cross-section **once per
object** instead of on every miss, leaving the per-`(object, region, z)` cache
contract unchanged. Direct probe (attribution only; both outputs equal the
reference SHA): **431 queries, 431 misses, 431 geometry builds → 1**,
`ExactZQueryService::query` **22,636.3 → 8,922.3 ms (−60.6 %)** and the
`validate_entry` loop **24,335.0 → 10,004.3 ms**; the remaining 8.9 s is the
requested-Z cross-section. The `post_planner_gap_ms` bracket falls in all four
cells (14.7→9.9 / 18.4→7.4 / 21.8→9.9 / 13.8→12.1 s). The memo retains one
world-space mesh per object — **42.25 MiB** measured for `tmp/base.stl`
(1,230,602 vertices + 7,383,702 indices) — recorded, not gated. Both freshness
gates exit 0, `verify.py --self-test` exits 0 checkout-only with 13/13 negative
controls rejected. Frontier now:
[92](issues/92-per-surface-bridge-boolean-locality.md) only, open and unclaimed;
the take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged. Evidence:
[`evidence/t91-exact-z-memo/`](evidence/t91-exact-z-memo/FINDINGS.md).

**Current selection scope (2026-10-27, on the ticket 90 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation at
claim time found [90](issues/90-postpass-layer-collection-ownership.md) and
[91](issues/91-exact-z-object-geometry-memo.md) open, and the session took the
first in order. Ticket 90 is **resolved (2026-10-27, human-approved KEEP,
candidate committed `7b22fccc`)** — the standing paired gate **PASSED in all
four pre-declared base supports-on cells**: wall **−5.54 / −7.56 / −6.54 /
−7.64 %** (ordinary/accelerated × classic/arachne) with process CPU
**−1.13 … −1.67 %** (flat or lower, not merely inside the +3.0 % ceiling),
**24/24 pairs byte-identical**, one SHA per arm, zero starvation exclusions,
every pair moving the right way (−1.56 … −13.20 %). One ownership pass: the
finalization stage takes the caller's `Vec<LayerCollectionIR>`
(`execute_layer_finalization_owned`) instead of cloning a rollback snapshot,
`GCodeEmitter::emit_gcode_owned` lets the emitter rotate and walk the caller's
own vector, and the postpass hands it over, so the collection is dropped once
instead of cloned and dropped twice. Attribution (instrumented, attribution
only): the **1,765 / 1,684 ms clone**, the **3,685 / 3,758 ms commit drop** and
the **4,490 / 3,723 ms postpass teardown drop** are gone, the one remaining drop
lives inside the `host:gcode_emit` bracket and costs **1.56 / 2.68 s less** than
the old clone+drop pair, and the postpass phase falls **66.5 / 70.3 %** — which
is why the measured wall deltas (**12.0 / 14.8 / 15.9 / 15.4 s**) beat the
ticket's pre-declared net ceiling of **10.76 / 10.29 s**. Exactness: 16/16 proof
cells byte-identical and equal to the derived post-t81 references, ordinary ≡
accelerated **8/8**, both freshness gates exit 0, host-only candidate proven by
`provenance.json` (byte-identical 24-module trees per mode, distinct binaries
per arm). Both the abort-state change (a fatal finalization error now consumes
the working collection) and the pre-existing post-t81 `resolution`-fixture reds
(23 contract — measured identical with and without the candidate — plus 3
executor and 50 integration diagnosed by signature) are disclosed. Frontier now:
[91](issues/91-exact-z-object-geometry-memo.md) and the concurrent screening
take's [92](issues/92-per-surface-bridge-boolean-locality.md), both open and
unclaimed; the take order is the human's. Standing paired acceptance and
KEEP/DROP rules unchanged. Evidence:
[`evidence/t90-postpass-ownership/`](evidence/t90-postpass-ownership/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 11/11 negative controls
rejected).

**Previous selection scope (2026-10-27, PR screening take):** at the human's
request, three merged OrcaSlicer performance PRs were screened against this
map's measured shapes. [#16741 "Speed up slicing of finely detailed
models"](https://github.com/OrcaSlicer/OrcaSlicer/pull/16741) has no PnP
counterpart for either half: the emitter has no per-extrusion object-mass
locator, and the dense-below bridge shortcut fires only when every fill surface
of the lower layer prints at 100 % density, which the matched job's 20 % sparse
infill never reaches. [#16733 "Speed up slicing of objects with many
parts"](https://github.com/OrcaSlicer/OrcaSlicer/pull/16733) has no per-part
contact test to accelerate: the loader merges normal 3MF parts into one mesh and
`SkirtBrim::generate_brim_entities` builds one print-wide rectangular loop set.
[#15985 "Faster slicing of layers cut into many
pieces"](https://github.com/OrcaSlicer/OrcaSlicer/pull/15985) maps only its
neighbourhood-restriction / fill-boundary-hoist mechanisms, and only to the
per-surface bridge qualification: the t75-named clip+angles terms are 3.9 s
accumulated ≈ 0.55 % of the current base classic-off cell, below the 0.80 %
floor before the unmeasured per-surface count. One screening ticket graduated:
[92](issues/92-per-surface-bridge-boolean-locality.md) — Phase 1 measures the
per-surface share and closes measured-dead unless it clears the floor; Phase 2
is conditional. Frontier now:
[90](issues/90-postpass-layer-collection-ownership.md),
[91](issues/91-exact-z-object-geometry-memo.md) and
[92](issues/92-per-surface-bridge-boolean-locality.md); the take order is the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-27, on the ticket 89 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found [89](issues/89-support-plan-exact-z-validation-parallel.md) and
[90](issues/90-postpass-layer-collection-ownership.md) open, and the session
took the first in order. Ticket 89 is **resolved (2026-10-27) as a measured
GATE NOT MET — human-approved DROP, candidate reverted**. Exactness passed
in full: 16/16 cells byte-identical between arms, 8/8 ordinary ≡ accelerated,
ordinary cells equal to the derived post-t81 references, contract suite
160/160 (including a new 64-entry order test that derives its expectation three
independent ways). The standing four-cell campaign was **not run by the stop
discipline**: a 3-pair pre-gate on ordinary base classic-on at 2 % load
measured wall **−5.50 %** against process CPU **+6.78 %**, more than twice the
pre-declared **+3.0 %** work-preserving ceiling, every pair byte-identical. The
probe shows the CPU is contention, not extra work — 431 entries = 431 distinct
`(object, z)` = 431 queries = 431 misses — and splits the tail into
requested-Z cross-section **50.96 %**, constant top-Z cross-section **43.39 %**,
world-mesh build **5.65 %** of 92.73 s accumulated thread-time for a 7.99 s
wall, against t88's **15.24 s serial** tail: ≈6x thread-time for the same
queries at 12-way concurrency (cross-run), so the kernel is
memory/allocator-bound. The ticket's
optional `(object, z)` cache is **measured worthless here** (one entry per
anchor Z) and was not implemented. One candidate graduated:
[91](issues/91-exact-z-object-geometry-memo.md) — memoize the z-independent
terms, **49.04 %** of the tail and ≈**7.5 s / 6.8 s** ceiling (**4.1 % / 3.6 %**
of the cell), which removes work instead of spreading it and should lower CPU.
Two pre-existing reds in `slice_region_view_contract_tdd` were repaired to make
the contract suite runnable (t81's mandatory `resolution` key; t20's
consumer-live stamp gate); neither touches the candidate. Evidence:
[`evidence/t89-exact-z-parallel/`](evidence/t89-exact-z-parallel/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls
rejected). Frontier now:
[90](issues/90-postpass-layer-collection-ownership.md) and
[91](issues/91-exact-z-object-geometry-memo.md), open and unclaimed; the take
order is the human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-25, on the ticket 88 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found [88](issues/88-supports-on-increment-attribution.md) as the only open
child, and the session took it in order. It is **resolved (2026-10-25) as a
measurement-only attribution — no candidate, no production edit, no keep/drop
campaign.** Four quiet (5 % load) instrumented base captures at HEAD
`5c7fa03e`, all byte-identical to the post-t81 references, plus a
corroborating loaded pass (68–73 % desktop load) and a config-only
`tree_support_branch_distance = 5.0` arm. The supports-on increment is
**106.8 s classic / 109.7 s arachne** and is **~69 % the support geometry
itself** (planner 55.5 s = 52.0 % of the classic increment; canonical contact
detection 10.2 s; per-layer render 5.9 s wall) plus **~31 % host-side**: a
**serial 15.2 / 13.9 s exact-Z validation loop** (99.3 % of the post-planner
gap, every entry independent) and **one finalization clone + two whole-collection
drops** (finalization 6.8 s, emission 6.3 s, teardown 4.6 s; the probe's
`SystemTime` marks show the emitter's clone is ~5 ms and the 6 s is the drop).
Two candidates graduate on the map's 0.80 % floor:
[89](issues/89-support-plan-exact-z-validation-parallel.md) — parallelize the
validation, **8.28 % / 7.42 %** of the cell — and
[90](issues/90-postpass-layer-collection-ownership.md) — one ownership path
through finalization/emission, **5.84 % / 5.47 % net** (9.18 % / 8.60 % upper).
Measured dead/geometry-bound: the SupportAnalysis eligibility fold (2.24 s
gross, no above-floor removable term), the contact kernel (10.2 s, parallel canonical), the
planner (55.5 s, support geometry, t78 batching in place) and the per-layer
render (5.9 s wall, already parallel). The **`tree_support_branch_distance`
5 mm arm is −7.25 s (−3.94 %) and ~6.8 s of the increment (6.4 %)** — a modest
driver, not the reason; the parity ruling stays with the human. The probe also
caught a measurement trap: an `Instant` clock read disagreed with the event
timestamps, so a 5 ms clone read as 4.09 s; all probe claims use the
`SystemTime` marks and the stage brackets. The probe is reverted; both
freshness gates exit 0. Evidence:
[`evidence/t88-supports-increment/`](evidence/t88-supports-increment/FINDINGS.md)
(`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls
rejected). Frontier now: **tickets 89 and 90, open and unclaimed**; the take
order is the human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-25, on the ticket 87 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found **86/86 children resolved**, and the human selected the **Scoreboard
refresh** route from the options presented.
[87](issues/87-scoreboard-refresh-post-resolution-port.md) is **resolved
(2026-10-25) as a measurement-only take — no candidate, no production edit.**
The first fully resolution-matched scoreboard, measured at HEAD `6e71c1a1`
with both standing trees rebuilt from clean HEAD: **Orca still wins all 8
cells at 2.71x–9.36x**, CPU corroborating in every cell (PnP burns
4.1x–21.3x Orca's CPU). Best cell **2.71x** (benchy classic-off accelerated;
2.75x ordinary; 2.74x benchy arachne-off), worst **9.36x** (base arachne-on
ordinary). **No cell within ~1.5x — the below-fold park is still binding**,
though the release bound has moved **3.30x → 2.71x**. PnP ordinary wall fell
**39.7–66.2 %** in all eight cells since 2026-10-18 (t74/t75/t76/t20/t78/t81/t85
+ t65 on Arachne), while **Orca also measured 16.2–35.3 % faster today**
(machine drift, disclosed; ratios are today-vs-today). Output: PnP 16/16
`degraded=false`/`non_fatal=0`, ordinary ≡ accelerated **byte for byte in all
8 cells**, one SHA per group, new post-t81 reference set recorded; the base
SHAs reproduce the t83/t84/t85 references. 96 rows / 72 measured, zero
starvation exclusions, zero validation failures, both freshness gates exit 0.
Disclosed: an accidental duplicate benchy round (preserved as
`rows-benchy-duplicate-round.csv`, not ledger input; the two rounds agree
within ~1 %, and the clean ledger re-run is 4–6 % slower on PnP, so **benchy
uncertainty is larger than base's**). The headline finding — every supports-on
cell is the worse of its pair, and the base increment is **+97.90 / +113.38 s**
for PnP against **+4.50 / +5.79 s** for Orca while PnP emits *fewer* support
sections — graduated
[88](issues/88-supports-on-increment-attribution.md) (open, unclaimed).
Evidence: [`evidence/scoreboard-refresh-20261025/`](evidence/scoreboard-refresh-20261025/SCOREBOARD.md)
(`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls
detected). Frontier now: ticket 88 only; the take order is the human's.
Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-25, on the ticket 85 take):** the human's
"continue the perf vs orca map" named no ticket, so the session took the only
frontier ticket, [85](issues/85-seam-inset-offset-batching.md) — and it is
**resolved (2026-10-25, human-approved KEEP)**. The candidate batches the seam
planner's 495 per-region inset offsets into **one** `offset_polygons_batch`
(two guest passes, unchanged per-request arguments, unchanged entry order and
fallback path), and the standing gate **PASSED in all four pre-declared base
classic cells**: wall **−7.46 % / −2.72 %** (ordinary) and
**−7.03 % / −3.86 %** (accelerated), process CPU **+0.34 … +0.47 %** (ceiling
+3.0 %), **24/24 pairs byte-identical**, one SHA per arm, **zero starvation
exclusions**, zero pair moving the wrong way (individual wall ratios
0.887–0.977). Output identity is **16/16** against the derived post-t81
reference set, with cross-mode equality **8/8**.

Attribution: the seam stage falls **8,356 → 1,737 ms** (off) and
**8,288 → 1,703 ms** (on) in instrumented captures — **≈7x batch occupancy** on
the offset request set (conservative bound 4.4x), against this ticket's
pre-declared **1.15x** DROP floor. **t78's 2.6x pricing was too pessimistic**,
which is why ticket 84's predicted 5.43 % (off) is beaten by the measured
7.46 %; the shape is now 2.3 % of a base cell, so the seam stage has left the
base common floor. The parity case is exact by construction (no geometry
change) and pinned by
`batched_inset_offsets_match_the_serial_reference_per_region`
(`modules/core-modules/seam-planner-default/src/lib.rs`), which compares the
batched path entry-for-entry with the pre-batching serial loop on a fixture
whose two regions carry different insets.

**One infra prerequisite was repaired inside this take:**
[86](issues/86-accelerated-shim-workspace-root.md) — a cold accelerated build
died in `proc-macro2`'s build script because the rustc shim resolved the
workspace root from CWD (t71's runtime-walk contract, wrong for the one caller
whose CWD is a cargo-registry package). Fixed by exporting
`PNP_XTASK_WORKSPACE_ROOT` from the three accelerated entry points and
re-validating it in `resolve_shim_root` (`xtask/src/rustc_driver.rs`;
`cargo test -p xtask` 187/0). No candidate behaviour depends on it; without it
the accelerated arm of this gate could not be measured.

Disclosures: the two arms' `pnp_cli.exe` differ (the seam planner is linked into
the host via `slicer-integrated-modules`; both arms run
`--no-integrated-modules`, so it is inert), guest fuel rises +3.1 %, and the
pre-campaign sanity pair taken under desktop load is not used anywhere.
Evidence: [`evidence/t85-seam-inset-batching/`](evidence/t85-seam-inset-batching/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls rejected).
Frontier now: **no open child remains**; the fog and the next route are the
human's. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-24, on the ticket 84 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found **no open child** (83/83 resolved) and the human selected the **Base
common-floor attribution** route.
[84](issues/84-base-common-floor-attribution.md) is **resolved as an
attribution take — one candidate graduated, two shapes measured dead; no
production edit, probe reverted.** The seam stage's wall is **92.4 / 93.2 %** one
serial native `offset_polygons` per region (495 calls, 8,195 / 8,177 ms of a
9.14 / 8.97 s stage = **8.83 / 3.92 %** of the 92.8 / 208.6 s cell walls); the
candidate list is 0.29 / 0.13 % and host projection + harvest are under 1 %. The
overhang stage is **engine-bound**: 99.7 % is the parallel `annotate_overhangs`
kernel and 85.4 % of its work is the canonical three round offsets per band
layer (1,455 calls) at **11.86x** measured parallelism; the only operand-side
shape (bbox window) retains **97.84 %** of the previous points, a
**0.189 / 0.080 %** ceiling. Graduated
[85](issues/85-seam-inset-offset-batching.md) (open, unclaimed): batch the inset
offsets through the existing `offset_polygons_batch`, ceiling **8.83 / 3.92 %**
upper and **5.43 / 2.41 %** at t78's measured 2.6x. Frontier now: ticket 85
only; t83's `tree_support_branch_distance` observation stays fog. Evidence:
[`evidence/t84-common-floor/`](evidence/t84-common-floor/FINDINGS.md)
(`verify.py --self-test` exits 0, 8/8 negative controls rejected).

**Previous selection scope (2026-10-23 → 2026-10-24, on the ticket 83 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found [83](issues/83-arachne-wall-simplification-tolerance-pricing.md) as the
only open child and the session took it. It is **resolved (2026-10-24) as a
measurement-only pricing take — DROP, below the map's pre-declared floor; no
candidate, no parity port, no production edit.** One binary, one module tree,
two config-only arms (default 0.05/0.005 mm vs canonical 0.5/0.025 mm), 2
alternating instrumented pairs per Arachne cell plus 6 uninstrumented pairs on
the two borderline cells. The tolerance is **real work**: arachne guest fuel
falls **−10.76 %** (benchy) / **−16.21 %** (base), whole-job fuel −3.41/−1.41 %
and −11.56/−3.08 %, G-code bytes −3.84…−8.23 % with identical TYPE counts, all
runs clean and byte-deterministic per arm (the default arm reproduces t81's
base-arachne-off reference `01a1434c…`). But the module is only **2.9–6.9 % of
cell wall** at **5.1–10.7x** phase parallelism, so the occupancy-normalized
ceiling is **0.039–0.376 %** of wall against the pre-declared **0.80 %** floor;
the generous fuel-proportional bound reaches only 0.844 % (base-off, whose
measured elapsed moved +0.8 % the wrong way). The direct wall check agrees:
base-off **+0.17 % wall / +0.09 % CPU**, benchy-on **+3.18 % wall / +0.17 %
CPU** (paired medians; benchy-on's instrumented −2.9 % was load drift). `D-168`
is retained with this measured magnitude. An observation, not a ticket:
`tree_support_branch_distance` (Orca 5 mm vs PnP 1 mm, supports-on) is the other
t82 "PnP does more work" geometry key, unmeasured and parity-scoped. Frontier
now: **no open child remains**; the fog and the next route are the human's.
Standing paired acceptance and KEEP/DROP rules unchanged. Evidence:
[`evidence/t83-arachne-tolerance-price/`](evidence/t83-arachne-tolerance-price/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 5/5 negative controls rejected).

**Previous selection scope (2026-10-23, on the ticket 82 take):** the human's
"continue the perf vs orca map" named no ticket; the frontier re-derivation
found **no open child** (81/81 resolved) and the human selected the
**Fairness-key parity audit** from the route options.
[82](issues/82-fairness-key-parity-audit.md) was graduated, claimed and
**resolved (2026-10-23) as a measurement-only take — no candidate, no timing,
no production edit**. Eight Orca runs produced a **618-key resolved-config
appendix per cell** (identical key set in all eight; only `wall_generator`,
`enable_support` and `print_settings_id` vary by cell) and every key was
classified: 227 in pre-emission geometry scope, 14 `geometry-differs`, 18
`absent-material`, 112 equal (value / unit / rename / canonical clamp), 40
emission-only, 102 absent-and-inert, 332 absent-and-not-geometry.

**Verdict: no third `resolution`-class lever at scale.** The only unmatched
geometry pair where PnP does materially more work than the matched Orca job is
the Arachne wall-simplification tolerance pair — `wall_maximum_resolution`
**0.5 mm vs 0.05 mm** and `wall_maximum_deviation` **0.025 mm vs 0.005 mm**
(PnP 10x / 5x tighter), Arachne cells only, **magnitude unmeasured**. It is
already logged as `D-168-ARACHNE-SIMPLIFY-FALLBACKS-TIGHTER-THAN-CANONICAL`
(re-derive the row from `docs/DEVIATION_LOG.md`) but has never been priced
against wall, and **the generated deviations table cannot see it**: that table
compares against `docs/ORCA_CONFIG_REFERENCE.md`, dated 2026-05-19, and **82 of
the 618 resolved keys postdate the snapshot** — the same blind spot that made
`resolution` a lucky catch rather than a systematic one. Everything else runs
in the other direction or is inert: Orca resolves `top_shell_layers` 4 vs PnP 3,
`support_interface_top_layers` 3 vs 2, `precise_outer_wall` 1 vs 0, and ports
`ensure_vertical_shell_thickness` / `detect_narrow_internal_solid_infill` /
tree-support brim that PnP lacks, so the measured ratios **understate** PnP's
per-unit-work deficit where they differ at all.

**Controls matched in all eight cells:** `resolution = 0.01` (ticket 81's port
is visible in the appendix) and `slice_closing_radius = 0.049` (same key, same
default, applied before the resolution simplify — same order as canonical).
The `slice_closing_radius` half of the fog entry is therefore retired;
`enable_arc_fitting` remains unexpressible and inert (`= 0` on the matched
job). **Observations recorded, not defects proven:** `classic-perimeters.toml`
declares `detect_thin_wall = true` while `arachne-perimeters.toml` declares
`false` and the emitted block records `0` (two modules, one key, two defaults —
matched with Orca by accident); `support_interface_bottom_layers` is declared
`-1` but emits `2`; `brim_width` (8 mm vs 0) is unresolved at the output level
because both captured cells emit a `Brim` role; and Orca's overhang speed bands
are live (50/30/10) while PnP's matched config leaves all four at 0, so t20's
consume-only gate is valid for PnP's job but the two engines are not doing the
same work there. Evidence:
[`evidence/t82-fairness-key-audit/`](evidence/t82-fairness-key-audit/FINDINGS.md)
(`verify.py --self-test` exits 0 checkout-only, 9/9 negative controls rejected;
`key-table.csv` carries all 618 rows). Frontier now: the pricing take is
graduated as
[83](issues/83-arachne-wall-simplification-tolerance-pricing.md) (open,
unclaimed) — price the Arachne tolerance match on the four Arachne cells with
its own pre-declared gate — and the fog carries the snapshot-staleness method
note. The take order is the human's. Standing paired acceptance and KEEP/DROP
rules unchanged.

**Previous selection scope (2026-10-23, on the ticket 81 take):** the human's
"continue the perf vs orca map" named no ticket, so the session took the only
frontier ticket, [81](issues/81-resolution-slicing-time-contour-simplification.md),
and **resolved it (2026-10-23, human-approved KEEP)**. Nine decisions were put to
the human first; the one that mattered was the scope split, and they ruled that
**the map carries this port end to end** rather than handing the parity work to a
spec packet — an explicit carve-out from the Out-of-scope bullet below, recorded
here because it changes what this map is allowed to do.

The port: `resolution` (default 0.01 mm, `normalize_fdm` floor 0.001 mm) applied
at four canonical call sites — the pre-wall per-surface simplify in both
perimeter modules (region granularity; PnP has no `Surface` model), the gap
`douglas_peucker` before `medial_axis`, the classic innermost-contour simplify
before the infill boundary, and the arachne infill contour. Canonical's fifth
site (classic inner-part pass) has no PnP counterpart and is recorded, not
invented. The `0.2 ×` arc-fitting branch is unexpressible (no arc-fitting key)
and inert on the matched job.

**Parity**: the DP is canonical-exact (closed-ring seeding, `<=` acceptance,
strictly-greater tie-break) and the invariant suite pins its actual guarantee;
the union step runs on `clipper2-rust` where canonical uses legacy Clipper 6.4.2
with `StrictlySimple`, so set semantics match and representation does not —
disclosed, not claimed. **Confinement 20/20**: `Layer::Slice` typed IR identical
in all eight cells, `Layer::Perimeters` differs in all eight, support silhouette
byte-identical in the four supports-on cells. **Campaign (gate met)**: classic
cells **−14.31 / −9.00 %** (benchy) and **−22.24 / −10.87 %** (base) wall with CPU
−18…−27 %; Arachne cells wall-neutral (−0.07…−1.87 %); `base arachne-off` failed
its first six pairs at +2.30 %, stopped the campaign by rule, and re-ran at
−2.77 % (combined n=12 **−1.07 %**, spread ±9) — that cell is wall-neutral within
measurement resolution, and the spread is the finding. CPU fell in every cell.
Every cell is byte-deterministic across its runs, and the baseline arm
reproduces the standing t58 reference `34886ccb…`.

Because the port changes geometry, the byte-identity proof regime is replaced by
invariants + stage confinement + determinism, with per-cell candidate SHAs
recorded as the new reference set ([evidence](evidence/t81-resolution-port/FINDINGS.md),
`campaign/results.json`). Frontier now: **no open child remains**; the fog's
`slice_closing_radius` / `enable_arc_fitting` question and the un-graduated leads
are the human's to route. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-22, on the ticket 80 take):** the frontier
re-derivation found no open child (79/79 resolved), and the human's route answer
challenged the map's own fairness contract instead of choosing a route — "has this
study been running on the same resolution as Orca does? The resolution defaults
for PNP are higher". [80](issues/80-resolution-fairness-premise.md) was
graduated, claimed and **resolved as a measurement-only premise audit**: the job
has **not** been resolution-matched. Orca's matched profile inherits
`resolution = 0.01 mm` and applies it as a **slicing-time contour
simplification before walls and infill are generated**, while PnP implements only
emission-time per-role D-P; re-running the same profile at Orca's canonical
0.001 mm floor costs Orca **+9.5 % to +65.2 % wall** across all eight cells
(median **+40.2 %**; CPU **+21 % to +92 %**; G-code **+9 % to +91 %**), 3
alternating appendix-verified pairs per cell. A first attempt at a `resolution =
0` arm was rejected by the appendix gate, which is how the canonical **0.001 mm
clamp** was found — "disabled" is not expressible in OrcaSlicer. The disclosure
is now in the fairness contract above; the implementation candidate is graduated
as [81](issues/81-resolution-slicing-time-contour-simplification.md) (open,
unclaimed) and is explicitly **canonical parity work, not output-preserving**,
so its proof gate cannot be byte-identity against the t58 references. The
feature-gap tracker's `03-asset-scoped-gap.md` adjudication (per-domain emission
keys as `resolution`'s counterpart) covers only the emission half and needs
re-deriving there. No production edit, no PnP build, no candidate, no scoreboard
revision. Evidence:
[`evidence/t80-resolution-fairness/`](evidence/t80-resolution-fairness/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 5/5 negative controls rejected).
Frontier now: **ticket 81 only**; the take order is the human's. Standing paired
acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-22, on the ticket 79 take):** the frontier
re-derivation found **no open child** (78 children, all resolved) and returned
the route to the human, who selected the **t78 constant sweep**.
[79](issues/79-tree-support-planner-emit-batch-constant-sweep.md) was
graduated, claimed and **resolved (2026-10-22) as a measurement-only take —
DROP, no constant changed**. The chunk sweep found the committed
`EMIT_BATCH_LAYERS = 4` at the knee (module bracket 92,875 / 75,503 / **61,519**
/ 65,492 / 62,574 / 76,813 ms at 1 / 2 / 4 / 8 / 16 / 32); the cap sweep chose
`CLIP_BATCH_MAX_REQUESTS = 4096` on a 5.5 % bracket edge over `256` after a
load-confounded pass, but the frozen-snapshot pre-gate confirmation measured it
**+4.97 % and +7.63 % slower on wall** in two byte-identical pairs (paired
median **+6.30 %**, CPU +1.34 %). The standing 16-cell proof and 6-pair campaign
were **not started** by the stop discipline. Source restored byte-identical to
HEAD (blob `a59a11e5…`), both freshness gates exit 0, standing snapshots
rebuilt from the restored HEAD. Evidence:
[`evidence/t79-emit-batch-sweep/`](evidence/t79-emit-batch-sweep/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 13/13 negative controls rejected).
Frontier now: **no open child remains**; the remaining fog is the human's to
route. Standing paired acceptance and KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-22, on the ticket 78 take):** the human's
"continue" named no ticket, so the session took the only frontier ticket,
[78](issues/78-tree-support-planner-emit-batching.md), and **resolved it
(2026-10-22, human-approved KEEP, candidate committed `73a6f1e8`)**.
`build_roles` stays as the serial reference; `build_roles_batched` runs the emit
pass four layers at a time through `clip_polygons_batch` (per-region carve, role
union, set-wide difference, three body-minus-interface rounds, territory
intersect), keeping entry order and every per-call argument identical. **Proof
16/16 PASS**: baseline and candidate are byte-identical in every cell and equal
to the preserved t58 references; the snapshots differ in exactly one module file
per mode (`tree-support-planner.wasm`) and their binaries are byte-identical.
**Standing gate PASS in all four pre-declared classic supports-on cells**,
6 alternating pairs each, 24/24 retained, zero starvation exclusions, every pair
byte-identical: wall **−13.41 % / −12.30 % / −7.22 % / −16.66 %** and CPU
**+0.51 % / +1.51 % / +0.74 % / +2.80 %** (ordinary/accelerated × benchy/base;
accelerated-base's +2.80 % sits 0.2 pp under the pre-declared +3.0 % ceiling).
Attribution: `PrePass::SupportGeometry` **127,723 → 81,362 ms**,
`com.core.tree-support-planner` **109,018 → 60,038 ms** — the 79.7 s clip core
falls to ~30.7 s, a **2.6x** effective speedup rather than the 12x idealized
ceiling. Guest suite 146/0, including an item-for-item parity test against
`build_roles`; guest and workspace clippy clean, `check-literals` 0,
`check-test-quality --report` 0, both freshness gates exit 0. Frontier now: **no
open child remains**; the unswept `EMIT_BATCH_LAYERS`/`CLIP_BATCH_MAX_REQUESTS`
lever and the rest of the fog are the human's to route. Standing paired
acceptance and KEEP/DROP rules unchanged. Evidence:
[`evidence/t78-planner-emit-batching/`](evidence/t78-planner-emit-batching/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 19/19 negative controls rejected).

**Previous selection scope (2026-10-22, on the ticket 77 take):** the frontier
re-derivation found **no open child** (all 76 children resolved), so the route
returned to the human, who selected the **tree-support planner union/carve**
option. [77](issues/77-tree-support-planner-batch-parallel-contract.md) was
graduated, claimed and **resolved as a measured attribution + contract take** —
no production edit, no timing campaign, no commit beyond its records. Measured
at HEAD on base classic supports-on with a reverted host/guest probe whose
output is byte-identical to the t58 reference `f60e5e9b…`: the planner module is
**126.674 s** of the 147.040 s `PrePass::SupportGeometry` stage, and its
**79.659 s** `clip_polygons` core is **99.03 % inside `build_roles`** — 472 role
unions (52.4 %, median 79.2 ms, mean 312 input polygons) and 69,161 per-region
carves (44.9 %, median 0.474 ms). The three shapes the fog pointed at are
refuted by measurement: the `layer_outlines_below` prefix chain is **0.13 %**,
t76's intra-call partition has no target (max call 436 ms) and its sliver
tolerance cannot carry printed geometry, and the ladder unions are 0.7 %. The
surviving contract is **cross-call batching** through the existing
`clip_polygons_batch` — exact per call, input-ordered, fanned out on estimated
work — needing guest-side buffering, not host capability. Idealized ceiling
(arithmetic, unmeasured): **≈ 72.3 s off a 147.0 s stage**. One candidate
graduated: [emit-pass batched carve and role union](issues/78-tree-support-planner-emit-batching.md)
(open, unclaimed). Frontier now: **ticket 78 only**; the take order is the
human's. Standing paired acceptance and KEEP/DROP rules unchanged. Evidence:
[`evidence/t77-planner-contract/`](evidence/t77-planner-contract/FINDINGS.md)
(`verify.py` exits 0 checkout-only with 8/8 negative controls rejected; raw
capture under `target/t77-probe2/`, reproducible from `probe.patch`).

**Previous selection scope (2026-10-21 → 2026-10-22, on the ticket 20 take):**
the human lifted the below-fold park and selected
[20](issues/20-consume-only-when-read-annotations.md) explicitly ("Lift the
park; take ticket 20"). It is now **resolved (2026-10-22, human-approved KEEP,
candidate committed `053904aa`)**. The candidate is the audit's conditional
stamp: a shared classifier predicate
(`slicer_sdk::config_predicates::overhang_annotations_consumed`) is read by
both perimeter generators, and `PathAnnotationMode::Unconsumed` skips both
per-vertex exact scans when nobody reads the stamps. The dynamic premise check
logged `consumed=false` on 495 base classic-off layers; the probe was removed
before any snapshot build. **Proof 16/16 PASS** against the preserved t58
references; the standing paired gate **PASSED in all four pre-declared classic
supports-off cells**, 6/6 pairs each, wall **−25.93 % / −34.47 % / −14.38 % /
−18.10 %** and CPU **−37.67 % / −39.61 % / −24.47 % / −21.58 %**
(ordinary/accelerated × benchy/base), every pair byte-identical. The park lift
was candidate-scoped; the destination and ticket 12's ~1.5x condition are
unchanged. Frontier now: no open child remains; the fog holds the un-graduated
structural leads and the `is_bridge` parity observation. The take order is the
human's.

**Previous selection scope (2026-10-21, on the ticket 76 take):** the human's
"continue" resumed the already-claimed
[76](issues/76-meshanalysis-footprint-union.md); it is now **resolved
(2026-10-21, human-approved KEEP, candidate committed `465f2fd3`)**. The base
overhang union was 326,546 facets costing 15.0-15.9 s of a 19.5-20.0 s stage;
exact input reductions were refuted (0 duplicates and 0 zero-area facets; the
containment drop of 5.9 % saves ~1.5 s; any input reorder changes the serialized
footprint), so the candidate is a work-preserving partition: above 100,000
facets, Morton-ordered centroids are split into a fixed 48 chunks, unioned in
parallel, and merged by one final union. Production stage **19.5-20.0 s ->
4.8-5.9 s**; the region differs only by sub-resolution slivers (4.2e-8
relative) and polygon ordering. **Proof 16/16 PASS**; standing gate **6/6 pairs
in every cell, every pair byte-identical**, wall **-4.46 % / -5.21 %** and CPU
**-0.14 % / -0.93 %** (ordinary/accelerated base classic-on), controls
+2.11 % / +0.71 % wall inside overlapping min-max ranges with flat CPU. Three
campaigns were lost to host stalls and a requested pause; their partial trees
are preserved and unused, and a detached fourth campaign produced the
measurements. Evidence:
[`evidence/t76-footprint-union/`](evidence/t76-footprint-union/FINDINGS.md)
(`verify.py` exits 0 checkout-only with four negative controls rejected).
Frontier now: only [20](issues/20-consume-only-when-read-annotations.md),
parked below-fold; no unclaimed child remains. Standing paired acceptance and
KEEP/DROP rules unchanged.

**Previous selection scope (2026-10-19 to 2026-10-21, on the ticket 75 take):** the human's
"continue" named no ticket, so the session took the first frontier ticket in
order, [75](issues/75-shellclassification-phase-b-commit.md); it is now
**resolved (2026-10-19, human-approved KEEP, candidate committed `07b97cad`)**.
Attribution: the serial phase-B commit was **78–81 % two full-layer Miter
offsets** on base (65 % on Benchy) and only 8–18 % the `internal_bridge_angles`
input step, refuting t27's iteration-1 candidate shape; only 9 of 121 base
entries (13 of 43 Benchy entries) carry a non-empty lower-candidate bridge set.
The candidate precomputes each layer's exact commit result in the parallel
qualification phase and recomputes only those affected layers, so no geometry
function changed. Candidate stage walls: **40.8 → 12.6 s** (base classic-off),
**28.1 → 12.8 s** (base classic-on), **30.1 → 13.6 s** (base arachne-on),
**5.75 → 1.77 s** (Benchy classic-on); **proof gate 16/16 PASS**; standing gate
**24/24 pairs retained**, every pair byte-identical, wall **−3.08 % / −3.48 % /
−3.06 % / −4.11 %** and CPU **+0.42 % / +1.08 % / +0.86 % / +0.51 %**
(ordinary/accelerated × benchy/base). The ordinary-base **+1.08 %** is a
0.08 pp miss of the pre-declared +1.0 % work-preserving ceiling; the human
accepted it on the ticket-27 serialization precedent. The first timing campaign
was evicted by the harness after 4/6 ordinary-base pairs; the partial set is
preserved as `measure-interrupted-20261019` and not used, and the cell was
re-run whole in a detached replacement campaign. Evidence:
[`evidence/t75-shellclass-phase-b/`](evidence/t75-shellclass-phase-b/FINDINGS.md)
(`qualification.json`, `ab-reduction.json`, `split.json`, `verify.py` exit 0
with four negative controls). Frontier now:
[76](issues/76-meshanalysis-footprint-union.md) unclaimed;
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-18, on the t72 take):** the frontier was
re-derived on resume — the only open child was
[20](issues/20-consume-only-when-read-annotations.md), parked below-fold — and
the human selected a **Scoreboard refresh** from the route options (the map's
first full matched-pair re-run since 2026-09-22).
[72](issues/72-scoreboard-refresh.md) was graduated, claimed and **resolved
(2026-10-18) as a measurement-only take: Orca still wins all 8 cells
(3.30x–18.03x median wall; process CPU corroborating in every cell), no cell
within ~1.5x, so the [Gap budget per cell](issues/12-gap-budget-per-cell.md)
below-fold park condition is re-derived and still binding.** PNP ordinary wall
improved in 7 of 8 cells vs the 2026-09-22 table (−13.9 % to −34.6 %; benchy
classic-off +5.4 %), with the Orca-side drift (+1.5 %…+47 %, disclosed) meaning
part of the raw gap improvement is machine state, not PNP work. The DEV-174
supports-on taint is retired at the output level (all 16 PNP groups
`degraded=false`, `non_fatal=0`; ordinary ≡ accelerated bytes in all 8 cells).
Two infrastructure evictions of long background batches were disclosed and
recovered per the t57 discipline (interrupted cells re-run whole; evicted
partial CSVs preserved; base classic-on/arachne-on assembled from per-tool
chunks under a disclosed interleave deviation). Zero starvation exclusions,
n=3 per tool, both freshness gates green in every preflight. Evidence:
[`evidence/scoreboard-refresh-20261017/`](evidence/scoreboard-refresh-20261017/SCOREBOARD.md)
(`verify.py` exits 0 checkout-only + self-test). No production edit, no
candidate, no production-source commit. Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-17, on the t71 take):** the human selected
[71](issues/71-xtask-compile-time-workspace-root.md) from the route options
presented after t70's closeout and approved the commit. It is **resolved
(2026-10-17) as a verified infra repair, committed `f067efa2`**: ticket 70's
xtask compile-time-root mechanism was reproduced contained in a detached
worktree and refined (a foreign invocation overwrites the shared
`target/debug/xtask.exe` with a foreign-baked binary; while the foreign root
is alive the main tree's gates silently scan the foreign tree
indistinguishably, and only once it dies do they print the recorded false
green — "cannot read <dead>\crates" + `watchlist: 0 types` + exit 0). The
fix: `workspace_root()` resolves at runtime (walk up from CWD to the nearest
`Cargo.toml` + `crates/` ancestor; loud panic otherwise; the compile-time
macro is gone from the production path; one fix point covers every
subcommand and derived artifact path), both gates refuse a zero-file scan
(exit 1, every mode incl. `--report`), and the two `#[cfg(test)]` helpers
resolve runtime-first with the compile-time fallback. Behaviorally proven
with a foreign-checkout-built binary: CWD in the main tree → real 159-type
watchlist, exit 0; CWD in a repo-less void → loud exit 101.
`cargo test -p xtask` 185/0 (9 new), workspace clippy exit 0,
check-literals green (159), check-test-quality --report 0 findings,
build-guests --check exit 0; no guest touched. Evidence:
[`evidence/t71-xtask-worktree-root/`](evidence/t71-xtask-worktree-root/FINDINGS.md).
Commit decision is the human's — given 2026-10-17 (approved), the fix is
committed `f067efa2`. Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the
take order is the human's. Standing paired acceptance and KEEP/DROP rules
unchanged.

**Previous selection scope (2026-10-17, on the t70 take):** the frontier was
re-derived on resume — open:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold —
and the human selected [70](issues/70-clipper-in-tree-vendoring-first-candidate.md)
as the route for this take ("Pick 70 (candidate take)"). It is **resolved
(2026-10-17) as a kept production change**: `clipper2-rust` 1.1.0 vendored
in-tree with sealed provenance, the engine's hot-loop software `nearbyint`
round-half-to-even emulation taken to the hardware `round_ties_even`
(44-line candidate, MSRV 1.77 in the vendored crate), value-exact 295/295
corpus cases, 16/16 output-identity cells also matching the t58 preserved
standing references, and the standing paired gate PASS in all four
pre-declared classic-supports-on cells, both modes (wall −15.59 to −18.30 %,
CPU corroborated, 6/6 pairs, zero starvation exclusions). Human-approved
KEEP; candidate committed `7127b4d6`; standing dirs purged pre-commit and
refreshed at the kept HEAD. The closeout surfaced one upstream follow-up —
xtask's `workspace_root()` bakes the compile-time
`CARGO_MANIFEST_DIR`, so a worktree invocation poisons the shared target's
cached xtask until its build units are purged (gates then silently scan zero
files — details on ticket 70's answer) — for the human to route. It is now
ticketed as [71](issues/71-xtask-compile-time-workspace-root.md) (open,
unclaimed; charted on continuation 2026-10-17). Frontier now:
[71](issues/71-xtask-compile-time-workspace-root.md) and
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold; the
take order is the human's. The standing paired acceptance
and human KEEP/DROP rules are unchanged.

**Previous selection scope (2026-10-16, on the t69 take):** the frontier
was re-derived on resume — open:
[20](issues/20-consume-only-when-read-annotations.md) parked below-fold and
[69](issues/69-clipper-vendoring-optimization.md) unclaimed — and the human
selected [69](issues/69-clipper-vendoring-optimization.md) as the route for
this take ("rely on subagents"). It is **resolved (2026-10-16) as a measured
headroom verdict — not parity**: the sealed pristine engine runs **1.42–2.86x
slower than the canonical Clipper2 C++ 1.5.2** per opclass on identical
matched-workload inputs (offset 2.03–2.09x / union 2.61–2.63x / intersection
1.94–2.00x / difference 1.60–1.76x; 295 cases × two passes), with idealized
base-off accumulated ceilings of ≈ 256–258 s (offset) / 199–200 s (closing_ex)
/ 80–82 s (clip_polygons) and a directly-sampled CP-only diagnostic scale of
≈ 1.2–1.5 % — above the t68 below-floor context, so one candidate ticket is
graduated:
[70](issues/70-clipper-in-tree-vendoring-first-candidate.md) (open,
unclaimed). No candidate was implemented, adopted, or timed against the
slicer; the capture probe is reverted (both capture runs byte-identical to
the t58 references), both freshness gates green, no auto-commit. Frontier
now: [70](issues/70-clipper-in-tree-vendoring-first-candidate.md) and
[20](issues/20-consume-only-when-read-annotations.md) (parked below-fold);
the take order is the human's. The standing paired acceptance and human
KEEP/DROP rules are unchanged.

**Previous selection scope (2026-10-16, on the t68 take):** the frontier was
re-derived on resume — open:
[20](issues/20-consume-only-when-read-annotations.md) only, parked
below-fold, with the route decision back to the human — and the human
selected the **classic-perimeters per-layer host-service round trips** lead
from the t60 un-graduated set as the route for this take.
[68](issues/68-classic-perimeters-host-round-trips.md) was graduated,
claimed and **resolved as an attribution take — no candidate**: the 8+3
per-layer round trips decompose per site (guest-observed + host conversion/
ops split) to **97.55 % (benchy-off) / 99.02 % (base-off) native clipper
ops**; the entire round-trip overhead is 2.45 % / 0.98 % and every
reduction direction t60 named (site-level, batching, in-guest migration)
has a measured ceiling ≤ 0.80 % of run wall even if perfectly serialized
(≤ 0.07 % saturated) — below any gate this map has applied. Both captures
byte-identical to the t58 references; probe reverted, freshness green; no
production edit, no timing campaign, no auto-commit. Evidence:
[`evidence/t68-host-roundtrips/`](evidence/t68-host-roundtrips/FINDINGS.md).
Ticket 67's stale `Status: claimed` line was flipped to `resolved` (its
KEEP was already committed as `f4c542a3` / `84ec520a`). Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) remains the only open
child (parked below-fold); the `region_partition` dedup lead remains
un-graduated — route decision back to the human. The standing paired
acceptance and human KEEP/DROP rules are unchanged.

**Same-session addendum (2026-10-16, on the human's direction):** after t68
measured 97–99 % of the round-trip bucket as native clipper ops, the human
directed a new exploration ticket on the engine itself —
[69](issues/69-clipper-vendoring-optimization.md): vendor `clipper2-rust`
in-tree and optimize its hot paths, with the headroom measured against the
bundled canonical Clipper2 C++ first, byte-exactness and the standing gate
unchanged. It is open and unclaimed. Frontier now:
[20](issues/20-consume-only-when-read-annotations.md) (parked below-fold) and
[69](issues/69-clipper-vendoring-optimization.md) (unclaimed) — the take
order is the human's.

**Previous selection scope (2026-10-16, on the t67 take):** the frontier was
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

**Previous selection scope (2026-10-14, on the t60 take):** the frontier was
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
  **Resolution was NOT matched until 2026-10-23** (disclosed 2026-10-22 by
  [ticket 80](issues/80-resolution-fairness-premise.md), closed by
  [ticket 81](issues/81-resolution-slicing-time-contour-simplification.md): PnP
  now implements the same slicing-time simplification). The disclosure below is
  kept because it is what every pre-t81 ratio in this map was measured under.**
  **Pre-t81 disclosure:** The matched Orca
  profile inherits `resolution = 0.01 mm` and applies it as a *slicing-time*
  contour simplification before walls and infill are generated
  (`PerimeterGenerator`'s `surface_simplify_resolution` → `simplify_p`,
  `Fill.cpp`); PnP implements no slicing-time equivalent, only emission-time
  per-role D-P (`gcode_resolution` / `infill_resolution` / `support_resolution`),
  whose defaults are exactly Orca's internal per-role constants — faithful
  porting of a *different* mechanism.
  Running Orca at its canonical 0.001 mm floor instead costs it **+9.5 % to
  +65.2 % wall** across the eight cells (median **+40.2 %**, CPU +21 % to
  +92 %) — so every ratio in this map compares PnP at full contour detail
  against an Orca doing materially less geometric work. Correction factor per
  cell is `1 + Δ` from
  [`evidence/t80-resolution-fairness/summary.json`](evidence/t80-resolution-fairness/summary.json);
  apply it to a current ratio, never quote it as a scoreboard. The
  **fairness-counterfactual column** (raw ÷ (1 + Δ), 2026-10-23) is derived by
  [`corrected_scoreboard.py`](evidence/t81-resolution-port/corrected_scoreboard.py)
  into [`corrected-scoreboard.json`](evidence/t81-resolution-port/corrected-scoreboard.json):
  benchy 3.25/2.50/3.55/3.23, base 8.35/6.75/12.63/11.96 (ordinary, raw ÷ (1+Δ)).
  No verdict flips and the best cell moves **3.30x → 2.50x**, so the ~1.5x park
  condition stays binding. Raw ratios remain the headline.
  **Supports-on engine-style pin (2026-11-02, human ruling on
  [ticket 105](issues/105-supports-on-engine-style-fairness.md); supersedes
  ticket 97's mismatch disclosure):** the matched job now pins Orca's tree
  style to **`tree_strong`**, so both sides run the classic engine class PnP
  implements. Orca's canonical default on a tree family is `smsTreeOrganic`
  (`SupportParameters` → `generate_tree_support_3D`); PnP substitutes **Strong**
  for every canonically-organic input (`DEV-156`). The pin is a **protocol
  choice, not Orca's default job**, and it is disclosed beside every
  supports-on ratio: measured on 2026-11-02 over 4 interleaved paired rounds,
  it costs Orca **+39.6 / +44.4 %** wall on the two base cells and
  **+7.6 / +9.5 %** on the two benchy cells, and changes output
  (**+23.5–35.3 %** bytes; support-interface sections 230 → 423 base
  classic-on, 7 → 57 benchy classic-on). The **effective branch distance is
  1 mm on both sides** (`tree_support_branch_distance_organic` on the Orca side,
  the normal-key default on the PnP side); the deviations table's 5 mm
  normal-key row is inert. **Post-pin scoreboard, re-derived by
  [ticket 106](issues/106-post-pin-supports-on-scoreboard.md) (2026-11-02, 1
  warmup + 3 measured per cell, load 16.4 % → 3.3 %, no starvation
  exclusions):** Orca **6.02x / 6.11x** (base classic / arachne) and
  **4.77x / 4.53x** (benchy classic / arachne), so the park still needs
  **66.9–75.5 %** of each cell removed. Ticket 103's supports-on Orca medians are
  **superseded (pre-pin)** and may be quoted only with that label. The pinned job
  is **byte-identical to ticket 105's `tree_strong` arm** on all four cells while
  running **1.9–9.6 % faster today**, so t105's derived transfer is **retired as
  an absolute predictor** — it agreed on two cells and missed base arachne-on by
  **−17.95 %** — and any style price must be paired same-session, with the day
  named beside any absolute median.
- **Win matrix**: all 8 cells decide, so support-correctness defects are route
  work — a degraded slice is not Orca's job.
- **Measurement mode**: paired ordinary + accelerated runs are the default
  keep/drop gate; the human may accept a one-sided win case by case.
- **Output-changing prepass candidates (2026-11-03, [ticket
  111](issues/111-prepass-rewrite-scope-ruling.md)):** for the **prepass only**,
  the byte-identity proof is replaced by a fresh **16-cell** reference set
  (8 cells x ordinary / accelerated), candidate and baseline arms byte-identical
  **within themselves**, the baseline reproducing the post-t81 set, the
  candidate-vs-old-reference G-code delta audited (feature / TYPE /
  support-interface counts, byte volume, confinement to prepass-derived geometry)
  **before** timing, and the standing paired wall/CPU gate at the **0.80 %**
  floor in **all 8 cells**. The exception is scoped to `PrePass::Slice` (first,
  [ticket 112](issues/112-prep-slice-parity-phase1.md)) and the Strong planner
  (second, [ticket 113](issues/113-strong-planner-phase1.md)); no other parity
  work is authorized, and the next parity question goes back to the human.
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
- **Recipe trap (new 2026-10-27, t89):** a memory-bound host kernel can make
  parallelization *burn* CPU. Twelve-way concurrent exact-Z cross-sections ran
  **92.73 s of accumulated thread-time for a 7.99 s wall**, while the same 431
  queries cost **15.24 s serial** (t88) — a **−5.50 %** wall win for a
  **+6.78 %** process-CPU rise, measured on a 2 %-load machine over three
  alternating pairs. Price any future parallel candidate in thread-time
  (wall × occupancy), not wall alone, and treat "work-preserving" as false the
  moment process CPU rises. A work-reducing serial shape (memoize what does not
  depend on the varying input) is the alternative to check first
  ([ticket 91](issues/91-exact-z-object-geometry-memo.md)).
- **Recipe trap (new 2026-10-27, t91):** a per-object memo is invisible to the
  per-key cache counters. After the z-independent terms were memoized the
  exact-Z service still reported **431 queries / 431 misses** on the base job,
  because the cache key carries the requested Z; the memo was visible only in a
  **geometry-term build** counter (**431 → 1**) and in the query-time split
  (**22.6 → 8.9 s**). Price a memo by the term it builds, not by the hit rate
  ([ticket 91](issues/91-exact-z-object-geometry-memo.md)).
- **Recipe trap (new 2026-10-28, t92):** a per-item / per-surface shape must be
  priced against the per-layer operand build it rides on. The internal-bridge
  qualification's per-surface offsets and intersections are **0.25 s**
  accumulated, while the two whole-layer offsets that build their operands are
  **20.7–22.5 s** — ~80x larger and invisible to a probe that starts after the
  operands exist. Put the operand construction in the same bracket as the
  consumer, or the screen prices the wrong order of magnitude
  ([ticket 92](issues/92-per-surface-bridge-boolean-locality.md)).
- **Recipe trap (new 2026-10-25, t88):** a probe's `Instant` bracket can disagree
  with the event timestamps by the width of an earlier phase (clock-read
  hoisting or a non-monotonic QPC on this host), so a 5 ms clone can read as
  4.09 s while the run is otherwise correct. Anchor probe marks to
  `SystemTime` (`at_ms`) or black-box the clock, and cross-check every
  `Instant` delta against the event timestamps before quoting it
  ([ticket 88](issues/88-supports-on-increment-attribution.md)).
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
- [Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md): **resolved (2026-11-07; measured DROP — 2/8 cells pass)** — per-origin role-carve offsets batched inside the bounded emit flush, output byte-identical in all 16 arm/cell combinations and all 80 output/paired runs clean, but six cells fail the amended gate; base classic-on gains +6.13 % wall with CPU regressing 1.7656 s, while benchy classic-on regresses −17.90 % wall. Candidate and WASMs retired, production restored, 148 planner tests passed. Evidence: [`evidence/t125-occupancy-offset-batching/`](evidence/t125-occupancy-offset-batching/FINDINGS.md) (`verify.py` 442 checks; 10/10 negative controls).
- [Strong planner carry: fold the occupancy batching into the Slice campaign, amend the gate, or park it?](issues/117-strong-planner-carry-ruling.md): **resolved (2026-11-07; human ruling — amend this representation-safe candidate's gate)** — the original Slice carrier is already banked, so the occupancy-offset batch proceeds with the 0.80 % paired wall/CPU gate on the four supports-on cells, byte-identical output in all eight, and no median wall/CPU regression on the four supports-off cells; the 8/8 destination is unchanged. Candidate graduated as [Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md).
- [Post-t123 Orca scoreboard](issues/124-post-t123-orca-scoreboard.md): **resolved (2026-11-07; measurement-only — PnP loses all 8 cells at 2.36x–4.65x ordinary / 2.40x–4.67x accelerated)** — fresh 8 cells × 3 tools × (1 warmup + 3 measured), interleaved, on the post-t123 tree; **zero starvation exclusions**; 96 rows, PnP ordinary ≡ accelerated bytes in 8/8, every PnP hash equal to t123's candidate set, 64/64 PnP `degraded=false` / `non_fatal=0`, 32/32 Orca runs pinned (arc disabled, elephant-foot 0, brim 0) with the repaired filament leg `1.24 / 12 / 220`; required cut **58%–79%** of today's PnP wall (**72%–86%** for 1.5x). Every ratio improved against t121's pre-t123 **2.78x–5.11x**, but the Orca side also moved (**−12.3% … +11.5%**), so no causal claim is taken; the base supports-off PnP deltas (**−24.2% / −25.7%**) agree with t123's paired **−22.2% / −20.9%**. The first batch was killed by the host mid-cell: its `base classic-on` partial is discarded whole and that cell plus `base arachne-on` were re-taken in interleaved batches. Evidence: [`evidence/t124-post-t123-scoreboard/`](evidence/t124-post-t123-scoreboard/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 10/10 negative controls rejected).
- [Facet bridge-path retirement: canonical classification over the whole unsupported set](issues/123-facet-bridge-path-retirement.md): **resolved (2026-11-07; BUILD — 8/8 cells clear the 0.80 % floor at 3.4x–27.8x, CPU corroborating)** — `PrePass::Slice` retires the facet point-in-polygon scan, the 1.0 mm `expansion_margin_mm` offset, the flat-family special case, the postpass unsupported-span gate and the bottom-surface footprint cache, and applies canonical `PrintObject::detect_surfaces_type`'s `opening_ex(diff(current_raw, safety_offset(previous_raw)), 0.1 × external_perimeter_width)` to the region's whole unsupported set (`classify_unsupported_bridge_areas`). **16-cell output:** baseline ordinary ≡ baseline accelerated ≡ the post-t116 set in all eight cells, candidate ordinary ≡ candidate accelerated, candidate moves every cell; benchy **+873 / +832 / +1,281 / +1,159** bytes, base **−20,294 / −20,255 / −20,767 / −20,745**. **Delta audit: 0 structural geometry lines in all eight cells** — wall/support differences are `E`-token-only on unchanged `G1` motion or non-motion commands (`M73`, `M106`/`M107` fan, appendix); no coordinate, feedrate, travel, skirt, brim or raft line moved. **Paired gate (3 interleaved pairs):** median wall **−7.60 / −3.04 / −7.02 / −2.73 %** (benchy classic-off/on, arachne-off/on) and **−22.23 / −7.23 / −20.94 / −7.94 %** (base), **3.4x–27.8x** the floor, process CPU **−1.0 … −110.2 s** in every cell; all eight strictly separated on wall and CPU. **Ledger:** the base win is mostly the removed facet work (1.04x set at 27x the canonical classifier); the benchy set grows 1.85x; the removed footprint is an extra saving the t122 screen did not price. `DEV-200` records the unported expansion/trim; `DEV-039` is closed. Evidence: [`evidence/t123-facet-retirement/`](evidence/t123-facet-retirement/FINDINGS.md) (`verify.py` exits 0 checkout-only, 309 checks; `--self-test` rejects 11/11 negative controls).

- [`PrePass::Slice` parity Phase-1: wave expansion and the facet-path retirement](issues/122-prep-slice-wave-expansion-facet-retirement.md): **resolved (2026-11-06; split verdict — STOP on the combined expansion shape, BUILD the classification-only retirement)** — canonical expands bridges into `stInternalSolid` / `stInternal` / `stTop` **after** perimeters and fill surfaces exist (`LayerRegion::process_external_surfaces`), but PnP's sparse zone and `perimeter.infill_areas` are created only at the `Layer::Perimeters` commit partition; `PrePass::Slice` has none of the three zones. A faithful port is therefore a per-layer change, not a prepass one. The probe reconstructs production inputs (fidelity exact on six sampled layers; canonical area 1,306.72 / 4,653.28 mm² reproduces t112's 1,307.30 / 4,654.62; flat 447.16 / 438.76 reproduces t116's 447.27 / 438.92). Classification-only net **+1,122.9 ms** benchy / **+94,852.4 ms** base; the expansion costs **1,151.9 / 4,976.6 ms** = **102.6 %** of the benchy saving and 5.2 % of the base saving. **The classification-only shape clears the 0.80 % floor in all eight cells at 12.5x–24.5x (benchy) and 89.7x–213.9x (base); the combined shape is negative in all four benchy cells**, so the all-8-cell gate fails. Delta is bounded: benchy added 618.44 / removed 18.45 mm², expansion reach ≤300.64 mm²; base added 564.17 / removed 384.82 mm², reach ≤513.85 mm². **Ledger correction:** the current facet path's pre-gate polygon sum is 14,232.05 / 419,106.22 mm² and at most 5.0 % / 1.1 % survives the gate — on base it produces an almost identical set (1.04x) at 27x the canonical classifier's cost, and `classify_region_surfaces` is 18,275.8 ms of that cost on its own (the `is_bridge` boolean is re-derivable). The classification-only candidate is [ticket 123](issues/123-facet-bridge-path-retirement.md). Evidence: [`evidence/t122-wave-expansion/`](evidence/t122-wave-expansion/FINDINGS.md) (`verify.py` exits 0 checkout-only; `--self-test` rejects 14/14 negative controls).

- [Post-merge per-layer dispatch regression: localize and remove the per-call cost](issues/121-post-merge-per-layer-dispatch-regression.md): **resolved (2026-11-06; the per-layer cost is removed, and the first post-merge Orca scoreboard is recorded at 2.78x–5.11x ordinary)** — the per-layer transport was deriving `prepare_regions` for every module call even though the arena had already computed the same pure function for the staged slice, and it synthesized the prepared view for `Layer::PerimetersPostProcess` even though that stage is outside the prepared-region contract. Reuse the arena record when the perimeter-source projection is the identity and every projected origin is staged; pass `derive_prepared=false` for the non-prepared stage. Four classic cells, 3 interleaved pairs: CPU **−27%…−46%** in **12/12** pairs, wall −9.3%…−37.5% in 11/12 (one +0.66% wall with CPU −37.23%; cell median −14.29%), all four G-code hashes identical. Per-layer base-off **42,406 → 6,758 ms**, below the 7,265 ms pre-merge bracket. The complete 8-cell scoreboard (Orca + PnP ordinary + PnP accelerated, 1 warmup + 3 measured) has ordinary ≡ accelerated bytes in 8/8 and PnP losing all 8 at **2.78x–5.11x ordinary / 2.76x–5.10x accelerated**; the destination's 8/8 is not met, and the required cut is 64%–80%. Ledger correction: the t115 output hashes predate t116; the current references are `35600ee4…` / `596f6da5…` / `6a6a3d9d…` / `8f46fe92…`. Evidence: [`evidence/t121-per-layer-dispatch/`](evidence/t121-per-layer-dispatch/FINDINGS.md) (`verify.py` 8/8 controls; `verify_scoreboard.py` 6/6).

- [Lightning corpus consolidation: share the 5-case captures across the 10 tests](issues/119-lightning-corpus-consolidation.md): **resolved (2026-11-05; corpus shared — 41 → 32 active invocations, 688.89 s → 620.44 s serial module wall, −9.94 %, every assertion preserved)**: a process-global `OnceLock<Corpus>` holds one observation per `(case, arm)` (5 cases × 2 arms = 10 runs) plus its lifecycle trace; the five `fresh_consumers` visual legs, the legacy/comparator/retirement positive captures and the selection external-artifact run read it. The runtime trace emitted exactly 32 starts (`run_visual` 13 / native 5 / preparation 6 / full slice 8), matching the source inventory. The saving is smaller than t114's pre-cache ~423 s estimate because t118 had already made each capture cheap (≈7.6 s of wall per removed capture here), so this is claim hygiene and aggregate CPU, not a large wall lever. Audit fixes: the typed-projection `Arc::ptr_eq` scene check now requests two layers (it was vacuous at one); the test-local selection Boolean is dropped; `max_retained_after_disposal` returns `Option` and cleanup requires `Some(0)`; a real cross-owner `InvalidOwner` control replaces the mislabelled foreign-owner block; the zero-slice selection predicate and store controls moved to `tests/contract/lightning_selection_contract_tdd.rs` (2 tests, 0.00 s). The box was **not idle** (43 % CPU sampled, browsers running), the arms are same-session not interleaved, a second after run measured 611.64 s, and no full-bucket wall or per-test split is claimed. Evidence: [`evidence/t119-lightning-corpus/`](evidence/t119-lightning-corpus/FINDINGS.md) (`verify.py` exits 0 checkout-only; `--self-test` rejects 7/7 negative controls).

- [`PrePass::Slice` candidate: canonical flat-bridge classification](issues/116-prep-slice-canonical-bridge-classification.md): **resolved (2026-11-05; BUILD — 8/8 cells clear the 0.80 % floor at 11.1x–30.5x, CPU corroborating)**: the PnP-local enclosure discriminator is gone, replaced by canonical's `opening_ex(flat_bridge, 0.1 × external_perimeter_width, Miter, 3.0)` over PnP's flat family, canonical's `ApplySafetyOffset` (1-unit lower-layer inflate) and the `lower_layer == nullptr ⇒ stBottom` guard; the `flat_bridge_closing_join` key is retired end to end. **16-cell output:** baseline ordinary ≡ baseline accelerated ≡ the t115 post-merge reference set, candidate ordinary ≡ candidate accelerated, candidate moves every cell. **Delta audit: 0 structural geometry lines in all eight cells** — the wall/support deltas are extrusion-token-only (same `G1` motion, different `E`) on bridge-flagged segments, plus the retired config appendix row and `M73` counters; no coordinate, feedrate, travel, skirt, brim or raft line moved. **Paired gate (3 interleaved pairs):** median wall **7.23 → 5.79 / 11.58 → 10.21 / 7.27 → 5.94 / 11.75 → 10.12 s** (benchy classic-off/on, arachne-off/on) and **106.83 → 83.98 / 176.41 → 160.73 / 106.39 → 80.39 / 185.47 → 164.41 s** (base), i.e. **−8.9 % … −24.4 %**, **11.1x–30.5x** the floor, with process CPU **−16.6 … −288.9 s** in every cell. Narrowest multiple base classic-on at **11.1x**; one disclosed wall-range overlap there (0.55 s). Seven of eight cells strictly separated on wall, all eight on CPU. **Ledger corrections:** the baseline check moved from the post-t81 set (the ticket's pre-merge wording) to the t115 **post-merge** set, which is what HEAD produces; and the retirement of `flat_bridge_closing_join` leaves **no declared key carrying `wire_type: "enum"`**, so the scheduler manifest's enum-metadata assertions retire with it. Disclosures: the campaign was interrupted twice (session recycle mid-pair-2, then a requested pause), so pairs 1–2 and pair 3 come from two windows ~4 h apart — each pair is internally paired; host load was not independently measured (the slicer saturates its own 12 threads); no Orca arm, so no ratio of record — the post-merge scoreboard remains [ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md)'s closing measurement. Evidence: [`evidence/t116-prep-slice-canonical-bridge/`](evidence/t116-prep-slice-canonical-bridge/FINDINGS.md) (`verify.py` exits 0 checkout-only, 316 checks; `--self-test` rejects 10/10 negative controls).

- [Post-merge reconciliation: re-baseline the perf keeps and audit master's semantic drift](issues/115-post-merge-reconciliation.md): **resolved (2026-11-04; the merge is output-neutral on the matched job and regresses the per-layer phase)** — all eight matched cells grow by exactly **+34 bytes**, the added config line `; prepared_plan_diagnostic = none`, and a direct line diff on the four classic cells returns that single hunk (base classic-on `22200671… → 4b6d2843…`, benchy classic-off `3672dc7e… → 39298f40…`), so the reference set moves by a disclosure line and no toolpath needs re-recording. Painted fixtures do move: `cube_4color.3mf` **−1.08 %** bytes with up to **8.19 %** of extrusion redistributed between tools (T0 −8.19 / T1 −3.18 / T2 +1.73 / T3 +5.06 %) while total extrusion moves **−0.11 %**, and `painted_seams.3mf` keeps 111,958 moves with 9,608 reordered lines — the classic paint-attribution fix and variant-chain ordering, with no painted ratio of record affected. Performance: a three-pair interleaved A/B (preserved t103 developer snapshot `4cb96942…` vs the merged ordinary build, load 71 % disclosed) puts **process CPU up in 12/12 pairs** (+24.11 / +16.96 / +53.57 / +34.88 % medians) and the cost entirely in the **per-layer phase** (2.23x / 1.91x / 6.02x / 5.08x; instrumented base-off 7,265 → 42,886 ms = 5.90x, prepass flat at −0.3 %); call counts are unchanged while `fuzzy-skin` goes 162 → 56,068 ms and ten modules land at ~28.5 s, with the per-call cost rising 55 → 141 ms across layers. The accelerated build does not show it (2,318 vs 4,807–5,860 ms, same hash), pointing at a build-mode-sensitive host path. All keeps are structurally present (t78/t81/t90/t95/t101/t102) but their prices were not re-measured. Ruling: **no ratio of record may be quoted against the merged tree**; the post-merge Orca scoreboard is deferred to the fix's closing gate; references move to the post-merge set; the regression is [ticket 121](issues/121-post-merge-per-layer-dispatch-regression.md). Evidence: [`evidence/t115-post-merge/`](evidence/t115-post-merge/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 403 checks, 6/6 negative controls rejected).

- [Lightning test-leg concurrency](issues/120-lightning-test-leg-concurrency.md): **resolved (2026-11-04; test-only screen — 798.61 s → 700.16 s on one warm shared-machine pair)** — `module_plan_matches_recorded_baseline` joins its WASM and native arms, `fresh_consumers_emit_baseline_segments` schedules preparation-only / visual-consumer / cancellation legs on the Rayon pool, and `typed_projection_is_atomic_and_neutral` schedules its projection / failure / empty-scene / G-code runs and joins the two G-code results before comparing. 172 passed / 1 ignored with identical rosters, calicat SHA `d12cdfd1…` unchanged, −98.45 s (−12.33 %) with browser load active, so no idle-box or statistically established effect is claimed; the 6–8 minute projection was not reached. No production code, assertion, roster entry or pipeline invocation removed. Ticket 119 remains open. Evidence: [`evidence/t120-lightning-parallel-legs/`](evidence/t120-lightning-parallel-legs/FINDINGS.md) (`verify.py --self-test` 6/6 mutations rejected).

- [Compiled-component cache in the live load path (e2e dev loop)](issues/118-compiled-component-cache.md): **resolved (2026-11-04; milestone met — full e2e bucket 755.12 s / 12.59 min < 900 s, 172 passed / 1 ignored)** — a process-global `(profile, canonical path, mtime, length)` component cache plus one shared engine per profile cuts a debug calicat slice from **11.732 s to 2.813 s warm** (15.424 s cold; byte-identical G-code `d12cdfd1…`); wasmtime's validated 15 MB disk cache makes separate `pnp_cli` processes hit; the per-layer capture loop now uses the same rayon parallelism as the ordinary path, and the lightning corpus tests run their five independent cases in parallel. Module walls: prep **~1,800 → 267.60 s**, `slice_end_to_end` **200.86 → 49.96 s**, precision **86.66 → 11.37 s**, visual capture **96.22 → 22.67 s**. Full bucket **1,282.18 s → 755.12 s** after case parallelism (21.37 → 12.59 min). Evidence: [`evidence/t118-component-cache/`](evidence/t118-component-cache/FINDINGS.md) (`verify.py` 22/22 checks, 11/11 negative controls).


- [E2E bucket latency: the full runtime e2e suite is the dev-loop bottleneck](issues/114-e2e-bucket-latency.md): **resolved (2026-11-04; Phase-1 screen — the bucket is 85 % per-slice module load, no deletion or lane split recovers it, and the carrying shape is graduated as [ticket 118](issues/118-compiled-component-cache.md))** — one debug slice of `resources/calicat.stl` through the 24 core modules is **11.732 s** median, split **1.833 s** pipeline (validation 4 / prepass 425 / per-layer 1,174 / postpass 230 ms) and **10.011 s** module load + startup + emit; `load_live_modules_for_plan_manifest_first` builds a fresh `WasmEngine` and compiles every on-disk component **serially in the binding loop** with no cache. Serial inventory of 178 tests: the non-whale modules sum to **854 s**, `lightning_migration_tdd` measured **1,411 s for 6 of 10 tests** (~39 min extrapolated) and `preparation_runtime_lifecycle_tdd` is the ticket's recorded **14-of-16-in-30-min** module (~34 min) — the two whales are **83.8 %** of the **5,263 s (≈88 min)** extrapolated suite, whose measured/cited floor is **4,065 s (≈68 min)**. `slice_end_to_end_tdd` is the counter-example: 42 tests in **201 s** because ~10 `pnp_cli` runs back them (23 cached call sites over 3 keys + 7 uncached determinism runs). A reverted 24-thread compile probe (byte-identical G-code `d12cdfd1…`) removed only **2.431 s (−20.7 %)**, so parallelising the loop is **not** the lever; a compiled-component cache is (modelled ceiling **≈ 4.5 min**, `10 s + ~145 slices × 1.8 s`). **Pruning: the first pass's "no deletion justified" was corrected by an adversarial audit** — eight claim fixes/merges adopted (a never-running disk support-blocker test revived; both test-local `scenario_2` fakes deleted and the one real `PaintSemantic` order claim moved to `slicer-ir`; a lying override test renamed; an empty-tolerant pipeline test tightened; `modifier_infill` merged into its identical sibling, **38.76 s → 26.78 s / −11.98 s**; the precision merge landed inside noise, 86.66 → 86.71 s, not claimed). The audit also estimated **~11 min** of duplicate work; the largest piece is graduated as [ticket 119](issues/119-lightning-corpus-consolidation.md) (**41 → 32** active invocations, all assertions preserved). **Lane split: not viable** (a fast lane would be ≈ 22 s of unit/model-IO tests). **The ~237 pipeline starts** are the real bloat: 10.0 s of load each ≈ **39.5 min**, so even perfect deduplication leaves ~77 min; the cache is the carrying fix. **Ledger correction:** the 40-minute baseline was **under-counted** — nine `pnp_cli`-backed tests fail in 0.1 s on the stale-binary gate under plain `cargo test`, which does not build `pnp_cli`; `cargo xtask test` owns that preflight. Evidence: [`evidence/t114-e2e-latency/`](evidence/t114-e2e-latency/FINDINGS.md) (`verify.py` exits 0 checkout-only, 30/30 checks, 12/12 negative controls rejected).

- [Strong planner Phase 1: sub-term decomposition and the occupancy-offset shape](issues/113-strong-planner-phase1.md): **resolved (2026-11-04; Phase-1 screen — stop under the all-8-cell contract, no candidate, no production edit)** — the planner's largest term is **model-occupancy inflation** (`inflate_model_occupancy`'s singular `host::offset_polygons`), **56.2 % / 46.0 % / 51.4 % / 41.4 %** of the planner on base classic-on / benchy classic-on / base arachne-on / benchy arachne-on, split almost evenly between the per-layer emit check and the per-origin role carve; the clip core is **31.0 % / 28.6 % / 35.9 % / 32.4 %**, the drop loop **6.2 % / 10.8 % / 6.0 % / 11.1 %**, contacts **2.4 % / 6.0 % / 2.2 % / 5.4 %**. Batching the offsets through `offset_polygons_batch` is exact by construction (same `slicer_core::polygon_ops::offset` per request, input-order results; t65/t78 precedent), needs **no canonical-parity exception**, and clears the **0.80 %** floor on all four supports-on cells at **12.58x / 7.44x / 7.97x / 4.76x** the floor under t78's measured **2.6x** clamp (18.73x / 11.08x / 11.87x / 7.09x ideal), with zero expected output delta (probe byte-identical to the HEAD baseline on base classic-on `4b6d2843…`, benchy classic-on `3b8fa86a…`, benchy arachne-on `d4644c14…`). **But the all-8-cell gate is structural:** the whole `PrePass::SupportGeometry` stage on the supports-off cells is **300 ms = 0.22 %** of base classic-off and **46 ms = 0.51 %** of benchy classic-off, below the floor even if deleted; the planner module alone is **141 / 23 ms = 0.13x / 0.32x**. No planner-only or support-stage shape can clear 0.80 % there, so the stop rule fired and the carry decision is graduated as [ticket 117](issues/117-strong-planner-carry-ruling.md). **Ledger correction:** t88/t107's "no host batch shape in the non-clip residual" is falsified — the ladders, offsets, MST and node moves are **6.2 % / 10.8 %** of the planner, while the occupancy offset batch is **56.2 % / 46.0 %**; the t104 `87,824 ms` is an instrumented bracket whose composition (≈49.3 s occupancy / ≈27.2 s clip / ≈5.5 s drop) is the transferable result. Evidence: [`evidence/t113-strong-planner-phase1/`](evidence/t113-strong-planner-phase1/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

- [`PrePass::Slice` parity Phase 1: canonical closing vs the artefact-bound gate](issues/112-prep-slice-parity-phase1.md): **resolved (2026-11-03; Phase-1 recommendation — build P1, no production edit)** — canonical has **no enclosure closing**: `PrintObject::detect_surfaces_type` marks every unsupported bottom `stBottomBridge` after `opening_ex(diff_ex(slices, safety_offset(lower_slices), Yes), external_perimeter_width / 10)`, and `merge_bridges`' `closing_ex` is a **≈0.15–0.17 mm** hole filler on the bridge ∪ wave-anchor surface. PnP's 12 mm `closing_ex(support)` + **≥ 10 %** refill gate is a PnP-local discriminator with **no canonical counterpart and no `DEV` row**, so the parity move is to **replace** it, not to compute it cheaper. Shape **P1** = canonical opening over PnP's flat family, every surviving component kept, plus canonical's `ApplySafetyOffset` (1-unit lower inflate) and the width resolved as `resolve_contact_params` does. **Ceiling:** closings are **86.8 %** (benchy) / **70.8 %** (base) of `PrePass::Slice`'s thread-time; net ceiling **1,893 / 37,682 ms** = **15.3–24.7 % / 23.4–50.0 %** of the cell = **19.1x–62.5x** the 0.80 % floor in **all 8** cells, replacement opening **146x / 853x** cheaper. **Delta:** **182 / 239** benchy and **437 / 469** base committed layers change; net **+8.86 mm² (+2.0 %)** / **+115.90 mm² (+35.9 %)** (gate rejected **68.4 / 187.2 mm²**; 0.04 mm opening removed **59.58 / 71.27 mm²** of sub-0.08 mm features); layer 0 pays a **595.8 ms** benchy closing whose result the next stage clears. **P2** (full classification + `assemble_bridge_areas` retirement) **deferred** — canonical's wave expansion (`propagate_waves_ex`) has no PnP counterpart, so its delta is unbounded; **P3** (radius swap) **rejected** on correctness. Consumer map at HEAD adds `arachne-perimeters`, `infill-linker`, `rectilinear-infill`, packet 234a's angle inputs and paint splits to t59's list. Candidate graduated as [ticket 116](issues/116-prep-slice-canonical-bridge-classification.md). Evidence: [`evidence/t112-prep-slice-parity/`](evidence/t112-prep-slice-parity/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 9/9 negative controls rejected).

- [Prepass rewrite: scope, output contract and acceptance](issues/111-prepass-rewrite-scope-ruling.md): **resolved (2026-11-03; human ruling — authorized, scoped and staged)** — a **canonical-parity exception scoped to the prepass** (the second recorded exception after ticket 81's `resolution` port; the rest of the parity program stays out of scope, and the next parity question returns to the human). Both carrying shapes are covered, but **`PrePass::Slice` is the first campaign** (**41,472 ms = 51.2 %** of base classic-off, **44,558 ms = 19.5 %** of base classic-on, present in all 8 cells) and the **Strong planner** is second (**87,824 ms = 38.4 %** of base classic-on, **2,943 ms = 23.6 %** of benchy-on; graduated as [ticket 113](issues/113-strong-planner-phase1.md), blocked by 112; it may still turn out representation-safe and need no exception). Acceptance: a fresh **16-cell** reference set, candidate and baseline arms byte-identical **within themselves**, the baseline reproducing the post-t81 set, the candidate-vs-old-reference G-code delta audited (feature / TYPE / support-interface counts, byte volume, confinement to prepass-derived geometry) **before** timing, and the standing paired wall/CPU gate at the **0.80 %** floor in **all 8 cells** — no four-cell subset. First step is a **Phase-1 parity and consumer analysis**, graduated as [ticket 112](issues/112-prep-slice-parity-phase1.md); no production edit before it clears the floor with a bounded delta, and a shape that only clears base classic-off is not enough. **Staging:** the rewrite is **stage 1** of a multi-stage engine program; the 8/8 destination stays, and the per-layer/emit program is the next stage's fog (deleting both largest prepass stages still leaves base at **2.86x–4.00x** inside t104's instrumented arithmetic against Orca's quiet walls). **Ledger caveat:** t110's base-on **9.49x / 16.3x / 19.6x / 13.7x** multiples divide instrumented PnP spans by quiet Orca walls (t104: that pass ran **+43.6 %** over its t103 median; the uninstrumented paired record is **6.02x**, t106), so they are treated as upper bounds until corrected. **Postscript (2026-11-04, [t113](issues/113-strong-planner-phase1.md)):** the all-8-cell clause is structurally unattainable for the Strong planner — the whole `PrePass::SupportGeometry` stage on the supports-off cells is **0.22–0.51 %** of the cell, below the floor even if deleted — so the carry decision is [ticket 117](issues/117-strong-planner-carry-ruling.md).

- [Orca-side attribution: what its matched-job wall is made of](issues/110-orca-side-attribution.md): **resolved (2026-11-03; measurement-only — no per-stage timing channel works on this install, and the phase split that is observable reverses the map's reading of its own ratios)** — the CLI log buffers (27 stage markers in **3 bursts**, 18 of 27 together, no timestamps at `--debug` 3/4/5), ETW is refused without elevation (`wpr` policy failure / `xperf` invalid flags), and `--pipe` accepted the flag but never connected in two attempts, so the boundary is a **~15 ms directory tap** on `plate_1.gcode.tmp`. Orca's cells are **import-dominated**: startup + model import is **1,229.8 ms (65.9 %) / 1,229.8 (49.0 %) / 10,130.0 (73.3 %) / 10,130.0 (42.0 %)** of the benchy-off / benchy-on / base-off / base-on wall (1,867.5 / 2,509.8 / 13,814.5 / 24,140.3 ms), and Orca's import on the same 123 MB `base.stl` is **≥ 14.3x** PnP's (≤ 0.708 s outside-phase time). Orca's **whole slicing** phase is **637.8 / 1,280.1 / 3,684.5 / 14,010.3 ms** against PnP's **7,365 / 11,784 / 80,216 / 228,317 ms** = **11.6x / 9.2x / 21.8x / 16.3x**, so the end-to-end ratios of record (5.86x / 9.49x / 4.34x / 4.96x today) **understate the engine gap by 1.7x–3.7x** — PnP's fast import was offsetting part of a much larger slicing deficit. Prepass multiples **≥ 40x (bound) / 17.5x / 34.4x / 19.6x**, emit multiples **5.0x / 4.4x / 5.8x / 7.3x**; the prepass is the carrying phase on **both** engines (Orca 57.1 % / 74.6 % of base slicing, PnP 89.5 % / 89.4 %), and on the t105-pinned supports-on job Orca's prepass grows **+8.34 s** where PnP's support stages add **~113.9 s** (**13.7x**; the planner alone is **8.4x** Orca's whole supports-on prepass). The feature stack is nearly free for Orca: 1 wall loop, 1 shell, 0 % infill, no support = **13,639.6 ms** against 13,814.5 ms (**+0.1 s**), which kills "Orca does less prepass work". **Ledger correction to t104:** its Table 3's "Orca's whole slice (2.03 / 2.29 / 15.28 / 19.49 s)" are *process* walls, 66–73 % import on base; its shares and counterfactuals are unaffected. Disclosures: `--info` is an upper bound on import, so Orca prepass/slicing are lower bounds and the multiples upper bounds; 1.8–6.5 % repeat spread; load 0.25–0.43 system CPU; mixed vintage (PnP t104, Orca today, ~9 % faster than t103); the abandoned STL-prefix control (non-manifold truncation → `--info` repair never returned) and the failed `--pipe` attempts are recorded. Evidence: [`evidence/t110-orca-phases/`](evidence/t110-orca-phases/FINDINGS.md) (`verify.py` exits 0 checkout-only; 7/7 negative controls rejected). **Ledger caveat (2026-11-03, t111):** the base-on **9.49x / 16.3x / 19.6x / 13.7x** figures divide instrumented PnP spans by Orca's quiet walls; the uninstrumented paired record is **6.02x** (t106), so they are upper bounds until corrected.

- [Harness defects A1/A2: conditional `host-algos` and the narrow-run example](issues/109-xtask-host-algos-injection-and-narrow-run-example.md): **resolved (2026-11-02; both fixed — harness/doc take, no floor applies)** — `xtask test` injected `--features slicer-core/host-algos` on every selection, so `cargo xtask test -p <pkg>` exited 101 for the six packages that do not depend on `slicer-core`; `build_test_arguments` now injects only when the selection includes `slicer-core` (parsing `-p`/`--package`, `--workspace`/`--all`, `--exclude`, `--manifest-path`, attached forms and `name@version`), and the pure predicate carries five unit tests. `cargo xtask test -p xtask --summary` goes from exit 101 to **exit 0 / 193 passed**, `-p slicer-config` reaches its tests, and `-p slicer-core` still compiles with the feature. `AGENTS.md`'s narrow example pointed at `slicer-runtime`'s `contract` bucket, which matched **0 tests (314 filtered)**; it now names `slicer-scheduler`'s real `scheduler_contract` target and runs **9 passed, 0 failed**. Adjacent, not fixed: `slicer-config`'s census is red before and after (**293 vs 281**, plain cargo reproduces), and `--no-run --summary` prints a false build-failure line beside `VERDICT: PASS`. Evidence: [`evidence/t109-harness-defects/`](evidence/t109-harness-defects/FINDINGS.md) (`verify.py` exit 0; 5/5 negative controls rejected).

- [Dev-loop latency: the `xtask test` gate](issues/108-dev-loop-latency-preflight-concurrency.md): **resolved (2026-11-02; human-directed scope extension — KEEP, candidate committed)** — the three `xtask test` preflights (`check-literals`, `check-test-quality --report`, `build-guests --check`) were strictly serial read-only scans; they now run concurrently behind `run_preflights_with` (`xtask/src/test.rs`), with the guest-freshness check on the calling thread because `Invocation` owns a non-`Send` canonical-WIT loader closure. **Median invocation 18.771 s → 10.802 s (−7.969 s, −42.4 %)**, four interleaved pairs with frozen harness binaries, non-overlapping arms (slowest after 13.214 s < fastest before 18.438 s), pre-declared floor **5 s** cleared 1.6x; components measure 8.196/5.621/2.731 s and 9.375/7.686/4.543 s (serial sums 16.548/21.604 s), so the attainable saving is `sum − max` = 8.35/12.23 s and the end-to-end delta lands inside it. Reporting order is unchanged, so a source-gate failure still aborts before any guest rebuild. Inventory: warm no-op check/clippy 0.8–2.8 s, one-line edit to `slicer-ir`/`slicer-core` 65.3/72.0 s, **first codegen of the `contract` bucket 994 s vs 17.26 s incremental**, relink-only 12.8 s, bucket execution 99.8 s, contract binary 89,697,792 B beside a 770,166,784 B PDB. **Falsified:** check/clippy artifact split (1.7–2.4 s alternation, 0 re-checks — the earlier minutes-scale readings were a dirty test target), feature-set thrash for `slicer-runtime` (0.72 s no-op), linker share on incremental relinks, and `debug = 0` as a speedup on the incremental path (28.24 vs 17.26 s). **Adjacent defects recorded, not fixed:** A1, the unconditional `--features slicer-core/host-algos` injection breaks `cargo xtask test -p <pkg>` for packages that do not depend on `slicer-core` (`-p xtask`, `slicer-ir`, `slicer-config`, `slicer-scheduler`, `slicer-gcode`, `slicer-helpers`); A2, `AGENTS.md`'s narrow filter `core_module_ir_access_contract_tdd` matches **0 tests** (`314 filtered out`). Not priced → fog: cold-build `debug` level, cold-build linker, `cargo-nextest`, `sccache`, CI job graph/caching. **Blaze (ra-jit) was tried, not datasheeted:** #1527 has no installable Windows archive (packaging only, installer unlanded); #1591 builds here in **57m34s** and starts a daemon but needs manual ACL and `hard_link`/`read_link` workarounds, then refuses a `serde` + `serde_json` binary at **241 std instances that need the LLVM backend** (the author's LLVM build stops at the `f16`/`f128` build-script codegen); its LLVM rebuild was abandoned after **92 minutes**. **On WSL2 it runs and loses to cargo on this repository:** against cargo 1.96.0 in Ubuntu 24.04 (12 CPUs), blaze's warm edit is **23-33 ms vs 248-779 ms** on a toy crate, but on `slicer-helpers` cold build is **193 s vs 54 s**, warm edit **12.4 s vs 0.95-1.13 s** (crate root), **11.9 s vs 1.4-3.3 s** (module) and **15.8 s vs 2.4-3.5 s** (function body), and `blaze test` is **312 s vs 49 s** (all 18 tests pass under both). **No blaze speedup is claimed for this repo.** Evidence: [`evidence/t108-dev-loop-latency/`](evidence/t108-dev-loop-latency/FINDINGS.md) (`verify.py` exits 0 checkout-only, 33/33 claims, 12/12 negative controls).

- [Support-geometry producer decomposition](issues/107-support-geometry-producer-decomposition.md): **resolved (2026-11-02; measurement-only Phase-1 screen — measured-dead, the premise named the wrong stage; no candidate, no campaign, no production edit)** — `PrePass::SupportGeometry` is **two stages sharing one stage id**, and ticket 104's reducer summed them. Re-pairing t104's committed raw streams: the **host producer is 9 ms (0.0039 % of the cell, 0.0068x the 1.318424 s floor) on base classic-on and 0 ms on benchy classic-on**; the **planner module stage is 91,893 ms (40.13 %, 69.70x floor)** and **3,260 ms (26.17 %, 32.85x floor)** — `com.core.tree-support-planner` **87,824 / 2,943 ms (95.6 / 90.3 %)** plus `com.core.traditional-support-planner` 300 / 30 ms. The supports-off brackets (231 / 43 ms) are the planner merely declining with the producer at 4 / 0 ms. The producer's whole job is a **coarse outline set — 577 polygons / 444,259 points over 240 layers (~7.1 MB)** on benchy-on per `support-preview` — which is why a deep copy costs sub-millisecond to 9 ms and why the sentinel intermediate entries (exactly 1 on benchy-on) are inside the same bracket; **the entire producer is two orders of magnitude below the floor, so no shape on it can be a route** and the stop rule fired before candidate pricing. The real output is the **ledger correction**: t104's 91,902 / 3,260 ms is essentially the **planner**, i.e. [ticket 88](issues/88-supports-on-increment-attribution.md)'s 55.5 s term refreshed to an implied **64.5 s** of the 160.857 s base-on cell and **3.25 s** of the 12.403 s benchy-on cell, and the supports-on carrying shape of the destination's prepass-rewrite decision is the planner, not the geometry producer. Disclosures: no new slice pass, no wall claim (the split is a bracket property of t104's 2026-11-02 instrumented streams), no probe build (patch authored and reverted unbuilt — `target/release/pnp_cli.exe` remains `4cb96942…` on a clean tree), Arachne unmeasured, preview used for counts only. Evidence: [`evidence/t107-support-geometry-producer/`](evidence/t107-support-geometry-producer/FINDINGS.md) (`verify.py` exits 0 checkout-only, 71/71 claims, 9/9 negative controls rejected).

- [Post-pin supports-on scoreboard re-derivation](issues/106-post-pin-supports-on-scoreboard.md): **resolved (2026-11-02; measurement-only — no PnP run, no config change, no production edit)** — with Orca pinned to `tree_strong` the four supports-on ratios are **6.02x / 6.11x** (base classic / arachne) and **4.77x / 4.53x** (benchy classic / arachne); the park still needs **66.9–75.5 %** cuts. Every post-pin Orca median is slower than ticket 103's pre-pin one (**+37.04 / +14.49 / +13.74 / +5.69 %**), so all four ratios fall from the archive (8.25x → 6.02x, 7.00x → 6.11x, 5.43x → 4.77x, 4.78x → 4.53x). The t105 transfer **agreed on two cells and missed two**: base arachne-on measured **26.365 s** against a predicted **32.135 s** (**−17.95 %**, tolerance 4.670 s), benchy classic-on **2.600 s** against **2.503 s** (**+3.89 %**, 0.004 s past a 0.093 s tolerance). The misses are **vintage, not the pin**: the pinned job is **byte-identical to t105's `tree_strong` arm on all four cells** (44,803,778 / 43,802,018 / 9,481,130 / 10,331,360 B, **+23.46–35.34 %** over t103's pre-pin volumes; support-interface sections 423 / 426 / 57 / 58) yet runs **1.9–9.6 % faster today**, while t103's base arachne-on baseline sits **+12.25 %** above t105's organic arm on **2** retained samples with a 4.67 s spread. Orca g-code **SHA changes every run** (the appendix embeds timing), so SHA is never a job-identity signal on the Orca side; output volume is. Take ran quiet (load 16.4 % → 3.3 %, 12/12 measured samples retained, no starvation exclusions) after a game process and a **sibling checkout's** `cargo test` e2e binary exited; `build-guests --check` exit 0. Evidence: [`evidence/t106-post-pin-scoreboard/`](evidence/t106-post-pin-scoreboard/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 75/75 claims, 9/9 negative controls rejected).

- [Supports-on engine-style fairness](issues/105-supports-on-engine-style-fairness.md): **resolved (2026-11-02; measurement + human ruling — PIN Orca to `tree_strong`)** — the pin costs Orca **+39.6 / +44.4 %** wall on the two base cells (20.449 → 29.532 s; 20.514 → 28.627 s) and **+7.6 / +9.5 %** on the two benchy cells (2.422 → 2.651 s; 2.655 → 2.857 s), over 1 warmup + 4 measured per arm per cell with the arms interleaved. The job changes materially: **+23.5–35.3 %** G-code bytes and support-interface sections **230 → 423** (base classic-on) / **7 → 57** (benchy classic-on); PnP's byte volume on the base cells sits **within 3.4–5.1 %** of the `tree_strong` arm and **27.4–30.8 %** above organic, the output-level corroboration of `DEV-156`. The four supports-on ratios move by **−7.1…−30.8 %** (within-take 7.87x → 5.45x and 7.86x → 5.63x base; 5.12x → 4.68x and 4.60x → 4.28x benchy; derived transfer to the record 8.25x → 5.71x, 7.00x → 5.01x, 5.43x → 4.96x, 4.78x → 4.45x) — the best supported-on cell is still ~4.3x away and the park needs **65–73 %** cuts, so this is a fairness fix, not a route step. Implemented as the standing rig's default (`-OrcaSupportStyle tree_strong`, supports-on only, with a per-run appendix assertion; `vendor` restores the organic escape hatch); **ticket 103's supports-on Orca medians are pre-pin** and [ticket 106](issues/106-post-pin-supports-on-scoreboard.md) re-derives them. Disclosures: load peaked at 55.6 % in round 3; two benchy arachne-on organic samples were starved and excluded from pairing (4/4 base pairs positive, +5.39…+9.75 s); benchy classic-on is 3/4 positive with one load-inflated organic round; absolute ratios are mixed-vintage (today's organic differs from t103's Orca by −10.9…+5.9 %). Evidence: [`evidence/t105-tree-strong/`](evidence/t105-tree-strong/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 74/74 claims, 6/6 negative controls rejected).

- [Prepass versus per-layer: what a cell's wall is made of](issues/104-prepass-vs-per-layer-attribution.md): **resolved (2026-11-02; measurement-only — no candidate, no campaign, no production edit)** — the wall is the **prepass**: **68.5–89.5 %** of every classic cell (base-off 89.5 %, base-on 89.4 %, benchy-on 77.0 %, benchy-off 68.5 %) against **7.9–20.1 %** for the per-layer pipeline, so the park's 57–82 % cut maps onto removing **77.8–93.9 % of the prepass**; deleting the whole per-layer phase leaves **3.02x / 4.51x / 4.66x / 7.60x** (never the park), deleting the whole prepass reaches **0.52x / 0.87x / 1.19x / 1.25x**, and the two biggest prepass stages together leave **1.55x–3.34x** (closest: base classic-off **1.55x**, 3 % short). `PrePass::Slice` is the map's largest measured term (**41,472 ms = 51.2 %** of base classic-off; 28.4 / 18.1 / 19.5 % elsewhere) and is bounded by t59's representation-safety verdict on the closing; `PrePass::SupportGeometry` is **40.1 %** of base classic-on and 26.2 % of benchy classic-on (with `SupportAnalysis` ~50 % of that cell) at the `DEV-156` style-substitution site. Ledger: t73's 2026-10-18 base-off split (prepass 115.0 s / per-layer **94.3 s** of a 214.5 s wall) is superseded — per-layer is now **8.7 %** — while t91's 2026-10-27 base-on prepass share **89.0 %** re-measures at **89.4 %** (same semantics); every per-layer fog entry is bounded by a **6.5–12.7 s** phase. **Walls not claimed** (external load, inflation **1.32x–1.80x**, human-approved disclosure, load fell 95.6 % → 16.8 % mid-batch); ratios of record are t103's paired quiet medians; instrumented passes clean and byte-identical to the t87 references (`3672dc7e…`, `eb4817b4…`, `292df501…`, `22200671…`); benchy repeat moves no share by more than **5.92 pp**; Arachne unmeasured; `verify.py --self-test` exits 0 checkout-only (70/70 claims, 9/9 negative controls). Evidence: [`evidence/t104-phase-split/`](evidence/t104-phase-split/FINDINGS.md).

- [Orca re-baseline on the repaired rig](issues/103-orca-rebaseline-repaired-rig.md): **resolved (2026-10-31 campaign, 2026-11-01 disposal; measurement-only — the archived Orca table is superseded, no production edit)** — the rig carries the ruling's three pins and asserts them per run, and **Orca still wins all 8 cells at 3.52x–8.25x** today-vs-today (benchy classic-off 3.78/3.78, arachne-off 3.52/3.59, classic-on 5.43/5.50, arachne-on 4.78/4.85; base classic-off 4.93/4.96, arachne-off 4.72/4.58, classic-on 8.25/8.21, arachne-on 7.00/7.53, ordinary/accelerated). Fresh Orca: **−27.2 / −24.0 / −35.2 / −30.0 %** on the four benchy cells but **+10.2 / +18.3 / +6.1 / +15.6 %** on the four base cells, so **t98's "every archived multiple is optimistic for PnP" is falsified for the base cells** (base classic-on 9.31x → 8.25x, base arachne-on 9.36x → 7.00x) — the archive's error ran both ways and its gaps (2.71x–9.36x) are not comparable. Park re-derived and **still binding**: every cell needs **57–82 %** of its wall removed to reach ~1.5x = **72–102x** the whole 0.80 % floor; release bound **3.52x** (2026-10-25: 2.71x), moved by Orca on benchy, not by PnP (benchy −2.9…+1.9 %; base supports-on −6…−14 % from t102). Supports-on stays the concentration (+36…+67 % over its off sibling). Proof: 16/16 PnP groups `degraded=false`/`non_fatal=0`, ordinary ≡ accelerated bytes in 8/8, PnP output **exactly the t87 post-t81 reference set** in 8/8, both freshness gates exit 0, 32/32 Orca runs resolved the three pins and the filament leg. Two named starvation exclusions (`base-arachne-on-orca-m3`, `base-arachne-on-pnp-ordinary-m3`), three whole-cell re-runs attempted and all spoiled by external load (competing integration-test binary, then 78–96 % browser load); the human approved proceeding with the noise disclosed, all CSVs preserved, and that cell's band (6.95–7.05x / 7.23–7.53x) does not change its verdict. The brim asymmetry (PnP 8 mm vs pinned Orca 0) is disclosed beside the numbers and remains its own human decision. Evidence: [`evidence/t103-orca-rebaseline/`](evidence/t103-orca-rebaseline/SCOREBOARD.md) (`verify.py --self-test` exits 0 checkout-only, 13/13 negative controls detected).

- [Support-attribution side table materialisation](issues/101-support-identity-table-materialisation.md): **resolved (2026-10-31; Phase-1 screen — both exact families are BELOW the floor, so the ticket's own stop rule fires; no candidate, no campaign, no production edit, recommended family 3 "keep")** — the shape census (one deterministic census, identical on all three table arms) is **23,751 rows over 431 bodies** (`rows/body` p50 **50**, mean **55.1**), **34,662,046** demand ids, per-row list length p50 **1,322** / max **2,847**, **`mixed_bodies = 0`** (every row of a body carries a byte-identical list), only **418 distinct list contents** and **428,726 retained ids = 1.24 %** of the blocks — i.e. the table is **one body's demand list cloned once per emitted support path**, and sharing per body would be content-exact and keep **1.24 %** of the blocks and of the **1.2486 GB** heap. Both sides are measured in situ: the allocation side by a per-layer `Instant` window around `assemble_ordered_entities_with_support_identities` (`crates/slicer-runtime/src/layer_executor.rs`) (**412.6 ms** median, table **413.3 / 433.8 / 452.7 ms** vs skip **21.1 / 17.3 ms**), the free side by a sampling-free `drop(layer_irs)` timing (**491.2 ms** median, table **432.4 / 494.1 / 434.4 ms** vs skip **2.614 / 2.890 ms**; 34,796,481 blocks / 1,358,397,544 bytes reproduced, skip arm 62,751 / 107,020,175). **Combined ceiling 821.6 ms uncontended / 926.8 ms most generous = 0.50–0.56 % of the cell = 0.62–0.70x the 1,318 ms floor**, charging 100 % of the parallel assembly time to the critical path. **The headline is a ledger correction to t100**: its 3.468 s free-side ceiling was a **system-heap** artifact, and t102's mimalloc KEEP cut the same free **3.526 s → 0.432–0.494 s (7.1–8.2x)**; the cause-level route had already been banked by the allocator swap. The wall is **not claimed**: five runs on one binary spread **150.195–154.793 s (3.0 %)**, 5.0x the whole ceiling. Consumer audit re-derived: written only by the assembly, **read by no production path** (only `structured_support_identity.rs` and `raft_geometry.rs` under `crates/slicer-runtime/tests/integration/`, both fixture-level), set empty by `crates/slicer-macros/src/lib.rs` and `crates/slicer-runtime/src/anchored_rows.rs`, absent from `crates/slicer-schema/wit/` — and proven unread end-to-end by **5/5 runs byte-identical to the post-t81 reference** (`22200671…` / 42,507,912 bytes) with the table absent. Disclosures: the machine was **not quiet** (owner's browser at 15–56 % of 12 cores; wall drifted 151.3 → 154.6 s), so the ceiling is instrumented and the wall is not claimed; the table arm's drops spread 14 % and the ceiling uses the `min`/`max` envelope, over-charging the lever; the allocation side is summed, not critical-path, so the true wall contribution is smaller; only classic-on was screened; the 1.2486 GB heap figure is derived and agrees with the census to 0.2 %; memory is out of scope (DEV-026) so it is recorded and not priced; probe reverted, both freshness gates exit 0. Evidence: [`evidence/t101-side-table/`](evidence/t101-side-table/FINDINGS.md) (`probe.patch`, `summary.json`, `extract.py`, `verify.py`; `verify.py --self-test` exits 0 checkout-only, 12/12 claims, 15/15 negative controls rejected).

- [Global allocator substitution on the host binary](issues/102-global-allocator-substitution.md): **resolved (2026-10-31; human-approved KEEP — the standing four-cell gate PASSES, candidate committed)** — `pnp_cli` becomes `AccountingAllocator<mimalloc::MiMalloc>` (two host files plus the contract doc), the packager slot `docs/16_slicer_report.md` §"Global allocator contract" already prescribes, so `--report` accounting keeps its shape. Proof **16/16** (both arms byte-identical in every cell, both clean, non-vacuous roles; **8/8** ordinary cells equal to `evidence/t91-exact-z-memo/references.json`; **8/8** ordinary ≡ accelerated; identical 24-module trees, distinct host binaries `db153219…`/`1de06f27…` ordinary and `7f5ad486…`/`9986754e…` accelerated). Pre-gate on ordinary base classic-on: **−7.74 %** wall / **−2.25 %** CPU in 3/3 pairs. Campaign, 2 warmups + 6 alternating pairs per cell: wall **−6.19 / −5.64 / −5.54 / −4.80 %** (−10.19 / −9.42 / −9.04 / −7.41 s) and process CPU **−1.74 / −2.18 / −1.54 / −1.88 %** (ordinary classic-on / arachne-on, accelerated classic-on / arachne-on), **24/24 pairs byte-identical**, 6/6 retained per cell, zero starvation exclusions, **23/24 pairs wall-negative** (accelerated arachne-on #6 **+0.18 %**, cell median −4.80 %); floor multiples **7.73 / 7.05 / 6.92 / 6.00x** against 0.80 % of t91's walls. The **~5–6 %** effect is much larger than the free-only price (**3.527 → 1.302 s = 1.35–1.66 %**, t100) because the allocator acts on every allocation in the process, but it removes **no allocations** — the census still shows the same **34,796,481** blocks, each freed more cheaply, so the cause-level route stayed [101](issues/101-support-identity-table-materialisation.md) (now resolved: below the floor; see that row). Disclosures: the pre-gate ran under a stray integration-test binary pinning ~8 cores (walls 227–266 s) which exited before the campaign; the pre-campaign snapshot records load median **1 %**; today's baseline walls (152–156 s) differ from t91's 164.803 / 166.931 s, so ratios are today-vs-today and the floor is conservative; the change is process-wide (no stage attribution); memory is not gated; one transient `cl.exe` failure on the accelerated build passed on retry (no artifact). Evidence: [`evidence/t102-allocator/`](evidence/t102-allocator/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 12/12 negative controls rejected).

- [Allocation volume of the whole-print layer collection](issues/100-allocation-volume-layer-collection.md): **resolved (2026-10-30; Phase-1 screen — the arena / bulk-free family as scoped is measured-dead: mis-aimed and construction-unreachable; no candidate, no campaign, probe reverted)** — the whole-print free inside `host:gcode_emit` is **34,796,481 blocks / 1,358,397,544 bytes / 3,526,544 µs** (**101.3 ns per block**, reproducing t94's 3.558 s) on ordinary base classic supports-on, and **34,662,668 blocks = 99.6 %** of them are **8–15-byte strings** sampled as `__skirt_…` support family/body ids and `demand-N` ids. That is the `support_entity_identities` side table that `assemble_ordered_entities_with_support_identities` (`crates/slicer-runtime/src/layer_executor.rs`) clones **per support path** — 23,751 rows, each carrying the body's whole `demand_ids` list (**~1,457 ids derived** from the histogram, 34,615,166 strings). Geometry is a rounding error of the shape: **32,673 blocks = 0.094 %** and **105.8 MB = 7.8 %** of the bytes; entities are 30,076 and points 2,782,098. **Bulk-free counterfactual** (same byte volume, pages touched, both pricing orders): **44.5–63.8 ms** for one block after the drop (189.7 ms before it, collection live) and **58.3 / 120.5 ms** for 495 blocks (one arena per layer), so the **ceiling is 3.468 s = 2.10 % of the cell = 2.63x** the 1,318 ms floor — **above** the floor. The family still dies twice: flattening a live collection costs a **73.3–115.1 ms** copy **plus the same 3.527 s storm** (3.60 s, **+73 ms worse**), and an at-creation arena aimed at the geometry would remove **0.094 %** of the blocks. **Allocator arm** (mimalloc): drop **3.527 → 1.302 s measured** (**2.225 s = 1.35 %**, 1.69x floor; **0.796 s / 2.731 s = 1.66 % / 2.07x** after subtracting the probe's measured **0.506 s** sampling overhead) — above the floor, but the shipped allocator is a **packager** decision under `docs/16_slicer_report.md` §"Global allocator contract" and it removes no work. Disclosures: instrumented walls (200.7–270.7 s) are not wall claims; the sampling instrumentation costs 0.506 s; the `before` bulk arms ran with the collection live (2–3x the `after` prices, conservative arm used); the copy arm was first **elided by the optimiser** (`copy_us = 0`) and re-measured with a `black_box`-ed checksum; only classic-on was censused; both freshness gates exit 0; probe reverted. Graduated [101](issues/101-support-identity-table-materialisation.md). Evidence: [`evidence/t100-allocation-arena/`](evidence/t100-allocation-arena/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 12/12 claims, 13/13 negative controls rejected).

- [Split and union the whole-layer clip-area dilation](issues/99-clip-split-union-dilation.md): **resolved (2026-10-30; measured REJECTED on price — the most favourable split is 0.41–0.43 % of the cell, about half the 0.80 % floor, and not representation-safe on top; no candidate, no campaign, probe reverted)** — offsetting every `deep_infill_area` polygon separately and unioning the pieces is genuinely cheaper per call (**0.8230x / 0.8252x**, two opposite pricing orders agreeing within **0.27 %**, both arms reproducing `292df501…` / 21,023,739 bytes) but the call's own share is too small for a 17.5 % cut to clear the floor: **2,434.2 / 2,464.8 ms accumulated** saving = **314.3 / 298.1 ms** wall-equivalent at each run's own cpu/wall = **0.43 / 0.41 %** of the 73.17 s cell, i.e. **0.51–0.54x the 585.4 ms floor**. The union-free lower bound (per-polygon offsets only, 0.8056 / 0.8069x) is **0.47 %** — still below — so no split shape in the family can clear it, because the offsets are **80.6 %** of production. The reusable decomposition: **~80 % per-path Miter offset, ~19 % batched internal union/cleanup, ~2 %** an explicit union of the already-rounded pieces — splitting partitions the cleanup instead of skipping work. Exactness is independently fatal: byte-identical on **1/130** calls, set-equal on **2/130**, **5,329** differing polygons, identical in both orders. Disclosures: instrumented walls only (the two probe walls are 27.0 / 17.9 s above the 73.17 s baseline and 9.1 s apart for the same work, so the production spans they denominate are inflated and only the ratio is read); the differing area was not quantified because the price already decides; probe reverted; both freshness gates exit 0. Evidence: [`evidence/t99-clip-split-union/`](evidence/t99-clip-split-union/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

- [Match Orca's tree-support branch distance in the matched job](issues/97-match-branch-distance-fairness.md): **resolved (2026-10-30; REJECTED on fairness — the premise is false, no profile edit, no campaign)** — `support_style = default` is not style-neutral. Canonical `SupportParameters` resolves it to `smsTreeOrganic` on a tree family (`// Orca: use organic as default`), `PrintObject::_generate_support_material` / `TreeSupport::generate` fork that style to `generate_tree_support_3D`, and the organic engine's config struct (`TreeSupportCommon.hpp`) reads **only** `tree_support_branch_distance_organic`. The matched job resolves it to **1 mm**; PnP's normal-key default is also **1 mm** (`tree-support-planner.toml`), and the supports-on profiles set neither key. So the effective distances are already matched, and t88's 5 mm arm (184,115 → 176,869 ms, −3.94 %, G-code −0.83 %, changed SHA) is a **5x PnP branch-spacing reduction in a different engine**, not a match. The real gap is the style substitution: `TreeSupportStyle::from_config` aliases every canonically-organic input (`default`/`organic`/`grid`/`snug` on a tree family) to the classic engine's **Strong** style because the organic engine is unimplemented (`DEV-156`; organic port queued in `docs/specs/support-generation-remediation-plan.md`). The deviations table's `tree_support_branch_distance` row (PnP 1.0 vs Orca 5.0) compares the **inert** normal key; the effective `_organic` key is 1.0 on both sides and is not registered by PnP. Disclosures: the 2026-10-27 Orca install (`2.5.0-beta`) rejects the rig's file-path `--load-settings` (`Preset was not found in the loaded bundle`) and no longer ships `resources/profiles/BBL/{machine,process}`, so Phase 0 rests on canonical source + `DEV-156`, not a fresh Orca run; that also blocks Orca-side scoreboard refreshes until the rig is updated or a stock Orca restored. Neither freshness gate applies (docs-only; no guest or code change). Evidence: [`evidence/t97-branch-distance/`](evidence/t97-branch-distance/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 5/5 negative controls rejected).

- [Windowed clip area in internal-bridge qualification](issues/96-windowed-clip-area-dilation.md): **resolved (2026-10-30; Phase-1 screen — REJECTED on exactness, no candidate, no campaign, probe reverted)** — the whole-layer `+1.5 × spacing` dilation is **11,672.7–11,813.6 ms** accumulated over 130 calls; a candidate-box window (`8 × spacing`, operand `11 × spacing`, result clipped back) keeps **12.2 %** of the deep-area ring vertices and saves **10,653.5–10,780.5 ms = 1.17 s wall-eq = 1.57–1.60 %** of the ordinary base classic-off cell (**2.0x** the 0.80 % floor, two runs within **1.2 %**). It is **not representation-safe**: the windowed arm emits `5fc8fa72…` / 21,023,889 bytes against `292df501…` / 21,023,739, identically in two runs, with the first differing block an `Internal Bridge` at `;Z:21.4` (8 → 10 moves). At that layer the two clip areas are **set-equal inside the window** (`mismatch_polys = 0`), so qualification and per-candidate `area` are unchanged; the failure is `internal_bridge_angles`' `expansion_area ∩ clip_area` term → `boundary_anchor_polylines`: with qualification and `area` unchanged, the nearest anchor must have lain **beyond the window** (or the cut introduced a closer ring), so the anchor reach is not bounded by the planned `3.3 × spacing` and the window moves it. No finite candidate-driven window is safe (mirror of t93, where the same operand's erosion was safe because its consumers are bounded by `2 × spacing`). Secondary: **116/130** calls show a non-zero inside-window set difference (**1,071** slivers). The wall screen was not reached; the single uninstrumented window wall (76.557 s) is unpaired and not a claim. Disclosures: instrumented walls attribution only; margins not swept (failure is unbounded by construction); both freshness gates exit 0. Evidence: [`evidence/t96-windowed-clip-area/`](evidence/t96-windowed-clip-area/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 6/6 negative controls rejected).

- [Batched exact-Z cross-sections in the support validation loop](issues/95-exact-z-cross-section-batching.md): **resolved (2026-10-30; human-approved KEEP, candidate committed `afe9c361`)** — the 431 anchor Z values were 431 distinct `(object, region, z)` keys, so the exact-Z cache never hit and every entry paid `slice_mesh_ex(mesh, &[z])`, a one-element Z array over `tmp/base.stl`'s **2,461,234** triangles. Phase 1 (probe, 3 slices, all outputs equal to the post-t81 references): **431/431** planes equal polygon-for-polygon; **6.42 / 6.72 / 6.84 s** of single-Z walks collapse to **0.99–1.05 s** in one batched walk at the object's sorted Z set = **5.37 / 5.73 / 5.83 s** saved = **3.26 / 3.48 / 3.49 %** of the cell, **4.07–4.37x** the 0.80 % floor; the batch buys **6.4–6.9x**, not 431x (two binary searches over 431 planes plus a straddle test per triangle, memory-latency-bound). Candidate: per-`(object, z_units)` cross-section memo + `ExactZQueryService::prefetch_cross_sections`, called once per object before the serial `validate_entry` loop; per-`(object, region, z)` contract, single-Z miss path and per-entry degradation unchanged; host-only. **Proof 16/16** (arms byte-identical, 8/8 ordinary equal to the post-t81 references, 8/8 cross-mode equal, 24 identical modules per mode). **Gate PASS in all four base supports-on cells**, 6/6 pairs, zero exclusions, **24/24 pairs byte-identical**: wall **−3.97 / −3.19 / −2.86 / −2.64 %**, CPU **−0.70 / −0.70 / −0.90 / −0.15 %**, wall deltas **−6.17 / −5.68 / −5.43 / −5.49 s**. One **disclosed** non-negative pair (accelerated classic-on #4, **+2.35 %**, during a load event that slowed both arms ~10 s). Evidence: [`evidence/t95-exact-z-batch/`](evidence/t95-exact-z-batch/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 7/7 negative controls rejected).

- [Remaining whole-print collection drop inside G-code emission](issues/94-remaining-whole-print-drop.md): **resolved (2026-10-29; Phase-1 screen — measured-dead, no candidate, no campaign, probe reverted)** — the `host:gcode_emit` bracket is **93-94 % drop**: rotation 6-7 µs, walk **176-253 ms**, free **3,242-3,558 ms**, bracket 3,415-3,811 ms (ordinary base supports-on, 495 layers, both cells byte-identical to `22200671…` / `0b29e1bf…`). **Parallel free (`into_par_iter().for_each(drop)`) is a wash**: classic **−0.321 s (+0.19 %** of the cell), arachne **+0.200 s (−0.12 %)** — 12 threads buy 9 % in one cell and lose 6 % in the other, so Windows' process-heap serialisation, not thread count, sets the cost. **Layer-by-layer consumption is a structural zero**: the walk is serial and totals 176-253 ms, so interleaving the same serial frees with it overlaps nothing. **Deferred free overlapping the post-emission tail** has an absolute ceiling equal to the tail — **1,603 ms classic (0.97 %)** and **1,367 ms arachne (0.82 %)** against the **1,318 / 1,335 ms** floor (**1.22x / 1.02x**) — before any overlap loss, wake-up or contention with the concurrent 42 MB serialisation, so under the pre-declared **strict four-cell wall** gate the screen does not justify a campaign. What the screen leaves is structural, not strategic: the free is an **allocation-volume** cost (~5.3 GB peak working set, t79), and only fewer/larger allocations (arena or bulk-free collection) can move it. Disclosures: load 8 % at start (not a quiet campaign), one probe run per arm per cell, instrumented walls never a wall claim (arachne serial vs parallel walls differ 4.8 % in the *opposite* direction of the 0.20 s drop difference). Evidence: [`evidence/t94-remaining-drop/`](evidence/t94-remaining-drop/FINDINGS.md) (`verify.py --self-test` exits 0 checkout-only, 8/8 negative controls rejected).

- [Whole-layer deep-infill offsets in internal-bridge qualification](issues/93-whole-layer-deep-infill-offsets.md): **resolved (2026-10-29; measured GATE NOT MET on the strict four-cell wall clause — human-approved DROP, candidate reverted; `candidate.patch` preserved)** — Phase 1 screened four shapes against the map's **0.80 %** floor: **lazy dilation dead** (0 of 130 calls skippable; 130 of 130 have a surface reaching the deep area), **cheaper predicate rejected** (Miter dilation/erosion are not adjoint), **duplicate recompute dead** (9 recomputes, 0 with an identical deep-area operand; deleting all 9 reaches only **0.28 %**), and **windowed erosion above floor** (**9,244.1 / 9,311.3 ms** accumulated saving = **1,006.9 / 1,045.6 ms wall-eq = 1.38 / 1.43 %** of the cell; two runs agree within 3.70 %). The candidate clipped the `−4.5 × spacing` erosion operand to the pre-retain qualified bbox grown by `14 × spacing`; **16/16** proof cells byte-identical between arms, **8/8** ordinary cells equal to the post-t81 references, **8/8** cross-mode equal, 24 identical modules per mode, unit + both calicat internal-bridge e2e tests pass. Campaign (6/6 pairs, zero exclusions, every pair byte-identical): ordinary classic-off **−3.31 % wall / −1.19 % CPU**, ordinary classic-on **−0.26 % / −0.48 %**, accelerated classic-off **+0.13 % / −0.81 %**, accelerated classic-on **+0.62 % / −0.45 %**. The CPU fall matches the probe's 9.2 s (9.7 s measured) and the wall transfer is lifted by cpu/wall 9.65 → 9.84; the accelerated wall misses are inside each cell's own repeat range but positive, so the pre-declared clause is NOT MET and the human ruled DROP. Disclosures: server restart during ordinary classic-on (cell re-run whole, partial preserved), campaign timed only the four base classic cells. Evidence: [`evidence/t93-deep-infill-offsets/`](evidence/t93-deep-infill-offsets/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 18/18 negative controls rejected).

- [Per-surface bridge boolean locality in ShellClassification](issues/92-per-surface-bridge-boolean-locality.md): **resolved (2026-10-28; Phase-1 screen — measured-dead, no candidate, probe reverted)** — the ticket's Phase-2 shapes (per-surface bbox gate, hoistable `1.3 ×` anchor offset, many-piece tiling) have an **absolute upper-bound ceiling of 0.341–0.371 %** of the base classic supports-off cell against the map's **0.80 %** floor (**2,429.6 / 2,303.2 ms accumulated = 271.1 / 249.7 ms wall-equivalent**, two probe runs, outputs byte-identical to `292df501…` and clean); deleting the entire `internal_bridge_angles` step as well reaches only 0.40–0.44 %. Shape counts: **130 calls, 196 surfaces (mean 1.51, max 7)**, `deep_infill_clip_area` 228 polygons (mean 1.75, max 7) — Orca's 128-piece tiling threshold never fires — and the per-surface `offset`/`intersection`/qualification is **0.25 s accumulated** against `internal_bridge_angles`' 2.58 s (t75's 2.4 s angles bracket transfers, its 1.5 s clip bracket does not: today 0.10 s). The screen surfaced a **larger term of a different shape**: two **whole-layer offsets on `deep_infill_area`** — `+1.5 × spacing` **12.5 / 11.4 s accumulated** (130 calls, median 81.6 ms) and `−4.5 × spacing` **10.0 / 9.3 s** (14 qualified calls, max 428.3 ms) — together **3.06–3.43 % of the cell**, ~4x the floor and ~9x this ticket's ceiling; whole-layer operands, so consumer bbox-gating cannot remove them. Graduated [93](issues/93-whole-layer-deep-infill-offsets.md). Disclosures: instrumented wall 79.0–83.0 s vs the uninstrumented 73.17 s cell; ~7–9 % inter-run bucket drift, verdict unchanged; both freshness gates exit 0 before and after. Evidence: [`evidence/t92-per-surface-bridge/`](evidence/t92-per-surface-bridge/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 15/15 negative controls rejected).

- [Memoize z-independent exact-Z query terms per object](issues/91-exact-z-object-geometry-memo.md): **resolved (2026-10-27; human-approved KEEP — wall pass in all four pre-declared base supports-on cells, strict CPU clause missed by +0.08 pp in one, candidate committed `f10cef5a`)** — `ExactZQueryService` memoizes the world mesh, the bounds-derived plate and the constant top-Z cross-section **per object**, keeping the per-`(object, region, z)` cache contract unchanged; the plate is built once instead of on each of 431 misses. Wall **−5.52 / −3.84 / −2.68 / −4.95 %** and CPU **−1.22 / −0.66 / +0.08 / −0.32 %** (ordinary/accelerated × classic/arachne), **24/24 pairs byte-identical**, one SHA per arm, zero exclusions; 16/16 proof cells byte-identical, 8/8 ordinary cells equal to the derived post-t81 references, 8/8 cross-mode equal. The **+0.08 pp** miss is 0.8 s of ~1,053 s process CPU inside that cell's own repeat spread, disclosed and accepted; the ordinary counterpart of the same job measured **−1.22 %**. Probe (attribution only): **431 queries / 431 misses / 431 geometry builds → 1**, `query` **22,636.3 → 8,922.3 ms (−60.6 %)**, `validate_entry` **24,335.0 → 10,004.3 ms**; the request-Z cross-section is the only per-Z work left. `post_planner_gap_ms` falls 14.7→9.9 / 18.4→7.4 / 21.8→9.9 / 13.8→12.1 s. Retained memory: **42.25 MiB** per object on `tmp/base.stl` (recorded, not gated). Evidence: [`evidence/t91-exact-z-memo/`](evidence/t91-exact-z-memo/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 13/13 negative controls rejected).

- [Single-ownership layer collection through finalization and G-code emission](issues/90-postpass-layer-collection-ownership.md): **resolved (2026-10-27; human-approved KEEP — gate met in all four pre-declared base supports-on cells, candidate committed `7b22fccc`)** — wall **−5.54 / −7.56 / −6.54 / −7.64 %** (ordinary/accelerated × classic/arachne) with process CPU **−1.13 … −1.67 %**, **24/24 pairs byte-identical**, one SHA per arm, zero starvation exclusions, every pair moving the right way (−1.56 … −13.20 %). One ownership pass: `execute_layer_finalization_owned` mutates the caller's `Vec<LayerCollectionIR>` and returns it instead of deep-cloning a rollback snapshot no production caller reads (a fatal abort now consumes the working collection, the ticket's pre-authorized delta), `GCodeEmitter::emit_gcode_owned` lets `DefaultGCodeEmitter` rotate and emit the caller's own vector (defaulted so every other emitter is untouched), and `execute_postpass_with_owned_capture` hands the collection over, so it is dropped **once** inside the `host:gcode_emit` bracket instead of being cloned there and dropped again at teardown; the visual-debug capture sinks keep the borrowed path and receive the collection by move. Attribution (instrumented, attribution only): clone **1,765 / 1,684 ms**, commit drop **3,685 / 3,758 ms** and teardown drop **4,490 / 3,723 ms** all gone, the single remaining drop **1.56 / 2.68 s cheaper** than the old clone+drop pair, postpass phase **−66.5 / −70.3 %** — hence measured wall deltas **12.0 / 14.8 / 15.9 / 15.4 s** beat the pre-declared net ceiling **10.76 / 10.29 s**. Exactness 16/16 byte-identical to the derived post-t81 references, ordinary ≡ accelerated **8/8**, both freshness gates exit 0, host-only candidate pinned by `provenance.json` (equal 24-module trees, distinct binaries). Evidence: [`evidence/t90-postpass-ownership/`](evidence/t90-postpass-ownership/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 11/11 negative controls rejected).

- [Parallel exact-Z validation](issues/89-support-plan-exact-z-validation-parallel.md): **resolved (2026-10-27; human-approved DROP — measured GATE NOT MET on the CPU clause, candidate reverted)** — the order-preserving fold is **exact** (16/16 cells byte-identical, 8/8 cross-mode, 160/160 contract, new 64-entry three-way order test) but **not work-preserving**: a 3-pair pre-gate on ordinary base classic-on at 2 % load measured wall **−5.50 %** against process CPU **+6.78 %**, more than twice the pre-declared **+3.0 %** ceiling, so the four-cell campaign was not run. The probe rules out duplicated work (431 entries = 431 distinct `(object, z)` = 431 queries = 431 misses) and splits the tail into requested-Z cross-section **50.96 %**, constant top-Z cross-section **43.39 %**, mesh build **5.65 %** of **92.73 s** accumulated thread-time for a **7.99 s** wall versus t88's **15.24 s** serial — ≈6.1x thread-time per query at 12-way concurrency, i.e. memory/allocator-bound. The optional `(object, z)` cache is **measured worthless** (one entry per anchor Z). Graduated [91](issues/91-exact-z-object-geometry-memo.md): memoize the z-independent terms (**49.04 %** of the tail, ≈**7.5 / 6.8 s** ceiling = **4.1 / 3.6 %** of the cell), which removes work and should lower CPU. Two pre-existing fixture reds (t81 `resolution`, t20 consumer-live stamps) repaired to run the contract suite. Evidence: [`evidence/t89-exact-z-parallel/`](evidence/t89-exact-z-parallel/FINDINGS.md) (`verify.py --self-test` exit 0 checkout-only, 8/8 negative controls rejected).

- [Supports-on increment attribution at post-resolution-port HEAD](issues/88-supports-on-increment-attribution.md): **resolved (2026-10-25; measurement-only — no candidate, no production edit)** — at HEAD `5c7fa03e`, four quiet (5 % load) instrumented base captures plus a loaded corroborating pass and a config-only `tree_support_branch_distance = 5.0` arm; all 1 mm outputs byte-identical to the post-t81 references. The increment is **106.8 s classic / 109.7 s arachne** and is **~69 % support geometry** (planner 55.5 s = 52.0 % of the classic increment, canonical contact detection 10.2 s, per-layer render 5.9 s wall) plus **~31 % host-side** — a **serial exact-Z validation loop** at **15.24 / 13.94 s** (99.3 % of the post-planner gap; every entry independent; cache keys on `region_id` although the computation is object-wide) and **one finalization clone + two whole-collection drops** (finalization 6.76 s, emission 6.26 s = a ~6.15 s drop after a ~5 ms clone, postpass teardown gap 4.58 s). Graduated [89](issues/89-support-plan-exact-z-validation-parallel.md) (parallelize the validation; **8.28 / 7.42 %** of the cell) and [90](issues/90-postpass-layer-collection-ownership.md) (one ownership path; **5.84 / 5.47 %** net, 9.18 / 8.60 % upper). Measured dead: the SupportAnalysis eligibility fold (2.24 s gross, no above-floor removable term), the contact kernel (10.2 s, parallel canonical), the planner (55.5 s, support geometry; t78 batching in place), the per-layer render (5.9 s wall, already parallel). The **`tree_support_branch_distance` 5 mm arm is −7.25 s (−3.94 %)** and **~6.8 s of the increment (6.4 %)** — a modest driver; the parity ruling returns to the human. Probe trap recorded: an `Instant` clock read disagreed with the event timestamps by the width of an earlier phase, so a 5 ms clone read as 4.09 s; all probe claims use the `SystemTime` marks and stage brackets. Evidence: [`evidence/t88-supports-increment/`](evidence/t88-supports-increment/FINDINGS.md) (`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls rejected).

- [Matched-pair scoreboard refresh at post-resolution-port HEAD](issues/87-scoreboard-refresh-post-resolution-port.md): **resolved (2026-10-25; measurement-only — the first fully resolution-matched 8-cell scoreboard)** — at HEAD `6e71c1a1` with both trees rebuilt, Orca still wins all 8 at **2.71x–9.36x**, CPU corroborating (PnP burns 4.1x–21.3x Orca's CPU). Best cell **2.71x** benchy classic-off accelerated (2.75x ordinary; 2.74x benchy arachne-off), worst **9.36x** base arachne-on ordinary. **No cell within ~1.5x → the below-fold park is still binding**, though the release bound moved **3.30x → 2.71x**. PnP ordinary walls fell **39.7–66.2 %** since 2026-10-18; Orca also measured **16.2–35.3 % faster** (drift, disclosed; today-vs-today). **Supports-on is the concentration:** base off 5.18–5.28x vs on 9.20–9.36x, PnP increment **+97.90 / +113.38 s** vs Orca **+4.50 / +5.79 s** with *fewer* PnP support sections. 96 rows / 72 measured, 1 warmup + 3 measured per group, **zero starvation exclusions**, zero validation failures; PnP ordinary ≡ accelerated bytes/SHA in all 8 cells; new post-t81 reference set recorded (base SHAs reproduce t83/t84/t85). Disclosure: an accidental duplicate benchy round is preserved but not ledger input. Graduated [88](issues/88-supports-on-increment-attribution.md). Evidence: [`evidence/scoreboard-refresh-20261025/`](evidence/scoreboard-refresh-20261025/SCOREBOARD.md) (`verify.py` exit 0 checkout-only, `--self-test` 8/8 negative controls).

- [Seam planner inset-offset batching](issues/85-seam-inset-offset-batching.md): **resolved (2026-10-25; human-approved KEEP — gate met in all four base classic cells)** — the seam planner's 495 per-region inset offsets become **one** `offset_polygons_batch`; wall **−7.46 / −2.72 %** (ordinary) and **−7.03 / −3.86 %** (accelerated), CPU **+0.34…+0.47 %**, 24/24 pairs byte-identical, 16/16 output identity, zero exclusions; seam stage **8,356 → 1,737 ms** = **≈7x batch occupancy** (conservative 4.4x, floor 1.15x), so t84's 5.43 % pricing is beaten and the stage drops to 2.3 % of a base cell. Parity is exact by construction, pinned by a per-entry serial-reference test.

- [Accelerated shim workspace-root resolution](issues/86-accelerated-shim-workspace-root.md): **resolved (2026-10-25; infra prerequisite of the t85 take)** — the rustc shim resolved the workspace root from CWD, which is a cargo-registry package directory when cargo compiles a dependency's `build.rs`, so every cold accelerated build died; the accelerated entry points now export `PNP_XTASK_WORKSPACE_ROOT` and `resolve_shim_root` re-validates it (env → CWD → binary dir). `cargo test -p xtask` 187/0, a 48-guest cold accelerated build completes, both freshness gates exit 0.

- [Base common-floor attribution: SeamPlanning and OverhangAnnotation](issues/84-base-common-floor-attribution.md): **resolved (2026-10-24; attribution take — one candidate graduated, two shapes measured dead; no production edit, probe reverted)** — at HEAD `ce8f3992` on base classic off/on, both arms byte-identical to the t81 references `292df501…` / `22200671…`. **SeamPlanning** (9.14 / 8.97 s = 9.84 / 4.30 % of cell wall) is **92.4 / 93.2 %** one serial native `offset_polygons` per region: 495 calls, 8,195 / 8,177 ms; candidate generation 94.6 / 94.3 ms, scoring 16.6 / 16.1 ms, harvest 58.1 / 44.7 ms, host view projection 4.2 / 11.4 ms — the candidate list is **not** the wall problem. **OverhangAnnotation** (10.01 / 10.30 s = 10.78 / 4.94 %) is **99.7 %** the parallel `annotate_overhangs` kernel; 1,455 round offsets (485 band layers × 3) are **85.4 %** of its native work at **11.86x** parallelism; host gather + merge + clone + commit is **0.18 %**. Graduated [85](issues/85-seam-inset-offset-batching.md): batch the inset offsets through the existing `offset_polygons_batch` — upper bound **8.83 / 3.92 %** of wall, **5.43 / 2.41 %** at t78's measured 2.6x. Dropped with measured ceilings: overhang bbox-window operand (retains **97.84 %** of previous points → **0.189 / 0.080 %**) and the seam candidate list (**0.288 / 0.127 %**). Evidence: [`evidence/t84-common-floor/`](evidence/t84-common-floor/FINDINGS.md) (`verify.py --self-test` exits 0, 8/8 negative controls rejected).

- [Arachne wall-simplification tolerance pricing](issues/83-arachne-wall-simplification-tolerance-pricing.md): **resolved (2026-10-24; measurement-only pricing take — DROP, below the map's pre-declared 0.80 % wall floor; no candidate, no parity port, no production edit)** — one binary and module tree, two config-only arms (default 0.05/0.005 mm vs canonical 0.5/0.025 mm), 2 alternating instrumented pairs per Arachne cell plus a 6-pair uninstrumented wall check on the two borderline cells. The tolerance is **real work**: arachne guest fuel **−10.76 %** (benchy) / **−16.21 %** (base), whole-job fuel −3.41/−1.41 % and −11.56/−3.08 %, G-code bytes **−3.84…−8.23 %** with identical TYPE counts, all runs clean and byte-deterministic per arm, and the default arm reproduces t81's base-arachne-off reference `01a1434c…`. It is not a wall lever: the module is only **2.9–6.9 % of cell wall** at **5.1–10.7x** phase parallelism, so the occupancy-normalized ceiling is **0.039–0.376 %** of wall (fuel-proportional bound 0.844 % best case, whose measured elapsed moved +0.8 % the wrong way). The direct wall check agrees — base-off **+0.17 % wall / +0.09 % CPU**, benchy-on **+3.18 % wall / +0.17 % CPU** (paired medians; benchy-on's instrumented −2.9 % was load drift). `D-168` is retained with the measured magnitude. Observed, not ticketed: `tree_support_branch_distance` (Orca 5 mm vs PnP 1 mm, supports-on) is the other t82 "PnP does more work" geometry key, unmeasured and parity-scoped. Evidence: [`evidence/t83-arachne-tolerance-price/`](evidence/t83-arachne-tolerance-price/FINDINGS.md) (`verify.py --self-test` exit 0, 5/5 negative controls rejected).

- [Fairness-key parity audit of the matched job](issues/82-fairness-key-parity-audit.md): **resolved (2026-10-23; measurement-only — all 618 resolved keys classified, no candidate, no timing)** — eight Orca runs gave a 618-key resolved appendix per cell; only `wall_generator`, `enable_support` and `print_settings_id` vary. **Verdict: no third `resolution`-class lever at scale.** The one unmatched geometry pair where PnP does materially more work is the Arachne tolerance pair — `wall_maximum_resolution` 0.5 vs 0.05 mm and `wall_maximum_deviation` 0.025 vs 0.005 mm (PnP 10x / 5x tighter), Arachne cells only, magnitude unmeasured, already `D-168` but invisible to the generated deviations table because **82 of the 618 keys postdate its 2026-05-19 upstream snapshot**. Everything else either makes Orca do more (`top_shell_layers` 4 vs 3, `support_interface_top_layers` 3 vs 2, `precise_outer_wall` 1 vs 0, `ensure_vertical_shell_thickness`, `detect_narrow_internal_solid_infill`, tree-support brim), is layer-0-only, shape-only, or inert. **Controls matched in all eight cells:** `resolution = 0.01` and `slice_closing_radius = 0.049` (same key, default and order as canonical), retiring the fog's `slice_closing_radius` half. Observations: `detect_thin_wall` declared `true` by classic-perimeters but emitted `0`; `support_interface_bottom_layers` declared `-1`, emitted `2`; `brim_width` 8 vs 0 unresolved at output level; Orca's overhang bands live while PnP's are 0. Recommended next step is a pricing take on the Arachne match, not implemented here. Evidence: [`evidence/t82-fairness-key-audit/`](evidence/t82-fairness-key-audit/FINDINGS.md) (`verify.py --self-test` exit 0, 9/9 negative controls rejected).

- [Slicing-time contour simplification (`resolution`) port](issues/81-resolution-slicing-time-contour-simplification.md): **resolved (2026-10-23; human-approved KEEP — committed)** — `resolution` (default 0.01 mm, canonical 0.001 mm floor applied in the resolver so the appendix discloses what the modules apply) now runs at four canonical call sites: pre-wall simplify in both perimeter modules (region granularity — PnP has no `Surface` model), the gap `douglas_peucker` before `medial_axis`, the classic innermost contour before the infill boundary, and the arachne infill contour. Canonical's classic inner-part site has no PnP counterpart (recorded, not invented); the `0.2 ×` arc-fitting branch is unexpressible and inert on the matched job. The DP is canonical-exact (closed-ring seeding, `<=` acceptance, strictly-greater tie-break, tolerance converted across the 1e6 → 1e4 units/mm scale); the union runs on `clipper2-rust` against canonical's legacy Clipper 6.4.2 `StrictlySimple` union, so set semantics match and representation does not (disclosed). **Confinement 20/20** on typed IR (`Layer::Slice` identical in all 8 cells, `Layer::Perimeters` differs in all 8, support silhouette identical supports-on). **Campaign: gate met, 8/8 wall-negative and 8/8 CPU-negative** — classic −14.31/−9.00 % (benchy) and −22.24/−10.87 % (base) wall with CPU −18…−27 %; Arachne −0.07…−1.87 % (wall-neutral); `base arachne-off` failed its first six pairs at +2.30 %, stopped by rule, re-ran at −2.77 % (n=12 −1.07 %, spread ±9 — neutral within resolution). Deterministic (one SHA per arm/cell) and the baseline reproduces the t58 reference `34886ccb…`. This is a **geometry-affecting keep**: the byte-identity proof regime is replaced by invariants + confinement + determinism, with new per-cell references in `evidence/t81-resolution-port/campaign/`. Evidence: [`evidence/t81-resolution-port/`](evidence/t81-resolution-port/FINDINGS.md).

- [Resolution-matched fairness premise for the matched job](issues/80-resolution-fairness-premise.md): **resolved (2026-10-22; measurement-only — the job was NOT resolution-matched; disclosure recorded, one candidate graduated)** — Orca's matched profile inherits `resolution = 0.01 mm` and consumes it as a **slicing-time** contour simplification before wall/infill generation (`PerimeterGenerator.cpp`'s `surface_simplify_resolution` → `union_ex(surface.expolygon.simplify_p(...))` and `ex.douglas_peucker(...)`, `Fill.cpp`'s `resolution` read), while PnP implements only emission-time per-role D-P (`gcode_resolution` 0.0125 / `infill_resolution` 0.04 / `support_resolution` 0.0375) and a collinear-only slicer filter. Re-running the same profile at Orca's canonical 0.001 mm floor (`DynamicPrintConfig::normalize_fdm` clamps anything below it, so "disabled" is inexpressible) costs Orca **+58.4 / +65.2 / +32.1 / +47.3 % wall** (benchy classic-off/on, arachne-off/on) and **+37.5 / +42.8 / +9.5 / +30.9 %** (base), median **+40.2 %**, CPU **+21 % to +92 %**, G-code **+9 % to +91 %**, 3 alternating appendix-verified pairs per cell; `base/arachne-off` wall is noise-limited (CPU +46.3 % stable). The default arm reproduces the historical matched Orca bytes (−0.0003 %). Follow-ups: the map's fairness contract now carries the disclosure, and [81](issues/81-resolution-slicing-time-contour-simplification.md) is graduated as **canonical parity work, not an output-preserving optimization**. Evidence: [`evidence/t80-resolution-fairness/`](evidence/t80-resolution-fairness/FINDINGS.md) (`verify.py` exits 0 checkout-only with 5/5 negative controls rejected).

- [Tree-support planner emit-batch chunk and request-cap sweep](issues/79-tree-support-planner-emit-batch-constant-sweep.md): **resolved (2026-10-22; measurement-only — DROP, no constant changed)** — the chunk sweep found the committed `EMIT_BATCH_LAYERS = 4` at the knee (`com.core.tree-support-planner` module bracket 92,875 / 75,503 / **61,519** / 65,492 / 62,574 / 76,813 ms at 1 / 2 / 4 / 8 / 16 / 32, instrumented screening only), and the cap sweep chose `CLIP_BATCH_MAX_REQUESTS = 4096` on a 5.5 % bracket edge over `256` (58,113 vs 61,519 ms; 3.4 % in an interleaved paired window) after the `64` leader failed to replicate and a pass-3 host-load spike confounded the single-sample cells. **The frozen-snapshot pre-gate confirmation rejected it:** two alternating byte-identical pairs measured the candidate **+4.97 % and +7.63 % slower on wall** (paired median **+6.30 %**, CPU +1.34 %), so the 16-cell proof and 6-pair campaign were **not started** by the stop discipline. Peak working set ~5.3 GB at `4 / 256`. Planner source restored byte-identical to HEAD (blob `a59a11e5…`), both guest freshness gates exit 0, standing `target/dist` and `target/dist-accelerated` rebuilt from the restored HEAD; no production edit, adoption or commit. Evidence: [`evidence/t79-emit-batch-sweep/`](evidence/t79-emit-batch-sweep/FINDINGS.md) (`verify.py` exits 0 checkout-only with 13/13 negative controls rejected; `candidate.patch` is the one-line re-appliable 256 → 4096 diff).

- [Tree-support planner emit-pass batched carve and role union](issues/78-tree-support-planner-emit-batching.md): **resolved (2026-10-22; human-approved KEEP — candidate committed `73a6f1e8`)** — the emit pass buffers four layers of `build_roles` inputs and issues each clip phase as one cross-item `clip_polygons_batch` call (per-region carve, role union, set-wide difference, three body-minus-interface rounds, territory intersect), with `build_roles` kept as the serial reference, `largest_part` shared by both carve paths, and entry order unchanged. **Proof 16/16 PASS** — baseline and candidate byte-identical in every cell *and* equal to the preserved t58 references. **Standing gate PASS in all four pre-declared classic supports-on cells**, 24/24 pairs retained, zero starvation exclusions, every pair byte-identical: wall **−13.41 % / −12.30 % / −7.22 % / −16.66 %** and CPU **+0.51 % / +1.51 % / +0.74 % / +2.80 %** (ordinary/accelerated × benchy/base; accelerated-base's +2.80 % is 0.2 pp under the pre-declared +3.0 % ceiling). Attribution: `PrePass::SupportGeometry` **127,723 → 81,362 ms** and `com.core.tree-support-planner` **109,018 → 60,038 ms**, i.e. the 79.7 s clip core falls to ~30.7 s (**2.6x effective**, not the 12x idealized ceiling); `EMIT_BATCH_LAYERS = 4` and `CLIP_BATCH_MAX_REQUESTS = 256` remain unswept. Evidence: [`evidence/t78-planner-emit-batching/`](evidence/t78-planner-emit-batching/FINDINGS.md) (`verify.py` exits 0 checkout-only with 19/19 negative controls rejected).

- [Tree-support planner native union/carve batch-or-parallel host contract](issues/77-tree-support-planner-batch-parallel-contract.md): **resolved (2026-10-22; measured attribution + contract — no production edit, no timing campaign; one candidate graduated)** — the planner's `clip_polygons` core is **79.659 s of a 126.674 s module stage** (stage `PrePass::SupportGeometry` 147.040 s), and **99.03 % of that core (78.885 s) sits inside `build_roles`**: **472 role unions** (52.4 %; median 79.2 ms, p95 301.6 ms, mean 312 input polygons, max 1,023) plus **69,161 per-region carves** (44.9 %; median 0.474 ms, 1.96 clip polygons). The shapes the fog pointed at are **refuted by measurement**: the `layer_outlines_below` running-union prefix chain is **494 calls / 0.102 s / 0.13 %** (each layer contributes ~1 polygon; the accumulator stays ~1k vertices), t76's intra-call partition has **no target** (no union above 436 ms) and its sliver tolerance does not transfer because the union output *is* printed support geometry, and the 20,275 `ladder_union` calls are 0.662 s. The surviving contract is **cross-call batching** through the existing `clip_polygons_batch` (`crate::batch::map_batch`, fan-out at 4,096 primitives, input-order results): each batch item is the same single operation with the same arguments, so exactness is **per-call identical** (t65 precedent), unlike a partition. Idealized ceiling **72.3 s off a 147.0 s stage** (arithmetic, unmeasured; excludes marshalling 69k requests, buffering memory, imperfect scaling and the 47.0 s of non-clip module time). Probe (`probe.patch`) output byte-identical to the t58 reference `f60e5e9b…`, reverted; both freshness gates exit 0. Evidence: [`evidence/t77-planner-contract/`](evidence/t77-planner-contract/FINDINGS.md) (`verify.py` exits 0 checkout-only, 8/8 negative controls rejected). Graduated: [emit-pass batched carve and role union](issues/78-tree-support-planner-emit-batching.md).

- [Consume-only-when-read annotations](issues/20-consume-only-when-read-annotations.md): **resolved (2026-10-22; human-approved KEEP — candidate committed `053904aa`)** — the only semantic reader of `overhang_quartile` / `overhang_distance_mm` is `OverhangClassifierDefault::run_finalization`, which returns before reading a stamp when `enable_overhang_speed` is false or all four `overhang_{1..4}_4_speed` bands are zero; none of the matched configs authors those keys, so the stamps were unread work. The candidate shares that exact predicate through `slicer_sdk::config_predicates::overhang_annotations_consumed` (the classifier calls the same helper), declares the five keys on both perimeter manifests, and selects `PathAnnotationMode::Unconsumed` when the predicate is false, skipping both per-vertex exact scans. Dynamic premise check: `consumed=false` on 495 base classic-off layers. **Proof 16/16 PASS** against the t58 references; standing gate **PASS in all four pre-declared classic supports-off cells**, 6/6 pairs, every pair byte-identical: wall **−25.93 % / −34.47 % / −14.38 % / −18.10 %** and CPU **−37.67 % / −39.61 % / −24.47 % / −21.58 %** (ordinary/accelerated × benchy/base). `is_bridge` was deliberately untouched (no consumer liveness predicate; t18's hot split is the two scan families). Evidence: [`evidence/t20-consume-only/`](evidence/t20-consume-only/FINDINGS.md) (`verify.py` exits 0 checkout-only with four negative controls rejected).

- [MeshAnalysis exact footprint union](issues/76-meshanalysis-footprint-union.md): **resolved (2026-10-21; human-approved KEEP — candidate committed `465f2fd3`)** — the base overhang union (326,546 facets, 15.0–15.9 s of a 19.5–20.0 s stage) could not be reduced exactly at its inputs (0 duplicate / 0 zero-area facets; the 5.9 % containment drop saves ~1.5 s; any reorder changes the serialized footprint), so the candidate is a work-preserving partition: above 100,000 facets, Morton-ordered centroids split into 48 chunks, unioned in parallel, merged by one final union. Stage **19.5–20.0 → 4.8–5.9 s**; the region differs only by sub-resolution slivers (4.2e-8 relative) and polygon ordering. Proof **16/16 PASS**; standing gate **6/6 pairs in every cell, every pair byte-identical**, wall **−4.46 % ordinary / −5.21 % accelerated** base classic-on, CPU **−0.14 % / −0.93 %**. Evidence: [`evidence/t76-footprint-union/`](evidence/t76-footprint-union/FINDINGS.md) (`verify.py` exits 0 checkout-only with four negative controls rejected).

- [ShellClassification phase-B serial commit](issues/75-shellclassification-phase-b-commit.md): **resolved (2026-10-19; human-approved KEEP — candidate committed `07b97cad`)** — the serial phase-B commit is **78.1 % / 81.0 % / 79.5 % two full-layer Miter offsets** on the three base cells (65.1 % on Benchy), not the `internal_bridge_angles` input step (2.4/1.4/1.8/0.48 s; 8–18 %); the t27 sibling-region memo remains refuted. Only **9 of 121** base entries (13 of 43 Benchy) carry a non-empty lower-candidate bridge set, the sole order-dependent input. The candidate precomputes each layer's exact commit result in the parallel qualification phase and recomputes only those affected layers; no geometry function changed. Candidate stage walls **40.8 → 12.6 s / 28.1 → 12.8 s / 30.1 → 13.6 s / 5.75 → 1.77 s**; proof gate **16/16 PASS** against the t58 references; standing gate **24/24 pairs retained**, every pair byte-identical, wall **−3.08 % / −3.48 % / −3.06 % / −4.11 %** and CPU **+0.42 % / +1.08 % / +0.86 % / +0.51 %** (ordinary/accelerated × benchy/base). Ordinary base's **+1.08 %** is a 0.08 pp miss of the pre-declared +1.0 % work-preserving CPU ceiling; the human accepted it on the ticket-27 serialization precedent. The first timing campaign was evicted after 4/6 ordinary-base pairs; the partial set is preserved as `measure-interrupted-20261019` and not used, and the cell was re-run whole detached. Evidence: [`evidence/t75-shellclass-phase-b/`](evidence/t75-shellclass-phase-b/FINDINGS.md) (`split.json`, `qualification.json`, `ab-reduction.json`, `candidate.patch`, `probe.patch`, `verify.py` exit 0 with four negative controls).

- [LayerFinalization supports-on whole-print view cost](issues/74-layerfinalization-supports-on-view-cost.md): **resolved (2026-10-19; human-approved KEEP — candidate committed `091e6bc9`)** — the supports-on brackets' **96.1 % copy/free churn** (61,555 ms module sum; input snapshot clone 12,112 ms, store drop 16,653 ms, merge rollback clone 13,446 ms, old-vector drop 16,940 ms; guest call 1,816 ms = 2.9 %) is owned by the whole `LayerCollectionIR` clone/free, dominated by the IR-only `support_entity_identities` side table (23,751 rows supports-on vs 0 supports-off), not by module reads or the 1.96x output-byte ratio. The candidate shares the ordered entity snapshot with the view via `Arc` and skips the merge rollback clone only when the order-lock remap is a no-op and the stream has no fallible op (fallible streams keep the rollback; the malformed-merge rollback contract stays green). Probe: **61,555 → 2,775 ms (−95.5 %)**, supports-off unchanged 1,290 → 1,252 ms, outputs byte-identical to the t58 references; proof gate **16/16 PASS**; standing paired gate **PASS** with 6/6 pairs in every cell: wall −3.13 % ordinary-benchy / −12.71 % ordinary-base / −3.89 % accelerated-benchy / −14.23 % accelerated-base, CPU −0.91 % / −2.30 % / −0.84 % / −2.97 %, all 24 pairs byte-identical. Base supports-on wall falls **46.00 s ordinary / 47.52 s accelerated**, inside t73's 45.0–48.5 s bracket attribution. Pre-commit gates: workspace clippy clean, check-literals 0, check-test-quality 0 findings, 24 executor + 10 contract finalization tests pass, both freshness gates exit 0. Evidence: [`evidence/t74-finalization-view/`](evidence/t74-finalization-view/FINDINGS.md) (`qualification.json`, `ab-reduction.json`, `candidate.patch`, `verify.py`).

- [Base current bottleneck refresh](issues/73-base-current-bottleneck-refresh.md): **resolved (2026-10-18; measurement-only take — eight fresh ordinary base captures at post-t70 HEAD, every output byte-identical to its t58 reference; three tickets graduated, t20 park unchanged)** — base is **prepass-dominated**: instrumented wall 214.5/408.7/130.2/360.2 s (classic-off/classic-on/arachne-off/arachne-on), prepass 115.0/237.6/109.2/259.9 s, per-layer 94.3/98.2/16.1/19.8 s, postpass 2.4/69.1/1.4/76.6 s; the common prepass floor is 104–115 s on every cell, supports-on adds 134–149 s of support stages, and ticket 59's Slice verdict (40.0–44.9 s) stands. New leads in recommendation order: **supports-on LayerFinalization view cost** (four whole-print module brackets 10.1–12.5 s each, 45.0–48.5 s total vs 0.30–1.39 s supports-off, while guest fuel is 5.45 B of 12,964 B = 0.042 %), **ShellClassification phase-B serial commit** (26.0–31.4 s; t27's serial commit still 25–30 s), **MeshAnalysis exact union** (19.4–22.2 s serial over 326,546 facets; bbox replacement unsafe). t20 updated with the base context (classic per-layer 94.3/98.2 s; module 98.8/95.0 % of fuel; t18's 57.9 % base scan share) and the below-fold park re-derived as still binding (best cell 3.30x). No candidate, timing run, production edit or production-source commit. Evidence: [`evidence/perf-refresh-base-2026-10-18/`](evidence/perf-refresh-base-2026-10-18/FINDINGS.md) (`verify.py` exit 0 checkout-only + four negative controls rejected; `analyze.py` reproduces the tables).

- [Matched-pair scoreboard refresh](issues/72-scoreboard-refresh.md): **resolved (2026-10-18; measurement-only take — Orca still wins all 8 cells at 3.30x–18.03x, no cell within ~1.5x; DEV-174 taint retired at the output level; two infrastructure evictions disclosed and recovered)** — full refreshed table in the evidence set: median-wall gaps ord/acc — benchy classic-off 5.14/4.34, arachne-off 3.30/3.31, classic-on 5.87/5.09, arachne-on 4.76/4.63; base classic-off 11.48/9.20, arachne-off 7.39/7.24, classic-on 18.03/16.75, arachne-on 15.65/15.50 (n=3 per tool, zero starvation exclusions, CPU corroborating in every cell, PNP 4.5x–41.5x Orca's CPU). PNP ordinary wall improved vs 2026-09-22 in 7 of 8 cells (−13.9 %…−34.6 %; benchy classic-off +5.4 %), while Orca-side walls drifted +1.5 %…+47 % (machine state, disclosed) — ratios are today-vs-today, not a PNP-work claim. Supports-on cells untainted for the first time (all 16 PNP groups `degraded=false`, `non_fatal=0`; ordinary ≡ accelerated bytes). Evidence: `evidence/scoreboard-refresh-20261017/` (`SCOREBOARD.md`, `SUMMARY.txt`, nine curated row CSVs + two `.interrupted.csv` disclosures, `REPRODUCE.md`, `verify.py` exit 0 + self-test).

- [xtask compile-time workspace root poisons gates from alternate checkouts](issues/71-xtask-compile-time-workspace-root.md): **resolved (2026-10-17; verified infra repair, committed `f067efa2`; docs reconciliation this commit)** — ticket 70's poisoning mechanism reproduced in a detached worktree and refined (foreign invocation overwrites the shared `xtask.exe` with a foreign-baked binary; alive foreign root = silent wrong-tree scan, dead = the recorded false green `watchlist: 0 types` + exit 0). Fix: runtime root resolution by `Cargo.toml`+`crates/` walk from CWD (loud panic otherwise; compile-time macro removed from the production path; one fix point covers all subcommands and derived artifact paths) + zero-scan refusal (exit 1) in both gates, every mode incl. `--report` + runtime-first `#[cfg(test)]` helpers. Foreign-built binary proof: CWD=main → real 159-type watchlist, exit 0; CWD=void → exit 101 loud. `cargo test -p xtask` 185/0 (9 new), workspace clippy 0, check-literals 159-green, check-test-quality --report 0 findings, build-guests --check exit 0. Evidence: [`evidence/t71-xtask-worktree-root/`](evidence/t71-xtask-worktree-root/FINDINGS.md).

- [Clipper engine vendoring and hot-path optimization](issues/69-clipper-vendoring-optimization.md): **resolved (2026-10-16; headroom take — measured headroom, not parity; one candidate graduated, none implemented)** — a dual-engine harness (pristine sealed `clipper2-rust` 1.1.0 vs canonical C++ 1.5.2, MinGW g++ -O2) over a fresh host-side input-dump corpus of the 11 t68 sites (both matched jobs, capture runs byte-identical to the t58 references) plus the t55 body-union corpus measured, on 295 cases × two passes (forward + reversed), **Rust/C++ median ratios offset 2.03–2.09x / union 2.61–2.63x / intersection 1.94–2.00x / difference 1.60–1.76x** (every site above 1 in both passes; 46/295 ordered-hash divergences = the known t31 `clean_collinear` delta, disclosed not investigated). Idealized base-off accumulated ceilings if Rust matched C++: offset 256.5–258.3 s / closing_ex 198.9–200.3 s / clip_polygons 79.5–81.6 s (t60 scope totals re-derived at take time); directly-sampled CP-only ceilings ≈ 46.7–46.8 s base / 2.5–2.7 s benchy, an occupancy-style diagnostic scale of ≈ 1.48 % / ≈ 1.21–1.28 % against t68's instrumented walls — above the ≤ 0.80 % below-floor context, hence candidate-justified. Coverage caveats stated on the ticket (RP leg captured intersections only; region-prep closing/offset ceilings conditional on within-opclass transfer; no engine-internal cost center measured). Probe reverted, both freshness gates green, `verify.py` exits 0 checkout-only with four negative controls rejected. Evidence: `evidence/t69-clipper-engine/` (FINDINGS, HARNESS, CAPTURE, corpus, results, drivers, verifier). Graduated: [70](issues/70-clipper-in-tree-vendoring-first-candidate.md).

- [Clipper in-tree vendoring and first value-exact engine candidate](issues/70-clipper-in-tree-vendoring-first-candidate.md): **resolved (2026-10-17; human-approved KEEP — candidate committed `7127b4d6`)** — vendored `clipper2-rust` 1.1.0 in-tree as a plain workspace member with sealed provenance (`vendor-provenance.json`; byte-identical registry extraction, explicit `path` dep so guest freshness fingerprints cover engine sources), profiled the engine internally on the t69 corpus (all phases at parity except `do_top_of_scanbeam` 2.19–4.26x and `build_intersect_list` 1.25–2.26x vs C++; root cause = software `nearbyint` round-half-to-even emulation on the per-scanline `top_x` path), and shipped a 44-line candidate (`nearbyint_f64` finite fast path via `round_ties_even`, MSRV 1.77): value-exact 295/295 corpus cases, 16/16 proof cells byte-identical to the baseline arm AND to the t58 preserved standing references, and the standing paired gate **PASS in all four pre-declared classic-supports-on cells, both modes (6/6 pairs, zero starvation exclusions): wall −15.59 %/−13.70 %/−16.30 %/−18.30 %, CPU −12.25 %/−13.45 %/−13.71 %/−16.82 %** (ordinary/accelerated × benchy/base; order as on the ticket). Candidate-built standing binaries purged per the t36 precedent (rebuilt from the pre-take HEAD in a detached worktree, copied byte-exact back; candidate sha set absent; both freshness gates exit 0) and refreshed at the kept HEAD after the commit (force path; the refreshed standing module byte-matches the take-era measured candidate module `a04a77f7…`); FINDINGS has the full measured record. A v1 scanbeam-walk candidate was measured value-exact, a wash, and dropped. Closeout disclosure: xtask's compile-time-baked root lets a worktree invocation silently zero out the main tree's pre-commit gate scans (contained + upstream follow-up named on the ticket). Evidence: `evidence/t70-clipper-candidate/` (FINDINGS, vendor-provenance.json, candidate.patch, profiling/, results/, campaign/ with qualification.json + ab-reduction.json, verifiers in-repo).

- [Classic-perimeters per-layer host-service round trips](issues/68-classic-perimeters-host-round-trips.md): **resolved (2026-10-16; attribution take — no candidate; the bucket is 97.55 %/99.02 % native clipper ops)** — the T68-PROBE (two-sided: a dispatch-recorder plus per-call conversion/ops/conversion split in the host services, `now_us` site brackets in the guest; `probe.patch`, reverted) decomposes all 8 offsets + 3 clips per layer on benchy-off (5,726.4 ms guest-observed, 2,640 calls) and base-off (87,900.3 ms, 5,442 calls; both outputs byte-identical to the t58 references `34886ccb…`/`20e7204c…`): **ops 97.55 %/99.02 %, host conversions 0.23 %/0.11 %, guest marshalling + ABI 2.23 %/0.86 %**. Site shares (base): A0 21.4 %, A1 22.3 %, K 9.6 % — the walls/infill output themselves — and the gap machinery B–J 46.8 % feeding the width-band morph (98.6 % of raw candidate vertices discarded; gap fill emitted on 162/495 layers). Every t60-named direction's ceiling measured below any gate: migration ≤ remainder+conversions minus the wasm-ops penalty; batching ≤ per-crossing fixed costs on a saturated phase; site-level has no dead/duplicate site and no exact algebraic shortcut under byte-exactness. Wall relevance: 6.3 %/6.6 % of per-layer phase occupancy; the removable overhead ≤ 0.33 %/0.80 % serial and ≤ 0.03 %/0.07 % saturated of run wall. Thin-wall clip never fires (0 calls, dynamically confirmed); base layer 270 skips B/C/D. Two-run ±11 % accumulated drift with ~1-point-stable shares; t60 cross-check −1.5 %/−3.1 %. No production edit, no timing campaign, no auto-commit; probe reverted, freshness green. Evidence: `evidence/t68-host-roundtrips/` (`FINDINGS.md`, `probe.patch`, `run-t68.ps1`, `captures/`, `verify.py` exits 0 with seven negative controls rejected).

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

- [Repair the Orca rig](issues/98-orca-rig-repair.md): **rig measures Orca again on the 2.5.0 install, and the repair falsified the archived Orca side.** OrcaSlicer 2.5 resolves `--load-settings` through a vendor preset bundle (`PresetBundle::resolve_preset_config`: file-identity match, else a `"from": "system"` manifest walk that flattens `inherits`/`include`), and the install ships only `BBL.opc` with no preset JSON tree. The rig now stages a version-gated source-manifest tree from the tag whose `BBL.json` version equals the installed cache stamp (`02.08.00.16` == `2.8.0.16`, fail-closed) and applies the matched keys as CLI overrides, with `enable_arc_fitting` pinned to 0 because 2.5 flipped that default to enabled. Three measured runs of benchy classic-off: **1.978 s** wall / 6.50 s CPU / 4,000,709 bytes, against the archive's 2.787 s / 9.27 s / 3,890,392 — the archived Orca applied its hardcoded **filament** defaults (density 0, max volumetric speed 2, first-layer temp 200) while the process chain resolved in both eras (300 mm/s walls). So the archived Orca under-measured Orca's own work: PnP's recorded multiples are optimistic for PnP and every archived Orca median needs re-derivation (fog). No ratio requoted. Evidence: `evidence/orca-rig-repair/` (`verify.py --self-test` exit 0, 4 claims, 7/7 negative controls rejected).

## Not yet specified

- **The bridge wave expansion is a per-layer partition change, and its benchy
  cost is the blocker (surfaced 2026-11-06 by
  [t122](issues/122-prep-slice-wave-expansion-facet-retirement.md)).** Canonical's
  `expand_expolygons` / `wave_seeds` / `propagate_waves_ex` / `merge_bridges`
  grow classified bridges into `stInternalSolid` / `stInternal` / `stTop` and
  trim those zones. PnP's sparse zone and `perimeter.infill_areas` do not exist
  until the `Layer::Perimeters` commit partition, so the port belongs there,
  not in `PrePass::Slice`. The tight wall-inset zone-proxy screen prices the
  expansion at **1,151.9 ms** thread-time on benchy (**102.6 %** of the
  classification saving) and **4,976.6 ms** on base (5.2 %); the combined shape
  is negative in all four benchy cells, while the classification-only
  retirement clears all eight. **The classification half landed as
  [ticket 123](issues/123-facet-bridge-path-retirement.md) on 2026-11-07**
  (8/8 at 3.4x–27.8x); only the expansion/trim remains in this patch.
  Re-enter only with a per-zone implementation at
  the Perimeters commit, a measured benchy expansion cost below roughly **1.1 s**
  thread-time, and an explicit `detect_bridge_directions` vs PnP-direction
  decision. Do not re-open the old prepass framing.

- **The per-layer/emit program is the next stage after the prepass rewrite
  (t111, 2026-11-03).** The destination stays 8/8, but even deleting both largest
  prepass stages leaves base classic-off at **2.86x** and base classic-on at
  **4.00x** inside t104's instrumented arithmetic against Orca's quiet walls — an
  upper bound on the improvement — so no future prepass ticket may claim the
  destination closes alone. [t110](issues/110-orca-side-attribution.md) measured
  Orca's per-layer + emit phases **4.4x–7.3x** cheaper than PnP's (again with an
  instrumented PnP side), and t104's counterfactual says deleting the entire
  per-layer phase still leaves **3.02x / 4.51x / 4.66x / 7.60x** (benchy-off /
  base-off / benchy-on / base-on). The shapes that could move those phases are
  unmeasured; this patch graduates only after the prepass stage lands or the human
  opens it. The per-origin occupancy batch failed its paired gate in
  [Strong planner occupancy-offset batching: representation-safe four-cell campaign](issues/125-strong-planner-occupancy-offset-batching.md);
  the per-layer emit-check half was not tried and must not inherit either a win or
  a failure claim. Any per-layer batch candidate needs a separate scope and price,
  not automatic rescue status.
- **t110's base-on multiples need a ledger correction (surfaced 2026-11-03 by the
  t111 re-derivation).** The t110 answer's "end-to-end today" **9.49x** divides
  PnP's **instrumented** base-on slice span (**228,997 ms**) by Orca's quiet
  process wall (**24,140.3 ms**); t104's own disclosure says that instrumented
  pass ran **+43.6 %** over its t103 median and that instrumented walls are never
  wall claims. The uninstrumented paired record is **160.857 s** (t106) ->
  **6.02x**, or 6.66x against today's Orca. The derived **16.3x** slicing,
  **19.6x** prepass and **13.7x** support-increment multiples inherit the same
  inflation, so they are **upper bounds**, not ratios of record. The qualitative
  finding (the engine gap exceeds the wall ratio) survives; the numbers need
  re-deriving from t104's implied walls (share x the wall of record) before any
  of them is quoted as a scoreboard anchor. This is a bounded measurement-only
  task, not a route; it graduates when the human picks it.

- **A first-touch vendor tree makes the warmup load-bearing (t98 method note).** The
  first Orca run against a freshly staged vendor tree costs ~10 s more wall than the
  steady state with process CPU flat (10.084 s / 11.684 s vs 1.97–2.04 s measured
  after), and a warm tree with no cache beside the manifest still runs at 2.0 s. So the
  premium is one-time, but a `-Warmups 0` batch against a fresh tree would be
  contaminated by roughly the whole steady-state cell time. Any campaign on this rig
  must keep `-Warmups 1` or greater.

- **The surviving whole-print free is allocation-volume-bound, not a
  drop-strategy problem (surfaced 2026-10-29 by
  [ticket 94](issues/94-remaining-whole-print-drop.md); graduated as
  [ticket 100](issues/100-allocation-volume-layer-collection.md) on
  2026-10-30 — the human selected this route from six measured fog routes).** The `host:gcode_emit`
  free is **3,242-3,558 ms** for 495 layers on the base supports-on cells, at
  ~5.3 GB peak working set (t79). The free does **not** parallelise (12 threads:
  classic −0.32 s, arachne +0.20 s), layer-by-layer consumption overlaps
  nothing (the walk is 176-253 ms serial), and the deferred-overlap ceiling is
  the **1.37-1.60 s** post-emission tail — at the 0.80 % floor before overlap
  loss. The lever was thought to be **fewer/larger allocations: an arena or
  bulk-free layer collection** (one allocation per layer or per print).
  **[Ticket 100](issues/100-allocation-volume-layer-collection.md) priced that
  family on 2026-10-30 and it is dead as scoped:** the census is
  **34,796,481 blocks / 1.358 GB / 3.53 s (101.3 ns per block)**, but **99.6 %**
  of the blocks are **8–15-byte strings** — the `support_entity_identities` side
  table cloned per support path — while geometry is **0.094 %** of the blocks /
  **7.8 %** of the bytes. A bulk free of the same bytes costs **44.5–63.8 ms**
  (ceiling **3.468 s = 2.10 % = 2.63x** the floor), yet flattening a live
  collection pays a **73.3–115.1 ms** copy **plus the same storm** (**+73 ms
  worse**), so only arena-at-creation is reachable and it would aim at 0.094 %
  of the blocks. **The surviving shape was the table itself, and
  [ticket 101](issues/101-support-identity-table-materialisation.md) priced it on
  2026-10-31: below the floor (combined ceiling 821.6–926.8 ms = 0.50–0.56 % =
  0.62–0.70x), so the family is closed too.** Its shape is worth keeping: 23,751
  rows over 431 bodies (55.1 paths per body), each row carrying the body's whole
  ~1,459-id list, `mixed_bodies = 0`, 418 distinct contents, 1.2486 GB of heap —
  **one body's list cloned once per emitted support path**. The arithmetic that
  matters now: under mimalloc a block costs **~12.6 ns to free** (t101) and the
  whole table **~0.8 s**, so **re-enter the arena / block-count question only if
  the block count stops being dominated by a structure with no production
  reader**, and re-derive any per-block price before quoting it — t100's
  101.3 ns was the system heap.

- **The shipped allocator is now mimalloc, and the whole-process swap is worth
  ~5–6 % of a supports-on cell (measured 2026-10-30/31 by
  [ticket 100](issues/100-allocation-volume-layer-collection.md) and
  [ticket 102](issues/102-global-allocator-substitution.md), **resolved KEEP**).**
  The free alone fell **3.527 → 1.302 s measured** (**1.35 %** of the cell;
  **1.66 %** after the probe correction) on the same 34,796,481-block collection,
  and the process-wide swap passed the standing four-cell gate at wall
  **−6.19 / −5.64 / −5.54 / −4.80 %** with process CPU **−1.74 / −2.18 / −1.54 /
  −1.88 %**, **24/24 pairs byte-identical** and zero exclusions. The process-wide
  effect is the larger number because **every** allocation in the pipeline gets
  cheaper, not just the whole-print free. It removes **no allocations** — the
  census is unchanged — so the per-block cost now sets what a block-count
  reduction is worth. Two prices, don't conflate them: the **free alone** is
  **~12.6 ns per block** measured directly (t101: 432.4–494.1 ms over
  34,796,481 blocks), while t102's **~37 ns per block** is the *process-wide*
  effect implied by its wall delta, not a free-side price. The cause-level route
  [101](issues/101-support-identity-table-materialisation.md) is **resolved
  below the floor** on that cheaper price. Any future allocator experiment
  (snmalloc, jemalloc) must re-run the same four-cell gate;
  `docs/16_slicer_report.md` §"Global allocator contract" now names mimalloc as
  the shipped choice and keeps the accounting wrapper outside it.

- **A free-strategy census must sample block contents, not just sizes
  (method note, 2026-10-30, [ticket 100](issues/100-allocation-volume-layer-collection.md)).**
  The whole-print free presented as a generic "allocation-volume" problem for
  two takes (t94, t79) until the allocator probe copied the first bytes of the
  dominant size class out of each block: **99.6 %** of the blocks were 8–15-byte
  `String`s, not geometry, which inverted the target. A histogram alone cannot
  distinguish a million tiny geometry buffers from a million duplicated
  attribution strings, and the two want opposite fixes. Any future
  "N blocks are too many" screen should sample contents and attribute the
  dominant class to a producer before pricing a representation change.

- **The whole-layer clip-area dilation is priced, and its last
  representation-safe shape is closed (t96 window rejected 2026-10-30;
  [ticket 99](issues/99-clip-split-union-dilation.md) split rejected
  2026-10-30).** `offset(deep_infill_area, +1.5 × spacing)` is
  **11.4–12.8 s accumulated** across t92/t93/t96 (**1.57–1.85 %** of the base
  classic-off cell). Its cost decomposes **~80 % per-path Miter offset,
  ~19 % batched internal union/cleanup, ~2 %** an explicit union of the
  already-rounded pieces (t99, 130 calls, two pricing orders). **Both named
  shapes are closed:** a candidate-box window removes **91 %** of the call but
  changes the bridge direction (t96 — `internal_bridge_angles`'
  `expansion_area ∩ clip_area` term feeds a *global* nearest-anchor choice, so
  no finite candidate-driven window is safe), and the split-and-union family
  tops out at **0.41–0.43 %** of the cell — **0.51–0.54x the 0.80 % floor**, and
  not even set-equal (2/130). **No in-scope shape remains on this call.** The
  only standing shape is to change the angle consumer
  (`determine_bridging_angle`) so the clip area need not be whole — OrcaSlicer
  geometry-parity work, which the map holds out of scope unless the human makes
  a t81-style exception. Re-enter only if that consumer changes.

- **A host-side work removal can be wall-neutral in accelerated cells
  (surfaced 2026-10-29 by
  [ticket 93](issues/93-whole-layer-deep-infill-offsets.md)).** The windowed
  erosion removed **6.5 / 3.2 s** of process CPU in the two accelerated base
  classic cells while their paired wall moved **+0.13 / +0.62 pp** — inside
  those cells' own 6–8 % repeat ranges, and with CPU falling in every cell. A
  strict four-cell wall clause can therefore fail on a real work removal.
  Price the gate per mode, and settle before the campaign whether a
  CPU-corroborated wall-neutral cell is acceptable — not after it.

- **An inside-the-window set check is not a consumer-equality proof when a
  consumer is a global nearest-neighbour choice (t96, 2026-10-30).** Ticket 96's
  production and windowed clip areas were **set-equal inside the window** on the
  layer whose bridge direction changed (`mismatch_polys = 0`), because the
  consumer (`determine_bridging_angle`) samples candidate edges but scans every
  anchor ring globally. A window proof must bound each consumer's **reach**;
  comparing the windowed set to the production set bounds the operand, not the
  consumer.

- **A batched whole-mesh walk scales with per-triangle cost, not with plane
  count (surfaced 2026-10-30 by
  [ticket 95](issues/95-exact-z-cross-section-batching.md)).** Folding 431
  single-plane `slice_mesh_ex` walks into one sorted-Z walk bought
  **6.4–6.9x**, not 431x: the batched walk pays two binary searches over the
  plane array plus a straddle test per triangle, and it is memory-latency-bound
  on the 1.23 M-vertex array (**0.99–1.05 s** for 2.46 M triangles). Any future
  "batch N planes/queries into one walk" estimate must price the per-triangle
  overhead at N rather than assume N-fold. Method note, not route work.

- **Ownership changes the cost of what it removes — do not price the survivor
  from the old bracket (surfaced 2026-10-27 by
  [ticket 90](issues/90-postpass-layer-collection-ownership.md)).** Ticket 90's
  pre-declared net ceiling (10.76 / 10.29 s) assumed the one surviving
  whole-print drop would still cost what the baseline's cloned drop cost. The
  measured wall deltas were **12.0 / 14.8 / 15.9 / 15.4 s** and the surviving
  drop was **1.56 / 2.68 s cheaper**, because no fresh whole-print copy exists
  to free. Any future clone/drop-removal estimate that carries the old drop cost
  forward inherits the same bias; measure the survivor in the candidate arm, or
  state the ceiling as a range. Method note, not route work (same class as the
  `map_batch` occupancy note below).

- **`map_batch` occupancy must be measured per request set, not borrowed
  (surfaced 2026-10-25 by [ticket 85](issues/85-seam-inset-offset-batching.md)).**
  Ticket 84 priced the seam batching at t78's measured **2.6x** effective
  speedup for the same `map_batch` fan-out; the candidate actually achieved
  **≈7x** (8,356 → 1,737 ms), so the pricing under-predicted the wall win by
  roughly a third of the recoverable time. Any future batch-adoption candidate
  that copies that 2.6x as a planning constant inherits the same bias; price
  with the request set's own measured occupancy, or state the ceiling as a
  range. Method note, not route work — it graduates only with its own reason.

- **Fairness machinery reads a stale upstream snapshot (surfaced 2026-10-23 by
  [ticket 82](issues/82-fairness-key-parity-audit.md)).** The generated
  *Deviations from OrcaSlicer* table in `docs/15_config_keys_reference.md` (27
  rows) compares PnP defaults against `docs/ORCA_CONFIG_REFERENCE.md`, dated
  **2026-05-19**. Ticket 82 measured **82 of the 618 keys the matched job
  resolves absent from that snapshot**, including both Arachne tolerance keys —
  which is why the deviation that makes PnP do 10x/5x more Arachne wall
  simplification never appeared in the table. Any future fairness audit
  inherits this blind spot: the matched job's own G-code appendix is the only
  authority for what the comparison engine resolves. Regenerating the snapshot
  (or making the generator read an appendix) is method work, not route work;
  it graduates only with its own reason.
- **`enable_arc_fitting` is unexpressible and inert on the matched job
  (settled 2026-10-23 by [ticket 82](issues/82-fairness-key-parity-audit.md)).**
  Orca resolves `enable_arc_fitting = 0` in all eight cells, so there is no work
  difference today. It matters only if arc fitting is ever ported, because it
  halves `resolution`'s tolerance
  ([ticket 81](issues/81-resolution-slicing-time-contour-simplification.md)'s
  disclosures). No action while it stays 0.

- **The supports-on branch-distance lead is dead, and the style mismatch is now
  a human ruling (resolved 2026-10-30 → 2026-11-02; see
  [ticket 97](issues/97-match-branch-distance-fairness.md) and
  [ticket 105](issues/105-supports-on-engine-style-fairness.md)).**
  `support_style = default` on a tree family resolves to `smsTreeOrganic`
  (`SupportParameters`), `TreeSupport::generate` forks it to
  `generate_tree_support_3D`, and the organic engine reads only
  `tree_support_branch_distance_organic` — resolved to **1 mm**, the same as
  PnP's normal-key default. The normal 5 mm key is inert on that path; t88's arm
  was a 5x PnP branch-spacing reduction, not a match. PnP substitutes **Strong**
  for every canonically-organic input (`DEV-156`; organic port queued in
  `docs/specs/support-generation-remediation-plan.md`). The human ruled **pin
  Orca to `tree_strong`**; the measured price, the protocol disclosure and the
  post-pin scoreboard now live in the fairness contract above, re-derived by
  [ticket 106](issues/106-post-pin-supports-on-scoreboard.md) to **6.02x / 6.11x
  / 4.77x / 4.53x**.

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
- **Classic `is_bridge` empty-area parity guard (t20 observation, 2026-10-22).**
  Arachne guards its per-vertex `context.is_bridge` loop with
  `!bridge_areas.is_empty()`; classic calls it for every ring vertex even when
  the region has no bridge areas. The call is not in t18's measured hot set
  (the two exact scan families are), so this graduates only with its own
  measured reason; it was deliberately excluded from the t20 candidate.
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
  reduction, `split_top_surfaces` secondary work, the
  `emit_walls` memory-shape idea, and the ticket-18 query-internals candidates
  — 2-D per-polygon bbox records, pre-converted winding vertex cache +
  short-circuits, descending quartile first-hit) stays parked — the park
  decision is in
  [Gap budget per cell](issues/12-gap-budget-per-cell.md), and ticket 18's
  measured fuel attribution keeps its candidates in the same below-fold class —
  until a cell lands
  within ~1.5x of Orca or the structural tickets come back short.
  **Consume-only-when-read was lifted by the human and resolved as KEEP
  `053904aa` (2026-10-22; see Decisions), so it is no longer part of the
  below-fold list.**
  **Re-derived 2026-10-18 on the
  [scoreboard refresh](issues/72-scoreboard-refresh.md): the best cell is
  3.30x (benchy arachne-off) — the park is still binding.**
  **Re-derived 2026-10-25 on the
  [post-resolution-port refresh](issues/87-scoreboard-refresh-post-resolution-port.md):
  the best cell is 2.71x (benchy classic-off accelerated) and no cell is within
  ~1.5x — still binding, with the release bound moved 3.30x → 2.71x.**
  **Base refresh (2026-10-18):** the fresh base split graduated three
  tickets — [supports-on finalization view
  cost](issues/74-layerfinalization-supports-on-view-cost.md) (**resolved
  2026-10-19, KEEP `091e6bc9`**), [ShellClassification phase-B
  commit](issues/75-shellclassification-phase-b-commit.md) (**resolved
  2026-10-19, KEEP `07b97cad`**) and [MeshAnalysis exact union](issues/76-meshanalysis-footprint-union.md)
  (**resolved 2026-10-21, KEEP `465f2fd3`**) - and the remaining un-graduated base leads stay in the fog:
  the tree-support planner's 97-120 s serial native union/carve now has its
  measured batch/parallel contract ([ticket 77](issues/77-tree-support-planner-batch-parallel-contract.md),
  2026-10-22): **99.03 % of its 79.659 s clip core is inside `build_roles`**
  (472 big role unions + 69,161 tiny per-region carves, both independent per
  item), so the shape is **cross-call batching** through the existing
  `clip_polygons_batch` — exact per call — and the graduated candidate
  [emit-pass batched carve and role union](issues/78-tree-support-planner-emit-batching.md)
  is **resolved 2026-10-22, human-approved KEEP `73a6f1e8`**: proof 16/16 and
  the standing gate PASS in all four classic supports-on cells (wall
  −13.41/−12.30/−7.22/−16.66 %, CPU +0.51/+1.51/+0.74/+2.80 %), taking the
  79.7 s clip core to ~30.7 s (2.6x effective). The
  `EMIT_BATCH_LAYERS`/`CLIP_BATCH_MAX_REQUESTS` lever was
  [swept and closed](issues/79-tree-support-planner-emit-batch-constant-sweep.md)
  on 2026-10-22: the committed `4 / 256` is the measured choice (chunk knee at
  `4`; the `4096` cap's instrumented bracket edge did not transfer — **+6.30 %
  wall** in the frozen-snapshot confirmation), so no constant changed.
  The prefix chain, t76's intra-call partition and the ladder unions are
  measured dead (0.13 %, no target above 436 ms, 0.7 %). Still fog: the
  `PrePass::OverhangAnnotation` (9.3-10.9 s wall) and `SeamPlanning`
  (8.6-10.9 s serial) are **resolved by
  [ticket 84](issues/84-base-common-floor-attribution.md)** (2026-10-24). Seam
  is 92-93 % one serial native inset offset per region and graduates as
  [85](issues/85-seam-inset-offset-batching.md); overhang is engine-bound
  (99.7 % kernel, 85.4 % canonical three-offset work, 11.86x parallel, operand
  window retains 97.84 %), so no shape ticket exists and only an engine-level
  speedup moves it. The rest of the base common floor is ticket 59's closed
  Slice verdict. After t75 the residual phase-B serial work is the
  depth gather, the angles step and the 9/121 affected-layer recompute; the
  stage's Pass 1 `apply_opening` round-join arc-tolerance-0 anomaly (t27
  candidate (a)) remains the largest measured, un-ticketed term in the stage
  and stays in the fog pending its own representation/parity check — **bounded
  by t104: `PrePass::ShellClassification` is 14.0 s of the base classic-off cell
  and 22.0 s of base classic-on, so no shape inside it can supply the park's
  77.8–93.9 % prepass cut.**
- **Named leads from the t60 host:native attribution (2026-10-14).**
  [host:native attribution](issues/60-host-native-attribution.md)
  names two reducible shapes beyond the now-resolved (KEEP 2026-10-16)
  [Region-preparation derivation sharing across region projections](issues/67-region-preparation-derivation-sharing.md):
  the **classic-perimeters per-layer host-service round trips** (8
  `offset_polygons` + 3 `clip_polygons` calls per layer; 89.26 s accumulated
  base-off and 85.83 s base-on vs 5.91–6.31 s benchy — the largest
  supports-off owner) **resolved 2026-10-16 as an attribution take with no
  candidate** ([ticket 68](issues/68-classic-perimeters-host-round-trips.md);
  the bucket is 97–99 % native clipper ops and every named reduction
  direction's ceiling is sub-noise — see the ticket and
  [`evidence/t68-host-roundtrips/`](evidence/t68-host-roundtrips/FINDINGS.md));
  the **`region_partition` 3×-per-layer re-run** (10.04 s accumulated base-off;
  dedup needs semantic analysis of what the post-process commit can change)
  remains un-graduated, awaiting the human's route decision — **bounded by
  t104: the whole per-layer phase is 6.54 s implied wall of the 75.38 s
  base-off cell (12.70 s of base-on), so no per-layer shape can reach the
  park.** The standing
  paired A/B governs any candidate.
- **Gap-candidate pipeline cost (t68 observation, 2026-10-16).** 46.8 %
  (base-off) / 49.5 % (benchy-off) of the classic-perimeters round-trip
  bucket produces raw gap candidates of which the width-band morph discards
  ~98.6 % of the raw vertex volume on every layer (D 4,632 + H 2,776
  verts/call → 105 verts/call into the filter; gap fill emitted on 162/495
  base layers). Re-enter only if the parity program ever re-derives the
  gap-fill algebra in a representation-identical form.
- **Orca-side attribution is now measured, and the remaining depth is
  elevation-gated (resolved 2026-11-03 by
  [ticket 110](issues/110-orca-side-attribution.md)).** The map carried "little
  introspection" as an assumption; t110 measured it: the CLI log buffers (27 stage
  markers in **3 bursts**, no timestamps at any `--debug` level), ETW kernel sampling
  is refused without elevation, and `--pipe` never connected in two attempts. The
  observable split has Orca's base wall **42–73 %** model import + startup and its
  whole slicing phase at **0.64 / 1.28 / 3.68 / 14.01 s** against PnP's
  **7.37 / 11.78 / 80.22 / 228.32 s** (**11.6x / 9.2x / 21.8x / 16.3x**), with the
  prepass carrying both engines and the pinned supports-on prepass increment at
  **8.34 s** against PnP's **~113.9 s** (**13.7x**). The only deeper route left needs something
  this rig does not have: an elevated ETW session, a working `--pipe` connection
  (re-test before pricing it), or an instrumented Orca build. Re-enter only with one
  of those in hand — otherwise the phase split is the floor of what this rig can see,
  and no substage estimate should be quoted.
- **Occupancy gate still measures the identity aggregate.**
  [Support-correctness repair](issues/14-support-correctness-repair.md)
  un-widened only the extent bound; `validate_entry`'s exact-Z occupancy check
  still drops a whole identity entry when any one body cross-section overlaps
  model occupancy — the same class of aggregate mis-measurement, latent (zero
  occupancy rejections in the measured failure data). Becomes a ticket the day
  it fires on a matched cell.

- **Dense-infill bridge shortcut and per-extrusion object mass (screened
  2026-10-27, OrcaSlicer
  [#16741](https://github.com/OrcaSlicer/OrcaSlicer/pull/16741)).** The PR's
  dense-below path (`prints_dense` → `diff_expanded`) fires only when every fill
  surface of the lower layer prints at 100 % density. The matched job resolves
  `sparse_infill_density = 20 %`, so the path cannot fire on any map cell. PnP's
  `gate_internal_bridge_sites` already pools dense `infill_areas` into
  `lower_layer_solids` under its own `density >= 0.999` branch, but
  `unsupported_span_areas` still pays the full four-op chain. Re-enter only with
  a 100 %-infill model-class arm, which the map does not measure. The PR's other
  half, the per-extrusion object-mass locator, has no PnP counterpart: the
  emitter walks `ordered_entities` with no per-object lookup, and
  `PostPass::GCodeEmit` is 0.17 s off / 7.13 s on, of which t88 attributes
  ~6.15 s to the collection drop.
- **Per-part brim contact grouping (screened 2026-10-27, OrcaSlicer
  [#16733](https://github.com/OrcaSlicer/OrcaSlicer/pull/16733)).** PnP has no
  per-part identity: `resolve_object` (`crates/slicer-model-io/src/loader.rs`)
  merges normal 3MF parts into one solid mesh, and
  `SkirtBrim::generate_brim_entities`
  (`modules/core-modules/skirt-brim/src/lib.rs`) builds one print-wide
  rectangular loop set from `compute_bbox`. There is no contact test to
  accelerate. Re-enter only if brims ever become per-part.

- **Dev-loop compile/test levers not yet priced (2026-11-02, from t108).** The
  inventory in [ticket 108](issues/108-dev-loop-latency-preflight-concurrency.md)
  leaves five levers unpriced rather than rejected: the **cold-workspace effect
  of `debug = 1` / `debug = 0`** (needs two clean target dirs), **linker
  substitution on a cold full build** (the incremental relink is only 12.8 s,
  but cold full builds were not measured), **`cargo-nextest`** on the five test
  buckets, **`sccache`**, and the **CI half** (cache keys, job parallelism, path
  filters, `cancel-in-progress`). **Blaze (formerly ra-jit) was tried rather than
  assumed.** Its two open Windows PRs were built and run on this host
  (#1527 ships no archive; #1591 builds in **57m34s** and starts a daemon), and
  the spike stops for reasons stronger than platform eligibility: two port bugs
  (`stand_ins` makes `hard_link` stand-ins and reads them with `read_link`, so
  every daemon start after the first fails with `os error 4390`; the store's
  `Access::Alone` check refuses a default profile ACL) and a **refusal to compile
  any crate with a build script** — 241 std instances need the LLVM backend, and
  the author's LLVM build stops at the `f16`/`f128` codegen. Its LLVM rebuild was
  abandoned after **92 minutes**. On WSL2 the released Linux build does run and is
  **slower than cargo on this repository in all three measured paths** (cold build
  193 s vs 54 s, warm edit 12.4 s vs 0.95-1.13 s at the crate root and 15.8 s vs
  2.4-3.5 s for a function body, `blaze test` 312 s vs 49 s; the 23-33 ms win is
  real only on a toy crate). **No blaze speedup is claimed for this repository**;
  re-enter only when upstream changes the workspace re-check shape, lands
  f16/f128 codegen and proc-macro/build-script support on Windows, or the
  dependency graph shrinks enough for the measured inversion to move.
  The measured weight sits in **first codegen of a bucketed test target**
  (994 s once, 17.26 s incremental), so a lever that only moves gate overhead is
  not the route. Any adoption must beat t108's **5 s per invocation** floor on
  paired medians. Both harness-correctness findings recorded by this take are now
  fixed by [ticket 109](issues/109-xtask-host-algos-injection-and-narrow-run-example.md):
  the `--features slicer-core/host-algos` injection is conditional on a
  `slicer-core` selection, and the `AGENTS.md` narrow example runs its 9 tests
  instead of zero.

## Out of scope

- **Pruning the support-attribution table as a memory or hygiene change.**
  [Ticket 101](issues/101-support-identity-table-materialisation.md) measured the
  table at **1.2486 GB** of heap with **no production reader** and priced its
  removal at **822–927 ms = 0.62–0.70x the floor** — below the floor on the map's
  own currency, so it is not route work. Deleting the field would still reclaim a
  quarter of the process's peak working set and retire a dead `pub` IR surface,
  but **memory is not judged here** (DEV-026, first bullet below) and a contract
  deletion is not a speed lever. Returns only if the destination is redrawn to
  include memory, or if a production consumer appears and makes the table work
  rather than dead weight.
- **Memory instrumentation / peak-RSS bounds (DEV-026).** The destination's
  verdict is speed (wall, CPU corroborating); memory is not judged
  (2026-09-22 scoping decision). Returns only if the destination is redrawn.
- **OrcaSlicer geometry-parity work.** The repo's parity program runs as spec
  packets; this map treats output as disclosed evidence only (the fairness
  contract), not as a parity target.
  **Two recorded exceptions.** (1) **2026-10-23:** the human ruled that ticket
  81's `resolution` port be carried in-map end to end, because the fairness
  contract needed the key implemented rather than merely disclosed. That
  exception is scoped to that key. (2) **2026-11-03:** [ticket
  111](issues/111-prepass-rewrite-scope-ruling.md) authorizes a **scoped
  canonical-parity exception for the prepass** — `PrePass::Slice` first, the
  Strong planner second — under the fresh-16-cell-reference, all-8-cell paired
  gate; no production edit happens before the Phase-1 analysis
  ([ticket 112](issues/112-prep-slice-parity-phase1.md)). This exception is
  scoped to the prepass and its shapes, is not a standing licence for the rest of
  the parity program, and the next such question still goes back to the human.
  **Campaign-specific amendment (2026-11-07, [Strong planner carry: fold the occupancy batching into the Slice campaign, amend the gate, or park it?](issues/117-strong-planner-carry-ruling.md)):**
  the representation-safe Strong-planner occupancy-offset batch uses the four
  supports-on cells for its positive improvement gate, requires byte-identical
  output in all eight, and requires no median wall/CPU regression in the four
  supports-off cells. This is not a parity exception, does not generalize to
  other candidates, and leaves the 8/8 destination unchanged.
- **Making the resolved-config-less fallback block satisfy Orca's viewer gate.**
  Ticket 34's residual — the test/legacy path's `CONFIG_BLOCK` cannot reach 80
  *loadable* pairs because the remainder are PnP's own key names — is past this
  destination: every measured production path passes a registry projection (233
  pairs, 161 loadable), so no cell's disclosure depends on it. Whether those
  entry points should synthesize a viewer block at all is a product decision, not
  route work; it returns only if the destination is redrawn.
