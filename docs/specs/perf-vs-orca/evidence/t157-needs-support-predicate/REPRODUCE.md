# Reproducing Ticket 157 (`t157-needs-support-predicate`)

This directory contains the attribution probe, reduction scripts, and verification suite for Ticket 157 ([`docs/specs/perf-vs-orca/issues/157-needs-support-predicate-attribution.md`](../../issues/157-needs-support-predicate-attribution.md)).

## Committed Artifacts

- [`probe.patch`](probe.patch) — temporary `PNP_T157_PROBE` / `PNP_T157_FAST_PREDICATE` instrumentation patch against `crates/slicer-sdk/src/views.rs`, `crates/slicer-runtime/src/layer_executor.rs`, and `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` (reverted from the working tree before closing Ticket 157).
- [`run-probe.py`](run-probe.py) — runs `pnp_cli slice --instrument-stderr` with `PNP_T157_PROBE=1` across `benchy-classic-off`, `benchy-classic-on`, `base-classic-off`, and `base-classic-on` in both `dual` and `fast` (`PNP_T157_FAST_PREDICATE=1`) modes, validating exit code `0` and G-code SHA-256 parity against [`docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/reference.json`](../t151-skirt-hull-contour/reference.json).
- [`runs.json`](runs.json) — per-run exit status, attribution-only wall time, and G-code SHA-256 verification.
- [`reduce.py`](reduce.py) — reduces `target/t157-probe/<cell>.<mode>.jsonl` into [`probe.json`](probe.json).
- [`probe.json`](probe.json) — reduced per-cell, per-mode, per-caller predicate statistics (`ns_dual_by_caller`, `ns_fast_by_caller`), `sa_eligibility` / `sa_contacts` walls, and `per_layer_stages` `prep_views_ms` / `run_stage_ms` breakdowns.
- [`price.py`](price.py) — combines [`probe.json`](probe.json) with `t154`'s [`arithmetic.json`](../t154-benchy-per-layer/arithmetic.json) and [`carriers.json`](../t154-benchy-per-layer/carriers.json) to produce [`summary.json`](summary.json).
- [`summary.json`](summary.json) — committed reader census, equivalence verification (`3,680` dual calls, `0` mismatches), per-cell thread and priced wall-equivalent savings, and stop-rule evaluation.
- [`FINDINGS.md`](FINDINGS.md) — full narrative report and tables.
- [`verify.py`](verify.py) — standalone verifier (`--self-test` supported) checking internal consistency of `runs.json`, `probe.json`, `summary.json`, `t151` reference hashes, and `t154` ledger arithmetic.

## Step-by-Step Reproduction

1. **Verify the committed evidence and self-test (no rebuild needed):**
   ```powershell
   python docs/specs/perf-vs-orca/evidence/t157-needs-support-predicate/verify.py --self-test
   ```

2. **(Optional) Re-run the temporary probe from scratch:**
   ```powershell
   git apply docs/specs/perf-vs-orca/evidence/t157-needs-support-predicate/probe.patch
   cargo build --release -p pnp-cli
   python docs/specs/perf-vs-orca/evidence/t157-needs-support-predicate/run-probe.py
   python docs/specs/perf-vs-orca/evidence/t157-needs-support-predicate/reduce.py
   python docs/specs/perf-vs-orca/evidence/t157-needs-support-predicate/price.py
   git checkout -- crates/
   cargo build --release -p pnp-cli
   cargo xtask build-guests --check
   ```
