# Orca re-baseline scoreboard — 2026-10-31 (both sides today-vs-today)

Route: [ticket 103](../../issues/103-orca-rebaseline-repaired-rig.md). Measured
at HEAD `c135e3e7` (the tree carries only this take's rig + evidence edits),
with **both** standing trees rebuilt from that HEAD after the ticket-102
mimalloc KEEP:

| arm | binary | SHA-256 | build |
| --- | --- | --- | --- |
| ordinary | `target/release/pnp_cli.exe` | `4cb969421ca1…` | `cargo xtask dist --edition developer` |
| accelerated | `target/dist-accelerated/developer/pnp_cli.exe` | `80897e627d21…` | `cargo xtask dist --accelerated --edition developer` |

Both trees report `module diagnose` `pass=true`, 24/24 modules, provenance
`external`; both guest freshness gates exit 0
(`build-guests --check`, `build-guests --accelerated --check`).

OrcaSlicer is the **2.5.0** install (upstream tag `v2.5.0-beta`) served from the
version-gated vendor tree: installed cache stamp `2.8.0.16` == upstream
`BBL.json` version `02.08.00.16`, so the rig measures the vendor content the
installed slicer itself resolves. Every Orca run in this campaign asserts, in
its own resolved-config appendix, the ruling's three pins **and** the filament
leg — all 32 runs (8 cells x 1 warmup + 3 measured) resolved
`enable_arc_fitting = disabled`, `elefant_foot_compensation = 0`,
`brim_width = 0`, `filament_density = 1.24`,
`filament_max_volumetric_speed = 12`, `first_layer_temperature = 220`. The
verifier re-checks those values per row, so a future install or preset bump
fails the scoreboard instead of drifting silently.

## Headline: PnP still loses all 8 cells, at 3.52x–8.25x

Median uninstrumented wall (s), 1 warmup + 3 measured per group, tools
alternated within each round, 12 threads, starvation rule applied per group.
**Floor multiples** = the fraction of PnP wall that must disappear to reach the
park's ~1.5x condition, in units of the map's **0.80 %** small-effect floor.

| Cell | Orca | PnP ordinary | PnP accelerated | gap ord | gap acc | reduction needed to park | floor multiples |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.03 | 7.68 | 7.67 | **3.78x** | **3.78x** | 60.3 % | 75x |
| benchy arachne-off | 2.19 | 7.69 | 7.84 | **3.52x** | **3.59x** | 57.4 % | 72x |
| benchy classic-on | 2.29 | 12.40 | 12.57 | **5.43x** | **5.50x** | 72.4 % | 90x |
| benchy arachne-on | 2.55 | 12.22 | 12.40 | **4.78x** | **4.85x** | 68.7 % | 86x |
| base classic-off | 15.28 | 75.38 | 75.74 | **4.93x** | **4.96x** | 69.6 % | 87x |
| base arachne-off | 16.72 | 78.89 | 76.51 | **4.72x** | **4.58x** | 68.2 % | 85x |
| base classic-on | 19.49 | 160.86 | 160.09 | **8.25x** | **8.21x** | 81.8 % | 102x |
| base arachne-on | 23.03 | 161.14 | 173.47 | **7.00x** | **7.53x** | 78.6 % | 98x |

**Destination status: Orca still wins all 8 cells.** Best cell **benchy
arachne-off 3.52x ordinary / 3.59x accelerated**; worst **base classic-on
8.25x ordinary / 8.21x accelerated**. No cell is within 2.3x of the park, and
every cell would need a **57–82 %** wall cut — the equivalent of **72–102**
wins at the map's whole 0.80 % floor — so the park condition stays binding and
no single in-map candidate class reaches it.

**Supports-on is still the concentration**, now with the supports-off baseline
also re-measured on the same rig:

| fixture / generator | off | on | on/off |
| --- | ---: | ---: | ---: |
| benchy classic | 3.78x | 5.43x | +44 % |
| benchy arachne | 3.52x | 4.78x | +36 % |
| base classic | 4.93x | 8.25x | +67 % |
| base arachne | 4.72x | 7.00x | +48 % |

## The adhesion asymmetry, beside the numbers

PnP prints an **8 mm brim** in every cell; pinned-back Orca prints **none**. It
is the skirt-brim module's own default (`modules/core-modules/skirt-brim/skirt-brim.toml`
declares `brim_width` default `8.0`), the matched config authors no brim key, and
the emitted PnP appendix resolves `brim_width = 8` in **all 16 PnP groups**
(the verifier asserts it). Pinned-back Orca resolves `brim_width = 0` in all 32
Orca runs. So the two arms are not printing the same adhesion geometry, and this
scoreboard's ratios carry that difference. Its slice-time price is small —
`com.core.skirt-brim` was measured at **235 ms** for the whole print in
ticket 103's ruling — but the job is not identical, and the disclosure belongs
next to the numbers rather than in a footnote.

This is a **separate key from the filament leg** ticket 98 repaired: the
filament leg changed which config chain resolves; this changes who prints
adhesion geometry. Do not fold the two into one story.

## The archived table is superseded — and the direction was not uniform

Archived numbers re-derived from the committed 2026-10-25 ledger
([`../scoreboard-refresh-20261025/`](../scoreboard-refresh-20261025/SCOREBOARD.md))
with the same protocol rules. **Every archived Orca median is superseded at
this HEAD** (the rig, the vendor resolution and the pins all changed).

| Cell | Orca arch | Orca fresh | Orca Δ | PnP ord arch | PnP ord fresh | ord Δ | PnP acc arch | PnP acc fresh | acc Δ |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 2.79 | 2.03 | **−27.2 %** | 7.68 | 7.68 | +0.1 % | 7.56 | 7.67 | +1.4 % |
| benchy arachne-off | 2.88 | 2.19 | **−24.0 %** | 7.90 | 7.69 | −2.7 % | 7.89 | 7.84 | −0.6 % |
| benchy classic-on | 3.53 | 2.29 | **−35.2 %** | 12.30 | 12.40 | +0.8 % | 12.33 | 12.57 | +1.9 % |
| benchy arachne-on | 3.64 | 2.55 | **−30.0 %** | 12.58 | 12.22 | −2.9 % | 12.47 | 12.40 | −0.5 % |
| base classic-off | 13.87 | 15.28 | **+10.2 %** | 73.17 | 75.38 | +3.0 % | 72.92 | 75.74 | +3.9 % |
| base arachne-off | 14.14 | 16.72 | **+18.3 %** | 73.25 | 78.89 | +7.7 % | 73.23 | 76.51 | +4.5 % |
| base classic-on | 18.37 | 19.49 | **+6.1 %** | 171.07 | 160.86 | −6.0 % | 171.71 | 160.09 | −6.8 % |
| base arachne-on | 19.93 | 23.03 | **+15.6 %** | 186.63 | 161.14 | −13.7 % | 183.34 | 173.47 | −5.4 % |

Read the columns that matter: the **fresh** gap table above is the headline; the
archive gaps (2.71x–9.36x) are **superseded** and are not comparable.

**Ticket 98's inference is falsified for four of the eight cells.** It measured
one benchy cell, found Orca ~29 % faster under the repaired filament chain, and
recorded "the archived Orca did *less* work, so every recorded multiple is
optimistic for PnP". Fresh measurement agrees on **benchy** (Orca −24…−35 %,
PnP's multiples worsen: best cell 2.71x → 3.52x) but **reverses on base** (Orca
**+6…+18 %**, PnP's multiples improve as a ratio: base classic-on 9.31x → 8.25x,
base arachne-on 9.36x → 7.00x). The benchy extrapolation did not hold across the
matrix; only measurement could settle it. **No mechanism is asserted here** for
why base Orca got slower — the scoreboard records what both arms did.

The PnP side also carries the ticket-102 allocator KEEP (mimalloc), which is
why base classic-on's PnP wall falls 6.0 % while the benchy cells are flat:
the allocator's measured four-cell effect was 4.8–6.2 % on supports-on base.

## CPU corroboration (median, s) and work density

| Cell | Orca | PnP ord | PnP acc | cpu/wall ord | cpu/wall acc | PnP/Orca cpu |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| benchy classic-off | 6.56 | 48.38 | 48.08 | 6.30 | 6.27 | 7.4x / 7.3x |
| benchy arachne-off | 7.70 | 48.97 | 49.22 | 6.37 | 6.28 | 6.4x / 6.4x |
| benchy classic-on | 8.77 | 64.25 | 64.08 | 5.18 | 5.10 | 7.3x / 7.3x |
| benchy arachne-on | 10.20 | 65.95 | 65.84 | 5.40 | 5.31 | 6.5x / 6.5x |
| base classic-off | 30.78 | 710.34 | 708.72 | 9.42 | 9.36 | 23.1x / 23.0x |
| base arachne-off | 40.11 | 719.66 | 714.16 | 9.12 | 9.33 | 17.9x / 17.8x |
| base classic-on | 49.84 | 1033.33 | 1028.75 | 6.42 | 6.43 | 20.7x / 20.6x |
| base arachne-on | 59.15 | 1035.36 | 1035.14 | 6.43 | 5.97 | 17.5x / 17.5x |

Process CPU corroborates the wall ordering in every cell: PnP burns **6.4x–23.1x**
Orca's CPU, so no cell is a wall result hiding a CPU regression.

## Output disclosure

- **PnP 16/16 groups `degraded=false`, `non_fatal=0`** (every measured PnP run,
  both modes, all eight cells).
- **PnP ordinary ≡ PnP accelerated, byte for byte, in all 8 cells** — one SHA
  per cell, shared by both modes and by every measured run and warmup.
- **The PnP output is the t87 post-t81 reference set, exactly**: all eight
  cells' SHA-256 match
  [`../scoreboard-refresh-20261025/SCOREBOARD.md`](../scoreboard-refresh-20261025/SCOREBOARD.md)'s
  reference table (e.g. base classic-off `292df501…`, 21,023,739 bytes;
  base classic-on `22200671…`, 42,507,912 bytes). So the PnP job is unchanged
  and the only moving side is Orca.
- **Orca G-code bytes are constant within each cell** across this campaign's
  four runs (e.g. benchy classic-off 3,999,822; base classic-off 20,299,883).
  Its SHA-256 varies run to run because the header carries run metadata.
  Orca bytes are **not** the matched job's output parity target (the map's
  fairness contract treats output as disclosed evidence only).

## Disclosures (measurement environment)

- The campaign ran 2026-10-31 20:42–21:30 (five ledger batches, all same
  session, tools alternating). Pre-campaign load samples were 28.6 / 26.1 /
  17.9 / 6.1 / 9.2 % of 12 logical cores; the machine carries the owner's
  browser and desktop apps, as every prior take on this rig has disclosed.
- **`base arachne-on` is noise-limited and its exclusions are named.** Its
  ledger batch `t103-base-arachne-on` starved
  **`base-arachne-on-orca-m3`** (30.54 s, cpu/wall 2.00 vs group best 2.78) and
  **`base-arachne-on-pnp-ordinary-m3`** (244.63 s, 4.25 vs 6.47); both are
  excluded by the 0.75 rule and the retained medians are over two samples for
  those two tools. Its accelerated `m3` (188.10 s, 5.51 vs 6.22) is **retained
  by the rule** but sits 11 % above the other two rounds; its median therefore
  carries that round. The two clean rounds give a range of
  **6.95–7.05x ordinary** and **7.23–7.53x accelerated**; the table reports the
  protocol medians (7.00x / 7.53x). The verdict (PnP loses by ~7x, 79 % of the
  cell would have to disappear) is unaffected by the band.
- **The whole-cell re-run requirement was attempted three times and defeated by
  external load.** `t103-base-arachne-on-r2` (2026-11-01 20:07) overlapped a
  competing integration-test binary in another checkout
  (`D:\slicerProject\pinch_n_print_cli\target\debug\deps\executor-…exe`, started
  20:13:45, ~7 of 12 cores); `r3` and `r4` ran at 30–96 % background load
  (sustained browser CPU) and every PnP sample in them was 208–302 s against a
  clean ~160–175 s. All three CSVs are preserved in this directory as
  disclosure and are **not** ledger input. The human was asked and instructed
  the take to proceed with the noise disclosed (2026-11-01).
- The accelerated host tree's first build attempts failed in `libmimalloc-sys`
  with `cl.exe` exit 2. Root cause (measured, not guessed): cargo hands `cc` a
  `\\?\` verbatim `OUT_DIR` for the accelerated target dir, and `cl.exe`
  mishandles it (`mimalloc.h` missing on one run, `\\mimalloc-static.cc` on the
  next). Building the same target dir through a **relative** `CARGO_TARGET_DIR`
  with the same shim environment compiles cleanly; `xtask dist` then stages the
  built artifacts. No source or codegen input differs — only the absolute path
  cargo embeds.
- The verifier asserts zero starvation across the **ledger** batches; the two
  named exclusions above are the only ones in those batches.

## Re-derived park bound

The park condition ([ticket 12](../../issues/12-gap-budget-per-cell.md)) is "a
cell within ~1.5x of Orca". Every cell in this scoreboard needs a **57–82 %**
wall reduction to reach it, i.e. **72–102x** the map's whole 0.80 % floor:

- nearest cells: **benchy arachne-off 3.52x** (60.3 %, 75 floor-multiples) and
  **benchy classic-off 3.78x** (60.3 %, 75x on the ordinary arm);
- farthest: **base classic-on 8.25x** (81.8 %, 102 floor-multiples) and
  **base arachne-on 7.00x** (78.6 %, 98x).

The park stays binding, and the release bound is **3.52x** — *worse* than the
2026-10-25 scoreboard's 2.71x, but the cause is the Orca side moving fast on
benchy under the repaired rig (−24…−35 %), not a PnP regression. PnP's own
benchy walls are within −2.9…+1.9 % against 2026-10-25, and its base supports-on
walls fell 6–14 % (t102's allocator).

**Route options that follow** (the take order is the human's; no ticket is
graduated by this scoreboard):

1. **Supports-on geometry** remains the only family with a measured large
   increment: supports-on costs +36…+67 % of the corresponding supports-off
   multiple, and the increment is ~69 % support geometry (t88), dominated by
   the tree planner. Every in-map candidate already priced is 0.5–7 % of a cell
   — real, but 10–30 of them would be needed for the nearest cell's 60 %.
2. **The organic support engine** (`DEV-156`) is the one structural lever of
   the right size, and it is **OrcaSlicer geometry-parity work** — out of scope
   unless the human makes a t81-style exception.
3. **The adhesion asymmetry** (PnP 8 mm brim vs pinned Orca 0) is a
   job-definition decision, not a route: its slice-time price is ~235 ms, far
   below any floor, but closing it changes the PnP side of every cell and
   re-derives the reference byte set. It needs its own human decision first.

## Reproduction

See [REPRODUCE.md](REPRODUCE.md). The five ledger batches are the row CSVs in
this directory (`rows-*.csv`; the four 2026-11-01 base arachne-on attempts are
committed as disclosure only). `analyze.py` reprints this file's tables and
writes `summary.json`; `SUMMARY.txt` is the rig's own run dump;
`verify.py` re-derives every claim from the committed CSVs
(`--self-test` requires every negative control to be detected). Raw capture
trees remain under the gitignored `target/matched-pair/<BatchId>/`.
