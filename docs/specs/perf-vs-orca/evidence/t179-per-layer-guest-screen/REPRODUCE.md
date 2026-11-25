# Reproducing Ticket 179 (`t179-per-layer-guest-screen`)

## 1. Checkout-Only Verification (No `target/`, No STL Fixtures, Python 3 Stdlib Only)

From the repository root:

```bash
python docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/arithmetic.py
python docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/verify.py
python docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/verify.py --self-test
```

Expected:
- `verify.py` prints `VERIFY PASS: 6/6 checks green (arithmetic recompute, 130/130 ref+diag, 16/16 exact fuel fold, tree-support invariants, infill-linker + L2 invariants, 8/8 combined screen floor + 24/24 round wins, probe.patch)` and exits `0`.
- `verify.py --self-test` rejects `14/14` targeted negative controls (each matched against its specific error prefix) and exits `0`.

## 2. Clean-Tree & Guest Freshness Verification

```bash
git diff --stat -- crates/ modules/ vendor/ xtask/
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

Expected: empty diff on code paths; both guest freshness checks exit `0`.

## 3. Re-Capturing the Observation-Only Attribution Probe (Requires `tmp/3dbenchy.stl` and `tmp/base.stl`)

1. Apply [`probe.patch`](probe.patch) cleanly on a clean working tree:
   ```bash
   git apply docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/probe.patch
   ```
2. Build `tree-support` and `infill-linker` guest WASMs into an isolated scratch module directory (e.g. `target/t179-probe-modules/` copied from `modules/core-modules/`, replacing only `tree-support/tree-support.wasm` and `infill-linker/infill-linker.wasm`).
3. Run all 8 cells (`{benchy, base} x {classic, arachne} x {off, on}`) with `RAYON_NUM_THREADS=12`, unsandboxed, against `target/release/pnp_cli.exe`:
   ```bash
   target/release/pnp_cli.exe slice \
     --model tmp/<model>.stl \
     --output target/t179-probe-raw/<cell>.gcode \
     --config docs/specs/perf-vs-orca/evidence/matched-pair/configs/pnp-<gen>-supports-<sup>.json \
     --module-dir target/t179-probe-modules \
     --no-default-module-paths \
     --profile --profile-verbose --instrument-stderr \
     2> target/t179-probe-raw/<cell>.stderr.txt
   ```
4. Verify each emitted `.gcode` SHA-256 and byte count against `docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/reference.json`.
5. Revert the probe cleanly (`git checkout -- crates/ modules/`) and re-run `cargo xtask build-guests --check`.
6. Reduce raw streams into `reduced/probe_attribution.json`:
   ```bash
   python docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/reduce_probe.py
   ```

## 4. Re-Running the Unsandboxed Candidate Screen

With isolated scratch module directories staged at `target/t179-arm-{ts1,ts2,il,both}` (copy of `modules/core-modules/` with candidate `tree-support.wasm` and/or `infill-linker.wasm` swapped in) and clean `target/release/pnp_cli.exe`:

```bash
python docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/run_screen.py
python docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/arithmetic.py
python docs/specs/perf-vs-orca/evidence/t179-per-layer-guest-screen/verify.py --self-test
```

**Important (Windows):** Run `run_screen.py` unsandboxed (`RAYON_NUM_THREADS=12` is set by the script); running inside the Windows AppContainer sandbox inflates per-layer WASM dispatches by ~2.8× on `benchy` (`t177` Methodological Disclosure #3).

## 5. Committed Artifact Inventory

| File | Role |
|---|---|
| `FINDINGS.md` | Headline attribution, premise correction, ablation & screen tables, route impact (`ESTIMATE ONLY`) |
| `REPRODUCE.md` | Exact checkout-only verification & full re-capture instructions |
| `probe.patch` | Reverted observation-only attribution probe diff (UTF-8, clean apply on `11a356b5`) |
| `reduce_probe.py` | Raw `--profile --profile-verbose --instrument-stderr` reducer → `reduced/probe_attribution.json` |
| `run_screen.py` | Unsandboxed Part 1 (`--instrument-stderr`) + Part 2 (1 warmup + 3 rounds wall/CPU) runner → `reduced/screen_runs.json` |
| `arithmetic.py` | Checkout-only calculator → `reduced/arithmetic.json` |
| `verify.py` | Checkout-only verifier (`6/6` checks + `14/14` negative-control `--self-test`) |
| `reduced/probe_attribution.json` | 8-cell probe scope folds, structural counters, `benchy` `L2` scopes, and 18 deterministic fuel ablation runs |
| `reduced/screen_runs.json` | 26 instrumented + 78 uninstrumented wall/CPU runs (`104/104` byte-identical to `t175`) |
| `reduced/arithmetic.json` | Re-derived reconciliation, kernel breakdowns, screen summaries, and `ESTIMATE ONLY` route projections |
