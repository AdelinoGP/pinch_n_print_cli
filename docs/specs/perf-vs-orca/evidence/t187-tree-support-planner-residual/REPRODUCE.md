# Reproducing Ticket 187 Evidence

## 1. Checkout-Only Verification (No Build, No `target/`, No Fixtures)

Verify every committed reduced JSON, arithmetic re-derivation, `t182/reference.json` hash match, classic ≡ arachne invariance check, `P5_tile` falsification record, and all 13 negative controls:

```bash
python docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/verify.py --self-test
```

To re-derive `reduced/arithmetic.json` directly from the committed `reduced/{probe_wall,probe_fuel,screen}.json`, `t182/reference.json`, and `t185/reduced/route.json`:

```bash
python docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/arithmetic.py
```

Verify zero production code edits on the checkout and guest freshness on both arms:

```bash
git diff -- crates/ modules/ vendor/ xtask/
cargo xtask build-guests --check
cargo xtask build-guests --accelerated --check
```

---

## 2. Re-Capturing the Observation-Only Probe (Optional, Local Only)

The observation-only host+guest probe diff is archived at [`probe.patch`](probe.patch). To re-capture raw probe runs under `target/t187-probe/` on a clean checkout (`RAYON_NUM_THREADS=12`, unsandboxed):

```bash
git apply docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/probe.patch
cargo build --release --bin pnp_cli
cargo xtask build-guests
python docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/run_probe.py
git checkout -- crates/ modules/
rm -f crates/slicer-wasm-host/src/t187_probe.rs
cargo xtask build-guests
```

---

## 3. Re-Capturing the Pure-Guest Candidate Screen (Optional, Local Only)

`run_screen.py` expects clean HEAD `target/release/pnp_cli.exe` plus staged module directories under `target/t187-screen/baseline-modules/` (clean HEAD `.wasm` set) and `target/t187-screen/candidate-full-modules/` (clean HEAD `.wasm` set with only `tree-support-planner/tree-support-planner.wasm` replaced by the `P1 + P2 + P3 + P4 + P5_zeroclone` build):

```bash
python docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/run_screen.py
python docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/reduce.py
python docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/arithmetic.py
python docs/specs/perf-vs-orca/evidence/t187-tree-support-planner-residual/verify.py --self-test
```
