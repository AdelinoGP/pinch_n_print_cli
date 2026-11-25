# t163 — Accelerated arm on all eight cells: it works, it is output-exact, and it is worth about half a percent

Route: [Accelerated timing arm: three-tool eight-cell board](../../issues/163-accelerated-timing-board.md).
Graduated from: [t162](../t162-accelerated-host-verbatim-path/FINDINGS.md)'s
"not claimed" — the arm builds and is output-exact, but had no timing number.

**Headline: the accelerated arm is now measurable and byte-identical to the
ordinary arm in 8/8 cells, but it is not a route lever: its wall effect across
the eight cells is −13.2 % to +4.4 % with per-cell spreads up to 21.6 %, and the
median cell gains 1.1 %. Its process CPU is lower in 6 of 8 cells (−4.3 % to
+1.4 %), which is the only signal tight enough to read. It does not move the
destination: 0 of 8 cells are won on either arm, and the tightest cell
(`base classic-off`) is 1.389x ordinary but 1.450x accelerated — the arm is
slightly worse exactly where the map is closest.**

## 1. What was measured

One campaign, one window, **three tools**: OrcaSlicer 2.5.1, PnP ordinary
(`target/release`), PnP accelerated (`target/dist-accelerated/developer`, built
by t162). 8 cells × 1 warmup + 3 measured runs each, interleaved; 96 data rows.

**Output identity is complete:** both PnP arms reproduce t151's active set in
**8/8** cells, every run `exit 0`, `degraded: false`, matched keys asserted on
all 24 Orca runs. So the accelerated build is not a different slicer — it is the
same output by construction.

## 2. The board

| cell | ordinary | accelerated | Orca | ordinary/Orca | accelerated/Orca | acc vs ord |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 3.900 s | 3.846 s | 2.189 s | 1.782x | **1.757x** | −1.4 % |
| benchy classic-on | 8.241 s | 7.951 s | 2.941 s | 2.802x | **2.703x** | −3.5 % |
| benchy arachne-off | 4.849 s | 4.209 s | 2.500 s | 1.940x | **1.684x** | **−13.2 %** |
| benchy arachne-on | 8.269 s | 8.199 s | 2.923 s | 2.829x | **2.805x** | −0.8 % |
| base classic-off | 25.283 s | 26.391 s | 18.200 s | **1.389x** | 1.450x | **+4.4 %** |
| base classic-on | 77.537 s | 78.683 s | 28.607 s | 2.710x | 2.751x | +1.5 % |
| base arachne-off | 25.861 s | 25.311 s | 17.755 s | 1.456x | **1.426x** | −2.1 % |
| base arachne-on | 76.732 s | 76.517 s | 29.310 s | 2.618x | 2.611x | −0.3 % |

Ranges: ordinary **1.389x–2.829x**, accelerated **1.426x–2.805x**; accelerated
vs ordinary wall **−13.2 %…+4.4 %**, CPU **−4.3 %…+1.4 %**. Faster in 6 cells,
slower in 2. **0 of 8 won on either arm**; the required cut is 28.0 %–64.7 %
ordinary and 29.9 %–64.3 % accelerated.

## 3. Reading it honestly

**The signal is weaker than the spread on most cells.** Per-cell wall spreads
are 2.9–21.6 %, so only two cells' deltas exceed their own spread:
`benchy arachne-off` (−13.2 %, spreads 9.1 % / 10.5 %) and `base classic-off`
(+4.4 %, spreads 10.5 % / 5.2 %). The median cell delta is **−1.1 %** and the
mean is −1.9 %. On this instrument the arm is **≈0 to a few percent**, which is
two orders of magnitude short of the 28–65 % the destination needs.

**CPU is the tighter channel** (all spreads ≤ 10 %, most < 3 %): the accelerated
arm is CPU-lower in 6 of 8 cells, by −0.8 % to −4.3 %, and CPU-higher only where
wall was also higher (`benchy classic-on` +0.7 %, `benchy classic-off` +1.4 %).
That is the signature of cheaper geometry work, not of a scheduling change.

**Where the map is closest, the arm is worse.** `base classic-off` is the
tightest cell (1.389x ordinary) and the accelerated arm measures 1.450x, i.e.
+4.4 % wall. Nothing about the accelerated build should be relied on to win that
cell.

## 4. The cross-window control this take accidentally provides

The two PnP arms and Orca ran ~20 h after t161's board (the box clock shows
2026-11-18 20:10), so this is a different window and no absolute wall is
comparable. Both arms moved together anyway:

| cell | PnP ordinary t161 → t163 | Orca t161 → t163 | ratio t161 → t163 |
| --- | ---: | ---: | ---: |
| benchy classic-off | 4.076 → 3.900 (−4.3 %) | 2.269 → 2.189 (−3.5 %) | 1.796 → 1.782 (−0.8 %) |
| base classic-off | 22.603 → 25.283 (+11.8 %) | 15.812 → 18.200 (+15.1 %) | 1.429 → 1.389 (−2.8 %) |

That is a second, independent demonstration of t161's method claim: a common
shift moves both arms and leaves the paired ratio nearly intact (−0.8 % and
−2.8 % against wall movements of 3.5–15 %). It also means **no absolute wall in
this take may be compared with t161's**, and none is.

## 5. Disclosures

- **One window, three tools, no cross-window composition.** Orca is 2.5.1
  (t161's tool boundary); every number above is from this campaign.
- **No null band.** This is a board, not a candidate: 3 measured runs per cell
  is enough for a median and a spread, not for a paired significance claim. The
  per-cell deltas are reported with their spreads for exactly that reason.
- **Box load** sampled at 20 s through the window: 114 samples, min 1 %,
  median 76 %, max 100 %. The median is high because the sampler mostly caught
  12-thread runs; it is reported as recorded, not as a quiet-box claim.
- **Same binary, same profile tree:** the accelerated snapshot stages its own 24
  modules from the same guests (`module diagnose` provenance `external`), so the
  output identity is a real cross-build check, not a re-run of one binary.
- **The 2.5.1 vendor tree is the staged tag** (`02.08.00.16`), unchanged from
  t161; matched keys are asserted on every Orca run.

## 6. What this means for the map

- The destination's ordinary-arm arithmetic stands unchanged (0/8, required cut
  28.0 %–64.6 %).
- **Accelerated corroboration is now available** for any future candidate, and
  every candidate should be checked on it — the arm can move a cell by −13 % to
  +4 %, so a candidate that only clears the floor on the ordinary arm may not
  clear it here.
- The arm itself is not worth pursuing as a route term: it is worth ≈0.5 % on the
  median cell, and it costs wall on the tightest one.

## 7. Can the arm be improved? A profile answer

Asked after the board: the arm buys ~1 %, so is there headroom in it? Both
binaries were run on **base classic-off** with `--profile` (fuel is deterministic
and exact; the wall columns are accumulated thread-time and are indicative only,
and profiling costs throughput, so nothing here is a timing claim).

| quantity | ordinary | accelerated | delta |
| --- | ---: | ---: | ---: |
| total wasm fuel | 190.753 B | 117.949 B | **−38.2 %** |
| `classic-perimeters` total fuel | 142.545 B | 69.740 B | −51.1 % |
| `classic-perimeters` **self** fuel | 109.995 B | 37.191 B | **−66.2 %** |
| `classic-perimeters` self wall | 9.855 s | 4.777 s | −51.5 % |
| `offset2_ex` (host Clipper call) | 32.549 B | 32.549 B | +0.00004 % |
| wall per unit of self fuel | 0.0896 s/B | 0.1285 s/B | **+43.4 %** |
| G-code | `9bb18f0e…` | `9bb18f0e…` | identical |

**1. The arm already does its job.** Two thirds of the module's own work is
gone — 110.0 B → 37.2 B fuel — with byte-identical output. Whatever the arm was
designed to remove, it removes.

**2. It buys little wall because the module is not where the cell's wall is.**
`classic-perimeters` accumulates 12.90 s of thread-time against a 21.2 s
profiled run; after the pipeline's worker concurrency that is a few percent of
wall, which is exactly the board's median (−1.1 %) — and on `base classic-off`,
where the arm measured **+4.4 %**, the saving never reaches the critical path at
all. The module's remaining large term is a *host* call (`offset2_ex`, 32.5 B
fuel, unchanged by the cfg).

**3. There is one measured inefficiency inside the arm.** The indexed path
spends **43 % more wall per unit of fuel** (0.0896 → 0.1285 s per billion), so a
66 % fuel cut becomes only a 52 % wall cut inside the module. That is the
signature of memory-latency-bound work — an R-tree traversal and index build
against a cache-resident scan — and it is the only place a tuning change could
pay.

**4. The cells where the arm pays are the ones where the module owns more of the
cell.** `benchy arachne-off` is **−13.2 %** in the board. Same mechanism, more
module share — not a counter-example, and it bounds the ceiling by composition
rather than by any property of the index.

**5. What would actually improve it, and what that costs.**
(a) Narrow the per-fuel regression (index layout, build-vs-query amortization) —
bounded by the module's wall share, so the base-cell ceiling is ~3 %.
(b) Widen what the cfg covers to the **host** Clipper work that owns the base
cells' wall — `host:shell_classification` is 104.2 s of accumulated thread-time
in this same profile, 73 % of all native work. That is a different design
problem (it is the operand-volume route), not a tuning change, and no ticket is
implied here.

**Conclusion: the arm is not the route, and optimizing it cannot make it the
route.** It is already within roughly a factor of two of its own ceiling on the
base cells, and the remaining wall sits in host Clipper work the cfg does not
reach.
