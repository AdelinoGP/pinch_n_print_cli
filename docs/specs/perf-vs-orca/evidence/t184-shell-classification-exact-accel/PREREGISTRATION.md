# t184 gate pre-registration (written before window 2 was measured)

Recorded 2026-11-23 ~06:45 UTC, after window 1's ordinary A/B (`target/t184/ab/ab`)
and null control (`target/t184/ab/null`) completed and before any window-2 run.

## What window 1 showed

- Ordinary 5-pair A/B: candidate won 40/40 wall pairs; median wall saving
  +5.48 % … +15.29 %; CPU median lower in 8/8.
- Null control (baseline vs baseline, 3 pairs): wall medians 0.66–5.79 %, worst
  pair 11.73 % (base-classic-on). Same-binary base-classic-on walls ranged
  22.3–27.8 s while process CPU stayed within 112.5–118.3 s, i.e. wall noise from
  scheduling/frequency, not work. Load samples 7 % → 9 %.
- On the literal standing gate (median wall saving ≥ 0.80 %, strictly above the
  cell's null median wall, CPU lower), base-classic-on (+5.48 % vs null median
  5.79 %) is INSIDE-NULL-MEDIAN in window 1. All other cells clear it.

## Rule fixed now

1. Window 1 is kept and reported in full; nothing is discarded.
2. A second complete ordinary window is measured with the same script and
   design: 5 interleaved pairs (`-Mode ab -BatchId ab2`) and 3 null pairs
   (`-Mode ab -NullControl -BatchId null2`), all 8 cells, CPU load sampled.
3. **Verdict of record:** the literal standing gate applied to the *pooled*
   windows — 10 ordinary pairs and 6 null pairs per cell — using the same
   criteria. Per-window verdicts are disclosed beside it.
4. No further windows are added to change the verdict. If the pooled gate
   fails in any cell, the ticket records the failure and asks for a human ruling.
