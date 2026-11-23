# t106 — Post-pin supports-on Orca scoreboard

Route: [ticket 106](../../issues/106-post-pin-supports-on-scoreboard.md).
Measurement-only take at HEAD `e2294453` (tree carries this take's evidence and
nothing else). OrcaSlicer **2.5.0-beta** (CLI log line, binary
`7663f5d3…` — the same binary ticket 105 measured), source-manifest vendor tree
`target/orca-vendor/`, four supports-on cells, ordinary Orca only. The PnP side
is untouched: the denominators are ticket 103's PnP ordinary medians, which a
style pin on Orca cannot move.

## Headline

**With Orca pinned to `tree_strong`, PnP's four supports-on ratios are 6.02x /
6.11x (base classic / arachne) and 4.77x / 4.53x (benchy classic / arachne)**, so
the park still needs **66.9–75.5 %** of each cell removed. Every post-pin Orca
median is **slower** than ticket 103's pre-pin one, and every ratio therefore
falls from the archive: **8.25x → 6.02x**, **7.00x → 6.11x**, **5.43x → 4.77x**,
**4.78x → 4.53x**.

Ticket 105's transfer — the pre-pin median scaled by the measured style price —
**agrees in two cells and misses in two**. The misses are **not** evidence about
the pin: the pinned job is byte-for-byte the job ticket 105 measured, and today's
run is **1.9–9.6 % faster on that identical job**. The transfer compounds two
cross-day drifts (ticket 103's baseline vintage and today's), so it is retired as
an absolute predictor; the measured table below is the scoreboard of record.

## Method

- **Single arm, pinned.** Orca only (`-Tools orca`), `support_style =
  tree_strong` passed explicitly and asserted per row from the resolved-config
  appendix, so no row can silently inherit a version default. Every retained row
  resolved `support_style = tree_strong` and `enable_support = 1`.
- **Schedule.** 1 warmup + 3 measured per cell — ticket 103's own schedule, so
  the pre-pin and post-pin medians rest on the same sample count.
- **Quiet window.** `load.txt`: start 16.4 %, round 1 14.1 → 9.2 %, round 2
  15.2 → 6.0 %, round 3 3.7 → 5.9 %, end 3.3 %. No starvation exclusions: all
  three samples retained in all four cells.
- **Pre-registered transfer rule** (`REPRODUCE.md`): a cell agrees when
  `|measured − predicted| ≤ max(t106 measured range, t103 measured range)`, a
  range being `max − min` over that take's own samples. Written before
  measuring; the rule is not re-tuned after the fact.
- **Output disclosure.** Bytes, SHA (per run), TYPE counts, support and
  support-interface section counts.

## Table 1 — post-pin medians, ratios and the park

| cell | t103 pre-pin Orca | post-pin Orca | delta | PnP ordinary | ratio | cut to park |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| base classic-on | 19.488 s | **26.706 s** | +7.218 s / +37.04 % | 160.857 s | **6.02x** | 75.10 % |
| base arachne-on | 23.028 s (n=2) | **26.365 s** | +3.338 s / +14.49 % | 161.139 s | **6.11x** | 75.46 % |
| benchy classic-on | 2.286 s | **2.600 s** | +0.314 s / +13.74 % | 12.403 s | **4.77x** | 68.56 % |
| benchy arachne-on | 2.553 s | **2.698 s** | +0.145 s / +5.69 % | 12.216 s | **4.53x** | 66.87 % |

Measured ranges (3 samples): base classic-on 25.876–26.884 s, base arachne-on
26.149–27.050 s, benchy classic-on 2.529–2.622 s, benchy arachne-on
2.676–2.709 s.

CPU corroborates that the pinned style does more work, not just slower work:
median CPU **102.41 s / 109.05 s** on the base cells against ticket 103's pre-pin
**49.84 s / 59.15 s** (cpu/wall 3.84 / 4.12 against 2.56), and **10.83 s /
11.94 s** on benchy against 8.77 s / 10.20 s. No sample fell below the 0.75x
starvation threshold.

## Table 2 — ticket 105 transfer versus measurement

Prediction is `t103 pre-pin median × (1 + t105 style delta)`.

| cell | t103 pre-pin | t105 style price | predicted | measured | diff | tol | verdict |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| base classic-on | 19.488 s | +44.42 % | 28.145 s | 26.706 s | −1.439 s / −5.11 % | 2.1195 s | **agrees** |
| base arachne-on | 23.028 s | +39.55 % | 32.135 s | 26.365 s | −5.770 s / **−17.95 %** | 4.6696 s | **disagrees** |
| benchy classic-on | 2.286 s | +9.48 % | 2.503 s | 2.600 s | +0.097 s / +3.89 % | 0.0926 s | **disagrees** |
| benchy arachne-on | 2.553 s | +7.60 % | 2.747 s | 2.698 s | −0.049 s / −1.78 % | 0.1513 s | **agrees** |

In ratio terms the transfer predicted **5.72x / 5.01x / 4.96x / 4.45x**;
measurement gives **6.02x / 6.11x / 4.77x / 4.53x**. A higher ratio is worse for
PnP, so on **three of four cells the transfer was optimistic for PnP**: it
predicted a larger post-pin Orca wall than Orca actually took.

The benchy classic-on miss is **0.004 s past its 0.093 s tolerance** — nominal,
and disclosed rather than rounded away. The base arachne-on miss is material and
is explained by Table 4, not by the pin.

## Table 3 — the pin's job, and that it reproduces across sessions

| cell | t103 pre-pin bytes | t106 post-pin bytes | delta | t105 strong bytes | t106 support / interface |
| --- | ---: | ---: | ---: | ---: | ---: |
| base classic-on | 33,366,060 | 44,803,778 | +34.28 % | 44,803,778 | 739 / 423 |
| base arachne-on | 32,363,932 | 43,802,018 | +35.34 % | 43,802,018 | 742 / 426 |
| benchy classic-on | 7,544,100 | 9,481,130 | +25.68 % | 9,481,130 | 186 / 57 |
| benchy arachne-on | 8,367,863 | 10,331,360 | +23.46 % | 10,331,360 | 188 / 58 |

**All four byte counts equal ticket 105's `tree_strong` arm exactly**, on a
different day, in a separate campaign. The pinned job is reproducible; the
timing around it is not. This is also the non-vacuity witness: had the rig
silently run the vendor default, bytes would have sat at the pre-pin volumes
(ticket 103's column), 23–35 % lower.

Orca's g-code **SHA changes every run** — the appendix embeds timing, so ticket
103 recorded three distinct SHAs for one cell and ticket 105 recorded four. SHA
is therefore never a job-identity signal on the Orca side, and this take does not
use it as one.

## Table 4 — why the transfer misses: two cross-day drifts

Both columns compare the **same measured quantity** against ticket 105's own
session. Column 1 is ticket 103's pre-pin median against ticket 105's organic
arm; column 2 is today's pinned median against ticket 105's pinned arm (same job,
identical bytes).

| cell | t103 baseline vs t105 organic | t106 pinned vs t105 pinned |
| --- | ---: | ---: |
| base classic-on | −4.70 % | **−9.57 %** |
| base arachne-on | **+12.25 %** | **−7.90 %** |
| benchy classic-on | −5.60 % | −1.93 % |
| benchy arachne-on | −3.84 % | −5.55 % |

The transfer multiplies a t103-vintage median by a t105-vintage ratio, then
compares against a t106-vintage measurement, so its error is the sum of those
two drifts. For **base arachne-on** they point the same way and add up: ticket
103's baseline sits **12.25 % high** (and rests on **2** retained samples after
its own starvation exclusion, with a 4.67 s spread — ticket 105 already flagged
this as "ticket 103's own noisy cell"), while today's run is **7.90 % fast**.

## Findings

1. **The post-pin scoreboard is 4.53x–6.11x, park still binding.** Best cell is
   benchy arachne-on at 4.53x; the park needs **66.9–75.5 %** of each cell
   removed. The pin is a fairness fix and does not move the destination.
2. **Pinning costs Orca on every cell it touches, measured today-vs-t103:**
   +37.0 % / +14.5 % base, +13.7 % / +5.7 % benchy. **All four** ratios
   therefore get worse than the archive's pre-pin numbers (8.25x → 6.02x,
   7.00x → 6.11x, 5.43x → 4.77x, 4.78x → 4.53x).
3. **The transfer is not an absolute predictor.** It is a product of two
   mixed-vintage terms; it agreed on two cells and missed on two, one materially
   (base arachne-on, −17.95 %). Retired as a source of quotable post-pin medians.
4. **The pin's job is exactly reproducible; its timing is not.** Identical byte
   counts to ticket 105 across all four cells, 1.9–9.6 % faster today on that
   identical job. Any future comparison must be paired same-session to price a
   style, and any absolute median must name the day it was measured.
5. **Neither derived column dominates the other.** t105's within-take ratios
   (5.45x / 5.63x / 4.68x / 4.28x) and its derived transfer (5.71x / 5.01x /
   4.96x / 4.45x) each land closer to today's measurement on two of four cells,
   with per-cell misses from 0.08x to 1.10x. Both columns carry vintage error;
   only the measured table answers "what is Orca's post-pin wall".

## Disclosures

- **Load.** `load.txt` records 16.4 % at start, 14.1 → 9.2 % (round 1),
  15.2 → 6.0 % (round 2), 3.7 → 5.9 % (round 3), 3.3 % at end. The take was
  started only after an unrelated game process and a **sibling checkout's**
  `cargo test -p slicer-runtime` e2e binary (a `D:\slicerProject\pinch_n_print_cli`
  tree, not this workspace) had both exited; load then fell monotonically
  through the batch. No round ran above 16.4 %.
- **Warmup.** One per cell, discarded, present in `runs.csv` with `warmup=1`.
- **Starvation.** No exclusions: 12/12 measured samples retained (3 per cell,
  cpu/wall 3.84–4.43).
- **Mixed vintage, unavoidable in scope.** No organic arm was re-measured: the
  ticket's scope is the pinned Orca side only. The style price is therefore
  quoted as measured-today-vs-ticket-103 and cross-checked against ticket 105's
  paired same-session price (Table 4); it is **not** a new paired style price.
- **t103's base arachne-on baseline is the weak link** — 2 retained samples,
  4.67 s spread on a 23.03 s median. Its `+12.25 %` offset from ticket 105's
  organic arm is larger than the whole spread of today's three samples.
- **Not claimed.** No PnP runs, no config change, no rig change, no production
  edit, no parity port. Arachne support correctness is untouched.
- **Freshness.** `cargo xtask build-guests --check` exit **0** (`freshness.txt`);
  the PnP side was not exercised, but the map's discipline wants the gate on the
  record.

## Reproduction

`REPRODUCE.md` in this directory. Committed reduced set: `runs.csv` (16 rows),
`summary.json`, `provenance.json`, `load.txt`, `freshness.txt`.
`verify.py --self-test` exits 0 checkout-only (75/75 claims, 9/9 negative
controls rejected). Raw batch dirs, g-code and Orca logs are gitignored under
`target/matched-pair/t106-*`.
