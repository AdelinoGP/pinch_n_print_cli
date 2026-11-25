# Reproduce t193 (final single-HEAD `n >= 4` Orca closing scoreboard)

Zero production edits: this ticket builds the post-`t192` HEAD, runs the
matched-pair board, and records the result.

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/verify.py
```

Both exit `0` using only the committed `reduced/summary.json`,
`reduced/load-samples.csv`, and the committed sibling artifact
`../t182-spatial-hierarchical-phase3-union/reference.json`.

## Full campaign replay

1. **Preflight** (post-`t192` HEAD `97b2474d`, last code commit `e7b3c68e`):

   ```bash
   cargo xtask build-guests --check                 # exit 0
   cargo xtask build-guests --accelerated --check   # exit 0
   cargo xtask dist --edition developer             # stages target/dist/developer
   cargo xtask dist --edition developer --accelerated
   ```

   `run_scoreboard.ps1`'s `pnp-ordinary` tool is
   `target/release/pnp_cli.exe` + `modules/core-modules`; `cargo xtask dist
   --edition developer` leaves that binary byte-identical to
   `target/dist/developer/pnp_cli.exe` (measured
   `cc8a2a94a389b7665db4e894b3ba47d873992fcfa3ac767b237175bd13a13319`).

2. **Campaign** (quiet box, `RAYON_NUM_THREADS=12` set inside
   `run_scoreboard.ps1`):

   ```powershell
   pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/run_t193.ps1 `
     -BatchId t193-closing-board -Runs 4 -Warmups 1 -Threads 12
   ```

   The wrapper starts a 20 s background CPU + free-RAM sampler and invokes
   `docs/specs/perf-vs-orca/evidence/matched-pair/run_scoreboard.ps1` with
   `-Fixtures benchy,base -Cells classic-off,classic-on,arachne-off,arachne-on
   -Tools orca,pnp-ordinary,pnp-accelerated -Runs 4 -Warmups 1`, writing
   **120 data rows** (24 warmups + 96 measured) to
   `target/matched-pair/t193-closing-board/scoreboard-rows.csv` and the load
   samples to `target/t193-closing-board-load.csv`. A first invocation was
   aborted after one Orca warmup to add the sampler; its completed warmup row
   remains in the raw CSV, and `reduce_t193.py` keeps the latest warmup per
   cell/tool and discloses `warmup_rows_dropped_aborted_launch: 1`. No measured
   row is affected.

3. **Reduce and verify:**

   ```bash
   cp target/t193-closing-board-load.csv \
      docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/reduced/load-samples.csv
   python docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/reduce_t193.py
   python docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/verify.py --self-test
   python docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/verify.py
   ```

   `reduce_t193.py` reads the gitignored campaign CSV under
   `target/matched-pair/` plus `t182`'s `reference.json` and writes
   `reduced/summary.json` (medians over retained unstarved runs, t176 Option C
   margin tiers and closing-gate flags, PnP/Orca CPU efficiency disclosure).

4. **Attribution probe** (`benchy-classic-on`, one output-neutral pass each):

   ```bash
   target/release/pnp_cli.exe slice --model tmp/3dbenchy.stl \
     --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
     --module-dir modules/core-modules --output target/t193-attr/benchy-classic-on.gcode \
     --instrument-stderr
   target/release/pnp_cli.exe slice --model tmp/3dbenchy.stl \
     --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-classic-supports-on.json \
     --module-dir modules/core-modules --output target/t193-attr/benchy-classic-on-profile.gcode \
     --profile
   python docs/specs/perf-vs-orca/evidence/t193-closing-scoreboard/reduce_attribution_t193.py
   ```

   The probe G-code hashes to `a8c93535…` (the `t182` reference), so the
   attribution pass is output-neutral. `reduce_attribution_t193.py` reads the
   gitignored `target/t193-attr/` streams and writes
   `reduced/attribution.json` (phase split, postpass stages, fuel, output
   identity).

5. **Restore:** nothing to restore (zero production files edited). Re-check
   both freshness gates exit `0`.

## Stamped identities

| Artifact | Path | Identity |
| --- | --- | --- |
| Git HEAD | `97b2474da202f8728c63e23df00dfa08fba6fd18` | tracker claim commit |
| Last code commit | `e7b3c68e93da717481d21290b0df4818b34d11d8` | `t192` |
| OrcaSlicer | `C:/Program Files/OrcaSlicer/orca-slicer.exe` | **2.5.1**, `5013b1bd…`, 292,160 bytes, 2026-11-13T23:59:44-03:00 |
| PnP ordinary | `target/release/pnp_cli.exe` | `cc8a2a94…`, 24 modules, tree `2371167c…` |
| PnP accelerated | `target/dist-accelerated/developer/pnp_cli.exe` | `eec2a2ad…`, 24 modules, tree `7197a518…` |
| Guest freshness | ordinary / accelerated | both exit `0` |

## What this map's protocol requires

- `pnp-ordinary` is the **binding arm**; `pnp-accelerated` is disclosed beside
  it.
- A closing cell needs `< 1.00x` median uninstrumented wall on the binding arm,
  **`>= 4` retained unstarved runs per arm** (`cpu/wall >= 0.75 × group best`),
  **strict separation** (`max(PnP) < min(Orca)`), identity to
  `t182/reference.json`, and the **CPU integrity guard** (no starvation,
  `exit 0`, `degraded=false`, zero fatal/non-fatal, zero code-1200, matched
  Orca keys). PnP/Orca CPU ratio is disclosed as an efficiency metric, not a
  gate.
