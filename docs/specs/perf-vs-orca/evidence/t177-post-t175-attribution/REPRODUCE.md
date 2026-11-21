# Reproduce t177 — post-t175 full-matrix attribution and 0.50x headroom map

Ticket: [Post-t175 full-matrix attribution and 0.50x headroom map](../../issues/177-post-t175-full-matrix-attribution.md).

## 1. Checkout-only verification (no fixtures, no builds)

Verifies every number in `reduced/arithmetic.json` and `FINDINGS.md` from committed artifacts only (Python 3 stdlib only):

```bash
python docs/specs/perf-vs-orca/evidence/t177-post-t175-attribution/verify.py --self-test
```

Expected output: 5/5 `PASS` checks (`payload_recomputes`, `runs_match_reference`, `instrumented_valid`, `profile_valid`, `brackets_reconcile`), `12/12 negative controls rejected`, final `PASS` (exit 0).

## 2. Full re-capture from scratch (requires local `tmp/3dbenchy.stl` and `tmp/base.stl`)

Run unsandboxed on Windows (sandboxed execution distorts short per-layer WASM/stderr calls by ~2.8×):

```bash
cargo xtask build-guests --check
cargo build --release -p pnp-cli
python docs/specs/perf-vs-orca/evidence/t177-post-t175-attribution/run_instrumented.py
python docs/specs/perf-vs-orca/evidence/t177-post-t175-attribution/run_profile.py --tag clean
python docs/specs/perf-vs-orca/evidence/t177-post-t175-attribution/reduce_instrumented.py
python docs/specs/perf-vs-orca/evidence/t177-post-t175-attribution/reduce_profile.py --tag clean
python docs/specs/perf-vs-orca/evidence/t177-post-t175-attribution/arithmetic.py
python docs/specs/perf-vs-orca/evidence/t177-post-t175-attribution/verify.py --self-test
```

## 3. Stamped capture identities (2026-11-21)

- Git HEAD: `24848d9e07dbf87d0c5dade687ed34de3fc44b2e`
- Last code commit: `5b138f5c2b2868e0bd436353633f5119266d8214` (t175)
- `cargo xtask build-guests --check`: exit `0`
- `target/release/pnp_cli.exe`: sha256 `f30752321eac2bac4a45be37a1e82f1e646515ffdb95af150ed6fb595e177fb3`
- `modules/core-modules/` (24 `.wasm`): tree sha256 `b46eb4cfc648bf2ec633773f43be0bb269802b5f9a0c842a8dd4e696322eb4db` (`tree-support-planner.wasm` = `c56f07b33594573b86cea64d932e427d417a4bd044bfa0116eeaa2a1f951eaef`)
- Reference currency: `docs/specs/perf-vs-orca/evidence/t175-tiled-collision-carve/reference.json`
- Thread pin: `RAYON_NUM_THREADS=12`, `--no-default-module-paths`

## 4. Committed file inventory

- `FINDINGS.md` — full post-t175 attribution, route analysis (`< 1.0x` and `<= 0.50x`), and sharpened inputs for `t178`–`t181`.
- `REPRODUCE.md` — this file.
- `runs.json` — stamped build identity + 24 instrumented pass records (hash, bytes, validation flags, code-1200 counts).
- `run_instrumented.py`, `run_profile.py` — capture runners (raw streams land in gitignored `target/t177-instrumented/` and `target/t177-profile/`).
- `reduce_instrumented.py`, `reduce_profile.py` — raw-to-reduced reducers.
- `arithmetic.py` — checkout-only phase/stage/census/headroom calculator.
- `verify.py` — checkout-only leaf-by-leaf verifier + 12 negative-control `--self-test`.
- `reduced/instrumented.json` — reduced 24-pass instrumented phase, stage, per-layer module, straggler-tail, and SupportGeometry post-planner tail metrics.
- `reduced/profile-clean.json` — 8-cell `--profile` verification + summed scope totals.
- `reduced/profile-detail-clean.json` — 8-cell `--profile` per-module fuel and per-builtin host thread-time breakdown.
- `reduced/arithmetic.json` — complete machine-readable post-t175 split, `ESTIMATE ONLY` `< 1.0x`/`<= 0.50x` ceilings, census, and `ticket_inputs` for `t178`–`t180`.
