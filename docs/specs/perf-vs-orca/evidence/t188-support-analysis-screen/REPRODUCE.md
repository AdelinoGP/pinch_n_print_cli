# Reproducing Ticket 188 Evidence

## 1. Checkout-Only Verification (No Build, No `target/`, No Fixtures)

Verify every committed reduced JSON, arithmetic re-derivation, `t182/reference.json` hash match, classic ≡ arachne invariance proof, monotonic ablation ladder, rejected mechanism disclosures, and all 13 negative controls:

```bash
python docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/verify.py --self-test
```

Re-derive `reduced/arithmetic.json` directly from committed reduced JSONs:

```bash
python docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/arithmetic.py
```

Verify zero production code edits on the checkout and guest freshness on both arms:

```bash
git diff -- crates/ modules/ vendor/ xtask/
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

---

## 2. Re-Capturing the Observation-Only Probe (Optional, Local Only)

The observation-only host probe diff is archived at [`probe.patch`](probe.patch). To re-capture raw probe runs under `target/t188/probe/` on a clean checkout (`RAYON_NUM_THREADS=12`, unsandboxed):

```bash
git apply docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/probe.patch
cargo build --release --bin pnp_cli
mkdir -p target/t188/probe && cp target/release/pnp_cli.exe target/t188/probe/pnp_cli.exe
python docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/run_probe.py
git checkout -- crates/
rm -f crates/slicer-core/src/t188_probe.rs
```

---

## 3. Re-Capturing the Candidate Screen (Optional, Local Only)

`run_screen.py` expects clean HEAD `target/t188/baseline/pnp_cli.exe` alongside the `SA-1`..`SA-6` candidate build at `target/t188/candidate/pnp_cli.exe`, both run against the untouched `modules/core-modules/` tree (`RAYON_NUM_THREADS=12`, unsandboxed):

```bash
python docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/run_screen.py
python docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/reduce.py
python docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/arithmetic.py
python docs/specs/perf-vs-orca/evidence/t188-support-analysis-screen/verify.py --self-test
```
