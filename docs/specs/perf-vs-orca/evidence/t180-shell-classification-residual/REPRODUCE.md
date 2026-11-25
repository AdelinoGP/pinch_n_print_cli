# Reproducing Wayfinder Ticket 180 Evidence

## 1. Checkout-Only Verification (No `target/`, No STL Fixtures, Python 3 Stdlib Only)

From the repository root:

```bash
python docs/specs/perf-vs-orca/evidence/t180-shell-classification-residual/verify.py
python docs/specs/perf-vs-orca/evidence/t180-shell-classification-residual/verify.py --self-test
```

*(On Windows with blocked WindowsApps `python` aliases inside a sandbox, invoke `C:\preFlight\python\python.exe`.)*

Expected output:

- `verify.py`: `VERIFY PASS: 7/7 check groups green (15/15 ref-identical runs, 4/4 invariance, 100% t177 call recon, 8/8 EXACT_BUNDLE gate)` (exit `0`)
- `verify.py --self-test`: `SELF-TEST PASS: 15/15 negative controls rejected with exact error prefixes` (exit `0`)

To regenerate `reduced/arithmetic.json` directly from committed inputs (`reduced/probe.json`, `reduced/replay_benchy.json`, `reduced/replay_base.json`, `../t175-tiled-collision-carve/reference.json`, `../t177-post-t175-attribution/reduced/arithmetic.json`, `../t178-planner-phase3-union/reduced/arithmetic.json`, `../t179-per-layer-guest-screen/reduced/arithmetic.json`):

```bash
python docs/specs/perf-vs-orca/evidence/t180-shell-classification-residual/arithmetic.py
```

---

## 2. Re-Capturing Raw Observation-Only Probe Data (Optional; Requires Local STL Fixtures)

Requires the gitignored user-supplied matched fixtures (`tmp/3dbenchy.stl` and `tmp/base.stl`) and clean guest artifacts:

```bash
# 1. Confirm clean guests before probing
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check

# 2. Apply the observation-only probe patch (env-gated: PNP_T180_PROBE_OUT & PNP_T180_DUMP_IO)
git apply docs/specs/perf-vs-orca/evidence/t180-shell-classification-residual/probe.patch

# 3. Build release pnp_cli with probe hooks enabled
cargo build --release -p pnp-cli

# 4. Run the 15-run qualification + invariance capture (writes target/t180-probe/)
python docs/specs/perf-vs-orca/evidence/t180-shell-classification-residual/run_probe.py

# 5. Reduce raw captures into committed reduced/probe.json
python docs/specs/perf-vs-orca/evidence/t180-shell-classification-residual/reduce_probe.py

# 6. Optional: dump exact IR IO snapshots for standalone replay (benchy & base)
#    Set PNP_T180_DUMP_IO=target/t180-probe/benchy.shell_io.postcard (resp. base.shell_io.postcard)
#    and slice benchy-classic-off / base-classic-off once with matched settings.

# 7. Cleanly revert probe edits, rebuild clean release pnp_cli, and verify guest freshness
git checkout -- crates/
cargo build --release -p pnp-cli
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

---

## 3. Standalone 12-Thread Exact IR Replay (Optional)

The standalone replay harness (`.local-artifacts/perf-vs-orca/t180-shell-replay/`) links against clean unmodified `crates/slicer-core`, `crates/slicer-ir`, and `vendor/clipper2-rust`, decodes `benchy.shell_io.postcard` (`240` layers) and `base.shell_io.postcard` (`496` layers), verifies `M0_baseline == expected_out` bit-for-bit, runs 3 interleaved rounds at `RAYON_NUM_THREADS=12`, and writes `reduced/replay_benchy.json` and `reduced/replay_base.json`.

---

## 4. Committed Artifacts

| File | Purpose |
|---|---|
| [`FINDINGS.md`](FINDINGS.md) | Complete attribution, canonical OrcaSlicer comparison, replay matrix, exactness proofs, and `ESTIMATE ONLY` 8-cell route tables |
| [`REPRODUCE.md`](REPRODUCE.md) | This reproduction guide |
| [`probe.patch`](probe.patch) | Cleanly reverted observation-only probe + `PNP_T180_DUMP_IO` hook (`crates/slicer-core/src/algos/bridge_over_infill.rs`, `crates/slicer-core/src/algos/prepass_slice.rs`, `crates/slicer-runtime/src/slice_postprocess_prepass.rs`) |
| [`run_probe.py`](run_probe.py) | 15-run probe & qualification capture runner |
| [`analyze_probe.py`](analyze_probe.py) | Console explorer for raw `target/t180-probe/` captures |
| [`reduce_probe.py`](reduce_probe.py) | Raw-capture reducer producing `reduced/probe.json` |
| [`arithmetic.py`](arithmetic.py) | Checkout-only calculator producing `reduced/arithmetic.json` |
| [`verify.py`](verify.py) | Checkout-only verifier + 15-case negative-control `--self-test` |
| [`reduced/probe.json`](reduced/probe.json) | Reduced 15-run qualification, 4-pair invariance, bracket/site/locality data |
| [`reduced/replay_benchy.json`](reduced/replay_benchy.json) | 12-thread Round-2 replay on 240 `benchy` layers |
| [`reduced/replay_base.json`](reduced/replay_base.json) | 12-thread Round-2 replay on 496 `base` layers |
| [`reduced/arithmetic.json`](reduced/arithmetic.json) | Full checkout-recomputable reconciliation, gate, and route arithmetic |
