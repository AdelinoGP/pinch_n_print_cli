# t193 findings — final single-HEAD `n >= 4` Orca closing scoreboard

Ticket: [Final single-HEAD `n >= 4` Orca closing scoreboard](../../issues/193-final-closing-orca-scoreboard.md).
No production code change. One interleaved eight-cell × three-tool window
(`orca`, `pnp-ordinary`, `pnp-accelerated`), 1 warmup + 4 measured runs per
cell/tool, against OrcaSlicer **2.5.1** (`5013b1bd…`), at HEAD `97b2474d`
(last code commit `e7b3c68e`, `t192`).

## Verdict

- **Wall clock: 8 of 8 cells beat OrcaSlicer on the binding arm**
  (`pnp-ordinary`), ratios **0.6803x–0.9646x** (required cut 3.67 %–47.00 %).
  The accelerated arm also wins **8/8** (**0.6840x–0.9705x**).
- **Mechanical tally: 7 of 8.** Seven cells satisfy `< 1.00x` + `>= 4` retained
  unstarved runs + strict separation + identity + CPU integrity guard.
  **`benchy-classic-on` fails strict separation by 2.9 ms**: binding median
  **2.4473 s** vs Orca **2.5370 s** (**0.9646x**, −3.67 %), but the binding
  arm's slowest run (**2.4913 s**) is 2.9 ms slower than Orca's fastest run
  (**2.4884 s**). It remains a **close-tier tie**, not a proven win.
- **Mechanical closure is 7/8; the human ruled that sufficient and closed the
  map at 8/8 (2026-11-25).** Per the ticket's decision branch, the remaining
  cell is attributed below and the targeted follow-on
  [Close the `benchy-classic-on` tie: land `PP-4` `GCodePostProcess`
  stream-view and `PP-1b` `part-cooling` `entity-roles`](../../issues/194-benchy-classic-on-postpass-wit-close.md)
  was graduated, then **deferred** to the `<= 0.50x` effort by the closure
  ruling.
- **Correctness/output identity: 16/16** cell × arm PnP groups (**80 PnP runs**,
  including warmups) byte-identical to `t182/reference.json`; every run
  `exit_code = 0`, `degraded = false`, `fatal = 0`, `non_fatal = 0`,
  `code-1200 = 0`. All **10 matched keys** (9 CSV keys + `support_style`:
  `tree_strong` on supports-on, `default` on supports-off) asserted on
  **40/40** Orca runs. **Zero starvation flags** in 96 measured samples.
- **CPU efficiency disclosure (not a gate, per `t176` Option C):**
  PnP/Orca process-CPU ratio is **0.9447x–2.0980x** ordinary
  (**0.9365x–2.0527x** accelerated). PnP spends less CPU on
  `base-classic-on` (0.977x), `base-arachne-on` (0.945x) and
  `benchy-arachne-on` (0.995x), and more on the two `base` supports-off cells
  (2.098x / 1.758x) and both `benchy` supports-off cells (1.260x / 1.105x).
  The CPU integrity guard (tight CPU, unstarved `cpu/wall`, matched
  non-degraded output) passes **8/8**.

## 1. Board (medians over 4 measured runs; retained = unstarved)

| Cell | Orca wall | PnP ordinary | Ratio | Cut | CPU eff. | Strict sep. | Tier | Close |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| `benchy-classic-off` | 1.842 s | 1.601 s | **0.8691x** | 15.06 % | 1.260x | +0.163 s | close | **PASS** |
| `benchy-classic-on` | 2.537 s | 2.447 s | **0.9646x** | 3.67 % | 1.026x | **−0.003 s** | close | **FAIL** |
| `benchy-arachne-off` | 2.058 s | 1.783 s | **0.8662x** | 15.45 % | 1.105x | +0.170 s | close | **PASS** |
| `benchy-arachne-on` | 2.568 s | 2.428 s | **0.9454x** | 5.77 % | 0.995x | +0.091 s | close | **PASS** |
| `base-classic-off` | 12.738 s | 9.440 s | **0.7411x** | 34.94 % | 2.098x | +2.518 s | wide | **PASS** |
| `base-classic-on` | 22.522 s | 15.500 s | **0.6882x** | 45.31 % | 0.977x | +6.352 s | wide | **PASS** |
| `base-arachne-off` | 13.596 s | 10.312 s | **0.7584x** | 31.85 % | 1.758x | +3.017 s | wide | **PASS** |
| `base-arachne-on` | 23.213 s | 15.792 s | **0.6803x** | 47.00 % | 0.945x | +7.165 s | wide | **PASS** |

Accelerated arm (disclosed, not binding): `benchy` **0.8898x / 0.9182x /
0.8441x / 0.9705x**, `base` **0.7352x / 0.7022x / 0.7806x / 0.6840x**.
The accelerated arm **would close `benchy-classic-on`** (0.9182x, +0.117 s
separation), but `pnp-ordinary` binds cell closure per `t176`.

Measured ratios came in **4.66 %–23.00 % worse** than `t192`'s cross-window
route composition (`ESTIMATE ONLY`), i.e. the estimates were optimistic, but
the direction held: all eight are now `< 1.00x` on the binding arm.

## 2. Attribution of the remaining cell (`benchy-classic-on`)

One fresh `--instrument-stderr` pass at HEAD (`--profile` run also taken;
G-code hash `a8c93535…` = `t182` reference, so the probe is output-neutral):

| Phase | Wall | Share |
| --- | ---: | ---: |
| validation | 3 ms | 0.12 % |
| **prepass** | **1,629 ms** | **63.96 %** |
| **per-layer** | **563 ms** | **22.10 %** |
| **postpass** | **272 ms** | **10.68 %** |
| outside brackets | 80 ms | 3.14 % |
| total (`slice_complete`) | 2,547 ms | 100 % |

Postpass decomposition: **`GCodePostProcess` 127 ms** (46.7 % of postpass,
5.0 % of wall), `LayerFinalization` 82 ms, `GCodeEmit` 45 ms,
`GCodeSerialize` 14 ms. `GCodePostProcess` is still the single largest
postpass item after `t192`'s Zero-WIT bundle, and it is exactly the
`PP-4` target: **102K–977K WIT push calls to add `+2` start/end commands**.
`com.core.part-cooling` still burns **143.2 M fuel** at HEAD (one call); that
is `PP-1b`'s `entity-roles` query.

**Priced closing arithmetic (`ESTIMATE ONLY`, `t189` prices at the
post-`t192` wall):** `PP-4` is **43–86 ms** on `benchy`, `PP-1b`
**8–35 ms**; combined **51–121 ms = 2.08–4.95 %** of the 2.4473 s binding
median. That is **17x–42x the 2.9 ms strict-separation deficit**, and would
move the cell to **0.917x–0.945x** with **48–118 ms** of separation. The
mechanism is exact (no geometry), additive-WIT only, and already priced; it
was parked for the `<= 0.50x` follow-on because `t192` projected 8/8
`< 1.00x`, and that projection is now falsified on strict separation.

## 3. Protocol, identities, and disclosures

| Artifact | Path / Version | Identity |
| --- | --- | --- |
| Git HEAD | `97b2474da202f8728c63e23df00dfa08fba6fd18` | tracker claim commit |
| Last code commit | `e7b3c68e93da717481d21290b0df4818b34d11d8` | `t192` |
| OrcaSlicer | `C:/Program Files/OrcaSlicer/orca-slicer.exe` | **2.5.1**, `5013b1bd…`, 292,160 bytes, vendor `02.08.00.16`, `tree_strong` |
| PnP ordinary | `target/release/pnp_cli.exe` | `cc8a2a94…`, 24 modules, tree `2371167c…` |
| PnP accelerated | `target/dist-accelerated/developer/pnp_cli.exe` | `eec2a2ad…`, 24 modules, tree `7197a518…` |
| Freshness | ordinary / accelerated | both exit `0` |
| Box load | 52 samples | min 0 %, median **21 %**, max 100 % |
| Free RAM | 52 samples | min 13.9 GB, median **15.2 GB**, max 16.1 GB |

Disclosures:

- **Aborted first launch (1 warmup row):** the first invocation was stopped
  after `benchy-classic-off-orca-w1` to add continuous load sampling. Its
  completed warmup row remains in the raw CSV; the reducer keeps at most one
  warmup per cell/tool (latest) and discloses
  `warmup_rows_dropped_aborted_launch: 1`. No measured row is affected.
- **Load max 100 %** is an instantaneous `LoadPercentage` sample; the median
  is 21 % and every retained wall is tightly grouped (see the per-arm series
  in `reduced/summary.json`). No cell carries a starvation flag.
- **Instrumented pass is attribution only.** The 2,547 ms instrumented wall is
  not a ratio of record; its G-code hash matches the currency, so the probe
  did not change behaviour.
- **No causal claim** is taken from the gap between the `t192` estimate and
  this board; the board is the measurement.
- **No production edits**, no reference move, no WIT change.

Evidence: [`reduced/summary.json`](reduced/summary.json) (all per-arm series,
starvation indices, margin tiers, closing flags),
[`reduced/attribution.json`](reduced/attribution.json) (phase/postpass/fuel
split from the output-neutral probe; regenerated by
`reduce_attribution_t193.py`), and
[`reduced/load-samples.csv`](reduced/load-samples.csv); `verify.py` PASS
**853/853**, `--self-test` **34/34** mutations rejected.
Reproduce: [`REPRODUCE.md`](REPRODUCE.md).

## 4. Decision

`7/8` closed mechanically. The remaining cell is a measured 2.9 ms tie caused
by postpass `GCodePostProcess` WIT traffic (plus the `part-cooling` query), not
by a prepass or geometry deficit. The human **accepted the cell's unstarved
`0.9646x` median win and closed the map at 8/8 on 2026-11-25**, with the
strict-separation waiver scoped to this cell on this board. The graduated
[Close the `benchy-classic-on` tie: land `PP-4` `GCodePostProcess`
stream-view and `PP-1b` `part-cooling` `entity-roles`](../../issues/194-benchy-classic-on-postpass-wit-close.md)
is **deferred** to the `<= 0.50x` effort.
