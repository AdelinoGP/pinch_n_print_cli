# Destination ruling: CPU-corroboration semantics and sample count for cell closure

Type: grilling
Label: wayfinder:grilling
Status: resolved (2026-11-21; human ruling — Option C: amend `## Destination` to
  unstarved median wall-clock win with CPU integrity guard + separate efficiency
  disclosure, keeping candidate A/B CPU corroboration unchanged; `pnp-ordinary`
  binds cell closure; margin-tiered provisional closure (`n=3` strict separation
  with 0 starvation at `ratio <= 0.85x`, `>= 4` retained unstarved strict
  separation for close cells and final single-HEAD 8/8 closing board, allowing
  starved-outlier exclusion when `>= 4` unstarved runs remain); provisional wins
  carry forward unless regressed; tally moves **0/8 → 2/8 provisionally won**
  (`base-classic-off`, `base-arachne-off`); Destination stays `< 1.0x` (8/8)
  with a fresh `<= 0.50x` effort planned afterward; graduated `177`, `178`,
  `179`, `180`. See ## Answer)
Assignee: dev (2026-11-21 — user invoked `/wayfinder Continue the perf vs orca map`; 176 was the sole open frontier ticket and resolved in-session)
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

**Resolved 2026-11-21 (HITL grilling, 8 decisions Q1–Q8, tracker & glossary only
— zero production code edits). The Destination wording is explicitly amended to
separate cross-engine wall-clock closure from per-core CPU efficiency, the
running tally moves from `0/8 → 2/8` provisionally won (`base-classic-off` and
`base-arachne-off`), the Destination stays `< 1.0x` in 8 of 8 cells with a
follow-on `<= 0.50x` effort afterward, and four structured AFK speedup-discovery
tickets graduate onto the frontier.**

### 1. Cross-engine CPU semantics (Q1 → Option C)

- **Why the contradiction happens physically** (verified from
  [t110](110-orca-side-attribution.md), [t165](165-canonical-prepass-cost.md),
  and [t174](174-post-t172-scoreboard-refresh.md)): OrcaSlicer 2.5.1 is *not*
  single-threaded during slicing (its 2.17 s `base-off` prepass burns 14.03 s
  CPU ≈ **6.46x** effective threads), but it spends **~10.04 s** in serial
  startup + 123 MB STL import (`--info`), pulling its whole-process average down
  to **1.89x / 2.24x** threads (`33.36 s / 42.13 s` CPU over `17.69 s / 18.77 s`
  wall). PnP loads in ≤ 0.71 s and slices across **5.18–5.24x** effective
  threads (`70.92 s / 77.67 s` CPU over `13.60 s / 14.87 s` wall). All six
  remaining cells also carry `CPU_pnp > CPU_orca` (`1.24x–2.94x`).
- **Ruling:** Explicitly amend `## Destination` in [`map.md`](../map.md) (not a
  silent re-read) so that a **Destination cell win** is decided by **lower
  median uninstrumented wall clock** on the 12-core dev rig under matched
  settings, guarded by the **CPU integrity guard** (tight process CPU, unstarved
  `cpu/wall`, matched non-degraded output) while disclosing the **PnP/Orca CPU
  ratio beside every cell as a separate efficiency metric**.
- **Unchanged:** Within-PnP **candidate A/B CPU corroboration** (candidate vs
  baseline on the same engine) is untouched.

### 2. Binding PnP arm (Q2 → Option 1)

- **`pnp-ordinary` (the current production default)** is the **binding arm**
  that closes a Destination cell.
- `pnp-accelerated` continues to be measured and disclosed beside it as
  corroboration, and becomes the binding arm only if accelerated mode is
  formally adopted as the production default.

### 3. Sample count, starvation handling, and provisional vs final closure (Q3, Q4, Q5)

- **Wide-margin provisional cell win (`ratio <= 0.85x`, i.e., ≥ 15 % faster):**
  Quiet-window `n = 3` strict separation with **0 starvation flags**
  (`cpu/wall >= 0.75 × group best`) on both arms closes the cell provisionally.
- **Close-cell provisional cell win (`0.85x < ratio < 1.00x`):** Requires
  **`>= 4` retained unstarved runs** per arm with strict separation. Starved
  outliers (`cpu/wall < 0.75 × group best`) may be excluded per
  [`SCOREBOARD.md`](../evidence/matched-pair/SCOREBOARD.md) §Protocol as long
  as `>= 4` retained unstarved runs per arm remain and strictly separate.
- **Carry-forward rule (Q5):** A **provisional cell win** stays counted in the
  running tally across later commits that pass their own paired non-regression
  gate (and stays re-verified on the final single-HEAD closing board); only a
  later Orca board showing `>= 1.0x` on the binding arm or an unwaived
  regression re-opens it.
- **Final 8-of-8 map closure:** Closing the map requires **one final
  single-HEAD `n >= 4` 8-of-8 board** satisfying the `>= 4` retained unstarved
  strict-separation rule in every cell on the binding arm.

### 4. Immediate Destination tally update (`0/8 → 2/8` provisionally won)

From [t174](174-post-t172-scoreboard-refresh.md)'s quiet-window board (with
[t175](175-tiled-collision-carve.md) confirmed structurally inert and
byte-identical on both supports-off cells):

| Cell | Binding arm (`pnp-ordinary`) wall vs Orca | Ratio | Strict separation | Starvation | CPU efficiency disclosure (`ord / acc`) | Status |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `base-classic-off` | `13.599 s` `[13.09, 13.60, 13.77]` vs `17.690 s` `[17.43, 17.69, 17.80]` | **0.7687x** (`<= 0.85x`) | **3.664 s** (3/3) | 0 / 6 | `2.126x / 2.066x` | **Provisionally won** |
| `base-arachne-off` | `14.872 s` `[14.77, 14.87, 15.69]` vs `18.774 s` `[18.07, 18.77, 19.13]` | **0.7921x** (`<= 0.85x`) | **2.372 s** (3/3) | 0 / 6 | `1.844x / 1.810x` | **Provisionally won** |

*(Both cells also won on `pnp-accelerated` at `0.7656x` and `0.7739x`, and won
median wall in the disclosed loaded-box window at `0.832x / 0.863x`.)*

### 5. Destination bar & `2×` faster (`<= 0.50x`) horizon (Q7)

- Keep this map's Destination at **`< 1.0x` (beating OrcaSlicer in 8 of 8
  cells)** as a closeable milestone, and spin a **fresh `<= 0.50x` (2× faster
  than OrcaSlicer)** map afterward once 8/8 closes — while prioritizing
  high-leverage speedup tracks now that push cells well past `1.0x` toward
  `0.50x`.

### 6. Domain model updates (`GLOSSARY.md`)

Added four canonical terms to [`GLOSSARY.md`](../../../../GLOSSARY.md)
(skipping an ADR — effort governance, not a codebase architecture decision):
**CPU integrity guard**, **Candidate CPU corroboration**, **Binding arm**, and
**Provisional cell win**.

### 7. Graduated frontier (Q8 — 4 structured AFK tickets)

1. [Post-t175 full-matrix attribution and 0.50x headroom map](177-post-t175-full-matrix-attribution.md) *(unblocked)* — refresh 8-cell phase/stage/host-op/guest-fuel shares at post-t172+t175 HEAD and price both `< 1.0x` (binding `benchy-classic-on` ~1.57x, `benchy` supports-off 1.14–1.15x) and `<= 0.50x` ceilings.
2. [Planner Phase-3 batched union attribution and mechanism screen](178-planner-phase3-union-attribution.md) *(unblocked)* — attribute `build_roles_batched` Phase 3 (`51.6 s` thread / `5.21 s` wall, ~92 % of remaining `host:native` clip thread on `base` supports-on) and screen concrete mechanisms (e.g., bbox-connected component partitioning + singleton bypass) to push `base` supports-on (`~1.20–1.22x`) across `1.0x` toward 4/8.
3. [Per-layer `tree-support` renderer and `infill-linker` guest-fuel screen](179-per-layer-tree-support-and-infill-linker-screen.md) *(blocked by `177`)* — attribute and price concrete per-layer guest speedups using `177`'s verified post-t175 shares.
4. [`ShellClassification` post-t172 residual attribution and speedup screen](180-shell-classification-post-t172-residual-screen.md) *(blocked by `177`)* — attribute and price concrete work-set/parallelism speedups in `ShellClassification`'s surviving post-t172 cost using `177`'s verified post-t175 shares.
