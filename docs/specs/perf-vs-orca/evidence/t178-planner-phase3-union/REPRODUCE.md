# Reproduce t178 — planner Phase-3 batched union attribution and mechanism screen

Ticket: [Planner Phase-3 batched union attribution and mechanism screen](../../issues/178-planner-phase3-union-attribution.md).

## 1. Checkout-only verification (no fixtures, no builds, no raw captures)

Re-derives `reduced/arithmetic.json` leaf-for-leaf from committed reduced JSONs and runs 14 negative-control checks (Python 3 stdlib only):

```bash
python docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/verify.py --self-test
```

Expected output: all checks `PASS`, `14/14 negative controls rejected`, final `PASS` (exit 0).

## 2. Full re-capture & offline replay from scratch (requires local `tmp/3dbenchy.stl` and `tmp/base.stl`)

```bash
# 1. Apply temporary capture probe and build probed guest + host
git apply docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/probe.patch
cargo xtask build-guests
cargo build --release -p pnp-cli

# 2. Capture qualification + binary Phase-3 corpora + clean uninstrumented timings (unsandboxed on Windows)
python docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/run_capture.py --timing-passes 3
python docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/reduce_timing.py

# 3. Build and run offline 12-thread replay harness on captured corpora
cargo build --manifest-path .local-artifacts/perf-vs-orca/t178-phase3-replay/Cargo.toml --release --offline
RAYON_NUM_THREADS=12 .local-artifacts/perf-vs-orca/t178-phase3-replay/target/release/t178-phase3-replay.exe \
  target/t178-capture/benchy-classic-on.phase3.bin \
  docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/reduced/replay_benchy.json 3
RAYON_NUM_THREADS=12 .local-artifacts/perf-vs-orca/t178-phase3-replay/target/release/t178-phase3-replay.exe \
  target/t178-capture/base-classic-on.phase3.bin \
  docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/reduced/replay_base.json 3

# 4. Re-derive arithmetic & verify
python docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/arithmetic.py
python docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/verify.py --self-test

# 5. Revert temporary probe cleanly and restore clean artifacts
git apply -R docs/specs/perf-vs-orca/evidence/t178-planner-phase3-union/probe.patch
cargo xtask build-guests
cargo build --release -p pnp-cli
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

## 3. Stamped capture identities (2026-11-21)

- Git HEAD at capture: `0c242fa5` (last code commit `5b138f5c`, t175)
- Probed `pnp_cli.exe` sha256: `22e3429981fb46546fa4702c9b2cdffc76ea341ad4aa862742e27f983c1adaaa`
- Probed `tree-support-planner.wasm` sha256: `b469a2ff0227d204eeb66b79ca5f05613d493a8c16c4316d13099fc1f982ed9f`
- Reference currency: `docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/reference.json` (`12/12` runs byte-identical)
- Thread pin: `RAYON_NUM_THREADS=12`, `--no-default-module-paths`
- Post-revert check: `git diff -- crates/ modules/ vendor/ xtask/` empty; `cargo xtask build-guests --check` exit `0`; `cargo xtask build-guests --accelerated --check` exit `0`.

## 4. Committed file inventory

- `FINDINGS.md` — full Phase-3 attribution, structural census, 29-mechanism screen, scanbeam proof, Hausdorff audit, and route pricing.
- `REPRODUCE.md` — this file.
- `probe.patch` — temporary env-gated Phase-3 timing + binary corpus dump probe (cleanly reverted from production tree).
- `run_capture.py` — qualification + dump + clean timing runner (raw dumps land in gitignored `target/t178-capture/`).
- `reduce_timing.py` — raw Phase-3 timing JSONL reducer.
- `arithmetic.py` — checkout-only route projection and floor calculator.
- `verify.py` — checkout-only leaf-by-leaf verifier + 14 negative-control `--self-test`.
- `reduced/prod_timing.json` — reduced 12-run capture qualification + clean per-role, per-batch, and top-call production Phase-3 metrics.
- `reduced/replay_benchy.json` — complete `benchy-classic-on` (`202` calls) structural census, singleton audit, scanbeam proof, and 29-mechanism screen.
- `reduced/replay_base.json` — complete `base-classic-on` (`480` calls) structural census, singleton audit, scanbeam proof, and 29-mechanism screen.
- `reduced/arithmetic.json` — complete machine-readable route projections (`ESTIMATE ONLY`), `t137` floor multiples, and geometric audit summary.
