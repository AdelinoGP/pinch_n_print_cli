# t161 — Post-t158 eight-cell scoreboard: 0 of 8 won, 1.366x–2.718x, and both arms measured slower than t145's window

Route: [Post-t158 eight-cell scoreboard](../../issues/161-post-t158-scoreboard-refresh.md).
Graduated from: map fog, "The benchy per-layer program is margin, not route
(t154)", item (b) — the ratio-of-record chain needed a fresh paired window.

**Headline: measured in one interleaved window on the current tree (HEAD
`50594ead`, t158 landed), PnP is 1.366x–2.718x Orca's wall across the eight
matched cells and wins 0 of 8. The required cut is 26.80 %–63.21 %. The two
base supports-off cells are now the tightest cells in the matrix (1.366x /
1.429x) and both sit inside the 1.5x margin. The board also records a fact that
constrains every cross-window comparison in this tracker: **the installed
OrcaSlicer is 2.5.1 (binaries dated 2026-11-13 23:59–11-14 00:00), where the map
records 2.5.0**, and the Orca arm measured 5.4–24.9 % slower in all eight cells
than on t145's board with its CPU nearly flat — so the movement is execution
efficiency, not PnP's work, and the tool change is the leading explanation.**

## 1. What was measured

- One campaign, one window, one CSV: 8 cells × {Orca, PnP ordinary}, 1 warmup +
  3 measured runs per cell, interleaved. Raw:
  `target/matched-pair/t161-scoreboard/scoreboard-rows.csv` (64 data rows).
- Driver: `evidence/matched-pair/run_scoreboard.ps1` unchanged, with the
  matched job and the `tree_strong` supports-on pin it already carries. The
  driver asserts the matched keys from each arm's own resolved-config appendix
  and validates every run; it aborted nothing.
- **Output identity: 8/8.** Every PnP cell produced one hash across its three
  measured runs and each equals t151's active set
  ([`evidence/t151-skirt-hull-contour/reference.json`](../t151-skirt-hull-contour/reference.json)),
  i.e. `3d28d1b3…` / `eb202973…` / `39ad6289…` / `77c3c578…` / `9bb18f0e…` /
  `eccbd523…` / `2f249cdb…` / `1811425f…`. So the tree that was timed is the
  tree the map describes.
- Every PnP run `exit 0`, `degraded: false`, and **zero** `exact-Z occupancy`
  diagnostics on all four supports-on cells (t148's plane carve holds).
- Matched keys asserted from all 24 Orca runs: `brim_width = 8`,
  `brim_type = outer_only`, `skirt_loops = 6`, `skirt_distance = 3`,
  `skirt_height = 1`, `initial_layer_line_width = 0.4`,
  `tree_support_brim_width = 0`, `enable_arc_fitting = disabled`,
  `elefant_foot_compensation = 0`.

## 2. The board

Medians of three measured runs, ordinary arm, one window. Required cut is the
fraction of PnP's wall that must go for the cell to tie Orca; the 1.5x column is
the same arithmetic against a 1.5x margin.

| cell | PnP | Orca | ratio | required cut | 1.5x cut |
| --- | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 4.076 s | 2.269 s | **1.796x** | 44.31 % | 16.47 % |
| benchy classic-on | 8.144 s | 3.050 s | **2.671x** | 62.55 % | 43.83 % |
| benchy arachne-off | 3.790 s | 2.113 s | **1.794x** | 44.25 % | 16.37 % |
| benchy arachne-on | 7.337 s | 2.831 s | **2.591x** | 61.41 % | 42.12 % |
| base classic-off | 22.603 s | 15.812 s | **1.429x** | 30.04 % | **−4.94 %** |
| base classic-on | 70.754 s | 26.030 s | **2.718x** | 63.21 % | 44.82 % |
| base arachne-off | 22.505 s | 16.474 s | **1.366x** | 26.80 % | **−9.80 %** |
| base arachne-on | 71.377 s | 27.219 s | **2.622x** | 61.87 % | 42.80 % |

Ranges: **1.366x–2.718x** overall, **1.794x–2.671x** benchy-only,
**1.366x–2.718x** base-only; required cut **26.80 %–63.21 %**. **0 of 8 cells
won.** The two `base *-off` rows carry a **negative** 1.5x cut, which is the
only sense in which a cell is "inside" a margin here: PnP would beat a 1.5x
slower Orca on those cells today.

## 3. What moved, and in which direction

Against t145's board (2026-11-13), which is the last time both tools were timed
on all eight cells. **Both arms are shown, because the ratio alone hides which
arm moved.**

| cell | PnP Δ | Orca Δ | ratio then → now | ratio Δ |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | +4.2 % | **+18.5 %** | 2.043x → 1.796x | −12.1 % |
| benchy classic-on | +24.3 % | +24.9 % | 2.683x → 2.671x | **−0.5 %** |
| benchy arachne-off | +3.2 % | +6.6 % | 1.853x → 1.794x | −3.2 % |
| benchy arachne-on | +9.7 % | +9.2 % | 2.580x → 2.591x | +0.4 % |
| base classic-off | −18.9 % | **+17.9 %** | 2.078x → 1.429x | −31.2 % |
| base classic-on | +0.7 % | +11.2 % | 3.001x → 2.718x | −9.4 % |
| base arachne-off | −24.0 % | +10.4 % | 1.984x → 1.366x | −31.1 % |
| base arachne-on | −20.0 % | +5.4 % | 3.455x → 2.622x | −24.1 % |

Against t148's supports-on ratios (2.705x / 2.829x / 3.356x / 3.248x for
benchy classic-on / benchy arachne-on / base classic-on / base arachne-on),
today's are **2.671x / 2.591x / 2.718x / 2.622x** — all lower, but see §4 before
reading that as progress.

Three readings that survive the movement:

1. **The paired ratio is state-robust where the wall is not.** The two benchy
   supports-on cells moved **+24.3 % / +24.9 %** on both arms and their ratio
   moved **−0.5 % / +0.4 %**. A ~25 % box shift therefore does not, by itself,
   move those ratios — which is what makes the interleaved protocol the right
   instrument and what makes single-arm cross-window wall comparisons unsafe.
2. **The base supports-off improvement is corroborated by an independent
   composition.** The landed deltas predict base classic-off
   `0.8792 × 0.8513 = 0.748` (−25.2 %) and base arachne-off `0.9063 × 0.8536 =
   0.774` (−22.6 %) of t145's PnP wall; the measured movement is **−18.9 %** and
   **−24.0 %**. The direction, order of magnitude and cell ordering agree.
3. **The benchy supports-off ratio improvement is not PnP's.** PnP rose **+4.2 %**
   on benchy classic-off while Orca rose **+18.5 %**; the entire −12.1 % ratio
   movement is the Orca arm, and the map must not bank it as engine work.

## 4. The instrument changed: OrcaSlicer 2.5.1 replaced the recorded 2.5.0

The Orca arm measured **+5.4 % to +24.9 %** above t145's medians, cell for cell,
with no exception. The cause is **not** established as box state, and an
earlier reading of this take that called it "state, not job" was wrong: the
installed slicer is a **different binary** from the one the map records.

**Identity.** `%APPDATA%/OrcaSlicer/OrcaSlicer.conf` carries
`"header": "OrcaSlicer 2.5.1"`; the install's own binaries are dated
**2026-11-13 23:59:44 – 2026-11-14 00:00:44** (`orca-slicer.exe` sha256
`5013b1bd…`, 292,160 bytes; the OCCT DLLs land at 00:00), and the map's pinned
description of the rig is "the installed `C:\Program Files\OrcaSlicer`
(files dated 2026-11-06 23:00, embedded version `2.5.0`)" (map, *Notes*,
canonical source pin). **Settings identity is unchanged:** the vendor cache
stamp is still `2.08.00.16` / `2.8.0.16`, so the staged profile tree the rig
loads — and the nine matched keys asserted per run — are the same tree as
before. Only the executable moved.

**Which boards sit on which binary.** t145's board was committed
2026-11-13 00:37 and t148's 04:06, both **before** the install, so the map's
`1.853x–3.455x` and t148's supports-on ratios are **2.5.0** numbers. This board
is **2.5.1**. Nothing in the tracker noticed the swap, because the rig gates the
*profile* version and never records the *application* version.

**The movement is efficiency, not work.** Orca's process CPU (a work measure
independent of clock speed) is nearly flat while its wall is not:

| cell | Orca CPU Δ | Orca wall Δ | PnP effective parallelism | Orca effective parallelism |
| --- | ---: | ---: | ---: | ---: |
| benchy classic-off | **−2.2 %** | **+18.5 %** | 4.79 → 4.34 (−9.4 %) | 3.41 → 2.82 (**−17.4 %**) |
| benchy classic-on | +11.7 % | +24.9 % | 5.34 → 4.68 | 4.25 → 3.81 (−10.5 %) |
| benchy arachne-off | +0.2 % | +6.6 % | 5.36 → 4.83 | 3.87 → 3.64 (−6.0 %) |
| benchy arachne-on | +1.8 % | +9.2 % | 5.49 → 5.23 | 4.67 → 4.35 (−6.8 %) |
| base classic-off | +7.8 % | +17.9 % | 7.45 → 7.21 | 2.17 → 1.99 (−8.6 %) |
| base classic-on | +3.6 % | +11.2 % | 7.01 → 7.51 | 4.15 → 3.86 (−6.8 %) |
| base arachne-off | +5.7 % | +10.4 % | 7.18 → 7.41 | 2.55 → 2.44 (−4.3 %) |
| base arachne-on | +3.1 % | +5.4 % | 5.58 → 7.49 | 4.13 → 4.04 (−2.2 %) |

Two things follow. First, the extra wall is not extra work: Orca's CPU moved
−2.2…+11.7 % while its wall moved +5.4…+24.9 %, so the process is finishing the
same job less efficiently. Second, **Orca's effective parallelism (CPU/wall)
fell in 8/8 cells** (−2.2…−17.4 %) while **PnP's did not** (−12.2…+34.4 %,
mixed) — a box-wide core shortage would have hit both arms. That makes the
2.5.1 binary the leading explanation.

**What cannot be separated.** 2.5.0 is no longer installed, so the tool change
and machine state are confounded: the state control (benchy classic-off
re-run after the batch: Orca 2.3313 / 2.7851 / 2.5758 s, PnP 5.4075 / 4.7636 /
4.8429 s) shows only that the slower behaviour persists **within 2.5.1**, not
why. What *is* established: at least that cell's job is unchanged, because its
Orca output is 4,077,463 bytes — t145's exact figure — and byte-constant across
runs.

**Rules this take adds.** (1) OrcaSlicer version is a currency boundary:
numbers measured before 2026-11-13 23:59 are 2.5.0 and may not be composed with
2.5.1 numbers, in either direction. (2) The required-cut arithmetic is
therefore not comparable across that boundary either: t145's
`46.03 %–71.06 %` and today's `26.80 %–63.21 %` are each correct only for their
own tool and window. (3) Any future campaign must record the Orca executable's
identity beside the run; the rig's profile-stamp gate cannot see a binary swap.

## 5. What this means for the route

- **0 of 8 won.** The destination is unchanged.
- **The identity of the tightest cells changed.** t145's tightest were the
  benchy supports-off pair (1.853x / 2.043x, cut ~46–51 %); today's are the base
  supports-off pair (1.366x / 1.429x, cut ~27–30 %). Both base supports-off
  cells are inside the 1.5x margin; no benchy or supports-on cell is.
- **The prepass route is untouched by this board.** The map's phase shares
  (prepass 63.5–82.4 %, per-layer 14.4–29.6 %) were measured **before** t158
  landed, and the route projections built on them (`base classic-on 1.0508x →
  0.8343x` at canonical prepass cost) are cross-window compositions of those
  shares and t145's walls. This take neither re-attributes the phases nor
  re-derives the projection; both are recorded as limits, not as findings.

## 6. Disclosures

- **Accelerated timing arm not taken, for two now-separated reasons.** The
  driver's freshness preflight runs `build-guests --check` (exit 0) and runs the
  *accelerated* check only when the tool set includes `pnp-accelerated`, so this
  ordinary-only batch never validated that tree. Checking it afterwards found
  **45 stale guests** — the whole accelerated namespace, a pre-existing state
  (it is only rebuilt on demand), not caused by this take. It was rebuilt
  **after** the campaign, so no timing run was contaminated:
  `build-guests --accelerated` built 45 guests and `--accelerated --check` now
  exits 0 (ordinary `--check` also exits 0). With the artifacts fresh,
  `cargo xtask dist --accelerated --edition developer` **still fails at exit 1**
  in `libmimalloc-sys` — `c1xx: fatal error C1083: Cannot open source file:
  '\\mimalloc-static.cc'` from the MSVC 14.51 `cl.exe` invocation on the
  `\\?\` verbatim path, the documented t142 failure. So the accelerated arm
  remains unavailable on the host build, and the stale-artifact obstacle that
  would also have blocked it is now cleared.
  **Correction (2026-11-18, [t162](../../issues/162-accelerated-host-verbatim-path.md)):
  that attribution was wrong** — the failure was a verbatim `CARGO_TARGET_DIR`
  reaching `cl.exe`, not the toolchain, and the accelerated arm builds and is
  output-identical 16/16 after t162.
- **Accelerated output parity is *not* re-verified here.** It remains on
  t144/t151's record (8/8 on the accelerated component tree with the ordinary
  host); this take ran the ordinary arm only.
- **First Orca run of the window was 33.10 s** against 2.24–2.28 s for the same
  cell afterwards — t98's first-touch vendor-tree premium. It is the warmup and
  never enters a median, and every later Orca warmup in the window
  (3.00 / 2.43 / 2.81 / 15.88 / 25.84 / 16.67 / 27.02 s) sits at its cell's
  normal cost.
- **One Orca measurement is an outlier and stays disclosed, not dropped:**
  base classic-off run 2 measured **29.807 s** against 15.812 s and 15.202 s
  (spread 92.4 % on that cell). It is excluded from the median by being the
  largest of three, which is why the cell's median is 15.812 s; PnP's own spread
  on that cell is 5.3 %.
- **Box load** sampled at 20 s cadence through the window: 62 samples, min 2 %,
  median 69.5 %, max 100 %. The series alternates between single digits and
  saturation; saturation samples coincide with heavy runs (including the 123 MB
  base model import), so the series cannot separate the workload from background
  activity at this cadence and is reported as recorded, not as a quiet-box claim.
- **Supports-on cells carry the `tree_strong` pin** (t105's protocol choice;
  measured price Orca +39.6–44.4 % base / +7.6–9.5 % benchy against the organic
  default). Every supports-on ratio above is quoted under that disclosure.
- **No causal claim is taken from any cross-window movement.** Per §4, the Orca
  arm moved too, so the deltas in §3 are descriptive.
- **Three measured runs per cell**, so no null band is derived here: this is a
  board, not a candidate gate.

## 7. Evidence

| artifact | what it is |
| --- | --- |
| [`reduced/summary.json`](reduced/summary.json) | the board: per-cell runs, medians, ratios, cuts, spreads, identity, matched-key audit, state control, load summary |
| [`reduced/load-samples.csv`](reduced/load-samples.csv) | the box-load samples as recorded |
| [`reduce_t161.py`](reduce_t161.py) | rebuilds `summary.json` from the raw campaign tree + t151's reference |
| [`verify.py`](verify.py) | stdlib-only re-derivation of every quoted number — **PASS 226/226** |
| [`REPRODUCE.md`](REPRODUCE.md) | the exact commands, prerequisites and restore |

`python verify.py --self-test` rejects **16/16** mutations (median, ratio,
required cut, hash, identity flag, missing cell, matched key, diagnostics,
cells-won count, protocol row count, cut range, benchy range, spread, degraded
flag, load ordering, state-control median), so the checks are not vacuous.
