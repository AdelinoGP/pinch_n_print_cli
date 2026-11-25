# Reproducing Wayfinder Ticket 181 Evidence

Ticket: [`SupportGeometry` post-planner host tail attribution and reduction screen](../../issues/181-support-geometry-post-planner-host-tail.md).

## 1. Checkout-Only Verification (No `target/`, No STL Fixtures, Python 3 Stdlib Only)

From the repository root:

```bash
python docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/verify.py
python docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/verify.py --self-test
```

*(On Windows with blocked WindowsApps `python` aliases inside a sandbox, invoke `C:\preFlight\python\python.exe`.)*

Expected output:

- `verify.py`: `VERIFY PASS: 7/7 check groups green (22/22 ref-identical runs, 6/6 qual clean, 632/632 PPO exact, 10918/10918 adversarial 0 FN / 0 FP)` (exit `0`)
- `verify.py --self-test`: `SELF-TEST PASS: 14/14 negative controls rejected with exact error prefixes` (exit `0`)

To regenerate `reduced/arithmetic.json` strictly from committed JSON files:

```bash
python docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/arithmetic.py
```

---

## 2. Re-Capturing Raw Observation-Only Probe Data (Optional; Requires Local STL Fixtures)

Requires the gitignored user-supplied matched fixtures (`tmp/3dbenchy.stl` and `tmp/base.stl`) and clean guest artifacts:

```bash
# 1. Confirm clean guests before probing
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check

# 2. Apply the observation-only probe patch (env-gated: PNP_T181_PROBE_OUT, PNP_T181_VERIFY_PPO, PNP_T181_DUMP_IO)
git apply docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/probe.patch

# 3. Build release pnp_cli with probe hooks enabled
cargo build --release -p pnp-cli

# 4. Run the 22-run attribution, qualification, PPO verification & IO dump capture (writes target/t181-probe/)
python docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/run_probe.py

# 5. Reduce raw captures into committed reduced/probe.json
python docs/specs/perf-vs-orca/evidence/t181-support-geometry-host-tail/reduce_probe.py

# 6. Cleanly revert probe edits, rebuild clean release pnp_cli, and verify guest freshness
git checkout -- crates/
cargo build --release -p pnp-cli
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

---

## 3. Standalone 12-Thread Exact IR Replay & Adversarial Falsification (Optional)

The standalone replay harness (`.local-artifacts/perf-vs-orca/t181-host-tail-replay/`) links against clean unmodified `crates/slicer-ir` and `crates/slicer-core` (`host-algos`), decodes `target/t181-probe/benchy.agg_io.postcard` (`200` entries) and `target/t181-probe/base.agg_io.postcard` (`432` entries), verifies bit-for-bit `SupportPlanIR` + diagnostic count equality for all 7 modes, runs 5 interleaved timing rounds at `RAYON_NUM_THREADS=12` (recording both wall and Windows `GetProcessTimes` CPU), runs the `10,918`-pair zero-tolerance adversarial falsification suite against raw Clipper2, and writes `reduced/replay_benchy.json` and `reduced/replay_base.json`.

---

## 4. Stamped Capture Identities (2026-11-22)

- Git HEAD: `7b82f188` (last code commit `5b138f5c`, t175; uncommitted tracker-only `t180` updates present in `docs/specs/perf-vs-orca/` and untouched)
- `cargo xtask build-guests --check`: exit `0` (before and after probe revert)
- `cargo xtask build-guests --accelerated --check`: exit `0` (before and after probe revert)
- Reference currency: `docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/reference.json`
- Thread pin: `RAYON_NUM_THREADS=12`, `--no-default-module-paths`

---

## 5. Committed File Inventory

| File | Purpose |
| --- | --- |
| [`FINDINGS.md`](FINDINGS.md) | Complete sub-bracket attribution, PPO equivalence proof, 12-thread replay matrix, zero-tolerance adversarial proof, `t137` gate & `ESTIMATE ONLY` route tables |
| [`REPRODUCE.md`](REPRODUCE.md) | This reproduction guide |
| [`probe.patch`](probe.patch) | Cleanly reverted observation-only probe + `PNP_T181_VERIFY_PPO` + `PNP_T181_DUMP_IO` (`crates/slicer-runtime/src/prepass.rs`, `crates/slicer-wasm-host/src/support_aggregation.rs`, `crates/slicer-wasm-host/src/exact_z_query.rs`) |
| [`run_probe.py`](run_probe.py) | 22-run probe, qualification, PPO check, and IO dump capture runner |
| [`reduce_probe.py`](reduce_probe.py) | Raw-capture reducer producing `reduced/probe.json` |
| [`arithmetic.py`](arithmetic.py) | Checkout-only calculator producing `reduced/arithmetic.json` |
| [`verify.py`](verify.py) | Checkout-only verifier + 14-case negative-control `--self-test` |
| [`reduced/probe.json`](reduced/probe.json) | Reduced 22-run qualification, 2-pair invariance, PPO verification, and sub-bracket attribution |
| [`reduced/replay_benchy.json`](reduced/replay_benchy.json) | 12-thread 5-round replay + adversarial zero-tolerance proof on `benchy` (`200` entries) |
| [`reduced/replay_base.json`](reduced/replay_base.json) | 12-thread 5-round replay + adversarial zero-tolerance proof on `base` (`432` entries) |
| [`reduced/arithmetic.json`](reduced/arithmetic.json) | Full checkout-recomputable reconciliation, gate, and stacked route arithmetic |
