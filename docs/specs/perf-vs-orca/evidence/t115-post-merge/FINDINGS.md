# Post-merge reconciliation (ticket 115): the merge is output-neutral on the matched job and expensive in the per-layer phase

Take: 2026-11-04, branch `feature/perf-vs-orca` at `9a7371aa`, i.e. after the
2026-11-03 merge `a5e9a403` (52 master commits: lightning migration, preparation
capability, shared selection, variant chains) and the thirteen branch commits on
top of it. `origin/master` has no commit newer than the merge.

**Headline:** the merge does **not** change the matched job's toolpath. All eight
cells' ordinary output grows by exactly **34 bytes**, the one added config line
`; prepared_plan_diagnostic = none`; on the four classic cells a direct line diff
confirms that is the *only* difference. But the merge **regresses the per-layer
phase**: 1.9x–6.0x in wall on every classic cell, with process CPU
**+17 %…+54 % in 12 of 12 pairs**. The prepass is unchanged (within 0.3 %). So
the map's ratios of record (t103/t106) and phase splits (t104/t110) are
**pre-merge** and must not be quoted against this tree.

## 1. Output audit — matched job is byte-identical apart from one config line

Pre-merge references are the t103 ordinary hashes (t103's rig rows also carry
the t104 captures used for the direct diff). Post-merge values are this take's
ordinary runs.

| cell | pre bytes | post bytes | Δ | pre hash | post hash |
| --- | ---: | ---: | ---: | --- | --- |
| benchy classic-off | 3,925,554 | 3,925,588 | **+34** | `3672dc7e…` | `39298f40…` |
| benchy classic-on | 8,252,412 | 8,252,446 | **+34** | `eb4817b4…` | `3b8fa86a…` |
| benchy arachne-off | 4,064,270 | 4,064,304 | **+34** | `51585398…` | `9ef7617b…` |
| benchy arachne-on | 8,390,168 | 8,390,202 | **+34** | `6b8a0074…` | `d4644c14…` |
| base classic-off | 21,023,739 | 21,023,773 | **+34** | `292df501…` | `910afa32…` |
| base classic-on | 42,507,912 | 42,507,946 | **+34** | `22200671…` | `4b6d2843…` |
| base arachne-off | 20,846,160 | 20,846,194 | **+34** | `01a1434c…` | `7f36b089…` |
| base arachne-on | 42,330,265 | 42,330,299 | **+34** | `0b29e1bf…` | `d8ef39a5…` |

Direct `diff` of the pre-merge captures against this take's post-merge captures
(the archived `target/t104-gcode/` files against the four classic cells) returns
**one hunk**: the added line `; prepared_plan_diagnostic = none` in the config
appendix. No move, no `;TYPE:`, no extrusion changes. The arachne cells were not
diffed line-by-line because no pre-merge arachne capture is committed; they are
covered by the exact `+34` byte delta and by the emitter's unconditional line.

**Consequence:** the t87/t91 post-t81 reference *set* is superseded by the
post-merge set, but the supersession is a disclosure line, not a geometry delta;
no reference re-record of toolpaths is needed. `4b6d2843…` (base classic-on) and
`3b8fa86a…` (benchy classic-on) are corroborated independently by t113's HEAD
captures.

## 2. Painted fixtures — attribution moves, total extrusion does not

The merge's classic paint-attribution fix (`build_wall_flags` now attributes
against the **original, pre-simplification** polygons for outer walls too) and
master's variant-chain/shared-selection work do change painted output. Both arms
are deterministic (a repeat run reproduces each hash exactly).

`resources/cube_4color.3mf` (four tools, painted faces):

| | pre | post | Δ |
| --- | ---: | ---: | ---: |
| bytes | 1,050,129 | 1,038,805 | **−1.08 %** |
| G1 moves | 19,137 | 18,894 | −1.27 % |
| total extrusion (mm) | 7,055.694 | 7,047.839 | **−0.11 %** |
| T0 / T1 / T2 / T3 extrusion | 793.2 / 2,115.0 / 2,568.0 / 1,579.5 | 728.2 / 2,047.7 / 2,612.4 / 1,659.5 | **−8.19 / −3.18 / +1.73 / +5.06 %** |
| tool moves | 2,247 / 4,561 / 6,063 / 6,266 | 2,339 / 4,492 / 6,020 / 6,043 | mixed |

`resources/painted_seams.3mf` (single tool): 4,682,386 → 4,682,219 bytes
(−167 B), G1 moves identical (111,958), but **9,608 diff lines** — start
vertices and path order move, and one `;TYPE:Outer wall` block moves; two
`;TYPE:Outer wall` blocks fewer per print.

**Reading:** the same filament total is redistributed between tools (up to 8.2 %
per tool) and the emission order changes. That is consistent with an
attribution/ordering correction, not a geometry rewrite — but the map holds **no
painted-job ratio of record**, so no performance claim depends on it. Any future
painted-job reference must be re-recorded post-merge.

## 3. Paired pre-merge vs merged A/B — the per-layer phase regresses

Method: the pre-merge arm is the preserved t103 developer snapshot
(`target/dist/developer/`, binary `4cb96942…` + its own 24-module tree), which
reproduces the archived pre-merge hashes exactly; the merged arm is the current
`target/release/pnp_cli.exe` + `modules/core-modules`. Three interleaved pairs
per cell, order alternating, `RAYON_NUM_THREADS=12`, uninstrumented.
**Load disclosure:** CPU load was **71 %** at batch start (browser + chat
processes); wall deltas are therefore load-confounded and quoted as a bound, not
a precise effect. Process CPU is far less sensitive and moved the same way in
every pair.

| cell | wall pre → post (s) | wall Δ | CPU pre → post (s) | CPU Δ |
| --- | --- | ---: | --- | ---: |
| benchy classic-off | 14.96 → 17.75 | **+19.26 %** | 53.5 → 66.4 | **+24.11 %** |
| benchy classic-on | 21.47 → 21.99 | **+4.73 %** | 70.0 → 81.8 | **+16.96 %** |
| base classic-off | 169.34 → 240.95 | **+42.29 %** | 799.9 → 1,228.4 | **+53.57 %** |
| base classic-on | 272.85 → 296.48 | **+13.85 %** | 1,132.6 → 1,562.6 | **+34.88 %** |

CPU is positive in **12/12 pairs**. Two wall pairs are negative
(benchy-on p2 −5.10 %, base-on p3 −29.71 %); both are single-run load spikes on
the pre arm and are inside that cell's ±30 % round-to-round spread.

**Where it lands:** the per-layer phase, measured two independent ways.

| cell | per-layer pre → post (ms, uninstrumented medians) | ratio |
| --- | --- | ---: |
| benchy classic-off | 2,608 → 5,816 | 2.23x |
| benchy classic-on | 3,119 → 5,949 | 1.91x |
| base classic-off | 15,341 → 92,418 | 6.02x |
| base classic-on | 20,876 → 106,103 | 5.08x |

The instrumented pair on base classic-off (back-to-back, same conditions) gives
**prepass 70,662 → 70,463 ms (−0.3 %)** and **per_layer 7,265 → 42,886 ms
(5.90x)**, confirming the phase split. Absolute per-layer values differ between
the instrumented and uninstrumented runs (instrumentation overhead and load) but
the ratio agrees.

**Module brackets (base classic-off, instrumented, accumulated over 495 calls):**

| module | pre ms | post ms | calls |
| --- | ---: | ---: | ---: |
| `com.core.fuzzy-skin` | 162 | 56,068 | 495 → 495 |
| `com.core.seam-placer` | 211 | 56,226 | 495 → 495 |
| `com.core.path-optimization-default` | 68 | 28,398 | 495 → 495 |
| `com.core.gyroid-infill` | 554 | 28,678 | 495 → 495 |
| `com.core.lightning-infill` | 412 | 28,510 | 495 → 495 |
| `com.core.rectilinear-infill` | 434 | 28,887 | 495 → 495 |
| `com.core.top-surface-ironing` | 399 | 28,843 | 495 → 495 |
| `com.core.support-surface-ironing` | 398 | 28,541 | 495 → 495 |
| `com.core.tree-support` | 477 | 28,729 | 495 → 495 |
| `com.core.wave-overhangs` | 400 | 28,494 | 495 → 495 |
| `com.core.infill-linker` | 4,314 | 32,266 | 495 → 495 |
| `com.core.classic-perimeters` | 42,082 | 69,194 | 495 → 495 |
| `com.core.raft-default` | 28,869 | 56,918 | 495 → 495 |

The **call count is unchanged**; the cost per call is not. Modules that do
almost no work on this job jump to a nearly uniform ~28.5 s accumulated
(~58 ms per call), and `fuzzy-skin`/`seam-placer` to ~56 s. Within a run the
per-call cost grows with layer index (`fuzzy-skin` 55 ms at layer 0 → 141 ms at
the last layers), which is the signature of a per-call cost proportional to
committed state rather than of guest work. Prepass stages are unchanged
(`host:slice` 37,897 → 38,505 ms, `host:shell_classification` 16,087 → 14,851 ms,
`host:overhang_annotation` 10,324 → 9,553 ms, `host:mesh_analysis`
4,083 → 5,123 ms).

**The accelerated arm does not show it.** Post-merge accelerated benchy
classic-off instrumented per-layer is **2,318 ms** against the ordinary arm's
4,807–5,860 ms, with the **same output hash** `39298f40…`. The accelerated dist
is built through the controlled accelerated compiler policy, so the divergence
is a build-mode-sensitive host path, not a difference in the job. Limitation:
the t103 accelerated snapshot was overwritten by the post-merge rebuild this
take required, so the accelerated arm's **pre-merge** per-layer value was not
captured; the statement is "accelerated post-merge ≈ ordinary pre-merge", not a
measured accelerated A/B. A mixed-arm probe (ordinary host + accelerated module
dir) produced a different job (**171 layers vs 240**), so mixed arms are invalid
and are not used.

## 4. Keeps — structural survival confirmed

Source check only, at `9a7371aa`: t78's `EMIT_BATCH_LAYERS = 4` and
`CLIP_BATCH_MAX_REQUESTS = 256` (`modules/core-modules/tree-support-planner`),
t95's `ExactZQueryService::prefetch_cross_sections`
(`crates/slicer-wasm-host/src/exact_z_query.rs`) and its call in
`support_aggregation.rs`, t90's `execute_layer_finalization_owned`
(`crates/slicer-runtime/src/layer_finalization.rs`), t102's
`AccountingAllocator<mimalloc::MiMalloc>` (`crates/pnp-cli/src/main.rs`), t101's
support-identity table (`assemble_ordered_entities_with_support_identities`,
`crates/slicer-runtime/src/layer_executor.rs`), and t81's `resolution` sites in
`classic-perimeters`. No keep was removed by the merge. Their **prices** are not
re-measured here: the per-layer regression dominates the phase they sit in and
would confound any bracket comparison until it is fixed.

## 5. Ruling

1. **No ratio of record may be quoted against the merged tree.** t103/t106
   ratios and t104/t110 phase splits are pre-merge (the t110 Orca side is fresh,
   its PnP side is t104's table).
2. **The post-merge Orca scoreboard is deferred, not skipped.** Running it now
   would bake a known per-layer regression into the ledger; the scoreboard is
   the acceptance gate of the regression's fix (graduated as
   [ticket 121](../../issues/121-post-merge-per-layer-dispatch-regression.md)).
3. **The output reference set moves to post-merge.** Matched-job toolpaths are
   unchanged; the hash set changes only by `; prepared_plan_diagnostic = none`.
   Painted fixtures need their own re-record before any painted-job claim.
4. **The `resolution`/config disclosure is intact** — the matched cells resolve
   the same generator/support keys (validator) and the same geometry.
5. **The merge's preparation-lifecycle cost on the measured cells is the
   regression above**, not a separate term.

## Reproduce

See [`REPRODUCE.md`](REPRODUCE.md). `python verify.py --self-test` exits 0 from
a plain checkout (403 checks, 6/6 negative controls rejected).
