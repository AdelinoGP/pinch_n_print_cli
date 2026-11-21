# Destination ruling: CPU-corroboration semantics and sample count for cell closure

Type: grilling
Label: wayfinder:grilling
Status: open
Assignee: unclaimed
Parent: [Slice performance vs OrcaSlicer](../map.md)
Blocked by: none
Graduated from: [Post-t172 eight-cell two-tool scoreboard refresh](174-post-t172-scoreboard-refresh.md)
([`evidence/t174-post-t172-scoreboard/FINDINGS.md`](../evidence/t174-post-t172-scoreboard/FINDINGS.md)),
where both `base` supports-off cells beat OrcaSlicer 2.5.1 decisively on
unstarved wall clock (**0.7687x / 0.7656x** and **0.7921x / 0.7739x**) with
strict separation on all three measured runs, while spending **1.81x–2.13x**
OrcaSlicer's process CPU via higher thread concurrency (**5.1–5.2x** vs
OrcaSlicer's **1.9–2.2x**).

## Question

**Now that PnP beats OrcaSlicer on unstarved wall clock by 21–23 % in
`base-classic-off` and `base-arachne-off` via higher parallelism while burning
~2x OrcaSlicer's process CPU, what closes a destination cell?**

Specifically, two decisions are required:

1. **Q1 — Meaning of *"process CPU corroborating, never contradicting"*:**
   - **Option A (wall win via unstarved parallelism closes the cell):** Interpret
     CPU corroboration as an **artifact / starvation guard** (CPU is measured,
     tight, and unstarved — confirming the wall win is real computation rather
     than skipped work or deschedule noise — without requiring `CPU_pnp <=
     CPU_orca` when PnP achieves higher concurrency on the same 12-core box).
     Under Option A, `base-classic-off` and `base-arachne-off` move the
     Destination tally from **0/8 → 2/8** (subject to Q2).
   - **Option B (strict `CPU_pnp <= CPU_orca` ordering required):** Keep
     `SCOREBOARD.md`'s literal *"same ordering, never contradicting"* reading:
     a cell is not won until PnP beats OrcaSlicer on **both** median wall clock
     **and** median process CPU. Under Option B, the tally stays **0/8**, and
     even `base-classic-off` (0.769x wall) and `base-arachne-off` (0.792x wall)
     require another **~52–53 %** and **~45–46 %** CPU reduction before closing.
   - **Option C (wall closes the cell, CPU tracked as a separate efficiency
     metric / disclosure):** Same tally effect as Option A (**2/8**), while
     explicitly amending the Destination wording in `map.md` to separate
     wall-clock parity from per-core CPU efficiency.

2. **Q2 — Sample count (`n=3` vs `n>=4`) for closing a cell:**
   - `SCOREBOARD.md` (§Caveats) states: *"n=3 is thin ... any future close cell
     needs the acceptance protocol (n>=4, strict separation) before a win
     claim."*
   - In t174's quiet window (`n=3`), both `base` supports-off cells are not
     close — they win by **21–23 %** with **2.15–3.66 s** strict separation
     between the slowest PnP run and fastest Orca run, and also won on median
     wall in the separate loaded-box window (**0.832x / 0.863x**).
   - **Option 1:** Accept t174's quiet-window `n=3` strict separation as
     sufficient when the margin is unambiguous (≥ 15–20 %), closing those two
     cells immediately if Q1 allows parallelism-driven wall wins (reserving
     `n>=4` for genuinely close cells like `benchy-arachne-off` at `1.064x`).
   - **Option 2:** Require a dedicated `n>=4` acceptance confirmation on every
     winning cell before marking it closed on the map regardless of margin.

## Facts going in (verified from [`evidence/t174-post-t172-scoreboard/reduced/summary.json`](../evidence/t174-post-t172-scoreboard/reduced/summary.json))

| Cell | Arm | Wall med (all 3 runs) | Orca wall med (all 3 runs) | Wall ratio | CPU med | Orca CPU med | CPU ratio | Eff. threads (PnP vs Orca) |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- |
| `base-classic-off` | ordinary | **13.599 s** `[13.09, 13.60, 13.77]` | 17.690 s `[17.43, 17.69, 17.80]` | **0.7687x** | 70.922 s | 33.359 s | **2.1260x** | 5.24x vs 1.89x |
| `base-classic-off` | accelerated | **13.543 s** `[13.03, 14.17, 13.54]` | 17.690 s `[17.43, 17.69, 17.80]` | **0.7656x** | 68.906 s | 33.359 s | **2.0656x** | 5.09x vs 1.89x |
| `base-arachne-off` | ordinary | **14.872 s** `[14.87, 15.69, 14.77]` | 18.774 s `[19.13, 18.77, 18.07]` | **0.7921x** | 77.672 s | 42.125 s | **1.8438x** | 5.18x vs 2.24x |
| `base-arachne-off` | accelerated | **14.530 s** `[15.92, 14.31, 14.53]` | 18.774 s `[19.13, 18.77, 18.07]` | **0.7739x** | 76.250 s | 42.125 s | **1.8101x** | 5.25x vs 2.24x |

- **Output correctness:** 16/16 cell × arm PnP outputs match t172's active
  `reference.json` byte-for-byte, `degraded = false`, 0 code-1200 diagnostics,
  10/10 matched Orca keys asserted on every run.
- **Zero starvation:** 0/72 quiet-window runs tripped the `0.75 × best` cpu/wall
  rule; steady 10.8–14.1 GB free RAM throughout.
- **Why CPU is still higher:** PnP's `base` supports-off slice still does ~2x
  Orca's total CPU work across its prepass + per-layer stages, but distributes
  it across ~5.2 effective cores (`RAYON_NUM_THREADS=12`) vs Orca's ~1.9–2.2
  effective cores. Under the other six cells, `CPU Ord/Orca` is `1.28x–2.94x` —
  meaning the same question will recur on `benchy-classic-off` (`1.144x` wall,
  `1.477x` CPU) and `benchy-arachne-off` (`1.155x / 1.064x` wall, `1.280x /
  1.243x` CPU) as soon as those cross `1.0x` wall.

## Answer

_(pending human ruling)_
