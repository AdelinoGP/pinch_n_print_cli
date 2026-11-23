# Supports-on increment attribution at post-resolution-port HEAD

Route: [Supports-on increment attribution](../../issues/88-supports-on-increment-attribution.md).
Attribution take only: no candidate implemented, no production edit, no
keep/drop campaign. The env-gated probe (`PNP_T88_PROBE=1`) is preserved as
[`probe.patch`](probe.patch) and was reverted before the take closed.

## Verdict

**The supports-on increment is ~107 s of slice wall on base classic and
~110 s on base arachne, and it is ~69 % the support geometry's own work plus
~31 % host-side shapes that can be removed without touching geometry.**
Two host shapes clear the map's 0.80 % floor and graduate:

| candidate | base classic | base arachne | mechanism |
| --- | ---: | ---: | --- |
| [Parallelize support-plan exact-Z validation](../../issues/89-support-plan-exact-z-validation-parallel.md) | **15.24 s (8.28 %)** | **13.94 s (7.42 %)** | the post-planner validation loop is serial though every entry is independent |
| [Single-ownership layer collection through finalization/emission](../../issues/90-postpass-layer-collection-ownership.md) | **10.76 s net (5.84 %)** upper 16.91 s | **10.29 s net (5.47 %)** upper 16.16 s | one clone + two whole-collection drops per supports-on slice |

Everything else the increment spends is the support geometry itself:
the tree-support planner's 55 s, the canonical contact-detection kernel's
10 s, and the per-layer tree render's ~5 s wall. Those are already parallel or
batched (t78's clip batching; 12-thread per-layer scheduling); no new
representation-safe reduction was found in this take.

The `tree_support_branch_distance` parity key is **not** the driver: the
config-only 5 mm arm is **−7.25 s (−3.94 %)** on the whole cell and **~−6.8 s**
of the support increment (**6.4 %**), while changing support geometry. It is
parity-scoped and returns to the human, not to a performance ticket.

## What ran

HEAD `5c7fa03e` (slicer sources = the t87 scoreboard tree `6e71c1a1`; the
intervening commit is docs only). Ordinary external mode, 12 rayon threads,
`--module-dir modules/core-modules --no-default-module-paths
--no-integrated-modules --instrument-stderr`. Each capture is one slice; the
probe build adds stopwatch terms only and is output-neutral.

**Primary set (quiet, 5 % machine load):**

| capture | wall (s) | slice (ms) | output SHA |
| --- | ---: | ---: | --- |
| base classic off | 78.9 | 77,290 | `292df501…` |
| base classic on | 185.9 | 184,115 | `22200671…` |
| base arachne off | 79.9 | 78,294 | `01a1434c…` |
| base arachne on | 189.9 | 187,972 | `0b29e1bf…` |

All four SHAs are the post-t81/t87 references; every run is `status=ok`,
`degraded=false`, `fatal=0`, `non_fatal=0`, with non-vacuous role counts.
The probe build's output is byte-identical to the HEAD wall runs (the same
four SHAs).

**Corroborating set (loaded, 68–73 % machine load):** four `t88-wall-*`
instrumented wall captures and a loaded classic probe pair. They are preserved
under `target/` and reduced into `captures/`, but
their absolute walls are **not** used for the split. They are disclosed because
the first pass ran under the user's desktop load, and their existence is why
the ticket reports a load-affected range for the same terms (e.g. the validation
loop measured 27.2 s classic in the loaded pass and 15.24 s in the quiet pass).

The quiet cell walls are the t87 scoreboard medians (PnP ordinary, uninstrumented,
171.07 s classic / 186.63 s arachne). The instrumented quiet runs are +7.6 % /
+1.7 % on those walls; the increment they measure is +106.8 s / +109.7 s against
the scoreboard's +97.9 s / +113.4 s. All shares below use the **same-run
instrumented slice wall** as the denominator, so they are scale-invariant to the
instrumentation; the absolute milliseconds are attribution, not a wall claim.

## The increment, by stage

Base classic (instrumented quiet, slice 77.29 → 184.12 s):

| stage | off (ms) | on (ms) | delta (ms) | share of the 106.8 s increment |
| --- | ---: | ---: | ---: | ---: |
| `PrePass::SupportGeometry` | 261 | 71,117 | **70,856** | 66.3 % |
| `PrePass::SupportAnalysis` | 1 | 12,850 | **12,849** | 12.0 % |
| per-layer support (phase increment) | 7,887 | 13,797 | **5,910** | 5.5 % |
| `PostPass::LayerFinalization` | 204 | 6,959 | **6,755** | 6.3 % |
| `PostPass::GCodeEmit` | 81 | 6,342 | **6,261** | 5.9 % |
| `PostPass::GCodeSerialize` | 458 | 719 | 261 | 0.2 % |
| `PostPass::GCodePostProcess` | 318 | 627 | 309 | 0.3 % |
| postpass teardown/reconcile gap | 0 | 4,579 | **4,579** | 4.3 % |
| common-stage drift (`Slice`/`ShellClassification`/`OverhangAnnotation`/`MeshAnalysis`) | — | — | −955 | −0.9 % |
| total slice increment | 77,290 | 184,115 | **106,825** | 100 % |

Base arachne (instrumented quiet, slice 78.29 → 187.97 s):

| stage | off (ms) | on (ms) | delta (ms) | share of the 109.7 s increment |
| --- | ---: | ---: | ---: | ---: |
| `PrePass::SupportGeometry` | 249 | 69,628 | **69,379** | 63.3 % |
| `PrePass::SupportAnalysis` | 1 | 13,022 | **13,021** | 11.9 % |
| per-layer support (phase increment) | 8,184 | 13,266 | **5,082** | 4.6 % |
| `PostPass::LayerFinalization` | 179 | 6,916 | **6,737** | 6.1 % |
| `PostPass::GCodeEmit` | 87 | 6,091 | **6,004** | 5.5 % |
| `PostPass::GCodeSerialize` | 344 | 787 | 443 | 0.4 % |
| `PostPass::GCodePostProcess` | 314 | 684 | 370 | 0.3 % |
| postpass teardown/reconcile gap | 0 | 4,212 | **4,212** | 3.8 % |
| common-stage drift (incl. `Slice` +3.7 s) | — | — | +4,430 | +4.0 % |
| total slice increment | 78,294 | 187,972 | **109,678** | 100 % |

The two generators produce almost the same support increment (106.8 vs
109.7 s): the wall generator changes the per-layer baseline, not the support
work. That is why the two candidates price nearly identically on both cells.

## The increment, by owner

`PrePass::SupportGeometry` is the planner plus a host-side post-module tail:

| term | classic on | arachne on | source |
| --- | ---: | ---: | --- |
| `com.core.tree-support-planner` | 55,524 ms | 55,308 ms | the planner guest call (support geometry; clip core batched by t78) |
| `validate_entry` over the plan entries | **15,237 ms** | **13,941 ms** | `aggregate_support_plan_irs_degrading_with_attributed_diagnostics` (`crates/slicer-wasm-host/src/support_aggregation.rs`) |
| plan clone + drop | 40 ms | 37 ms | `support_plans.push((*plan).clone())` + drop |
| plans/entries clones inside aggregation | 56 ms | 56 ms | `plans.clone()` + per-entry clone |
| sort + union + commit | 0.5 ms | 0.5 ms | `union_same_family_entries`, blackboard commit |

The probe's per-term split of `SupportAnalysis` (`host:support_analysis`,
`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`):

| term | classic on | arachne on | shape |
| --- | ---: | ---: | --- |
| eligibility pre-pass (`derive_needs_support`) | 2,236 ms | 2,280 ms | parallel per-region predicate |
| layer unions (object/layer `union_ex`) | 386 ms | 381 ms | serial map over 495 layers |
| contact detection (`detect_support_contacts_with_annotations`) | **10,160 ms** | **10,303 ms** | parallel canonical kernel |
| candidate assembly + family assignments + territory | 0.3 ms | 0.2 ms | serial, after the parallel pass |

`Layer::Support` is the per-layer tree renderer:
`com.core.tree-support` accumulates 51,956 ms classic / 50,578 ms arachne over
495 layers, which is **5.91 s / 5.08 s of phase wall** at the measured 12-thread
parallelism (the per-layer phase itself goes 7.89 → 13.80 s and 8.18 → 13.27 s).
The host-side gap around it is 0.36 s accumulated, i.e. negligible.

## The host shapes

### 1. Exact-Z validation is a serial 15 s loop (candidate)

`validate_entry` calls `ExactZQueryService::query`, which computes an
object-wide mesh cross-section at the entry's anchor Z (`cross_section_at_z`,
`crates/slicer-wasm-host/src/exact_z_query.rs`) and then tests the entry's
bodies against the occupancy. The loop is serial over entries; the queries are
independent and the service is read-only apart from its internal cache. On the
quiet cells the term is 15.24 s / 13.94 s (**8.28 % / 7.42 %** of the cell), and
the plan's clone/sort/union/commit terms together are under 0.1 s. A
12-thread order-preserving map is exact by construction; the cache also keys on
`region_id` although the computation is object-wide, so an object-level
`(object, z)` entry would reuse identical cross-sections across regions at one
Z (value-identical today).

### 2. One clone and two whole-collection drops per supports-on slice (candidate)

| seam | classic on | arachne on | what the probe shows |
| --- | ---: | ---: | --- |
| finalization | 6,959 ms | 6,916 ms | 1,843 / 1,979 ms clone + 779 / 837 ms modules + 4,337 / 4,100 ms old-vector drop |
| `host:gcode_emit` | 6,342 ms | 6,091 ms | walk 109 / 119 ms + estimate 66 / 83 ms + M73 16 / 19 ms; the remaining **6,151 / 5,870 ms is the drop of the cloned collection** at function exit |
| postpass teardown/reconcile gap | 4,579 ms | 4,212 ms | the `postpass` phase wall minus its four stage brackets; the final drop of the original collection plus `reconcile_finalization_travel` |

The three seams cost **16.91 s classic / 16.16 s arachne** in total. Taking one
ownership path through both stages leaves one collection to drop once, so the
**net upper estimate** is the finalization clone+drop plus the postpass gap,
**10.76 s / 10.29 s (5.84 % / 5.47 %)**; the emitter's remaining drop is the one
the current code already pays. The finalization comment already records that
the pre-stage snapshot is observable only on a mid-stage abort, where
production aborts the slice.

The probe's `SystemTime` marks corrected a tempting mis-read: the
`layer_irs.to_vec()` clone itself completes in ~5 ms (the `Instant` bracket
reported 4.09 s, disagreeing with the event timestamps by the width of an
earlier phase); the 6 s is the **drop**, not the clone. The first (finalization) clone pays the
cold-page cost; the emitter clone reuses the pages freed by the finalization
drop, which is why a 6 s drop can follow a 5 ms clone.

### 3. Measurement dead / geometry-bound

- **SupportAnalysis eligibility pre-pass** — 2.24 s / 2.28 s gross (1.21 % of
  the cell), but it is the predicate's own work; folding it into the contact
  pass cannot remove that work, and the surviving pass scaffolding is not
  resolved above the floor by this probe.
- **Support contact detection** — 10.16 s / 10.30 s (5.5 %), but it is the
  canonical parallel kernel (`detect_support_contacts_with_annotations`), not a
  host shape.
- **Tree-support planner** — 55.52 s / 55.31 s (30.2 % / 29.4 % of the cell),
  the support geometry itself; t78's batched clip core is in place, and the
  non-clip residual (offsets, MST, node moves) has no new representation-safe
  batch shape in this take.
- **Per-layer tree render** — 51.96 s / 50.58 s accumulated, 5.91 / 5.08 s wall;
  already parallel across 495 layers.

## `tree_support_branch_distance` (Orca 5 mm vs PnP 1 mm)

A config-only arm (`tree_support_branch_distance = 5.0`, everything else
unchanged) on base classic supports-on, same probe build:

| quantity | 1 mm | 5 mm | delta |
| --- | ---: | ---: | ---: |
| slice wall | 184,115 ms | 176,869 ms | **−7,246 ms (−3.94 %)** |
| planner | 55,524 ms | 54,077 ms | −1,447 ms |
| exact-Z validation | 15,237 ms | 13,673 ms | −1,564 ms |
| `Layer::Support` accumulated | 51,956 ms | 49,940 ms | −2,016 ms |
| G-code bytes | 42,507,912 | 42,154,300 | −353,612 (−0.83 %) |
| output SHA | `22200671…` | `4727ebb6…` | geometry changed (intended) |

The common-stage drift in the same pair is −426 ms, so the support-specific
effect is **~6.8 s of the 106.8 s increment (6.4 %)**. The key is a modest work
driver, not the reason PnP pays a 107 s increment; changing it is
geometry-changing parity work and needs a separate human ruling. This
measurement is the bound the ticket asked for, not a port.

## Candidate pricing

Floor: **0.80 % of the cell wall**, the map's established screening floor
(t83/t84), declared here before the arithmetic.

| shape | classic | arachne | verdict |
| --- | ---: | ---: | --- |
| parallel exact-Z validation | **8.28 %** (15.24 s) | **7.42 %** (13.94 s) | **graduate** → ticket 89 |
| single-ownership layer collection | **5.84 % net** (10.76 s; upper 9.18 %) | **5.47 % net** (10.29 s; upper 8.60 %) | **graduate** → ticket 90 |
| eligibility pre-pass fold | 1.21 % gross, predicate work remains | 1.21 % gross, predicate work remains | not graduated |
| contact detection | 5.52 % but canonical parallel kernel | 5.48 % but canonical parallel kernel | not graduated |
| planner | 30.16 % but support geometry | 29.42 % but support geometry | not graduated |
| per-layer tree render | ~3.2 % but support geometry | ~2.7 % but support geometry | not graduated |

The candidates are independent: 89 is a prepass host loop, 90 is postpass
ownership. Neither changes geometry; both must pass the standing paired
ordinary + accelerated gate before any keep/drop.

## Measurement traps and disclosures

1. **Machine load.** The first pass ran at 68–73 % total CPU (Discord, Steam,
   Opera, Deadlock on the user's desktop). Its absolute walls are inflated
   (classic-on 233.8 s vs the quiet 184.1 s; arachne-on 419.8 s vs 188.0 s).
   The quiet pass at 5 % load is the primary set; the loaded pass is preserved
   as corroboration and is why the same validation term appears as 27.2 s
   (loaded) and 15.24 s (quiet). Instrumented walls are never wall claims.
2. **Probe `Instant` bracket disagrees with `SystemTime`.** The probe's
   `ge.clone` `Instant` bracket reported 4.09 s while its `SystemTime` marks
   showed the clone finished 5 ms after the module start. Whether this is a
   hoisted clock read or a non-monotonic QPC on this host, the `Instant` delta
   is not usable. All probe claims in this take use the `SystemTime` (`at_ms`)
   marks and the stage brackets; the `Instant` deltas are not quoted. This is
   recorded for future probes: anchor to `SystemTime`, or black-box the clock.
3. **Probe build.** Four `crates/` files differ from HEAD only in the
   env-gated probe (`probe.patch`); the source was reverted and the guests
   re-checked before the take closed. The probe build's output is byte-identical
   to the HEAD wall captures on all four cells.
4. **Branch arm.** It is a config-only measurement, not a production config
   change; its output intentionally differs from the 1 mm reference. The
   per-run validation still passes (clean, non-degraded, non-vacuous roles).
5. **One run per cell.** The attribution is a single quiet instrumented capture
   per cell, not a paired campaign; the candidates' own A/B campaigns decide
   their wall wins. The quiet captures' stage walls are internally consistent
   (the phases sum to the slice wall within 0.1 %), and the output SHAs pin the
   tree.
6. **`D-168` and the fairness contract** are untouched. The t88 runs use the
   matched job at `resolution = 0.01`, the same key and order as t81/t87.

## Reproduce

```bash
python docs/specs/perf-vs-orca/evidence/t88-supports-increment/extract.py --raw target/t88-supports-increment
python docs/specs/perf-vs-orca/evidence/t88-supports-increment/reduce.py
python docs/specs/perf-vs-orca/evidence/t88-supports-increment/verify.py
python docs/specs/perf-vs-orca/evidence/t88-supports-increment/verify.py --self-test
```

`verify.py` exits 0 checkout-only against `captures/*.json`, `summary.json` and
the t87 row CSVs; `--self-test` rejects 8/8 negative controls (a moved quiet
wall, a moved postpass gap, a moved validation term, a moved ownership share, a
raised floor, a moved branch planner, a flipped output SHA and a moved
finalization gap). The raw capture tree is gitignored under
`target/t88-supports-increment/`.
