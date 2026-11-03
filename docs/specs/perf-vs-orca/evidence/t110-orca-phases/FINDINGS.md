# t110 — Orca-side attribution: what its matched-job wall is made of

Route: [ticket 110](../../../issues/110-orca-side-attribution.md).
Campaign: 2026-11-03, HEAD `1e72d9e3`, OrcaSlicer 2.5.0-beta on the repaired vendor
tree. Harness: `run_campaign.py` (tap), `reduce.py`, `controls.py`,
`probe_capabilities.py`, `verify.py`.

## Verdict

**Orca is not faster because it does less prepass work — it is faster because its
slicing phase is 9x–22x cheaper than ours, and most of its wall is not slicing at
all.** On the base cells **42 %–73 %** of Orca's process wall is model import +
startup; its whole slicing phase is **3.68 s / 14.01 s** against PnP's **80.2 s /
228.3 s** for the same cells. The prepass is the concentration on both sides
(PnP 89.5 % of slicing; Orca 57.1 % / 74.6 %), and on the pinned supports-on job
Orca's prepass grows by **8.34 s** where PnP's support stages add **~91.7 s** — an
**11x** engine gap on matched engine style.

**No per-stage timing channel works on this install**, measured three ways (below),
so the finest boundary is the phase split this take measures.

## What introspection exists (Phase 0, measured)

| channel | test | result |
| --- | --- | --- |
| `--logfile` CLI log | one benchy classic-off slice at `--debug 4`, log tapped every 2 ms | **Not usable for timing.** 27 stage markers arrive in **3 bursts**, the largest holding **18 of 27**; the whole prepass collapses into one arrival. No timestamps are emitted at `--debug` 3 / 4 / 5. |
| ETW kernel CPU sampling | `wpr -start cpu -filemode`; `xperf -on PROC_THREAD+LOADER+PROFILE -stackwalk PROFILE` | **Blocked.** `wpr`: "Failed to enable the policy to profile system performance". `xperf`: "NT Kernel Logger: Invalid flags. (0x3ec)". Needs elevation this session does not have. |
| `--pipe` progress | 90 s `NamedPipeServerStream` listening, two Orca runs (`--pipe <name>`, then `--pipe='\\.\pipe\<name>'`) | **No connection observed** (server exited `NO_CONNECTION` both times) though the CLI log records `Will use pipe <name>`. Unverified; not relied on. |
| `--info` control | `--info` with the same settings and model | **Works.** Parses and reports the mesh without slicing — the import proxy used below. |
| steady-state floor | `resources/calicat.stl` slice, warm vendor tree | **0.92–0.98 s** per run. (A first-touch tree costs ~10 s more, reproducing t98's method note.) |

## Table 1 — Orca phase split (medians of 3, tap at ~15 ms resolution)

| cell | wall | startup+import | prepass | emit (per-layer + G-code) | tail | Orca slicing |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 1,867.5 ms | **1,229.8 (65.9 %)** | 50.5 \* | 365.8 (19.6 %) | 225.5 (12.1 %) | 637.8 |
| benchy classic-on | 2,509.8 ms | **1,229.8 (49.0 %)** | 549.8 (21.9 %) | 497.5 (19.8 %) | 225.2 (9.0 %) | 1,280.1 |
| base classic-off | 13,814.5 ms | **10,130.0 (73.3 %)** | 2,103.0 (15.2 %) | 1,349.1 (9.8 %) | 237.9 (1.7 %) | 3,684.5 |
| base classic-on | 24,140.3 ms | **10,130.0 (42.0 %)** | 10,444.8 (43.3 %) | 3,230.6 (13.4 %) | 287.9 (1.2 %) | 14,010.3 |

\* benchy classic-off's prepass is **below 3x the tap's worst poll gap** (50.5 ms
against 138.9 ms), so it is a **bound** — Orca's prepass there is at most ~140 ms,
not a point estimate. Every other phase in the table is above the pre-declared 3x
rule.

Repeat spread: 1.8 % / 1.8 % / 6.5 % / 1.9 % of the cell wall. G-code bytes are
identical across repeats in every cell (4,000k / 9,481k / 20,300k / 44,804k), so the
job is deterministic and the tap is not perturbing output.

Process CPU by phase (same medians), which is what makes "cheaper" mean *less work*
rather than *better overlapped*:

| cell | CPU to prepass end | CPU in emit | CPU in tail |
| --- | ---: | ---: | ---: |
| benchy classic-off | 3.47 s | 2.84 s | 0.16 s |
| benchy classic-on | 7.80 s | 3.06 s | 0.17 s |
| base classic-off | 24.42 s | 4.28 s | 0.17 s |
| base classic-on | 92.47 s | 7.17 s | 0.25 s |

## Table 2 — PnP (ticket 104) against Orca, same four cells

PnP side is [ticket 104](../t104-phase-split/phase-split.json)'s instrumented phase
table; no PnP run was made in this take.

| cell | PnP slicing | Orca slicing | slicing multiple | PnP prepass | Orca prepass | prepass multiple | PnP per-layer+post | Orca emit | emit multiple |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 7,365 ms | 637.8 ms | **11.6x** | 5,545 | 50.5 \* | ≥ 40x \* | 1,816 | 365.8 | **5.0x** |
| benchy classic-on | 11,784 ms | 1,280.1 ms | **9.2x** | 9,595 | 549.8 | **17.5x** | 2,187 | 497.5 | **4.4x** |
| base classic-off | 80,216 ms | 3,684.5 ms | **21.8x** | 72,433 | 2,103.0 | **34.4x** | 7,781 | 1,349.1 | **5.8x** |
| base classic-on | 228,317 ms | 14,010.3 ms | **16.3x** | 204,832 | 10,444.8 | **19.6x** | 23,482 | 3,230.6 | **7.3x** |

Import is the one phase PnP wins, and it wins it by a lot where it matters: Orca's
startup + import is **10.13 s** on base against PnP's **≤ 0.708 s** of outside-phase
time (startup + mesh IO + teardown inside t104's instrumented span) — **≥ 14.3x**
slower at the same 123 MB file; on benchy it is **1,229.8 ms** vs **≤ 731 ms**.

## Findings

1. **Orca's wall is import-dominated on base, not slicing-dominated.** 73.3 % of
   base classic-off and 42.0 % of base classic-on is startup + model import. A slice
   with every per-layer feature turned down (1 wall loop, 1 top/bottom shell, 0 %
   infill, no support) still costs **13,639.6 ms** against the full cell's
   13,814.5 ms — **+0.1 s** — and its emit phase moves 1,241 ms against 1,349 ms. The
   feature stack is nearly free at this scale; the cost is fixed overheads
   (`controls.json`).
2. **The engine gap is 1.7x–3.7x larger than the ratio of record suggests.** The
   end-to-end wall ratio today is **5.86x / 9.49x** (base off/on) and
   **4.34x / 4.96x** (benchy off/on) — but the *slicing* ratio is
   **21.8x / 16.3x** and **11.6x / 9.2x**. PnP's fast import is worth up to 9.4 s of
   the base cell and has been masking the engine deficit in every scoreboard the map
   has quoted. The destination's currency is wall, so the ratios of record stand —
   but "we are ~5x behind" is an import-offset artefact; **the slicing engine is
   ~10–22x behind**.
3. **The prepass is the carrying phase on both engines, so the destination's
   prepass-rewrite ruling is not an artefact of PnP's phase mix.** Orca's prepass is
   57.1 % of its base-off slicing and 74.6 % of its base-on slicing (PnP: 89.5 % /
   89.4 %). The difference is absolute: 2.10 s / 10.44 s against 72.43 s / 204.83 s.
4. **On the pinned supports-on job the support planner gap is ~11x.** Orca's prepass
   grows **2,103.0 → 10,444.8 ms (+8,341.8 ms, +397 %)** when supports go on with
   `tree_strong`; PnP's support stages add **~91.7 s** (`SupportGeometry` 231 →
   91,902 ms, `SupportAnalysis` 0 → 22,254 ms). This is a matched-engine-style
   comparison (t105's pin), and it is the sharpest supports-on number the map has:
   PnP's planner is **10.5x** Orca's entire supports-on prepass increment and
   **8.4x** Orca's whole supports-on prepass.
5. **The emit phase is a 4.4x–7.3x gap, not a rounding error.** PnP's per-layer
   pipeline + postpass is 1,816 / 2,187 / 7,781 / 23,482 ms against Orca's
   365.8 / 497.5 / 1,349.1 / 3,230.6 ms. Deleting it entirely still leaves the park
   unreachable (t104's arithmetic), but it is the second-largest term and it tracks
   the prepass gap's shape: our per-layer work is also several times the cost of the
   same work in Orca.
6. **Ledger correction to t104.** t104's Table 3 reads the implied PnP phases against
   "Orca's whole slice (2.03 / 2.29 / 15.28 / 19.49 s)". Those are **process walls**,
   and on the base cells 66–73 % of them is model import, so the comparison
   understates the engine gap by 1.7x–3.7x. The slicing-only numbers for today's
   Orca are **0.64 / 1.28 / 3.68 / 14.01 s** (bench-off / bench-on / base-off /
   base-on). t104's phase shares and its counterfactual arithmetic are unaffected;
   only the "against Orca's whole slice" reading changes.
7. **No "Orca front-loads less work" story survives.** Orca's prepass does the same
   canonical job (slice, walls, solid surfaces, vertical shells, external surfaces,
   `tree_strong` supports) and does it in seconds; the trivial-feature arm proves its
   feature work is marginal, so the gap is not a work-volume difference at the
   feature level. It is per-phase implementation cost, concentrated in the prepass.

## Disclosures

- **The tap's resolution is ~15 ms, not 2 ms.** Windows' timer granularity makes
  `time.sleep(0.002)` behave as ~15.6 ms; the worst observed poll gap was
  33.1–46.3 ms. Phases below 3x the worst gap are reported as bounds (bench-off
  prepass, and the bench-off prepass multiple).
- **The tap's own price is below measurement.** Against a 5 s no-op child the tap
  process accumulated **0.0 s** of CPU (the `GetProcessTimes` resolution is 15.6 ms
  over 4.06 s wall), so the poller is under 0.4 % of a core.
- **`--info` is an upper bound on the slice path's import**, because it also prints a
  manifold/geometry report. So Orca's prepass and slicing numbers here are **lower
  bounds** and every PnP/Orca multiple is an **upper bound**. The trivial-feature arm
  bounds the direction from the other side: it lands 0.1 s from the full cell, so the
  import term cannot be small enough to move the conclusion by an order of magnitude.
- **Mixed vintage.** PnP's side is t104's 2026-11-02 instrumented table; Orca's side
  is today. Today's Orca walls are ~9 % below t103's medians (base-off 13.81 s vs
  15.28 s), so the multiples may be slightly overstated. Every ratio here is
  Orca-today over PnP-t104; neither arm is re-run.
- **Load was not quiet.** System CPU utilisation 0.25–0.43 (the campaign's own runs
  included); the tap's wall spread is 1.8–6.5 % per cell. No wall claim is made
  against another day's Orca.
- **The abandoned import-linearity control is recorded, not hidden.** `--info` on
  binary-STL prefixes of base.stl (25 % / 615k facets) does not return: a truncated
  STL is a non-manifold mesh and Orca's repair path never finished (killed at 300 s).
  It was replaced by the trivial-feature arm (`controls.json`, `abandoned_import_linearity`).
- **The `--pipe` channel was not established.** Two server attempts, both
  `NO_CONNECTION`; the CLI log line `Will use pipe <name>` is the only evidence that
  Orca accepted the flag. Anyone pricing a live progress feed must re-test it.
- No production edit, no candidate, no KEEP/DROP campaign; no guest or IR change.
  `verify.py` exits 0 checkout-only and `--self-test` rejects **7/7** negative
  controls; `verify.py` reads only `runs.csv`, `phase-split.json` and t104's
  committed table.
