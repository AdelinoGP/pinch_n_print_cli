# Post-t172 eight-cell two-tool scoreboard refresh (verify the two estimated base supports-off wins)

Type: task / measured board (two-tool OrcaSlicer 2.5.1 vs PnP ordinary + accelerated, no production change)
Label: wayfinder:task
Status: resolved (2026-11-21; measured three-tool eight-cell board on a quiet box
  — **0.7687x–1.8704x** ordinary / **0.7656x–1.9162x** accelerated, **16/16**
  byte-identical to t172's currency, **2 of 8** decisive unstarved wall-clock
  wins with strict separation on both arms (`base-classic-off` 0.769x / 0.766x,
  `base-arachne-off` 0.792x / 0.774x), **0 of 8** under literal `CPU <= Orca`
  because PnP burns 1.81x–2.13x Orca's CPU via ~5.2x vs ~1.9–2.2x thread
  concurrency. Official Destination tally held at **0/8** pending human ruling
  in graduated HITL [176](176-destination-cpu-corroboration-ruling.md);
  `benchy-classic-on` becomes the new binding cell at **1.8704x / 46.54 %**
  ordinary (**1.9162x / 47.81 %** accelerated). Loaded first take disclosed and
  preserved as `reduced/loaded-window.json`.)
Assignee: 2026-11-20 Antigravity wayfinder session — the human said "Continue the
  perf vs orca map" (no ticket named); 174 was first on the frontier (leaving
  175 unclaimed).
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [t172 OffsetConcaveCrossing port](172-port-offset-concave-crossing.md)
([`evidence/t172-offset-concave-crossing/FINDINGS.md`](../evidence/t172-offset-concave-crossing/FINDINGS.md)),
which landed +14.89–23.01 % wall across all eight cells on top of t169's +5.74–25.08 %,
putting `base classic-off` at an estimated **~0.82x** and `base arachne-off` at an
estimated **~0.89x** against OrcaSlicer (cross-window composition from t163's board).

## Question

**On a fresh, same-window eight-cell matched-pair board against the installed
OrcaSlicer 2.5.1, do `base classic-off` and `base arachne-off` officially move
the map's destination tally from 0/8 to 2/8 cells won (lower median
uninstrumented wall with process CPU corroborating), and what are the exact
ratios of record and remaining required cuts for all eight cells after t169 and
t172?**

## Why this ticket exists now

- Every ratio quoted since t163 (`1.04x`, `1.12x`, now `~0.82x`, `~0.89x`, and
  the six remaining `1.26x–1.99x` cuts) is a **two-step cross-window estimate**
  (`t163` board × `t169` paired saving × `t172` paired saving). Standing rule
  (t161): a cross-window composition is never a ratio of record and cannot close
  a destination cell.
- Measuring a single-window 8-cell × {OrcaSlicer 2.5.1, PnP ordinary, PnP
  accelerated} board (1 warmup + 3 measured runs/cell, matched keys asserted per
  Orca run, Orca binary sha256 stamped, PnP output verified 16/16 against t172's
  `reference.json`) establishes the official win count (`2/8` if the estimate
  holds) and replaces the two-step estimated cuts that t173 and the benchy route
  depend on.

## Scope

1. Verify `cargo xtask build-guests --check` and `--accelerated --check` both
   exit 0, and stamp the installed OrcaSlicer executable sha256/version + both
   PnP executable/module-tree hashes.
2. Run one interleaved 8-cell × {OrcaSlicer 2.5.1, PnP ordinary, PnP
   accelerated} board (1 warmup + 3 measured runs/cell, `RAYON_NUM_THREADS=12`),
   asserting all matched keys on every Orca run and verifying all 16 PnP outputs
   reproduce t172's active `reference.json` byte-for-byte with `degraded: false`
   and zero code-1200 diagnostics.
3. Reduce wall/CPU medians, ratios of record, required cuts, and the official
   destination tally (`N/8` won). Update the map's destination status and
   remaining cuts.

## Acceptance

- No production code changes; all PnP runs reproduce t172's `reference.json`
  byte-for-byte (`16/16`).
- `evidence/t174-post-t172-scoreboard/` with reduced board JSONs, `FINDINGS.md`,
  and checkout-only `verify.py --self-test`.
- Map: destination-status block with official ratios of record and win count,
  Decisions-so-far line, and updated remaining cuts for t173 / benchy fog.

## Answer

**Measured (2026-11-21, zero production code changes).** Both guest freshness
gates exited 0 (`cargo xtask build-guests --check` and `--accelerated --check`),
and all tool identities were stamped at HEAD `0ce4be39` (last code commit
`48a5e583`, t172): OrcaSlicer **2.5.1** (`5013b1bd…`, 292,160 B, vendor
`02.08.00.16`, `support_style = tree_strong`), PnP ordinary `06b05b6b…` +
24-module tree `e75be5c0…`, and PnP accelerated `490c6dc2…` (exact t172
candidate build) + 24-module tree `481ff759…`.

### Official quiet-box board (`reduced/summary.json`, `batch_id = t174-quiet-board`)

8 cells × 3 tools × (1 warmup + 3 interleaved measured runs) = **96 rows** (72
measured), `RAYON_NUM_THREADS=12`, **0/72 starvation flags**, steady
10.8–14.1 GB free RAM (85 samples @ 20 s; CPU load min 14.0 % / median 79.0 % /
max 100.0 %). **16/16** cell × arm PnP outputs (all 64 runs) match t172's
active `reference.json` byte-for-byte (`exit 0`, `degraded = false`, `fatal = 0`,
`non_fatal = 0`, **code-1200 = 0**); all **10 matched keys** asserted on all
32/32 Orca runs.

| Cell | Orca med | Ord med | Acc med | Ord / Orca (cut) | Acc / Orca (cut) | CPU Ord / Acc vs Orca | Acc vs Ord wall / CPU |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `benchy-classic-off` | 2.0557 s | 2.3513 s | 2.2981 s | **1.1438x** (12.57 %) | **1.1179x** (10.55 %) | 1.477x / 1.485x | −2.26 % / +0.52 % |
| `benchy-classic-on` | 2.7070 s | 5.0633 s | 5.1872 s | **1.8704x** (**46.54 %**) | **1.9162x** (**47.81 %**) | 2.003x / 1.996x | +2.45 % / −0.36 % |
| `benchy-arachne-off` | 2.3478 s | 2.7112 s | 2.4980 s | **1.1548x** (13.40 %) | **1.0640x** (6.01 %) | 1.280x / 1.243x | −7.86 % / −2.91 % |
| `benchy-arachne-on` | 3.1660 s | 5.2720 s | 5.3007 s | **1.6652x** (39.95 %) | **1.6743x** (40.27 %) | 1.858x / 1.836x | +0.54 % / −1.22 % |
| **`base-classic-off`** | 17.6898 s | **13.5988 s** | **13.5427 s** | **0.7687x** (−23.13 % wall) | **0.7656x** (−23.44 % wall) | **2.126x / 2.066x** | −0.41 % / −2.84 % |
| `base-classic-on` | 30.7617 s | 54.6721 s | 52.4219 s | **1.7773x** (43.73 %) | **1.7041x** (41.32 %) | 2.940x / 2.908x | −4.12 % / −1.10 % |
| **`base-arachne-off`** | 18.7741 s | **14.8718 s** | **14.5296 s** | **0.7921x** (−20.79 % wall) | **0.7739x** (−22.61 % wall) | **1.844x / 1.810x** | −2.30 % / −1.83 % |
| `base-arachne-on` | 30.1595 s | 52.4232 s | 50.7964 s | **1.7382x** (42.47 %) | **1.6843x** (40.63 %) | 2.724x / 2.700x | −3.10 % / −0.91 % |

### Key answers

1. **Destination win tally — 2/8 wall-clock wins, 0/8 under literal `CPU <= Orca`:**
   - `base-classic-off` (`13.09–13.77 s` ord / `13.03–14.17 s` acc vs `17.43–17.80 s` Orca)
     and `base-arachne-off` (`14.77–15.69 s` ord / `14.31–15.92 s` acc vs `18.07–19.13 s`
     Orca) beat OrcaSlicer on wall clock by **21–23 %** with **strict separation**
     on both arms and zero starvation flags — beating t172's `0.82x / 0.89x` estimates.
   - However, PnP uses **2.126x / 2.066x** Orca's process CPU on `base-classic-off`
     and **1.844x / 1.810x** on `base-arachne-off` (effective thread concurrency
     **5.1–5.2x** vs Orca's **1.9–2.2x**). Under the literal Destination text
     (*"process CPU corroborating, never contradicting"*) and `SCOREBOARD.md`'s
     *"same ordering"* / *"n>=4, strict separation"* clauses, neither cell is
     closed unilaterally. The official tally stays **0/8** pending the human
     ruling graduated to [176](176-destination-cpu-corroboration-ruling.md).
2. **Route & remaining cuts:**
   - **Binding cell shifts** to `benchy-classic-on` (**1.8704x / 46.54 %** ord,
     **1.9162x / 47.81 %** acc), replacing t172's estimated `benchy-arachne-on`
     (now **1.6652x / 39.95 %** ord, **1.6743x / 40.27 %** acc). All four `benchy`
     cells beat t172's estimates because t169's cross-window chain had omitted
     t166's landed benchy shell-opening gains.
   - `base` supports-on confirms t173's cuts (**43.73 % / 42.47 %** ord vs
     44.5 % / 42.1 % estimated), leaving [175](175-tiled-collision-carve.md)'s
     scope unchanged.
   - All four supports-off cells (**0.769x–1.155x** ord, **0.766x–1.118x** acc;
     `benchy-arachne-off` accelerated just **0.150 s** behind Orca at **1.064x**)
     cross the below-fold **~1.5x** park release bound in `map.md`.
   - Accelerated beats ordinary in **6/8** wall (**−7.86 % to +2.45 %**, median
     **−2.28 %**) and **7/8** CPU (**−2.91 % to +0.52 %**, median **−1.16 %**),
     reversing t163's +4.4 % `base-classic-off` regression.
3. **Loaded first take disclosed (`reduced/loaded-window.json`):** An initial
   96-row campaign (`t174-post-t172-scoreboard`) ran under an external ~10 GB /
   high-CPU workload that exited mid-`base-arachne-on` (`03:39:21Z → 03:39:43Z`,
   +10.44 GB free RAM jump), triggering 4 starvation flags and 64–98 % wall
   spreads. Per `t156` precedent and user approval, it is preserved in
   `reduced/loaded-window.json` + `reduced/loaded-window-load-samples.csv` and
   excluded from the ratios of record (it still independently confirmed 16/16
   identity and 2/8 wall wins at `0.832x / 0.863x`).

Evidence: [`evidence/t174-post-t172-scoreboard/FINDINGS.md`](../evidence/t174-post-t172-scoreboard/FINDINGS.md)
(`verify.py` **PASS: 1280/1280 checks**; `verify.py --self-test` **18/18
mutations rejected**). Graduated [176](176-destination-cpu-corroboration-ruling.md).
