# Orca-side attribution: what its matched-job wall is made of

Type: task
Label: wayfinder:task
Status: resolved (2026-11-03; measurement-only — **no per-stage timing channel
works on this install**, and at the finest observable boundary Orca's wall is
**import-dominated on base (42–73 %)** while its whole slicing phase is
**9.2x–21.8x cheaper than PnP's**; the prepass is 17.5x–34.4x cheaper and the
pinned supports-on support-stage increment is ~13.7x. See ## Answer)
Assignee: 2026-11-03 OpenCode wayfinder session — the human said "continue the perf
vs orca map" (no ticket named), the frontier re-derivation at HEAD `1e72d9e3` found
**109/109 children resolved — no open child, no claim**, and the human selected
**Orca-side attribution** from six fog routes.
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: the fog entry **"Orca-side attribution"** ("What OrcaSlicer spends
its time on per cell is unknown (GUI-subsystem binary, little introspection). Worth
scoping only if the gap narrows to specific stages where 'why is Orca faster here'
becomes the blocking question"), which [ticket 104](104-prepass-vs-per-layer-attribution.md)
made current by narrowing the gap to the prepass (**68.5–89.5 %** of every classic
cell) and naming `PrePass::Slice` (**41,472 ms = 51.2 %** of base classic-off) as the
map's largest single measured term.

## Question

Three parts, in order:

1. **What introspection does the installed OrcaSlicer 2.5.0-beta actually expose**
   for per-stage time? The map has carried "little introspection" as an assumption
   since it was charted; nobody has measured which channels work on this install.
2. **At the finest boundary that is observable without per-stage timings**, what
   does an Orca cell's wall split into — startup + vendor/preset resolution, model
   import, the prepass (`PrintObject::slice`'s phase, which is where Orca does its
   slice/wall/shell/support derivation), the per-layer path generation + G-code
   emission phase, and the process tail?
3. **Does that split explain the gap as a phase-share anomaly, a work-volume
   difference, or an absolute per-phase cost difference** — and if the last, on
   which phase, at what multiple, using PnP's own phase map (ticket 104) as the
   other arm?

The decision this serves is the destination's standing prepass-rewrite ruling: a
rewrite needs to know whether PnP's prepass is **slower at the same job** (an
implementation/representation route) or **doing work Orca never does** (a
parity/work-shape route), and whether Orca's advantage sits in its prepass at all.

## Method (pre-declared; feasibility probe disclosed separately)

**Phase 0 — capability census (feasibility probe, already run before this ticket
was written; results are evidence, not a campaign).** Measure, do not assume:

- the pinned `--logfile` CLI log at `--debug` 3 / 4 / 5: does it carry timestamps,
  and does it flush per line or per buffer? A log tap is only usable if the stage
  markers arrive spread across the run.
- `xperf` / `wpr` kernel CPU sampling: available, and does it run without
  elevation on this host?
- `--pipe`: does Orca connect to a listening `NamedPipeServerStream` and stream
  progress?
- the steady-state process floor: a tiny model (`resources/calicat.stl`) on a warm
  vendor tree, against t98's one-time first-touch premium.

**Phase 1 — harness.** Orca writes `<outdir>/plate_1.gcode.tmp` for the whole export
and renames it to `plate_1.gcode` at the end, so a ~2 ms directory tap observes two
hard boundaries without any Orca cooperation:

| boundary | observable | phase it closes |
| --- | --- | --- |
| `t_tmp` | `plate_1.gcode.tmp` first appears | startup + vendor + import + **prepass** |
| `t_rename` | `plate_1.gcode` appears | **per-layer path generation + G-code emission** |
| `t_exit` | process exit | process tail (teardown, result bookkeeping) |

Cumulative process CPU (`GetProcessTimes`) is read at every temp-file growth change,
so each phase also gets a **CPU** number, not only wall. Startup + vendor + import is
separated from the prepass by an `--info` control on the same model and the same
`--load-settings` / `--load-filaments` (it parses and reports the mesh, and does not
slice).

Accuracy controls, pre-declared:

- the tap's wall must agree with the rig's own `GetProcessTimes` wall for the same
  run within **1 %**;
- the tap's own cost is measured (poller CPU) and disclosed;
- every phase number is reported with its repeat spread, and a phase shorter than
  **3x** the tap interval is reported as a bound, not a point.

**Phase 2 — campaign.** The four classic cells (benchy/base x supports off/on),
**3 repeats each**, with the t105 pin (`--support-style=tree_strong`) on the
supports-on cells and the t103 matched keys throughout, plus one `--info` control per
model. Sequential, load sampled at start, end and every tap sample; ratios and
shares are computed within the campaign, never against a different day's Orca.

**Phase 3 — alignment.** Put the measured Orca phases against ticket 104's PnP phase
table for the same four cells (its `phase-split.json` and Table 1) and against
t103/t106's medians of record. Report, per phase, the absolute seconds on both sides
and the multiple. No PnP run is needed: t104's instrumented streams are committed and
its shares are the robust part.

**No production edit, no candidate, no KEEP/DROP campaign.** This is an attribution
take; wall numbers from the tap are attribution only, and PnP's side is quoted from
t104, not re-measured.

## Stop rule

If the temp-file boundary is not observable (the file appears once, at the end), or
if the `--info` control cannot be made comparable to the slice path's startup+import,
**stop and report what was measured**: no invented substage split, no number that
cannot be re-derived from a committed artifact. A capability channel that fails is a
result and must be recorded with the command that failed, not silently dropped.

## Answer

**Verdict: no per-stage timing exists on this install (three channels measured and
all unusable), and the phase split that is observable reframes the map's headline:
Orca's advantage is not that it does less prepass work — its slicing phase is
9.2x–21.8x cheaper than PnP's, and on base most of its wall is model import.**
Evidence: [`evidence/t110-orca-phases/`](../evidence/t110-orca-phases/FINDINGS.md).

### Phase 0 — no usable per-stage channel

| channel | test | result |
| --- | --- | --- |
| `--logfile` CLI log | one benchy classic-off slice at `--debug 4`, tapped every 2 ms | 27 stage markers arrive in **3 bursts**, largest **18 of 27**; the prepass collapses into one arrival; no timestamps at `--debug` 3/4/5 |
| ETW kernel sampling | `wpr -start cpu -filemode`; `xperf -on … -stackwalk PROFILE` | blocked without elevation: "Failed to enable the policy to profile system performance" / "NT Kernel Logger: Invalid flags. (0x3ec)" |
| `--pipe` progress | 90 s named-pipe server, two runs (bare name and `\\.\pipe\…`) | **no connection** (`NO_CONNECTION` both times) though the log records `Will use pipe <name>` |
| `--info` | same settings and model, no slicing | works — the import proxy used below |

### The observable split (medians of 3 per cell, tap ~15 ms)

| cell | wall | startup+import | prepass | emit | tail | Orca slicing |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 1,867.5 ms | **1,229.8 (65.9 %)** | 50.5 \* | 365.8 | 225.5 | 637.8 |
| benchy classic-on | 2,509.8 ms | **1,229.8 (49.0 %)** | 549.8 | 497.5 | 225.2 | 1,280.1 |
| base classic-off | 13,814.5 ms | **10,130.0 (73.3 %)** | 2,103.0 | 1,349.1 | 237.9 | 3,684.5 |
| base classic-on | 24,140.3 ms | **10,130.0 (42.0 %)** | 10,444.8 | 3,230.6 | 287.9 | 14,010.3 |

\* below 3x the tap's worst poll gap (138.9 ms) → a bound, not a point.

### Against PnP (ticket 104, same cells)

| cell | PnP slicing | Orca slicing | slicing multiple | PnP prepass | Orca prepass | prepass multiple | emit multiple |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 7,365 ms | 637.8 ms | **11.6x** | 5,545 | 50.5 \* | ≥ 40x \* | 5.0x |
| benchy classic-on | 11,784 ms | 1,280.1 ms | **9.2x** | 9,595 | 549.8 | **17.5x** | 4.4x |
| base classic-off | 80,216 ms | 3,684.5 ms | **21.8x** | 72,433 | 2,103.0 | **34.4x** | 5.8x |
| base classic-on | 228,317 ms | 14,010.3 ms | **16.3x** | 204,832 | 10,444.8 | **19.6x** | 7.3x |

### What this establishes

1. **Import, not slicing, dominates Orca's base wall** (73.3 % / 42.0 %), and PnP
   wins that phase outright: Orca's startup+import is **10.13 s** on base against
   PnP's **≤ 0.708 s** (≥ 14.3x slower at the same 123 MB file). The feature stack is
   nearly free on Orca's side: a slice with 1 wall loop, 1 shell, 0 % infill and no
   support still costs **13,639.6 ms** against the full cell's 13,814.5 ms (+0.1 s).
2. **The engine gap is 1.7x–3.7x larger than the ratio of record** (median ~2.7x). End-to-end today is
   5.86x / 9.49x (base off/on) and 4.34x / 4.96x (benchy off/on), but the
   *slicing* ratio is **21.8x / 16.3x** and **11.6x / 9.2x**. PnP's fast import was
   masking most of the slicing deficit in every scoreboard the map has quoted.
   Corollary: PnP's per-layer phase alone (1.54 / 1.74 / 6.54 / 12.70 s) is larger
   than Orca's entire slicing (0.64 / 1.28 / 3.68 / 14.01 s) in three of four cells —
   t104's finding 1 holds against process walls, not against slicing.
3. **The prepass is the carrying phase on both engines** (Orca 57.1 % / 74.6 % of its
   base slicing; PnP 89.5 % / 89.4 %), so the destination's prepass-rewrite ruling is
   not an artefact of PnP's phase mix — the difference is absolute seconds.
4. **On the t105-pinned supports-on job the support-stage gap is ~13.7x:** Orca's prepass
   grows **2,103.0 → 10,444.8 ms (+8.34 s)**; PnP's support stages add **~113.9 s**
   (`SupportGeometry` 231 → 91,902 ms = +91.67 s, `SupportAnalysis` 0 → 22,254 ms
   = +22.25 s), a **13.7x** gap on the increment. PnP's
   planner alone is **8.4x** Orca's entire supports-on prepass.
5. **Emit is a 4.4x–7.3x gap**, not a rounding error: PnP's per-layer + postpass is
   1,816 / 2,187 / 7,781 / 23,482 ms against Orca's 365.8 / 497.5 / 1,349.1 /
   3,230.6 ms.
6. **Ledger correction to t104:** its Table 3 reads PnP's implied phases against
   "Orca's whole slice (2.03 / 2.29 / 15.28 / 19.49 s)" — those are *process* walls,
   which are 66–73 % model import on base, so that comparison understates the engine gap
   by ~3–4x. t104's phase shares and counterfactual arithmetic are unaffected.

### Verification

- `verify.py` exits **0** checkout-only (4 cells, 18 rows, 3 repeats each, shares sum
  to 100 %, PnP multiples reproduced from t104); `verify.py --self-test` rejects
  **7/7** negative controls.
- Output identity: G-code byte counts identical across repeats in every cell
  (3,999,822 / 9,481,130 / 20,299,883 / 44,803,778 B), so the tap does not perturb
  the job.
- No production edit, no candidate, no KEEP/DROP campaign, no guest or IR change.

### Disclosures

The tap's real resolution is **~15 ms** (Windows timer granularity; worst poll gap
33–46 ms), so phases under 3x the gap are bounds. `--info` is an **upper bound** on
import, so Orca's prepass/slicing are lower bounds and the multiples are upper
bounds. PnP's side is t104's 2026-11-02 table, Orca's is today, and today's Orca is
~9 % faster than t103's medians — mixed vintage, disclosed. Load 0.25–0.43 system CPU,
wall spread 1.8–6.5 % per cell. The abandoned STL-prefix linearity control (a
truncated mesh is non-manifold; `--info`'s repair path never returned) and the failed
`--pipe` attempts are recorded in `capabilities.json` / `controls.json`, not dropped.

### Ledger caveat (added 2026-11-03 by [ticket 111](111-prepass-rewrite-scope-ruling.md))

The **base-on** multiples in this answer — the 9.49x "end-to-end today", 16.3x
slicing, 19.6x prepass and 13.7x support-stage increment — divide an
**instrumented** PnP span by Orca's **quiet** process wall. t104's own disclosure
puts that instrumented pass at **+43.6 %** over its t103 median and says
instrumented walls are never denominated as wall claims, so those figures are
**upper bounds**, not ratios of record. The uninstrumented paired record is
**160.857 s** ([ticket 106](106-post-pin-supports-on-scoreboard.md)) -> **6.02x**
for the base classic-on cell. The qualitative finding (the engine gap exceeds the
wall ratio) survives; the base-on numbers need re-deriving from t104's implied
walls before they are quoted as scoreboard anchors. Tracked in the map's **Not yet
specified**.

