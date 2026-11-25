# Reproducing Ticket 189 Evidence

## 1. Checkout-Only Verification (No Build, No `target/`, No Fixtures)

Verify every committed reduced JSON, arithmetic re-derivation, `t182/reference.json` hash/byte parity across all `32` probe runs and `92` measured screen runs (`94` total including warmups), ablation monotonicity, deterministic guest-fuel reduction, deferred WIT-lead disclosures, and all `14` negative controls:

```bash
python docs/specs/perf-vs-orca/evidence/t189-postpass-screen/verify.py --self-test
```

Re-derive `reduced/arithmetic.json` directly from committed reduced JSONs:

```bash
python docs/specs/perf-vs-orca/evidence/t189-postpass-screen/arithmetic.py
```

Verify zero production code edits on the checkout and guest freshness on both arms:

```bash
git diff -- crates/ modules/ vendor/ xtask/
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

---

## 2. Re-Capturing the Observation-Only Probe (Optional, Local Only)

The observation-only host probe diff is archived at [`probe.patch`](probe.patch). To re-capture raw 8-cell probe runs (`32` runs total) under `target/t189/probe/` on a clean checkout (`RAYON_NUM_THREADS=12`, unsandboxed):

```bash
git apply docs/specs/perf-vs-orca/evidence/t189-postpass-screen/probe.patch
cargo build --release --bin pnp_cli
mkdir -p target/t189/probe && cp target/release/pnp_cli.exe target/t189/probe/pnp_cli.exe
python docs/specs/perf-vs-orca/evidence/t189-postpass-screen/run_probe.py
git checkout -- crates/
```

---

## 3. Re-Capturing the Zero-WIT Candidate Screen (Optional, Local Only)

`run_screen.py` expects:
- Clean HEAD baseline binary at `target/t189/baseline/pnp_cli.exe` run against untouched `modules/core-modules/`.
- `PP-1 + PP-2 + PP-3` candidate binary at `target/t189/candidate/pnp_cli.exe` (supporting `T189_FLAGS` env-var toggles for ablations) run against the staged lazy-`LayerCollectionView` module directory at `target/t189/candidate-mods/` (containing the 4 rebuilt `LayerFinalization` guests built in an isolated `CARGO_TARGET_DIR` plus copies of all other untouched core modules).

Run (`RAYON_NUM_THREADS=12`, unsandboxed):

```bash
python docs/specs/perf-vs-orca/evidence/t189-postpass-screen/run_screen.py
python docs/specs/perf-vs-orca/evidence/t189-postpass-screen/reduce.py
python docs/specs/perf-vs-orca/evidence/t189-postpass-screen/arithmetic.py
python docs/specs/perf-vs-orca/evidence/t189-postpass-screen/verify.py --self-test
```
