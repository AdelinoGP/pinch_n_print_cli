# Arachne wall-simplification tolerance pricing

Route: [Arachne wall-simplification tolerance pricing](../../issues/83-arachne-wall-simplification-tolerance-pricing.md).
This is a measurement-only pricing take (ticket 83 step 1): no candidate, no
parity port, no production edit, no default change.

## Verdict

**DROP — below the map's pre-declared floor.** Matching canonical's Arachne
wall-simplification tolerances (`wall_maximum_resolution` 0.5 mm,
`wall_maximum_deviation` 0.025 mm against PnP's 0.05 / 0.005 mm) is **real
work**: the module's guest fuel falls **10.76 %** on both Benchy Arachne cells
and **16.21 %** on both base Arachne cells, and the printed G-code shrinks
**3.8–8.2 %** with identical TYPE counts. But `com.core.arachne-perimeters` is
only **2.9–6.9 %** of cell wall and runs at **5.1–10.7x** phase parallelism, so
the occupancy-normalized wall ceiling is **0.039–0.376 %** of cell wall —
below the pre-declared **0.80 %** floor in every cell. The direct uninstrumented
6-pair A/B on the two borderline cells agrees: no wall win outside paired noise.
No parity work is justified; `D-168` is retained with a measured magnitude.

## Method

- Captured 2026-10-23 23:23 → 2026-10-24 00:11 local (the batch crossed midnight).
- HEAD `40eefc5a557b3d5ad81a9a4757bbf4ec59e36735`, `cargo build --release --bin
  pnp_cli`, guests rebuilt from scratch (the shared `target/guests` tree was
  absent) and `cargo xtask build-guests --check` exit 0 before the batch;
  `module diagnose` 24/24 modules `external`; binary and module-tree hashes
  recorded in `inputs.json` and re-checked after each fixture.
- **Arms — one binary, one module tree, config-only.** `default` = the committed
  matched Arachne config ([`matched-pair/configs/`](../matched-pair/configs/)),
  both keys unset → code fallback 0.05 / 0.005 mm. `canonical` =
  [`configs/pnp-arachne-supports-*-canonical.json`](configs/), the same job plus
  `wall_maximum_resolution = 0.5` and `wall_maximum_deviation = 0.025`.
  The emitted G-code appendix discloses the applied values in both arms
  (`; wall_maximum_resolution = 0.05` / `0.5`), so the override is confirmed at
  the resolver, not just in the config file.
- **Instrumented runs** (`run_price.ps1`): 2 alternating pairs per cell
  (`A B`, then `B A`), 12 rayon threads, `--profile --profile-verbose
  --instrument-stderr`, ordinary mode. 16 runs total. These runs are
  **attribution only**; they are never a wall claim.
- **Uninstrumented wall check** (`run_wall_pairs.ps1`): 6 alternating pairs on
  the two cells whose instrumented wall or fuel-proportional bound was closest
  to the floor (`benchy-arachne-supports-on`, `base-arachne-supports-off`),
  clean process wall/CPU, no instrumentation.
- **Disclosure:** the machine was the user's desktop, with background processes
  (browser, terminal) live throughout. Every run records `cpu/wall`; no
  starvation exclusion was needed or applied. Instrumented `cpu/wall` is
  4.5–8.3, uninstrumented 4.6–8.1.

## Instrumented price (attribution only)

Arachne module fuel and accumulated module elapsed, medians of two runs per arm.
The module share of wall uses the measured phase parallelism
(`total per-layer module accumulated elapsed / per_layer phase wall`).

| cell | arachne fuel default → canonical | fuel Δ | module accum default → canonical | module share of cell wall | occupancy-normalized ceiling | idealized serial ceiling | fuel-proportional bound |
|---|---:|---:|---:|---:|---:|---:|---:|
| benchy arachne-off | 10.183 B → 9.087 B | −10.76 % | 3.59 → 3.46 s (−3.8 %) | 6.87 % | 0.261 % | 1.329 % | 0.739 % |
| benchy arachne-on | 10.183 B → 9.087 B | −10.76 % | 3.52 → 3.33 s (−5.6 %) | 3.39 % | 0.190 % | 1.240 % | 0.365 % |
| base arachne-off | 127.428 B → 106.766 B | −16.21 % | 49.47 → 49.09 s (−0.8 %) | 5.20 % | 0.039 % | 0.403 % | 0.844 % |
| base arachne-on | 127.428 B → 106.766 B | −16.21 % | 64.89 → 56.54 s (−12.9 %) | 2.92 % | 0.376 % | 4.023 % | 0.474 % |

Reading the table:

- **The fuel delta is deterministic and real.** Within an arm the two repeats
  agree on the module fuel to ≤ 0.0026 %, and the whole-job
  guest fuel falls **−3.41 % / −1.41 %** on Benchy arachne off/on and
  **−11.56 % / −3.08 %** on base arachne off/on. The host scope
  `polygon_ops::offset` is unchanged (313.2 M fuel on Benchy, 2.43 B on base),
  so the entire delta is module self fuel — the tolerance acts inside the guest,
  not on host clipping.
- **The accumulated elapsed delta is mostly noise.** Its per-cell medians are
  −0.8 % / −5.6 % / −3.8 % / −12.9 %, but the sample ranges overlap on three
  cells (base-off is a single default outlier at 74.0 s) and base-off's median
  moves the wrong way; elapsed is accumulated worker time under background load,
  not a work measure. Fuel is the load-independent signal.
- **The ceilings are the price.** Even the generous *fuel-proportional* bound
  (every guest instruction costing the same wall) reaches the floor only on
  base-off, and only just (0.844 %); the measured elapsed on that cell is
  +0.8 % in the wrong direction. The idealized serial bound is an upper bound
  that assumes no parallelism at all and is not attainable on this phase.

## Output delta

| cell | G-code bytes | Δ | TYPE counts | completion |
|---|---:|---:|---|---|
| benchy arachne-off | 4,064,270 → 3,742,107 | −7.93 % | identical | `ok`, not degraded, 0 fatal / 0 non-fatal |
| benchy arachne-on | 8,390,168 → 8,067,990 | −3.84 % | identical | `ok`, not degraded, 0 fatal / 0 non-fatal |
| base arachne-off | 20,846,160 → 19,131,553 | −8.23 % | identical | `ok`, not degraded, 0 fatal / 0 non-fatal |
| base arachne-on | 42,330,265 → 40,615,661 | −4.05 % | identical | `ok`, not degraded, 0 fatal / 0 non-fatal |

Each arm is byte-deterministic across its repeats (one SHA per arm per cell),
and the two arms differ in every cell. The default arm reproduces the t81
post-port reference for `base-arachne-off` (`01a1434c…`), so the pricing arm is
measured at the same geometry the map currently scores.

## Direct wall check

Uninstrumented, six alternating pairs per cell, clean process wall/CPU
(`run_wall_pairs.ps1`, reduced by `wall_pairs.py`):

| cell | wall default → canonical (medians) | paired wall deltas | median wall | CPU median | output |
|---|---:|---|---:|---:|---|
| base arachne-off | 87.68 → 88.30 s | +0.58, −2.33, −0.23, +3.68, −2.42, +1.68 % | **+0.17 %** | +0.09 % | deterministic per arm |
| benchy arachne-on | 13.67 → 14.08 s | +3.16, +3.21, +0.14, +5.19, −4.68, +13.42 % | **+3.18 %** | +0.17 % | deterministic per arm |

Neither cell shows a win. `base-arachne-off` is flat (+0.17 % wall, +0.09 %
CPU). `benchy-arachne-on`'s **instrumented −2.9 % was load drift**: the clean
A/B is nominally *slower* (+3.18 % wall) while CPU is flat (+0.17 %), i.e. the
wall delta is scheduling noise on the small cell, not a cost. The pre-declared
gate reads the same verdict either way: both cells are below the 0.80 % floor.

## What this means

1. **The deviation is a work discrepancy, not a wall discrepancy.** It is worth
   recording as measured: matching canonical would cut Arachne guest work by
   10.8–16.2 % and G-code volume by 3.8–8.2 %, which is a real bandwidth and
   consumer-side benefit. It is *not* a lever toward this map's destination,
   because the per-layer phase is a small, deeply parallel slice of cell wall
   and the serial prepass dominates.
2. **No parity port is justified by the wall verdict.** Ticket 83's step-1 stop
   rule fires. `D-168` stays open with this measured magnitude; the row records
   that the fallback is retained deliberately.
3. **A future decision about G-code size is separate.** The t36 precedent
   (modal Z/F compression, dropped because bytes are not this destination's
   verdict) applies. If a bandwidth-bound consumer ever becomes a target, this
   measurement is reusable; nothing here authorises a default change.

## Evidence inventory

| file | role |
|---|---|
| `run_price.ps1` | instrumented 4-cell pricing harness (2 alternating pairs per cell) |
| `run_wall_pairs.ps1` | uninstrumented 6-pair wall check on the borderline cells |
| `configs/pnp-arachne-supports-*-canonical.json` | canonical-tolerance arm (matched job + 2 keys) |
| `reduce.py` | reduces raw captures, recomputes the gate verdict, writes `summary.json` |
| `verify.py` | checkout-only re-derivation + negative controls (`--self-test`) |
| `wall_pairs.py` | reduces the uninstrumented pairs, writes `wall-pairs.json` |
| `captures/*.json` | reduced per-run captures (phase brackets, module calls, profile, output census) |
| `summary.json` | per-run metrics, per-cell arm medians, gate verdict |
| `wall-pairs.json` | paired wall/CPU deltas and output determinism for the wall check |
| `inputs.json` | commit, binary/model/module hashes, provenance |

Raw captures (G-code, stderr JSONL, row CSVs) live under the gitignored
`target/t83-price/` and are regenerable from the two runners; the in-repo
reduced set is self-sufficient.
