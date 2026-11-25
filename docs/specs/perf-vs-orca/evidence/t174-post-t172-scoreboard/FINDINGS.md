# t174 findings — post-t172 three-tool eight-cell scoreboard refresh

Ticket: [Post-t172 eight-cell two-tool scoreboard refresh](../../issues/174-post-t172-scoreboard-refresh.md).
No production code change. Measures one interleaved eight-cell × three-tool
window (`orca`, `pnp-ordinary`, `pnp-accelerated`) at HEAD `0ce4be39` (last
code commit `48a5e583`, t172) against OrcaSlicer **2.5.1** (`5013b1bd…`).

## Verdict

- **Wall clock:** PnP **decisively beats OrcaSlicer in 2 of 8 cells** on both
  build arms, with **strict separation** across all three measured runs and
  zero starvation flags:
  - `base-classic-off`: **0.7687x** ordinary (`13.599 s` vs `17.690 s`, −23.13 %)
    and **0.7656x** accelerated (`13.543 s`, −23.44 %).
  - `base-arachne-off`: **0.7921x** ordinary (`14.872 s` vs `18.774 s`, −20.79 %)
    and **0.7739x** accelerated (`14.530 s`, −22.61 %).
- **Process CPU contradiction (Destination ruling required):** On
  `GetProcessTimes` (kernel+user CPU seconds), PnP burns **more** CPU than
  OrcaSlicer in **all 8 cells** (**1.280x–2.940x** ordinary, **1.243x–2.908x**
  accelerated) — specifically **2.126x / 2.066x** on `base-classic-off`
  (`70.92 / 68.91 s` vs `33.36 s`) and **1.844x / 1.810x** on
  `base-arachne-off` (`77.67 / 76.25 s` vs `42.13 s`). The wall win is real,
  unstarved, and driven by higher effective thread parallelism (**5.18–5.24x**
  vs Orca's **1.89–2.24x**).
- **Official Destination tally held at 0 of 8 pending human ruling:** Under the
  literal Destination clause (*"lower median uninstrumented wall-clock (process
  CPU corroborating, never contradicting)"*, `map.md`) and `SCOREBOARD.md`'s
  *"same ordering, never contradicting"* / *"n>=4, strict separation"* notes,
  neither cell is unilaterally marked closed here. Both tallies are disclosed
  (**2/8 wall-clock wins with strict separation; 0/8 under literal
  `CPU <= Orca`**), and
  [Destination CPU-corroboration and cell-closure ruling](../../issues/176-destination-cpu-corroboration-ruling.md)
  is graduated for the human decision.
- **Official ratios of record (quiet window, ordinary / accelerated):**
  - `benchy-classic-off`: **1.1438x / 1.1179x** (cut 12.57 % / 10.55 %)
  - `benchy-classic-on`: **1.8704x / 1.9162x** (cut **46.54 % / 47.81 %** — **new binding cell**)
  - `benchy-arachne-off`: **1.1548x / 1.0640x** (cut 13.40 % / 6.01 %)
  - `benchy-arachne-on`: **1.6652x / 1.6743x** (cut 39.95 % / 40.27 %)
  - `base-classic-off`: **0.7687x / 0.7656x** (wall win, −23.13 % / −23.44 %)
  - `base-classic-on`: **1.7773x / 1.7041x** (cut 43.73 % / 41.32 %)
  - `base-arachne-off`: **0.7921x / 0.7739x** (wall win, −20.79 % / −22.61 %)
  - `base-arachne-on`: **1.7382x / 1.6843x** (cut 42.47 % / 40.63 %)
- **Correctness & output identity:** **16/16** cell × arm PnP outputs (all 64
  PnP runs including warmups in the quiet board, plus all 64 in the disclosed
  loaded first take) match t172's active `reference.json` byte-for-byte
  (sha256 + bytes), `exit_code = 0`, `degraded = false`, `fatal = 0`,
  `non_fatal = 0`, and **code-1200 count = 0**. All **10 matched keys** (9 CSV
  keys + `support_style`: `tree_strong` on supports-on, `default` on
  supports-off) are asserted on **32/32** Orca runs in each window.

---

## 1. Stamped identities and freshness gates

All builds and checks ran before the campaigns; zero production files were
modified.

| Artifact | Path / Version | Identity |
| --- | --- | --- |
| Git HEAD | `0ce4be393eeb06ea21cc07834e293a94fc33b45e` | last code commit `48a5e58312d202897554f0f48b4a807434c75719` (t172) |
| Ordinary guests check | `cargo xtask build-guests --check` | exit **0** (`EXIT_FRESH`) |
| Accelerated guests check | `cargo xtask build-guests --accelerated --check` | exit **0** (`EXIT_FRESH`) |
| OrcaSlicer | **2.5.1**, vendor `02.08.00.16`, `C:/Program Files/OrcaSlicer/orca-slicer.exe` | 292,160 B, mtime `2026-11-13T23:59:44-03:00`, sha256 `5013b1bd…`, `support_style = tree_strong` |
| PnP ordinary exe | `target/release/pnp_cli.exe` | sha256 `06b05b6b2c4e8a5616272dde4baf90b8b09b0ee402613b62c8ce5736b3df17e9` |
| PnP ordinary modules | `modules/core-modules` (24 `.wasm`, `module diagnose` pass) | tree sha256 `e75be5c0744101314803d874bbe92b400fc8e103f1e484c25bdfa2015464da3d` |
| PnP accelerated exe | `target/dist-accelerated/developer/pnp_cli.exe` | sha256 `490c6dc251eade2f7c42c8973d5ab567234092f1b7d88f021987a20534c3a46a` (exact t172 candidate build) |
| PnP accelerated modules | `target/dist-accelerated/developer/modules` (24 `.wasm`, `module diagnose` pass) | tree sha256 `481ff759cedd02d133f3e9dd78fff036d33ebfa016a16d129c030d436a4ab284` |
| Active currency | [`evidence/t172-offset-concave-crossing/reference.json`](../t172-offset-concave-crossing/reference.json) | matched **16/16** cell × arm |

---

## 2. Primary quiet-box board (`reduced/summary.json`)

Window: `2026-11-21T04:31:35Z` → `05:00:09Z` (`batch_id = t174-quiet-board`).
Protocol: 8 cells × 3 tools × (1 warmup + 3 interleaved measured runs) = **96
runs** (72 measured), `RAYON_NUM_THREADS=12`, Latin rotation across rounds.
Box state (`reduced/load-samples.csv`, 85 samples @ 20 s): CPU load min
**14.0 %** / median **79.0 %** / max **100.0 %** (matching t163's 76 % active-run
median); free RAM steady at **10,815.4–14,069.5 MB** (median **12,647.5 MB**).
Starvation flags (`cpu/wall < 0.75 × group best`): **0 of 72**.

### Wall-clock medians, spreads, ratios of record, and required cuts

| Cell | Orca med (spread) | PnP ord med (spread) | PnP acc med (spread) | Ord / Orca | Ord req. cut | Acc / Orca | Acc req. cut | t172 est. (ord) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | 2.0557 s (3.56 %) | 2.3513 s (3.53 %) | 2.2981 s (6.02 %) | **1.1438x** | 12.57 % | **1.1179x** | 10.55 % | 1.26x |
| `benchy-classic-on` | 2.7070 s (2.57 %) | 5.0633 s (7.87 %) | 5.1872 s (5.12 %) | **1.8704x** | **46.54 %** | **1.9162x** | **47.81 %** | 1.94x |
| `benchy-arachne-off` | 2.3478 s (9.34 %) | 2.7112 s (11.81 %) | 2.4980 s (7.73 %) | **1.1548x** | 13.40 % | **1.0640x** | 6.01 % | 1.44x |
| `benchy-arachne-on` | 3.1660 s (2.28 %) | 5.2720 s (3.09 %) | 5.3007 s (12.58 %) | **1.6652x** | 39.95 % | **1.6743x** | 40.27 % | 1.99x |
| **`base-classic-off`** | 17.6898 s (2.06 %) | **13.5988 s** (5.02 %) | **13.5427 s** (8.40 %) | **0.7687x** | −30.08 % (win) | **0.7656x** | −30.62 % (win) | 0.82x |
| `base-classic-on` | 30.7617 s (5.01 %) | 54.6721 s (2.73 %) | 52.4219 s (3.97 %) | **1.7773x** | 43.73 % | **1.7041x** | 41.32 % | 1.80x |
| **`base-arachne-off`** | 18.7741 s (5.67 %) | **14.8718 s** (6.23 %) | **14.5296 s** (11.08 %) | **0.7921x** | −26.24 % (win) | **0.7739x** | −29.21 % (win) | 0.89x |
| `base-arachne-on` | 30.1595 s (9.35 %) | 52.4232 s (4.90 %) | 50.7964 s (2.16 %) | **1.7382x** | 42.47 % | **1.6843x** | 40.63 % | 1.73x |

*(Required cut = `100 × (1 − 1 / ratio)`; negative values on the two winning cells
indicate how much slower PnP would have to get to fall back to `1.0x`, while
actual wall-time reductions vs Orca are **−23.13 % / −23.44 %** on
`base-classic-off` and **−20.79 % / −22.61 %** on `base-arachne-off`.)*

### Process CPU medians, effective parallelism (`cpu/wall`), and accelerated deltas

| Cell | Orca CPU (eff. thr) | Ord CPU (eff. thr) | Acc CPU (eff. thr) | CPU Ord/Orca | CPU Acc/Orca | Acc vs Ord wall | Acc vs Ord CPU |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | 6.0938 s (3.01x) | 9.0000 s (3.89x) | 9.0469 s (3.86x) | 1.4769x | 1.4846x | −2.26 % | +0.52 % |
| `benchy-classic-on` | 10.9375 s (4.02x) | 21.9062 s (4.22x) | 21.8281 s (4.21x) | 2.0029x | 1.9957x | +2.45 % | −0.36 % |
| `benchy-arachne-off` | 7.9688 s (3.57x) | 10.2031 s (3.81x) | 9.9062 s (3.94x) | 1.2804x | 1.2431x | −7.86 % | −2.91 % |
| `benchy-arachne-on` | 12.4531 s (3.99x) | 23.1406 s (4.35x) | 22.8594 s (4.31x) | 1.8582x | 1.8356x | +0.54 % | −1.22 % |
| `base-classic-off` | 33.3594 s (1.89x) | 70.9219 s (5.24x) | 68.9062 s (5.09x) | **2.1260x** | **2.0656x** | −0.41 % | −2.84 % |
| `base-classic-on` | 102.3594 s (3.38x) | 300.9219 s (5.51x) | 297.6250 s (5.68x) | 2.9399x | 2.9076x | −4.12 % | −1.10 % |
| `base-arachne-off` | 42.1250 s (2.24x) | 77.6719 s (5.18x) | 76.2500 s (5.25x) | **1.8438x** | **1.8101x** | −2.30 % | −1.83 % |
| `base-arachne-on` | 111.9062 s (3.66x) | 304.8750 s (5.81x) | 302.1094 s (5.94x) | 2.7244x | 2.6997x | −3.10 % | −0.91 % |

---

## 3. Destination tally and the wall-vs-CPU contradiction

Both `base` supports-off cells beat OrcaSlicer cleanly on wall clock with
non-overlapping sample ranges across all three measured runs:

- **`base-classic-off`**:
  - Orca wall runs: `[17.4342, 17.6898, 17.7984] s` (min `17.4342 s`)
  - PnP ordinary wall runs: `[13.0880, 13.5988, 13.7703] s` (max `13.7703 s`, **3.664 s** below fastest Orca)
  - PnP accelerated wall runs: `[13.0328, 14.1705, 13.5427] s` (max `14.1705 s`, **3.264 s** below fastest Orca)
- **`base-arachne-off`**:
  - Orca wall runs: `[19.1304, 18.7741, 18.0657] s` (min `18.0657 s`)
  - PnP ordinary wall runs: `[14.8718, 15.6936, 14.7668] s` (max `15.6936 s`, **2.372 s** below fastest Orca)
  - PnP accelerated wall runs: `[15.9156, 14.3057, 14.5296] s` (max `15.9156 s`, **2.150 s** below fastest Orca)

Yet on **process CPU**, OrcaSlicer uses roughly **half** the total CPU-seconds
PnP uses in those two cells (`33.36 s` vs `70.92 / 68.91 s` on
`base-classic-off`; `42.13 s` vs `77.67 / 76.25 s` on `base-arachne-off`).
Why: PnP runs with **5.09–5.25x** effective thread concurrency across the 12
logical cores (`RAYON_NUM_THREADS=12`), whereas OrcaSlicer's default schedule
achieves **1.89x** (`base-classic-off`) and **2.24x** (`base-arachne-off`)
effective concurrency on this fixture. That is a genuine architectural
tradeoff, not measurement noise or starvation: every sample's `cpu/wall` ratio
is tight (spreads 0.97–2.47 % CPU, 2.06–11.08 % wall, zero starvation flags).

This surfaces two contract questions that only the human can rule on:

1. **CPU-corroboration semantics:** `map.md` states *"lower median
   uninstrumented wall-clock (process CPU corroborating, never contradicting)"*,
   and `SCOREBOARD.md` glossed this as *"same ordering, never contradicting"*.
   When PnP beats OrcaSlicer on wall clock by **21–23 %** via higher
   concurrency while spending **1.81–2.13x** as much CPU time, does that close
   the cell (tally → **2/8**), or must PnP also cut another **45–53 %** of CPU
   work on `base` supports-off to satisfy `CPU <= Orca` (tally stays **0/8**)?
2. **Sample count for cell closure:** `SCOREBOARD.md` (§Caveats) notes *"n=3 is
   thin ... any future close cell needs the acceptance protocol (n>=4, strict
   separation) before a win claim."* Here the gap is large (**21–23 %**, strict
   separation on n=3 in the quiet window, and median wall wins even under the
   loaded first window). Does n=3 with 21–23 % strict separation close the two
   cells if Q1 allows parallelism-driven wins, or is a dedicated `n>=4`
   confirmation pass required?

Per Wayfinder discipline, t174 does **not** unilaterally relax the Destination.
The official tally stays **0 of 8** pending the human ruling in
[ticket 176](../../issues/176-destination-cpu-corroboration-ruling.md), while
recording **2 of 8 unstarved wall-clock wins** on both arms.

---

## 4. Accelerated vs ordinary arm

- **Wall:** Accelerated is faster than ordinary in **6 of 8** cells, delta
  range **−7.86 % to +2.45 %**, median **−2.28 %** (mean −2.13 %).
  - Unlike t163 (where accelerated regressed `base-classic-off` by +4.38 % and
    `base-classic-on` by +1.48 %), accelerated is now faster on **all four
    `base` cells** (−0.41 %, −4.12 %, −2.30 %, −3.10 %) and both `benchy`
    supports-off cells (−2.26 %, −7.86 %).
  - The two slower cells are `benchy-classic-on` (+2.45 %) and
    `benchy-arachne-on` (+0.54 %), both well within their own wall spreads
    (5.12–12.58 %).
  - On `benchy-arachne-off`, the −7.86 % accelerated lift pulls PnP to
    **1.0640x** — just **0.150 s** behind OrcaSlicer.
- **Process CPU:** Accelerated is lower in **7 of 8** cells (−2.91 % to
  +0.52 %, median **−1.16 %**), with only `benchy-classic-off` slightly higher
  (+0.52 %, well inside its 2.42–5.90 % CPU spread).
- **Output exactness:** Accelerated and ordinary produce byte-identical G-code
  in **8 of 8** cells (and both match t172's active currency).

---

## 5. Why all four `benchy` cells beat t172's estimates

Every `benchy` cell landed meaningfully faster than t172's cross-window
estimate (`1.144x` vs `1.26x`, `1.870x` vs `1.94x`, `1.155x` vs `1.44x`,
`1.665x` vs `1.99x`), with the largest drops on `benchy-arachne-off` (−0.285x)
and `benchy-arachne-on` (−0.325x). The reason is bookkeeping, not a mystery:
t169's cross-window route estimate composed `t163 × t169` directly, omitting
**t166**'s landed shell-opening parity gains (+20.4 % / +7.9 % / +21.4 % /
+9.3 % on `benchy`), and t172 chained onto t169's estimate. Including t166 in
the mental sanity check predicts ~1.00–1.81x on `benchy` and ~0.79–1.76x on
`base`, aligning closely with the fresh single-window board. This is another
demonstration of t161's rule that chained cross-window estimates drift and
never replace a single-window board.

---

## 6. Disclosed loaded first take (`reduced/loaded-window.json`)

The first 96-row campaign (`batch_id = t174-post-t172-scoreboard`,
`2026-11-21T03:01:35Z` → `03:43:29Z`) completed 96/96 rows with exit 0 and
**16/16** byte-identical PnP outputs, **0** code-1200 diagnostics, and **10/10**
matched keys on every Orca run, but ran concurrently with an external ~10 GB /
high-CPU workload on the workstation. `reduced/loaded-window-load-samples.csv`
captures the external workload ending abruptly between sample 110
(`03:39:21Z`: 87 % CPU, **4,985.6 MB** free RAM) and sample 111 (`03:39:43Z`:
25 % CPU, **15,674.1 MB** free RAM — an instant **+10.44 GB** RAM release),
mid-cell inside `base-arachne-on`.

Consequences in that window (preserving the `t156` disclosure precedent):

- **4 starvation flags** (`cpu/wall < 0.75 × best`): `base-arachne-off` acc m2,
  plus all three tools' m1 runs in `base-arachne-on` (wall spreads **64.18 %**,
  **75.52 %**, **97.82 %** as `m1` ran under load and `m2`/`m3` ran after it
  exited: Orca `59.08 / 29.56 / 30.17 s`, PnP ord `91.51 / 56.10 / 55.50 s`,
  PnP acc `87.98 / 49.20 / 51.35 s`).
- **Asymmetric concurrency penalty:** External CPU contention penalized
  5.2-thread PnP more steeply than 1.9-thread Orca on `base` supports-off,
  compressing the wall win from **0.769x / 0.792x** (quiet) to **0.832x /
  0.863x** (loaded), and shifting `base-classic-on` Orca from `30.8 s` to
  `50.1 s`.
- **What stayed invariant:** Even under heavy contention, both `base`
  supports-off cells won on wall clock on both arms (**0.8322x / 0.8307x** and
  **0.8629x / 0.8529x**), all 8 cells showed `CPU > Orca` (`1.194x–2.810x`
  ord), and **16/16** outputs reproduced `t172`'s currency byte-for-byte.

Per user authorization, the loaded take is preserved as
`reduced/loaded-window.json` + `reduced/loaded-window-load-samples.csv` and
excluded from the ratios of record; `reduced/summary.json` (the quiet window)
is the sole basis for all ratios of record.

| Cell | Quiet Ord/Orca | Loaded Ord/Orca | Quiet Acc/Orca | Loaded Acc/Orca | Loaded starved |
| --- | ---: | ---: | ---: | ---: | --- |
| `benchy-classic-off` | 1.1438x | 1.1304x | 1.1179x | 1.0376x | none |
| `benchy-classic-on` | 1.8704x | 1.8958x | 1.9162x | 1.7175x | none |
| `benchy-arachne-off` | 1.1548x | 1.0077x | 1.0640x | 1.0246x | none |
| `benchy-arachne-on` | 1.6652x | 1.9405x | 1.6743x | 1.5758x | none |
| `base-classic-off` | 0.7687x | 0.8322x | 0.7656x | 0.8307x | none |
| `base-classic-on` | 1.7773x | 1.7047x | 1.7041x | 1.7243x | none |
| `base-arachne-off` | 0.7921x | 0.8629x | 0.7739x | 0.8529x | acc m2 |
| `base-arachne-on` | 1.7382x | 1.8595x | 1.6843x | 1.7022x | orca m1, ord m1, acc m1 |

---

## 7. Route and fog implications

1. **New binding cell:** `benchy-classic-on` (**1.8704x / 46.54 % cut**
   ordinary; **1.9162x / 47.81 % cut** accelerated) replaces t172's estimated
   `benchy-arachne-on` (`1.99x / 49.8 %`, now measured **1.6652x / 39.95 %**
   ordinary, **1.6743x / 40.27 %** accelerated).
2. **`base` supports-on matches t173's budget closely:** `base-classic-on`
   measured **1.7773x / 43.73 % cut** (vs t172's 1.80x / 44.5 %) and
   `base-arachne-on` measured **1.7382x / 42.47 % cut** (vs t172's 1.73x /
   42.1 %). t173's structural conclusion — that the `base` supports-on pair
   needs phase-1 tiling ([ticket 175](../../issues/175-tiled-collision-carve.md))
   plus a phase-3 union companion — stands unchanged.
3. **`benchy` supports-off is near parity:** `benchy-classic-off` is **1.1438x
   / 1.1179x** (+0.30 / +0.24 s) and `benchy-arachne-off` is **1.1548x /
   1.0640x** (+0.36 / +0.15 s).
4. **Below-fold ~1.5x park bound crossed:** All four supports-off cells
   (**0.769x, 0.792x, 1.144x, 1.155x** ordinary; **0.766x, 0.774x, 1.118x,
   1.064x** accelerated) now sit well inside the ~1.5x park release bound
   recorded in `map.md` (`## Not yet specified`).

---

## 8. Disclosures and verification

- **No null band:** Like t161 and t163, this is a three-tool board, not an A/B
  candidate gate; no same-arm null band was collected.
- **OrcaSlicer executable boundary:** Same OrcaSlicer **2.5.1** executable
  (`5013b1bd…`, vendor `02.08.00.16`) as t161 and t163. Orca medians in the
  quiet window sit within −8.0 % to +8.3 % of t163's quiet board across all 8
  cells; no number is composed across windows.
- **No `--profile` pass:** Uninstrumented timing only.
- **Checkout-only verification:**
  - `python docs/specs/perf-vs-orca/evidence/t174-post-t172-scoreboard/verify.py`
    → **`PASS: 1280/1280 checks`** (re-derives every median, spread, wall/CPU
    ratio, required cut, win/corroboration/strict-separation flag, starvation
    index, hash/byte identity against `t172/reference.json`, code-1200 count,
    10 matched Orca keys, and load-CSV stats for both `summary.json` and
    `loaded-window.json`).
  - `python docs/specs/perf-vs-orca/evidence/t174-post-t172-scoreboard/verify.py --self-test`
    → **`self-test: 18/18 mutations rejected`**.
