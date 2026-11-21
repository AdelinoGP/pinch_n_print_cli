# Reproduce t175 (tiled collision operand for planner phase 1)

## Checkout-only verification (no build, no `target/` needed)

```bash
python docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/audit_t175.py --self-test
python docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/verify.py
```

Every script exits `0` using only committed JSONs under `reduced/`, `reference.json`, and sibling committed reduced artifacts (`t172`, `t173`, `t174`).

## Full campaign replay from HEAD + baseline

1. **Stage baseline arms** (at pre-t175 commit `d8892479`): build ordinary and accelerated distributions (`cargo xtask dist --edition developer` and `cargo xtask dist --edition developer --accelerated`) and copy into `target/t175-baseline/` and `target/t175-baseline-accelerated/`.
2. **Stage candidate arms** (at t175 commit): build ordinary and accelerated distributions and copy into `target/t175-candidate/` and `target/t175-candidate-accelerated/`. For the tile-size sweep, build `tree-support-planner.wasm` with `DEFAULT_COLLISION_TILE_UNITS` set to `10_000` (1 mm), `20_000` (2 mm), `50_000` (5 mm), and `100_000` (10 mm) into `target/t175-sweep/tile-{1,2,5,10}mm/modules/tree-support-planner/`.
3. **Run manifest, sweep, output check, and paired A/B / null / accelerated campaigns:**

```powershell
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/run_t175.ps1 -Mode manifest
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/run_t175.ps1 -Mode sweep -BatchId sweep1 -Rounds 3
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/run_t175.ps1 -Mode output -BatchId ref
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/run_t175.ps1 -Mode output -Accelerated -BatchId accel
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/run_t175.ps1 -Mode ab -BatchId ab -Pairs 5
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/run_t175.ps1 -Mode ab -NullControl -BatchId null -Pairs 3
pwsh -NoProfile -File docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/run_t175.ps1 -Mode ab -Accelerated -BatchId accel -Pairs 3
```

4. **Attribution probe (optional; never committed in production):**

Apply `probe.patch` (`git apply docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/probe.patch`), rebuild `tree-support-planner` (with `PROBE_ENABLE_TILING = false` for baseline and `true` across `DEFAULT_COLLISION_TILE_UNITS = 10_000 / 20_000 / 50_000 / 100_000`), run `pnp_cli slice --profile --instrument-stderr` into `target/t175/probe/<cell>__<arm>.stderr.txt`, and revert `probe.patch` (`git apply -R`).

5. **Reduce & verify:**

```bash
python docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/audit_t175.py
python docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/reduce_t175.py
python docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/verify.py --self-test
python docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/verify.py
```
