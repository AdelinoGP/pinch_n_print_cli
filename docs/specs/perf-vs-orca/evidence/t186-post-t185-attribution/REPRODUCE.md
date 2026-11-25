# Reproduce t186 — post-t185 full-matrix attribution and 8/8 + 0.50x headroom map

Ticket: [Post-t185 full-matrix attribution and 8/8 closure headroom map](../../issues/186-post-t185-full-matrix-attribution.md).

## 1. Checkout-only verification (no fixtures, no builds)

Verifies every number in `reduced/arithmetic.json` and `FINDINGS.md` from committed artifacts only (Python 3 stdlib only):

```bash
python docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/verify.py --self-test
```

Expected output: 5/5 `PASS` checks (`payload_recomputes`, `runs_match_reference`, `instrumented_valid`, `profile_valid`, `brackets_reconcile`), `12/12 negative controls rejected`, final `PASS` (exit 0).

## 2. Full re-capture from scratch (requires local `tmp/3dbenchy.stl` and `tmp/base.stl`)

Run unsandboxed on Windows (sandboxed execution distorts short per-layer WASM/stderr calls by ~2.8×):

```bash
cargo xtask build-guests --check
cargo build --release -p pnp-cli
python docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/run_instrumented.py
python docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/run_profile.py --tag clean
python docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/reduce_instrumented.py
python docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/reduce_profile.py --tag clean
python docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/arithmetic.py
python docs/specs/perf-vs-orca/evidence/t186-post-t185-attribution/verify.py --self-test
```

## 3. Stamped capture identities (2026-11-23)

- Git HEAD: `92b62d9a307d542247669101165fd1a815943049`
- Last code commit: `748efbf583f75e0289d169b7a65532379e476cfa` (t185)
- `cargo xtask build-guests --check`: exit `0`
- `target/release/pnp_cli.exe`: sha256 `e661baa515c64b6d1c79a3216e1388eaf593ece3948e0bf8ec09430d66685d37`
- `modules/core-modules/` (24 `.wasm`): tree sha256 `8d33516c5352a5d682316f5834a61f8fa39300e0101b61b5c87b381bc60c705b` (`tree-support-planner.wasm` = `cf917686c1d7e6bb6cdd1d9d69684ed69ef46ab05d7d092dd5161e3f5a88a0ac`)
- Reference currency: `docs/specs/perf-vs-orca/evidence/t182-spatial-hierarchical-phase3-union/reference.json`
- Thread pin: `RAYON_NUM_THREADS=12`, `--no-default-module-paths`
